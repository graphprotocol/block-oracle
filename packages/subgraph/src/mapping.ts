import { BigInt, Address } from "@graphprotocol/graph-ts";
import {
  newEpochBlock,
} from "../generated/EpochOracle/EpochOracle";
import { Oracle, Network, Epoch, EpochBlock } from "../generated/schema";

let networks = new Map<i32,string>()

networks.set(1, "mainnet")

export function handleNewEpochBlock(event: newEpochBlock): void {

  if(event.transaction.from !== "0xE09750abE36beA8B2236E48C84BB9da7Ef5aA07c") { // Can this be relied upon?
    return
  }

  let senderString = event.transaction.from.toHexString()
  let oracle = Oracle.load(senderString);

  if (oracle === null) {
    oracle = new Oracle(senderString);
    oracle.address = event.transaction.from;
    oracle.save();
  }

  if(networks.has(event.params.network)) {

  let network = Network.load(event.params.network.toHexString());

  if (network === null) {
    network = new Network(event.params.network.toHexString());
    network.name = networks.get(event.params.network);
    network.save();
  }

  let epoch = Epoch.load(event.params.epoch.toHexString());

  if (epoch === null) {
    epoch = new Epoch(event.params.epoch.toHexString());
    epoch.save();
  }

  let epochBlock = new EpochBlock(event.params.epoch.toHexString() + "-" + event.params.network.toHexString());
  epochBlock.epoch = event.params.epoch.toHexString();
  epochBlock.epoch = event.params.network.toHexString();
  epochBlock.blockHash = event.params.blockHash;
  epochBlock.save();
}
}
