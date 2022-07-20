use crate::{Environment, Message};
use block_oracle::models::JrpcProviderForChain;
use block_oracle::{
    config::{Config, ConfigFile},
    contracts::Contracts,
};
use epoch_encoding::{serialize_messages, CompressedMessage};
use web3::transports::Http;

pub(crate) async fn send_message(message: Message, environment: Environment) -> anyhow::Result<()> {
    let config = resolve_config(environment)?;
    let contracts = init_contracts(&config)?;
    let payload: Vec<u8> = build_payload(message);
    contracts
        .submit_call(payload, &config.owner_private_key)
        .await?;
    Ok(())
}

fn init_contracts(config: &Config) -> anyhow::Result<Contracts<Http>> {
    let protocol_chain = {
        let transport = Http::new(config.protocol_chain.jrpc_url.as_str())?;
        JrpcProviderForChain::new(config.protocol_chain.id.clone(), transport)
    };
    Contracts::new(
        &protocol_chain.web3.eth(),
        config.data_edge_address,
        config.epoch_manager_address,
    )
}

fn resolve_config(environment: Environment) -> anyhow::Result<Config> {
    let config_path = environment.resolve_configuration_path()?;
    let config_file = ConfigFile::from_file(&config_path)?;
    Ok(Config::from_config_file(config_file))
}

fn build_payload(message: Message) -> Vec<u8> {
    let mut bytes = Vec::new();
    let message_block = match message {
        Message::Reset => vec![CompressedMessage::Reset],
    };
    serialize_messages(&message_block, &mut bytes);
    bytes
}
