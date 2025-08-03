import { CrossChainEpochOracleCall, Log } from "../generated/DataEdge/DataEdge";
import { Bytes, BigInt, log } from "@graphprotocol/graph-ts";
import {
  MessageBlock,
  Payload,
  GlobalState,
  Network
} from "../generated/schema";
import {
  BytesReader,
  decodeI64,
  decodeU64,
  decodeString,
  decodeTags
} from "./decoding";
import {
  createOrUpdateNetworkEpochBlockNumber,
  MessageTag,
  getActiveNetworks,
  commitNetworkChanges,
  wipeNetworkList,
  nextEpochId,
  parseCalldata,
  isSubmitterAllowed,
  doesSubmitterHavePermission,
  getSafeExecutionContext,
  SafeExecutionContext,
  epochBlockNumberId
} from "./helpers";
import { StoreCache } from "./store-cache";
import { BIGINT_ZERO, BIGINT_ONE, PRELOADED_ALIASES } from "./constants";

export function handleLogCrossChainEpochOracle(event: Log): void {
  // this is used in deployments on networks that lack trace support, and needs to strip the calldata to only
  // get the actual payload data we care about, without the selector and argument descriptors
  let data = parseCalldata(event.params.data);
  let safeExecutionContext = getSafeExecutionContext(event);
  if(safeExecutionContext != null) {
    log.warning("SafeExecutionContext multisig address: {}, submitter: {}", [
      safeExecutionContext.multisigAddress.toHexString(),
      event.transaction.from.toHexString()
    ])
  }
  // Support for Multisend type of transactions ONLY for EventfulDataEdge impl
  let payloadId = [event.transaction.hash.toHexString(), event.logIndex.toString()].join("-")
  processPayload(
    safeExecutionContext != null ? safeExecutionContext.multisigAddress.toHexString() : event.transaction.from.toHexString(),
    data,
    payloadId,
    event.block.number
  );
}

export function handleCrossChainEpochOracle(
  call: CrossChainEpochOracleCall
): void {
  processPayload(
    call.from.toHexString(),
    call.inputs._payload,
    call.transaction.hash.toHexString(),
    call.block.number
  );
}

export function processPayload(
  submitter: string,
  payloadBytes: Bytes,
  payloadId: string,
  blockNumber: BigInt
): void {
  log.warning(
    "Processing payload. Submitter: {}, payloadId: {}, blockNumber: {}",
    [submitter, payloadId, blockNumber.toString()]
  );
  // Start the StoreCache
  let cache = new StoreCache();

  // This is the only thing not handled through the store cache since we want all
  // Payload entity to persist (to provide context for validity of the payload)
  let payload = new Payload(payloadId);
  payload.data = payloadBytes;
  payload.submitter = submitter;
  payload.valid = true;
  payload.createdAt = blockNumber;

  let reader = new BytesReader(payloadBytes);
  let blockIdx = 0;

  if (!isSubmitterAllowed(cache, payload.submitter, blockNumber)) {
    log.error(
      "Invalid submitter. Allowed addresses: {}. Submitter: {}. Avoiding payload",
      [cache.getGlobalState().permissionList.toString(), payload.submitter]
    );
    return;
  }

  while (reader.length() > 0) {
    let i = blockIdx.toString();
    log.warning("New message block (num. {}) with remaining data: {}", [
      i,
      reader.data().toHexString()
    ]);

    // Handle message block
    let messageBlock = cache.getMessageBlock([payloadId, i].join("-"));
    messageBlock.payload = payload.id;
    processMessageBlock(cache, messageBlock, reader, payload.submitter);
    if (!reader.ok) {
      log.error("Failed to process message block num. {}", [i]);
      payload.valid = false;
      payload.errorMessage = reader.errorMessage;
      payload.save();
      return;
    }

    log.warning("Finished processing message block num. {}", [i]);
    blockIdx++;
  }

  payload.save();
  cache.commitChanges();
  log.warning(
    "Processed payload. Submitter: {}, payloadId: {}, blockNumber: {}",
    [submitter, payloadId, blockNumber.toString()]
  );
}

