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

export function getGlobalState(): GlobalState {
  let state = GlobalState.load("0");
  if (state == null) {
    state = new GlobalState("0");
    state.save();
  }
  return state;
}

export function getAuxGlobalState(): GlobalState {
  let state = GlobalState.load("1");
  if (state == null) {
    state = new GlobalState("1");
    state.save();
  }
  return state;
}

export function commitToGlobalState(state: GlobalState): void {
  let realGlobalState = getGlobalState();
  let networks = getNetworkList(state);
  commitNetworkChanges([], networks, realGlobalState);
  realGlobalState.networkCount = state.networkCount;
  realGlobalState.activeNetworkCount = state.activeNetworkCount;
  realGlobalState.latestValidEpoch = state.latestValidEpoch;
  realGlobalState.save();
  state.save();
}

export function rollbackToGlobalState(state: GlobalState): void {
  // ToDo: Add rollback of network entities here...
  let realGlobalState = getGlobalState();
  state.networkCount = realGlobalState.networkCount;
  state.activeNetworkCount = realGlobalState.activeNetworkCount;
  state.networkArrayHead = realGlobalState.networkArrayHead;
  state.latestValidEpoch = realGlobalState.latestValidEpoch;
  state.save();
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
  networkId: String,
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

export function getTags(preamble: Bytes): Array<MessageTag> {
  let tags = new Array<MessageTag>();
  for (let i = 0; i < PREAMBLE_BIT_LENGTH / TAG_BIT_LENGTH; i++) {
    tags.push(getTag(preamble, i));
  }
  return tags;
}

function getTag(preamble: Bytes, index: i32): MessageTag {
  return (
    (BigInt.fromUnsignedBytes(preamble).toI32() >> (index * TAG_BIT_LENGTH)) &
    (2 ** TAG_BIT_LENGTH - 1)
  );
}

// Returns the decoded i64 and the amount of bytes read. [0,0] -> Error
export function decodePrefixVarIntI64(bytes: Bytes, offset: u32): Array<i64> {
  let result: i64 = 0;

  // First we need to decode the raw bytes into a u64 and check that it didn't error out
  let zigZagDecodeInput = decodePrefixVarIntU64(bytes, offset);
  if (zigZagDecodeInput[1] != 0) {
    // Then we need to decode the U64 with ZigZag
    result = zigZagDecode(zigZagDecodeInput[0]);
  }
  return [result, zigZagDecodeInput[1]];
}

// Returns the decoded u64 and the amount of bytes read. [0,0] -> Error
export function decodePrefixVarIntU64(bytes: Bytes, offset: u32): Array<u64> {
  let first = bytes[offset];
  // shift can't be more than 8, but AS compiles u8 to an i32 in bytecode, so ctz acts weirdly here without the min.
  let shift = min(ctz(first), 8);

  // Checking for invalid inputs that would break the algorithm
  if (((offset + shift) as i32) >= bytes.length) {
    return [0, 0];
  }

  let result: u64;
  if (shift == 0) {
    result = (first >> 1) as u64;
  } else if (shift == 1) {
    result = ((first >> 2) as u64) | ((bytes[offset + 1] as u64) << 6);
  } else if (shift == 2) {
    result =
      ((first >> 3) as u64) |
      ((bytes[offset + 1] as u64) << 5) |
      ((bytes[offset + 2] as u64) << 13);
  } else if (shift == 3) {
    result =
      ((first >> 4) as u64) |
      ((bytes[offset + 1] as u64) << 4) |
      ((bytes[offset + 2] as u64) << 12) |
      ((bytes[offset + 3] as u64) << 20);
  } else if (shift == 4) {
    result =
      ((first >> 5) as u64) |
      ((bytes[offset + 1] as u64) << 3) |
      ((bytes[offset + 2] as u64) << 11) |
      ((bytes[offset + 3] as u64) << 19) |
      ((bytes[offset + 4] as u64) << 27);
  } else if (shift == 5) {
    result =
      ((first >> 6) as u64) |
      ((bytes[offset + 1] as u64) << 2) |
      ((bytes[offset + 2] as u64) << 10) |
      ((bytes[offset + 3] as u64) << 18) |
      ((bytes[offset + 4] as u64) << 26) |
      ((bytes[offset + 5] as u64) << 34);
  } else if (shift == 6) {
    result =
      ((first >> 7) as u64) |
      ((bytes[offset + 1] as u64) << 1) |
      ((bytes[offset + 2] as u64) << 9) |
      ((bytes[offset + 3] as u64) << 17) |
      ((bytes[offset + 4] as u64) << 25) |
      ((bytes[offset + 5] as u64) << 33) |
      ((bytes[offset + 6] as u64) << 41);
  } else if (shift == 7) {
    result =
      (bytes[offset + 1] as u64) |
      ((bytes[offset + 2] as u64) << 8) |
      ((bytes[offset + 3] as u64) << 16) |
      ((bytes[offset + 4] as u64) << 24) |
      ((bytes[offset + 5] as u64) << 32) |
      ((bytes[offset + 6] as u64) << 40) |
      ((bytes[offset + 7] as u64) << 48);
  } else if (shift == 8) {
    result =
      (bytes[offset + 1] as u64) |
      ((bytes[offset + 2] as u64) << 8) |
      ((bytes[offset + 3] as u64) << 16) |
      ((bytes[offset + 4] as u64) << 24) |
      ((bytes[offset + 5] as u64) << 32) |
      ((bytes[offset + 6] as u64) << 40) |
      ((bytes[offset + 7] as u64) << 48) |
      ((bytes[offset + 8] as u64) << 56);
  }

  return [result, shift + 1];
}

export function zigZagDecode(input: u64): i64 {
  return ((input >> 1) ^ -(input & 1)) as i64;
}

export function getStringFromBytes(
  bytes: Bytes,
  offset: u32,
  stringLength: u32
): String {
  let slicedBytes = changetype<Bytes>(
    bytes.slice(offset, offset + stringLength)
  );
  return slicedBytes.toString();
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

export function swapAndPop(
  index: i32,
  networks: Array<Network>
): Network {
  if (index >= networks.length) {
    log.warning("[popAndSwap] Index out of bounds. Index {}, list length: {}", [
      index.toString(),
      networks.length.toString()
    ]);
  }

  let tail = networks[-1]
  let elementToRemove = networks[index]

  networks[index] = tail
  networks[-1] = elementToRemove

  return networks.pop();
}

export function commitNetworkChanges(
  removedNetworks: Array<Network>,
  newNetworksList: Array<Network>,
  state: GlobalState
): void {
  for (let i = 0; i < removedNetworks.length; i++) {
    removedNetworks[i].state = null
    removedNetworks[i].nextArrayElement = null
    removedNetworks[i].arrayIndex = null
    removedNetworks[i].save()
  }

  for (let i = 0; i < newNetworksList.length; i++) {
    newNetworksList[i].state = state.id
    newNetworksList[i].nextArrayElement = i < newNetworksList.length - 1 ? newNetworksList[i + 1].id : null
    newNetworksList[i].arrayIndex = i
    newNetworksList[i].save()
  }

  if (newNetworksList.length > 0) {
    state.networkArrayHead = newNetworksList[0].id
  } else {
    state.networkArrayHead = null
  }
  state.activeNetworkCount = newNetworksList.length
  state.save()
}
