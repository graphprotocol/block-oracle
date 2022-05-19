#!/usr/bin/env bash
set -e

cd ../packages/contracts/
yarn install
yarn hardhat node --port "${HARDHAT_JRPC_PORT}"
