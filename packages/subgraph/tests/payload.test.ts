import {
  clearStore,
  test,
  assert,
  afterEach,
  beforeEach,
  createMockedFunction,
  describe
} from "matchstick-as/assembly/index";
import { processPayload, handleCrossChainEpochOracle } from "../src/mapping";
import { parseCalldata } from "../src/helpers";
import { EPOCH_MANAGER_ADDRESS, BIGINT_ONE, BIGINT_ZERO } from "../src/constants";
import { Bytes, BigInt, Address, ethereum } from "@graphprotocol/graph-ts";
import { Network, GlobalState, PermissionListEntry, Epoch, NetworkEpochBlockNumber } from "../generated/schema";
import { CrossChainEpochOracleCall } from "../generated/DataEdge/DataEdge";

let DATA_EDGE_ADDRESS = Address.fromString("0x0000000000000000000000000000000000000000");

function createCrossChainEpochOracleCall(
  from: Address,
  payload: Bytes,
  blockNumber: BigInt,
  txHash: string
): CrossChainEpochOracleCall {
  let call = changetype<CrossChainEpochOracleCall>(newMockCall());
  call.from = from;
  call.inputs._payload = payload;
  call.block.number = blockNumber;
  call.transaction.hash = Bytes.fromHexString(txHash);
  return call;
}

function newMockCall(): ethereum.Call {
  return changetype<ethereum.Call>(newMockEvent());
}

function newMockEvent(): ethereum.Event {
  let event = changetype<ethereum.Event>(new Entity());
  event.address = Address.fromString("0x0000000000000000000000000000000000000000");
  event.logIndex = BigInt.fromI32(0);
  event.transactionLogIndex = BigInt.fromI32(0);
  event.logType = null;
  event.block = changetype<ethereum.Block>(new Entity());
  event.block.baseFeePerGas = null;
  event.block.difficulty = BigInt.fromI32(0);
  event.block.gasLimit = BigInt.fromI32(0);
  event.block.gasUsed = BigInt.fromI32(0);
  event.block.hash = Bytes.fromHexString("0x0000000000000000000000000000000000000000000000000000000000000000");
  event.block.miner = Address.fromString("0x0000000000000000000000000000000000000000");
  event.block.nonce = null;
  event.block.number = BigInt.fromI32(0);
  event.block.parentHash = Bytes.fromHexString("0x0000000000000000000000000000000000000000000000000000000000000000");
  event.block.receiptsRoot = null;
  event.block.size = null;
  event.block.stateRoot = null;
  event.block.timestamp = BigInt.fromI32(0);
  event.block.totalDifficulty = null;
  event.block.transactionsRoot = null;
  event.block.unclesHash = null;
  event.transaction = changetype<ethereum.Transaction>(new Entity());
  event.transaction.from = Address.fromString("0x0000000000000000000000000000000000000000");
  event.transaction.gasLimit = BigInt.fromI32(0);
  event.transaction.gasPrice = BigInt.fromI32(0);
  event.transaction.hash = Bytes.fromHexString("0x0000000000000000000000000000000000000000000000000000000000000000");
  event.transaction.index = BigInt.fromI32(0);
  event.transaction.to = null;
  event.transaction.value = BigInt.fromI32(0);
  event.transaction.nonce = BigInt.fromI32(0);
  event.transactionHash = Bytes.fromHexString("0x0000000000000000000000000000000000000000000000000000000000000000");
  event.parameters = [];
  event.receipt = changetype<ethereum.TransactionReceipt>(new Entity());
  event.receipt.transactionHash = Bytes.fromHexString("0x0000000000000000000000000000000000000000000000000000000000000000");
  event.receipt.transactionIndex = BigInt.fromI32(0);
  event.receipt.blockHash = Bytes.fromHexString("0x0000000000000000000000000000000000000000000000000000000000000000");
  event.receipt.blockNumber = BigInt.fromI32(0);
  event.receipt.cumulativeGasUsed = BigInt.fromI32(0);
  event.receipt.gasUsed = BigInt.fromI32(0);
  event.receipt.contractAddress = null;
  event.receipt.logs = [];
  event.receipt.logsBloom = Bytes.fromHexString("0x00");
  event.receipt.root = null;
  event.receipt.status = null;
  return event;
}

class Entity extends Bytes {
  constructor() {
    super();
  }
}

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

