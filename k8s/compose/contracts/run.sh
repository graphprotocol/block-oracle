#!/usr/bin/sh
set -eux

# deploy the DataEdge contract
yarn hardhat run --network compose scripts/deploy-local.ts

# deploy the network contracts (including the Epoch Manager)
cd network-contracts
yarn hardhat migrate --config extended.config.ts --network compose --graph-config config/graph.localhost.yml --skip-confirmation

# set automining interval
yarn hardhat run --config extended.config.ts --network compose /set-automining-interval.js
