#!/usr/bin/env bash
set -eu

. ./prelude.sh

pushd ../

cargo build

await_contract "DataEdge" "$DATA_EDGE_CONTRACT_ADDRESS"

popd
await_ready epoch-manager

EPOCH_MANAGER_CONTRACT_ADDRESS=$(jq -r '."1337".EpochManager.address' < build/graphprotocol/contracts/addresses.json)
export EPOCH_MANAGER_CONTRACT_ADDRESS

await_contract "EpochManager" "$EPOCH_MANAGER_CONTRACT_ADDRESS"
await_subgraph

pushd ../
./target/debug/block-oracle ./crates/oracle/config/dev/config.toml || true

echo
echo "Epoch Block Oracle crashed."
echo "Press C-c to terminate this process."

while true; do
    sleep 1000
done
