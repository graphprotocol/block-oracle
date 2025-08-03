# Context for CorrectLastEpoch Implementation

## Current Situation
- An incorrect block number was posted for a specific chain in the current epoch
- Need to implement a correction mechanism quickly
- Full CorrectEpochs (for historical corrections) is too complex for immediate needs

## Key Technical Context

### How the Oracle Works
1. **Data Flow**: Oracle collects BlockPtr (number + hash) → compresses to accelerations/deltas → sends merkle root + accelerations on-chain
2. **Block hashes are NEVER sent on-chain** - only merkle roots and accelerations
3. **Subgraph reconstructs block numbers** from accelerations using: `blockNumber[N] = blockNumber[N-1] + delta[N]`

### Why CorrectLastEpoch is Simpler
- No cascade updates needed (it's the last epoch, no subsequent epochs exist)
- No complex delta recalculation for future epochs
- Single network correction per message keeps it atomic

### Implementation Approach
- Implementing CorrectLastEpoch first (simple, immediate need)
- CorrectEpochs documented in docs/future_work/ for later
- Using message type 7 (need to update default case to 8)

### Key Design Decisions Made
1. **One network per message** - simpler than BTreeMap
2. **Don't modify original SetBlockNumbersForEpochMessage** - preserve for audit trail
3. **Include new merkle root** - for offchain verification
4. **CLI with --dry-run and confirmation prompt** - safety first
5. **Keep merkle root instead of just block hash** - ensures complete verifiability even if original data was garbage

### Technical Gotchas
- Network IDs in subgraph are strings ("0", "1"), not u64
- StoreCache needs explicit save() calls
- Merkle root computation needs ALL networks' data, not just the corrected one

### What's in TODO.md
- Complete implementation plan for CorrectLastEpoch
- All code snippets and structure
- Testing strategy
- CLI design with safety features

### What's NOT in TODO.md but Important
- The protocol now runs on Arbitrum One (not Ethereum)
- Epochs are 7200 blocks (24 hours)
- This is implementing part of GIP-0038
- The immediate problem: wrong block posted for current epoch
- Original incorrect block data might be garbage/unobtainable

### Next Steps When Resuming
1. Start with Rust message definition (crates/encoding/src/messages.rs)
2. Implement serialization
3. Add JSON encoder support
4. Implement subgraph handler
5. Create CLI command
6. Test locally before deploying fix