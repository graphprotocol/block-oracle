#!/usr/bin/env bash
set -eu

. ./prelude.sh

echo "Waiting for hardhat"
await "curl --silent --fail localhost:${HARDHAT_JRPC_PORT}" > /dev/null

echo "Waiting for data edge deployment"
await_ready "data-edge"

github_clone graphprotocol/contracts dev

cd build/graphprotocol/contracts
yarn install && yarn deploy-localhost --skip-confirmation


# Send a JRPC to hardhat so it mines blocks periodically
curl -X POST -H "Content-Type: application/json" -d @"$BASE_PATH"/hardhat-set-interval.json http://localhost:8545

signal_ready epoch-manager
