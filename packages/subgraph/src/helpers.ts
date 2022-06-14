import { Bytes, BigInt } from "@graphprotocol/graph-ts";
import {
  GlobalState,
  Epoch,
  NetworkEpochBlockNumber,
  Network
} from "../generated/schema";
import { PREAMBLE_BIT_LENGTH, TAG_BIT_LENGTH, BIGINT_ONE } from "./constants";
import { log } from "@graphprotocol/graph-ts";

export class BytesReader {
  bytes: Bytes;
  offset: u32;
  ok: boolean;

  constructor(bytes: Bytes) {
    this.bytes = bytes;
    this.offset = 0;
    this.ok = true;
  }

  snapshot(): BytesReader {
    let r = new BytesReader(this.bytes);
    r.offset = this.offset;
    r.ok = this.ok;
    return r;
  }

  diff(snapshot: BytesReader): Bytes {
    return changetype<Bytes>(this.bytes.slice(snapshot.offset, this.offset));
  }

  data(): Bytes {
    return changetype<Bytes>(this.bytes.slice(this.offset));
  }

  length(): u32 {
    return this.bytes.length - this.offset;
  }

  advance(n: u32): Bytes {
    if (n > this.length()) {
      this.ok = false;
      return Bytes.empty();
    }

    this.offset += n as u32;
    return changetype<Bytes>(this.bytes.slice(this.offset - n, this.offset));
  }

  peek(i: u32): u64 {
    if (i >= this.length()) {
      this.ok = false;
      return 0;
    } else {
      return this.bytes[this.offset + i] as u64;
    }
  }

  fail(): this {
    this.ok = false;
    return this;
  }
}

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

export function getGlobalState(): GlobalState {
  return getOrCreateGlobalState("0");
}

export function getAuxGlobalState(): GlobalState {
  return getOrCreateGlobalState("1");
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
export function decodePrefixVarIntI64(reader: BytesReader): i64 {
  // First we need to decode the raw bytes into a u64 and check that it didn't error out
  // Then we need to decode the U64 with ZigZag.
  return zigZagDecode(decodePrefixVarIntU64(reader));
}

// Returns the decoded u64 and the amount of bytes read. [0,0] -> Error
export function decodePrefixVarIntU64(reader: BytesReader): u64 {
  // Please note that `BytesReader` never throws an exception on out-of-bounds
  // access, but it simply marks `reader.ok` as false and returns fake data.
  // This means we can simply ignore bounds checks, and let the caller deal
  // with it.

  let first = reader.peek(0);
  // shift can't be more than 8, but AS compiles u8 to an i32 in bytecode, so
  // ctz acts weirdly here without the min.
  let shift = min(ctz(first), 8);

  let num: u64 = 0;
  if (shift == 0) {
    num = first >> 1;
  } else if (shift == 1) {
    num = (first >> 2) | (reader.peek(1) << 6);
  } else if (shift == 2) {
    num =
      ((first >> 3) as u64) |
      (reader.peek(1) << 5) |
      (reader.peek(2) << 13);
  } else if (shift == 3) {
    num =
      ((first >> 4) as u64) |
      (reader.peek(1) << 4) |
      (reader.peek(2) << 12) |
      (reader.peek(3) << 20);
  } else if (shift == 4) {
    num =
      ((first >> 5) as u64) |
      (reader.peek(1) << 3) |
      (reader.peek(2) << 11) |
      (reader.peek(3) << 19) |
      (reader.peek(4) << 27);
  } else if (shift == 5) {
    num =
      ((first >> 6) as u64) |
      (reader.peek(1) << 2) |
      (reader.peek(2) << 10) |
      (reader.peek(3) << 18) |
      (reader.peek(4) << 26) |
      (reader.peek(5) << 34);
  } else if (shift == 6) {
    num =
      ((first >> 7) as u64) |
      (reader.peek(1) << 1) |
      (reader.peek(2) << 9) |
      (reader.peek(3) << 17) |
      (reader.peek(4) << 25) |
      (reader.peek(5) << 33) |
      (reader.peek(6) << 41);
  } else if (shift == 7) {
    num =
      (reader.peek(1) << 0) |
      (reader.peek(2) << 8) |
      (reader.peek(3) << 16) |
      (reader.peek(4) << 24) |
      (reader.peek(5) << 32) |
      (reader.peek(6) << 40) |
      (reader.peek(7) << 48);
  } else if (shift == 8) {
    num =
      (reader.peek(1) << 0) |
      (reader.peek(2) << 8) |
      (reader.peek(3) << 16) |
      (reader.peek(4) << 24) |
      (reader.peek(5) << 32) |
      (reader.peek(6) << 40) |
      (reader.peek(7) << 48) |
      (reader.peek(8) << 56);
  }

  reader.advance((shift as u32) + 1);
  return num;
}

export function zigZagDecode(input: u64): i64 {
  return ((input >> 1) ^ -(input & 1)) as i64;
}

export function decodePrefixVarIntString(reader: BytesReader): string {
  let length = decodePrefixVarIntU64(reader);
  if (!reader.ok) {
    return "";
  }

  return reader.advance(length as u32).toString();
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