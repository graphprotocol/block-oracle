#!/usr/bin/env bash

cd ../contracts
yarn hardhat run --network localhost scripts/deploy.ts
cd ../oracle

cargo run -- \
	--config-file=config/dev/config.toml \
	--database-url=:memory: \
	--owner-private-key=75dc16000b877ea0d4f764281c4c3fb8a047a7a0219361ac0bc82f325bc6ef1d
