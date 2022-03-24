import { clearStore, test, assert } from "matchstick-as/assembly/index";
import { handleCrossChainEpochOracle } from "../src/mapping";
import { getGlobalState, getTags, decodePrefixVarIntU64 } from "../src/helpers";
import { log } from "@graphprotocol/graph-ts";

import { CrossChainEpochOracleCall } from "../generated/DataEdge/DataEdge";
import { Bytes, BigInt } from "@graphprotocol/graph-ts";
import { DataEdge, MessageBlock, Payload } from "../generated/schema";

test("U64 Decoding 0x2F", () => {
  // 23 -> [47] -> 0x2F

  let encoded = Bytes.fromHexString("0x2F"); // 23 u64

  let decoded = decodePrefixVarIntU64(changetype<Bytes>(encoded), 0);

  // Assert decoded value
  assert.bigIntEquals(BigInt.fromU64(decoded[0]), BigInt.fromU64(23 as u64));

  // Assert length of bytes read
  assert.bigIntEquals(BigInt.fromU64(decoded[1]), BigInt.fromU64(1 as u64));

  // Clear the store in order to start the next test off on a clean slate
  clearStore();
});
test("U64 Decoding 0xA28C", () => {
  // 9000 -> [162, 140] -> 0xA28C

  let encoded = Bytes.fromHexString("0xA28C"); // 9000 u64

  let decoded = decodePrefixVarIntU64(changetype<Bytes>(encoded), 0);

  // Assert decoded value

  assert.bigIntEquals(BigInt.fromU64(decoded[0]), BigInt.fromU64(9000 as u64));

  // Assert length of bytes read

  assert.bigIntEquals(BigInt.fromU64(decoded[1]), BigInt.fromU64(2 as u64));

  // Clear the store in order to start the next test off on a clean slate
  clearStore();
});
test("U64 Decoding 0x54AFB1", () => {
  // 1455594 -> [84, 175, 177] -> 0x54AFB1

  let encoded = Bytes.fromHexString("0x54AFB1"); // 1455594 u64

  let decoded = decodePrefixVarIntU64(changetype<Bytes>(encoded), 0);

  // Assert decoded value

  assert.bigIntEquals(
    BigInt.fromU64(decoded[0]),
    BigInt.fromU64(1455594 as u64)
  );

  // Assert length of bytes read

  assert.bigIntEquals(BigInt.fromU64(decoded[1]), BigInt.fromU64(3 as u64));

  // Clear the store in order to start the next test off on a clean slate
  clearStore();
});
test("U64 Decoding 0x58B1AF68", () => {
  // 109771541 -> [88, 177, 175, 104] -> 0x58B1AF68

  let encoded = Bytes.fromHexString("0x58B1AF68"); // 109771541 u64

  let decoded = decodePrefixVarIntU64(changetype<Bytes>(encoded), 0);

  // Assert decoded value

  assert.bigIntEquals(
    BigInt.fromU64(decoded[0]),
    BigInt.fromU64(109771541 as u64)
  );

  // Assert length of bytes read

  assert.bigIntEquals(BigInt.fromU64(decoded[1]), BigInt.fromU64(4 as u64));

  // Clear the store in order to start the next test off on a clean slate
  clearStore();
});
test("U64 Decoding 0xF0DF2264B5", () => {
  // 24345908991 -> [240, 223, 34, 100, 181] -> 0xF0DF2264B5

  let encoded = Bytes.fromHexString("0xF0DF2264B5"); // 24345908991 u64

  let decoded = decodePrefixVarIntU64(changetype<Bytes>(encoded), 0);

  // Assert decoded value

  assert.bigIntEquals(
    BigInt.fromU64(decoded[0]),
    BigInt.fromU64(24345908991 as u64)
  );

  // Assert length of bytes read

  assert.bigIntEquals(BigInt.fromU64(decoded[1]), BigInt.fromU64(5 as u64));

  // Clear the store in order to start the next test off on a clean slate
  clearStore();
});
test("U64 Decoding 0x608FF0EBC86E", () => {
  // 1903269233213 -> [96, 143, 240, 235, 200, 110] -> 0x608FF0EBC86E

  let encoded = Bytes.fromHexString("0x608FF0EBC86E"); // 1903269233213 u64

  let decoded = decodePrefixVarIntU64(changetype<Bytes>(encoded), 0);

  // Assert decoded value

  assert.bigIntEquals(
    BigInt.fromU64(decoded[0]),
    BigInt.fromU64(1903269233213 as u64)
  );

  // Assert length of bytes read

  assert.bigIntEquals(BigInt.fromU64(decoded[1]), BigInt.fromU64(6 as u64));

  // Clear the store in order to start the next test off on a clean slate
  clearStore();
});
test("U64 Decoding 0x40FD170EAC2DFE", () => {
  // 558944227176442 -> [64, 253, 23, 14, 172, 45, 254] -> 0x40FD170EAC2DFE

  let encoded = Bytes.fromHexString("0x40FD170EAC2DFE"); // 558944227176442 u64

  let decoded = decodePrefixVarIntU64(changetype<Bytes>(encoded), 0);

  // Assert decoded value
  assert.bigIntEquals(
    BigInt.fromU64(decoded[0]),
    BigInt.fromU64(558944227176442 as u64)
  );

  // Assert length of bytes read

  assert.bigIntEquals(BigInt.fromU64(decoded[1]), BigInt.fromU64(7 as u64));

  // Clear the store in order to start the next test off on a clean slate
  clearStore();
});
test("U64 Decoding 0x80FFFFFFFFFFFFFF", () => {
  // 72057594037927935 -> [128, 255, 255, 255, 255, 255, 255, 255] -> 0x80FFFFFFFFFFFFFF

  let encoded = Bytes.fromHexString("0x80FFFFFFFFFFFFFF"); // 72057594037927935 u64

  let decoded = decodePrefixVarIntU64(changetype<Bytes>(encoded), 0);

  // Assert decoded value

  assert.bigIntEquals(
    BigInt.fromU64(decoded[0]),
    BigInt.fromU64(72057594037927935 as u64)
  );

  // Assert length of bytes read

  assert.bigIntEquals(BigInt.fromU64(decoded[1]), BigInt.fromU64(8 as u64));

  // Clear the store in order to start the next test off on a clean slate
  clearStore();
});
test("U64 Decoding 0x00FFFFFFFFFFFFFFFF", () => {
  // 18446744073709551615 -> [0, 255, 255, 255, 255, 255, 255, 255, 255] 0x00FFFFFFFFFFFFFFFF

  let encoded = Bytes.fromHexString("0x00FFFFFFFFFFFFFFFF"); // 18446744073709551615 u64

  let decoded = decodePrefixVarIntU64(changetype<Bytes>(encoded), 0);

  // Assert decoded value

  assert.bigIntEquals(
    BigInt.fromU64(decoded[0]),
    BigInt.fromU64(18446744073709551615 as u64)
  );

  // Assert length of bytes read

  assert.bigIntEquals(BigInt.fromU64(decoded[1]), BigInt.fromU64(9 as u64));

  // Clear the store in order to start the next test off on a clean slate
  clearStore();
});
test("U64 Decoding InvalidFormat 0x0011", () => {
  // 18446744073709551615 -> [0, 255, 255, 255, 255, 255, 255, 255, 255] 0x00FFFFFFFFFFFFFFFF

  let encoded = Bytes.fromHexString("0x0011"); // 18446744073709551615 u64

  let decoded = decodePrefixVarIntU64(changetype<Bytes>(encoded), 0);

  // Assert decoded value (0 since invalid cases return 0 here)

  assert.bigIntEquals(BigInt.fromU64(decoded[0]), BigInt.fromU64(0 as u64));

  // Assert length of bytes read (0 since ivalid cases return 0 here.)

  assert.bigIntEquals(BigInt.fromU64(decoded[1]), BigInt.fromU64(0 as u64));

  // Clear the store in order to start the next test off on a clean slate
  clearStore();
});
test("U64 Decoding InvalidFormat 0x00", () => {
  // 18446744073709551615 -> [0, 255, 255, 255, 255, 255, 255, 255, 255] 0x00FFFFFFFFFFFFFFFF

  let encoded = Bytes.fromHexString("0x00"); // 18446744073709551615 u64

  let decoded = decodePrefixVarIntU64(changetype<Bytes>(encoded), 0);

  // Assert decoded value (0 since invalid cases return 0 here)

  assert.bigIntEquals(BigInt.fromU64(decoded[0]), BigInt.fromU64(0 as u64));

  // Assert length of bytes read (0 since ivalid cases return 0 here.)

  assert.bigIntEquals(BigInt.fromU64(decoded[1]), BigInt.fromU64(0 as u64));

  // Clear the store in order to start the next test off on a clean slate
  clearStore();
});
test("U64 Decoding InvalidFormat 0x00FFFFFFFFFFFFFF (1 byte less than valid)", () => {
  // 18446744073709551615 -> [0, 255, 255, 255, 255, 255, 255, 255, 255] 0x00FFFFFFFFFFFFFFFF

  let encoded = Bytes.fromHexString("0x00FFFFFFFFFFFFFF"); // 18446744073709551615 u64

  let decoded = decodePrefixVarIntU64(changetype<Bytes>(encoded), 0);

  // Assert decoded value (0 since invalid cases return 0 here)

  assert.bigIntEquals(BigInt.fromU64(decoded[0]), BigInt.fromU64(0 as u64));

  // Assert length of bytes read (0 since ivalid cases return 0 here.)

  assert.bigIntEquals(BigInt.fromU64(decoded[1]), BigInt.fromU64(0 as u64));

  // Clear the store in order to start the next test off on a clean slate
  clearStore();
});

test("I64 Decoding (U64 + ZigZag)", () => {
  //...
});
