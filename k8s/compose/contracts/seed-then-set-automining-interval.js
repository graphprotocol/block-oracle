async function main() {
  const epochManagerAddress = process.env.EPOCH_MANAGER_CONTRACT_ADDRESS;
  const dataEdgeAddress = process.env.DATA_EDGE_CONTRACT_ADDRESS;
  console.table({ epochManagerAddress, dataEdgeAddress });
  const [signer] = await ethers.getSigners();
  const epochManager = await ethers.getContractAt(
    "EpochManager",
    epochManagerAddress,
    signer
  );

  // RegisterNetworks message: add network eip155:1337 (same as protocol chain)
  // This is necessary because since commit #cc729541e5d3fe0a11ef7f9a4382dd693525eb9e the Epoch Block Oracle won't send the RegisterNetworks message.
  await network.provider.send("eth_sendTransaction", [
    {
      from: signer.address,
      to: dataEdgeAddress,
      data: "0xa1dce3320000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000f030103176569703135353a313333370000000000000000000000000000000000",
    },
  ]);

  // set hardhat to produce blocks every 2 seconds
  await network.provider.send("evm_setIntervalMining", [2000]);

  // set epoch length
  await epochManager.setEpochLength(2);
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