test("ChangePermissions for new permissions", () => {
  let payloadBytes = Bytes.fromHexString("0x041234567890123456789012345678901234567890f7030d") as Bytes;
  let submitter = "0x0000000000000000000000000000000000000000";
  let txHash = "0x00";

  assert.notInStore("PermissionListEntry", "0x1234567890123456789012345678901234567890");
  
  processPayload(submitter, payloadBytes, txHash, BIGINT_ONE);
  
  assert.entityCount("Epoch", 0);
  
  // Check message composition and entities created based on it
  assert.entityCount("Payload", 1);
  assert.entityCount("MessageBlock", 1);
  assert.entityCount("SetBlockNumbersForEpochMessage", 0);
  assert.entityCount("RegisterNetworksMessage", 0);
  assert.entityCount("CorrectEpochsMessage", 0);
  assert.entityCount("UpdateVersionsMessage", 0);
  assert.entityCount("ChangePermissionsMessage", 1);
  assert.fieldEquals("PermissionListEntry", "0x1234567890123456789012345678901234567890", "id", "0x1234567890123456789012345678901234567890");
  assert.fieldEquals("PermissionListEntry", "0x1234567890123456789012345678901234567890", "permissions", "[RegisterNetworksAndAliasesMessage]");
});


test("ChangePermissions for new permissions and then updated permissions", () => {
  let payloadBytes = Bytes.fromHexString("0x041234567890123456789012345678901234567890f7030d") as Bytes;
  let payloadBytes2 = Bytes.fromHexString("0x041234567890123456789012345678901234567890f705090b") as Bytes;
  let submitter = "0x0000000000000000000000000000000000000000";
  let txHash = "0x00";
  let txHash2 = "0x01";

  assert.notInStore("PermissionListEntry", "0x1234567890123456789012345678901234567890");
  
  processPayload(submitter, payloadBytes, txHash, BIGINT_ONE);
  
  assert.entityCount("Epoch", 0);
  
  // Check message composition and entities created based on it
  assert.entityCount("Payload", 1);
  assert.entityCount("MessageBlock", 1);
  assert.entityCount("SetBlockNumbersForEpochMessage", 0);
  assert.entityCount("RegisterNetworksMessage", 0);
  assert.entityCount("CorrectEpochsMessage", 0);
  assert.entityCount("UpdateVersionsMessage", 0);
  assert.entityCount("ChangePermissionsMessage", 1);
  assert.fieldEquals("PermissionListEntry", "0x1234567890123456789012345678901234567890", "id", "0x1234567890123456789012345678901234567890");
  assert.fieldEquals("PermissionListEntry", "0x1234567890123456789012345678901234567890", "permissions", "[RegisterNetworksAndAliasesMessage]");

  processPayload(submitter, payloadBytes2, txHash2, BIGINT_ONE);
  assert.entityCount("Payload", 2);
  assert.entityCount("MessageBlock", 2);
  assert.entityCount("SetBlockNumbersForEpochMessage", 0);
  assert.entityCount("RegisterNetworksMessage", 0);
  assert.entityCount("CorrectEpochsMessage", 0);
  assert.entityCount("UpdateVersionsMessage", 0);
  assert.entityCount("ChangePermissionsMessage", 2);
  assert.fieldEquals("PermissionListEntry", "0x1234567890123456789012345678901234567890", "id", "0x1234567890123456789012345678901234567890");
  assert.fieldEquals("PermissionListEntry", "0x1234567890123456789012345678901234567890", "permissions", "[ChangePermissionsMessage, ResetStateMessage]");
});

