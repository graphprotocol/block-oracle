mod config;
mod emitter;
mod encoder;
mod event_source;
mod metrics;
mod store;

use event_source::EventSource;
use lazy_static::lazy_static;
use std::collections::HashMap;
use store::models::Caip2ChainId;

pub use config::Config;
pub use emitter::Emitter;
pub use encoder::Encoder;
pub use metrics::Metrics;
pub use store::Store;

lazy_static! {
    pub static ref CONFIG: Config = Config::parse();
    pub static ref METRICS: Metrics = Metrics::default();
}

/// Tracks current Ethereum mainnet epoch.
type EpochTracker = ();

// -------------

type BlockChainState = ();

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
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Immediately dereference some constants to trigger `lazy_static`
    // initialization.
    let _ = &*CONFIG;
    let _ = &*METRICS;

    let store = Store::new(CONFIG.database_url.as_str()).await?;
    let networks = store.networks().await?;

    let (event_source, mut receiver) = EventSource::new(&CONFIG.jrpc_providers);

    // start EventSource main loop
    let event_source_task = tokio::spawn(async move { event_source.work().await });

    loop {
        while let Some(event) = receiver.recv().await {
            use crate::event_source::Event::*;
            match event {
                Ok(event) => match event {
                    NewBlock {
                        chain_id,
                        block_number,
                    } => todo!(),
                },
                Err(event_source_error) => {
                    // Handle event source internal errors
                    use event_source::EventSourceError::*;
                    match event_source_error {
                        Web3(_) => todo!("decide how should we handle JRPC errors"),
                    }
                }
            }
        }
        // If whe exit the previous loop, then it means that the channel's sender was dropped.
        return Err(todo!("define a new error variant for this case"));
    }
    Ok(())
}
