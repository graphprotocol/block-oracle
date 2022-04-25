use crate::indexed_chain::{self, IndexedChain};
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
use url::Url;
use web3::{api::Web3, transports::http::Http, types::U64};

type BlockNumber = U64;
type Client = Web3<Http>;

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

    async fn get_latest_blocks(
        &self,
    ) -> Result<HashMap<&Caip2ChainId, BlockNumber>, EventSourceError> {
        let mut block_number_per_chain: HashMap<&Caip2ChainId, BlockNumber> = HashMap::new();

        // TODO: Find a way to not block on this.
        //
        // Maybe we can use a new trait for abstracting over `get_latest_block` behaviour and use
        // both `ProtocolChainClient` and `IndexedChain` as trait objects in the `tasks` future
        // collection below
        let protocol_chain_latest_block = self.protocol_chain.get_latest_block().await?;
        block_number_per_chain.insert(self.protocol_chain.id(), protocol_chain_latest_block);

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

        // Confidence check: Did we get the info we wanted for all our chains?
        // We add +1 to account for the protocol chain
        if block_number_per_chain.len() != self.indexed_chains.len() + 1 {
            todo!("we should log this as a detailed error (missing chains) and continue")
        }

        Ok(block_number_per_chain)
    }

    /// This is the "main" operation of this component.
    ///
    /// Currently, it indefinitely tries to fetch recent block numbers from the registered
    /// blockchains and sends an [`Event::NewBlock`] for each of them, then sleeps.
    pub async fn work(&self) {
        loop {
            match self.get_latest_blocks().await {
                Ok(latest_blocks_by_chain) => {
                    // TODO: We may want to send the NewBlock event for the protocol chain last, as
                    // it will possibly trigger a new DataEdge call. This way we ensure that it will
                    // be up to date about the latest blocks for the indexed chains.
                    for (chain_id, block_number) in latest_blocks_by_chain.into_iter() {
                        let event = Event::NewBlock {
                            chain_id: chain_id.clone(),
                            block_number,
                        };
                        self.sender
                            .send(Ok(event))
                            .expect("failed to send an Event through channel");
                    }
                }
                // Let the receiver deal with internal errors
                Err(error) => self
                    .sender
                    .send(Err(error))
                    .expect("failed to send Error through channel"),
            }
            tokio::time::sleep(CONFIG.json_rpc_polling_interval).await;
        }
    }
}
