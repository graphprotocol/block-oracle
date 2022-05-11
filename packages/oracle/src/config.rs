use crate::{
    indexed_chain::IndexedChain, protocol_chain::ProtocolChain, store::models::Caip2ChainId,
};
use clap::Parser;
use secp256k1::SecretKey;
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs::read_to_string,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use thiserror::Error;
use tracing_subscriber::filter::LevelFilter;
use url::Url;
use web3::types::H160;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Error deserializing config file")]
    Toml(#[from] toml::de::Error),
}

const DEFAULT_EPOCH_DURATION: u64 = 6_646;
const DEFAULT_PROTOCOL_CHAIN_POLLING_INTERVAL_IN_SECONDS: u64 = 120;
const DEFAULT_WEB3_TRANSPORT_RETRY_MAX_WAIT_TIME_IN_SECONDS: u64 = 60;

pub struct Config {
    pub log_level: LevelFilter,
    pub owner_address: H160,
    pub owner_private_key: SecretKey,
    pub contract_address: H160,
    pub database_url: String,
    pub epoch_duration: u64,
    pub protocol_chain_polling_interval: Duration,
    pub indexed_chains: Arc<Vec<IndexedChain>>,
    pub protocol_chain: Arc<ProtocolChain>,
}

impl Config {
    pub fn parse() -> Self {
        let clap = Clap::parse();
        let config_file =
            ConfigFile::from_file(&clap.config_file).expect("Failed to read config file.");

        let retry_strategy_max_wait_time =
            Duration::from_secs(config_file.web3_transport_retry_max_wait_time_in_seconds);

        Self {
            log_level: clap.log_level,
            owner_address: config_file.owner_address.parse().unwrap(),
            owner_private_key: SecretKey::from_str(clap.owner_private_key.as_str()).unwrap(),
            contract_address: config_file.contract_address.parse().unwrap(),
            database_url: clap.database_url,
            epoch_duration: config_file.epoch_duration,
            protocol_chain_polling_interval: Duration::from_secs(
                config_file.protocol_chain_polling_interval_in_seconds,
            ),
            indexed_chains: Arc::new(
                config_file
                    .indexed_chains
                    .into_iter()
                    .map(|(chain_id, url)| {
                        IndexedChain::new(chain_id, url, retry_strategy_max_wait_time)
                    })
                    .collect(),
            ),
            protocol_chain: Arc::new(ProtocolChain::new(
                config_file.protocol_chain.name,
                config_file.protocol_chain.jrpc,
                retry_strategy_max_wait_time,
            )),
        }
    }
}

#[derive(Parser, Debug, Clone)]
#[clap(name = "block-oracle")]
#[clap(bin_name = "block-oracle")]
#[clap(author, version, about, long_about = None)]
struct Clap {
    /// The private key for the oracle owner account.
    #[clap(long)]
    owner_private_key: String,
    /// Only show log messages at or above this level. `INFO` by default.
    #[clap(short, long, default_value = "info")]
    log_level: LevelFilter,
    /// The Ethereum address of the Data Edge smart contract.
    #[clap(long)]
    database_url: String,
    /// The filepath of the TOML JSON-RPC configuration file.
    #[clap(long, default_value = "config.toml", parse(from_os_str))]
    config_file: PathBuf,
}

/// Represents the TOML config file
#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
struct ConfigFile {
    owner_address: String,
    contract_address: String,
    indexed_chains: HashMap<Caip2ChainId, Url>,
    protocol_chain: SerdeProtocolChain,
    #[serde(default = "ConfigFile::default_epoch_duration")]
    epoch_duration: u64,
    #[serde(default = "ConfigFile::default_protocol_chain_polling_interval_in_seconds")]
    protocol_chain_polling_interval_in_seconds: u64,
    #[serde(default = "ConfigFile::default_web3_transport_retry_max_wait_time_in_seconds")]
    web3_transport_retry_max_wait_time_in_seconds: u64,
}

impl ConfigFile {
    /// Tries to Create a [`ConfigFile`] from a TOML file.
    fn from_file(file_path: &Path) -> Result<Self, ConfigError> {
        let string = read_to_string(file_path)?;
        toml::from_str(&string).map_err(ConfigError::Toml)
    }

    fn default_epoch_duration() -> u64 {
        DEFAULT_EPOCH_DURATION
    }

    fn default_protocol_chain_polling_interval_in_seconds() -> u64 {
        DEFAULT_PROTOCOL_CHAIN_POLLING_INTERVAL_IN_SECONDS
    }

    fn default_web3_transport_retry_max_wait_time_in_seconds() -> u64 {
        DEFAULT_WEB3_TRANSPORT_RETRY_MAX_WAIT_TIME_IN_SECONDS
    }
}

#[derive(Deserialize, Debug)]
struct SerdeProtocolChain {
    name: Caip2ChainId,
    jrpc: Url,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_CONFIG: &str = include_str!("../config/dev/config.toml");

    #[test]
    fn deserialize_protocol_chain() {
        toml::de::from_str::<ConfigFile>(SAMPLE_CONFIG).unwrap();
    }
}
