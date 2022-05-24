mod config;
mod ctrlc;
mod diagnostics;
mod emitter;
mod epoch_tracker;
mod event_source;
mod indexed_chain;
mod jsonrpc_utils;
mod metrics;
mod models;
mod networks_diff;
mod protocol_chain;
mod subgraph;

use crate::ctrlc::CtrlcHandler;
use diagnostics::init_logging;
use ee::CURRENT_ENCODING_VERSION;
use epoch_encoding::{self as ee, BlockPtr, Encoder, Message};
use epoch_tracker::EpochTrackerError;
use event_source::{EventSource, EventSourceError};
use lazy_static::lazy_static;
use models::Caip2ChainId;
use std::collections::HashMap;
use tracing::{debug, info, warn};

pub use config::Config;
pub use emitter::Emitter;
pub use epoch_tracker::EpochTracker;
pub use metrics::Metrics;
pub use networks_diff::NetworksDiff;

lazy_static! {
    pub static ref CONFIG: Config = Config::parse();
    pub static ref METRICS: Metrics = Metrics::default();
    pub static ref CTRLC_HANDLER: CtrlcHandler = CtrlcHandler::init();
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
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

    init_logging(CONFIG.log_level);
    info!(log_level = %CONFIG.log_level, "Block oracle starting up.");

    let mut oracle = Oracle::new(&*CONFIG)?;
    while !CTRLC_HANDLER.poll_ctrlc() {
        oracle.wait_and_process_next_event().await?;
        println!(
            "sugraph state: {:?}",
            subgraph::query(CONFIG.subgraph_url.as_str()).await.unwrap()
        );
        tokio::time::sleep(CONFIG.protocol_chain_polling_interval).await;
    }

    Ok(())
}

/// The main application in-memory state
struct Oracle<'a> {
    emitter: Emitter<'a>,
    epoch_tracker: EpochTracker,
    event_source: EventSource,
}

impl<'a> Oracle<'a> {
    pub fn new(config: &'a Config) -> Result<Self, Error> {
        let event_source = EventSource::new(config);
        let emitter = Emitter::new(config);
        let epoch_tracker = EpochTracker::new(&*CONFIG);

        Ok(Self {
            event_source,
            emitter,
            epoch_tracker,
        })
    }

    pub async fn wait_and_process_next_event(&mut self) -> Result<(), Error> {
        match self.event_source.get_latest_protocol_chain_block().await {
            Ok(block_number) => {
                debug!(
                    block = %block_number,
                    "Received latest block information from the protocol chain."
                );

                if self.is_new_epoch(block_number.as_u64()).await? {
                    self.handle_new_epoch().await?;
                }

                Ok(())
            }
            Err(EventSourceError::Web3(_)) => {
                todo!("decide how should we handle JRPC errors")
            }
        }
    }

    async fn handle_new_epoch(&mut self) -> Result<(), Error> {
        info!("A new epoch started in the protocol chain.");
        let mut messages = vec![];

        // First, we need to make sure that there are no pending
        // `RegisterNetworks` messages.
        let networks_diff = NetworksDiff::calculate((), &CONFIG).await?;
        info!(
            created = networks_diff.insertions.len(),
            deleted = networks_diff.deletions.len(),
            "Performed indexed chain diffing."
        );
        if let Some(msg) = networks_diff_to_message(&networks_diff) {
            messages.push(msg);
        }

        // Get indexed chains' latest blocks.
        let latest_blocks = self.event_source.get_latest_blocks().await?;
        messages.push(latest_blocks_to_message(latest_blocks));

        let available_networks = vec![];
        debug!(
            messages = ?messages,
            messages_count = messages.len(),
            networks_count = available_networks.len(),
            "Compressing message(s)."
        );

        let mut compression_engine = Encoder::new(CURRENT_ENCODING_VERSION, available_networks);
        let encoded = compression_engine.encode(&messages[..]);
        debug!(encoded = ?encoded, "Successfully encoded message(s).");

        self.submit_oracle_messages(encoded).await?;

        Ok(())
    }

    async fn submit_oracle_messages(&mut self, calldata: Vec<u8>) -> Result<(), Error> {
        let _receipt = self
            .emitter
            .submit_oracle_messages(calldata.clone())
            .await?;

        // TODO: After broadcasting a transaction to the protocol chain and getting a transaction
        // receipt, we should monitor it until it get enough confirmations. It's unclear which
        // component should do this task.

        Ok(())
    }

