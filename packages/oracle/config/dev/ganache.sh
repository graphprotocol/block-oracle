#!/usr/bin/env bash
 
# Accounts addresses (private keys are below):
# 1. 0xc10031a50E1B68Fb170A81159cd9d8543D6563dF
# 2. 0x8BDB21e9E8164539b062D3c09e9effCAe64f1253
# 3. 0x7Cf2f46F77CCB5C27070559C7E917331011271B7
# 4. 0x3544738060f7ac4FFdE6e583E971FFd91525ccdC

ganache ethereum \
	--wallet.accounts 0x75dc16000b877ea0d4f764281c4c3fb8a047a7a0219361ac0bc82f325bc6ef1d,10000000000000000000000 \
	--wallet.accounts 0x55d5afd06a153af67c601467bd6c83c723289eb46f08f3a0a32b31540512097a,10000000000000000000000 \
	--wallet.accounts 0x59d5b84a8c38ad0ba29f4cca27e482cd120cd4c3290394a17fcfb752130b23e0,10000000000000000000000 \
	--wallet.accounts 0x53f8ed68cc08cab1facc8a747f6a6a231b3a738d11b04735aa83c714af0cabd2,10000000000000000000000 \
	--server.port 8545

