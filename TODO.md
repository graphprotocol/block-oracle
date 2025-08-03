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

#### 3.3 Migration to CAIP-2 Chain IDs - DONE
- Changed from numeric network IDs to CAIP-2 chain ID strings (e.g., "eip155:42161")
- Updated message structure to use `chainId` string instead of `network_id` integer
- Updated all tests to use string chain IDs
- Fixed network validation to use `cache.isNetworkAlreadyRegistered()`

#### 3.4 Permission System Updates - DONE
- Added CorrectLastEpochMessage permission to production config (arbitrum.json)
- Added CorrectLastEpochMessage permission to test config (test.json)
- Updated constants generation from config files using mustache templates

#### 3.5 Test Implementation - DONE
- Added comprehensive tests for CorrectLastEpoch message handling
- Fixed VarInt encoding issues by using Rust encoder for all test messages
- Added JSON documentation comments for all encoded hex strings
- Fixed network validation for invalid networks
- All subgraph tests now pass

### 🔄 4. Create Manual Correction Tool - IN PROGRESS

Create CLI command to send CorrectLastEpoch messages:

- [🔄] Add subcommand to oracle binary:
  ```rust
  CorrectLastEpoch {
      #[clap(short, long)]
      config_file: PathBuf,
      #[clap(short = 'n', long)]
      chain_id: String,
      #[clap(short, long)]
      block_number: u64,
      #[clap(short, long)]
      merkle_root: String,
  }
  ```

- [🔄] Implementation logic:
  1. Query subgraph for latest epoch and current state
  2. For the specified network:
     - If block number provided, use it
     - Otherwise, query RPC for current block
  3. Fetch block hashes for ALL networks in the epoch:
     - For the network being corrected: use the new block number
     - For all other networks: use the block numbers from the subgraph (NOT current blocks)
  4. Compute new merkle root with corrected values
  5. Display correction summary and prompt for confirmation (unless --yes)
  6. If --dry-run, exit without sending
  7. Generate and submit message

Current implementation status:
- ✅ CLI argument parsing updated with correct structure
- ✅ Dry-run and confirmation prompts implemented
- 🔄 Core logic needs implementation:
  - ⏳ Subgraph querying for latest epoch data
  - ⏳ RPC client initialization for all networks
  - ⏳ Block hash fetching from multiple networks
  - ⏳ Merkle root computation using epoch-encoding crate
  - ⏳ Message creation and submission

Example usage:
```bash
# Dry run - see what would happen without sending
cargo run --bin block-oracle -- correct-last-epoch \
  --config-file config.toml \
  --chain-id "eip155:42161" \
  --block-number 12345 \
  --dry-run

# Correct with confirmation prompt  
cargo run --bin block-oracle -- correct-last-epoch \
  --config-file config.toml \
  --chain-id "eip155:42161" \
  --block-number 12345

# Auto-detect current block and skip confirmation
cargo run --bin block-oracle -- correct-last-epoch \
  --config-file config.toml \
  --chain-id "eip155:42161" \
  --yes

# Short form
cargo run --bin block-oracle -- correct-last-epoch \
  -c config.toml -n "eip155:1" -b 18500000 -y
```

### ✅ 5. Testing Strategy - COMPLETED

- ✅ Unit tests for encoding/decoding in Rust
- ✅ Integration tests for subgraph handler (all passing)
- ✅ Test edge cases:
  - ✅ Correcting when only one epoch exists
  - ✅ Correcting networks that weren't in original message
  - ✅ Invalid network IDs
  - ✅ Missing epochs to correct
  - ✅ Proper delta and acceleration calculations

### ✅ 6. Security Considerations - ADDRESSED

- ✅ Only authorized addresses can submit corrections (existing security model)
- ✅ Complete audit trail via LastEpochCorrection entities in subgraph
- ✅ Permission system properly configured for production and test environments

## 📋 Implementation Summary

**Status: 95% Complete** - Core functionality implemented and tested

### What's Done ✅
1. **Rust Message Definition** - CorrectLastEpoch message type with CAIP-2 chain IDs
2. **Encoding/Serialization** - Full implementation with comprehensive tests
3. **JSON Encoder Support** - Complete with validation and examples
4. **Subgraph Schema** - New entities for message and audit trail
5. **Subgraph Handler** - Full implementation with proper validation
6. **Permission System** - Production and test configurations updated
7. **Comprehensive Testing** - All edge cases covered, tests passing
8. **Infrastructure** - .gitignore updates, constants.ts cleanup

### What's In Progress 🔄
1. **CLI Command** - Structure complete, core logic needs implementation:
   - ✅ Argument parsing with correct options (dry-run, confirmation, optional block number)
   - ✅ User interface with emojis and clear prompts
   - ⏳ Subgraph integration for epoch data
   - ⏳ Multi-network RPC client setup
   - ⏳ Merkle root computation

### Current Status
The CLI command structure is complete and ready for testing the user interface:
```bash
# Test the help and dry-run functionality
cargo run --bin block-oracle -- correct-last-epoch --help
cargo run --bin block-oracle -- correct-last-epoch -c config.toml -n "eip155:42161" -b 12345 --dry-run
```

**Next Steps:** Implement the core logic for subgraph querying, RPC integration, and merkle root computation.

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