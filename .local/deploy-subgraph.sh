#!/usr/bin/env bash
set -eu

. ./prelude.sh

cd ../packages/subgraph/

yarn
yarn codegen
yarn build --network hardhat

await "curl --silent --fail localhost:${GRAPH_NODE_JRPC_PORT}"

yarn create-local
yarn deploy-local
