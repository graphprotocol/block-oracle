import {
  CrossChainEpochOracleCall,
  Log
} from "../generated/DataEdge/DataEdge";
import { Bytes, BigInt, log } from "@graphprotocol/graph-ts";
import {
  SetBlockNumbersForEpochMessage,
  CorrectEpochsMessage,
  UpdateVersionsMessage,
  RegisterNetworksMessage,
  MessageBlock,
  Payload,
  GlobalState,
  Network
} from "../generated/schema";
import {
  getTags,
  decodePrefixVarIntU64,
  decodePrefixVarIntI64,
  getStringFromBytes,
  getAuxGlobalState,
  commitToGlobalState,
  getOrCreateEpoch,
  createOrUpdateNetworkEpochBlockNumber,
  MessageTag,
  getNetworkList,
  swapAndPop,
  commitNetworkChanges
} from "./helpers";
import {
  PREAMBLE_BIT_LENGTH,
  BIGINT_ZERO,
  BIGINT_ONE,
  PREAMBLE_BYTE_LENGTH
} from "./constants";

export function handleLogCrossChainEpochOracle(
  event: Log
): void {
  // Read input vars
  let submitter = event.transaction.from.toHexString();
  let payloadBytes = event.params.data;
  let txHash = event.transaction.hash.toHexString();

  processPayload(submitter, payloadBytes, txHash);
}

export function handleCrossChainEpochOracle(
  call: CrossChainEpochOracleCall
): void {
  // Read input vars
  let submitter = call.transaction.from.toHexString();
  let payloadBytes = call.inputs._payload;
  let txHash = call.transaction.hash.toHexString();

  processPayload(submitter, payloadBytes, txHash);
}

export function processPayload(
  submitter: string,
  payloadBytes: Bytes,
  txHash: string
): void {
  // Load auxiliary GlobalState for rollback capabilities.
  let globalState = getAuxGlobalState();

  // Save raw payload.
  let payload = new Payload(txHash);
  payload.data = payloadBytes;
  payload.submitter = submitter;
  payload.save();

  let rawPayloadData = payloadBytes;
  let numBlocksRead = 0;
  let numBytesRead = 0;

  while (numBytesRead < rawPayloadData.length) {
    let i = BigInt.fromI32(numBlocksRead).toString();
    log.warning("New message block (num. {}) with remaining data: {}", [i, rawPayloadData.toHexString()]);

    // Save raw message.
    let messageBlock = new MessageBlock([txHash, i].join("-"));

    numBytesRead += processMessageBlock(globalState, messageBlock, rawPayloadData);
    if (numBytesRead == 0) {
      log.error("Failed to process message block num. {}", [i]);
      return;
    }

    log.warning("Finished processing message block num. {}", [i]);
    numBlocksRead++;
  }

  commitToGlobalState(globalState);
}

export function processMessageBlock(
  globalState: GlobalState,
  messageBlock: MessageBlock,
  payload: Bytes
): i32 {
  let numBytesRead = PREAMBLE_BYTE_LENGTH;

  // Read the message block's tags.
  let preamble = changetype<Bytes>(payload.slice(0, PREAMBLE_BYTE_LENGTH));
  let tags = getTags(preamble);

  for (let i = 0; i < tags.length; i++) {
    let messageBytes = changetype<Bytes>(payload.slice(numBytesRead));
    numBytesRead += processMessage(globalState, messageBlock, i, tags[i], messageBytes);
  }

  messageBlock.data = changetype<Bytes>(payload.slice(PREAMBLE_BYTE_LENGTH, numBytesRead));
  messageBlock.save();

  return numBytesRead;
}

// Finishes decoding the message, executes it, and finally returns the amount
// of bytes read.
export function processMessage(
  globalState: GlobalState,
  messageBlock: MessageBlock,
  i: i32,
  tag: MessageTag,
  payload: Bytes
): i32 {
  log.warning("Processing new message with tag {}", [MessageTag.toString(tag)]);
  log.warning("The remaining payload is {}", [payload.toHexString()]);

  if (payload.length == 0) {
    return 0;
  }

  let bytesRead = 0;
  let id = [messageBlock.id, BigInt.fromI32(i).toString()].join("-");
  // The message type can then be changed according to the tag.
  let message = new SetBlockNumbersForEpochMessage(id);
  message.block = messageBlock.id;

  log.warning("Executing message {}", [MessageTag.toString(tag)]);
  if (tag == MessageTag.SetBlockNumbersForEpochMessage) {
    bytesRead = executeSetBlockNumbersForEpochMessage(
      changetype<SetBlockNumbersForEpochMessage>(message), globalState, payload
    );
  } else if (tag == MessageTag.CorrectEpochsMessage) {
    bytesRead = executeCorrectEpochsMessage(
      changetype<CorrectEpochsMessage>(message), globalState, payload
    );
  } else if (tag == MessageTag.UpdateVersionsMessage) {
    bytesRead = executeUpdateVersionsMessage(
      changetype<UpdateVersionsMessage>(message), globalState, payload
    );
  } else if (tag == MessageTag.RegisterNetworksMessage) {
    bytesRead = executeRegisterNetworksMessage(
      changetype<RegisterNetworksMessage>(message), globalState, payload
    );
  } else {
    log.error("Unknown message tag '{}'. This is most likely a bug!", [MessageTag.toString(tag)]);
    return 0;
  }

  log.warning("Bytes read: {}", [bytesRead.toString()]);
  return bytesRead;
}

