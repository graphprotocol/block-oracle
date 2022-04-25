mod config;
mod ctrlc;
mod diagnostics;
mod emitter;
mod encoder;
mod epoch_tracker;
mod event_source;
mod indexed_chain;
mod metrics;
mod protocol_chain;
mod store;

use crate::ctrlc::CtrlcHandler;
use crate::{epoch_tracker::EpochTrackerError, store::DataEdgeCall};
pub use config::Config;
use diagnostics::init_logging;
use ee::CompressionEngine;
pub use emitter::Emitter;
pub use encoder::Encoder;
use epoch_encoding::{self as ee, encode_messages, messages::BlockPtr, Message};
pub use epoch_tracker::EpochTracker;
use event_source::{Event, EventSource, EventSourceError};
use lazy_static::lazy_static;
pub use metrics::Metrics;
use std::collections::HashMap;
use store::models::Caip2ChainId;
pub use store::Store;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{debug, info, warn};

lazy_static! {
    pub static ref CONFIG: Config = Config::parse();
    pub static ref METRICS: Metrics = Metrics::default();
    pub static ref CTRLC_HANDLER: CtrlcHandler = CtrlcHandler::init();
}

struct BlockChainState {
    latest_block_number: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Error fetching blockchain data: {0}")]
    EventSource(#[from] event_source::EventSourceError),
    #[error("Can't publish events to Ethereum mainnet: {0}")]
    Web3(#[from] web3::Error),
    #[error(transparent)]
    EpochTracker(#[from] epoch_tracker::EpochTrackerError),
    #[error(transparent)]
    Emitter(#[from] emitter::EmitterError),
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Immediately dereference some constants to trigger `lazy_static`
    // initialization.
    let _ = &*CONFIG;
    let _ = &*METRICS;
    let _ = &*CTRLC_HANDLER;

    init_logging();
    info!("Program started");

    let store = Store::new(CONFIG.database_url.as_str()).await?;
    let mut oracle = Oracle::new(store, &*CONFIG)?;
    while !CTRLC_HANDLER.poll_ctrlc() {
        oracle.wait_and_process_next_event().await?;
    }

    Ok(())
}

/// The main application in-memory state
struct Oracle<'a> {
    store: Store,
    event_source: EventSource,
    //encoder: Encoder,
    emitter: Emitter<'a>,
    epoch_tracker: EpochTracker,
    state_by_blockchain: HashMap<Caip2ChainId, BlockChainState>,
    event_receiver: UnboundedReceiver<Result<Event, EventSourceError>>,
}

impl<'a> Oracle<'a> {
    pub fn new(store: Store, config: &'a Config) -> Result<Self, Error> {
        let (event_source, receiver) = EventSource::new(&config);

        let emitter = Emitter::new(config);
        let state_by_blockchain = HashMap::with_capacity(CONFIG.networks().len());
        let epoch_tracker = EpochTracker::new(&store, &*CONFIG);

        // Start EventSource main loop.
        let event_source_cloned = event_source.clone();
        let _event_source_task = tokio::spawn(async move { event_source_cloned.work().await });

        Ok(Self {
            store,
            event_source,
            emitter,
            epoch_tracker,
            state_by_blockchain,
            event_receiver: receiver,
        })
    }

    pub async fn wait_and_process_next_event(&mut self) -> Result<(), Error> {
        let event_res = self.event_receiver.recv().await.expect("This means the channel's sender was dropped, which shouldn't be possible because we still have ownership over it.");

        match event_res {
            Ok(event) => self.handle_event(event).await,
            Err(err) => {
                // Handle event source internal errors
                use event_source::EventSourceError::*;

                match err {
                    Web3(_) => todo!("decide how should we handle JRPC errors"),
                }
            }
        }
    }

    async fn networks(&self) -> Result<Vec<(String, ee::Network)>, Error> {
        let networks = self.store.networks().await?;
        Ok(networks
            .into_iter()
            .map(|n| {
                (
                    n.data.name.as_str().to_string(),
                    ee::Network {
                        block_delta: n.data.latest_block_delta.unwrap_or(0) as _,
                        block_number: n.data.latest_block_number.unwrap_or(0) as _,
                    },
                )
            })
            .collect())
    }

    async fn handle_event(&mut self, event: Event) -> Result<(), Error> {
        match event {
            Event::NewBlock {
                chain_id,
                block_number,
            } => {
                debug!(
                    network = %chain_id,
                    block = %block_number,
                    "Received NewBlock event"
                );

                let is_new_epoch =
                    match self.epoch_tracker.is_new_epoch(block_number.as_u64()).await {
                        Ok(boolean) => boolean,
                        Err(EpochTrackerError::PreviousEpochNotFound) => {
                            // FIXME: At the moment, we are unable to determine the latest epoch from an
                            // empty state. Until the Oracle is capable of reacting upon that, we will
                            // consider that we have reached a new epoch in such cases.
                            warn!("Failed to determine the previous epoch.");
                            true
                        }
                        Err(other_error) => return Err(other_error.into()),
                    };

                // TODO: Maybe we want to check if this chain_id is from the ProtocolChain, instead
                // of directly comparing against Ethereum Mainnet.
                if chain_id == Caip2ChainId::ethereum_mainnet() && is_new_epoch {
                    info!("A new epoch started in the protocol chain");
                    let message = Message::SetBlockNumbersForNextEpoch(
                        self.state_by_blockchain
                            .iter()
                            .map(|(chain_id, state): (&Caip2ChainId, &BlockChainState)| {
                                (
                                    chain_id.as_str().to_owned(),
                                    BlockPtr {
                                        number: state.latest_block_number,
                                        hash: [0u8; 32], // FIXME
                                    },
                                )
                            })
                            .collect(),
                    );
                    let networks = self.networks().await?;
                    dbg!();
                    let mut compression_engine = CompressionEngine::new(networks);
                    compression_engine.compress_messages(&[message]);
                    dbg!();
                    let encoded = encode_messages(&compression_engine.compressed);
                    let nonce = self.store.next_nonce().await?;

                    match self
                        .emitter
                        .submit_oracle_messages(nonce, encoded.clone())
                        .await
                    {
                        Ok(receipt) => {
                            // TODO: After broadcasting a transaction to eip155:1 and getting a
                            // transaction receipt, we should monitor it until it get enough
                            // confirmations. It's unclear which component should do this task.

                            // Record the DataEdge call
                            let data_edge_call: DataEdgeCall = {
                                // FIXME: The only purpose of this scope is to transform a
                                // transaction receipt and an encoded payload into a DataEdgeCall
                                // value. We could refactor it into a dedicated function.
                                let web3::types::TransactionReceipt {
                                    transaction_hash,
                                    block_hash,
                                    block_number,
                                    ..
                                } = &receipt;
                                if let (Some(block_hash), Some(block_number)) =
                                    (block_hash, block_number)
                                {
                                    DataEdgeCall::new(
                                        transaction_hash.as_bytes(),
                                        nonce,
                                        block_number.as_u64(),
                                        block_hash.as_bytes(),
                                        encoded,
                                    )
                                } else {
                                    todo!(
                                        "The expected block number and hash were not present, \
                                         despite having the transaction receipt at hand. \
                                         We should make an new Error variant out of this."
                                    )
                                }
                            };

                            self.store.insert_data_edge_call(data_edge_call).await?;
                        }
                        Err(other) => return Err(other.into()),
                    };
                }
                // ^^^^^
                // TODO: End of scope for handling SetBlockNumbersForNextEpoch message. We should
                // consider placing this inside a function.

                Ok(())
            }
        }
    }
}
