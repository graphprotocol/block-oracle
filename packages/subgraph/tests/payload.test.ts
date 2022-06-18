import {
  clearStore,
  test,
  assert,
  afterEach
} from "matchstick-as/assembly/index";
import { processPayload } from "../src/mapping";
import { Bytes, BigInt } from "@graphprotocol/graph-ts";
import { Network } from "../generated/schema";

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

  // Check message composition and entities created based on it
  assert.entityCount("Payload", 1);
  assert.entityCount("MessageBlock", 1);
  assert.entityCount("SetBlockNumbersForEpochMessage", 1);
  assert.entityCount("RegisterNetworksMessage", 0);
  assert.entityCount("CorrectEpochsMessage", 0);
  assert.entityCount("UpdateVersionsMessage", 0);

  assert.fieldEquals("GlobalState", "0", "activeNetworkCount", "0");
  assert.fieldEquals("GlobalState", "0", "latestValidEpoch", "100");
  assert.fieldEquals("Epoch", "100", "id", "100"); // assert that Epoch 100 exists

  assert.fieldEquals(
    "SetBlockNumbersForEpochMessage",
    "0x00-0-0",
    "id",
    "0x00-0-0"
  );
  assert.fieldEquals(
    "SetBlockNumbersForEpochMessage",
    "0x00-0-0",
    "block",
    "0x00-0"
  );

  assert.fieldEquals("MessageBlock", "0x00-0", "payload", "0x00");
  assert.fieldEquals("MessageBlock", "0x00-0", "data", "0x00c9");
  assert.fieldEquals("Payload", "0x00", "valid", "true");
});

test("(SetBlockNumbersForNextEpoch) EMPTY but invalid", () => {
  let payloadBytes = Bytes.fromHexString("0x00c900") as Bytes;
  let submitter = "0x00";
  let txHash = "0x00";

  processPayload(submitter, payloadBytes, txHash);

  assert.entityCount("Epoch", 0);

  // Check message composition and entities created based on it
  assert.entityCount("Payload", 1);
  assert.entityCount("MessageBlock", 0);
  assert.entityCount("SetBlockNumbersForEpochMessage", 0);
  assert.entityCount("RegisterNetworksMessage", 0);
  assert.entityCount("CorrectEpochsMessage", 0);
  assert.entityCount("UpdateVersionsMessage", 0);

  assert.fieldEquals("GlobalState", "0", "activeNetworkCount", "0");
  assert.fieldEquals("Payload", "0x00", "valid", "false");
});

// crates/oracle-encoder/examples/02-register-networks-and-set-block-numbers-same-payload.json
// 1 (RegisterNetworks, SetBlockNumbersForNextEpoch): 0x030103034166ebb0afd80c906e2b0564e921c3feefa9a5ecb71e98e3c7b7e661515e87dc493d
// [
//   [  messages.add_networks(["A"]),
//      messages.set_block_numbers([15]),
//   ]
// ]

