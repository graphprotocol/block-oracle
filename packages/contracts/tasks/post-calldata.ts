import { task } from 'hardhat/config'
import '@nomiclabs/hardhat-ethers'

task('data:post', 'Post calldata')
  .addParam('vault', 'Address of the data vault contract')
  .addParam('data', 'Call data to post')
  .setAction(async (taskArgs, hre) => {
    // prepare data
    const vaultAddress = taskArgs.vault
    const txData = taskArgs.data
    const contract = await hre.ethers.getContractAt('DataVault', vaultAddress)
    const txRequest = {
      data: txData,
      to: contract.address,
    }

    // send transaction
    console.log(`Sending data...`)
    console.log(`> vault: ${contract.address}`)
    console.log(`> sender: ${await contract.signer.getAddress()}`)
    console.log(`> payload: ${txData}`)
    const tx = await contract.signer.sendTransaction(txRequest)
    console.log(`> tx: ${tx.hash} nonce:${tx.nonce} limit: ${tx.gasLimit} gas: ${tx.gasPrice.toNumber() / 1e9} (gwei)`)
    const rx = await tx.wait()
    console.log('> rx: ', rx.status == 1 ? 'success' : 'failed')
    console.log(`Done!`)
  })
