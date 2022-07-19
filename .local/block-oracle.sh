#!/usr/bin/env bash
set -eu

. ./prelude.sh

cd ../

cargo build

await_contract
await_subgraph

./target/debug/block-oracle ./crates/oracle/config/dev/config.toml || true

echo
echo "Epoch Block Oracle crashed."
echo "Press C-c to terminate this process."

while true; do
    sleep 1000
done
