import {
  clearStore,
  test,
  assert,
  afterEach
} from "matchstick-as/assembly/index";
import { logStore } from "matchstick-as/assembly/store";
import { processPayload } from "../src/mapping";
import { log } from "@graphprotocol/graph-ts";
import { Bytes, BigInt } from "@graphprotocol/graph-ts";

// Previously valid 2 tag bit length transaction
// test("Payload processing latest example", () => {
//   let payloadBytes = Bytes.fromHexString(
//     "0x0c2901090d413a313939310b423a326b6c0b433a3139300f443a31383831386c5fd2e9c3875bbbc8533fb99bfeefe7da0877ec424bf19d1b2831a9f84cf476016209c212221c2cd36745f8ecf16243c11ca6bbd507dea7a452beea7b0ac31093dabafdb9f1356a09055609b612006848a26c8bded1673259a391cb548013c6ea0640f2ff38f390917c15f0da9b8801010101c76430fea08e4b1ee3e427d55cc814386bb1ac0d63536e35928b120f5d4f7bd701010101"
//   ) as Bytes;
//   let submitter = "0x00";
//   let txHash = "0x00";
//
//   processPayload(submitter, payloadBytes, txHash);
//
//     // To Do add asserts
//
//
// });

// crates/oracle-encoder/examples/01-empty-blocknums.json
// 1 (SetBlockNumbersForNextEpoch): 0x00c9
// [
//   messages.empty_block_numbers(100),
// ]

afterEach(() => {
  clearStore();
});

test("(SetBlockNumbersForNextEpoch) EMPTY", () => {
  let payloadBytes = Bytes.fromHexString("0x00c9") as Bytes;
  let submitter = "0x00";
  let txHash = "0x00";

  processPayload(submitter, payloadBytes, txHash);

  assert.entityCount("Epoch", 100);
  assert.fieldEquals("GlobalState", "0", "latestValidEpoch", "100");
  assert.fieldEquals("Epoch", "100", "id", "100"); // assert that Epoch 100 exists
});

// crates/oracle-encoder/examples/02-register-networks-and-set-block-numbers-same-payload.json
// 1 (RegisterNetworks, SetBlockNumbersForNextEpoch): 0x030103034166ebb0afd80c906e2b0564e921c3feefa9a5ecb71e98e3c7b7e661515e87dc493d
// [
//   [  messages.add_networks(["A"]),
//      messages.set_block_numbers([15]),
//   ]
// ]

test("RegisterNetworks, SetBlockNumbersForNextEpoch)", () => {
  let payloadBytes = Bytes.fromHexString(
    "0x030103034166ebb0afd80c906e2b0564e921c3feefa9a5ecb71e98e3c7b7e661515e87dc493d"
  ) as Bytes;
  let submitter = "0x00";
  let txHash = "0x00";

  processPayload(submitter, payloadBytes, txHash);

  assert.entityCount("Epoch", 1);
  assert.entityCount("Network", 1);
  assert.entityCount("NetworkEpochBlockNumber", 1);
  assert.fieldEquals("Network", "A", "id", "A");
  assert.fieldEquals("Epoch", "1", "id", "1");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "id", "1-A");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "acceleration", "15");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "delta", "15");
  assert.fieldEquals("GlobalState", "0", "networkArrayHead", "A");

});

// crates/oracle-encoder/examples/03-register-networks-and-set-block-numbers.json
// 1 (RegisterNetworks): 0x0301030341
// 2 (SetBlockNumbersForNextEpoch): 0x0066ebb0afd80c906e2b0564e921c3feefa9a5ecb71e98e3c7b7e661515e87dc493d
// [
//   messages.add_networks(["A"]),
//   messages.set_block_numbers([15]),
// ]

