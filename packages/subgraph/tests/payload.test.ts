import {
  clearStore,
  test,
  assert,
  afterEach,
  beforeEach,
  createMockedFunction
} from "matchstick-as/assembly/index";
import { processPayload } from "../src/mapping";
import { parseCalldata } from "../src/helpers";
import { EPOCH_MANAGER_ADDRESS, BIGINT_ONE } from "../src/constants";
import { Bytes, BigInt, Address, ethereum } from "@graphprotocol/graph-ts";
import { Network, GlobalState, PermissionListEntry } from "../generated/schema";

// Previously valid 2 tag bit length transaction
// test("Payload processing latest example", () => {
//   let payloadBytes = Bytes.fromHexString(
//     "0x0c2901090d413a313939310b423a326b6c0b433a3139300f443a31383831386c5fd2e9c3875bbbc8533fb99bfeefe7da0877ec424bf19d1b2831a9f84cf476016209c212221c2cd36745f8ecf16243c11ca6bbd507dea7a452beea7b0ac31093dabafdb9f1356a09055609b612006848a26c8bded1673259a391cb548013c6ea0640f2ff38f390917c15f0da9b8801010101c76430fea08e4b1ee3e427d55cc814386bb1ac0d63536e35928b120f5d4f7bd701010101"
//   ) as Bytes;
//   let submitter = "0x0000000000000000000000000000000000000000";
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

function mockEpochNumber(number: i32): void {
  createMockedFunction(
    Address.fromString(EPOCH_MANAGER_ADDRESS),
    "currentEpoch",
    "currentEpoch():(uint256)"
  )
    .withArgs([])
    .returns([ethereum.Value.fromSignedBigInt(BigInt.fromI32(number))]);
}

beforeEach(() => {
  mockEpochNumber(1);
});

afterEach(() => {
  clearStore();
});

test("parseCalldata", () => {
  let calldataBytes = Bytes.fromHexString(
    "0xa1dce3320000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000a0011223344556677889900000000000000000000000000000000000000000000"
  ) as Bytes;
  let expectedPayloadBytes = Bytes.fromHexString(
    "0x00112233445566778899"
  ) as Bytes;

  let parsedPayloadBytes = parseCalldata(calldataBytes);

  assert.bytesEquals(expectedPayloadBytes, parsedPayloadBytes);
});

test("Wrong Submitter", () => {
  let payloadBytes = Bytes.fromHexString("0x00c9") as Bytes;
  let submitter = "0x0000000000000000000000000000000000000001";
  let txHash = "0x00";

  processPayload(submitter, payloadBytes, txHash, BIGINT_ONE);

  assert.entityCount("Epoch", 0);

  // Check message composition and entities created based on it
  assert.entityCount("Payload", 0); // we shouldn't create payloads for wrong submitters
  assert.entityCount("MessageBlock", 0);
  assert.entityCount("SetBlockNumbersForEpochMessage", 0);
  assert.entityCount("RegisterNetworksMessage", 0);
  assert.entityCount("CorrectEpochsMessage", 0);
  assert.entityCount("UpdateVersionsMessage", 0);
  assert.entityCount("ChangePermissionsMessage", 0);

  let globalState = GlobalState.load("0")!
  assert.assertTrue(!globalState.permissionList.includes(submitter));
});

test("Wrong Permissions (No register network permission)", () => {
  let payloadBytes = Bytes.fromHexString("0x0301030341") as Bytes;
  let submitter = "0x0000000000000000000000000000000000000010";
  let txHash = "0x00";

  processPayload(submitter, payloadBytes, txHash, BIGINT_ONE);

  assert.entityCount("Epoch", 0);

  // Check message composition and entities created based on it
  assert.entityCount("Payload", 1);
  assert.entityCount("MessageBlock", 0);
  assert.entityCount("SetBlockNumbersForEpochMessage", 0);
  assert.entityCount("RegisterNetworksMessage", 0);
  assert.entityCount("CorrectEpochsMessage", 0);
  assert.entityCount("UpdateVersionsMessage", 0);
  assert.entityCount("ChangePermissionsMessage", 0);

  let globalState = GlobalState.load("0")!
  assert.assertTrue(globalState.permissionList.includes(submitter));
  let permissionEntry = PermissionListEntry.load(submitter)!
  assert.assertTrue(!permissionEntry.permissions.includes("RegisterNetworksMessage"));
});

