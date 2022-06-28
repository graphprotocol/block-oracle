mod config;
mod ctrlc;
mod emitter;
mod epoch_tracker;
mod error_handling;
mod event_source;
mod jsonrpc_utils;
mod metrics;
mod models;
mod networks_diff;
mod oracle;
mod subgraph;

pub use crate::ctrlc::CtrlcHandler;
pub use config::Config;
pub use emitter::{Emitter, EmitterError};
pub use epoch_tracker::{EpochTracker, EpochTrackerError};
pub use error_handling::{MainLoopFlow, OracleControlFlow};
pub use event_source::{EventSource, EventSourceError};
pub use jsonrpc_utils::JrpcExpBackoff;
pub use metrics::Metrics;
pub use models::Caip2ChainId;
pub use networks_diff::NetworksDiff;
pub use oracle::Oracle;
pub use subgraph::{SubgraphApi, SubgraphQuery, SubgraphStateTracker};

use lazy_static::lazy_static;
use std::env::set_var;
use tracing::{info, metadata::LevelFilter};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

lazy_static! {
    pub static ref CONFIG: Config = Config::parse();
    pub static ref METRICS: Metrics = Metrics::default();
    pub static ref CTRLC_HANDLER: CtrlcHandler = CtrlcHandler::init();
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Error fetching blockchain data: {0}")]
    EventSource(#[from] EventSourceError),
    #[error(transparent)]
    EpochTracker(#[from] EpochTrackerError),
    #[error(transparent)]
    Emitter(#[from] EmitterError),
}

impl MainLoopFlow for Error {
    fn instruction(&self) -> OracleControlFlow {
        use Error::*;
        match self {
            EventSource(event_source) => event_source.instruction(),
            EpochTracker(epoch_tracker) => epoch_tracker.instruction(),
            Emitter(emitter) => emitter.instruction(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Immediately dereference some constants to trigger `lazy_static`
    // initialization.
    let _ = &*CONFIG;
    let _ = &*METRICS;
    let _ = &*CTRLC_HANDLER;

    init_logging(CONFIG.log_level);
    info!(log_level = %CONFIG.log_level, "Block oracle starting up.");

    let mut oracle = Oracle::new(&*CONFIG);

    while !CTRLC_HANDLER.poll_ctrlc() {
        match oracle.run().await.map_err(|e| e.instruction()) {
            Ok(()) | Err(OracleControlFlow::Continue(None)) => {}
            Err(OracleControlFlow::Break(())) => break,
            Err(OracleControlFlow::Continue(wait)) => {
                tokio::time::sleep(wait.unwrap_or_default()).await
            }
        }

        tokio::time::sleep(CONFIG.protocol_chain.polling_interval).await;
    }

    Ok(())
}

fn init_logging(log_level: LevelFilter) {
    set_var("RUST_LOG", "block_oracle=trace");

    let filter = EnvFilter::builder()
        .with_default_directive(log_level.into())
        .from_env_lossy();

    let stdout = fmt::layer()
        .without_time()
        .with_target(false)
        .with_writer(std::io::stdout);

    tracing_subscriber::registry()
        .with(filter)
        .with(stdout)
        .init();
}