test("(RegisterNetworks) -> (SetBlockNumbersForNextEpoch)", () => {
  let payloadBytes1 = Bytes.fromHexString("0x0301030341") as Bytes;
  let payloadBytes2 = Bytes.fromHexString(
    "0x0066ebb0afd80c906e2b0564e921c3feefa9a5ecb71e98e3c7b7e661515e87dc493d"
  ) as Bytes;
  let submitter = "0x00";
  let txHash = "0x00";

  processPayload(submitter, payloadBytes1, txHash); // Network registration

  assert.entityCount("Epoch", 0);
  assert.entityCount("Network", 1);
  assert.entityCount("NetworkEpochBlockNumber", 0);

  processPayload(submitter, payloadBytes2, txHash); // Acceleration

  assert.entityCount("Epoch", 1);
  assert.entityCount("Network", 1);
  assert.entityCount("NetworkEpochBlockNumber", 1);
  assert.fieldEquals("Network", "A", "id", "A");
  assert.fieldEquals("Epoch", "1", "id", "1");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "id", "1-A");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "acceleration", "15");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "delta", "15");
  assert.fieldEquals("GlobalState", "0", "networkArrayHead", "A");
});

// crates/oracle-encoder/examples/04-register-multiple-and-set-block-numbers-thrice.json
// 1 (RegisterNetworks, SetBlockNumbersForNextEpoch, SetBlockNumbersForNextEpoch, SetBlockNumbersForNextEpoch): 0x030109034103420343034466ebb0afd80c906e2b0564e921c3feefa9a5ecb71e98e3c7b7e661515e87dc4905090d110066ebb0afd80c906e2b0564e921c3feefa9a5ecb71e98e3c7b7e661515e87dc4915191d2166ebb0afd80c906e2b0564e921c3feefa9a5ecb71e98e3c7b7e661515e87dc4925292d31
// [
//   [
//     messages.add_networks(["A", "B", "C", "D"]),
//     messages.set_block_numbers([1,  2,  3,  4]),
//     messages.set_block_numbers([5,  6,  7,  8]),
//     messages.set_block_numbers([9, 10, 11, 12]),
//   ]
// ]

test("(RegisterNetworks, SetBlockNumbersForNextEpoch, SetBlockNumbersForNextEpoch, SetBlockNumbersForNextEpoch)", () => {
  let payloadBytes = Bytes.fromHexString(
    "0x030109034103420343034466ebb0afd80c906e2b0564e921c3feefa9a5ecb71e98e3c7b7e661515e87dc4905090d110066ebb0afd80c906e2b0564e921c3feefa9a5ecb71e98e3c7b7e661515e87dc4915191d2166ebb0afd80c906e2b0564e921c3feefa9a5ecb71e98e3c7b7e661515e87dc4925292d31"
  ) as Bytes;
  let submitter = "0x00";
  let txHash = "0x00";

  processPayload(submitter, payloadBytes, txHash);

  // To Do add asserts
});

// crates/oracle-encoder/examples/05-register-multiple-and-unregister.json
// 1 (RegisterNetworks, SetBlockNumbersForNextEpoch): 0x030109034103420343034466ebb0afd80c906e2b0564e921c3feefa9a5ecb71e98e3c7b7e661515e87dc4905090d11
// 2 (RegisterNetworks, SetBlockNumbersForNextEpoch): 0x0303030166ebb0afd80c906e2b0564e921c3feefa9a5ecb71e98e3c7b7e661515e87dc4915191d
// [
//   [
//     messages.add_networks(["A", "B", "C", "D"]),
//     messages.set_block_numbers([1,  2,  3,  4]),
//   ],
//   [
//     messages.remove_networks([1]),
//     messages.set_block_numbers([5, 6, 7]),
//   ]
// ]

test("(RegisterNetworks, SetBlockNumbersForNextEpoch) -> (RegisterNetworks, SetBlockNumbersForNextEpoch)", () => {
  let payloadBytes1 = Bytes.fromHexString(
    "0x030109034103420343034466ebb0afd80c906e2b0564e921c3feefa9a5ecb71e98e3c7b7e661515e87dc4905090d11"
  ) as Bytes;
  let payloadBytes2 = Bytes.fromHexString(
    "0x0303030166ebb0afd80c906e2b0564e921c3feefa9a5ecb71e98e3c7b7e661515e87dc4915191d"
  ) as Bytes;
  let submitter = "0x00";
  let txHash = "0x00";

  processPayload(submitter, payloadBytes1, txHash);
  processPayload(submitter, payloadBytes2, txHash);

  // To Do add asserts
});
