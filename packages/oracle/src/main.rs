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

    let mut event_source = EventSource::new(todo!());

    loop {
        let latest_blocks = event_source.get_latest_blocks().await?;

        for (chain, latest_block) in latest_blocks.iter() {
            //
        }

        tokio::time::sleep(CONFIG.json_rpc_polling_interval).await;
    }

    Ok(())
}
