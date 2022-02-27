import inquirer from 'inquirer'
import '@nomiclabs/hardhat-ethers'
import '@nomiclabs/hardhat-waffle'
import { utils, BigNumber } from 'ethers'
import { getAddress } from 'ethers/lib/utils'

const { hexlify, randomBytes } = utils

export const toBN = (value: string | number): BigNumber => BigNumber.from(value)
export const randomHexBytes = (n = 32): string => hexlify(randomBytes(n))
export const randomAddress = (): string => getAddress(randomHexBytes(20))

export const sure = async (message = 'Are you sure?'): Promise<boolean> => {
  const res = await inquirer.prompt({
    name: 'confirm',
    type: 'confirm',
    message,
  })
  if (!res.confirm) {
    console.info('Cancelled')
    return false
  }
  return true
}
