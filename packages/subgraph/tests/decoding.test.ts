import { clearStore, test, assert } from 'matchstick-as/assembly/index'
import { handleCrossChainEpochOracle } from '../src/mapping'
import { getGlobalState, getTags, decodePrefixVarIntU64 } from '../src/helpers'
import { log } from '@graphprotocol/graph-ts'

import { CrossChainEpochOracleCall } from "../generated/DataEdge/DataEdge";
import { Bytes, BigInt } from "@graphprotocol/graph-ts";
import { DataEdge, MessageBlock, Payload } from "../generated/schema";

test('U64 Decoding', () => {
  // 23 -> [47] -> 0x2F
  // 9000 -> [162, 140] -> 0xA28C
  // 1455594 -> [84, 175, 177] -> 0x54AFB1
  // 109771541 -> [88, 177, 175, 104] -> 0x58B1AF68
  // 24345908991 -> [240, 223, 34, 100, 181] -> 0xF0DF2264B5
  // 1903269233213 -> [96, 143, 240, 235, 200, 110] -> 0x608FF0EBC86E
  // 558944227176442 -> [64, 253, 23, 14, 172, 45, 254] -> 0x40FD170EAC2DFE
  // 72057594037927935 -> [128, 255, 255, 255, 255, 255, 255, 255] -> 0x80FFFFFFFFFFFFFF

  let encoded0x17 = Bytes.fromHexString('0x2F') // 23 u64
  let encoded0x2328 = Bytes.fromHexString('0xA28C') // 9000 u64
  let encoded0x1635EA = Bytes.fromHexString('0x54AFB1') // 1455594 u64
  let encoded0x068AFB15 = Bytes.fromHexString('0x58B1AF68') // 109771541 u64
  let encoded0x05AB2116FF = Bytes.fromHexString('0xF0DF2264B5') // 24345908991 u64
  let encoded0x01BB23AFC23D = Bytes.fromHexString('0x608FF0EBC86E') // 1903269233213 u64
  let encoded0x01FC5B581C2FFA = Bytes.fromHexString('0x40FD170EAC2DFE') // 558944227176442 u64
  let encoded0xFFFFFFFFFFFFFF = Bytes.fromHexString('0x80FFFFFFFFFFFFFF') // 72057594037927935 u64

  let decoded0x17 = decodePrefixVarIntU64(changetype<Bytes>(encoded0x17), 0)
  let decoded0x2328 = decodePrefixVarIntU64(changetype<Bytes>(encoded0x2328), 0)
  let decoded0x1635EA = decodePrefixVarIntU64(changetype<Bytes>(encoded0x1635EA), 0)
  let decoded0x068AFB15 = decodePrefixVarIntU64(changetype<Bytes>(encoded0x068AFB15), 0)
  let decoded0x05AB2116FF = decodePrefixVarIntU64(changetype<Bytes>(encoded0x05AB2116FF), 0)
  let decoded0x01BB23AFC23D = decodePrefixVarIntU64(changetype<Bytes>(encoded0x01BB23AFC23D), 0)
  let decoded0x01FC5B581C2FFA = decodePrefixVarIntU64(changetype<Bytes>(encoded0x01FC5B581C2FFA), 0)
  let decoded0xFFFFFFFFFFFFFF = decodePrefixVarIntU64(changetype<Bytes>(encoded0xFFFFFFFFFFFFFF), 0)

  // assert.bytesEquals(changetype<Bytes>(Bytes.fromU64(decoded0x17)), Bytes.empty())
  assert.bigIntEquals(BigInt.fromU64(decoded0x17), BigInt.fromU64(23 as u64))
  assert.bigIntEquals(BigInt.fromU64(decoded0x2328), BigInt.fromU64(9000 as u64))
  assert.bigIntEquals(BigInt.fromU64(decoded0x1635EA), BigInt.fromU64(1455594 as u64))
  assert.bigIntEquals(BigInt.fromU64(decoded0x068AFB15), BigInt.fromU64(109771541 as u64))
  assert.bigIntEquals(BigInt.fromU64(decoded0x05AB2116FF), BigInt.fromU64(24345908991 as u64))
  assert.bigIntEquals(BigInt.fromU64(decoded0x01BB23AFC23D), BigInt.fromU64(1903269233213 as u64))
  assert.bigIntEquals(BigInt.fromU64(decoded0x01FC5B581C2FFA), BigInt.fromU64(558944227176442 as u64))
  assert.bigIntEquals(BigInt.fromU64(decoded0xFFFFFFFFFFFFFF), BigInt.fromU64(72057594037927935 as u64))

  // Clear the store in order to start the next test off on a clean slate
  clearStore()
})

test('I64 Decoding (U64 + ZigZag)', () => {
  //...
})
