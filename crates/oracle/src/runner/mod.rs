pub mod ctrlc;
pub mod error_handling;
pub mod jrpc_utils;
pub mod oracle;
pub mod transaction_monitor;

use self::ctrlc::CtrlcHandler;
use crate::contracts::ContractError;
use crate::metrics::{metrics_server, METRICS};
use crate::{Caip2ChainId, Config, SubgraphQueryError};
use error_handling::{MainLoopFlow, OracleControlFlow};
use lazy_static::lazy_static;
use oracle::Oracle;
use std::{env::set_var, path::Path, time::Duration};
use tracing::{error, info, metadata::LevelFilter};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

lazy_static! {
    static ref CTRLC_HANDLER: CtrlcHandler = CtrlcHandler::init();
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
    #[error(transparent)]
    ContractError(#[from] ContractError),
    #[error("Failed to call Epoch Manager")]
    EpochManagerCallFailed(#[from] web3::contract::Error),
    #[error("Epoch Manager latest epoch ({manager}) is behind Epoch Subgraph's ({subgraph})")]
    EpochManagerBehindSubgraph { manager: u64, subgraph: u64 },
    #[error("The subgraph hasn't indexed all relevant transactions yet")]
    SubgraphNotFresh,
    #[error("The subgraph has not been initialized yet")]
    SubgraphNotInitialized,
}

impl MainLoopFlow for Error {
    fn instruction(&self) -> OracleControlFlow {
        use Error::*;
        match self {
            Subgraph(err) => err.instruction(),
            BadJrpcProtocolChain(_) => OracleControlFlow::Continue(0),
            BadJrpcIndexedChain { .. } => OracleControlFlow::Continue(0),

            // TODO: Put those variants under a new `contracts::Error` enum
            ContractError(_) => OracleControlFlow::Continue(0),
            EpochManagerCallFailed(_) => OracleControlFlow::Continue(0),
            EpochManagerBehindSubgraph { .. } => OracleControlFlow::Continue(0),

            // TODO: Put those variants under the `SubgraphQueryError` enum
            SubgraphNotFresh => OracleControlFlow::Continue(2),
            SubgraphNotInitialized => OracleControlFlow::Continue(2),
        }
    }
}

pub async fn run(config_file: impl AsRef<Path>) -> Result<(), Error> {
    // Immediately dereference some constants to trigger `lazy_static`
    // initialization.
    let config = Config::parse(config_file);
    let _ = &*METRICS;

    init_logging(config.log_level);
    info!(log_level = %config.log_level, "The block oracle is starting.");

    // Validate RPC chain IDs before starting
    if let Err(err) = crate::chain_validation::validate_chain_ids(&config).await {
        error!("Chain ID validation failed: {}", err);
        return Err(Error::BadJrpcProtocolChain(web3::Error::Decoder(
            err.to_string(),
        )));
    }

    // Spawn the metrics server
    tokio::spawn(metrics_server(&METRICS, config.metrics_port));

    // Start the Epoch Block Oracle
    oracle_task(config).await
}

async fn oracle_task(config: Config) -> Result<(), Error> {
    let mut oracle = Oracle::new(config.clone());
    info!("Entering the main polling loop. Press CTRL+C to stop.");

    while !CTRLC_HANDLER.poll_ctrlc() {
        if let Err(err) = oracle.run().await {
            handle_error(err, config.protocol_chain.polling_interval).await?;
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

async fn handle_error(err: Error, polling_interval: Duration) -> Result<(), Error> {
    error!(
        error = err.to_string().as_str(),
        "An error occurred and interrupted the last polling iteration."
    );
    match err.instruction() {
        OracleControlFlow::Break(()) => {
            error!("This error is non-recoverable. Exiting now.");
            Err(err)
        }
        OracleControlFlow::Continue(cooldown_multiplier) => {
            let wait = polling_interval * cooldown_multiplier;
            error!(
                cooling_off_seconds = wait.as_secs(),
                "This error is recoverable.",
            );
            tokio::time::sleep(wait).await;
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