test("Submitter invalid after removal of permissions", () => {
  let payloadBytes = Bytes.fromHexString(
    "0x030103034166ebb0afd80c906e2b0564e921c3feefa9a5ecb71e98e3c7b7e661515e87dc493d"
  ) as Bytes;
  let payloadBytes2 = Bytes.fromHexString(
    "0x0400000000000000000000000000000000000001000101"
  ) as Bytes;
  let submitter = "0x0000000000000000000000000000000000000100";
  let txHash = "0x00";
  let txHash2 = "0x01";
  let txHash3 = "0x02";

  // First transaction goes through
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
  assert.entityCount("ChangePermissionsMessage", 0);

  assert.fieldEquals("GlobalState", "0", "activeNetworkCount", "1");
  assert.fieldEquals("Network", "A", "id", "A");
  assert.fieldEquals("Epoch", "1", "id", "1");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "id", "1-A");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "acceleration", "15");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "delta", "15");
  assert.fieldEquals("GlobalState", "0", "networkArrayHead", "A");

  processPayload(submitter, payloadBytes2, txHash2, BIGINT_ONE);

  assert.entityCount("Epoch", 1);
  assert.entityCount("Network", 1);
  assert.entityCount("NetworkEpochBlockNumber", 1);

  assert.entityCount("Payload", 2);
  assert.entityCount("MessageBlock", 2);
  assert.entityCount("SetBlockNumbersForEpochMessage", 1);
  assert.entityCount("RegisterNetworksMessage", 1);
  assert.entityCount("CorrectEpochsMessage", 0);
  assert.entityCount("UpdateVersionsMessage", 0);
  assert.entityCount("ChangePermissionsMessage", 1);

  processPayload(submitter, payloadBytes, txHash3, BIGINT_ONE);

  assert.entityCount("Epoch", 1);
  assert.entityCount("Network", 1);
  assert.entityCount("NetworkEpochBlockNumber", 1);

  assert.entityCount("Payload", 2);
  assert.entityCount("MessageBlock", 2);
  assert.entityCount("SetBlockNumbersForEpochMessage", 1);
  assert.entityCount("RegisterNetworksMessage", 1);
  assert.entityCount("CorrectEpochsMessage", 0);
  assert.entityCount("UpdateVersionsMessage", 0);
  assert.entityCount("ChangePermissionsMessage", 1);

  assert.fieldEquals("GlobalState", "0", "activeNetworkCount", "1");
  assert.fieldEquals("Network", "A", "id", "A");
  assert.fieldEquals("Epoch", "1", "id", "1");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "id", "1-A");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "acceleration", "15");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "delta", "15");
  assert.fieldEquals("GlobalState", "0", "networkArrayHead", "A");
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

