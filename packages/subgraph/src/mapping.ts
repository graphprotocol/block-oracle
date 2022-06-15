import {
  CrossChainEpochOracleCall,
  Log
} from "../generated/DataEdge/DataEdge";
import { Bytes, BigInt, log, store } from "@graphprotocol/graph-ts";
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
  getOrCreateEpoch,
  createOrUpdateNetworkEpochBlockNumber,
  MessageTag,
  getGlobalState,
  getActiveNetworks,
  swapAndPop,
  commitNetworkChanges,
  nextEpochId,
} from "./helpers";
import {
  DirtyChanges
} from "./dirty";
import {
  BIGINT_ZERO,
  BIGINT_ONE,
  ENTITY_MESSAGE_BLOCK,
  ENTITY_NETWORK,
  ENTITY_PAYLOAD,
  ENTITY_GLOBAL_STATE,
  ENTITY_NETWORK_EPOCH_BLOCK_NUMBER,
} from "./constants";

export function handleLogCrossChainEpochOracle(
  event: Log
): void {
  processPayload(
    event.transaction.from.toHexString(),
    event.params.data,
    event.transaction.hash.toHexString(),
  );
}

export function handleCrossChainEpochOracle(
  call: CrossChainEpochOracleCall
): void {
  processPayload(
    call.transaction.from.toHexString(),
    call.inputs._payload,
    call.transaction.hash.toHexString(),
  );
}

export function processPayload(
  submitter: string,
  payloadBytes: Bytes,
  txHash: string
): void {
  let state = getGlobalState();
  DirtyChanges.set(ENTITY_GLOBAL_STATE, state);

  // Prepare the raw `Payload` entity.
  let payload = new Payload(txHash);
  payload.data = payloadBytes;
  payload.submitter = submitter;
  payload.messageBlocks = [];
  DirtyChanges.set(ENTITY_PAYLOAD, payload);

  let reader = new BytesReader(payloadBytes);

  while (reader.length() > 0) {
    let i = payload.messageBlocks.length.toString();
    log.info("New message block (num. {}) with remaining data: {}", [i, reader.data().toHexString()]);

    let messageBlock = new MessageBlock([txHash, i].join("-"));
    messageBlock.payload = payload.id;
    messageBlock.messages = [];
    processMessageBlock(state, messageBlock, reader);

    if (!reader.ok) {
      log.error("Failed to process message block num. {}", [i]);
      // We only persist the `Payload` in case of failure, without any
      // `GlobalState` changes nor `MessageBlock`s.
      payload.messageBlocks = [];
      payload.save();
      return;
    }

    log.info("Finished processing message block num. {}", [i]);
    payload.messageBlocks.push(messageBlock.id);
    DirtyChanges.set(ENTITY_MESSAGE_BLOCK, messageBlock);
  }

  DirtyChanges.persist();
}

export function processMessageBlock(
  globalState: GlobalState,
  messageBlock: MessageBlock,
  reader: BytesReader
): void {
  let snapshot = reader.snapshot();
  let tags = decodeTags(reader);

  for (let i = 0; i < tags.length && reader.ok && reader.length() > 0; i++) {
    processMessage(
      globalState,
      messageBlock,
      i,
      tags[i],
      reader
    );
  }

  messageBlock.data = reader.diff(snapshot);
}

// Finishes decoding the message and executes it.
export function processMessage(
  globalState: GlobalState,
  messageBlock: MessageBlock,
  i: i32,
  tag: MessageTag,
  reader: BytesReader
): void {
  log.info("Processing new message with tag {}. The remaining payload is {}", [
    MessageTag.toString(tag),
    reader.data().toHexString()
  ]);
  let id = [messageBlock.id, i.toString()].join("-");
  let snapshot = reader.snapshot();

  // The message type can then be changed according to the tag.
  let message = new SetBlockNumbersForEpochMessage(id);

  log.info("Executing message {}", [MessageTag.toString(tag)]);
  if (tag == MessageTag.SetBlockNumbersForEpochMessage) {
    executeSetBlockNumbersForEpochMessage(
      changetype<SetBlockNumbersForEpochMessage>(message), globalState, reader
    );
  } else if (tag == MessageTag.CorrectEpochsMessage) {
    executeCorrectEpochsMessage(
      changetype<CorrectEpochsMessage>(message), globalState, reader
    );
  } else if (tag == MessageTag.UpdateVersionsMessage) {
    executeUpdateVersionsMessage(
      changetype<UpdateVersionsMessage>(message), globalState, reader
    );
  } else if (tag == MessageTag.RegisterNetworksMessage) {
    executeRegisterNetworksMessage(
      changetype<RegisterNetworksMessage>(message), globalState, reader
    );
  } else {
    reader.fail();
    log.error("Unknown message tag '{}'", [MessageTag.toString(tag)]);
    return;
  }

  message.block = messageBlock.id;
  message.data = reader.diff(snapshot);
  DirtyChanges.set(MessageTag.toString(tag), message);
  messageBlock.messages.push(message.id);
}

