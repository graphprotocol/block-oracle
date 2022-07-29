import { BigInt } from "@graphprotocol/graph-ts";

export const PREAMBLE_BIT_LENGTH = 8;
export const PREAMBLE_BYTE_LENGTH = PREAMBLE_BIT_LENGTH / 8;
export const TAG_BIT_LENGTH = 4;
export const OWNER_ADDRESS_STRING = "0xfa711da0f9336f27e7b7483398cbd8f0880f259a";
export const EPOCH_MANAGER_ADDRESS = "0x03541c5cd35953CD447261122F93A5E7b812D697";

export let BIGINT_ZERO = BigInt.fromI32(0);
export let BIGINT_ONE = BigInt.fromI32(1);
