use crate::indexed_chain::IndexedChain;
use crate::{protocol_chain::ProtocolChain, store::Caip2ChainId};
use crate::{Config, CONFIG};
use futures::{
    stream::{FuturesUnordered, StreamExt},
    FutureExt,
};
use std::collections::{hash_map::Entry, HashMap};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use web3::types::U64;

type BlockNumber = U64;

#[derive(Error, Debug)]
pub enum EventSourceError {
    #[error("Ethereum client error")]
    Web3(#[from] web3::Error),
}

#[derive(Debug)]
pub enum Event {
    /// The annoucement of a recent block from a given blockchain
    NewBlock {
        chain_id: Caip2ChainId,
        block_number: U64,
    },
}

type EventSourceResult = Result<Event, EventSourceError>;

/// Actively listens for new blocks and reorgs from registered blockchains. Also, it checks the
/// number of confirmations for transactions sent to the DataEdge contract.
#[derive(Debug, Clone)]
pub struct EventSource {
    protocol_chain: Arc<ProtocolChain>,
    indexed_chains: Arc<Vec<IndexedChain>>,
    sender: UnboundedSender<EventSourceResult>,
}

impl EventSource {
    /// Creates an [`EventSource`]. Returns a tuple with the event source and the receiver end of a
    /// channel, which will be used to pass [`Events`] through.
    pub fn new(config: &Config) -> (Self, UnboundedReceiver<EventSourceResult>) {
        let (sender, receiver) = unbounded_channel();
        let event_source = Self {
            protocol_chain: config.protocol_chain_client.clone(),
            indexed_chains: config.indexed_chains.clone(),
            sender,
        };
        (event_source, receiver)
    }

    pub async fn get_latest_blocks(
        &self,
    ) -> Result<HashMap<&Caip2ChainId, BlockNumber>, EventSourceError> {
        let mut block_number_per_chain: HashMap<&Caip2ChainId, BlockNumber> = HashMap::new();

        let mut tasks = self
            .indexed_chains
            .iter()
            .map(|indexed_chain| {
                indexed_chain
                    .get_latest_block()
                    .map(|block| (indexed_chain.id(), block))
            })
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

        if block_number_per_chain.len() != self.indexed_chains.len() {
            todo!("we should log this as a detailed error (missing chains) and continue")
        }

        Ok(block_number_per_chain)
    }

    /// Pools the latest block from the protocol chain and sends it over a channel, in a loop.
    pub async fn work(&self) {
        loop {
            match self.protocol_chain.get_latest_block().await {
                Ok(block_number) => {
                    let event = Event::NewBlock {
                        chain_id: self.protocol_chain.id().clone(),
                        block_number,
                    };
                    self.sender
                        .send(Ok(event))
                        .expect("failed to send an Event through channel");
                }

                // Let the receiver deal with internal errors
                Err(error) => self
                    .sender
                    .send(Err(error.into()))
                    .expect("failed to send Error through channel"),
            }
            tokio::time::sleep(CONFIG.json_rpc_polling_interval).await;
        }
    }
}
