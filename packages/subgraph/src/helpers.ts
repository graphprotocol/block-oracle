import {
  BigInt,
  ByteArray,
  Bytes,
  Address,
  log,
  ethereum,
  log,
  crypto
} from "@graphprotocol/graph-ts";
import { Log } from "../generated/DataEdge/DataEdge";
import {
  GlobalState,
  Epoch,
  NetworkEpochBlockNumber,
  Network
} from "../generated/schema";
import { BytesReader } from "./decoding";
import { EpochManager } from "../generated/DataEdge/EpochManager";
import { StoreCache } from "./store-cache";
import { BIGINT_ONE, BIGINT_ZERO, EPOCH_MANAGER_ADDRESS } from "./constants";

export enum MessageTag {
  SetBlockNumbersForEpochMessage = 0,
  CorrectEpochsMessage,
  UpdateVersionsMessage,
  RegisterNetworksMessage,
  ChangePermissionsMessage,
  ResetStateMessage,
  RegisterNetworksAndAliasesMessage,
  CorrectLastEpochMessage
}

export namespace MessageTag {
  const tags = [
    "SetBlockNumbersForEpochMessage",
    "CorrectEpochsMessage",
    "UpdateVersionsMessage",
    "RegisterNetworksMessage",
    "ChangePermissionsMessage",
    "ResetStateMessage",
    "RegisterNetworksAndAliasesMessage",
    "CorrectLastEpochMessage"
  ];
  export function toString(tag: MessageTag): string {
    return tags[tag];
  }
  export function isValid(tag: MessageTag): boolean {
    return tags.length > tag;
  }
}

let EVENT_NAME = "SafeMultiSigTransaction";
let EVENT_SIGNATURE =
  "SafeMultiSigTransaction(address,uint256,bytes,uint8,uint256,uint256,uint256,address,address,bytes,bytes)";
let EVENT_DATA_TYPES =
  "(address,uint256,bytes,uint8,uint256,uint256,uint256,address,address,bytes,bytes)";
let LOG_EVENT_SIGNATURE = "Log(bytes)"

// For some reason it's erroring when trying to parse the calldata
export class SafeExecutionContext {
  multisigAddress: Bytes;
}

export function nextEpochId(state: GlobalState, reader: BytesReader): BigInt {
  log.warning("[nextEpochId] Using EpochManager address: {}", [
    EPOCH_MANAGER_ADDRESS
  ]);
  let epochManager = EpochManager.bind(
    Address.fromString(EPOCH_MANAGER_ADDRESS)
  );
  let response = epochManager.try_currentEpoch(); // maybe add try_ version later
  log.warning("[nextEpochId] latestValidEpoch: {}, response: {}.", [
    state.latestValidEpoch ? state.latestValidEpoch! : "null",
    response.reverted ? "Contract call reverted" : response.value.toString()
  ]);
  if (response.reverted) {
    reader.fail(
      "currentEpoch transaction reverted. Can't read current epoch from EpochManager contract"
    );
  } else if (
    state.latestValidEpoch &&
    response.value.toString() == state.latestValidEpoch
  ) {
    reader.fail("currentEpoch is the same as the latestValidEpoch");
  }
  return response.reverted ? BIGINT_ZERO : response.value;
}

