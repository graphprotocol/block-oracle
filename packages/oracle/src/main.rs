mod config;
mod ctrlc;
mod emitter;
mod encoder;
mod epoch_tracker;
mod event_source;
mod metrics;
mod networks_diff;
mod store;

use crate::ctrlc::CtrlcHandler;
use epoch_encoding::{self as ee, encode_messages, messages::BlockPtr, CompressionEngine, Message};
use event_source::{Event, EventSource, EventSourceError};
use lazy_static::lazy_static;
use std::collections::HashMap;
use store::models::Caip2ChainId;
use tokio::sync::mpsc::UnboundedReceiver;

pub use config::Config;
pub use emitter::Emitter;
pub use encoder::Encoder;
pub use epoch_tracker::EpochTracker;
pub use metrics::Metrics;
pub use store::Store;

lazy_static! {
    pub static ref CONFIG: Config = Config::parse();
    pub static ref METRICS: Metrics = Metrics::default();
    pub static ref CTRLC_HANDLER: CtrlcHandler = CtrlcHandler::init();
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Error fetching blockchain data: {0}")]
    EventSource(#[from] event_source::EventSourceError),
    #[error("Can't publish events to Ethereum mainnet: {0}")]
    Web3(#[from] web3::Error),
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Immediately dereference some constants to trigger `lazy_static`
    // initialization.
    let _ = &*CONFIG;
    let _ = &*METRICS;
    let _ = &*CTRLC_HANDLER;

    let mut oracle = Oracle::new(&*CONFIG).await?;
    while !CTRLC_HANDLER.poll_ctrlc() {
        oracle.wait_and_process_next_event().await?;
    }

    Ok(())
}

struct BlockChainState {
    latest_block_number: u64,
}

/// The main application in-memory state
pub struct Oracle {
    // -- components --
    store: Store,
    event_source: EventSource,
    ethereum_emitter: Emitter,
    epoch_tracker: EpochTracker,

    // -- data --
    state_by_blockchain: HashMap<Caip2ChainId, BlockChainState>,
    event_receiver: UnboundedReceiver<Result<Event, EventSourceError>>,
}

impl Oracle {
    pub async fn new(config: &Config) -> Result<Self, Error> {
        let store = Store::new(config.database_url.as_str()).await?;
        let (event_source, receiver) = EventSource::new(&config.jrpc_providers);
        let ethereum_emitter = Emitter::new(config)?;

        let state_by_blockchain = HashMap::with_capacity(CONFIG.networks().len());
        let epoch_tracker = EpochTracker::new(&store, &*CONFIG);

        // Start EventSource main loop.
        let event_source_cloned = event_source.clone();
        let _event_source_task = tokio::spawn(async move { event_source_cloned.work().await });

        Ok(Self {
            store,
            event_source,
            ethereum_emitter,
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
                dbg!(&chain_id, block_number);

                let networks = self.networks().await?;
                let is_new_epoch = chain_id == Caip2ChainId::ethereum_mainnet()
                    && self
                        .epoch_tracker
                        .is_new_epoch(block_number.as_u64())
                        .await?;
                if is_new_epoch {
                    let mut compression_engine = CompressionEngine::new(networks);
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
                    compression_engine.compress_messages(&[message]);
                    let encoded = encode_messages(&compression_engine.compressed);

                    let nonce = self.store.next_nonce().await?;
                    self.ethereum_emitter
                        .submit_oracle_messages(nonce, encoded)
                        .await?;
                } else {
                    self.state_by_blockchain
                        .entry(chain_id)
                        .or_insert(BlockChainState {
                            latest_block_number: 0,
                        })
                        .latest_block_number = block_number.as_u64();
                }

                Ok(())
            }
        }
    }
}
