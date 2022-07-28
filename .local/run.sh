#!/usr/bin/env bash
set -eu

PROCESS_NAME=${1:-block-oracle}

rm .overmind.sock || true

overmind start --daemonize
sleep 2
overmind connect "$PROCESS_NAME"
