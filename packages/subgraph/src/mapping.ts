import { BigInt, Address, log } from "@graphprotocol/graph-ts";
import {
  newEpochBlock,
} from "../generated/EpochOracle/EpochOracle";
import { Oracle, Network, Epoch, EpochBlock } from "../generated/schema";

let networks = new Map<string,string>()

networks.set("1", "mainnet")

export function handleNewEpochBlock(event: newEpochBlock): void {

  if(event.transaction.from != Address.fromString("0xE09750abE36beA8B2236E48C84BB9da7Ef5aA07c")) { // Can this be relied upon?
    return
  }

  let senderString = event.transaction.from.toHexString()
  let oracle = Oracle.load(senderString);

  if (oracle == null) {
    oracle = new Oracle(senderString);
    oracle.address = event.transaction.from;
    oracle.save();
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
    epochBlock.save();
    }
}