export function createOrUpdateNetworkEpochBlockNumber(
  network: Network,
  epochId: BigInt,
  acceleration: BigInt,
  cache: StoreCache
): NetworkEpochBlockNumber {
  let networkId = network.id;
  let id = epochBlockNumberId(epochId, networkId);
  let previousId = network.latestValidBlockNumber;

  let networkEpochBlockNumber = cache.getNetworkEpochBlockNumber(id);
  networkEpochBlockNumber.network = networkId;
  networkEpochBlockNumber.epoch = epochId.toString();
  networkEpochBlockNumber.epochNumber = epochId;
  networkEpochBlockNumber.acceleration = acceleration;

  if (previousId && cache.hasNetworkEpochBlockNumber(previousId)) {
    let previousNetworkEpochBlockNumber = cache.getNetworkEpochBlockNumber(
      previousId
    );
    networkEpochBlockNumber.delta = previousNetworkEpochBlockNumber.delta.plus(
      acceleration
    );
    networkEpochBlockNumber.blockNumber = previousNetworkEpochBlockNumber.blockNumber.plus(
      networkEpochBlockNumber.delta
    );
    networkEpochBlockNumber.previousBlockNumber = previousId;
  } else {
    networkEpochBlockNumber.delta = acceleration;
    networkEpochBlockNumber.blockNumber = networkEpochBlockNumber.delta;
  }

  network.latestValidBlockNumber = networkEpochBlockNumber.id;

  return networkEpochBlockNumber;
}

export function getActiveNetworks(cache: StoreCache): Array<Network> {
  let state = cache.getGlobalState();
  let networks = new Array<Network>();
  let nextId = state.networkArrayHead;

  while (nextId != null) {
    let network = cache.getNetwork(nextId!);
    let isActive = network.removedAt == null;
    if (isActive) {
      networks.push(network);
    }
    nextId = network.nextArrayElement;
  }

  assert(
    networks.length == state.activeNetworkCount,
    `Found ${networks.length} active networks but ${state.activeNetworkCount} were expected. This is a bug!`
  );
  return networks;
}

export function isSubmitterAllowed(
  cache: StoreCache,
  submitter: String,
  blockNumber: BigInt
): boolean {
  let permissionList = cache.getGlobalState().permissionList;
  let filteredPermissionList: Array<String> = []

  for(let i = 0; i < permissionList.length; i++) {
    let entity = cache.getPermissionListEntry(permissionList[i]);
    if(entity.validThrough == BIGINT_ZERO || entity.validThrough > blockNumber) {
      filteredPermissionList.push(permissionList[i])
    }
  }

  //double check that this works or whether we need to load entity.

  return filteredPermissionList.includes(submitter);
}

export function doesSubmitterHavePermission(
  cache: StoreCache,
  submitter: String,
  permissionRequired: String
): boolean {
  let permissionList = cache.getGlobalState().permissionList;
  let permissionListEntry = cache.getPermissionListEntry(submitter);

  return permissionListEntry.permissions.includes(permissionRequired);
}

// export function swapAndPop(index: u32, networks: Array<Network>): Network {
//   assert(
//     index < (networks.length as u32),
//     `Tried to pop network at index ${index.toString()} but ` +
//       `there are only ${networks.length.toString()} active networks. This is a bug!`
//   );
//
//   let tail = networks[networks.length - 1];
//   let elementToRemove = networks[index];
//
//   networks[index] = tail;
//   networks[networks.length - 1] = elementToRemove;
//
//   return networks.pop();
// }

export function commitNetworkChanges(
  removedNetworks: Array<Network>,
  newNetworksList: Array<Network>,
  state: GlobalState,
  messageId: String
): void {
  for (let i = 0; i < removedNetworks.length; i++) {
    removedNetworks[i].state = null;
    removedNetworks[i].nextArrayElement = null;
    removedNetworks[i].arrayIndex = null;
    removedNetworks[i].removedAt = messageId;
    removedNetworks[i].lastUpdatedAt = messageId;
  }

  for (let i = 0; i < newNetworksList.length; i++) {
    newNetworksList[i].state = state.id;
    newNetworksList[i].nextArrayElement =
      i < newNetworksList.length - 1 ? newNetworksList[i + 1].id : null;
    newNetworksList[i].arrayIndex = i;
    newNetworksList[i].lastUpdatedAt = messageId;
  }

  if (newNetworksList.length > 0) {
    state.networkArrayHead = newNetworksList[0].id;
  } else {
    state.networkArrayHead = null;
  }
  state.activeNetworkCount = newNetworksList.length;
}

