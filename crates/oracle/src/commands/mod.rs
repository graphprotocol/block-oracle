pub mod correct_epoch;
pub mod current_epoch;
pub mod send_message;

pub use correct_epoch::correct_last_epoch;
pub use current_epoch::print_current_epoch;
pub use send_message::send_message;

use crate::contracts::Contracts;
use crate::{Config, JrpcProviderForChain};
use reqwest::Client;
use std::time::Duration;
use web3::transports::Http;

pub(crate) fn init_contracts(config: Config) -> anyhow::Result<Contracts<Http>> {
    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .unwrap();
    let transport = Http::with_client(client, config.protocol_chain.jrpc_url);
    let protocol_chain = JrpcProviderForChain::new(config.protocol_chain.id, transport);
    Contracts::new(
        protocol_chain.web3,
        config.data_edge_address,
        config.epoch_manager_address,
        config.transaction_monitoring_options,
    )
}
