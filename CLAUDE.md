# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Block Oracle is a Rust-based implementation of the [Epoch Block Oracle (GIP-0038)](https://github.com/graphprotocol/graph-improvement-proposals/blob/main/gips/0038-epoch-block-oracle.md) that enables The Graph Protocol to support indexing rewards across multiple blockchain networks. It provides a standardized mechanism to determine canonical blocks for closing allocations on different chains.

### Core Purpose
- Tracks epoch transitions across multiple blockchain networks
- Submits cross-chain block data to the Data Edge contract
- Enables indexers to close allocations with network-specific epoch block numbers
- Supports The Graph's expansion beyond Ethereum mainnet

### Components
- Core Rust oracle binary that polls blockchain data and manages epochs
- AssemblyScript subgraph for indexing oracle-related blockchain data
- Supporting crates for epoch encoding, JSON encoding, and build automation
- Kubernetes deployment configurations

## Development Commands

### Building and Running

```bash
# Build the entire workspace
cargo build

# Build with optimizations
cargo build --release

# Run the oracle with config file
cargo run --bin oracle -- run --config-file config.toml

# Encode JSON messages
cargo run --bin oracle -- encode --json-path message.json

# Get current epoch
cargo run --bin oracle -- current-epoch --config-file config.toml

# Send a message
cargo run --bin oracle -- send-message --config-file config.toml --payload "0x..."

# Run tests for entire workspace
cargo test

# Run tests for specific crate
cargo test -p oracle
cargo test -p epoch-encoding
cargo test -p json-oracle-encoder

# Run a single test
cargo test test_name -- --exact

# Run tests with output
cargo test -- --nocapture

# Format code
cargo fmt

# Check linting
cargo clippy -- -D warnings

# Run xtask commands
cargo xtask encode-message-samples

# Build Docker image
docker build -t block-oracle .
```

### AssemblyScript/Subgraph Development

```bash
# Navigate to subgraph directory
cd packages/subgraph

# Install dependencies
yarn install

# Generate types from GraphQL schema
yarn codegen

# Build the subgraph
yarn build

# Deploy to specific networks
yarn deploy-mainnet
yarn deploy-arbitrum
yarn deploy-sepolia
yarn deploy-arbitrum-sepolia

# Run tests
yarn test

# Local development
yarn create-local
yarn deploy-local
yarn remove-local
```

### Local Development Environment

```bash
# Navigate to compose directory
cd k8s/compose

# Start local development environment with Postgres and Graph Node
docker-compose up -d

# View logs
docker-compose logs -f

# Stop environment
docker-compose down

# Reset environment (removes volumes)
docker-compose down -v
```

## Architecture Overview

### Core Components

**Oracle Binary (`crates/oracle/`)**
- Main entry point in `src/main.rs` with multiple subcommands
- `Oracle` struct in `src/runner/oracle.rs` - manages the main polling loop
- `Contracts` in `src/contracts.rs` - handles smart contract interactions
- `Config` in `src/config.rs` - TOML-based configuration system
- Transaction monitoring in `src/runner/transaction_monitor.rs`

**Key Design Patterns:**
- Oracle polls multiple chains for block data at epoch boundaries
- Encodes block information using efficient compression (see GIP-0038)
- Submits messages to the Data Edge contract (gas-efficient ~25K gas)
- Integrates with subgraph for querying current epoch state
- Uses tokio for async runtime
- Supports both JSON-RPC providers and Blockmeta providers

**Oracle Message Types** (implemented from GIP-0038):
- SetBlockNumbersForNextEpoch: Update epoch block numbers for networks ✅
- RegisterNetworks: Add/remove supported networks ✅
- UpdateVersion: Change message encoding version ✅
- Reset: Clear all known chain data ✅
- CorrectEpochs: Handle chain reorganizations ❌ (TODO - not yet implemented)

**Additional Message Types** (beyond GIP-0038):
- RegisterNetworksAndAliases: Register networks with human-readable aliases
- ChangePermissions: Manage who can submit oracle updates

