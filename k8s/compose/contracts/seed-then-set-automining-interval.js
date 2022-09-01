async function main() {
	await network.provider.send('eth_sendTransaction', [{
		from: '0x90F8bf6A479f320ead074411a4B0e7944Ea8c9C1',
		to: '0xe78a0f7e598cc8b0bb87894b0f60dd2a88d6a8ab',
		data: '0xa1dce33200000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000017030105136569703135353a3737136569703135353a3939000000000000000000'
	}]);
	await network.provider.send('evm_setIntervalMining', [2000]);
}

main()
	.then(() => process.exit(0))
	.catch((error) => {
		console.error(error)
		process.exit(1)
	})
