pub mod config;
pub mod contracts;
pub mod metrics;
pub mod models;
pub mod runner;
pub mod subgraph;

use clap::Parser;
use contracts::Contracts;
use json_oracle_encoder::{print_encoded_json_messages, OutputKind};
use reqwest::Client;
use std::path::PathBuf;
use std::time::Duration;
use web3::transports::Http;

pub use config::Config;
pub use models::{BlockmetaProviderForChain, Caip2ChainId, JrpcProviderForChain};
pub use runner::*;
pub use subgraph::{query_subgraph, SubgraphQueryError};

pub mod blockmeta {
    pub mod blockmeta_client;
}

use runner::jrpc_utils::JrpcExpBackoff;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match Clap::parse() {
        Clap::Run { config_file } => runner::run(config_file).await?,
        Clap::Encode {
            json_path,
            calldata,
        } => {
            let file_contents = std::fs::read_to_string(json_path)?;
            let json = serde_json::from_str(&file_contents)?;
            let output_kind = if calldata {
                OutputKind::Calldata
            } else {
                OutputKind::Payload
            };
            print_encoded_json_messages(output_kind, json)?;
        }
        Clap::CurrentEpoch { config_file } => {
            let config = Config::parse(config_file);
            print_current_epoch(config).await?;
        }
        Clap::SendMessage {
            config_file,
            payload,
        } => {
            let config = Config::parse(config_file);
            let payload = hex::decode(payload)?;
            send_message(config, payload).await?;
        }
        Clap::CorrectLastEpoch {
            config_file,
            chain_id,
            block_number,
            dry_run,
            yes,
        } => {
            let config = Config::parse(config_file);
            correct_last_epoch(config, chain_id, block_number, dry_run, yes).await?;
        }
    }

    Ok(())
}

#[derive(Parser, Debug, Clone)]
#[clap(name = "block-oracle")]
#[clap(bin_name = "block-oracle")]
#[clap(author, version, about, long_about = None)]
enum Clap {
    /// Run the block oracle and regularly sends block number updates.
    Run {
        /// The path of the TOML configuration file.
        #[clap(parse(from_os_str))]
        config_file: PathBuf,
    },
    /// Compile block oracle messages from JSON to calldata.
    Encode {
        /// The path to the JSON file containing the message(s).
        json_path: PathBuf,
        /// Whether to output the full calldata instead of just the payload.
        #[clap(short, long, action)]
        calldata: bool,
    },
    /// Query the Epoch Manager for the current epoch.
    CurrentEpoch {
        /// The path of the TOML configuration file.
        #[clap(short, long)]
        config_file: PathBuf,
    },
    /// Send a message to the DataEdge contract.
    SendMessage {
        /// The path of the TOML configuration file.
        #[clap(short, long)]
        config_file: PathBuf,
        payload: String,
    },
    /// Correct the block number for a network in the latest epoch.
    CorrectLastEpoch {
        /// The path of the TOML configuration file.
        #[clap(short, long)]
        config_file: PathBuf,
        /// The CAIP-2 chain ID of the network to correct (e.g. "eip155:42161")
        #[clap(short = 'n', long)]
        chain_id: String,
        /// The corrected block number for the network (if not provided, uses current block from RPC)
        #[clap(short, long)]
        block_number: Option<u64>,
        /// Show what would be done without sending the transaction
        #[clap(long)]
        dry_run: bool,
        /// Skip confirmation prompt
        #[clap(short, long)]
        yes: bool,
    },
}

async fn send_message(config: Config, payload: Vec<u8>) -> anyhow::Result<()> {
    let private_key = config.owner_private_key;
    let contracts = init_contracts(config)?;
    let tx = contracts.submit_call(payload, &private_key).await?;
    println!("Sent message.\nTransaction hash: {tx:?}");
    Ok(())
}

async fn print_current_epoch(config: Config) -> anyhow::Result<()> {
    let contracts = init_contracts(config)?;
    let current_epoch = contracts.query_current_epoch().await?;
    println!("{current_epoch}");
    Ok(())
}

