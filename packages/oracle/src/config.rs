use clap::Parser;
use secp256k1::key::SecretKey;
use std::str::FromStr;
use web3::types::H160;

pub struct Config {
    pub owner_address: H160,
    pub owner_private_key: SecretKey,
    pub contract_address: H160,
}

impl Config {
    pub fn parse() -> Self {
        let clap = Clap::parse();
        Self {
            owner_address: clap.owner_address.parse().unwrap(),
            owner_private_key: SecretKey::from_str(clap.owner_private_key.as_str()).unwrap(),
            contract_address: clap.contract_address.parse().unwrap(),
        }
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
}
