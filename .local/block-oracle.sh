#!/usr/bin/env bash
set -eu

. ./prelude.sh

cd ../

cargo build

await_contract
await_subgraph

./target/debug/block-oracle \
	--config-file=./crates/oracle/config/dev/config.toml \
	--subgraph-url="http://127.0.0.1:${GRAPH_NODE_GRAPHQL_PORT}/subgraphs/name/edgeandnode/block-oracle" \
	--owner-private-key=4f3edf983ac636a65a842ce7c78d9aa706d3b113bce9c46f30d7d21715b23b1d || true


echo
echo "Epoch Block Oracle crashed."
echo "Press C-c to terminate this process."

while true; do
    sleep 1000
done
