pub mod ctrlc;
pub mod error_handling;
pub mod jrpc_utils;
pub mod metrics;
pub mod oracle;

use self::ctrlc::CtrlcHandler;
use crate::{Caip2ChainId, Config, SubgraphQueryError};
use error_handling::{MainLoopFlow, OracleControlFlow};
use futures::TryFutureExt;
use lazy_static::lazy_static;
use metrics::{server::metrics_server, METRICS};
use oracle::Oracle;
use std::{env::set_var, path::Path, sync::Arc, time::Duration};
use tracing::{error, info, metadata::LevelFilter};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

lazy_static! {
    static ref CTRLC_HANDLER: CtrlcHandler = CtrlcHandler::init();
}

#[derive(Debug, thiserror::Error)]
enum ApplicationError {
    #[error(transparent)]
    Oracle(Error),
    #[error("The metrics server crashed")]
    Metrics(#[from] hyper::Error),
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
    Subgraph(#[from] Arc<SubgraphQueryError>),
    #[error("Couldn't submit a transaction to the mempool of the JRPC provider: {0}")]
    CantSubmitTx(web3::contract::Error),
    #[error("Failed to call Epoch Manager")]
    EpochManagerCallFailed(#[from] web3::contract::Error),
    #[error("Epoch Manager latest epoch ({manager}) is behind Epoch Subgraph's ({subgraph})")]
    EpochManagerBehindSubgraph { manager: u64, subgraph: u64 },
    #[error("The subgraph hasn't indexed all relevant transactions yet")]
    SubgraphNotFresh,
}

impl MainLoopFlow for Error {
    fn instruction(&self) -> OracleControlFlow {
        use Error::*;
        match self {
            Subgraph(err) => err.instruction(),
            BadJrpcProtocolChain(_) => OracleControlFlow::Continue(None),
            BadJrpcIndexedChain { .. } => OracleControlFlow::Continue(None),

            // TODO: Put those variants under a new `contracts::Error` enum
            CantSubmitTx(_) => OracleControlFlow::Continue(None),
            EpochManagerCallFailed(_) => OracleControlFlow::Continue(None),
            EpochManagerBehindSubgraph { .. } => OracleControlFlow::Continue(None),

            // TODO: Put those variants under the `SubgraphQueryError` enum
            SubgraphNotFresh => OracleControlFlow::Continue(Some(Duration::from_secs(30))),
        }
    }
}

pub async fn run(config_file: impl AsRef<Path>) -> anyhow::Result<()> {
    // Immediately dereference some constants to trigger `lazy_static`
    // initialization.
    let config = Config::parse(config_file);
    let _ = &*METRICS;

    init_logging(config.log_level);
    info!(log_level = %config.log_level, "The block oracle is starting.");

    let metrics_server =
        metrics_server(&METRICS, config.metrics_port).map_err(ApplicationError::Metrics);
    let oracle = oracle_task(config).map_err(ApplicationError::Oracle);
    tokio::try_join!(metrics_server, oracle)?;
    Ok(())
}

async fn oracle_task(config: Config) -> Result<(), Error> {
    let mut oracle = Oracle::new(config.clone());
    info!("Entering the main polling loop. Press CTRL+C to stop.");

    while !CTRLC_HANDLER.poll_ctrlc() {
        if let Err(err) = oracle.run().await {
            handle_error(err).await?;
            continue;
        }

        // After every polling iteration, we go to sleep for a bit. Wouldn't
        // want to DDoS our data providers, wouldn't we?
        info!(
            seconds = config.protocol_chain.polling_interval.as_secs(),
            "Going to sleep before next polling iteration."
        );
        tokio::time::sleep(config.protocol_chain.polling_interval).await;
    }
    Ok(())
}

async fn handle_error(err: Error) -> Result<(), Error> {
    error!(
        error = err.to_string().as_str(),
        "An error occurred and interrupted the last polling iteration."
    );
    match err.instruction() {
        OracleControlFlow::Break(()) => {
            error!("This error is non-recoverable. Exiting now.");
            Err(err)
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
