#!/usr/bin/env bash
set -eu

. ./prelude.sh

cd ../packages/subgraph/

yarn
yarn codegen
yarn build

await "curl --silent --fail localhost:${GRAPH_NODE_JRPC_PORT}"

yarn create-hardhat
yarn deploy-hardhat
