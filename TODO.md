# TODO: Implement CorrectLastEpoch Message Handling

## Problem Statement
Need to implement the CorrectLastEpoch message type to fix incorrect block numbers in the most recent epoch. This is a simplified version that avoids the complexity of correcting historical epochs and their cascade effects.

## Current State
- CorrectEpochs message type is defined but not implemented across the stack
- Rust encoding: Has TODO comment about including hash, count, and merkle root
- Subgraph handler: Empty function with just `// TODO.`
- JSON encoder: Empty struct with `// TODO.`

## Implementation Progress

### ✅ 1. Define Message Structure - COMPLETED

The CorrectLastEpoch message:
- Always corrects the latest epoch (no epoch_number parameter needed)
- Corrects exactly ONE network per message (send multiple messages for multiple networks)
- Includes new merkle root for verification

Implemented in `crates/encoding/src/messages.rs`:
- Added `CorrectLastEpoch` variant with `network_id`, `block_number`, `merkle_root`
- Message type 7 assigned, default updated to 8

### ✅ 2. Update Rust Implementation - COMPLETED

#### 2.1 Encoding crate - DONE
- Added `CorrectLastEpoch` to Message and CompressedMessage enums
- Updated `str_to_u64` mapping
- Implemented `serialize_correct_last_epoch` in serialize.rs
- Added comprehensive unit tests

#### 2.2 JSON encoder - DONE
- Added JSON structure with camelCase fields
- Implemented conversion from JSON to encoding format
- Created example JSON file: `06-correct-last-epoch.json`
- Added tests for valid/invalid JSON parsing

### ✅ 3. Update Subgraph Implementation - COMPLETED

#### 3.1 Schema Updates - DONE
- Added `CorrectLastEpochMessage` entity with `newMerkleRoot` field
- Added `LastEpochCorrection` entity for audit trail
- Includes previous/new values for block number, acceleration, and delta

#### 3.2 Handler Implementation - DONE
- Added `CorrectLastEpochMessage` to MessageTag enum (value 7)
- Implemented `executeCorrectLastEpochMessage` handler
- Added getters to StoreCache for new entities
- Fixed `epochBlockNumberId` to accept string parameter
- Handler validates epoch exists, parses message, updates values
- Properly handles AssemblyScript nullable types with `!` operator

### 4. Create Manual Correction Tool

Create CLI command to send CorrectLastEpoch messages:

- [ ] Add subcommand to oracle binary:
  ```rust
  #[derive(Parser)]
  enum Commands {
      // ... existing commands ...
      CorrectLastEpoch {
          #[clap(long)]
          config_file: PathBuf,
          #[clap(long)]
          network: String,  // Single CAIP-2 ID
          #[clap(long)]
          block_number: Option<u64>,  // Optional specific block
          #[clap(long)]
          dry_run: bool,  // Show what would be done without sending
          #[clap(long)]
          yes: bool,  // Skip confirmation prompt
      }
  }
  ```

- [ ] Implementation logic:
  1. Query subgraph for latest epoch and current state
  2. For the specified network:
     - If block number provided, use it
     - Otherwise, query RPC for current block
  3. Fetch block hashes for ALL networks in the epoch (with correction applied)
  4. Compute new merkle root with corrected values
  5. Display correction summary:
     ```
     Correction Summary:
     - Epoch: 123
     - Network: eip155:42161 (Arbitrum One)
     - Current block: 12345
     - New block: 12350
     - New merkle root: 0xabc...def
     ```
  6. If dry_run, exit here showing what would be sent
  7. If not --yes, prompt for confirmation:
     ```
     This will submit a correction to the blockchain.
     Are you sure you want to proceed? (y/N):
     ```
  8. Generate and submit message
  9. Display transaction hash and status

Example usage:
```bash
# Dry run - see what would happen without sending
cargo run --bin oracle -- correct-last-epoch \
  --config-file config.toml \
  --network "eip155:42161" \
  --dry-run

# Correct with confirmation prompt
cargo run --bin oracle -- correct-last-epoch \
  --config-file config.toml \
  --network "eip155:42161"

# Skip confirmation (useful for scripts)
cargo run --bin oracle -- correct-last-epoch \
  --config-file config.toml \
  --network "eip155:42161" \
  --block-number 12345 \
  --yes

# To correct multiple networks, send multiple messages:
cargo run --bin oracle -- correct-last-epoch --config-file config.toml --network "eip155:42161" --yes
cargo run --bin oracle -- correct-last-epoch --config-file config.toml --network "eip155:1" --yes
```

### 5. Testing Strategy

- [ ] Unit tests for encoding/decoding
- [ ] Integration test for subgraph handler
- [ ] End-to-end test on local environment:
  1. Deploy contracts and subgraph
  2. Submit some epochs
  3. Submit correction for last epoch
  4. Verify values updated correctly
- [ ] Test edge cases:
  - Correcting when only one epoch exists
  - Correcting networks that weren't in original message
  - Invalid network IDs

### 6. Security Considerations

- [ ] Only authorized addresses can submit corrections (existing security model)
- [ ] Add logging for all corrections for audit trail
- [ ] Consider rate limiting corrections

## Design Decisions

1. **No Epoch Parameter**: Always corrects the latest epoch, simplifying validation
2. **No Cascade Effects**: Since it's the last epoch, no subsequent epochs need updating
3. **Merkle Root Required**: For offchain verification of the correction
4. **Flexible Network Selection**: Only correct networks that need it, not all
5. **Preserve Original Messages**: Don't modify SetBlockNumbersForEpochMessage - keep for audit trail
6. **Offchain Reconstruction**: Observers can reconstruct final state from original + corrections

## Key Advantages Over Full CorrectEpochs

1. **Simpler Implementation**: No cascade update logic needed
2. **Lower Risk**: Can't corrupt historical data
3. **Faster Development**: ~1/3 the complexity
4. **Immediate Need**: Solves the current problem quickly

## Future Migration Path

Once CorrectLastEpoch is working:
1. Most code can be reused for full CorrectEpochs
2. Add epoch_number parameter
3. Add cascade update logic
4. Add historical epoch validation

## Additional Safety and Reliability Features

### 7. RPC Chain ID Verification

Add startup validation to ensure each RPC endpoint corresponds to the correct chain:

- [ ] On oracle startup, query `eth_chainId` from each configured RPC
- [ ] Verify it matches the expected chain ID from the CAIP-2 identifier
- [ ] Fail fast with clear error message if mismatch detected
- [ ] Example: RPC configured for "eip155:42161" must return chain ID 42161

This prevents misconfiguration errors where an RPC URL points to the wrong chain.

### 8. Backup RPC Configuration

Add support for fallback RPC endpoints for reliability:

- [ ] Update config structure to support multiple RPC URLs per network
- [ ] Implement automatic failover when primary RPC is unavailable
- [ ] Log RPC switches for monitoring
- [ ] Example config:
  ```toml
  [indexed_chains]
  "eip155:42161" = {
    primary = "https://primary-rpc.example.com"
    backups = ["https://backup1.example.com", "https://backup2.example.com"]
  }
  ```

## Next Steps

1. Start with Rust message definition
2. Implement encoding/serialization
3. Add subgraph handler
4. Create CLI tool
5. Test on local environment
6. Deploy fix for current issue