export function processMessageBlock(
  cache: StoreCache,
  messageBlock: MessageBlock,
  reader: BytesReader,
  submitter: String
): void {
  let snapshot = reader.snapshot();
  let tags = decodeTags(reader);

  if (!reader.ok) {
    return;
  }

  log.warning("Tags to process: {}. Remaining reader length: {}", [
    tags.toString(),
    reader.length().toString()
  ]);

  for (let i = 0; i < tags.length && reader.ok && reader.length() > 0; i++) {
    let permissionRequired = MessageTag.toString(tags[i]);
    if (!doesSubmitterHavePermission(cache, submitter, permissionRequired)) {
      reader.fail(
        "Submitter {} doesn't have the required permissions to execute {}."
          .replace("{}", submitter)
          .replace("{}", MessageTag.toString(tags[i]))
      );
    }
    processMessage(cache, messageBlock, i, tags[i], reader);
  }
  messageBlock.data = reader.diff(snapshot);
}

// Finishes decoding the message, executes it, and finally returns the amount
// of bytes read.
export function processMessage(
  cache: StoreCache,
  messageBlock: MessageBlock,
  i: i32,
  tag: MessageTag,
  reader: BytesReader
): void {
  log.warning(
    "Processing new message with tag {}. The remaining payload is {}",
    [MessageTag.toString(tag), reader.data().toHexString()]
  );
  let id = [messageBlock.id, i.toString()].join("-");
  let snapshot = reader.snapshot();

  log.warning("Executing message {}", [MessageTag.toString(tag)]);
  if (tag == MessageTag.SetBlockNumbersForEpochMessage) {
    executeSetBlockNumbersForEpochMessage(
      cache,
      snapshot,
      reader,
      id,
      messageBlock
    );
  } else if (tag == MessageTag.CorrectEpochsMessage) {
    executeCorrectEpochsMessage(cache, snapshot, reader, id, messageBlock);
  } else if (tag == MessageTag.UpdateVersionsMessage) {
    executeUpdateVersionsMessage(cache, snapshot, reader, id, messageBlock);
  } else if (tag == MessageTag.RegisterNetworksMessage) {
    executeRegisterNetworksMessage(cache, snapshot, reader, id, messageBlock);
  } else if (tag == MessageTag.ChangePermissionsMessage) {
    executeChangePermissionsMessage(cache, snapshot, reader, id, messageBlock);
  } else if (tag == MessageTag.ResetStateMessage) {
    executeResetStateMessage(cache, snapshot, reader, id, messageBlock);
  } else if (tag == MessageTag.RegisterNetworksAndAliasesMessage) {
    executeRegisterNetworksAndAliasesMessage(cache, snapshot, reader, id, messageBlock);
  } else if (tag == MessageTag.CorrectLastEpochMessage) {
    executeCorrectLastEpochMessage(cache, snapshot, reader, id, messageBlock);
  } else {
    reader.fail(
      "Unknown message tag '{}'. This is most likely a bug!".replace(
        "{}",
        tag.toString()
      )
    );
    log.error("Unknown message tag '{}'. This is most likely a bug!", [
      MessageTag.toString(tag)
    ]);
    return;
  }
}

function executeSetBlockNumbersForEpochMessage(
  cache: StoreCache,
  snapshot: BytesReader,
  reader: BytesReader,
  id: String,
  messageBlock: MessageBlock
): void {
  log.warning("There are {} currently active networks", [
    cache.getGlobalState().activeNetworkCount.toString()
  ]);

  if (cache.getGlobalState().activeNetworkCount != 0) {
    executeNonEmptySetBlockNumbersForEpochMessage(
      cache,
      snapshot,
      reader,
      id,
      messageBlock
    );
  } else {
    reader.fail("Network list is empty. Can't set new epoch block numbers");
    // executeEmptySetBlockNumbersForEpochMessage(
    //   cache,
    //   snapshot,
    //   reader,
    //   id,
    //   messageBlock
    // );
  }
}

