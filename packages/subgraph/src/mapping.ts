import { BigInt, Address, log } from "@graphprotocol/graph-ts";
import {
  NewEpochBlock,
  OwnershipTransferred
} from "../generated/EpochOracle/EpochOracle";
import { Oracle, Network, Epoch, EpochBlock } from "../generated/schema";

let networks = new Map<string,string>()
networks.set("1", "mainnet")

export function handleOwnershipTransferred(event: OwnershipTransferred): void {
    let oracle = new Oracle("oracle");
    oracle.address = event.params.newOwner;
    oracle.save();
}

export function handleNewEpochBlock(event: NewEpochBlock): void {

  let oracle = Oracle.load("oracle");

  if(oracle !== null && event.transaction.from != oracle.address) { // Can this be relied upon?
    return
  }

  if(networks.has(event.params.networkId.toString())) {

    let network = Network.load(event.params.networkId.toString());

    if (network === null) {
      network = new Network(event.params.networkId.toString());
      network.name = networks.get(event.params.networkId.toString());
      network.save();
    }

    let epoch = Epoch.load(event.params.epoch.toString());

    if (epoch === null) {
      epoch = new Epoch(event.params.epoch.toString());
      epoch.save();
    }

    let epochBlock = new EpochBlock(event.params.epoch.toString() + "-" + event.params.networkId.toString());
    epochBlock.epoch = event.params.epoch.toString();
    epochBlock.network = event.params.networkId.toString();
    epochBlock.blockHash = event.params.blockHash;
    epochBlock.timestamp = event.block.timestamp;
    epochBlock.save();
    }
}
