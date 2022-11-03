async function main() {
  // RegisterNetworks message: add network eip155:1337 (same as protocol chain)
  // This is necessary because since commit #cc729541e5d3fe0a11ef7f9a4382dd693525eb9e the Epoch Block Oracle won't send the RegisterNetworks message.
  await network.provider.send("eth_sendTransaction", [
    {
      from: "0x90F8bf6A479f320ead074411a4B0e7944Ea8c9C1",
      to: "0xe78a0f7e598cc8b0bb87894b0f60dd2a88d6a8ab",
      data: "0xa1dce3320000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000f030103176569703135353a313333370000000000000000000000000000000000",
    },
  ]);
  //
  await network.provider.send("evm_setIntervalMining", [2000]);
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