test("Wrong Permissions (No register network permission) with changePermissions", () => {
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

// [[
// 	{
// 		"add": [
// 			["eip155:20000000", "juanmanet"]
// 		],
// 		"message": "RegisterNetworksAndAliases",
// 		"remove": []
// 	},
// 	{
// 		"accelerations": [
// 		15
// 		],
// 		"merkleRoot": "0x66ebb0afd80c906e2b0564e921c3feefa9a5ecb71e98e3c7b7e661515e87dc49",
// 		"message": "SetBlockNumbersForNextEpoch"
// 	}
// ]
// ]

test("(RegisterNetworksAndAliases)", () => {
  let payloadBytes = Bytes.fromHexString("0x0601031f6569703135353a3230303030303030136a75616e6d616e6574") as Bytes;
  let submitter = "0x0000000000000000000000000000000000000000";
  let txHash = "0x00";

  processPayload(submitter, payloadBytes, txHash, BIGINT_ONE); // Network registration with aliases

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
  assert.fieldEquals("Network", "eip155:20000000", "alias", "juanmanet");
  assert.fieldEquals("Network", "eip155:20000000", "id", "eip155:20000000");

  // processPayload(submitter, payloadBytes2, txHash2, BIGINT_ONE); // Acceleration

  // assert.entityCount("Epoch", 1);
  // assert.entityCount("Network", 1);
  // assert.entityCount("NetworkEpochBlockNumber", 1);

  // // Check message composition and entities created based on it
  // assert.entityCount("Payload", 2);
  // assert.entityCount("MessageBlock", 2);
  // assert.entityCount("SetBlockNumbersForEpochMessage", 1);
  // assert.entityCount("RegisterNetworksMessage", 1);
  // assert.entityCount("CorrectEpochsMessage", 0);
  // assert.entityCount("UpdateVersionsMessage", 0);

  // assert.fieldEquals("Network", "A", "id", "A");
  // assert.fieldEquals("Epoch", "1", "id", "1");
  // assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "id", "1-A");
  // assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "acceleration", "15");
  // assert.fieldEquals("NetworkEpochBlockNumber", "1-A", "delta", "15");
  // assert.fieldEquals("GlobalState", "0", "networkArrayHead", "A");
  // assert.fieldEquals("Network", "A", "state", "0");
  // assert.fieldEquals("Network", "A", "arrayIndex", "0");
  // let networkA = Network.load("A")!;
  // assert.assertNull(networkA.nextArrayElement);
});

test("(RegisterNetworks, SetBlockNumbersForNextEpoch) -> CorrectLastEpoch", () => {
  // First, register a network and set block numbers for epoch 1
  // JSON: { "message": "RegisterNetworks", "add": ["A1"], "remove": [] }
  let registerPayloadBytes = Bytes.fromHexString("0x030103054131") as Bytes;
  let submitter = "0x0000000000000000000000000000000000000000"; // Zero address has all permissions in test
  let txHash1 = "0x01";

  processPayload(submitter, registerPayloadBytes, txHash1, BIGINT_ONE);

  // Set block numbers for epoch 1
  mockEpochNumber(1);
  // JSON: { "message": "SetBlockNumbersForNextEpoch", "merkleRoot": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef", "accelerations": [15] }
  let setBlockNumbersBytes = Bytes.fromHexString(
    "0x001234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef3d"
  ) as Bytes;
  let txHash2 = "0x02";
  processPayload(submitter, setBlockNumbersBytes, txHash2, BIGINT_ONE);

  // Verify initial state
  assert.entityCount("Epoch", 1);
  assert.entityCount("Network", 1);
  assert.entityCount("NetworkEpochBlockNumber", 1);
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A1", "blockNumber", "15");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A1", "acceleration", "15");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A1", "delta", "15");

  // Now send a CorrectLastEpoch message
  // JSON: { "message": "CorrectLastEpoch", "chainId": "A1", "blockNumber": 20, "merkleRoot": "0xabcd..." }
  let correctLastEpochBytes = Bytes.fromHexString(
    "0x0705413129abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"
  ) as Bytes;
  let txHash3 = "0x03";

  processPayload(submitter, correctLastEpochBytes, txHash3, BIGINT_ONE);

  // Verify correction was applied
  assert.entityCount("CorrectLastEpochMessage", 1);
  assert.entityCount("LastEpochCorrection", 1);
  
  // Check the correction message
  assert.fieldEquals(
    "CorrectLastEpochMessage",
    "0x03-0-0",
    "newMerkleRoot",
    "0xabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"
  );

  // Check the correction record
  assert.fieldEquals("LastEpochCorrection", "0x03-0-0-A1", "network", "A1");
  assert.fieldEquals("LastEpochCorrection", "0x03-0-0-A1", "epochNumber", "1");
  assert.fieldEquals("LastEpochCorrection", "0x03-0-0-A1", "previousBlockNumber", "15");
  assert.fieldEquals("LastEpochCorrection", "0x03-0-0-A1", "newBlockNumber", "20");
  assert.fieldEquals("LastEpochCorrection", "0x03-0-0-A1", "previousAcceleration", "15");
  assert.fieldEquals("LastEpochCorrection", "0x03-0-0-A1", "previousDelta", "15");
  assert.fieldEquals("LastEpochCorrection", "0x03-0-0-A1", "newAcceleration", "20");
  assert.fieldEquals("LastEpochCorrection", "0x03-0-0-A1", "newDelta", "20");

  // Verify the NetworkEpochBlockNumber was updated
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A1", "blockNumber", "20");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A1", "acceleration", "20");
  assert.fieldEquals("NetworkEpochBlockNumber", "1-A1", "delta", "20");
});

test("CorrectLastEpoch with no epochs should fail", () => {
  // Try to correct when no epochs exist
  // JSON: { "message": "CorrectLastEpoch", "chainId": "A1", "blockNumber": 20, "merkleRoot": "0xabcd..." }
  let correctLastEpochBytes = Bytes.fromHexString(
    "0x0705413129abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"
  ) as Bytes;
  let submitter = "0x0000000000000000000000000000000000000000"; // Zero address has all permissions in test
  let txHash = "0x01";

  processPayload(submitter, correctLastEpochBytes, txHash, BIGINT_ONE);

  // Verify the payload was marked as invalid
  assert.fieldEquals("Payload", "0x01", "valid", "false");
  assert.fieldEquals("Payload", "0x01", "errorMessage", "No epochs exist to correct");
  
  // No correction entities should be created
  assert.entityCount("CorrectLastEpochMessage", 0);
  assert.entityCount("LastEpochCorrection", 0);
});

