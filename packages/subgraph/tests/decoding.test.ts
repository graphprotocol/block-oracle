import { test, assert, describe } from "matchstick-as/assembly/index";
import {
  BytesReader,
  decodeU64,
  decodeZigZag,
} from "../src/decoding";
import { Bytes, BigInt } from "@graphprotocol/graph-ts";
import { BIGINT_ZERO } from "../src/constants";

describe("U64", () => {
  test("Decoding 0x2f", () => {
    testU64("0x2f", 23 as u64);
  });
  test("Decoding 0xa28c", () => {
    testU64("0xa28c", 9000 as u64);
  });
  test("Decoding 0x54afb1", () => {
    testU64("0x54afb1", 1455594 as u64);
  });
  test("Decoding 0x58B1AF68", () => {
    testU64("0x58B1AF68", 109771541 as u64);
  });
  test("Decoding 0xF0DF2264B5", () => {
    testU64("0xF0DF2264B5", 24345908991 as u64);
  });
  test("Decoding 0x608FF0EBC86E", () => {
    testU64("0x608FF0EBC86E", 1903269233213 as u64);
  });
  test("Decoding 0x40FD170EAC2DFE", () => {
    testU64("0x40FD170EAC2DFE", 558944227176442 as u64);
  });
  test("Decoding 0x80FFFFFFFFFFFFFF", () => {
    testU64("0x80FFFFFFFFFFFFFF", 72057594037927935 as u64);
  });
  test("Decoding 0x00FFFFFFFFFFFFFFFF", () => {
    testU64("0x00FFFFFFFFFFFFFFFF", 18446744073709551615 as u64);
  });

  test("Decoding 0x0011 (invalid)", () => {
    testU64Invalid("0x0011");
  });
  test("Decoding 0x00 (invalid)", () => {
    testU64Invalid("0x00");
  });
  test("Decoding 0x00FFFFFFFFFFFFFF (invalid; 1 byte less than valid)", () => {
    testU64Invalid("0x00FFFFFFFFFFFFFF");
  });
});

describe("ZigZag", () => {
  test("Decoding 0", () => {
    testZigZag(0, 0);
  });
  test("Decoding 1", () => {
    testZigZag(1, -1);
  });
  test("Decoding 4294967294", () => {
    testZigZag(4294967294, 2147483647);
  });
  test("Decoding 4294967295", () => {
    testZigZag(4294967295, -2147483648);
  });
});

function testU64(hexString: string, expected: u64): void {
  let encoded = Bytes.fromHexString(hexString);
  let reader = new BytesReader(encoded);
  let decoded = decodeU64(reader);
  // Assert successful decoding.
  assert.assertTrue(reader.ok);
  // Assert decoded value.
  assert.bigIntEquals(BigInt.fromU64(expected), BigInt.fromU64(decoded));
  // Assert no remaining bytes.
  assert.bigIntEquals(BIGINT_ZERO, BigInt.fromU64(reader.length() as u64));
};

function testU64Invalid(hexString: string): void {
  let encoded = Bytes.fromHexString(hexString);
  let reader = new BytesReader(encoded);
  let decoded = decodeU64(reader);
  // Assert decoding field.
  assert.assertTrue(!reader.ok);
};

function testZigZag(unsigned: u64, expected: u64): void {
  assert.bigIntEquals(BigInt.fromI64(decodeZigZag(unsigned)), BigInt.fromI64(expected));
};
