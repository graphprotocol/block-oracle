import hre from 'hardhat'
import '@nomiclabs/hardhat-ethers'
import { utils } from 'ethers'
import { randomHexBytes, sure } from './utils'
import { EpochOracle } from '../build/types/EpochOracle'

const ethers = hre.ethers
const { hexlify } = utils

// Payload format
// {
//     version: 1 byte
//     length: 1 byte
//     epochNumber: 8 bytes
//     arrayOfUpdates: [{
//        networkId: 2 bytes
//        blockHash: 32 bytes
//     }, ...]
// }

const VERSION = 0
const NETWORK_ID_BYTES = 2
const EPOCH_NUMBER_BYTES = 8

interface EpochBlockUpdatePayload {
  length: number
  epochNumber: number
  updates: EpochBlockUpdate[]
}

interface EpochBlockUpdate {
  networkId: number
  blockHash: string
}

const genEpochUpdate = (networkId: number): EpochBlockUpdate => {
  return {
    networkId,
    blockHash: randomHexBytes(32),
  }
}

const encodeEpochUpdate = (epochBlockUpdate: EpochBlockUpdate): string => {
  return utils.hexConcat([
    utils.hexZeroPad(hexlify(epochBlockUpdate.networkId), NETWORK_ID_BYTES),
    hexlify(epochBlockUpdate.blockHash),
  ])
}

const encodePayload = (epochNumber: number, items: EpochBlockUpdate[]) => {
  return utils.hexConcat([
    hexlify(VERSION), // version
    hexlify(items.length), // length
    utils.hexZeroPad(hexlify(epochNumber), EPOCH_NUMBER_BYTES), // epochNumber
    ...items.map(encodeEpochUpdate), // updates
  ])
}

const main = async () => {
  // Generate an update batch
  const items = [
    genEpochUpdate(1),
    genEpochUpdate(2),
    genEpochUpdate(3),
    genEpochUpdate(4),
    genEpochUpdate(5),
    genEpochUpdate(6),
    genEpochUpdate(7),
  ]
  const epochNumber = 3
  const epochOracleAddress = '0x706f4d56dd0d945c2a3096abb0de8fc49c175df2'
  const payload = encodePayload(epochNumber, items)

  console.log(items)
  console.log(payload)

  // Post update
  const accounts = await hre.ethers.getSigners()
  const signer = accounts[0]
  const epochOracle = (await ethers.getContractAt(
    'EpochOracle',
    epochOracleAddress,
    signer,
  )) as EpochOracle

  if (await sure('do you want to post the updates?')) {
    const tx = await epochOracle.setEpochBlocksPayload(payload)
    console.log(`Sent transaction ${tx.hash}...`)
    const receipt = await tx.wait()
    receipt.status // 1 = success, 0 = failure
      ? console.log(`Transaction succeeded: ${tx.hash}`)
      : console.log(`Transaction failed: ${tx.hash}`)
  }
}

main()
  .then(() => process.exit(0))
  .catch(error => {
    console.error(error)
    process.exit(1)
  })
