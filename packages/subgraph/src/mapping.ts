import { CrossChainEpochOracleCall } from "../generated/DataEdge/DataEdge";
import { Bytes, BigInt } from "@graphprotocol/graph-ts";
import { DataEdge, Message, MessageBlock } from "../generated/schema";
import {
  SET_BLOCK_NUMBERS_FOR_EPOCH,
  CORRECT_EPOCHS,
  UPDATE_VERSIONS,
  REGISTER_NETWORKS,
  PREAMBLE_BIT_LENGTH,
  TAG_BIT_LENGTH
} from "./constants";

export function handleCrossChainEpochOracle(
  call: CrossChainEpochOracleCall
): void {
  // Read input vars
  let submitter = call.transaction.from.toHexString();
  let payloadBytes = call.inputs._payload;
  let txHash = call.transaction.hash.toHexString();

  // Save raw message
  let messageBlock = new MessageBlock(txHash);
  messageBlock.data = payloadBytes;
  messageBlock.submitter = submitter;
  messageBlock.save();

  let tags = getTags(
    changetype<Bytes>(messageBlock.data.slice(0, PREAMBLE_BIT_LENGTH / 8))
  );
  for (let i = 0; i < tags.length; i++) {
    let message = new Message([txHash, BigInt.fromI32(i).toString()].join("-"));
    message.type = tags[i];
    message.block = messageBlock.id;
    message.save();
  }
}

function getTags(preamble: Bytes): Array<String> {
  let tags = new Array<String>();
  for (let i = 0; i < PREAMBLE_BIT_LENGTH / TAG_BIT_LENGTH; i++) {
    tags.push(getTag(preamble, i));
  }
  return tags;
}

function getTag(preamble: Bytes, index: i32): String {
  return translateI32ToMessageType(
    (BigInt.fromUnsignedBytes(preamble).toI32() >> (index * TAG_BIT_LENGTH)) &
      (2 ** TAG_BIT_LENGTH - 1)
  );
}

function translateI32ToMessageType(tag: i32): String {
  let result = "";
  if (tag == 0) {
    result = SET_BLOCK_NUMBERS_FOR_EPOCH;
  } else if (tag == 1) {
    result = CORRECT_EPOCHS;
  } else if (tag == 2) {
    result = UPDATE_VERSIONS;
  } else if (tag == 3) {
    result = REGISTER_NETWORKS;
  }
  return result;
}
