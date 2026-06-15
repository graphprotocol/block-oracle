pub mod chain_validation;
pub mod commands;
pub mod config;
pub mod contracts;
pub mod metrics;
pub mod models;
pub mod runner;
pub mod subgraph;

use clap::Parser;
use json_oracle_encoder::{print_encoded_json_messages, OutputKind};
use std::path::PathBuf;

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
            commands::print_current_epoch(config).await?;
        }
        Clap::SendMessage {
            config_file,
            payload,
        } => {
            let config = Config::parse(config_file);
            let payload = hex::decode(payload)?;
            commands::send_message(config, payload).await?;
        }
        Clap::CorrectLastEpoch {
            config_file,
            chain_id,
            block_number,
            dry_run,
            yes,
            skip_merkle,
        } => {
            let config = Config::parse(config_file);
            commands::correct_last_epoch(config, chain_id, block_number, dry_run, yes, skip_merkle)
                .await?;
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
        /// Skip merkle root computation and use 0x0 instead
        #[clap(long)]
        skip_merkle: bool,
    },
}
