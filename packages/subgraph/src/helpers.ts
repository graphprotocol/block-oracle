import { BigInt, Bytes, Address, log } from "@graphprotocol/graph-ts";
import { Log } from "../generated/DataEdge/DataEdge";
import {
  GlobalState,
  Epoch,
  NetworkEpochBlockNumber,
  Network,
  MultisigExecution
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
  ResetStateMessage
}

export namespace MessageTag {
  const tags = [
    "SetBlockNumbersForEpochMessage",
    "CorrectEpochsMessage",
    "UpdateVersionsMessage",
    "RegisterNetworksMessage",
    "ChangePermissionsMessage",
    "ResetStateMessage"
  ];
  export function toString(tag: MessageTag): string {
    return tags[tag];
  }
  export function isValid(tag: MessageTag): boolean {
    return tags.length > tag;
  }
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
  submitter: String
): boolean {
  let permissionList = cache.getGlobalState().permissionList;

  //double check that this works or whether we need to load entity.

  return permissionList.includes(submitter);
}

export function getMultisigFromEOAIfValid(
  cache: StoreCache,
  txHash: String,
  logIndex: BigInt,
  submitter: String
): String {
  let sub = submitter;
  let id = txHash.concat("-").concat(logIndex.minus(BIGINT_ONE).toString());
  log.warning("Trying to get multisig address. Current transaction hash: {}, LogIndex: {}. PreviousExecution supposed ID: {}", [txHash, logIndex.toString(), id]);
  let previousExecution = MultisigExecution.load(id);
  if (previousExecution != null) {
    log.warning(
      "Multisig execution detected. execution: {}, triggeringAddress: {}. submitter: {}",
      [previousExecution.id, previousExecution.triggeringAddress, submitter]
    );
    if (submitter == previousExecution.triggeringAddress) {
      let permissionList = cache.getGlobalState().permissionList;
      let multisigHasPermissions = permissionList.includes(
        previousExecution.multisigAddress
      );

      sub = previousExecution.multisigAddress;
    }
  }
  log.warning("Resulting submitter {}", [sub]);
  return sub;
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

function epochBlockNumberId(epochId: BigInt, networkId: string): string {
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
