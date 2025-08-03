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

async fn correct_last_epoch(
    config: Config,
    chain_id: String,
    block_number: Option<u64>,
    dry_run: bool,
    yes: bool,
) -> anyhow::Result<()> {
    use json_oracle_encoder::messages_to_payload;
    use std::io::{self, Write};
    
    println!("🔍 Querying subgraph for latest epoch information...");
    
    // TODO: Query subgraph for latest epoch and get all network block numbers
    // For now, we'll implement a simplified version that shows the structure
    
    println!("📡 Getting current block information for network: {}", chain_id);
    
    // TODO: Initialize RPC clients for all networks to get block hashes
    // TODO: Get current block for the target network if block_number not provided
    
    let corrected_block_number = match block_number {
        Some(num) => {
            println!("   Using provided block number: {}", num);
            num
        }
        None => {
            println!("   Querying RPC for current block...");
            // TODO: Query RPC for current block
            anyhow::bail!("Auto-detection of current block not yet implemented. Please provide --block-number");
        }
    };
    
    // TODO: Fetch block hashes for all networks in the epoch
    // TODO: Compute merkle root using the same algorithm as the oracle
    
    println!();
    println!("📋 Correction Summary:");
    println!("   Network: {}", chain_id);
    println!("   New block number: {}", corrected_block_number);
    println!("   ⚠️  Merkle root computation not yet implemented");
    
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
    
    // TODO: Implement the complete logic:
    // 1. Query subgraph for latest epoch data
    // 2. Get all network block numbers from that epoch  
    // 3. Query RPCs for block hashes (using corrected block for target network)
    // 4. Compute merkle root using epoch-encoding crate
    // 5. Create and send the message
    
    anyhow::bail!("Full implementation not yet complete. This is a placeholder showing the correct CLI structure.");
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
