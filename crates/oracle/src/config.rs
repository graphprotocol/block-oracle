use crate::models::Caip2ChainId;
use anyhow::Context;
use secp256k1::SecretKey;
use serde::Deserialize;
use serde_utils::{EitherLiteralOrEnvVar, FromStrWrapper};
use std::{
    collections::HashMap, fmt::Display, fs::read_to_string, path::Path, str::FromStr,
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

#[derive(Clone, Deserialize, Debug, Copy)]
pub struct TransactionMonitoringOptions {
    #[serde(default = "serde_defaults::transaction_monitoring_confirmation_timeout_in_seconds")]
    /// How long to wait for a transaction to be confirmed
    pub confirmation_timeout_in_seconds: u64,
    #[serde(default = "serde_defaults::transaction_monitoring_max_retries")]
    /// How many times it has tried to rebroadcast the original transaction.
    pub max_retries: u32,
    /// Gas price percentual increase
    #[serde(default = "serde_defaults::transaction_monitoring_gas_percentual_increase")]
    pub gas_percentual_increase: u32,
    /// How much time to wait between querying the JSON RPC provider for confirmations
    #[serde(default = "serde_defaults::transaction_monitoring_poll_interval_in_seconds")]
    pub poll_interval_in_seconds: u64,
    /// How many confirmations to wait for
    #[serde(default = "serde_defaults::transaction_monitoring_confirmations")]
    pub confirmations: usize,
    #[serde(default = "serde_defaults::transaction_monitoring_gas_limit")]
    pub gas_limit: u64,
    #[serde(default)]
    pub max_fee_per_gas: Option<u64>,
    #[serde(default)]
    pub max_priority_fee_per_gas: Option<u64>,
}

impl Default for TransactionMonitoringOptions {
    fn default() -> Self {
        use serde_defaults::*;
        Self {
            confirmation_timeout_in_seconds: transaction_monitoring_confirmation_timeout_in_seconds(
            ),
            max_retries: transaction_monitoring_max_retries(),
            gas_percentual_increase: transaction_monitoring_gas_percentual_increase(),
            poll_interval_in_seconds: transaction_monitoring_poll_interval_in_seconds(),
            confirmations: transaction_monitoring_confirmations(),
            gas_limit: transaction_monitoring_gas_limit(),
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub log_level: LevelFilter,
    pub owner_private_key: SecretKey,
    pub data_edge_address: H160,
    pub epoch_manager_address: H160,
    pub subgraph_url: Url,
    pub bearer_token: String,
    pub owner_address: H160,
    pub indexed_chains: Vec<IndexedChain>,
    pub freshness_threshold: u64,
    pub protocol_chain: ProtocolChain,
    pub retry_strategy_max_wait_time: Duration,
    pub metrics_port: u16,
    pub transaction_monitoring_options: TransactionMonitoringOptions,
}

impl Config {
    /// Loads all configuration options the provided TOML configuration file and environment
    /// variables.
    pub fn parse(config_file: impl AsRef<Path>) -> Self {
        let config_file = ConfigFile::from_file(config_file.as_ref())
            .context("Failed to read config file as valid TOML")
            .unwrap();

        Self::from_config_file(config_file)
    }

    fn from_config_file(config_file: ConfigFile) -> Self {
        Self {
            log_level: config_file.log_level.0,
            owner_private_key: config_file.owner_private_key.0,
            data_edge_address: config_file.data_edge_address.0,
            epoch_manager_address: config_file.epoch_manager_address.0,
            subgraph_url: config_file.subgraph_url.0,
            bearer_token: config_file.bearer_token.0,
            freshness_threshold: config_file.freshness_threshold,
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
            metrics_port: config_file.metrics_port,
            transaction_monitoring_options: config_file.transaction_monitoring_options,
        }
    }
}

/// Represents the TOML config file
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct ConfigFile {
    owner_address: FromStrWrapper<H160>,
    owner_private_key: EitherLiteralOrEnvVar<SecretKey>,
    data_edge_address: EitherLiteralOrEnvVar<H160>,
    epoch_manager_address: EitherLiteralOrEnvVar<H160>,
    subgraph_url: EitherLiteralOrEnvVar<Url>,
    bearer_token: EitherLiteralOrEnvVar<String>,
    /// Number of blocks that the Epoch Subgraph may be away from the protocol chain's head. If the
    /// block distance is lower than this, a `trace_filter` JSON RPC call will be used to infer if
    /// any relevant transaction happened within that treshold.
    #[serde(default = "serde_defaults::freshness_threshold")]
    freshness_threshold: u64,
    #[serde(default = "serde_defaults::web3_transport_retry_max_wait_time_in_seconds")]
    web3_transport_retry_max_wait_time_in_seconds: u64,
    #[serde(default = "serde_defaults::log_level")]
    log_level: FromStrWrapper<LevelFilter>,
    protocol_chain: SerdeProtocolChain,
    indexed_chains: HashMap<Caip2ChainId, EitherLiteralOrEnvVar<Url>>,
    #[serde(default = "serde_defaults::metrics_port")]
    metrics_port: u16,
    #[serde(rename = "transaction_monitoring")]
    transaction_monitoring_options: TransactionMonitoringOptions,
}

impl ConfigFile {
    /// Tries to Create a [`ConfigFile`] from a TOML file.
    fn from_file(file_path: &Path) -> Result<Self, ConfigError> {
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
mod serde_defaults {
    use super::serde_utils::FromStrWrapper;
    use tracing_subscriber::filter::LevelFilter;

    pub fn log_level() -> FromStrWrapper<LevelFilter> {
        FromStrWrapper(LevelFilter::INFO)
    }

    pub fn freshness_threshold() -> u64 {
        10
    }

    pub fn protocol_chain_polling_interval_in_seconds() -> u64 {
        120
    }

    pub fn web3_transport_retry_max_wait_time_in_seconds() -> u64 {
        60
    }

    pub fn transaction_monitoring_confirmation_timeout_in_seconds() -> u64 {
        120
    }

    pub fn transaction_monitoring_max_retries() -> u32 {
        10
    }

    pub fn transaction_monitoring_gas_percentual_increase() -> u32 {
        50 // 50%
    }

    pub fn transaction_monitoring_poll_interval_in_seconds() -> u64 {
        5
    }

    pub fn transaction_monitoring_confirmations() -> usize {
        2
    }

    pub fn transaction_monitoring_gas_limit() -> u64 {
        100_000
    }

    pub fn metrics_port() -> u16 {
        9090
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
        format!("{}/test/config/{}", env!("CARGO_MANIFEST_DIR"), filename)
    }

    #[test]
    #[should_panic]
    fn invalid_jrpc_provider_url() {
        Config::parse(config_file_path("invalid_jrpc_provider_url.toml"));
    }

    #[test]
    fn example_config() {
        Config::parse(config_file_path("config.sample.toml"));
    }

    #[test]
    fn set_jrpc_provider_via_env_var() {
        let jrpc_url = "https://sokol-archive.blockscout.com/";
        std::env::set_var("FOOBAR_EIP155:77", jrpc_url);

        let config = Config::parse(config_file_path("indexed_chain_provider_via_env_var.toml"));

        assert_eq!(
            indexed_chain(&config, "eip155:77").jrpc_url.as_str(),
            jrpc_url
        );
    }
}