**Epoch Encoding (`crates/encoding/`)**
- Encodes block data into compact format for on-chain storage
- Handles message encoding/decoding
- Version-aware encoding system

**JSON Oracle Encoder (`crates/json-oracle-encoder/`)**
- Encodes JSON messages for oracle consumption
- Provides both library and CLI interfaces
- Supports calldata and payload output formats

### Configuration System

The oracle uses TOML configuration files with:
- Protocol chain configuration (the chain where oracle contracts live)
- Indexed chains configuration (chains to monitor for block data)
- Transaction monitoring settings
- Contract addresses (DataEdge, EpochManager)

Example structure in `config.toml`:
```toml
owner_address = "0x..."
owner_private_key = "0x..."
data_edge_address = "0x..."
epoch_manager_address = "0x..."
subgraph_url = "https://..."

[transaction_monitoring]
confirmation_timeout_in_seconds = 60
max_retries = 3
gas_percentual_increase = 10
confirmations = 2

[protocol_chain]
name = "eip155:1"
jrpc = "https://..."
polling_interval_in_seconds = 5

[indexed_chains]
"eip155:1" = "https://..."
"eip155:42161" = "https://..."
```

### Testing Approach

- Unit tests live alongside source files
- Integration tests in `tests/` directories
- Mock providers for testing blockchain interactions
- Use `#[tokio::test]` for async tests

### Security Considerations

- Private keys handled via environment variables or files (never in config)
- All external data should be validated before processing
- Oracle owner infrastructure must be redundant and monitored (as per GIP-0038)
- Quick resolution of fork/branch ambiguity is critical
- Rate limiting on RPC calls to prevent abuse
- Only authorized addresses can submit oracle updates

## Common Development Tasks

### Understanding the Data Edge Contract

The Data Edge contract is a minimalist design (as per GIP-0038):
- Generic fallback function that accepts any payload without executing it
- Extremely gas-efficient (~25K gas per oracle update)
- Oracle messages are sent as calldata to this contract
- The subgraph processes these calls to maintain epoch state
- Contract address is configured in `data_edge_address` in config.toml

### Adding Support for a New Chain

1. Add the chain to `indexed_chains` in config.toml with its RPC URL
2. Ensure the chain ID follows CAIP-2 format (e.g., "eip155:1" for Ethereum mainnet)
3. If using Blockmeta provider, add configuration in the blockmeta section
4. Update subgraph configuration if the chain needs indexing
5. Test the oracle can fetch blocks from the new chain
6. Note: Adding production networks requires Graph Council governance approval

### Modifying Oracle Logic

The main oracle loop is in `crates/oracle/src/runner/oracle.rs`:
- `Oracle::tick()` - main polling iteration
- Fetches latest blocks from all indexed chains
- Queries subgraph for current state
- Determines if new messages need to be submitted
- Handles epoch transitions

### Understanding Epochs

Epochs are fundamental to The Graph Protocol:
- The protocol now runs on Arbitrum One (not Ethereum mainnet)
- Each epoch is 7200 blocks (24 hours using post-merge mainnet block times)
- Different chains may have different block times, but epochs close simultaneously
- The oracle tracks the specific block number on each chain when an epoch ends
- Indexers use these block numbers to close allocations consistently across networks

### Working with Subgraph

The subgraph indexes oracle-related data:
- Schema defined in `packages/subgraph/schema.graphql`
- AssemblyScript mappings in `packages/subgraph/src/`
- Generated types in `packages/subgraph/generated/`
- Network-specific configurations in `packages/subgraph/config/`
- Processes Data Edge contract calls to track epoch transitions

To modify:
1. Update schema.graphql
2. Run `yarn codegen` to generate AssemblyScript types
3. Update mappings to handle new entities
4. Use mustache templates for network-specific deployments
5. Test locally with `yarn prep:local && yarn deploy-local`
6. Deploy to specific networks with `yarn deploy-[network]`

## Deployment