export function epochBlockNumberId(epochId: BigInt, networkId: string): string {
  return [epochId.toString(), networkId].join("-");
}

export function parseCalldata(calldata: Bytes): Bytes {
  // hardcoded values to decode only the crossChainEpochOracle calldata
  // on the local development EventfulDataEdge contract
  let length = BigInt.fromUnsignedBytes(
    changetype<Bytes>(calldata.slice(36, 68).reverse())
  );
  return changetype<Bytes>(calldata.slice(68, 68 + length.toI32()));
}

export function wipeNetworkList(
  networks: Array<Network>,
  messageId: String
): void {
  for (let i = 0; i < networks.length; i++) {
    networks[i].state = null;
    networks[i].nextArrayElement = null;
    networks[i].arrayIndex = null;
    networks[i].removedAt = messageId;
    networks[i].lastUpdatedAt = messageId;
    networks[i].latestValidBlockNumber = null;
  }
}

// Loops through all logs in an event tx receipt
// Returns the Log that matches the event signature and logIndex
export function getEventFromReceipt(
  event: ethereum.Event,
  eventSignature: string,
  logIndex: BigInt
): ethereum.Log | null {
  let receipt = event.receipt;
  if (receipt === null) {
    log.error("Could not get tx receipt!", []);
    return null;
  }

  let desiredLog: ethereum.Log | null = null;

  let logs = receipt.logs;
  for (let i = 0; i < logs.length; i++) {
    if (logs[i].logIndex == logIndex) {
      let topics = logs[i].topics;

      if (isEventLog(topics[0], eventSignature)) {
        // maybe also check that the data contains the selector for the EBO 0xa1dce332
        // but we require the parsing of the calldata to work for that
        desiredLog = logs[i];
      } else if(isEventLog(topics[0], LOG_EVENT_SIGNATURE)) {
        // try with a previous value for multisend cases
        return getEventFromReceipt(event, eventSignature, logIndex.minus(BIGINT_ONE))
      }
    }
  }

  return desiredLog;
}

// Returns true if the topic corresponds to an event signature
function isEventLog(topic: Bytes, targetEventSignature: string): boolean {
  return topic == crypto.keccak256(Bytes.fromUTF8(targetEventSignature));
}

export function getSafeExecutionContext(
  event: ethereum.Event
): SafeExecutionContext | null {
  let log = getEventFromReceipt(
    event,
    EVENT_SIGNATURE,
    event.logIndex.minus(BIGINT_ONE)
  ); // if it's a safe execution, we need to search the previous logIndex
  if (log === null) return null;

  return parseSafeExecutionContext(log);
}

export function parseSafeExecutionContext(
  ethLog: ethereum.Log
): SafeExecutionContext | null {
  // Would be good to also parse the data to make sure the execution matches the
  // expected execution, i.e. that the data is actually an EBO message
  return {
    multisigAddress: ethLog.address
  };
}

// bytes helpers
export function numberToBytes(num: u64): ByteArray {
  return stripZeros(Bytes.fromU64(num).reverse());
}

export function bigIntToBytes(num: BigInt): Bytes {
  return Bytes.fromUint8Array(stripZeros(Bytes.fromBigInt(num).reverse()));
}

export function stripZeros(bytes: Uint8Array): ByteArray {
  let i = 0;
  while (i < bytes.length && bytes[i] == 0) {
    i++;
  }
  return Bytes.fromUint8Array(bytes.slice(i));
}

export function strip0xPrefix(input: string): string {
  return input.startsWith("0x") ? input.slice(2) : input;
}

// Pads a hex string with zeros to 64 characters
export function padZeros(input: string): string {
  let data = strip0xPrefix(input);
  return "0x".concat(data.padStart(64, "0"));
}

export function ensureEvenLength(input: string): string {
  if (input.length % 2 == 0) return input;
  return "0x0".concat(strip0xPrefix(input.toString()));
}
