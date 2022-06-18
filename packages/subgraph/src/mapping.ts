import { CrossChainEpochOracleCall, Log } from "../generated/DataEdge/DataEdge";
import { Bytes, BigInt, log } from "@graphprotocol/graph-ts";
import {
  SetBlockNumbersForEpochMessage,
  CorrectEpochsMessage,
  UpdateVersionsMessage,
  RegisterNetworksMessage,
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
  swapAndPop,
  commitNetworkChanges,
  nextEpochId
} from "./helpers";
import { StoreCache } from "./store-cache";
import { BIGINT_ZERO, BIGINT_ONE } from "./constants";

export function handleLogCrossChainEpochOracle(event: Log): void {
  processPayload(
    event.transaction.from.toHexString(),
    event.params.data,
    event.transaction.hash.toHexString()
  );
}

export function handleCrossChainEpochOracle(
  call: CrossChainEpochOracleCall
): void {
  processPayload(
    call.transaction.from.toHexString(),
    call.inputs._payload,
    call.transaction.hash.toHexString()
  );
}

export function processPayload(
  submitter: string,
  payloadBytes: Bytes,
  txHash: string
): void {
  // Start the StoreCache
  let cache = new StoreCache();

  // This is the only thing not handled through the store cache since we want all
  // Payload entity to persist (to provide context for validity of the payload)
  let payload = new Payload(txHash);
  payload.data = payloadBytes;
  payload.submitter = submitter;
  payload.valid = true;
  payload.save();

  let reader = new BytesReader(payloadBytes);
  let blockIdx = 0;

  while (reader.length() > 0) {
    let i = blockIdx.toString();
    log.warning("New message block (num. {}) with remaining data: {}", [
      i,
      reader.data().toHexString()
    ]);

    // Handle message block
    let messageBlock = cache.getMessageBlock([txHash, i].join("-"));
    messageBlock.payload = payload.id;
    processMessageBlock(cache, messageBlock, reader);
    if (!reader.ok) {
      log.error("Failed to process message block num. {}", [i]);
      payload.valid = false;
      payload.save();
      return;
    }

    log.warning("Finished processing message block num. {}", [i]);
    blockIdx++;
  }

  cache.commitChanges();
}

export function processMessageBlock(
  cache: StoreCache,
  messageBlock: MessageBlock,
  reader: BytesReader
): void {
  let snapshot = reader.snapshot();
  let tags = decodeTags(reader);

  for (let i = 0; i < tags.length && reader.ok && reader.length() > 0; i++) {
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
  } else {
    reader.fail();
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
    executeEmptySetBlockNumbersForEpochMessage(
      cache,
      snapshot,
      reader,
      id,
      messageBlock
    );
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
  let newEpoch = cache.getEpoch(nextEpochId(globalState));
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

    // Create new NetworkEpochBlockNumber
    createOrUpdateNetworkEpochBlockNumber(
      networks[i].id,
      newEpoch.epochNumber,
      acceleration,
      cache
    );
  }

  log.warning("Successfullly decocoded accelerations", []);
  message.accelerations = accelerations;
  message.data = reader.diff(snapshot);
}

function executeEmptySetBlockNumbersForEpochMessage(
  cache: StoreCache,
  snapshot: BytesReader,
  reader: BytesReader,
  id: String,
  messageBlock: MessageBlock
): void {
  let globalState = cache.getGlobalState();
  let message = cache.getSetBlockNumbersForEpochMessage(id);
  message.block = messageBlock.id;

  let numNetworks = BigInt.fromU64(decodeU64(reader));
  if (!reader.ok) {
    return;
  }

  message.count = numNetworks;

  log.warning("BEFORE EPOCH LOOP, AMOUNT TO CREATE: {}", [
    message.count!.toString()
  ]);

  for (let i = BIGINT_ZERO; i < message.count!; i += BIGINT_ONE) {
    log.warning("EPOCH LOOP, CREATING EPOCH: {}", [i.toString()]);
    let newEpoch = cache.getEpoch(nextEpochId(globalState));
    globalState.latestValidEpoch = newEpoch.id;
  }
  log.warning("AFTER EPOCH LOOP", []);

  message.data = reader.diff(snapshot);
}

function executeCorrectEpochsMessage(
  cache: StoreCache,
  snapshot: BytesReader,
  reader: BytesReader,
  id: String,
  messageBlock: MessageBlock
): void {
  // TODO.
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
  let removedNetworks: Array<Network> = [];

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
    if (!reader.ok || networkId >= networks.length) {
      reader.ok = false; // in case of the second check, to make sure we flag the payload as invalid
      return;
    }

    networks[networkId].removedAt = message.id;
    removedNetworks.push(swapAndPop(networkId, networks));
  }

  let numInsertions = decodeU64(reader) as i32;
  if (!reader.ok) {
    return;
  }

  // now get all the add network strings
  for (let i = 0; i < numInsertions; i++) {
    let chainId = decodeString(reader);
    if (!reader.ok) {
      return;
    }

    let network = cache.getNetwork(chainId);
    network.addedAt = message.id;
    network.removedAt = null; // unsetting to make sure that if the network existed before, it's no longer flagged as removed
    networks.push(network);
  }

  globalState.activeNetworkCount += numInsertions;
  globalState.activeNetworkCount -= numRemovals;
  globalState.networkCount += numInsertions;

  commitNetworkChanges(removedNetworks, networks, globalState);

  message.removeCount = BigInt.fromU64(numRemovals);
  message.addCount = BigInt.fromU64(numInsertions);
  message.block = messageBlock.id;
  message.data = reader.diff(snapshot);
}
