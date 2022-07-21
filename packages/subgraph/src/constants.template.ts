import { BigInt } from "@graphprotocol/graph-ts";

export const PREAMBLE_BIT_LENGTH = 8;
export const PREAMBLE_BYTE_LENGTH = PREAMBLE_BIT_LENGTH / 8;
export const TAG_BIT_LENGTH = 4;
export const OWNER_ADDRESS_STRING = "{{owner}}";
export const EPOCH_MANAGER_ADDRESS = "{{epochManager}}";

export let BIGINT_ZERO = BigInt.fromI32(0);
export let BIGINT_ONE = BigInt.fromI32(1);
