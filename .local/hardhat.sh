#!/usr/bin/env bash
set -eu

cd ../packages/contracts/
yarn install
yarn hardhat node --port "${HARDHAT_JRPC_PORT}"
