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
  MessageTag,
  getNetworkList,
  swapAndPop,
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

  let messageBlockCounter = 0;

  while (rawPayloadData.length > 0) {
    log.warning("NEW MESSAGE BLOCK {}", [messageBlockCounter.toString()]);
    // Save raw message
    let messageBlock = new MessageBlock(
      [txHash, BigInt.fromI32(messageBlockCounter).toString()].join("-")
    );

    let tags = getTags(
      changetype<Bytes>(rawPayloadData.slice(0, PREAMBLE_BIT_LENGTH / 8))
    );

    rawPayloadData = changetype<Bytes>(
      rawPayloadData.slice(PREAMBLE_BIT_LENGTH / 8)
    );

    for (let i = 0; i < tags.length; i++) {
      log.warning("NEW LOOP", []);
      log.warning("Payload size now {}", [rawPayloadData.length.toString()]);

      if (rawPayloadData.length == 0) {
        //rollbackToGlobalState(globalState);
        //return;
        break;
      }

      let bytesRead = executeMessage(
        tags[i],
        i,
        globalState,
        messageBlock.id,
        rawPayloadData
      );
      rawPayloadData = changetype<Bytes>(rawPayloadData.slice(bytesRead));
      log.warning("Bytes read: {}", [bytesRead.toString()]);
    }

    messageBlock.data = rawPayloadData; // cut it to the amount actually read
    messageBlock.save();
    log.warning("END OF MESSAGE BLOCK {}", [messageBlockCounter.toString()]);
    messageBlockCounter++;
  }

  commitToGlobalState(globalState);
}

// Executes the message and returns the amount of bytes read
function executeMessage(
  tag: i32,
  index: i32,
  globalState: GlobalState,
  messageBlockId: string,
  data: Bytes
): i32 {
  let bytesRead = 0;
  let id = [messageBlockId, BigInt.fromI32(index).toString()].join("-");
  // The message type can then be changed according to the tag.
  let message = new SetBlockNumbersForEpochMessage(id);
  message.block = messageBlockId;

  log.warning("Executing message {}", [MessageTag.toString(tag)]);
  if (tag == MessageTag.SetBlockNumbersForEpochMessage) {
    bytesRead = executeSetBlockNumbersForEpochMessage(
      changetype<SetBlockNumbersForEpochMessage>(message), globalState, data
    );
  } else if (tag == MessageTag.CorrectEpochsMessage) {
    bytesRead = executeCorrectEpochsMessage(
      changetype<CorrectEpochsMessage>(message), globalState, data
    );
  } else if (tag == MessageTag.UpdateVersionsMessage) {
    bytesRead = executeUpdateVersionsMessage(
      changetype<UpdateVersionsMessage>(message), globalState, data
    );
  } else if (tag == MessageTag.RegisterNetworksMessage) {
    bytesRead = executeRegisterNetworksMessage(
      changetype<RegisterNetworksMessage>(message), globalState, data
    );
  } else {
    log.error("Unknown message tag '{}'. This is most likely a bug!", [MessageTag.toString(tag)]);
    return 0;
  }

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
  let readVersion = decodePrefixVarIntU64(data, bytesRead);
  if (readVersion[1] == 0) {
    return 0;
  }

  globalState.encodingVersion = readVersion[0];
  bytesRead += readVersion[1] as i32;
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