test("(RegisterNetworks, SetBlockNumbersForNextEpoch)", () => {
  let payloadBytes = Bytes.fromHexString(
    "0x030103034166ebb0afd80c906e2b0564e921c3feefa9a5ecb71e98e3c7b7e661515e87dc493d"
  ) as Bytes;
  let submitter = "0x00";
  let txHash = "0x00";

  processPayload(submitter, payloadBytes, txHash);

  assert.entityCount("Epoch", 1);
  assert.entityCount("Network", 1);
  assert.entityCount("NetworkEpochBlockNumber", 1);

  // Check message composition and entities created based on it
  assert.entityCount("Payload", 1);
  assert.entityCount("MessageBlock", 1);
  // will need to double check why these interface based entities are being
  // improperly saved as the same entity type, and thus, breaking the entityCount
  // checks for these tests
  // assert.entityCount("SetBlockNumbersForEpochMessage", 1);
  // assert.entityCount("RegisterNetworksMessage", 1);
  // assert.entityCount("CorrectEpochsMessage", 0);
  // assert.entityCount("UpdateVersionsMessage", 0);

  assert.fieldEquals("GlobalState", "0", "activeNetworkCount", "1");
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
  let txHash1 = "0x00";
  let txHash2 = "0x01";

  processPayload(submitter, payloadBytes1, txHash1); // Network registration

  assert.entityCount("Epoch", 0);
  assert.entityCount("Network", 1);
  assert.entityCount("NetworkEpochBlockNumber", 0);

  // Check message composition and entities created based on it
  assert.entityCount("Payload", 1);
  assert.entityCount("MessageBlock", 1);
  // assert.entityCount("SetBlockNumbersForEpochMessage", 0);
  // assert.entityCount("RegisterNetworksMessage", 1);
  // assert.entityCount("CorrectEpochsMessage", 0);
  // assert.entityCount("UpdateVersionsMessage", 0);

  assert.fieldEquals("GlobalState", "0", "activeNetworkCount", "1");

  processPayload(submitter, payloadBytes2, txHash2); // Acceleration

  assert.entityCount("Epoch", 1);
  assert.entityCount("Network", 1);
  assert.entityCount("NetworkEpochBlockNumber", 1);

  // Check message composition and entities created based on it
  assert.entityCount("Payload", 2);
  assert.entityCount("MessageBlock", 2);
  // assert.entityCount("SetBlockNumbersForEpochMessage", 1);
  // assert.entityCount("RegisterNetworksMessage", 1);
  // assert.entityCount("CorrectEpochsMessage", 0);
  // assert.entityCount("UpdateVersionsMessage", 0);

  assert.fieldEquals("Network", "A", "id", "A");
  assert.fieldEquals("Epoch", "1", "id", "1");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "id", "1-A");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "acceleration", "15");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "delta", "15");
  assert.fieldEquals("GlobalState", "0", "networkArrayHead", "A");
  assert.fieldEquals("Network", "A", "state", "0");
  assert.fieldEquals("Network", "A", "arrayIndex", "0");
  let networkA = Network.load("A")!;
  assert.assertNull(networkA.nextArrayElement);
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

  // Check counts
  assert.entityCount("Epoch", 3);
  assert.entityCount("Network", 4);
  assert.entityCount("NetworkEpochBlockNumber", 12);

  // Check message composition and entities created based on it
  assert.entityCount("Payload", 1);
  assert.entityCount("MessageBlock", 2);
  // assert.entityCount("SetBlockNumbersForEpochMessage", 3);
  // assert.entityCount("RegisterNetworksMessage", 1);
  // assert.entityCount("CorrectEpochsMessage", 0);
  // assert.entityCount("UpdateVersionsMessage", 0);

  assert.fieldEquals("GlobalState", "0", "activeNetworkCount", "4");

  // Check entities created make sense (ids)
  assert.fieldEquals("Network", "A", "id", "A");
  assert.fieldEquals("Network", "B", "id", "B");
  assert.fieldEquals("Network", "C", "id", "C");
  assert.fieldEquals("Network", "D", "id", "D");

  assert.fieldEquals("Epoch", "1", "id", "1");
  assert.fieldEquals("Epoch", "2", "id", "2");
  assert.fieldEquals("Epoch", "3", "id", "3");

  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "id", "1-A");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-A", "id", "2-A");
  assert.fieldEquals("NetworkEpochBlockNumber", "3-A", "id", "3-A");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-B", "id", "1-B");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-B", "id", "2-B");
  assert.fieldEquals("NetworkEpochBlockNumber", "3-B", "id", "3-B");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-C", "id", "1-C");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-C", "id", "2-C");
  assert.fieldEquals("NetworkEpochBlockNumber", "3-C", "id", "3-C");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-D", "id", "1-D");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-D", "id", "2-D");
  assert.fieldEquals("NetworkEpochBlockNumber", "3-D", "id", "3-D");

  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "network", "A");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-A", "network", "A");
  assert.fieldEquals("NetworkEpochBlockNumber", "3-A", "network", "A");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-B", "network", "B");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-B", "network", "B");
  assert.fieldEquals("NetworkEpochBlockNumber", "3-B", "network", "B");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-C", "network", "C");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-C", "network", "C");
  assert.fieldEquals("NetworkEpochBlockNumber", "3-C", "network", "C");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-D", "network", "D");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-D", "network", "D");
  assert.fieldEquals("NetworkEpochBlockNumber", "3-D", "network", "D");

  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "epoch", "1");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-A", "epoch", "2");
  assert.fieldEquals("NetworkEpochBlockNumber", "3-A", "epoch", "3");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-B", "epoch", "1");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-B", "epoch", "2");
  assert.fieldEquals("NetworkEpochBlockNumber", "3-B", "epoch", "3");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-C", "epoch", "1");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-C", "epoch", "2");
  assert.fieldEquals("NetworkEpochBlockNumber", "3-C", "epoch", "3");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-D", "epoch", "1");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-D", "epoch", "2");
  assert.fieldEquals("NetworkEpochBlockNumber", "3-D", "epoch", "3");

  // Check accelerations and deltas make sense
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "acceleration", "1");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "delta", "1");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-B", "acceleration", "2");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-B", "delta", "2");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-C", "acceleration", "3");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-C", "delta", "3");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-D", "acceleration", "4");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-D", "delta", "4");

  assert.fieldEquals("NetworkEpochBlockNumber", "2-A", "acceleration", "5");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-A", "delta", "6");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-B", "acceleration", "6");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-B", "delta", "8");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-C", "acceleration", "7");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-C", "delta", "10");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-D", "acceleration", "8");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-D", "delta", "12");

  assert.fieldEquals("NetworkEpochBlockNumber", "3-A", "acceleration", "9");
  assert.fieldEquals("NetworkEpochBlockNumber", "3-A", "delta", "15");
  assert.fieldEquals("NetworkEpochBlockNumber", "3-B", "acceleration", "10");
  assert.fieldEquals("NetworkEpochBlockNumber", "3-B", "delta", "18");
  assert.fieldEquals("NetworkEpochBlockNumber", "3-C", "acceleration", "11");
  assert.fieldEquals("NetworkEpochBlockNumber", "3-C", "delta", "21");
  assert.fieldEquals("NetworkEpochBlockNumber", "3-D", "acceleration", "12");
  assert.fieldEquals("NetworkEpochBlockNumber", "3-D", "delta", "24");

  // Check network array
  assert.fieldEquals("Network", "A", "state", "0");
  assert.fieldEquals("Network", "B", "state", "0");
  assert.fieldEquals("Network", "C", "state", "0");
  assert.fieldEquals("Network", "D", "state", "0");
  assert.fieldEquals("Network", "A", "arrayIndex", "0");
  assert.fieldEquals("Network", "B", "arrayIndex", "1");
  assert.fieldEquals("Network", "C", "arrayIndex", "2");
  assert.fieldEquals("Network", "D", "arrayIndex", "3");

  assert.fieldEquals("GlobalState", "0", "networkArrayHead", "A");
  assert.fieldEquals("Network", "A", "nextArrayElement", "B");
  assert.fieldEquals("Network", "B", "nextArrayElement", "C");
  assert.fieldEquals("Network", "C", "nextArrayElement", "D");
  let networkD = Network.load("D")!;
  assert.assertNull(networkD.nextArrayElement);
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
  let txHash1 = "0x00";
  let txHash2 = "0x01";

  processPayload(submitter, payloadBytes1, txHash1);

  // Check counts
  assert.entityCount("Epoch", 1);
  assert.entityCount("Network", 4);
  assert.entityCount("NetworkEpochBlockNumber", 4);

  // Check message composition and entities created based on it
  assert.entityCount("Payload", 1);
  assert.entityCount("MessageBlock", 1);
  // assert.entityCount("SetBlockNumbersForEpochMessage", 1);
  // assert.entityCount("RegisterNetworksMessage", 1);
  // assert.entityCount("CorrectEpochsMessage", 0);
  // assert.entityCount("UpdateVersionsMessage", 0);

  assert.fieldEquals("GlobalState", "0", "activeNetworkCount", "4");

  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "id", "1-A");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-B", "id", "1-B");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-C", "id", "1-C");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-D", "id", "1-D");

  // Check network array
  assert.fieldEquals("Network", "A", "state", "0");
  assert.fieldEquals("Network", "B", "state", "0");
  assert.fieldEquals("Network", "C", "state", "0");
  assert.fieldEquals("Network", "D", "state", "0");
  assert.fieldEquals("Network", "A", "arrayIndex", "0");
  assert.fieldEquals("Network", "B", "arrayIndex", "1");
  assert.fieldEquals("Network", "C", "arrayIndex", "2");
  assert.fieldEquals("Network", "D", "arrayIndex", "3");

  assert.fieldEquals("GlobalState", "0", "networkArrayHead", "A");
  assert.fieldEquals("Network", "A", "nextArrayElement", "B");
  assert.fieldEquals("Network", "B", "nextArrayElement", "C");
  assert.fieldEquals("Network", "C", "nextArrayElement", "D");
  let networkD = Network.load("D")!;
  assert.assertNull(networkD.nextArrayElement);

  processPayload(submitter, payloadBytes2, txHash2);

  // Check counts
  assert.entityCount("Epoch", 2);
  assert.entityCount("Network", 4); // entity count won't change, but 1 would be inactive
  assert.entityCount("NetworkEpochBlockNumber", 7);

  // Check message composition and entities created based on it
  assert.entityCount("Payload", 2);
  assert.entityCount("MessageBlock", 2);
  // assert.entityCount("SetBlockNumbersForEpochMessage", 2);
  // assert.entityCount("RegisterNetworksMessage", 2);
  // assert.entityCount("CorrectEpochsMessage", 0);
  // assert.entityCount("UpdateVersionsMessage", 0);

  assert.fieldEquals("GlobalState", "0", "activeNetworkCount", "3");

  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "id", "1-A");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-B", "id", "1-B");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-C", "id", "1-C");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-D", "id", "1-D");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-A", "id", "2-A");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-C", "id", "2-C");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-D", "id", "2-D");

  assert.notInStore("NetworkEpochBlockNumber", "2-B"); // 2-B shouldn't exist since it was removed from the list

  // Check network array
  assert.fieldEquals("Network", "A", "state", "0");
  assert.fieldEquals("Network", "C", "state", "0");
  assert.fieldEquals("Network", "D", "state", "0");
  assert.fieldEquals("Network", "A", "arrayIndex", "0");
  assert.fieldEquals("Network", "D", "arrayIndex", "1"); // D takes the places of B, since it's swapAndPop
  assert.fieldEquals("Network", "C", "arrayIndex", "2");
  let networkB = Network.load("B")!;
  assert.assertNull(networkB.arrayIndex);
  assert.assertNull(networkB.state);

  assert.fieldEquals("GlobalState", "0", "networkArrayHead", "A");
  assert.fieldEquals("Network", "A", "nextArrayElement", "D");
  assert.fieldEquals("Network", "D", "nextArrayElement", "C");
  let networkC = Network.load("C")!;
  assert.assertNull(networkC.nextArrayElement);

  // Check accelerations and deltas make sense
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "acceleration", "1");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "delta", "1");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-B", "acceleration", "2");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-B", "delta", "2");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-C", "acceleration", "3");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-C", "delta", "3");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-D", "acceleration", "4");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-D", "delta", "4");

  assert.fieldEquals("NetworkEpochBlockNumber", "2-A", "acceleration", "5");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-A", "delta", "6");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-D", "acceleration", "6"); // D and C look like swapping places, since it's swapAndPop, and D takes the place of B
  assert.fieldEquals("NetworkEpochBlockNumber", "2-D", "delta", "10");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-C", "acceleration", "7");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-C", "delta", "10");
});