function executeNonEmptySetBlockNumbersForEpochMessage(
  cache: StoreCache,
  snapshot: BytesReader,
  reader: BytesReader,
  id: String,
  messageBlock: MessageBlock
): void {
  let globalState = cache.getGlobalState();
  let message = cache.getSetBlockNumbersForEpochMessage(id);
  message.block = messageBlock.id;

  let networks = getActiveNetworks(cache);

  let previousEpochNumber = parseInt(
    globalState.latestValidEpoch != null ? globalState.latestValidEpoch! : "0"
  ) as i32;
  let nextEpochID = nextEpochId(globalState, reader);
  let nextEpochNumber = nextEpochID.toI32();

  if (nextEpochNumber > previousEpochNumber + 1) {
    log.warning(
      "Next Epoch number is {}, but previous epoch number is {}. Creating empty epochs to fill the gaps",
      [nextEpochID.toString(), previousEpochNumber.toString()]
    );
    for (let i = previousEpochNumber + 1; i < nextEpochNumber; i++) {
      log.warning("Backfilling epochs. Creating epoch #{}", [i.toString()]);
      cache.getEpoch(BigInt.fromI32(i));
    }
  }

  let newEpoch = cache.getEpoch(nextEpochID);
  globalState.latestValidEpoch = newEpoch.id;

  let merkleRoot = reader.advance(32);
  message.merkleRoot = merkleRoot;
  log.warning("The Merkle root of the new epoch is {}", [
    merkleRoot.toHexString()
  ]);
  log.warning("Now decoding block updates: {}", [reader.data().toHexString()]);

  let accelerations: Array<BigInt> = [];
  for (let i = 0; i < globalState.activeNetworkCount; i++) {
    let acceleration = BigInt.fromI64(decodeI64(reader));
    if (!reader.ok) {
      log.warning("Failed to decode acceleration num. {}", [i.toString()]);
      return;
    }
    log.warning("Decoded acceleration num. {} with value {}", [
      i.toString(),
      acceleration.toString()
    ]);

    accelerations.push(acceleration);

    // Create new NetworkEpochBlockNumber and save it for negative delta checks
    let blockNumberEntity = createOrUpdateNetworkEpochBlockNumber(
      networks[i],
      newEpoch.epochNumber,
      acceleration,
      cache
    );
    // Check for negative delta
    if (blockNumberEntity.delta < BIGINT_ZERO) {
      reader.fail(
        "NetworkEpochBlockNumber {} experienced a negative delta. Delta: {}, Acceleration: {}"
          .replace("{}", blockNumberEntity.id)
          .replace("{}", blockNumberEntity.delta.toString())
          .replace("{}", blockNumberEntity.acceleration.toString())
      );
      return;
    }
  }

  log.warning("Successfully decocoded accelerations", []);
  message.accelerations = accelerations;
  message.data = reader.diff(snapshot);
}

// function executeEmptySetBlockNumbersForEpochMessage(
//   cache: StoreCache,
//   snapshot: BytesReader,
//   reader: BytesReader,
//   id: String,
//   messageBlock: MessageBlock
// ): void {
//   let globalState = cache.getGlobalState();
//   let message = cache.getSetBlockNumbersForEpochMessage(id);
//   message.block = messageBlock.id;
//
//   let emptyEpochCount = BigInt.fromU64(decodeU64(reader));
//   if (!reader.ok) {
//     return;
//   }
//
//   message.count = emptyEpochCount;
//
//   log.warning("BEFORE EPOCH LOOP, AMOUNT TO CREATE: {}", [
//     message.count!.toString()
//   ]);
//
//   for (let i = BIGINT_ZERO; i < message.count!; i += BIGINT_ONE) {
//     log.warning("EPOCH LOOP, CREATING EPOCH: {}", [i.toString()]);
//     let newEpoch = cache.getEpoch(nextEpochId(globalState, reader));
//     globalState.latestValidEpoch = newEpoch.id;
//   }
//   log.warning("AFTER EPOCH LOOP", []);
//
//   message.data = reader.diff(snapshot);
// }

function executeCorrectEpochsMessage(
  cache: StoreCache,
  snapshot: BytesReader,
  reader: BytesReader,
  id: String,
  messageBlock: MessageBlock
): void {
  // TODO.
}

