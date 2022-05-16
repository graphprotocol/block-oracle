#!/usr/bin/env bash
set -eu

. ./prelude.sh

github_clone graphprotocol/graph-node master
cd build/graphprotocol/graph-node

cargo build -p graph-node

await "curl --silent --fail localhost:${ETHEREUM_PORT}" 0
# graph-node has issues if the chain has no blocks, so we just make sure at least one exists
curl "localhost:${ETHEREUM_PORT}" -X POST --data '{"jsonrpc":"2.0","method":"evm_mine","params":[],"id":1}'

await "curl --silent --fail localhost:${IPFS_PORT}" 22
await "curl --silent --fail localhost:${POSTGRES_PORT}" 52

export POSTGRES_URL="postgresql://${POSTGRES_USER}:${POSTGRES_PASSWORD}@localhost:${POSTGRES_PORT}/${POSTGRES_USER}"

# the Epoch Subhraph declares `ropsten` as its network, so we have to configure Graph Node to use it
export ETHEREUM_RPC="ropsten:http://localhost:${ETHEREUM_PORT}"

export IPFS="localhost:${IPFS_PORT}"
export GRAPH_IPFS_TIMEOUT=10
export GRAPH_LOG=debug

cargo run -p graph-node
