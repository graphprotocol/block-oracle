use crate::store::models::Caip2ChainId;
use clap::Parser;
use secp256k1::SecretKey;
use serde::{Deserialize, Deserializer};
use std::{
    collections::HashMap,
    fs::read_to_string,
    path::{Path, PathBuf},
    str::FromStr,
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
    pub jrpc_providers: HashMap<Caip2ChainId, Url>,
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
            jrpc_providers: config_file.jrpc_providers,
        }
    }

    pub fn networks(&self) -> Vec<Caip2ChainId> {
        self.jrpc_providers.keys().into_iter().cloned().collect()
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
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct ConfigFile {
    #[serde(deserialize_with = "deserialize_jrpc_providers")]
    jrpc_providers: HashMap<Caip2ChainId, Url>,
}

impl ConfigFile {
    pub fn from_file<P: AsRef<Path>>(file_path: P) -> Result<Self, ConfigError> {
        let string = read_to_string(file_path)?;
        toml::de::from_str(&string).map_err(ConfigError::Toml)
    }
}

/// Helper function to deserialize [`Caip2ChainId`] keys
fn deserialize_jrpc_providers<'de, D>(
    deserializer: D,
) -> Result<HashMap<Caip2ChainId, Url>, D::Error>
where
    D: Deserializer<'de>,
{
    let values = HashMap::<String, Url>::deserialize(deserializer)?;
    let mut map = HashMap::new();
    for (key, value) in values.into_iter() {
        let caip2 = key.parse::<Caip2ChainId>().map_err(|()| {
            serde::de::Error::custom("failed to parse chain name as a CAIP-2 compliant string")
        })?;
        map.insert(caip2, value);
    }
    Ok(map)
}

#[test]
fn test_config_file_deserialize() {
    let text = r#"
[jrpc-providers]
"eip155:1" = "https://example.com"
"eip155:2" = "https://another.com"
"#;

    let parsed: ConfigFile = toml::de::from_str(text).expect("deserialization works");
    let key1: Caip2ChainId = "eip155:1".parse().unwrap();
    let value1: Url = "https://example.com".parse().unwrap();
    assert_eq!(parsed.jrpc_providers.get(&key1), Some(&value1));

    let key2: Caip2ChainId = "eip155:2".parse().unwrap();
    let value2: Url = "https://another.com".parse().unwrap();
    assert_eq!(parsed.jrpc_providers.get(&key2), Some(&value2));
}