function executeCorrectLastEpochMessage(
  cache: StoreCache,
  snapshot: BytesReader,
  reader: BytesReader,
  id: String,
  messageBlock: MessageBlock
): void {
  // 1. Get latest epoch from globalState
  let globalState = cache.getGlobalState();
  let latestEpochId = globalState.latestValidEpoch;
  if (!latestEpochId) {
    reader.fail("No epochs exist to correct");
    return;
  }
  
  // 2. Parse message
  let chainId = decodeString(reader);
  if (!reader.ok) {
    return;
  }
  let newBlockNumber = BigInt.fromU64(decodeU64(reader));
  if (!reader.ok) {
    return;
  }
  let merkleRoot = reader.advance(32);
  if (!reader.ok) {
    return;
  }
  
  // 3. Find and validate network
  if (!cache.isNetworkAlreadyRegistered(chainId)) {
    reader.fail("Invalid or removed network");
    return;
  }
  let network = cache.getNetwork(chainId);
  
  // 4. Find NetworkEpochBlockNumber for latest epoch
  let epochBlockId = epochBlockNumberId(latestEpochId!, network.id);
  if (!cache.hasNetworkEpochBlockNumber(epochBlockId)) {
    reader.fail("No block number found for network in latest epoch");
    return;
  }
  let epochBlock = cache.getNetworkEpochBlockNumber(epochBlockId);
  
  // 5. Store previous values for audit trail
  let correction = cache.getLastEpochCorrection(id + "-" + network.id);
  correction.message = id;
  correction.network = network.id;
  correction.epochNumber = BigInt.fromString(latestEpochId!);
  correction.previousBlockNumber = epochBlock.blockNumber;
  correction.newBlockNumber = newBlockNumber;
  
  // 6. Calculate previous and new acceleration/delta
  correction.previousAcceleration = epochBlock.acceleration;
  correction.previousDelta = epochBlock.delta;
  
  // Calculate new delta (difference from previous epoch)
  let prevEpochNumber = BigInt.fromString(latestEpochId!).minus(BIGINT_ONE);
  if (prevEpochNumber.gt(BIGINT_ZERO)) {
    let prevEpochBlockId = epochBlockNumberId(prevEpochNumber.toString(), network.id);
    let prevEpochBlock = cache.getNetworkEpochBlockNumber(prevEpochBlockId);
    if (prevEpochBlock) {
      let newDelta = newBlockNumber.minus(prevEpochBlock.blockNumber);
      let newAcceleration = newDelta.minus(prevEpochBlock.delta);
      
      correction.newDelta = newDelta;
      correction.newAcceleration = newAcceleration;
      
      // Update the epoch block with new values
      epochBlock.blockNumber = newBlockNumber;
      epochBlock.delta = newDelta;
      epochBlock.acceleration = newAcceleration;
    } else {
      // First epoch for this network
      correction.newDelta = newBlockNumber;
      correction.newAcceleration = newBlockNumber;
      
      epochBlock.blockNumber = newBlockNumber;
      epochBlock.delta = newBlockNumber;
      epochBlock.acceleration = newBlockNumber;
    }
  } else {
    // This is epoch 1, no previous epoch
    correction.newDelta = newBlockNumber;
    correction.newAcceleration = newBlockNumber;
    
    epochBlock.blockNumber = newBlockNumber;
    epochBlock.delta = newBlockNumber;
    epochBlock.acceleration = newBlockNumber;
  }
  
  // 7. Update merkle root on THIS message (not the original)
  let message = cache.getCorrectLastEpochMessage(id);
  message.block = messageBlock.id;
  message.newMerkleRoot = merkleRoot;
  message.data = reader.diff(snapshot);
  
  // 8. Update network's latest valid block number
  network.latestValidBlockNumber = epochBlock.id;
}

function executeUpdateVersionsMessage(
  cache: StoreCache,
  snapshot: BytesReader,
  reader: BytesReader,
  id: String,
  messageBlock: MessageBlock
): void {
  let globalState = cache.getGlobalState();
  let message = cache.getUpdateVersionsMessage(id);
  let version = decodeU64(reader);

  message.block = messageBlock.id;
  message.newVersion = version as i32;
  message.oldVersion = globalState.encodingVersion;
  message.data = reader.diff(snapshot);

  globalState.encodingVersion = version as i32;
}

