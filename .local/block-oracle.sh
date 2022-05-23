#!/usr/bin/env bash
set -eu

. ./prelude.sh

cd ../crates/oracle/

cargo build

await_contract
await_subgraph

cargo run -- \
	--config-file=config/dev/config.toml \
	--database-url=:memory: \
	--owner-private-key=4f3edf983ac636a65a842ce7c78d9aa706d3b113bce9c46f30d7d21715b23b1d
