use crate::models::Caip2ChainId;
use anyhow::Context;
use clap::Parser;
use secp256k1::SecretKey;
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs::read_to_string,
    path::{Path, PathBuf},
    str::FromStr,
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

#[derive(Clone, Debug)]
pub struct Config {
    pub log_level: LevelFilter,
    pub owner_private_key: SecretKey,
    pub contract_address: H160,
    pub subgraph_url: Url,
    pub owner_address: H160,
    pub epoch_duration: u64,
    pub indexed_chains: Vec<IndexedChain>,
    pub freshness_threshold: u64,
    pub protocol_chain: ProtocolChain,
    pub retry_strategy_max_wait_time: Duration,
}

#[derive(Clone, Debug)]
pub struct IndexedChain {
    pub id: Caip2ChainId,
    pub jrpc_url: Url,
}

#[derive(Clone, Debug)]
pub struct ProtocolChain {
    pub id: Caip2ChainId,
    pub jrpc_url: Url,
    pub polling_interval: Duration,
}

impl Config {
    /// Loads all configuration options from CLI arguments, the TOML
    /// configuration file, and environment variables.
    pub fn parse() -> Self {
        let clap = Clap::parse();
        let config_file = ConfigFile::from_file(&clap.config_file)
            .context("Failed to read config file as valid TOML")
            .unwrap();

        Self::from_clap_and_config_file(clap, config_file)
    }

    #[cfg(test)]
    fn parse_from(args: &[&str]) -> Self {
        let clap = Clap::parse_from(args);
        println!("clap config is {:?}", clap.config_file);
        let config_file = ConfigFile::from_file(&clap.config_file).unwrap();

        Self::from_clap_and_config_file(clap, config_file)
    }

    fn from_clap_and_config_file(clap: Clap, config_file: ConfigFile) -> Self {
        let retry_strategy_max_wait_time =
            Duration::from_secs(config_file.web3_transport_retry_max_wait_time_in_seconds);

        Self {
            log_level: clap.log_level,
            owner_private_key: SecretKey::from_str(clap.owner_private_key.as_str()).unwrap(),
            contract_address: config_file.contract_address.parse().unwrap(),
            subgraph_url: clap.subgraph_url,
            freshness_threshold: config_file.freshness_threshold,
            epoch_duration: config_file.epoch_duration,
            owner_address: config_file.owner_address.parse().unwrap(),
            retry_strategy_max_wait_time,
            indexed_chains: config_file
                .indexed_chains
                .into_iter()
                .map(|(id, provider)| IndexedChain {
                    id,
                    jrpc_url: parse_jrpc_provider_url(&provider)
                        .expect("Bad JSON-RPC provider url"),
                })
                .collect::<Vec<IndexedChain>>(),
            protocol_chain: ProtocolChain {
                id: config_file.protocol_chain.name,
                jrpc_url: parse_jrpc_provider_url(&config_file.protocol_chain.jrpc)
                    .expect("Invalid protocol chain JSON-RPC provider url"),
                polling_interval: Duration::from_secs(
                    config_file.protocol_chain_polling_interval_in_seconds,
                ),
            },
        }
    }
}