function executeRegisterNetworksMessage(
  cache: StoreCache,
  snapshot: BytesReader,
  reader: BytesReader,
  id: String,
  messageBlock: MessageBlock
): void {
  let globalState = cache.getGlobalState();
  let message = cache.getRegisterNetworksMessage(id);
  let networks = getActiveNetworks(cache);
  let networksMapped = networks.map<Array<Network>>(element => [element]);
  let removedNetworks: Array<Network> = [];
  let idsToRemove: Array<i32> = [];

  // Get the number of networks to remove.
  let numRemovals = decodeU64(reader) as i32;
  if (!reader.ok) {
    return;
  }

  // now get all the removed network ids and apply the changes to the pre-loaded list
  for (let i = 0; i < numRemovals; i++) {
    let networkId = decodeU64(reader) as i32;
    // Besides checking that the decoding was successful, we must perform a
    // bounds check over the newly provided network ID.
    if (networkId >= networks.length) {
      reader.fail(
        "Tried deleting a network ID that is out of bounds. NetworkID decoded: {}. Network list length: {}."
          .replace("{}", networkId.toString())
          .replace("{}", networks.length.toString())
      );
    }
    if (!reader.ok) {
      return;
    }

    idsToRemove.push(networkId);
  }
  log.warning("ids to remove {}", [
    idsToRemove
      .map<String>(element => element.toString())
      .toString()
  ]);

  for (let i = 0; i < idsToRemove.length; i++) {
    let network = networksMapped[idsToRemove[i]][0];
    removedNetworks.push(network);
    networksMapped[idsToRemove[i]] = [];
  }
  log.warning("networks mapped {}", [
    networksMapped
      .map<String>(element => (element.length > 0 ? element[0].id : ""))
      .toString()
  ]);
  networksMapped = networksMapped.filter(element => element.length > 0);
  log.warning("networks mapped filtered {}", [
    networksMapped
      .map<String>(element => (element.length > 0 ? element[0].id : ""))
      .toString()
  ]);

  networks = networksMapped.flat();

  let numInsertions = decodeU64(reader) as i32;
  let numReinsertions = 0;
  if (!reader.ok) {
    return;
  }

  // now get all the add network strings
  for (let i = 0; i < numInsertions; i++) {
    let chainId = decodeString(reader);
    if (!reader.ok) {
      return;
    }

    if (!cache.isNetworkAlreadyRegistered(chainId)) {
      let network = cache.getNetwork(chainId);
      network.alias = PRELOADED_ALIASES.keys().includes(network.id) ? PRELOADED_ALIASES.get(network.id).toString() : ""
      if(network.removedAt == null) {
        network.addedAt = message.id;
      } else {
        numReinsertions += 1;
        network.lastUpdatedAt = message.id;
        network.removedAt = null; // unsetting to make sure that if the network existed before, it's no longer flagged as removed
      }
      networks.push(network);
    } else {
      reader.fail("Network {} is already registered.".replace("{}", chainId));
      return;
    }
  }

  globalState.activeNetworkCount += numInsertions;
  globalState.activeNetworkCount -= numRemovals;
  globalState.networkCount += numInsertions;
  globalState.networkCount -= numReinsertions;

  commitNetworkChanges(removedNetworks, networks, globalState, message.id);

  message.removeCount = BigInt.fromU64(numRemovals);
  message.addCount = BigInt.fromU64(numInsertions);
  message.block = messageBlock.id;
  message.data = reader.diff(snapshot);
}