test("(SetBlockNumbersForNextEpoch) EMPTY but invalid", () => {
  let payloadBytes = Bytes.fromHexString("0x00c900") as Bytes;
  let submitter = "0x0000000000000000000000000000000000000000";
  let txHash = "0x00";

  processPayload(submitter, payloadBytes, txHash, BIGINT_ONE);

  assert.entityCount("Epoch", 0);

  // Check message composition and entities created based on it
  assert.entityCount("Payload", 1);
  assert.entityCount("MessageBlock", 0);
  assert.entityCount("SetBlockNumbersForEpochMessage", 0);
  assert.entityCount("RegisterNetworksMessage", 0);
  assert.entityCount("CorrectEpochsMessage", 0);
  assert.entityCount("UpdateVersionsMessage", 0);
  assert.entityCount("ChangePermissionsMessage", 0);

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
  let submitter = "0x0000000000000000000000000000000000000000";
  let txHash = "0x00";

  processPayload(submitter, payloadBytes, txHash, BIGINT_ONE);

  assert.entityCount("Epoch", 1);
  assert.entityCount("Network", 1);
  assert.entityCount("NetworkEpochBlockNumber", 1);

  // Check message composition and entities created based on it
  assert.entityCount("Payload", 1);
  assert.entityCount("MessageBlock", 1);
  assert.entityCount("SetBlockNumbersForEpochMessage", 1);
  assert.entityCount("RegisterNetworksMessage", 1);
  assert.entityCount("CorrectEpochsMessage", 0);
  assert.entityCount("UpdateVersionsMessage", 0);

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
  let submitter = "0x0000000000000000000000000000000000000000";
  let txHash1 = "0x00";
  let txHash2 = "0x01";

  processPayload(submitter, payloadBytes1, txHash1, BIGINT_ONE); // Network registration

  assert.entityCount("Epoch", 0);
  assert.entityCount("Network", 1);
  assert.entityCount("NetworkEpochBlockNumber", 0);

  // Check message composition and entities created based on it
  assert.entityCount("Payload", 1);
  assert.entityCount("MessageBlock", 1);
  assert.entityCount("SetBlockNumbersForEpochMessage", 0);
  assert.entityCount("RegisterNetworksMessage", 1);
  assert.entityCount("CorrectEpochsMessage", 0);
  assert.entityCount("UpdateVersionsMessage", 0);

  assert.fieldEquals("GlobalState", "0", "activeNetworkCount", "1");

  processPayload(submitter, payloadBytes2, txHash2, BIGINT_ONE); // Acceleration

  assert.entityCount("Epoch", 1);
  assert.entityCount("Network", 1);
  assert.entityCount("NetworkEpochBlockNumber", 1);

  // Check message composition and entities created based on it
  assert.entityCount("Payload", 2);
  assert.entityCount("MessageBlock", 2);
  assert.entityCount("SetBlockNumbersForEpochMessage", 1);
  assert.entityCount("RegisterNetworksMessage", 1);
  assert.entityCount("CorrectEpochsMessage", 0);
  assert.entityCount("UpdateVersionsMessage", 0);

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

// Test Backfilling

test("(RegisterNetworks) -> (SetBlockNumbersForNextEpoch) -> epochs elapsing -> (SetBlockNumbersForNextEpoch)", () => {
  let payloadBytes1 = Bytes.fromHexString("0x0301030341") as Bytes;
  let payloadBytes2 = Bytes.fromHexString(
    "0x0066ebb0afd80c906e2b0564e921c3feefa9a5ecb71e98e3c7b7e661515e87dc493d"
  ) as Bytes;
  let payloadBytes3 = Bytes.fromHexString(
    "0x0066ebb0afd80c906e2b0564e921c3feefa9a5ecb71e98e3c7b7e661515e87dc493d"
  ) as Bytes;
  let submitter = "0x0000000000000000000000000000000000000000";
  let txHash1 = "0x00";
  let txHash2 = "0x01";
  let txHash3 = "0x02";

  processPayload(submitter, payloadBytes1, txHash1, BIGINT_ONE); // Network registration

  assert.entityCount("Epoch", 0);
  assert.entityCount("Network", 1);
  assert.entityCount("NetworkEpochBlockNumber", 0);

  // Check message composition and entities created based on it
  assert.entityCount("Payload", 1);
  assert.entityCount("MessageBlock", 1);
  assert.entityCount("SetBlockNumbersForEpochMessage", 0);
  assert.entityCount("RegisterNetworksMessage", 1);
  assert.entityCount("CorrectEpochsMessage", 0);
  assert.entityCount("UpdateVersionsMessage", 0);

  assert.fieldEquals("GlobalState", "0", "activeNetworkCount", "1");

  processPayload(submitter, payloadBytes2, txHash2, BIGINT_ONE); // Acceleration

  assert.entityCount("Epoch", 1);
  assert.entityCount("Network", 1);
  assert.entityCount("NetworkEpochBlockNumber", 1);

  // Check message composition and entities created based on it
  assert.entityCount("Payload", 2);
  assert.entityCount("MessageBlock", 2);
  assert.entityCount("SetBlockNumbersForEpochMessage", 1);
  assert.entityCount("RegisterNetworksMessage", 1);
  assert.entityCount("CorrectEpochsMessage", 0);
  assert.entityCount("UpdateVersionsMessage", 0);

  assert.fieldEquals("Network", "A", "id", "A");
  assert.fieldEquals("Epoch", "1", "id", "1");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "id", "1-A");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "acceleration", "15");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "delta", "15");
  assert.fieldEquals("GlobalState", "0", "networkArrayHead", "A");
  assert.fieldEquals("Network", "A", "state", "0");
  assert.fieldEquals("Network", "A", "arrayIndex", "0");

  mockEpochNumber(5);

  processPayload(submitter, payloadBytes3, txHash3, BIGINT_ONE); // Acceleration

  assert.entityCount("Epoch", 5);
  assert.entityCount("Network", 1);
  assert.entityCount("NetworkEpochBlockNumber", 2);

  // Check message composition and entities created based on it
  assert.entityCount("Payload", 3);
  assert.entityCount("MessageBlock", 3);
  assert.entityCount("SetBlockNumbersForEpochMessage", 2);
  assert.entityCount("RegisterNetworksMessage", 1);
  assert.entityCount("CorrectEpochsMessage", 0);
  assert.entityCount("UpdateVersionsMessage", 0);

  assert.fieldEquals("Network", "A", "id", "A");
  assert.fieldEquals("Epoch", "1", "id", "1");
  assert.fieldEquals("Epoch", "2", "id", "2");
  assert.fieldEquals("Epoch", "3", "id", "3");
  assert.fieldEquals("Epoch", "4", "id", "4");
  assert.fieldEquals("Epoch", "5", "id", "5");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "id", "1-A");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "acceleration", "15");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "delta", "15");
  assert.notInStore("NetworkEpochBlockNumber", "2-A");
  assert.notInStore("NetworkEpochBlockNumber", "3-A");
  assert.notInStore("NetworkEpochBlockNumber", "4-A");
  assert.fieldEquals("NetworkEpochBlockNumber", "5-A", "id", "5-A");
  assert.fieldEquals("NetworkEpochBlockNumber", "5-A", "acceleration", "15");
  assert.fieldEquals("NetworkEpochBlockNumber", "5-A", "delta", "30");
  assert.fieldEquals("GlobalState", "0", "networkArrayHead", "A");
  assert.fieldEquals("Network", "A", "state", "0");
  assert.fieldEquals("Network", "A", "arrayIndex", "0");
  let networkA = Network.load("A")!;
  assert.assertNull(networkA.nextArrayElement);
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
  let submitter = "0x0000000000000000000000000000000000000000";
  let txHash1 = "0x00";
  let txHash2 = "0x01";

  processPayload(submitter, payloadBytes1, txHash1, BIGINT_ONE);

  // Check counts
  assert.entityCount("Epoch", 1);
  assert.entityCount("Network", 4);
  assert.entityCount("NetworkEpochBlockNumber", 4);

  // Check message composition and entities created based on it
  assert.entityCount("Payload", 1);
  assert.entityCount("MessageBlock", 1);
  assert.entityCount("SetBlockNumbersForEpochMessage", 1);
  assert.entityCount("RegisterNetworksMessage", 1);
  assert.entityCount("CorrectEpochsMessage", 0);
  assert.entityCount("UpdateVersionsMessage", 0);

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

  mockEpochNumber(2);

  processPayload(submitter, payloadBytes2, txHash2, BIGINT_ONE);

  // Check counts
  assert.entityCount("Epoch", 2);
  assert.entityCount("Network", 4); // entity count won't change, but 1 would be inactive
  assert.entityCount("NetworkEpochBlockNumber", 7);

  // Check message composition and entities created based on it
  assert.entityCount("Payload", 2);
  assert.entityCount("MessageBlock", 2);
  assert.entityCount("SetBlockNumbersForEpochMessage", 2);
  assert.entityCount("RegisterNetworksMessage", 2);
  assert.entityCount("CorrectEpochsMessage", 0);
  assert.entityCount("UpdateVersionsMessage", 0);

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
  assert.fieldEquals("Network", "C", "arrayIndex", "1");
  assert.fieldEquals("Network", "D", "arrayIndex", "2");
  let networkB = Network.load("B")!;
  assert.assertNull(networkB.arrayIndex);
  assert.assertNull(networkB.state);

  assert.fieldEquals("GlobalState", "0", "networkArrayHead", "A");
  assert.fieldEquals("Network", "A", "nextArrayElement", "C");
  assert.fieldEquals("Network", "C", "nextArrayElement", "D");
  networkD = Network.load("D")!;
  assert.assertNull(networkD.nextArrayElement);

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
  assert.fieldEquals("NetworkEpochBlockNumber", "2-C", "acceleration", "6");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-C", "delta", "9");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-D", "acceleration", "7");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-D", "delta", "11");
});

