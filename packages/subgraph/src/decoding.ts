import { Bytes, BigInt, log } from "@graphprotocol/graph-ts";
import { MessageTag } from "./helpers";
import {
  PREAMBLE_BIT_LENGTH,
  PREAMBLE_BYTE_LENGTH,
  TAG_BIT_LENGTH
} from "./constants";

export class BytesReader {
  bytes: Bytes;
  offset: u32;
  ok: boolean;
  errorMessage: string;

  constructor(bytes: Bytes) {
    this.bytes = bytes;
    this.offset = 0;
    this.ok = true;
    this.errorMessage = "";
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
      this.errorMessage = "Advance out of bounds";
      return Bytes.empty();
    }

    this.offset += n as u32;
    return changetype<Bytes>(this.bytes.slice(this.offset - n, this.offset));
  }

  peek(i: u32): u64 {
    if (i >= this.length()) {
      this.ok = false;
      this.errorMessage = "Peek out of bounds";
      return 0;
    } else {
      return this.bytes[this.offset + i] as u64;
    }
  }

  fail(reason: String): this {
    log.error(reason, []);
    this.ok = false;
    this.errorMessage = reason;
    return this;
  }
}

export function decodeTags(reader: BytesReader): Array<MessageTag> {
  let tags = new Array<MessageTag>();
  let bytes = reader.advance(PREAMBLE_BYTE_LENGTH);
  for (let i = 0; i < PREAMBLE_BIT_LENGTH / TAG_BIT_LENGTH; i++) {
    let tag = getTag(bytes, i);
    if (MessageTag.isValid(tag)) {
      tags.push(tag);
    } else {
      reader.fail("Decoded tag " + tag.toString() + " is invalid");
    }
  }
  return tags;
}

function getTag(bytes: Bytes, i: i32): MessageTag {
  return (
    (BigInt.fromUnsignedBytes(bytes).toI32() >> (i * TAG_BIT_LENGTH)) &
    (2 ** TAG_BIT_LENGTH - 1)
  );
}

// Returns the decoded i64 and the amount of bytes read.
export function decodeI64(reader: BytesReader): i64 {
  // First we need to decode the raw bytes into a u64 and check that it didn't error out
  // Then we need to decode the U64 with ZigZag.
  return decodeZigZag(decodeU64(reader));
}

// Returns the decoded u64 and the amount of bytes read.
export function decodeU64(reader: BytesReader): u64 {
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
      ((first >> 3) as u64) | (reader.peek(1) << 5) | (reader.peek(2) << 13);
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

export function decodeZigZag(input: u64): i64 {
  return ((input >> 1) ^ -(input & 1)) as i64;
}

export function decodeString(reader: BytesReader): string {
  let length = decodeU64(reader);
  if (!reader.ok) {
    return "";
  }

  return reader.advance(length as u32).toString();
}
