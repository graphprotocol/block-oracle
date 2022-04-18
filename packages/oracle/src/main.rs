mod config;
mod emitter;
mod encoder;
mod epoch_tracker;
mod event_source;
mod metrics;
mod store;

use epoch_encoding::{
    compress_messages, encode_messages, messages::BlockPtr, Blockchain, Database, Message,
    Transaction,
};
use event_source::{Event, EventSource};
use lazy_static::lazy_static;
use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc},
};
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

    // Gracefully stop the program if CTRL-C is detected.
    let ctrlc = Arc::new(AtomicBool::new(false));
    let ctrlc_clone = ctrlc.clone();
    ctrlc::set_handler(move || {
        println!("\nCTRL-C detected. Stopping... please wait.\n");
        ctrlc_clone.store(true, std::sync::atomic::Ordering::Relaxed);
    })
    .expect("Error setting CTRL-C handler.");

    let mut store = Store::new(CONFIG.database_url.as_str()).await?;
    let mut emitter = Emitter::new(&*CONFIG)?;
    let mut state_by_blockchain = HashMap::with_capacity(CONFIG.networks().len());
    let epoch_tracker = EpochTracker::new(&store, &*CONFIG);

    let (event_source, mut receiver) = EventSource::new(&CONFIG.jrpc_providers);
    // Start EventSource main loop.
    let _event_source_task = tokio::spawn(async move { event_source.work().await });

    loop {
        if ctrlc.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }

        match receiver.recv().await {
            Some(Ok(Event::NewBlock {
                chain_id,
                block_number,
            })) => {
                dbg!(&chain_id, block_number);

                if chain_id == Caip2ChainId::ethereum_mainnet()
                    && epoch_tracker.is_new_epoch(block_number.as_u64()).await?
                {
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
                    let compressed = match compress_messages(&mut store, &[message]).await? {
                        Ok(compressed) => compressed,
                        Err(e) => todo!(),
                    };
                    let encoded = encode_messages(&compressed);

                    let nonce = store.get_next_nonce().await?;
                    store.set_next_nonce(nonce).await?;

                    let transaction = Transaction {
                        nonce,
                        payload: encoded,
                    };

                    emitter.submit_oracle_messages(transaction).await.unwrap();
                }

                state_by_blockchain
                    .entry(chain_id)
                    .or_insert(BlockChainState {
                        latest_block_number: 0,
                    })
                    .latest_block_number = block_number.as_u64();

                // TODO: continue from here
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