function executeSetBlockNumbersForEpochMessage(
  message: SetBlockNumbersForEpochMessage,
  globalState: GlobalState,
  data: Bytes
): i32 {
  let bytesRead = 0;

  if (globalState.activeNetworkCount != 0) {
    let newEpoch = getOrCreateEpoch(
      (globalState.latestValidEpoch != null
        ? BigInt.fromString(globalState.latestValidEpoch!)
        : BIGINT_ZERO) + BIGINT_ONE
    );
    globalState.latestValidEpoch = newEpoch.id;

    message.merkleRoot = changetype<Bytes>(
      data.slice(bytesRead, bytesRead + 32)
    );
    bytesRead += 32;
    let accelerations: Array<BigInt> = [];
    for (let i = 0; i < globalState.activeNetworkCount; i++) {
      let readAcceleration = decodePrefixVarIntI64(data, bytesRead); // we should check for errors here
      bytesRead += readAcceleration[1] as i32;
      accelerations.push(BigInt.fromI64(readAcceleration[0]));

      // Create new NetworkEpochBlockNumber
      createOrUpdateNetworkEpochBlockNumber(
        i.toString(),
        newEpoch.epochNumber,
        BigInt.fromI64(readAcceleration[0])
      );
    }

    message.accelerations = accelerations;
    message.data = changetype<Bytes>(data.slice(0, bytesRead));
    message.save();
  } else {
    let readCount = decodePrefixVarIntU64(data, bytesRead); // we should check for errors here
    message.count = BigInt.fromU64(readCount[0]);
    bytesRead += readCount[1] as i32;
    message.save();

    log.warning("BEFORE EPOCH LOOP, AMOUNT TO CREATE: {}", [
      message.count!.toString()
    ]);

    for (let i = BIGINT_ZERO; i < message.count!; i += BIGINT_ONE) {
      log.warning("EPOCH LOOP, CREATING EPOCH: {}", [i.toString()]);
      let newEpoch = getOrCreateEpoch(
        (globalState.latestValidEpoch != null
          ? BigInt.fromString(globalState.latestValidEpoch!)
          : BIGINT_ZERO) + BIGINT_ONE
      );
      globalState.latestValidEpoch = newEpoch.id;
    }
    log.warning("AFTER EPOCH LOOP", []);
  }
  return bytesRead;
}

function executeCorrectEpochsMessage(
  message: CorrectEpochsMessage,
  globalState: GlobalState,
  data: Bytes
): i32 {
  let bytesRead = 0;
  // To Do
  return bytesRead;
}

function executeUpdateVersionsMessage(
  message: UpdateVersionsMessage,
  globalState: GlobalState,
  data: Bytes
): i32 {
  let bytesRead = 0;
  // To Do
  return bytesRead;
}

function executeRegisterNetworksMessage(
  message: RegisterNetworksMessage,
  globalState: GlobalState,
  data: Bytes
): i32 {
  let bytesRead = 0;
  // get remove length
  let readRemoveLength = decodePrefixVarIntU64(data, bytesRead); // we should check errors here
  bytesRead += readRemoveLength[1] as i32;

  let networks = getNetworkList(globalState);
  let removedNetworks: Array<Network> = [];

  // now get all the removed network ids and apply the changes to the pre-loaded list
  for (let i = 0; i < (readRemoveLength[0] as i32); i++) {
    let readRemove = decodePrefixVarIntU64(data, bytesRead);
    bytesRead += readRemove[1] as i32;
    // check network to remove is within bounds
    if (
      networks.length <= (readRemove[0] as i32) ||
      (readRemove[1] as i32) == 0
    ) {
      // trigger error here
    }
    let networkToRemoveID = readRemove[0] as i32;
    networks[networkToRemoveID].removedAt = message.id;
    removedNetworks.push(swapAndPop(networkToRemoveID, networks));
  }

  let readAddLength = decodePrefixVarIntU64(data, bytesRead); // we should check errors here
  bytesRead += readAddLength[1] as i32;

  // now get all the add network strings
  for (let i = 0; i < (readAddLength[0] as i32); i++) {
    let readStrLength = decodePrefixVarIntU64(data, bytesRead); // we should check errors here
    bytesRead += readStrLength[1] as i32;

    let chainID = getStringFromBytes(data, bytesRead, readStrLength[0] as u32);
    bytesRead += readStrLength[0] as i32;

    let network = new Network(chainID);
    network.addedAt = message.id;
    network.save();

    globalState.networkCount += 1;

    networks.push(network);
  }

  commitNetworkChanges(removedNetworks, networks, globalState);

  message.data = changetype<Bytes>(data.slice(0, bytesRead));
  message.removeCount = BigInt.fromU64(readRemoveLength[0]);
  message.addCount = BigInt.fromU64(readAddLength[0]);
  message.save();

  return bytesRead;
}
