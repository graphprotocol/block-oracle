import { BigInt, Address, log, Bytes } from "@graphprotocol/graph-ts";
import {
  NewEpochBlock,
  OwnershipTransferred,
  SetEpochBlocksPayloadCall,
} from "../generated/EpochOracle/EpochOracle";
import {
  Oracle,
  Epoch,
  EpochBlock,
  InvalidUpdate,
  Update,
} from "../generated/schema";

// Constants
const UPDATE_VERSION_OFFSET = 0;
const UPDATE_VERSION_BYTES = 1;
const UPDATE_LENGTH_OFFSET = UPDATE_VERSION_OFFSET + UPDATE_VERSION_BYTES;
const UPDATE_LENGTH_BYTES = 1;
const UPDATE_EPOCH_OFFSET = UPDATE_LENGTH_OFFSET + UPDATE_LENGTH_BYTES;
const UPDATE_EPOCH_BYTES = 8;
const UPDATE_ITEMS_OFFSET = UPDATE_EPOCH_OFFSET + UPDATE_EPOCH_BYTES;

const UPDATE_NETWORK_ID_BYTES = 2;
const UPDATE_BLOCKHASH_BYTES = 32;
const UPDATE_BYTES = UPDATE_NETWORK_ID_BYTES + UPDATE_BLOCKHASH_BYTES;

export function handleOwnershipTransferred(event: OwnershipTransferred): void {
  let oracle = new Oracle("oracle");
  oracle.owner = event.params.newOwner;
  oracle.save();
}

function loadOrSaveEpoch(epochNumber: BigInt): Epoch {
  let epochNumberStr = epochNumber.toString();
  let epoch = Epoch.load(epochNumberStr);
  if (epoch === null) {
    epoch = new Epoch(epochNumberStr);
    epoch.epochNumber = epochNumber;
    epoch.save();
  }
  return epoch;
}

class Payload {
  version: i32;
  length: i32;
  epochNumber: BigInt;
  itemsBytes: Bytes;

  constructor(
    version: i32,
    length: i32,
    epochNumber: BigInt,
    itemsBytes: Bytes
  ) {
    this.version = version;
    this.length = length;
    this.epochNumber = epochNumber;
    this.itemsBytes = itemsBytes;
  }
}

function decodePayload(payloadBytes: Bytes): Payload {
  // Decode payload version
  let version = payloadBytes.subarray(
    UPDATE_VERSION_OFFSET,
    UPDATE_VERSION_OFFSET + UPDATE_VERSION_BYTES
  );
  // Decode length
  let length = payloadBytes.subarray(
    UPDATE_LENGTH_OFFSET,
    UPDATE_LENGTH_OFFSET + UPDATE_LENGTH_BYTES
  );
  // Decode epochNumber
  let epochNumber = payloadBytes.subarray(
    UPDATE_EPOCH_OFFSET,
    UPDATE_EPOCH_OFFSET + UPDATE_EPOCH_BYTES
  );
  // Bytes for all the update items
  let itemsBytes = payloadBytes.subarray(UPDATE_ITEMS_OFFSET);

  return new Payload(
    Bytes.fromUint8Array(version).toI32(),
    Bytes.fromUint8Array(length).toI32(),
    changetype<BigInt>(epochNumber.reverse()),
    Bytes.fromUint8Array(itemsBytes)
  );
}

export function handleSetEpochBlocksPayload(
  call: SetEpochBlocksPayloadCall
): void {
  // TODO: verify if caller is not authorized oracle ignore payload
  // TODO: verify if not valid version discard
  // TODO: verify if not valid network discard

  // Read input vars
  let payloadBytes = call.inputs._payload;
  let txHash = call.transaction.hash.toHexString();

  // Decode payload
  let payload = decodePayload(payloadBytes);

  // Save raw update payload
  let update = new Update(txHash);
  update.version = payload.version;
  update.length = payload.length;
  update.epochNumber = payload.epochNumber;
  update.payload = payloadBytes.toHexString();
  update.save();

  // Create epoch if not there
  let epoch = loadOrSaveEpoch(payload.epochNumber);

  // Create each of the epoch updates
  for (let i = 0; i < payload.length; i++) {
    let startIndex = UPDATE_ITEMS_OFFSET + i * UPDATE_BYTES;

    // Parse networkId
    let networkId = Bytes.fromUint8Array(
      payloadBytes
        .subarray(startIndex, startIndex + UPDATE_NETWORK_ID_BYTES)
        .reverse()
    ).toI32();
    startIndex += UPDATE_NETWORK_ID_BYTES;

    // Parse blockHash
    let blockHash = Bytes.fromUint8Array(
      payloadBytes.subarray(startIndex, startIndex + UPDATE_BLOCKHASH_BYTES)
    );

    let epochBlockId = epoch.id + "-" + networkId.toString();
    let epochBlock = new EpochBlock(epochBlockId);
    epochBlock.epoch = epoch.id;
    epochBlock.networkId = networkId;
    epochBlock.blockHash = blockHash;
    epochBlock.timestamp = call.block.timestamp.toI32();
    epochBlock.transactionHash = call.transaction.hash;
    epochBlock.oracle = call.transaction.from.toString(); // caller
    epochBlock.save();
  }
}
