#!/usr/bin/env bash

cargo run -- \
	--config-file=config/dev/config.toml \
	--database-url=postgresql://postgres:letmein@localhost:5432/block-oracle \
	--owner-private-key=75dc16000b877ea0d4f764281c4c3fb8a047a7a0219361ac0bc82f325bc6ef1d
