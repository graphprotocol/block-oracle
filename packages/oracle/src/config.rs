use crate::{
    indexed_chain::IndexedChain, protocol_chain::ProtocolChain, store::models::Caip2ChainId,
};
use clap::Parser;
use secp256k1::SecretKey;
use serde::{Deserialize, Deserializer};
use std::{
    collections::HashMap,
    fs::read_to_string,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use thiserror::Error;
use url::Url;
use web3::types::H160;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Error deserializing config file")]
    Toml(#[from] toml::de::Error),
}

pub struct Config {
    pub owner_address: H160,
    pub owner_private_key: SecretKey,
    pub contract_address: H160,
    pub database_url: String,
    pub epoch_duration: u64,
    pub json_rpc_polling_interval: Duration,
    pub indexed_chains: Arc<Vec<IndexedChain>>,
    pub protocol_chain_client: Arc<ProtocolChain>,
}

impl Config {
    pub fn parse() -> Self {
        let clap = Clap::parse();
        let config_file =
            ConfigFile::from_file(&clap.config_file).expect("Failed to read config file.");
        Self {
            owner_address: clap.owner_address.parse().unwrap(),
            owner_private_key: SecretKey::from_str(clap.owner_private_key.as_str()).unwrap(),
            contract_address: clap.contract_address.parse().unwrap(),
            database_url: clap.database_url,
            epoch_duration: clap.epoch_duration,
            json_rpc_polling_interval: Duration::from_secs(
                clap.json_rpc_polling_interval_in_seconds,
            ),
            indexed_chains: Arc::new(config_file.indexed_chains),
            protocol_chain_client: Arc::new(config_file.protocol_chain),
        }
    }

    pub fn networks(&self) -> Vec<Caip2ChainId> {
        self.indexed_chains
            .iter()
            .map(|chain| chain.id())
            .cloned()
            .collect()
    }
}

#[derive(Parser, Debug, Clone)]
#[clap(name = "block-oracle")]
#[clap(bin_name = "block-oracle")]
#[clap(author, version, about, long_about = None)]
struct Clap {
    /// The Ethereum address of the oracle owner account.
    #[clap(long)]
    owner_address: String,
    /// The private key for the oracle owner account.
    #[clap(long)]
    owner_private_key: String,
    /// The Ethereum address of the Data Edge smart contract.
    #[clap(long)]
    contract_address: String,
    /// The Ethereum address of the Data Edge smart contract.
    #[clap(long)]
    database_url: String,
    /// The epoch length of the oracle, expressed in blocks.
    #[clap(long, default_value = "6646")]
    epoch_duration: u64,
    /// Approximate waiting period between two consecutive polls to the same
    /// JSON-RPC provider.
    #[clap(long, default_value = "120")]
    json_rpc_polling_interval_in_seconds: u64,
    /// The filepath of the TOML JSON-RPC configuration file.
    #[clap(long, default_value = "config.toml", parse(from_os_str))]
    config_file: PathBuf,
}

/// Represents the TOML config file
#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
struct ConfigFile {
    #[serde(deserialize_with = "deserialize_indexed_chains")]
    indexed_chains: Vec<IndexedChain>,
    #[serde(deserialize_with = "deserialize_protocol_chain")]
    protocol_chain: ProtocolChain,
}

impl ConfigFile {
    /// Tries to Create a [`ConfigFile`] from a TOML file.
    pub fn from_file<P: AsRef<Path>>(file_path: P) -> Result<Self, ConfigError> {
        let string = read_to_string(file_path)?;
        toml::de::from_str(&string).map_err(ConfigError::Toml)
    }
}

fn deserialize_protocol_chain<'de, D>(deserializer: D) -> Result<ProtocolChain, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    use serde::de::Unexpected;
    let record = toml::Value::deserialize(deserializer)?;
    let name = record
        .get("name")
        .ok_or_else(|| Error::missing_field("name"))?
        .as_str()
        .ok_or_else(|| Error::invalid_type(Unexpected::Other("unknown"), &"string"))?;
    let jrpc = record
        .get("jrpc")
        .ok_or_else(|| Error::missing_field("jrpc"))?
        .as_str()
        .ok_or_else(|| Error::invalid_type(Unexpected::Other("unknown"), &"string"))?;
    let chain_id = name.parse::<Caip2ChainId>().map_err(|()| {
        Error::invalid_value(Unexpected::Str(name), &"a valid CAIP-2 network identifier")
    })?;
    let url = jrpc
        .parse::<Url>()
        .map_err(|_| Error::invalid_type(Unexpected::Str(jrpc), &"a valid URL"))?;
    Ok(ProtocolChain::new(chain_id, url))
}

fn deserialize_indexed_chains<'de, D>(deserializer: D) -> Result<Vec<IndexedChain>, D::Error>
where
    D: Deserializer<'de>,
{
    let values = HashMap::<String, Url>::deserialize(deserializer)?;
    let mut indexed_chains = Vec::new();
    for (key, value) in values.into_iter() {
        let caip2 = key.parse::<Caip2ChainId>().map_err(|()| {
            serde::de::Error::custom("failed to parse chain name as a CAIP-2 compliant string")
        })?;
        let indexed_chain = IndexedChain::new(caip2, value);
        indexed_chains.push(indexed_chain);
    }
    Ok(indexed_chains)
}

#[cfg(test)]
mod tests {

    use super::*;
    const SAMPLE_CONFIG: &str = include_str!("../config.sample.toml");

    #[test]
    fn deserialize_protocol_chain() {
        let config: ConfigFile = toml::de::from_str(SAMPLE_CONFIG).unwrap();
        dbg!(config);
    }
}
