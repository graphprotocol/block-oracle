import {
  OwnershipTransferred,
  PostMessageBlocksCall,
} from "../generated/DataVault/DataVault";
import { Vault, Message } from "../generated/schema";

export function handleOwnershipTransferred(event: OwnershipTransferred): void {
  let oracle = new Vault(event.address.toString());
  oracle.owner = event.params.newOwner;
  oracle.save();
}

export function handlePostMessageBlocks(call: PostMessageBlocksCall): void {
  // Read input vars
  let submitter = call.transaction.from.toString();
  let payloadBytes = call.inputs._payload;
  let txHash = call.transaction.hash.toHexString();

  // Save raw message
  let message = new Message(txHash);
  message.data = payloadBytes;
  message.submitter = submitter;
  message.save();
}
