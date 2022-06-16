#!/usr/bin/env bash
set -eu

. ./prelude.sh

github_clone graphprotocol/graph-node master
cd build/graphprotocol/graph-node

cargo build -p graph-node

await "curl --silent --fail localhost:${ETHEREUM_PORT} -o /dev/null"
echo "Hardhat is up"

# graph-node has issues if the chain has no blocks, so we just make sure at least one exists
curl --silent --fail "localhost:${ETHEREUM_PORT}" -X POST --data '{"jsonrpc":"2.0","method":"evm_mine","params":[],"id":1}' -o /dev/null
echo "Requested Hardhat to mine a block"

await "curl --silent --fail localhost:${IPFS_PORT}" 22
echo "IPFS is up"

await "curl --silent --fail localhost:$POSTGRES_PORT" 52
echo "Postgres is up"

dropdb -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U "$POSTGRES_USER" "$POSTGRES_DB" --no-password || true
createdb -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U "$POSTGRES_USER" "$POSTGRES_DB" --no-password

echo "Created database"

./target/debug/graph-node
