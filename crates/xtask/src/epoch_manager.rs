use crate::Environment;
use anyhow::Context;
use serde::Deserialize;
use url::Url;
use web3::{contract::Contract, ethabi::Address, transports::Http, types::U256, Web3};

#[derive(Deserialize)]
struct ConfigFile {
    epoch_manager_address: Address,
    protocol_chain: ProtocolChain,
}

#[derive(Deserialize)]
pub struct ProtocolChain {
    pub jrpc: Url,
}

impl ConfigFile {
    fn new(environment: Environment) -> anyhow::Result<Self> {
        let config_path = environment.resolve_configuration_path()?;
        let text = std::fs::read_to_string(config_path)?;
        toml::from_str(&text).context("Failed to parse configuration file")
    }
}

pub async fn query(environment: Environment) -> anyhow::Result<()> {
    let config = ConfigFile::new(environment)?;
    let transport =
        Web3::new(Http::new(config.protocol_chain.jrpc.as_str()).context("Failed to parse URL")?);
    let contract: Contract<Http> = {
        let abi = include_bytes!("../../oracle/src/abi/EpochManager.json");
        Contract::from_json(transport.eth(), config.epoch_manager_address, abi)
            .context("Failed to initialize contract")?
    };
    let epoch_number: U256 = contract
        .query("currentEpoch", (), None, Default::default(), None)
        .await
        .context("Failed to query contract")?;
    println!("{}", epoch_number.as_u128());
    Ok(())
}
