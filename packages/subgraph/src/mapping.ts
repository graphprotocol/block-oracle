import { CrossChainEpochOracleCall } from "../generated/DataEdge/DataEdge";
import { Bytes, BigInt } from "@graphprotocol/graph-ts";
import { DataEdge, Message, MessageBlock, Payload } from "../generated/schema";
import {
  getGlobalState,
  getTags,
  decodePrefixVarIntU64,
  decodePrefixVarIntI64
} from "./helpers";

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

    for (let i = 0; i < tags.length; i++) {
      executeMessage(tags[i], i, globalState, messageBlock.id, rawPayloadData);
    }

    messageBlock.data = rawPayloadData; // cut it to the amount actually read
    messageBlock.save();
    messageBlockCounter++;
  }
}

function executeMessage(
  tag: i32,
  index: i32,
  globalState: GlobalState,
  messageBlockID: String,
  data: Bytes
): void {
  // ToDo, parse and actually execute message
  let message = new SetBlockNumbersForEpochMessage(
    [messageBlockID, BigInt.fromI32(i).toString()].join("-")
  );
  message.block = messageBlock.id;

  if (tag == 0) {
    // Do stuff
    message.save();
  } else if (tag == 1) {
    let coercedMessage = changetype<CorrectEpochsMessage>(message);
    // Do stuff
    coercedMessage.save();
  } else if (tag == 2) {
    let coercedMessage = changetype<UpdateVersionsMessage>(message);
    // Do stuff
    coercedMessage.save();
  } else if (tag == 3) {
    let coercedMessage = changetype<RegisterNetworksMessage>(message);
    // Do stuff
    coercedMessage.save();
  }
}
