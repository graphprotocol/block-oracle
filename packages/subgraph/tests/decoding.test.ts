import { clearStore, test, assert } from 'matchstick-as/assembly/index'
import { handleCrossChainEpochOracle } from '../src/mapping'
import { getGlobalState, getTags, decodePrefixVarIntU64 } from '../src/helpers'

import { CrossChainEpochOracleCall } from "../generated/DataEdge/DataEdge";
import { Bytes, BigInt } from "@graphprotocol/graph-ts";
import { DataEdge, Message, MessageBlock, Payload } from "../generated/schema";

test('U64 Decoding', () => {
  let encoded0x17 = Bytes.fromUnsignedBytes("2f") // 23 u64
  let encoded0x2328 = Bytes.fromUnsignedBytes("a28c") // 9000 u64

  let decoded0x17 = decodePrefixVarIntU64(encoded0x17, 0)
  let decoded0x2328 = decodePrefixVarIntU64(encoded0x2328, 0)
  // Assert the state of the store
  assert.equals(decoded0x17, 23 as u64)
  assert.equals(decoded0x2328, 9000 as u64)

  // Clear the store in order to start the next test off on a clean slate
  clearStore()
})

test('I64 Decoding (U64 + ZigZag)', () => {
  //...
})
