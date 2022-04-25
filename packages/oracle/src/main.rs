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
use epoch_encoding::{encode_messages, messages::BlockPtr, CompressionEngine, Message};
use event_source::{Event, EventSource};
use lazy_static::lazy_static;
use std::collections::HashMap;
use store::models::Caip2ChainId;

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

// -------------

struct BlockChainState {
    latest_block_number: u64,
}

/// The main application in-memory state
struct Oracle {
    // -- components --
    store: Store,
    event_source: EventSource,
    encoder: Encoder,
    ethereum_client: Emitter,
    epoch_tracker: EpochTracker,

    // -- data --
    state_by_blockchain: HashMap<Caip2ChainId, BlockChainState>,
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

    let mut store = Store::new(CONFIG.database_url.as_str()).await?;
    let mut emitter = Emitter::new(&*CONFIG)?;
    let mut state_by_blockchain = HashMap::with_capacity(CONFIG.networks().len());
    let epoch_tracker = EpochTracker::new(&store, &*CONFIG);

    let (event_source, mut receiver) = EventSource::new(&CONFIG.jrpc_providers);
    // Start EventSource main loop.
    let _event_source_task = tokio::spawn(async move { event_source.work().await });

    loop {
        if CTRLC_HANDLER.poll_ctrlc() {
            break;
        }

        match receiver.recv().await {
            Some(Ok(Event::NewBlock {
                chain_id,
                block_number,
            })) => {
                dbg!(&chain_id, block_number);

                let is_new_epoch = chain_id == Caip2ChainId::ethereum_mainnet()
                    && epoch_tracker.is_new_epoch(block_number.as_u64()).await?;
                if is_new_epoch {
                    let mut compression_engine = CompressionEngine::new(networks);
                    let message = Message::SetBlockNumbersForNextEpoch(
                        state_by_blockchain
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

                    let nonce = store.next_nonce().await?;
                    emitter.submit_oracle_messages(nonce, encoded).await?;
                } else {
                    state_by_blockchain
                        .entry(chain_id)
                        .or_insert(BlockChainState {
                            latest_block_number: 0,
                        })
                        .latest_block_number = block_number.as_u64();
                }
            }
            Some(Err(event_source_error)) => {
                // Handle event source internal errors
                use event_source::EventSourceError::*;
                match event_source_error {
                    Web3(_) => todo!("decide how should we handle JRPC errors"),
                }
            }
            None => {
                // If whe exit the previous loop, then it means that the channel's sender was dropped.
                return Err(todo!("define a new error variant for this case"));
            }
        }
    }
    Ok(())
}
