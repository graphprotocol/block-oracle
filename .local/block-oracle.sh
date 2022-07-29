#!/usr/bin/env bash
set -eu

. ./prelude.sh

pushd ../

cargo build

await_contract "DataEdge" "$DATA_EDGE_CONTRACT_ADDRESS"

popd
await_ready epoch-manager

await_contract "EpochManager" "$EPOCH_MANAGER_CONTRACT_ADDRESS"
await_subgraph

pushd ../
./target/debug/block-oracle ./crates/oracle/config.dev.toml || true

echo
echo "Epoch Block Oracle crashed."
echo "Press C-c to terminate this process."

while true; do
    sleep 1000
done
