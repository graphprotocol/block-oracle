import { CrossChainEpochOracleCall } from "../generated/DataEdge/DataEdge";
import { Bytes, BigInt } from "@graphprotocol/graph-ts";
import {
  DataEdge,
  SetBlockNumbersForEpochMessage,
  CorrectEpochsMessage,
  UpdateVersionsMessage,
  RegisterNetworksMessage,
  MessageBlock,
  Payload,
  GlobalState
} from "../generated/schema";
import {
  getGlobalState,
  getTags,
  decodePrefixVarIntU64,
  decodePrefixVarIntI64
} from "./helpers";
import { PREAMBLE_BIT_LENGTH, TAG_BIT_LENGTH } from "./constants";

export function handleCrossChainEpochOracle(
  call: CrossChainEpochOracleCall
): void {
  // Read input vars
  let submitter = call.transaction.from.toHexString();
  let payloadBytes = call.inputs._payload;
  let txHash = call.transaction.hash.toHexString();

  // Load GlobalState
  let globalState = getGlobalState();

  // Save raw payload
  let payload = new Payload(txHash);
  payload.data = payloadBytes;
  payload.submitter = submitter;
  payload.save();

  let rawPayloadData = payloadBytes;

  let messageBlockCounter = 0;

  while (rawPayloadData.length > 0) {
    // Save raw message
    let messageBlock = new MessageBlock(
      [txHash, BigInt.fromI32(messageBlockCounter).toString()].join("-")
    );

    let tags = getTags(
      changetype<Bytes>(rawPayloadData.slice(0, PREAMBLE_BIT_LENGTH / 8))
    );

    rawPayloadData = rawPayloadData.slice(PREAMBLE_BIT_LENGTH / 8);

    for (let i = 0; i < tags.length; i++) {
      let bytesRead = executeMessage(
        tags[i],
        i,
        globalState,
        messageBlock.id,
        rawPayloadData
      );
      rawPayloadData = rawPayloadData.slice(bytesRead);
    }

    messageBlock.data = rawPayloadData; // cut it to the amount actually read
    messageBlock.save();
    messageBlockCounter++;
  }

  globalState.save();
}

// Executes the message and returns the amount of bytes read
function executeMessage(
  tag: i32,
  index: i32,
  globalState: GlobalState,
  messageBlockID: String,
  data: Bytes
): i32 {
  let bytesRead = 0;
  // ToDo, parse and actually execute message
  let message = new SetBlockNumbersForEpochMessage(
    [messageBlockID, BigInt.fromI32(index).toString()].join("-")
  );
  message.block = messageBlockID;

  if (tag == 0) {
    bytesRead = executeSetBlockNumbersForEpochMessage(
      message,
      globalState,
      data
    );
  } else if (tag == 1) {
    bytesRead = executeCorrectEpochsMessage(
      changetype<CorrectEpochsMessage>(message),
      globalState,
      data
    );
  } else if (tag == 2) {
    bytesRead = executeUpdateVersionsMessage(
      changetype<UpdateVersionsMessage>(message),
      globalState,
      data
    );
  } else if (tag == 3) {
    bytesRead = executeRegisterNetworksMessage(
      changetype<RegisterNetworksMessage>(message),
      globalState,
      data
    );
  }

  return bytesRead;
}

function executeSetBlockNumbersForEpochMessage(
  message: SetBlockNumbersForEpochMessage,
  globalState: GlobalState,
  data: Bytes
): i32 {
  let bytesRead = 0;
  if (globalState.networkCount != 0) {
    // To Do
    message.save();
  } else {
    message.save();
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
  // To Do
  return bytesRead;
}
