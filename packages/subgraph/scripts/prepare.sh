#!/bin/bash

cat ./abis/DataEdge.json | jq --slurpfile value ./scripts/selectors/crossChainEpochOracle.json '. += $value' > ./abis/DataEdgeFull.json