test("(RegisterNetworks, SetBlockNumbersForNextEpoch), RESET, (RegisterNetworks, SetBlockNumbersForNextEpoch) ", () => {
  let payloadBytes1 = Bytes.fromHexString(
    "0x030103034166ebb0afd80c906e2b0564e921c3feefa9a5ecb71e98e3c7b7e661515e87dc493d"
  ) as Bytes;
  let payloadBytes2 = Bytes.fromHexString("0x0500") as Bytes;
  let submitter = "0x0000000000000000000000000000000000000000";
  let txHash1 = "0x00";
  let txHash2 = "0x01";
  let txHash3 = "0x02";

  processPayload(submitter, payloadBytes1, txHash1, BIGINT_ONE);

  assert.entityCount("Epoch", 1);
  assert.entityCount("Network", 1);
  assert.entityCount("NetworkEpochBlockNumber", 1);

  // Check message composition and entities created based on it
  assert.entityCount("Payload", 1);
  assert.entityCount("MessageBlock", 1);
  assert.entityCount("SetBlockNumbersForEpochMessage", 1);
  assert.entityCount("RegisterNetworksMessage", 1);
  assert.entityCount("CorrectEpochsMessage", 0);
  assert.entityCount("UpdateVersionsMessage", 0);

  assert.fieldEquals("GlobalState", "0", "activeNetworkCount", "1");
  assert.fieldEquals("Network", "A", "id", "A");
  assert.fieldEquals("Epoch", "1", "id", "1");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "id", "1-A");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "acceleration", "15");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "delta", "15");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "blockNumber", "15");
  assert.fieldEquals("GlobalState", "0", "networkArrayHead", "A");

  mockEpochNumber(2);
  processPayload(submitter, payloadBytes2, txHash2, BIGINT_ONE);

  mockEpochNumber(3);
  processPayload(submitter, payloadBytes1, txHash3, BIGINT_ONE);

  assert.fieldEquals("GlobalState", "0", "activeNetworkCount", "1");
  assert.fieldEquals("Network", "A", "id", "A");
  assert.fieldEquals("Epoch", "1", "id", "1");
  assert.fieldEquals("NetworkEpochBlockNumber", "3-A", "id", "3-A");
  assert.fieldEquals("NetworkEpochBlockNumber", "3-A", "acceleration", "15");
  assert.fieldEquals("NetworkEpochBlockNumber", "3-A", "delta", "15");
  assert.fieldEquals("NetworkEpochBlockNumber", "3-A", "blockNumber", "15");
  assert.fieldEquals("GlobalState", "0", "networkArrayHead", "A");
});

