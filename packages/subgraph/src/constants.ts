import { BigInt } from "@graphprotocol/graph-ts";

export const PREAMBLE_BIT_LENGTH = 8;
export const PREAMBLE_BYTE_LENGTH = PREAMBLE_BIT_LENGTH / 8;
export const TAG_BIT_LENGTH = 4;
export const OWNER_ADDRESS_STRING = "0x90f8bf6a479f320ead074411a4b0e7944ea8c9c1";

export let BIGINT_ZERO = BigInt.fromI32(0);
export let BIGINT_ONE = BigInt.fromI32(1);
