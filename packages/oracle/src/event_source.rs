use crate::store::Caip2ChainId;
use crate::CONFIG;
use futures::{
    stream::{FuturesUnordered, StreamExt},
    FutureExt,
};
use std::collections::{hash_map::Entry, HashMap};
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
pub struct EventSource {
    jrpc_providers: HashMap<Caip2ChainId, Client>,
    sender: UnboundedSender<EventSourceResult>,
}

impl EventSource {
    /// Creates an [`EventSource`]. Returns a tuple with the event source and the receiver end of a
    /// channel, which will be used to pass [`Events`] through.
    pub fn new(
        jrpc_providers: &HashMap<Caip2ChainId, Url>,
    ) -> (Self, UnboundedReceiver<EventSourceResult>) {
        let transports = jrpc_providers
            .iter()
            .map(|(chain_id, url)| {
                // Unwrap: URLs were already parsed and are valid.
                let transport = Http::new(url.as_str()).expect("failed to create HTTP transport");
                let client = Web3::new(transport);
                (chain_id.clone(), client)
            })
            .collect();

        let (sender, receiver) = unbounded_channel();
        let event_source = Self {
            jrpc_providers: transports,
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

    /// This is the "main" operation of this component.
    ///
    /// Currently, it indefinitely tries to fetch recent block numbers from the registered
    /// blockchains and sends an [`Event::NewBlock`] for each of them, then sleeps.
    pub async fn work(&self) {
        loop {
            match self.get_latest_blocks().await {
                Ok(latest_blocks_by_chain) => {
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
