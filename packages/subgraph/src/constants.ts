import { BigInt } from "@graphprotocol/graph-ts";

export const PREAMBLE_BIT_LENGTH = 8;
export const PREAMBLE_BYTE_LENGTH = PREAMBLE_BIT_LENGTH / 8;
export const TAG_BIT_LENGTH = 4;
//export const OWNER_ADDRESS_STRING = "";
export const EPOCH_MANAGER_ADDRESS = "0x64f990bf16552a693dcb043bb7bf3866c5e05ddb";

export let INITIAL_PERMISSION_SET = new Map<String,Array<String>>();
INITIAL_PERMISSION_SET.set("0xeb4ad97a099defc85c900a60adfd2405c455b2c0", ["SetBlockNumbersForEpochMessage","CorrectEpochsMessage","ResetStateMessage"])
INITIAL_PERMISSION_SET.set("0x48301fe520f72994d32ead72e2b6a8447873cf50", ["SetBlockNumbersForEpochMessage","UpdateVersionsMessage","RegisterNetworksMessage","ChangePermissionsMessage"])
export let BIGINT_ZERO = BigInt.fromI32(0);
export let BIGINT_ONE = BigInt.fromI32(1);