fn indexed_chains(config: &Config) -> Vec<JrpcProviderForChain<JrpcExpBackoff>> {
    config
        .indexed_chains
        .iter()
        .map(|chain| {
            let transport = JrpcExpBackoff::http(
                chain.jrpc_url.clone(),
                chain.id.clone(),
                config.retry_strategy_max_wait_time,
            );
            JrpcProviderForChain::new(chain.id.clone(), transport)
        })
        .collect()
}

fn blockmeta_indexed_chains(
    config: &Config,
) -> Vec<
    BlockmetaProviderForChain<
        tonic::codegen::InterceptedService<
            tonic::transport::Channel,
            blockmeta::blockmeta_client::AuthInterceptor,
        >,
    >,
> {
    config
        .blockmeta_indexed_chains
        .iter()
        .map(|chain| {
            BlockmetaProviderForChain::new(
                chain.id.clone(),
                chain.url.clone(),
                &config.blockmeta_auth_token,
            )
        })
        .collect()
}

async fn correct_last_epoch(
    config: Config,
    chain_id: String,
    block_number: Option<u64>,
    dry_run: bool,
    yes: bool,
) -> anyhow::Result<()> {
    use crate::runner::jrpc_utils::get_latest_block;
    use alloy_primitives::BlockHash;
    use epoch_encoding::BlockPtr;
    use json_oracle_encoder::messages_to_payload;
    use std::collections::BTreeMap;
    use std::io::{self, Write};
    use web3::helpers::CallFuture;
    use web3::types::{BlockNumber, U64};
    use web3::Transport;

    // Step 1: Query subgraph for latest epoch information
    println!("🔍 Querying subgraph for latest epoch information...");
    let subgraph_state = query_subgraph(&config.subgraph_url, &config.bearer_token).await?;

    let global_state = subgraph_state.global_state.ok_or_else(|| {
        anyhow::anyhow!("Subgraph has no global state. Has the oracle been initialized?")
    })?;

    let latest_epoch_number = global_state
        .latest_epoch_number
        .ok_or_else(|| anyhow::anyhow!("No latest epoch found in subgraph"))?;

    println!("   Latest epoch: {}", latest_epoch_number);
    println!("   Registered networks: {}", global_state.networks.len());

    // Verify the target chain exists in registered networks
    let target_network = global_state
        .networks
        .iter()
        .find(|n| n.id.as_str() == chain_id)
        .ok_or_else(|| {
            anyhow::anyhow!("Chain ID '{}' is not registered in the oracle", chain_id)
        })?;

    println!(
        "   Target network array index: {}",
        target_network.array_index
    );

    // Step 2: Initialize RPC clients for all networks
    println!("📡 Setting up RPC clients for all networks...");
    let indexed_chains = indexed_chains(&config);
    let blockmeta_indexed_chains = blockmeta_indexed_chains(&config);

    // Step 3: Get corrected block number for target network
    let corrected_block_number = match block_number {
        Some(num) => {
            println!("   Using provided block number: {}", num);
            num
        }
        None => {
            println!("   Auto-detecting current block for {}...", chain_id);

            // Try to find the target chain in JSON-RPC providers first
            let mut found_block = None;
            for jrpc_chain in &indexed_chains {
                if jrpc_chain.chain_id.as_str() == chain_id {
                    let latest_block =
                        get_latest_block(jrpc_chain.web3.clone())
                            .await
                            .map_err(|e| {
                                anyhow::anyhow!(
                                    "Failed to get latest block from {}: {}",
                                    chain_id,
                                    e
                                )
                            })?;
                    found_block = Some(latest_block.number);
                    println!("     Current block from JSON-RPC: {}", latest_block.number);
                    break;
                }
            }

            // If not found in JSON-RPC, try Blockmeta providers
            if found_block.is_none() {
                for blockmeta_chain in &blockmeta_indexed_chains {
                    if blockmeta_chain.chain_id.as_str() == chain_id {
                        let mut client = blockmeta_chain.client.clone();
                        let latest_block = client.get_latest_block().await?.ok_or_else(|| {
                            anyhow::anyhow!("No latest block found from Blockmeta for {}", chain_id)
                        })?;
                        found_block = Some(latest_block.num);
                        println!("     Current block from Blockmeta: {}", latest_block.num);
                        break;
                    }
                }
            }

            found_block.ok_or_else(|| {
                anyhow::anyhow!(
                    "Chain '{}' not found in either JSON-RPC or Blockmeta providers",
                    chain_id
                )
            })?
        }
    };

    // Step 4: Get block numbers for all networks from the latest epoch
    println!("🔍 Collecting block data from latest epoch for all networks...");
    let mut epoch_blocks: BTreeMap<Caip2ChainId, (u64, u64)> = BTreeMap::new(); // (block_number, array_index)

    for network in &global_state.networks {
        if let Some(block_update) = &network.latest_block_update {
            if block_update.updated_at_epoch_number == latest_epoch_number {
                epoch_blocks.insert(
                    network.id.clone(),
                    (block_update.block_number, network.array_index),
                );
                println!(
                    "   {}: block {} (index {})",
                    network.id.as_str(),
                    block_update.block_number,
                    network.array_index
                );
            }
        }
    }

    if epoch_blocks.is_empty() {
        anyhow::bail!("No networks have block data for epoch {}. This might indicate the epoch is too recent.", latest_epoch_number);
    }

    // Step 5: Fetch block hashes for all networks using their epoch block numbers
    println!("🔗 Fetching block hashes for merkle root computation...");
    let mut all_blocks: BTreeMap<Caip2ChainId, BlockPtr> = BTreeMap::new();

    // Fetch from JSON-RPC providers
    for jrpc_chain in &indexed_chains {
        if let Some((block_num, _array_index)) = epoch_blocks.get(&jrpc_chain.chain_id) {
            let use_corrected_block = jrpc_chain.chain_id.as_str() == chain_id;
            let target_block_number = if use_corrected_block {
                corrected_block_number
            } else {
                *block_num
            };

            // Get block by number
            let block_id =
                web3::helpers::serialize(&BlockNumber::Number(U64::from(target_block_number)));
            let include_txs = web3::helpers::serialize(&false);
            let fut = jrpc_chain
                .web3
                .transport()
                .execute("eth_getBlockByNumber", vec![block_id, include_txs]);

            #[derive(serde::Deserialize)]
            struct BlockResponse {
                hash: web3::types::H256,
                number: U64,
            }

            let call_fut: CallFuture<BlockResponse, _> = CallFuture::new(fut);
            let block = call_fut.await.map_err(|e| {
                anyhow::anyhow!(
                    "Failed to get block {} from {}: {}",
                    target_block_number,
                    jrpc_chain.chain_id.as_str(),
                    e
                )
            })?;

            let block_ptr = BlockPtr {
                number: block.number.as_u64(),
                hash: block.hash.0,
            };

            all_blocks.insert(jrpc_chain.chain_id.clone(), block_ptr);

            if use_corrected_block {
                println!(
                    "   {} (CORRECTED): block {} -> hash {}",
                    jrpc_chain.chain_id.as_str(),
                    target_block_number,
                    hex::encode(block_ptr.hash)
                );
            } else {
                println!(
                    "   {}: block {} -> hash {}",
                    jrpc_chain.chain_id.as_str(),
                    target_block_number,
                    hex::encode(block_ptr.hash)
                );
            }
        }
    }

    // Fetch from Blockmeta providers
    for blockmeta_chain in &blockmeta_indexed_chains {
        if let Some((block_num, _array_index)) = epoch_blocks.get(&blockmeta_chain.chain_id) {
            let use_corrected_block = blockmeta_chain.chain_id.as_str() == chain_id;
            let target_block_number = if use_corrected_block {
                corrected_block_number
            } else {
                *block_num
            };

            // Get block by number using Blockmeta gRPC
            let mut client = blockmeta_chain.client.clone();
            let request = crate::blockmeta::blockmeta_client::NumToIdReq {
                block_num: target_block_number,
            };

            let block_resp = client.num_to_id(request).await?;

            let block_hash = block_resp
                .id
                .parse::<BlockHash>()
                .map_err(|e| anyhow::anyhow!("Invalid block hash from Blockmeta: {}", e))?;

            let block_ptr = BlockPtr {
                number: block_resp.num,
                hash: block_hash.0,
            };

            all_blocks.insert(blockmeta_chain.chain_id.clone(), block_ptr);

            if use_corrected_block {
                println!(
                    "   {} (CORRECTED): block {} -> hash {}",
                    blockmeta_chain.chain_id.as_str(),
                    target_block_number,
                    hex::encode(block_ptr.hash)
                );
            } else {
                println!(
                    "   {}: block {} -> hash {}",
                    blockmeta_chain.chain_id.as_str(),
                    target_block_number,
                    hex::encode(block_ptr.hash)
                );
            }
        }
    }

    // Step 6: Compute merkle root using the same algorithm as the oracle
    println!("🧮 Computing merkle root...");

    // Use the encoder to compute the merkle root by creating a temporary SetBlockNumbersForNextEpoch message
    let available_networks: Vec<(String, epoch_encoding::Network)> = {
        global_state
            .networks
            .iter()
            .map(|network| (network.id.as_str().to_owned(), network.clone().into()))
            .collect()
    };

    let mut encoder =
        epoch_encoding::Encoder::new(epoch_encoding::CURRENT_ENCODING_VERSION, available_networks)
            .expect("Failed to create encoder");

    // Create a temporary message with our corrected blocks to compute the merkle root
    let message = epoch_encoding::Message::SetBlockNumbersForNextEpoch(
        all_blocks
            .iter()
            .map(|(chain_id, block_ptr)| (chain_id.as_str().to_owned(), *block_ptr))
            .collect(),
    );

    let compressed = encoder
        .compress(&[message])
        .expect("Failed to compress message for merkle root computation");

    let computed_merkle_root = if let Some(compressed_msg) = compressed.first() {
        if let Some((_, root)) = compressed_msg.as_non_empty_block_numbers() {
            root
        } else {
            anyhow::bail!("Expected non-empty block numbers message for merkle root computation");
        }
    } else {
        anyhow::bail!("Failed to compress message for merkle root computation");
    };

    println!(
        "   Computed merkle root: 0x{}",
        hex::encode(computed_merkle_root)
    );

    // Step 7: Display correction summary
    println!();
    println!("📋 Correction Summary:");
    println!("   Epoch: {}", latest_epoch_number);
    println!("   Network: {}", chain_id);
    println!("   New block number: {}", corrected_block_number);
    println!(
        "   New merkle root: 0x{}",
        hex::encode(computed_merkle_root)
    );
    println!("   Total networks in merkle tree: {}", all_blocks.len());

    if dry_run {
        println!();
        println!("🏃 Dry run complete. No transaction submitted.");
        return Ok(());
    }

    if !yes {
        print!("\n❓ This will submit a correction to the blockchain. Are you sure you want to proceed? (y/N): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().to_lowercase().starts_with('y') {
            println!("❌ Correction cancelled.");
            return Ok(());
        }
    }

    // Step 8: Create and submit the CorrectLastEpoch message
    println!("📤 Creating CorrectLastEpoch message...");
    let json_message = serde_json::json!([{
        "message": "CorrectLastEpoch",
        "chainId": chain_id,
        "blockNumber": corrected_block_number,
        "merkleRoot": format!("0x{}", hex::encode(computed_merkle_root))
    }]);

    let payload = messages_to_payload(json_message)?;
    println!("   Message payload: {} bytes", payload.len());

    println!("🚀 Submitting transaction...");
    let contracts = init_contracts(config.clone())?;
    let tx = contracts
        .submit_call(payload, &config.owner_private_key)
        .await?;

    println!("✅ CorrectLastEpoch message submitted successfully!");
    println!("   Transaction hash: {tx:?}");
    println!("   The subgraph will process this correction in the next few minutes.");

    Ok(())
}

fn init_contracts(config: Config) -> anyhow::Result<Contracts<Http>> {
    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .unwrap();
    let transport = Http::with_client(client, config.protocol_chain.jrpc_url);
    let protocol_chain = JrpcProviderForChain::new(config.protocol_chain.id, transport);
    Contracts::new(
        protocol_chain.web3,
        config.data_edge_address,
        config.epoch_manager_address,
        config.transaction_monitoring_options,
    )
}
