#!/usr/bin/env bash
set -eu

. ./prelude.sh

await "curl --silent --fail localhost:${ETHEREUM_PORT}" # hardhat

cd ../packages/oracle/

cargo run -- \
	--config-file=config/dev/config.toml \
	--database-url=:memory: \
	--owner-private-key=4f3edf983ac636a65a842ce7c78d9aa706d3b113bce9c46f30d7d21715b23b1d
