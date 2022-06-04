#!/usr/bin/env bash
set -eu

. ./prelude.sh

docker_run postgres \
  -p "${POSTGRES_PORT}:5432" \
  -e POSTGRES_HOST_AUTH_METHOD=trust \
  -e "POSTGRES_USER=${POSTGRES_USER}" \
  -e "POSTGRES_PASSWORD=${POSTGRES_PASSWORD}" \
  postgres