function executeSetBlockNumbersForEpochMessage(
  message: SetBlockNumbersForEpochMessage,
  globalState: GlobalState,
  reader: BytesReader
): void {
  log.info("There are {} currently active networks", [
    globalState.activeNetworkCount.toString()
  ]);

  if (globalState.activeNetworkCount != 0) {
    executeNonEmptySetBlockNumbersForEpochMessage(message, globalState, reader);
  } else {
    executeEmptySetBlockNumbersForEpochMessage(message, globalState, reader);
  }
}

function executeNonEmptySetBlockNumbersForEpochMessage(
  message: SetBlockNumbersForEpochMessage,
  globalState: GlobalState,
  reader: BytesReader
): void {
  let networks = getActiveNetworks(globalState);
  let newEpoch = getOrCreateEpoch(nextEpochId(globalState));
  globalState.latestValidEpoch = newEpoch.id;

  let merkleRoot = reader.advance(32);
  message.merkleRoot = merkleRoot;
  log.info("The Merkle root of the new epoch is {}", [
    merkleRoot.toHexString()
  ]);
  log.info("Now decoding block updates: {}", [reader.data().toHexString()]);

  let accelerations: Array<BigInt> = [];
  for (let i = 0; i < globalState.activeNetworkCount; i++) {
    let acceleration = BigInt.fromI64(decodeI64(reader));
    if (!reader.ok) {
      log.info("Failed to decode acceleration num. {}", [i.toString()]);
      return;
    }
    log.info("Decoded acceleration num. {} with value {}", [i.toString(), acceleration.toString()]);

    accelerations.push(acceleration);

    // Create new NetworkEpochBlockNumber
    let blockNum = createOrUpdateNetworkEpochBlockNumber(
      networks[i].id,
      newEpoch.epochNumber,
      acceleration
    );
    DirtyChanges.set(ENTITY_NETWORK_EPOCH_BLOCK_NUMBER, blockNum);
  }

  log.info("Successfully decocoded accelerations", []);
  message.accelerations = accelerations;
}

function executeEmptySetBlockNumbersForEpochMessage(
  message: SetBlockNumbersForEpochMessage,
  globalState: GlobalState,
  reader: BytesReader
): void {
  let numNetworks = BigInt.fromU64(decodeU64(reader));
  if (!reader.ok) {
    return;
  }

  message.count = numNetworks;

  log.info("BEFORE EPOCH LOOP, AMOUNT TO CREATE: {}", [
    message.count!.toString()
  ]);

  for (let i = BIGINT_ZERO; i < message.count!; i = i.plus(BIGINT_ONE)) {
    log.info("EPOCH LOOP, CREATING EPOCH: {}", [i.toString()]);
    let newEpoch = getOrCreateEpoch(nextEpochId(globalState));
    globalState.latestValidEpoch = newEpoch.id;
  }
  log.info("AFTER EPOCH LOOP", []);
}

function executeCorrectEpochsMessage(
  message: CorrectEpochsMessage,
  globalState: GlobalState,
  reader: BytesReader
): void {
  // TODO.
}

function executeUpdateVersionsMessage(
  message: UpdateVersionsMessage,
  globalState: GlobalState,
  reader: BytesReader
): void {
  let version = decodeU64(reader);
  globalState.encodingVersion = version as i32;
}

function executeRegisterNetworksMessage(
  message: RegisterNetworksMessage,
  globalState: GlobalState,
  reader: BytesReader
): void {
  let networks = getActiveNetworks(globalState);
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

    let network = new Network(chainId);
    network.addedAt = message.id;
    DirtyChanges.set(ENTITY_NETWORK, network);
    networks.push(network);
  }

  globalState.activeNetworkCount += numInsertions;
  globalState.activeNetworkCount -= numRemovals;
  globalState.networkCount += numInsertions;

  commitNetworkChanges(removedNetworks, networks, globalState);

  message.removeCount = BigInt.fromU64(numRemovals);
  message.addCount = BigInt.fromU64(numInsertions);
}
