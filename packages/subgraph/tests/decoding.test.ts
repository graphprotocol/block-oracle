import { clearStore, test, assert } from 'matchstick-as/assembly/index'
import { handleCrossChainEpochOracle } from '../src/mapping'
import { getGlobalState, getTags, decodePrefixVarIntU64 } from '../src/helpers'

import { CrossChainEpochOracleCall } from "../generated/DataEdge/DataEdge";
import { Bytes, BigInt } from "@graphprotocol/graph-ts";
import { DataEdge, MessageBlock, Payload } from "../generated/schema";

test('U64 Decoding', () => {
  let encoded0x17 = Bytes.fromU64(47) // 23 u64
  let encoded0x2328 = Bytes.fromU64(41612) // 23 u64

  let decoded0x17 = decodePrefixVarIntU64(changetype<Bytes>(encoded0x17), 0)
  let decoded0x2328 = decodePrefixVarIntU64(changetype<Bytes>(encoded0x2328), 0)

  assert.bigIntEquals(BigInt.fromU64(decoded0x17), BigInt.fromU64(23 as u64))
  assert.bigIntEquals(BigInt.fromU64(decoded0x2328), BigInt.fromU64(9000 as u64))
  assert.bytesEquals(changetype<Bytes>(Bytes.fromU64(decoded0x2328)), Bytes.empty())

  // Clear the store in order to start the next test off on a clean slate
  clearStore()
})

test('I64 Decoding (U64 + ZigZag)', () => {
  //...
})