    async fn is_new_epoch(&self, block_number: u64) -> Result<bool, Error> {
        match self.epoch_tracker.is_new_epoch(block_number).await {
            Ok(b) => Ok(b),
            Err(EpochTrackerError::PreviousEpochNotFound) => {
                // FIXME: At the moment, we are unable to determine the latest epoch from an
                // empty state. Until the Oracle is capable of reacting upon that, we will
                // consider that we have reached a new epoch in such cases.
                warn!("Failed to determine the previous epoch.");
                Ok(true)
            }
            Err(other_error) => Err(other_error.into()),
        }
    }
}

fn latest_blocks_to_message(latest_blocks: HashMap<&Caip2ChainId, BlockPtr>) -> ee::Message {
    Message::SetBlockNumbersForNextEpoch(
        latest_blocks
            .iter()
            .map(|(chain_id, block_ptr)| (chain_id.as_str().to_owned(), *block_ptr))
            .collect(),
    )
}

fn networks_diff_to_message(diff: &NetworksDiff) -> Option<ee::Message> {
    if diff.deletions.is_empty() && diff.insertions.is_empty() {
        None
    } else {
        Some(ee::Message::RegisterNetworks {
            remove: diff.deletions.iter().map(|x| *x.1 as u64).collect(),
            add: diff
                .insertions
                .iter()
                .map(|x| x.0.as_str().to_string())
                .collect(),
        })
    }
}

mod freshness {
    use crate::protocol_chain::ProtocolChain;
    use thiserror::Error;
    use tracing::{debug, error, trace};
    use web3::types::{H160, U64};

    #[derive(Debug, Error)]
    enum FreshnessCheckEror {
        #[error("Epoch Subgraph advanced beyond protocol chain's head")]
        SubgraphBeyondChain,
        #[error(transparent)]
        Web3(#[from] web3::Error),
    }

    /// Number of blocks that the Epoch Subgraph may be away from the protocol chain's head. If the
    /// block distance is lower than this, a `trace_filter` JSON RPC call will be used to infer if
    /// any relevant transaction happened within that treshold.
    ///
    /// This should be configurable.
    const FRESHNESS_THRESHOLD: u64 = 10;

    /// The Epoch Subgraph is considered fresh if it has processed all relevant transactions
    /// targeting the DataEdge contract.
    ///
    /// To assert that, the Block Oracle will need to get the latest block from a JSON RPC provider
    /// and compare its number with the subgraph’s current block.
    ///
    /// If they are way too different, then the subgraph is not fresh, and we should gracefully
    /// handle that error.
    ///
    /// Otherwise, if block numbers are under a certain threshold apart, we could scan the blocks
    /// in between and ensure they’re not relevant to the DataEdge contract.
    async fn subgaph_is_fresh(
        subgraph_latest_block: U64,
        current_block: U64,
        protocol_chain: &ProtocolChain,
        owner_address: H160,
        contract_address: H160,
    ) -> Result<bool, FreshnessCheckEror> {
        // If this ever happens, then there must be a serious bug in the code
        if subgraph_latest_block > current_block {
            let anomaly = FreshnessCheckEror::SubgraphBeyondChain;
            error!(%anomaly);
            return Err(anomaly);
        }
        let block_distance = (current_block - subgraph_latest_block).as_u64();
        if block_distance == 0 {
            return Ok(true);
        } else if block_distance > FRESHNESS_THRESHOLD {
            debug!(
                %subgraph_latest_block,
                %current_block,
                "Epoch Subgraph is not considered fresh because it is {} blocks behind \
                 protocol chain's head",
                block_distance
            );
            return Ok(false);
        }
        // Scan the blocks in betwenn for transactions from the Owner to the Data Edge contract
        let calls = protocol_chain
            .calls_in_block_range(
                subgraph_latest_block,
                current_block,
                owner_address,
                contract_address,
            )
            .await?;

        if calls.is_empty() {
            trace!(
                %subgraph_latest_block,
                %current_block,
                "Epoch Subgraph is fresh. \
                 Found no calls between last synced block and the protocol chain's head",
            );
            Ok(true)
        } else {
            debug!(
                %subgraph_latest_block,
                %current_block,
                "Epoch Subgraph is not fresh. \
                 Found {} calls between the last synced block and the protocol chain's head",
                calls.len()
            );
            Ok(false)
        }
    }
}
