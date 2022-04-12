import { Bytes, BigInt } from "@graphprotocol/graph-ts";
import { GlobalState } from "../generated/schema";
import { PREAMBLE_BIT_LENGTH, TAG_BIT_LENGTH } from "./constants";
import { log } from "@graphprotocol/graph-ts";

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
  realGlobalState.networkCount = state.networkCount
  realGlobalState.activeNetworkCount = state.activeNetworkCount
  realGlobalState.save()
  state.save()
}

export function rollbackToGlobalState(state: GlobalState): void {
  let realGlobalState = getGlobalState();
  state.networkCount = realGlobalState.networkCount
  state.activeNetworkCount = realGlobalState.activeNetworkCount
  state.save()
}

export function getTags(preamble: Bytes): Array<i32> {
  let tags = new Array<i32>();
  for (let i = 0; i < PREAMBLE_BIT_LENGTH / TAG_BIT_LENGTH; i++) {
    tags.push(getTag(preamble, i));
  }
  return tags;
}

function getTag(preamble: Bytes, index: i32): i32 {
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
  let slicedBytes = changetype<Bytes>(bytes.slice(offset, offset + stringLength))
  return slicedBytes.toString();
}
