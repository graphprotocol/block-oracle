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
  swapAndPop,
  commitNetworkChanges,
  wipeNetworkList,
  nextEpochId,
  parseCalldata
} from "./helpers";
import { StoreCache } from "./store-cache";
import { BIGINT_ZERO, BIGINT_ONE, OWNER_ADDRESS_STRING } from "./constants";

export function handleLogCrossChainEpochOracle(event: Log): void {
  // this is only used in local development, and needs to strip the calldata to only
  // get the actual payload data we care about, without the selector and argument descriptors
  let data = parseCalldata(event.params.data);
  processPayload(
    event.transaction.from.toHexString(),
    data,
    event.transaction.hash.toHexString(),
    event.block.number
  );
}

export function handleCrossChainEpochOracle(
  call: CrossChainEpochOracleCall
): void {
  processPayload(
    call.transaction.from.toHexString(),
    call.inputs._payload,
    call.transaction.hash.toHexString(),
    call.block.number
  );
}

export function processPayload(
  submitter: string,
  payloadBytes: Bytes,
  txHash: string,
  blockNumber: BigInt
): void {
  // Start the StoreCache
  let cache = new StoreCache();

  // This is the only thing not handled through the store cache since we want all
  // Payload entity to persist (to provide context for validity of the payload)
  let payload = new Payload(txHash);
  payload.data = payloadBytes;
  payload.submitter = submitter;
  payload.valid = true;
  payload.createdAt = blockNumber;

  let reader = new BytesReader(payloadBytes);
  let blockIdx = 0;

  if (cache.getGlobalState().owner != payload.submitter) {
    log.error("Invalid submitter. Owner: {}. Submitter: {}. Avoiding payload", [
      cache.getGlobalState().owner,
      payload.submitter
    ]);
    return;
  }

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
      payload.errorMessage = reader.errorMessage;
      payload.save();
      return;
    }

    log.warning("Finished processing message block num. {}", [i]);
    blockIdx++;
  }

  payload.save();
  cache.commitChanges();
}

export function processMessageBlock(
  cache: StoreCache,
  messageBlock: MessageBlock,
  reader: BytesReader
): void {
  let snapshot = reader.snapshot();
  let tags = decodeTags(reader);

  log.warning("Tags to process: {}. Remaining reader length: {}", [
    tags.toString(),
    reader.length().toString()
  ]);

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
  } else if (tag == MessageTag.ChangeOwnershipMessage) {
    executeChangeOwnershipMessage(cache, snapshot, reader, id, messageBlock);
  } else if (tag == MessageTag.ResetStateMessage) {
    executeResetStateMessage(cache, snapshot, reader, id, messageBlock);
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

    if (!cache.isNetworkAlreadyRegistered(chainId)) {
      let network = cache.getNetwork(chainId);
      network.addedAt = message.id;
      network.removedAt = null; // unsetting to make sure that if the network existed before, it's no longer flagged as removed
      networks.push(network);
    } else {
      reader.fail("Network {} is already registered.".replace("{}", chainId));
      return;
    }
  }

  globalState.activeNetworkCount += numInsertions;
  globalState.activeNetworkCount -= numRemovals;
  globalState.networkCount += numInsertions;

  commitNetworkChanges(removedNetworks, networks, globalState, message.id);

  message.removeCount = BigInt.fromU64(numRemovals);
  message.addCount = BigInt.fromU64(numInsertions);
  message.block = messageBlock.id;
  message.data = reader.diff(snapshot);
}

function executeChangeOwnershipMessage(
  cache: StoreCache,
  snapshot: BytesReader,
  reader: BytesReader,
  id: String,
  messageBlock: MessageBlock
): void {
  let globalState = cache.getGlobalState();
  let message = cache.getChangeOwnershipMessage(id);
  let address = reader.advance(20); // address should always be 20 bytes

  message.block = messageBlock.id;
  message.newOwner = address.toHexString();
  message.oldOwner = globalState.owner;
  message.data = reader.diff(snapshot);

  globalState.owner = message.newOwner;
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

  globalState.networkCount = 0;
  globalState.activeNetworkCount = 0;
  globalState.encodingVersion = 0;
  globalState.owner = OWNER_ADDRESS_STRING; // maybe not this one?
  globalState.networkArrayHead = null;
  globalState.latestValidEpoch = null;
}