Production deployments use Kubernetes:
- Base manifests in `k8s/base/`
- Environment-specific overlays in `k8s/overlays/`
- ConfigMaps for oracle configuration
- Secrets for private keys
- Uses Kustomize for configuration management

Docker image includes:
- Multi-stage build for smaller images
- Built from root Dockerfile
- Includes all necessary runtime dependencies

### Local Testing with Docker Compose

The `k8s/compose/` directory contains a complete local development environment:
- PostgreSQL database for Graph Node
- IPFS node for subgraph deployment
- Graph Node for local subgraph testing
- Configured with environment variables for easy setup

## Development Best Practices and Lessons Learned

### Subgraph Development

**Testing Limitations**
- Subgraph tests require TTY and must be run manually by the user, not via Claude
- Use `yarn test` in the packages/subgraph directory
- Docker environment needs interactive terminal for proper test execution

**AssemblyScript Quirks**
- AssemblyScript doesn't have TypeScript's type narrowing
- Use explicit `!` operator for nullable types: `network.addedAt!`
- Null checks must be done separately before accessing fields
- Use `load()` for existence checks, then `cache.get()` for updates

**Message Encoding in Tests**
- Always use Rust encoder for generating test message bytes
- Manual VarInt encoding is error-prone and should be avoided
- Add JSON documentation comments for all encoded hex strings in tests
- Example: `// JSON: { "message": "CorrectLastEpoch", "chainId": "A1", "blockNumber": 20 }`

**Network Validation**
- Use `cache.isNetworkAlreadyRegistered(chainId)` for existence checks
- Don't manually validate network entities - use the cache methods
- CAIP-2 chain IDs are strings, not numbers: `"eip155:42161"`

### Configuration Management

**Generated Files**
- `constants.ts` is generated from `constants.template.ts` using mustache templates
- Never commit generated files - they should be in .gitignore
- Permission configurations come from config/*.json files
- Use `git rm --cached` to remove accidentally committed generated files

**Permission System**
- Permissions are configured in `packages/subgraph/config/*.json`
- Each config file specifies addresses and their allowed message types
- Constants are generated during build using mustache templating
- Production config: `arbitrum.json`, Test config: `test.json`

### CLI Command Development

**Command Structure**
- Use clap derive macros for argument parsing
- Follow existing patterns in main.rs for new commands
- Import functions at module level, not within functions
- Example structure:
  ```rust
  SomeCommand {
      #[clap(short, long)]
      config_file: PathBuf,
      #[clap(short = 'n', long)]
      chain_id: String,
  }
  ```

**Error Handling**
- Use `anyhow::Result<()>` for function returns
- Use `anyhow::bail!()` for early returns with error messages
- Validate input parameters before processing

### Repository Management

**Git Ignore Patterns**
- Use `**` for recursive patterns: `**/tests/.docker/`
- Yarn files: `.yarn/`, `.yarnrc.yml`
- Test artifacts: `tests/.docker/`
- Generated files: `constants.ts`, `subgraph.yaml`

**Commit Practices**
- Run cargo fmt, cargo clippy, and cargo test before committing
- Use descriptive commit messages with clear scope
- Include co-authorship for AI-assisted development
- Never force push unless absolutely necessary (use `--force-with-lease`)

### Common Pitfalls and Solutions

**1. VarInt Encoding Issues**
- Problem: Manual encoding gives wrong values (e.g., 15 as 0x0f instead of 0x3d)
- Solution: Always use `cargo run --bin block-oracle -- encode message.json`

**2. Network Validation Errors**
- Problem: `Cannot return null for a required field` when checking networks
- Solution: Use `cache.isNetworkAlreadyRegistered()` instead of manual checks

**3. Test Entity ID Mismatches**
- Problem: Expected "0x03-0" but got "0x03-0-0"
- Solution: Check the actual entity ID format in the handler code

**4. Import Visibility Issues**
- Problem: `function is private` when importing from other crates
- Solution: Check the crate's public API in lib.rs, use public functions only

