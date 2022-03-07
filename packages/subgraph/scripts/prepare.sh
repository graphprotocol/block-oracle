#!/bin/bash

cat ./abis/DataVault.json | jq --slurpfile value ./scripts/selectors/postMessageBlocks.json '. += $value' > ./abis/DataVaultFull.json
