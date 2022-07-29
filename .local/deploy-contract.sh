#!/usr/bin/env bash
set -eu

. ./prelude.sh

await "curl --silent --fail ${HARDHAT_JRPC}" > /dev/null

cd ../packages/contracts/
yarn hardhat run --network localhost scripts/deploy-local.ts

signal_ready "data-edge"