fn parse_jrpc_provider_url(s: &str) -> anyhow::Result<Url> {
    if let Ok(url) = Url::parse(s) {
        Ok(url)
    } else {
        Ok(Url::parse(std::env::var(s)?.as_str())?)
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
    /// The subgraph endpoint.
    #[clap(long)]
    subgraph_url: Url,
    /// The filepath of the TOML JSON-RPC configuration file.
    #[clap(long, parse(from_os_str))]
    config_file: PathBuf,
}

/// Represents the TOML config file
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct ConfigFile {
    owner_address: String,
    contract_address: String,
    indexed_chains: HashMap<Caip2ChainId, String>,
    protocol_chain: SerdeProtocolChain,
    /// Number of blocks that the Epoch Subgraph may be away from the protocol chain's head. If the
    /// block distance is lower than this, a `trace_filter` JSON RPC call will be used to infer if
    /// any relevant transaction happened within that treshold.
    #[serde(default = "serde_defaults::freshness_threshold")]
    freshness_threshold: u64,
    #[serde(default = "serde_defaults::epoch_duration")]
    epoch_duration: u64,
    #[serde(default = "serde_defaults::protocol_chain_polling_interval_in_seconds")]
    protocol_chain_polling_interval_in_seconds: u64,
    #[serde(default = "serde_defaults::web3_transport_retry_max_wait_time_in_seconds")]
    web3_transport_retry_max_wait_time_in_seconds: u64,
    #[serde(default = "serde_defaults::transaction_confirmation_poll_interval_in_seconds")]
    transaction_confirmation_poll_interval_in_seconds: u64,
    #[serde(default = "serde_defaults::transaction_confirmation_count")]
    transaction_confirmation_count: usize,
}

impl ConfigFile {
    /// Tries to Create a [`ConfigFile`] from a TOML file.
    fn from_file(file_path: &Path) -> Result<Self, ConfigError> {
        let string = read_to_string(file_path)?;
        toml::from_str(&string).map_err(ConfigError::Toml)
    }
}

/// These should be expressed as constants once
/// https://github.com/serde-rs/serde/issues/368 is fixed.
#[allow(unused)]
mod serde_defaults {
    pub fn freshness_threshold() -> u64 {
        10
    }

    pub fn epoch_duration() -> u64 {
        6_646
    }

    pub fn protocol_chain_polling_interval_in_seconds() -> u64 {
        120
    }

    pub fn web3_transport_retry_max_wait_time_in_seconds() -> u64 {
        60
    }

    pub fn transaction_confirmation_poll_interval_in_seconds() -> u64 {
        5
    }

    pub fn transaction_confirmation_count() -> usize {
        0
    }
}

#[derive(Deserialize, Debug)]
struct SerdeProtocolChain {
    name: Caip2ChainId,
    jrpc: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn indexed_chain(config: &Config, id: &str) -> IndexedChain {
        config
            .indexed_chains
            .iter()
            .find(|x| x.id.as_str() == id)
            .unwrap()
            .clone()
    }

    fn config_file_flag(filename: &str) -> String {
        format!(
            "--config-file={}/config/{}",
            env!("CARGO_MANIFEST_DIR"),
            filename
        )
    }

    #[test]
    #[should_panic]
    fn invalid_jrpc_provider_url() {
        Config::parse_from(&[
            "",
            "--subgraph-url=https://example.com",
            "--owner-private-key=4f3edf983ac636a65a842ce7c78d9aa706d3b113bce9c46f30d7d21715b23b1d",
            config_file_flag("test/invalid_jrpc_provider_url.toml").as_str(),
        ]);
    }

    #[test]
    fn example_config() {
        Config::parse_from(&[
            "",
            "--subgraph-url=https://example.com",
            "--owner-private-key=4f3edf983ac636a65a842ce7c78d9aa706d3b113bce9c46f30d7d21715b23b1d",
            config_file_flag("dev/config.toml").as_str(),
        ]);
    }

    #[test]
    fn set_jrpc_provider_via_env_var() {
        let jrpc_url = "https://sokol-archive.blockscout.com/";
        std::env::set_var("FOOBAR_EIP155:77", jrpc_url);

        let config = Config::parse_from(&[
            "",
            "--subgraph-url=https://example.com",
            "--owner-private-key=4f3edf983ac636a65a842ce7c78d9aa706d3b113bce9c46f30d7d21715b23b1d",
            config_file_flag("test/indexed_chain_provider_via_env_var.toml").as_str(),
        ]);

        assert_eq!(
            indexed_chain(&config, "eip155:77").jrpc_url.as_str(),
            jrpc_url
        );
    }
}
