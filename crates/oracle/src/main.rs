mod config;
mod contracts;
mod ctrlc;
mod error_handling;
mod jrpc_utils;
mod metrics;
mod models;
mod networks_diff;
mod oracle;
mod subgraph;

pub use crate::ctrlc::CtrlcHandler;
pub use config::Config;
pub use error_handling::{MainLoopFlow, OracleControlFlow};
pub use jrpc_utils::JrpcExpBackoff;
pub use metrics::Metrics;
pub use models::{Caip2ChainId, JrpcProviderForChain};
pub use networks_diff::NetworksDiff;
pub use oracle::Oracle;
pub use subgraph::{SubgraphApi, SubgraphQuery, SubgraphQueryError, SubgraphStateTracker};

use lazy_static::lazy_static;
use std::env::set_var;
use tracing::{error, info, metadata::LevelFilter};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

lazy_static! {
    pub static ref CONFIG: Config = Config::parse();
    pub static ref METRICS: Metrics = Metrics::default();
    pub static ref CTRLC_HANDLER: CtrlcHandler = CtrlcHandler::init();
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("JSON-RPC issues for the protocol chain: {0}")]
    BadJrpcProtocolChain(web3::Error),
    #[error("Failed to get latest block information for the indexed chain with ID '{chain_id}': {error}")]
    BadJrpcIndexedChain {
        chain_id: Caip2ChainId,
        error: web3::Error,
    },
    #[error(transparent)]
    Subgraph(#[from] SubgraphQueryError),
    #[error("Couldn't submit a transaction to the mempool of the JRPC provider: {0}")]
    CantSubmitTx(web3::Error),
    #[error("Failed to call Epoch Manager")]
    EpochManagerCallFailed(#[from] web3::contract::Error),
    #[error("Epoch Manager latest epoch {manager} is behind Epoch Subgraph's: {subgraph}")]
    EpochManagerBehindSubgraph { manager: u64, subgraph: u64 },
}

impl MainLoopFlow for Error {
    fn instruction(&self) -> OracleControlFlow {
        use Error::*;
        match self {
            Subgraph(_) => OracleControlFlow::Continue(None),
            BadJrpcProtocolChain(_) => OracleControlFlow::Continue(None),
            BadJrpcIndexedChain { .. } => OracleControlFlow::Continue(None),
            CantSubmitTx(_) => OracleControlFlow::Continue(None),
            EpochManagerCallFailed(_) => OracleControlFlow::Continue(None),
            EpochManagerBehindSubgraph { .. } => OracleControlFlow::Continue(None),
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
    info!(log_level = %CONFIG.log_level, "The block oracle is starting.");

    let mut oracle = Oracle::new(&*CONFIG);
    info!("Entering the main polling loop. Press CTRL+C to stop.");

    while !CTRLC_HANDLER.poll_ctrlc() {
        if let Err(err) = oracle.run().await {
            handle_error(err).await?;
        }

        info!(
            seconds = CONFIG.protocol_chain.polling_interval.as_secs(),
            "Going to sleep before polling for the next epoch."
        );
        // After every polling iteration, we go to sleep for a bit. Wouldn't
        // want to DDoS our data providers, wouldn't we?
        tokio::time::sleep(CONFIG.protocol_chain.polling_interval).await;
    }

    Ok(())
}

async fn handle_error(err: Error) -> Result<(), Error> {
    error!("An error occurred: {}.", err);
    match err.instruction() {
        OracleControlFlow::Break(()) => {
            error!("This error is non-recoverable. Exiting now.");
            return Err(err);
        }
        OracleControlFlow::Continue(wait) => {
            error!(
                cooling_off_seconds = wait.unwrap_or_default().as_secs(),
                "This error is recoverable.",
            );
            tokio::time::sleep(wait.unwrap_or_default()).await;
            Ok(())
        }
    }
}

fn init_logging(log_level: LevelFilter) {
    set_var("RUST_LOG", "block_oracle=trace");

    let filter = EnvFilter::builder()
        .with_default_directive(log_level.into())
        .from_env_lossy();

    let stdout = fmt::layer()
        .with_ansi(false)
        .without_time()
        .with_target(false)
        .with_writer(std::io::stdout);

    tracing_subscriber::registry()
        .with(filter)
        .with(stdout)
        .init();
}

pub fn hex_string(bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(bytes))
}
