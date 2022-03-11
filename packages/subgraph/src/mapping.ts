import { CrossChainEpochOracleCall } from "../generated/DataEdge/DataEdge";
import { DataEdge, Message } from "../generated/schema";

export function handleCrossChainEpochOracle(
  call: CrossChainEpochOracleCall
): void {
  // Read input vars
  let submitter = call.transaction.from.toHexString();
  let payloadBytes = call.inputs._payload;
  let txHash = call.transaction.hash.toHexString();

  // Save raw message
  let message = new Message(txHash);
  message.data = payloadBytes;
  message.submitter = submitter;
  message.save();
}
