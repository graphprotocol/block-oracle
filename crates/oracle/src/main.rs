pub mod config;
pub mod contracts;
pub mod encode_json_messages;
pub mod models;
pub mod runner;
pub mod subgraph;

use clap::Parser;
use contracts::Contracts;
use encode_json_messages::{print_encoded_json_messages, OutputKind};
use std::path::PathBuf;
use web3::transports::Http;

pub use config::Config;
pub use models::{Caip2ChainId, JrpcProviderForChain};
pub use runner::*;
pub use subgraph::{SubgraphApi, SubgraphQuery, SubgraphQueryError, SubgraphStateTracker};

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
    println!("{}", current_epoch);
    Ok(())
}

fn init_contracts(config: Config) -> anyhow::Result<Contracts<Http>> {
    let transport = Http::new(config.protocol_chain.jrpc_url.as_str())?;
    let protocol_chain = JrpcProviderForChain::new(config.protocol_chain.id, transport);
    Contracts::new(
        &protocol_chain.web3.eth(),
        config.data_edge_address,
        config.epoch_manager_address,
    )
}
