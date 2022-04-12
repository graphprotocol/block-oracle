use crate::store::Caip2ChainId;
use futures::{
    stream::{FuturesUnordered, StreamExt},
    FutureExt,
};
use std::collections::{hash_map::Entry, HashMap};
use thiserror::Error;
use web3::{api::Web3, transports::http::Http as HttpTransport, types::U64};

type BlockNumber = U64;
type Client = Web3<HttpTransport>;

#[derive(Error, Debug)]
pub enum EventSourceError {
    #[error("Ethereum client error")]
    Web3(#[from] web3::Error),
}

/// Actively listens for new blocks and reorgs from registered blockchains. Also, it checks the
/// number of confirmations for transactions sent to the DataEdge contract.
pub struct EventSource {
    jrpc_providers: HashMap<Caip2ChainId, Client>,
}

impl EventSource {
    pub fn new(jrpc_providers: HashMap<Caip2ChainId, Client>) -> Self {
        Self { jrpc_providers }
    }

    async fn get_latest_blocks(
        &mut self,
    ) -> Result<HashMap<&Caip2ChainId, BlockNumber>, EventSourceError> {
        let mut block_number_per_chain: HashMap<&Caip2ChainId, BlockNumber> = HashMap::new();

        let mut tasks = self
            .jrpc_providers
            .iter()
            .map(|(chain, client)| client.eth().block_number().map(move |res| (chain, res)))
            .collect::<FuturesUnordered<_>>();

        while let Some((chain_id, eth_call_result)) = tasks.next().await {
            match eth_call_result {
                Ok(block_number) => {
                    match block_number_per_chain.entry(chain_id) {
                        Entry::Occupied(_) => todo!("receiving a result for the same chain twice is an error, we should log that and continue"),
                        Entry::Vacant(slot) => slot.insert(block_number),
                    };
                }
                Err(_) => todo!("we should log this as an error and continue"),
            }
        }

        // Confidence check: Did we get the info we wanted for all our chains?
        if block_number_per_chain.len() != self.jrpc_providers.len() {
            todo!("we should log this as a detailed error (missing chains) and continue")
        }

        Ok(block_number_per_chain)
    }
}