// // 5. Register(A,B,C,D) then SetBlocks(A,B,C,D) (same payload), then
// //    Unregister(B,D) and SetBlocks(A,C) (both on a different payload
// //    than the original)
//
// local messages = import 'messages.libsonnet';
//
// [
//   [
//     messages.add_networks(["A", "B", "C", "D"]),
//     messages.set_block_numbers([1,  2,  3,  4]),
//   ],
//   [
//     messages.remove_networks([1, 3]),
//     messages.set_block_numbers([5, 6]),
//   ]
// ]
//
// [sample: crates/oracle-encoder/examples/07-register-multiple-and-unregister-multiple]
// 1 (RegisterNetworks, SetBlockNumbersForNextEpoch): 0x030109034103420343034466ebb0afd80c906e2b0564e921c3feefa9a5ecb71e98e3c7b7e661515e87dc4905090d11
// 2 (RegisterNetworks, SetBlockNumbersForNextEpoch): 0x030503070166ebb0afd80c906e2b0564e921c3feefa9a5ecb71e98e3c7b7e661515e87dc491519

test("(RegisterNetworks, SetBlockNumbersForNextEpoch) -> (RegisterNetworks, SetBlockNumbersForNextEpoch) MULTIPLE UNREGISTERS", () => {
  let payloadBytes1 = Bytes.fromHexString(
    "0x030109034103420343034466ebb0afd80c906e2b0564e921c3feefa9a5ecb71e98e3c7b7e661515e87dc4905090d11"
  ) as Bytes;
  let payloadBytes2 = Bytes.fromHexString(
    "0x030503070166ebb0afd80c906e2b0564e921c3feefa9a5ecb71e98e3c7b7e661515e87dc491519"
  ) as Bytes;
  let submitter = "0x0000000000000000000000000000000000000000";
  let txHash1 = "0x00";
  let txHash2 = "0x01";

  processPayload(submitter, payloadBytes1, txHash1, BIGINT_ONE);

  // Check counts
  assert.entityCount("Epoch", 1);
  assert.entityCount("Network", 4);
  assert.entityCount("NetworkEpochBlockNumber", 4);

  // Check message composition and entities created based on it
  assert.entityCount("Payload", 1);
  assert.entityCount("MessageBlock", 1);
  assert.entityCount("SetBlockNumbersForEpochMessage", 1);
  assert.entityCount("RegisterNetworksMessage", 1);
  assert.entityCount("CorrectEpochsMessage", 0);
  assert.entityCount("UpdateVersionsMessage", 0);

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

  mockEpochNumber(2);

  processPayload(submitter, payloadBytes2, txHash2, BIGINT_ONE);

  // Check counts
  assert.entityCount("Epoch", 2);
  assert.entityCount("Network", 4); // entity count won't change, but 1 would be inactive
  assert.entityCount("NetworkEpochBlockNumber", 6);

  // Check message composition and entities created based on it
  assert.entityCount("Payload", 2);
  assert.entityCount("MessageBlock", 2);
  assert.entityCount("SetBlockNumbersForEpochMessage", 2);
  assert.entityCount("RegisterNetworksMessage", 2);
  assert.entityCount("CorrectEpochsMessage", 0);
  assert.entityCount("UpdateVersionsMessage", 0);

  assert.fieldEquals("GlobalState", "0", "activeNetworkCount", "2");

  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "id", "1-A");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-B", "id", "1-B");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-C", "id", "1-C");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-D", "id", "1-D");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-A", "id", "2-A");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-C", "id", "2-C");

  assert.notInStore("NetworkEpochBlockNumber", "2-B"); // 2-B shouldn't exist since it was removed from the list
  assert.notInStore("NetworkEpochBlockNumber", "2-D"); // 2-D shouldn't exist since it was removed from the list

  // Check network array
  assert.fieldEquals("Network", "A", "state", "0");
  assert.fieldEquals("Network", "C", "state", "0");
  assert.fieldEquals("Network", "A", "arrayIndex", "0");
  assert.fieldEquals("Network", "C", "arrayIndex", "1");
  let networkB = Network.load("B")!;
  assert.assertNull(networkB.arrayIndex);
  assert.assertNull(networkB.state);
  networkD = Network.load("D")!;
  assert.assertNull(networkB.arrayIndex);
  assert.assertNull(networkB.state);

  assert.fieldEquals("GlobalState", "0", "networkArrayHead", "A");
  assert.fieldEquals("Network", "A", "nextArrayElement", "C");
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
  assert.fieldEquals("NetworkEpochBlockNumber", "2-C", "acceleration", "6");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-C", "delta", "9");
});
