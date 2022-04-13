use crate::store::Caip2ChainId;
use crate::CONFIG;
use futures::{
    stream::{FuturesUnordered, StreamExt},
    FutureExt,
};
use std::collections::{hash_map::Entry, HashMap};
use thiserror::Error;
use tokio::sync::mpsc::{error::SendError, unbounded_channel, UnboundedReceiver, UnboundedSender};
use web3::{api::Web3, transports::http::Http as HttpTransport, types::U64};

type BlockNumber = U64;
type Client = Web3<HttpTransport>;

#[derive(Error, Debug)]
pub enum EventSourceError {
    #[error("Ethereum client error")]
    Web3(#[from] web3::Error),
}

pub enum Event {
    /// The annoucement of a recent block from a given blockchain
    NewBlock {
        chain_id: Caip2ChainId,
        block_number: U64,
    },
    /// Represents an error that occured during the [`EventSource`] execution.
    Error(EventSourceError),
}

/// Actively listens for new blocks and reorgs from registered blockchains. Also, it checks the
/// number of confirmations for transactions sent to the DataEdge contract.
pub struct EventSource {
    jrpc_providers: HashMap<Caip2ChainId, Client>,
    sender: UnboundedSender<Event>,
}

impl EventSource {
    /// Creates an [`EventSource`]. Returns a tuple with the event source and the receiver end of a
    /// channel, which will be used to pass [`Events`] through.
    pub fn new(jrpc_providers: HashMap<Caip2ChainId, Client>) -> (Self, UnboundedReceiver<Event>) {
        let (sender, receiver) = unbounded_channel();
        let event_source = Self {
            jrpc_providers,
            sender,
        };
        (event_source, receiver)
    }

    async fn get_latest_blocks(
        &self,
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

    /// TODO: FIXME: The Err variant should be something more usefull. It might change depending of
    /// how/where this method is used.
    async fn work(&self) -> Result<(), SendError<Event>> {
        loop {
            match self.get_latest_blocks().await {
                Ok(latest_blocks_by_chain) => {
                    for (chain_id, block_number) in latest_blocks_by_chain.into_iter() {
                        let event = Event::NewBlock {
                            chain_id: chain_id.clone(),
                            block_number,
                        };
                        self.sender.send(event)?
                    }
                }
                // Let the receiver end deal with internal errors
                Err(error) => self.sender.send(Event::Error(error))?,
            }
            tokio::time::sleep(CONFIG.json_rpc_polling_interval).await;
        }
    }
}
