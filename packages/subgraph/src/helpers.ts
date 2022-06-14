import { Bytes, BigInt } from "@graphprotocol/graph-ts";
import {
  GlobalState,
  Epoch,
  NetworkEpochBlockNumber,
  Network
} from "../generated/schema";
import { PREAMBLE_BIT_LENGTH, TAG_BIT_LENGTH, BIGINT_ONE } from "./constants";
import { log } from "@graphprotocol/graph-ts";

export enum MessageTag {
  SetBlockNumbersForEpochMessage = 0,
  CorrectEpochsMessage,
  UpdateVersionsMessage,
  RegisterNetworksMessage
}

export namespace MessageTag {
  export function toString(tag: MessageTag): string {
    return [
      "SetBlockNumbersForEpochMessage",
      "CorrectEpochsMessage",
      "UpdateVersionsMessage",
      "RegisterNetworksMessage"
    ][tag]
  }
}

export namespace AuxGlobalState {
  export function get(): GlobalState {
    return getOrCreateGlobalState("1");
  }

  export function commit(aux: GlobalState): void {
    let real = getRealGlobalState();
    let networks = getNetworkList(aux);
    commitNetworkChanges([], networks, real);
    real.networkCount = aux.networkCount;
    real.activeNetworkCount = aux.activeNetworkCount;
    real.latestValidEpoch = aux.latestValidEpoch;
    real.save();
    aux.save();
  }

  export function rollback(aux: GlobalState): void {
    // ToDo: Add rollback of network entities here...
    let real = getRealGlobalState();
    aux.networkCount = real.networkCount;
    aux.activeNetworkCount = real.activeNetworkCount;
    aux.networkArrayHead = real.networkArrayHead;
    aux.latestValidEpoch = real.latestValidEpoch;
    aux.save();
  }
}

function getRealGlobalState(): GlobalState {
  return getOrCreateGlobalState("0");
}

function getOrCreateGlobalState(id: string): GlobalState {
  let state = GlobalState.load(id);
  if (state == null) {
    state = new GlobalState(id);
    state.networkCount = 0;
    state.activeNetworkCount = 0;
    state.encodingVersion = 0;
    state.save();
  }
  return state;
}

export function nextEpochId(globalState: GlobalState): BigInt {
  if (globalState.latestValidEpoch == null) {
    return BIGINT_ONE;
  } else {
    return BigInt.fromString(globalState.latestValidEpoch!) + BIGINT_ONE;
  }
}

export function getOrCreateEpoch(epochId: BigInt): Epoch {
  let epoch = Epoch.load(epochId.toString());
  if (epoch == null) {
    epoch = new Epoch(epochId.toString());
    epoch.epochNumber = epochId;
    epoch.save();
  }
  return epoch;
}

export function createOrUpdateNetworkEpochBlockNumber(
  networkId: string,
  epochId: BigInt,
  acceleration: BigInt
): NetworkEpochBlockNumber {
  let id = [epochId.toString(), networkId].join("-");
  let previousId = [(epochId - BIGINT_ONE).toString(), networkId].join("-");

  let networkEpochBlockNumber = NetworkEpochBlockNumber.load(id);
  if (networkEpochBlockNumber == null) {
    networkEpochBlockNumber = new NetworkEpochBlockNumber(id);
    networkEpochBlockNumber.network = networkId;
    networkEpochBlockNumber.epoch = epochId.toString();
  }
  networkEpochBlockNumber.acceleration = acceleration;

  let previousNetworkEpochBlockNumber = NetworkEpochBlockNumber.load(
    previousId
  );
  if (previousNetworkEpochBlockNumber != null) {
    networkEpochBlockNumber.delta = previousNetworkEpochBlockNumber.delta.plus(
      acceleration
    );
    networkEpochBlockNumber.blockNumber = previousNetworkEpochBlockNumber.blockNumber.plus(
      networkEpochBlockNumber.delta
    );
  } else {
    // If there's no previous entity then we consider the previous delta 0
    // There might be an edge case if the previous entity isn't 1 epoch behind
    // in case where a network is removed and then re-added
    // (^ Should we retain the progress of the network if it's removed?)
    networkEpochBlockNumber.delta = acceleration;
    networkEpochBlockNumber.blockNumber = networkEpochBlockNumber.delta;
  }
  networkEpochBlockNumber.save();

  return networkEpochBlockNumber;
}

export function getNetworkList(state: GlobalState): Array<Network> {
  let result: Array<Network> = [];
  if (state.networkArrayHead != null) {
    let currentElement = Network.load(state.networkArrayHead!)!;
    result.push(currentElement);
    while (currentElement.nextArrayElement != null) {
      currentElement = Network.load(currentElement.nextArrayElement!)!;
      result.push(currentElement);
      if (result.length > state.activeNetworkCount) {
        log.warning(
          "[getNetworkList] Network list processed is longer than activeNetworkCount. Network list length: {}, activeNetworkCount: {}",
          [
            result.length.toString(),
            state.activeNetworkCount.toString(),
          ]
        );
      }
    }
  }
  return result;
}

export function swapAndPop(index: i32, networks: Array<Network>): Network {
  if (index >= networks.length) {
    log.warning("[popAndSwap] Index out of bounds. Index {}, list length: {}", [
      index.toString(),
      networks.length.toString()
    ]);
  }

  let tail = networks[networks.length - 1];
  let elementToRemove = networks[index];

  networks[index] = tail;
  networks[networks.length - 1] = elementToRemove;

  return networks.pop();
}

export function commitNetworkChanges(
  removedNetworks: Array<Network>,
  newNetworksList: Array<Network>,
  state: GlobalState
): void {
  for (let i = 0; i < removedNetworks.length; i++) {
    removedNetworks[i].state = null;
    removedNetworks[i].nextArrayElement = null;
    removedNetworks[i].arrayIndex = null;
    removedNetworks[i].save();
  }

  for (let i = 0; i < newNetworksList.length; i++) {
    newNetworksList[i].state = state.id;
    newNetworksList[i].nextArrayElement =
      i < newNetworksList.length - 1 ? newNetworksList[i + 1].id : null;
    newNetworksList[i].arrayIndex = i;
    newNetworksList[i].save();
  }

  if (newNetworksList.length > 0) {
    state.networkArrayHead = newNetworksList[0].id;
  } else {
    state.networkArrayHead = null;
  }
  state.activeNetworkCount = newNetworksList.length;
  state.save();
}