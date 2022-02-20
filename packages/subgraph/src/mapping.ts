import { BigInt, Address, log } from "@graphprotocol/graph-ts";
import {
  NewEpochBlock,
  OwnershipTransferred
} from "../generated/EpochOracle/EpochOracle";
import { Oracle, Network, Epoch, EpochBlock, EpochBlockUpdate, InvalidUpdate } from "../generated/schema";

let networks = new Map<string,string>()
networks.set("1", "mainnet")
networks.set("2", "polygon")

export function handleOwnershipTransferred(event: OwnershipTransferred): void {
    let oracle = new Oracle("oracle");
    oracle.address = event.params.newOwner;
    oracle.save();
}

export function handleNewEpochBlock(event: NewEpochBlock): void {

  let oracle = Oracle.load("oracle");

  if(oracle !== null && event.transaction.from != oracle.address) { // Can this be relied upon?

    let invalidUpdate = new InvalidUpdate(event.transaction.hash.toHexString());
    invalidUpdate.caller = event.transaction.from;
    invalidUpdate.timestamp = event.block.timestamp;
    invalidUpdate.transactionHash = event.transaction.hash;
    invalidUpdate.save()

    return
  }

  if(networks.has(event.params.networkId.toString())) {

    let epochBlockId = event.params.epoch.toString() + "-" + event.params.networkId.toString()

    let network = Network.load(event.params.networkId.toString());

    if (network === null) {
      network = new Network(event.params.networkId.toString());
      network.name = networks.get(event.params.networkId.toString());
      network.latestEpochBlock = event.params.epoch.toString() + "-" + event.params.networkId.toString()
      network.save();
    } else {
      network.latestEpochBlock = epochBlockId;
      network.save();
    }

    let epoch = Epoch.load(event.params.epoch.toString());

    if (epoch === null) {
      epoch = new Epoch(event.params.epoch.toString());
      epoch.save();
    }

    let epochBlock = new EpochBlock(epochBlockId);
    epochBlock.epoch = event.params.epoch.toString();
    epochBlock.network = event.params.networkId.toString();
    epochBlock.blockHash = event.params.blockHash;
    epochBlock.timestamp = event.block.timestamp;
    epochBlock.transactionHash = event.transaction.hash;
    epochBlock.oracle = event.transaction.from.toString();
    epochBlock.save();

    let epochBlockUpdate = new EpochBlockUpdate(epochBlockId + "-" + event.block.timestamp.toString() + "-" + event.transaction.index.toString() + "-" + event.logIndex.toString());
    epochBlockUpdate.epochBlock = epochBlockId
    epochBlockUpdate.epoch = event.params.epoch.toString();
    epochBlockUpdate.network = event.params.networkId.toString();
    epochBlockUpdate.blockHash = event.params.blockHash;
    epochBlockUpdate.timestamp = event.block.timestamp;
    epochBlockUpdate.transactionHash = event.transaction.hash;
    epochBlockUpdate.oracle = event.transaction.from.toString();
    epochBlockUpdate.save();
    }
}
