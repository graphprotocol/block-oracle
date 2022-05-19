#!/usr/bin/env bash
set -eu

. ./prelude.sh

docker_run ipfs \
  -p "${IPFS_PORT}:5001" \
  ipfs/go-ipfs:master