test("CorrectLastEpoch with invalid network should fail", () => {
  // First create an epoch by setting up a network and creating an epoch
  mockEpochNumber(1);
  
  // Register a network first so we have proper state
  // JSON: { "message": "RegisterNetworks", "add": ["A1"], "remove": [] }
  let registerPayload = Bytes.fromHexString("0x030103054131") as Bytes;
  processPayload("0x0000000000000000000000000000000000000000", registerPayload, "setup-tx", BIGINT_ONE);
  
  // Create an epoch
  // JSON: { "message": "SetBlockNumbersForNextEpoch", "merkleRoot": "0x1234...", "accelerations": [15] }
  let epochPayload = Bytes.fromHexString("0x001234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef3d") as Bytes;
  processPayload("0x0000000000000000000000000000000000000000", epochPayload, "epoch-tx", BIGINT_ONE);

  // Try to correct a network that doesn't exist (using non-existent chain ID "XX")
  // JSON: { "message": "CorrectLastEpoch", "chainId": "XX", "blockNumber": 29, "merkleRoot": "0xabcd..." }
  let correctLastEpochBytes = Bytes.fromHexString(
    "0x070558583babcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"
  ) as Bytes;
  let submitter = "0x0000000000000000000000000000000000000000"; // Zero address has all permissions in test
  let txHash = "0x01";

  processPayload(submitter, correctLastEpochBytes, txHash, BIGINT_ONE);

  // Verify the payload was marked as invalid
  assert.fieldEquals("Payload", "0x01", "valid", "false");
  assert.fieldEquals("Payload", "0x01", "errorMessage", "Invalid or removed network");
});

test("CorrectLastEpoch with multiple epochs calculates delta correctly", () => {
  // Register a network
  // JSON: { "message": "RegisterNetworks", "add": ["A1"], "remove": [] }
  let registerPayloadBytes = Bytes.fromHexString("0x030103054131") as Bytes;
  let submitter = "0x0000000000000000000000000000000000000000"; // Zero address has all permissions in test
  processPayload(submitter, registerPayloadBytes, "0x01", BIGINT_ONE);

  // Set block numbers for epoch 1
  mockEpochNumber(1);
  // JSON: { "message": "SetBlockNumbersForNextEpoch", "merkleRoot": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef", "accelerations": [10] }
  let setBlockNumbersBytes1 = Bytes.fromHexString(
    "0x001234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef29"
  ) as Bytes;
  processPayload(submitter, setBlockNumbersBytes1, "0x02", BIGINT_ONE);

  // Set block numbers for epoch 2 (block 25, delta 15, acceleration 5)
  mockEpochNumber(2);
  // JSON: { "message": "SetBlockNumbersForNextEpoch", "merkleRoot": "0x2234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef", "accelerations": [5] }
  let setBlockNumbersBytes2 = Bytes.fromHexString(
    "0x002234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef15"
  ) as Bytes;
  processPayload(submitter, setBlockNumbersBytes2, "0x03", BIGINT_ONE);

  // Verify state before correction
  assert.fieldEquals("NetworkEpochBlockNumber", "2-A1", "blockNumber", "25");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-A1", "delta", "15");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-A1", "acceleration", "5");

  // Correct epoch 2 to block 30
  // JSON: { "message": "CorrectLastEpoch", "chainId": "A1", "blockNumber": 30, "merkleRoot": "0xabcd..." }
  let correctLastEpochBytes = Bytes.fromHexString(
    "0x070541313dabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"
  ) as Bytes;
  processPayload(submitter, correctLastEpochBytes, "0x04", BIGINT_ONE);

  // Check the correction record
  assert.fieldEquals("LastEpochCorrection", "0x04-0-0-A1", "previousBlockNumber", "25");
  assert.fieldEquals("LastEpochCorrection", "0x04-0-0-A1", "newBlockNumber", "30");
  assert.fieldEquals("LastEpochCorrection", "0x04-0-0-A1", "previousDelta", "15");
  assert.fieldEquals("LastEpochCorrection", "0x04-0-0-A1", "newDelta", "20"); // 30 - 10 = 20
  assert.fieldEquals("LastEpochCorrection", "0x04-0-0-A1", "previousAcceleration", "5");
  assert.fieldEquals("LastEpochCorrection", "0x04-0-0-A1", "newAcceleration", "10"); // 20 - 10 = 10

  // Verify the NetworkEpochBlockNumber was updated correctly
  assert.fieldEquals("NetworkEpochBlockNumber", "2-A1", "blockNumber", "30");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-A1", "delta", "20");
  assert.fieldEquals("NetworkEpochBlockNumber", "2-A1", "acceleration", "10");
});