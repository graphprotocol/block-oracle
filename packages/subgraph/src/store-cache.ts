import { BigInt } from "@graphprotocol/graph-ts";
import { OWNER_ADDRESS_STRING } from "./constants";
import {
  GlobalState,
  Epoch,
  NetworkEpochBlockNumber,
  Network,
  SetBlockNumbersForEpochMessage,
  RegisterNetworksMessage,
  CorrectEpochsMessage,
  UpdateVersionsMessage,
  ChangeOwnershipMessage,
  ResetStateMessage,
  MessageBlock
} from "../generated/schema";

export class SafeMap<K, V> extends Map<K, V> {
  safeGet(id: K): V | null {
    return this.has(id) ? this.get(id) : null;
  }
}

/*
 *   The sole purpose of this class is to keep tabs of all entities requested
 *   during the execution of a payload, so that we can decide to save their changes
 *   or not during execution (transactional style)
 */
export class StoreCache {
  // Initialize all variables used in caching.
  //entities: Map<String, Map<String, Entity>> something like this would be great
  // but it might be hard to coerce the types for saving afterwards
  // as well as maintaining the proper fields
  // For now we can just use more bloated code, without needing to get into generics
  state: GlobalState;
  networks: SafeMap<String, Network>;
  epochs: SafeMap<String, Epoch>;
  blockNumbers: SafeMap<String, NetworkEpochBlockNumber>;
  setBlockNumbersForEpochMessages: SafeMap<
    String,
    SetBlockNumbersForEpochMessage
  >;
  registerNetworksMessages: SafeMap<String, RegisterNetworksMessage>;
  correctEpochsMessages: SafeMap<String, CorrectEpochsMessage>;
  updateVersionsMessages: SafeMap<String, UpdateVersionsMessage>;
  changeOwnershipMessages: SafeMap<String, ChangeOwnershipMessage>;
  resetStateMessages: SafeMap<String, ResetStateMessage>;
  messageBlocks: SafeMap<String, MessageBlock>;

  constructor() {
    let state = GlobalState.load("0");
    if (state == null) {
      state = new GlobalState("0");
      state.networkCount = 0;
      state.activeNetworkCount = 0;
      state.encodingVersion = 0;
      state.owner = OWNER_ADDRESS_STRING;
      state.save()
    }
    this.state = state;
    this.networks = new SafeMap<String, Network>();
    this.epochs = new SafeMap<String, Epoch>();
    this.blockNumbers = new SafeMap<String, NetworkEpochBlockNumber>();
    this.setBlockNumbersForEpochMessages = new SafeMap<
      String,
      SetBlockNumbersForEpochMessage
    >();
    this.registerNetworksMessages = new SafeMap<
      String,
      RegisterNetworksMessage
    >();
    this.correctEpochsMessages = new SafeMap<String, CorrectEpochsMessage>();
    this.updateVersionsMessages = new SafeMap<String, UpdateVersionsMessage>();
    this.changeOwnershipMessages = new SafeMap<String, ChangeOwnershipMessage>();
    this.resetStateMessages = new SafeMap<String, ResetStateMessage>();
    this.messageBlocks = new SafeMap<String, MessageBlock>();
  }

  getGlobalState(): GlobalState {
    return this.state;
  }

  getNetwork(id: String): Network {
    if (this.networks.safeGet(id) == null) {
      let network = Network.load(id);
      if (network == null) {
        network = new Network(id);
      }
      this.networks.set(id, network);
    }
    return this.networks.safeGet(id)!;
  }

  isNetworkAlreadyRegistered(id: String): bool {
    return (this.networks.has(id) || Network.load(id) != null) && this.getNetwork(id).removedAt == null
  }

  getNetworkEpochBlockNumber(id: String): NetworkEpochBlockNumber {
    if (this.blockNumbers.safeGet(id) == null) {
      let blockNum = NetworkEpochBlockNumber.load(id);
      if (blockNum == null) {
        blockNum = new NetworkEpochBlockNumber(id);
      }
      this.blockNumbers.set(id, blockNum);
    }
    return this.blockNumbers.safeGet(id)!;
  }

  hasNetworkEpochBlockNumber(id: String): bool {
    return this.blockNumbers.has(id) || NetworkEpochBlockNumber.load(id) != null
  }

  getEpoch(bigIntID: BigInt): Epoch {
    let id = bigIntID.toString();
    if (this.epochs.safeGet(id) == null) {
      let epoch = Epoch.load(id);
      if (epoch == null) {
        epoch = new Epoch(id);
        epoch.epochNumber = bigIntID;
      }
      this.epochs.set(id, epoch);
    }
    return this.epochs.safeGet(id)!;
  }