function executeRegisterNetworksAndAliasesMessage(
  cache: StoreCache,
  snapshot: BytesReader,
  reader: BytesReader,
  id: String,
  messageBlock: MessageBlock
): void {
  let globalState = cache.getGlobalState();
  let message = cache.getRegisterNetworksMessage(id);
  let networks = getActiveNetworks(cache);
  let networksMapped = networks.map<Array<Network>>(element => [element]);
  let removedNetworks: Array<Network> = [];
  let idsToRemove: Array<i32> = [];

  // Get the number of networks to remove.
  let numRemovals = decodeU64(reader) as i32;
  if (!reader.ok) {
    return;
  }

  // now get all the removed network ids and apply the changes to the pre-loaded list
  for (let i = 0; i < numRemovals; i++) {
    let networkId = decodeU64(reader) as i32;
    // Besides checking that the decoding was successful, we must perform a
    // bounds check over the newly provided network ID.
    if (networkId >= networks.length) {
      reader.fail(
        "Tried deleting a network ID that is out of bounds. NetworkID decoded: {}. Network list length: {}."
          .replace("{}", networkId.toString())
          .replace("{}", networks.length.toString())
      );
    }
    if (!reader.ok) {
      return;
    }

    idsToRemove.push(networkId);
  }
  log.warning("ids to remove {}", [
    idsToRemove
      .map<String>(element => element.toString())
      .toString()
  ]);

  for (let i = 0; i < idsToRemove.length; i++) {
    let network = networksMapped[idsToRemove[i]][0];
    removedNetworks.push(network);
    networksMapped[idsToRemove[i]] = [];
  }
  log.warning("networks mapped {}", [
    networksMapped
      .map<String>(element => (element.length > 0 ? element[0].id : ""))
      .toString()
  ]);
  networksMapped = networksMapped.filter(element => element.length > 0);
  log.warning("networks mapped filtered {}", [
    networksMapped
      .map<String>(element => (element.length > 0 ? element[0].id : ""))
      .toString()
  ]);

  networks = networksMapped.flat();

  let numInsertions = decodeU64(reader) as i32;
  let numReinsertions = 0;
  if (!reader.ok) {
    return;
  }

  // now get all the add network strings
  for (let i = 0; i < numInsertions; i++) {
    // Get CAIP2 chain ID
    let chainId = decodeString(reader);
    if (!reader.ok) {
      return;
    }

    if (!cache.isNetworkAlreadyRegistered(chainId)) {
      let network = cache.getNetwork(chainId);
      if(network.removedAt == null) {
        network.addedAt = message.id;
      } else {
        numReinsertions += 1;
        network.lastUpdatedAt = message.id;
        network.removedAt = null; // unsetting to make sure that if the network existed before, it's no longer flagged as removed
      }
      // Get manifest alias for that CAIP2 id
      let alias = decodeString(reader);
      if (!reader.ok) {
        return;
      }

      network.alias = alias;
      networks.push(network);
    } else {
      reader.fail("Network {} is already registered.".replace("{}", chainId));
      return;
    }
  }

  globalState.activeNetworkCount += numInsertions;
  globalState.activeNetworkCount -= numRemovals;
  globalState.networkCount += numInsertions;
  globalState.networkCount -= numReinsertions;

  commitNetworkChanges(removedNetworks, networks, globalState, message.id);

  message.removeCount = BigInt.fromU64(numRemovals);
  message.addCount = BigInt.fromU64(numInsertions);
  message.block = messageBlock.id;
  message.data = reader.diff(snapshot);
}

function executeChangePermissionsMessage(
  cache: StoreCache,
  snapshot: BytesReader,
  reader: BytesReader,
  id: String,
  messageBlock: MessageBlock
): void {
  let globalState = cache.getGlobalState();
  let message = cache.getChangePermissionsMessage(id);
  let address = reader.advance(20).toHexString(); // address should always be 20 bytes

  // Get valid_through
  let validThrough = decodeU64(reader);
  if (!reader.ok) {
    return;
  }

  // Get the length of the new premissions list
  let permissionsListLength = decodeU64(reader) as i32;
  if (!reader.ok) {
    return;
  }

  let permissionEntry = cache.getPermissionListEntry(address);
  permissionEntry.validThrough = BigInt.fromU64(validThrough)
  let oldPermissionList = permissionEntry.permissions;
  let newPermissionList = new Array<String>();

  for (let i = 0; i < permissionsListLength; i++) {
    let permission = decodeU64(reader) as i32;
    if(MessageTag.isValid(permission)) {
      newPermissionList.push(MessageTag.toString(permission));
    } else {
      reader.fail(`Permission to add is invalid. Permission index: ${permission.toString()}`)
    }
  }
  permissionEntry.permissions = newPermissionList;

  message.block = messageBlock.id;
  message.address = address;
  message.validThrough = BigInt.fromU64(validThrough);
  message.oldPermissions = oldPermissionList;
  message.newPermissions = newPermissionList;
  message.data = reader.diff(snapshot);


  let list = globalState.permissionList;
  if(!list.includes(permissionEntry.id)) {
    list.push(permissionEntry.id);
  } else if (permissionsListLength == 0) {
    // this will remove the now empty permission entry from the list, preventing spam
    list.splice(list.indexOf(permissionEntry.id), 1)
  }
  globalState.permissionList = list;
}

function executeResetStateMessage(
  cache: StoreCache,
  snapshot: BytesReader,
  reader: BytesReader,
  id: String,
  messageBlock: MessageBlock
): void {
  let globalState = cache.getGlobalState();
  let message = cache.getResetStateMessage(id);
  let networks = getActiveNetworks(cache);
  // advance 1 just so that we can make this a processable message
  // otherwise if there's no more data, processing is halted, and
  // changing how that works at the MessageBlock level has unintended
  // consequences for the rest of the messages
  reader.advance(1);

  message.block = messageBlock.id;
  message.data = reader.diff(snapshot);

  wipeNetworkList(networks, message.id);

  cache.resetGlobalState();
}
