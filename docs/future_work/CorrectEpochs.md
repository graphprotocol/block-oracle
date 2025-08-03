# Future Work: CorrectEpochs Message Implementation

## Overview
Full implementation of CorrectEpochs message type to allow correction of any past epoch (except epoch 0). This is more complex than CorrectLastEpoch due to cascade effects on subsequent epochs.

## Problem Statement
Need to implement the CorrectEpochs message type to:
- Fix incorrect block numbers posted for past epochs
- Handle chain reorganizations (as per GIP-0038)
- Maintain consistency across all affected subsequent epochs

## Message Structure

```rust
// In crates/encoding/src/messages.rs
CorrectEpochs {
    epoch_number: u64,      // Which epoch we're correcting
    merkle_root: Bytes32,   // New merkle root for the entire epoch (verifiable offchain)
    data_by_network_id: BTreeMap<NetworkIndex, u64>,  // network_id -> corrected block number
}
```

## Key Complexity: Cascade Updates

When correcting epoch N, ALL subsequent epochs (N+1, N+2, ...) need updating because:
- Each epoch's delta depends on the previous epoch's block number
- Each epoch's acceleration depends on the previous epoch's delta
- This creates a cascade effect through all future epochs

Example:
```
Epoch N:   delta[N] = blockNumber[N] - blockNumber[N-1]
           acceleration[N] = delta[N] - delta[N-1]

Epoch N+1: delta[N+1] = blockNumber[N+1] - correctedBlockNumber[N]  // Changed!
           acceleration[N+1] = delta[N+1] - correctedDelta[N]       // Changed!
```

## Implementation Requirements

### 1. Subgraph Handler
- Validate epoch_number <= globalState.latestValidEpoch
- Validate epoch_number > 0 (no correcting first epoch)
- Update the corrected epoch
- Cascade updates through ALL subsequent epochs for affected networks
- Create audit trail entries

### 2. Constraints
- Cannot correct epoch 0 (too complex)
- Cannot correct future epochs
- Correcting old epochs requires updating many subsequent epochs (performance concern)

### 3. Schema Additions

```graphql
type CorrectEpochsMessage implements Message @entity {
  id: ID!
  block: MessageBlock!
  data: Bytes
  epochNumber: BigInt!
  newMerkleRoot: Bytes!
  corrections: [EpochCorrection!]!
}

type EpochCorrection @entity {
  id: ID!
  message: CorrectEpochsMessage!
  network: Network!
  newAcceleration: BigInt!
  newDelta: BigInt!
  newBlockNumber: BigInt!
  previousAcceleration: BigInt!
  previousDelta: BigInt!
  previousBlockNumber: BigInt!
}
```

## Why This Is Complex

1. **Cascade Logic**: Must correctly update all subsequent epochs
2. **Performance**: Correcting old epochs could require updating hundreds of subsequent epochs
3. **Testing**: Many edge cases due to cascade effects
4. **Risk**: Bugs in cascade logic could corrupt the entire epoch chain

## Migration from CorrectLastEpoch

Once CorrectLastEpoch is implemented and tested, extending to CorrectEpochs requires:
1. Adding epoch_number to the message
2. Implementing cascade update logic
3. Adding validation for epoch constraints
4. Extensive testing of cascade effects

## Open Questions

1. Should we limit how far back corrections can go (e.g., max 10 epochs)?
2. How to handle performance if correcting very old epochs?
3. Should cascade updates be done in batches to avoid timeout?