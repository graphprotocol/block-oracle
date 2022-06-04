#!/usr/bin/env bash
set -eu

. ./prelude.sh

await "curl --silent --fail localhost:${HARDHAT_JRPC_PORT}" > /dev/null

cd ../packages/contracts/
yarn hardhat run --network localhost scripts/deploy-local.ts
