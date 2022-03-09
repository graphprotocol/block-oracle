#!/bin/bash

cat ./abis/DataEdge.json | jq --slurpfile value ./scripts/selectors/postMessageBlocks.json '. += $value' > ./abis/DataEdgeFull.json