  getSetBlockNumbersForEpochMessage(
    id: String
  ): SetBlockNumbersForEpochMessage {
    if (this.setBlockNumbersForEpochMessages.safeGet(id) == null) {
      let message = SetBlockNumbersForEpochMessage.load(id);
      if (message == null) {
        message = new SetBlockNumbersForEpochMessage(id);
      }
      this.setBlockNumbersForEpochMessages.set(id, message);
    }
    return this.setBlockNumbersForEpochMessages.safeGet(id)!;
  }

  getRegisterNetworksMessage(id: String): RegisterNetworksMessage {
    if (this.registerNetworksMessages.safeGet(id) == null) {
      let message = RegisterNetworksMessage.load(id);
      if (message == null) {
        message = new RegisterNetworksMessage(id);
      }
      this.registerNetworksMessages.set(id, message);
    }
    return this.registerNetworksMessages.safeGet(id)!;
  }

  getCorrectEpochsMessage(id: String): CorrectEpochsMessage {
    if (this.correctEpochsMessages.safeGet(id) == null) {
      let message = CorrectEpochsMessage.load(id);
      if (message == null) {
        message = new CorrectEpochsMessage(id);
      }
      this.correctEpochsMessages.set(id, message);
    }
    return this.correctEpochsMessages.safeGet(id)!;
  }

  getUpdateVersionsMessage(id: String): UpdateVersionsMessage {
    if (this.updateVersionsMessages.safeGet(id) == null) {
      let message = UpdateVersionsMessage.load(id);
      if (message == null) {
        message = new UpdateVersionsMessage(id);
      }
      this.updateVersionsMessages.set(id, message);
    }
    return this.updateVersionsMessages.safeGet(id)!;
  }

  getChangeOwnershipMessage(id: String): ChangeOwnershipMessage {
    if (this.changeOwnershipMessages.safeGet(id) == null) {
      let message = ChangeOwnershipMessage.load(id);
      if (message == null) {
        message = new ChangeOwnershipMessage(id);
      }
      this.changeOwnershipMessages.set(id, message);
    }
    return this.changeOwnershipMessages.safeGet(id)!;
  }

  getResetStateMessage(id: String): ResetStateMessage {
    if (this.resetStateMessages.safeGet(id) == null) {
      let message = ResetStateMessage.load(id);
      if (message == null) {
        message = new ResetStateMessage(id);
      }
      this.resetStateMessages.set(id, message);
    }
    return this.resetStateMessages.safeGet(id)!;
  }

  getMessageBlock(id: String): MessageBlock {
    if (this.messageBlocks.safeGet(id) == null) {
      let messageBlock = MessageBlock.load(id);
      if (messageBlock == null) {
        messageBlock = new MessageBlock(id);
      }
      this.messageBlocks.set(id, messageBlock);
    }
    return this.messageBlocks.safeGet(id)!;
  }

  commitChanges(): void {
    this.state.save();

    // forEach crashes for some reason, so unfortunately have to do this...
    let networks = this.networks.values();
    for (let i = 0; i < networks.length; i++) {
      networks[i].save();
    }

    let epochs = this.epochs.values();
    for (let i = 0; i < epochs.length; i++) {
      epochs[i].save();
    }

    let blockNumbers = this.blockNumbers.values();
    for (let i = 0; i < blockNumbers.length; i++) {
      blockNumbers[i].save();
    }

    let blockNumMessages = this.setBlockNumbersForEpochMessages.values();
    for (let i = 0; i < blockNumMessages.length; i++) {
      blockNumMessages[i].save();
    }

    let registerNetworkMessages = this.registerNetworksMessages.values();
    for (let i = 0; i < registerNetworkMessages.length; i++) {
      registerNetworkMessages[i].save();
    }

    let correctEpochMessages = this.correctEpochsMessages.values();
    for (let i = 0; i < correctEpochMessages.length; i++) {
      correctEpochMessages[i].save();
    }

    let updateVersionMessages = this.updateVersionsMessages.values();
    for (let i = 0; i < updateVersionMessages.length; i++) {
      updateVersionMessages[i].save();
    }

    let changeOwnershipMessages = this.changeOwnershipMessages.values();
    for (let i = 0; i < changeOwnershipMessages.length; i++) {
      changeOwnershipMessages[i].save();
    }

    let resetStateMessages = this.resetStateMessages.values();
    for (let i = 0; i < resetStateMessages.length; i++) {
      resetStateMessages[i].save();
    }

    let messageBlocks = this.messageBlocks.values();
    for (let i = 0; i < messageBlocks.length; i++) {
      messageBlocks[i].save();
    }

    //this.networks.values().forEach(elem => elem.save());
    //this.epochs.values().forEach(elem => elem.save());
    // this.blockNumbers.values().forEach(elem => elem.save());
    // this.setBlockNumbersForEpochMessages.values().forEach(elem => elem.save());
    // this.registerNetworksMessages.values().forEach(elem => elem.save());
    // this.correctEpochsMessages.values().forEach(elem => elem.save());
    // this.updateVersionsMessages.values().forEach(elem => elem.save());
    // this.messageBlocks.values().forEach(elem => elem.save());
  }
}
