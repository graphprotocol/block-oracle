use crate::models::Caip2ChainId;
use anyhow::Context;
use clap::Parser;
use secp256k1::SecretKey;
use serde::Deserialize;
use serde_utils::{EitherLiteralOrEnvVar, FromStrWrapper};
use std::{
    collections::HashMap,
    fmt::Display,
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

#[derive(Clone, Debug)]
pub struct Config {
    pub log_level: LevelFilter,
    pub owner_private_key: SecretKey,
    pub data_edge_address: H160,
    pub epoch_manager_address: H160,
    pub subgraph_url: Url,
    pub owner_address: H160,
    pub epoch_duration: u64,
    pub indexed_chains: Vec<IndexedChain>,
    pub freshness_threshold: u64,
    pub protocol_chain: ProtocolChain,
    pub retry_strategy_max_wait_time: Duration,
}

impl Config {
    /// Loads all configuration options from CLI arguments, the TOML
    /// configuration file, and environment variables.
    pub fn parse() -> Self {
        let clap = Clap::parse();
        let config_file = ConfigFile::from_file(&clap.config_file)
            .context("Failed to read config file as valid TOML")
            .unwrap();

        Self::from_config_file(config_file)
    }

    #[cfg(test)]
    fn parse_from(args: &[&str]) -> Self {
        let clap = Clap::parse_from(args);
        let config_file = ConfigFile::from_file(&clap.config_file).unwrap();

        Self::from_config_file(config_file)
    }

    pub fn from_config_file(config_file: ConfigFile) -> Self {
        Self {
            log_level: config_file.log_level.0,
            owner_private_key: config_file.owner_private_key.0,
            data_edge_address: config_file.data_edge_address.0,
            epoch_manager_address: config_file.epoch_manager_address.0,
            subgraph_url: config_file.subgraph_url.0,
            freshness_threshold: config_file.freshness_threshold,
            epoch_duration: config_file.epoch_duration,
            owner_address: config_file.owner_address.0,
            retry_strategy_max_wait_time: Duration::from_secs(
                config_file.web3_transport_retry_max_wait_time_in_seconds,
            ),
            indexed_chains: config_file
                .indexed_chains
                .into_iter()
                .map(|(id, provider)| IndexedChain {
                    id,
                    jrpc_url: provider.0,
                })
                .collect::<Vec<IndexedChain>>(),
            protocol_chain: ProtocolChain {
                id: config_file.protocol_chain.name,
                jrpc_url: config_file.protocol_chain.jrpc.0,
                polling_interval: Duration::from_secs(
                    config_file.protocol_chain.polling_interval_in_seconds,
                ),
            },
        }
    }
}

#[derive(Parser, Debug, Clone)]
#[clap(name = "block-oracle")]
#[clap(bin_name = "block-oracle")]
#[clap(author, version, about, long_about = None)]
struct Clap {
    /// The filepath of the TOML configuration file.
    #[clap(parse(from_os_str))]
    config_file: PathBuf,
}

/// Represents the TOML config file
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ConfigFile {
    owner_address: FromStrWrapper<H160>,
    owner_private_key: EitherLiteralOrEnvVar<SecretKey>,
    data_edge_address: FromStrWrapper<H160>,
    epoch_manager_address: FromStrWrapper<H160>,
    subgraph_url: EitherLiteralOrEnvVar<Url>,
    /// Number of blocks that the Epoch Subgraph may be away from the protocol chain's head. If the
    /// block distance is lower than this, a `trace_filter` JSON RPC call will be used to infer if
    /// any relevant transaction happened within that treshold.
    #[serde(default = "serde_defaults::freshness_threshold")]
    freshness_threshold: u64,
    #[serde(default = "serde_defaults::epoch_duration")]
    epoch_duration: u64,
    #[serde(default = "serde_defaults::web3_transport_retry_max_wait_time_in_seconds")]
    web3_transport_retry_max_wait_time_in_seconds: u64,
    #[serde(default = "serde_defaults::transaction_confirmation_poll_interval_in_seconds")]
    transaction_confirmation_poll_interval_in_seconds: u64,
    #[serde(default = "serde_defaults::transaction_confirmation_count")]
    transaction_confirmation_count: usize,
    #[serde(default = "serde_defaults::log_level")]
    log_level: FromStrWrapper<LevelFilter>,
    protocol_chain: SerdeProtocolChain,
    indexed_chains: HashMap<Caip2ChainId, EitherLiteralOrEnvVar<Url>>,
}

impl ConfigFile {
    /// Tries to Create a [`ConfigFile`] from a TOML file.
    pub fn from_file(file_path: &Path) -> Result<Self, ConfigError> {
        let string = read_to_string(file_path)?;
        toml::from_str(&string).map_err(ConfigError::Toml)
    }
}

#[derive(Deserialize, Debug)]
struct SerdeProtocolChain {
    name: Caip2ChainId,
    jrpc: EitherLiteralOrEnvVar<Url>,
    #[serde(default = "serde_defaults::protocol_chain_polling_interval_in_seconds")]
    polling_interval_in_seconds: u64,
}

mod serde_utils {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct EitherLiteralOrEnvVar<T>(pub T);

    impl<'de, T> serde::Deserialize<'de> for EitherLiteralOrEnvVar<T>
    where
        T: FromStr,
        T::Err: Display,
    {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let raw_s = String::deserialize(deserializer)?;
            let value_s = if raw_s.starts_with('$') {
                std::env::var(raw_s.strip_prefix('$').unwrap()).map_err(serde::de::Error::custom)?
            } else {
                raw_s
            };
            Ok(EitherLiteralOrEnvVar(
                T::from_str(&value_s).map_err(serde::de::Error::custom)?,
            ))
        }
    }

    pub struct FromStrWrapper<T>(pub T);

    impl<'de, T> Deserialize<'de> for FromStrWrapper<T>
    where
        T: FromStr,
        T::Err: Display,
    {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let s = String::deserialize(deserializer)?;
            FromStr::from_str(&s)
                .map(Self)
                .map_err(serde::de::Error::custom)
        }
    }
}

/// These should be expressed as constants once
/// https://github.com/serde-rs/serde/issues/368 is fixed.
#[allow(unused)]
mod serde_defaults {
    use super::serde_utils::FromStrWrapper;
    use tracing_subscriber::filter::LevelFilter;

    pub fn log_level() -> FromStrWrapper<LevelFilter> {
        FromStrWrapper(LevelFilter::INFO)
    }

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

    fn config_file_path(filename: &str) -> String {
        format!("{}/config/{}", env!("CARGO_MANIFEST_DIR"), filename)
    }

    #[test]
    #[should_panic]
    fn invalid_jrpc_provider_url() {
        Config::parse_from(&[
            "",
            config_file_path("test/invalid_jrpc_provider_url.toml").as_str(),
        ]);
    }

    #[test]
    fn example_config() {
        Config::parse_from(&["", config_file_path("test/config.sample.toml").as_str()]);
    }

    #[test]
    fn set_jrpc_provider_via_env_var() {
        let jrpc_url = "https://sokol-archive.blockscout.com/";
        std::env::set_var("FOOBAR_EIP155:77", jrpc_url);

        let config = Config::parse_from(&[
            "",
            config_file_path("test/indexed_chain_provider_via_env_var.toml").as_str(),
        ]);

        assert_eq!(
            indexed_chain(&config, "eip155:77").jrpc_url.as_str(),
            jrpc_url
        );
    }
}
