import { Bytes, BigInt } from "@graphprotocol/graph-ts";
import { GlobalState } from "../generated/schema";
import { PREAMBLE_BIT_LENGTH, TAG_BIT_LENGTH } from "./constants";

export function getGlobalState(): GlobalState {
  let state = GlobalState.load("0");
  if (state == null) {
    state = new GlobalState("0");
    state.save();
  }
  return state;
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

/*
Prefix Varint decode rust example

#[cfg(feature = "decode")]
pub fn decode_prefix_varint(bytes: &[u8], offset: &mut usize) -> DecodeResult<u64> {
    // TODO: (Performance) When reading from an array, a series of values can be decoded unchecked.
    // Eg: If there are 100 bytes, each number taken can read at most 9 bytes,
    // so 11 values can be taken unchecked (up to 99 bytes). This will likely read less,
    // so this can remain in an amortized check loop until the size of the remainder
    // is less than 9 bytes.

    let first = bytes.get(*offset).ok_or_else(|| DecodeError::InvalidFormat)?;
    let shift = first.trailing_zeros();

    // TODO: Check that the compiler does unchecked indexing after this
    if (*offset + (shift as usize)) >= bytes.len() {
        return Err(DecodeError::InvalidFormat);
    }

    let result = match shift {
        0 => (first >> 1) as u64,
        1 => (first >> 2) as u64 | ((bytes[*offset + 1] as u64) << 6),
        2 => (first >> 3) as u64 | ((bytes[*offset + 1] as u64) << 5) | ((bytes[*offset + 2] as u64) << 13),
        3 => (first >> 4) as u64 | ((bytes[*offset + 1] as u64) << 4) | ((bytes[*offset + 2] as u64) << 12) | ((bytes[*offset + 3] as u64) << 20),
        4 => {
            (first >> 5) as u64
                | ((bytes[*offset + 1] as u64) << 3)
                | ((bytes[*offset + 2] as u64) << 11)
                | ((bytes[*offset + 3] as u64) << 19)
                | ((bytes[*offset + 4] as u64) << 27)
        }
        5 => {
            (first >> 6) as u64
                | ((bytes[*offset + 1] as u64) << 2)
                | ((bytes[*offset + 2] as u64) << 10)
                | ((bytes[*offset + 3] as u64) << 18)
                | ((bytes[*offset + 4] as u64) << 26)
                | ((bytes[*offset + 5] as u64) << 34)
        }
        6 => {
            (first >> 7) as u64
                | ((bytes[*offset + 1] as u64) << 1)
                | ((bytes[*offset + 2] as u64) << 9)
                | ((bytes[*offset + 3] as u64) << 17)
                | ((bytes[*offset + 4] as u64) << 25)
                | ((bytes[*offset + 5] as u64) << 33)
                | ((bytes[*offset + 6] as u64) << 41)
        }
        7 => {
            (bytes[*offset + 1] as u64)
                | ((bytes[*offset + 2] as u64) << 8)
                | ((bytes[*offset + 3] as u64) << 16)
                | ((bytes[*offset + 4] as u64) << 24)
                | ((bytes[*offset + 5] as u64) << 32)
                | ((bytes[*offset + 6] as u64) << 40)
                | ((bytes[*offset + 7] as u64) << 48)
        }
        8 => {
            (bytes[*offset + 1] as u64)
                | ((bytes[*offset + 2] as u64) << 8)
                | ((bytes[*offset + 3] as u64) << 16)
                | ((bytes[*offset + 4] as u64) << 24)
                | ((bytes[*offset + 5] as u64) << 32)
                | ((bytes[*offset + 6] as u64) << 40)
                | ((bytes[*offset + 7] as u64) << 48)
                | ((bytes[*offset + 8] as u64) << 56)
        }
        _ => unreachable!(),
    };
    *offset += (shift + 1) as usize;
    Ok(result)
}
*/

export function decodePrefixVarIntI64(bytes: Bytes, offset: u32): i64 {}

export function decodePrefixVarIntU64(bytes: Bytes, offset: u32): u64 {
  let first = bytes[offset];
  let shift = ctz(first);

  // // TODO: Check that the compiler does unchecked indexing after this
  // if (*offset + (shift as usize)) >= bytes.len() {
  //     return Err(DecodeError::InvalidFormat);
  // }



  let result: u64;
  if(shift == 0) {
    result = ((first >> 1) as u64)
  } else if(shift == 1) {
    result  = (((first >> 2) as u64) | ((bytes[offset + 1] as u64) << 6))
  } else if(shift == 2) {
    result = (((first >> 3) as u64) | ((bytes[offset + 1] as u64) << 5) | ((bytes[offset + 2] as u64) << 13))
  } else if(shift == 3) {
    result = (((first >> 4) as u64) | ((bytes[offset + 1] as u64) << 4) | ((bytes[offset + 2] as u64) << 12) | ((bytes[offset + 3] as u64) << 20))
  } else if(shift == 4) {
    result = (((first >> 5) as u64)
        | ((bytes[offset + 1] as u64) << 3)
        | ((bytes[offset + 2] as u64) << 11)
        | ((bytes[offset + 3] as u64) << 19)
        | ((bytes[offset + 4] as u64) << 27))
  } else if(shift == 5) {
    result = (((first >> 6) as u64)
        | ((bytes[offset + 1] as u64) << 2)
        | ((bytes[offset + 2] as u64) << 10)
        | ((bytes[offset + 3] as u64) << 18)
        | ((bytes[offset + 4] as u64) << 26)
        | ((bytes[offset + 5] as u64) << 34))
  } else if(shift == 6) {
    result = (((first >> 7) as u64)
        | ((bytes[offset + 1] as u64) << 1)
        | ((bytes[offset + 2] as u64) << 9)
        | ((bytes[offset + 3] as u64) << 17)
        | ((bytes[offset + 4] as u64) << 25)
        | ((bytes[offset + 5] as u64) << 33)
        | ((bytes[offset + 6] as u64) << 41))
  } else if(shift == 7) {
    result = ((bytes[offset + 1] as u64)
        | ((bytes[offset + 2] as u64) << 8)
        | ((bytes[offset + 3] as u64) << 16)
        | ((bytes[offset + 4] as u64) << 24)
        | ((bytes[offset + 5] as u64) << 32)
        | ((bytes[offset + 6] as u64) << 40)
        | ((bytes[offset + 7] as u64) << 48))
  } else if(shift == 8) {
    result = ((bytes[offset + 1] as u64)
        | ((bytes[offset + 2] as u64) << 8)
        | ((bytes[offset + 3] as u64) << 16)
        | ((bytes[offset + 4] as u64) << 24)
        | ((bytes[offset + 5] as u64) << 32)
        | ((bytes[offset + 6] as u64) << 40)
        | ((bytes[offset + 7] as u64) << 48)
        | ((bytes[offset + 8] as u64) << 56))
  }

  return result;
}
