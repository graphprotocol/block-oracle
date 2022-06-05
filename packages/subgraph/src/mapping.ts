import {
  CrossChainEpochOracleCall,
  Log
} from "../generated/DataEdge/DataEdge";
import { Bytes, BigInt, log } from "@graphprotocol/graph-ts";
import {
  DataEdge,
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
  getGlobalState,
  getTags,
  decodePrefixVarIntU64,
  decodePrefixVarIntI64,
  getStringFromBytes,
  getAuxGlobalState,
  commitToGlobalState,
  rollbackToGlobalState,
  getOrCreateEpoch,
  createOrUpdateNetworkEpochBlockNumber,
  getNetworkList,
  swapAndPop,
  MessageTag,
  commitNetworkChanges
} from "./helpers";
import {
  PREAMBLE_BIT_LENGTH,
  TAG_BIT_LENGTH,
  BIGINT_ZERO,
  BIGINT_ONE
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

export function processMessage(globalState: GlobalState, messageBlock: MessageBlock, i: i32, tag: MessageTag, payload: Bytes): i32 {
  log.warning("Processing new message with tag {}", [MessageTag.toString(tag)]);
  log.warning("The remaining payload is {}", [payload.toHexString()]);

  if (payload.length == 0) {
    //rollbackToGlobalState(globalState);
    //return;
    return 0;
  }

  let numBytesRead = executeMessage(
    tag,
    i,
    globalState,
    messageBlock.id,
    payload
  );

  log.warning("Bytes read: {}", [numBytesRead.toString()]);
  return numBytesRead;
}

export function processMessageBlock(globalState: GlobalState, messageBlock: MessageBlock, payload: Bytes): i32 {
  let numBytesRead = 0;

  // Read the message block's tags.
  numBytesRead += 1;
  let preamble = changetype<Bytes>(payload.slice(0, PREAMBLE_BIT_LENGTH / 8));
  let tags = getTags(preamble);

  for (let i = 0; i < tags.length; i++) {
    let messageBytes = changetype<Bytes>(payload.slice(numBytesRead));
    numBytesRead += processMessage(globalState, messageBlock, i, tags[i], messageBytes);
  }

  messageBlock.data = payload; // cut it to the amount actually read
  messageBlock.save();

  return numBytesRead;
}

export function processPayload(
  submitter: String,
  payloadBytes: Bytes,
  txHash: String
): void {
  // Load auxiliary GlobalState for rollback capabilities
  let globalState = getAuxGlobalState();

  // Save raw payload
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

    // Save raw message
    let messageBlock = new MessageBlock(
      [txHash, i].join("-")
    );

    numBytesRead += processMessageBlock(globalState, messageBlock, rawPayloadData);
    log.warning("Finished processing message block num. {}", [i]);
    numBlocksRead++;
  }

  commitToGlobalState(globalState);
}

// Executes the message and returns the amount of bytes read
function executeMessage(
  tag: MessageTag,
  index: i32,
  globalState: GlobalState,
  messageBlockId: string,
  data: Bytes
): i32 {
  let bytesRead = 0;
  let id = [messageBlockId, BigInt.fromI32(index).toString()].join("-");

  log.warning("Executing message {}", [MessageTag.toString(tag)]);
  if (tag == MessageTag.SetBlockNumbersForEpochMessage) {
    let message = new SetBlockNumbersForEpochMessage(id);
    bytesRead = executeSetBlockNumbersForEpochMessage(message, globalState, data);
  } else if (tag == MessageTag.CorrectEpochsMessage) {
    let message = new CorrectEpochsMessage(id);
    bytesRead = executeCorrectEpochsMessage(message, globalState, data);
  } else if (tag == MessageTag.UpdateVersionsMessage) {
    let message = new UpdateVersionsMessage(id);
    bytesRead = executeUpdateVersionsMessage(message, globalState, data);
  } else if (tag == MessageTag.RegisterNetworksMessage) {
    let message = new RegisterNetworksMessage(id);
    bytesRead = executeRegisterNetworksMessage(message, globalState, data);
  } else {
    assert(false, "Unknown message tag. This is a bug!");
    // Makes the compiler happy.
    return 0;
  }

  message.block = messageBlockId;
  return bytesRead;
}

function executeNonEmptySetBlockNumbersForEpochMessage(message: SetBlockNumbersForEpochMessage, globalState: GlobalState, data: Bytes): i32 {
  let bytesRead = 0;
  log.warning("get or create epoch", []);
  let newEpoch = getOrCreateEpoch(
    (globalState.latestValidEpoch != null
      ? BigInt.fromString(globalState.latestValidEpoch!)
      : BIGINT_ZERO) + BIGINT_ONE
  );
  globalState.latestValidEpoch = newEpoch.id;
  log.warning("get or create epoch done", []);

  let merkleRoot = changetype<Bytes>(data.slice(bytesRead, bytesRead + 32))
  message.merkleRoot = merkleRoot;
  bytesRead += 32;
  log.warning("The Merkle root after the update is {}", [merkleRoot.toHexString()]);

  let accelerations: Array<BigInt> = [];
  for (let i = 0; i < globalState.activeNetworkCount; i++) {
    log.warning("Decoding acceleration num. {} from bytes {}", [
      i.toString(),
      changetype<Bytes>(data.slice(bytesRead)).toHexString()
    ]);

    let readAcceleration = decodePrefixVarIntI64(data, bytesRead); // we should check for errors here
    if (readAcceleration[1] == 0) {
      log.warning("Acceleration decoding failed", []);
    } else {
      log.warning("Acceleration decoded reading {} bytes", [readAcceleration[1].toString()]);
      bytesRead += readAcceleration[1] as i32;
      accelerations.push(BigInt.fromI64(readAcceleration[0]));
    }

    // Create new NetworkEpochBlockNumber
    createOrUpdateNetworkEpochBlockNumber(
      i.toString(),
      newEpoch.epochNumber,
      BigInt.fromI64(readAcceleration[0])
    );
    log.warning("created/update acceleration number {}", [i.toString()]);
  }

  message.accelerations = accelerations;
  message.data = changetype<Bytes>(data.slice(0, bytesRead));
  message.save();
  return bytesRead;
}

function executeEmptySetBlockNumbersForEpochMessage(message: SetBlockNumbersForEpochMessage, globalState: GlobalState, data: Bytes): i32 {
  let bytesRead = 0;

  log.warning("read count before", []);
  let readCount = decodePrefixVarIntU64(data, bytesRead); // we should check for errors here
  log.warning("read count after", []);
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
  return bytesRead;
}

function executeSetBlockNumbersForEpochMessage(
  message: SetBlockNumbersForEpochMessage,
  globalState: GlobalState,
  data: Bytes
): i32 {
  log.warning("Expecting {} block number updates", [globalState.activeNetworkCount.toString()]);
  if (globalState.activeNetworkCount == 0) {
    return executeEmptySetBlockNumbersForEpochMessage(message, globalState, data);
  } else {
    return executeNonEmptySetBlockNumbersForEpochMessage(message, globalState, data);
  }
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
  // Decode the next encoding version ID.
  let version = decodePrefixVarIntU64(data, 0);

  if (version[1] == 0) {
    // TODO: handle decoding error.
  } else {
    globalState.encodingVersion = version[0] as i32;
    bytesRead += version[1] as i32;
  }

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
    log.warning("New network", []);
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
