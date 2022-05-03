mod config;
mod ctrlc;
mod diagnostics;
mod emitter;
mod encoder;
mod epoch_tracker;
mod event_source;
mod indexed_chain;
mod metrics;
mod networks_diff;
mod protocol_chain;
mod store;
mod transport;

use crate::ctrlc::CtrlcHandler;
use diagnostics::init_logging;
use epoch_encoding::{self as ee, encode_messages, messages::BlockPtr, CompressionEngine, Message};
use epoch_tracker::EpochTrackerError;
use event_source::{Event, EventSource};
use lazy_static::lazy_static;
use std::collections::HashMap;
use store::{Caip2ChainId, DataEdgeCall};
use tracing::{debug, info, warn};

pub use config::Config;
pub use emitter::Emitter;
pub use encoder::Encoder;
pub use epoch_tracker::EpochTracker;
pub use metrics::Metrics;
pub use networks_diff::NetworksDiff;
pub use store::Store;

lazy_static! {
    pub static ref CONFIG: Config = Config::parse();
    pub static ref METRICS: Metrics = Metrics::default();
    pub static ref CTRLC_HANDLER: CtrlcHandler = CtrlcHandler::init();
}

struct BlockChainState {
    latest_block_number: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),
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

    let store = Store::new(CONFIG.database_url.as_str()).await?;

    let mut oracle = Oracle::new(store, &*CONFIG)?;
    while !CTRLC_HANDLER.poll_ctrlc() {
        oracle.wait_and_process_next_event().await?;
        tokio::time::sleep(CONFIG.json_rpc_polling_interval).await;
    }

    Ok(())
}

/// The main application in-memory state
struct Oracle<'a> {
    config: &'a Config,
    store: Store,
    //encoder: Encoder,
    emitter: Emitter<'a>,
    epoch_tracker: EpochTracker,
    state_by_blockchain: HashMap<Caip2ChainId, BlockChainState>,
    event_source: EventSource,
}

impl<'a> Oracle<'a> {
    pub fn new(store: Store, config: &'a Config) -> Result<Self, Error> {
        let event_source = EventSource::new(config);
        let emitter = Emitter::new(config);
        let state_by_blockchain = HashMap::with_capacity(CONFIG.indexed_chains.len());
        let epoch_tracker = EpochTracker::new(&store, &*CONFIG);

        Ok(Self {
            config,
            store,
            event_source,
            emitter,
            epoch_tracker,
            state_by_blockchain,
        })
    }

    pub async fn wait_and_process_next_event(&mut self) -> Result<(), Error> {
        let event_res = self.event_source.get_latest_protocol_chain_block().await;
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
                debug!(
                    network = %chain_id,
                    block = %block_number,
                    "Received NewBlock event."
                );

                if self.is_new_epoch(&chain_id, block_number.as_u64()).await? {
                    self.handle_new_epoch().await?;
                }

                Ok(())
            }
        }
    }

    async fn handle_new_epoch(&mut self) -> Result<(), Error> {
        info!("A new epoch started in the protocol chain.");
        let mut messages = vec![];

        let networks_diff = NetworksDiff::calculate(&self.store, &CONFIG).await?;
        info!(
            created = networks_diff.insertions.len(),
            deleted = networks_diff.deletions.len(),
            "Performed indexed chain diffing."
        );
        if let Some(msg) = networks_diff_to_message(networks_diff) {
            messages.push(msg);
        }

        // Get indexed chains' latest blocks.
        let latest_blocks = self.event_source.get_latest_blocks().await?;
        messages.push(latest_blocks_to_message(latest_blocks));

        let networks = self.networks().await?;
        debug!(
            messages = ?messages,
            messages_count = messages.len(),
            networks_count = networks.len(),
            "Compressing message(s)."
        );
        let mut compression_engine = CompressionEngine::new(networks);
        let messages = vec![];

        compression_engine.compress_messages(&messages[..]);
        debug!(msg = ?compression_engine.compressed, msg_count = 1u32, "Successfully compressed, now encoding message(s).");
        let encoded = encode_messages(&compression_engine.compressed);
        debug!(encoded = ?encoded, "Successfully encoded message(s).");
        let nonce = self.store.next_nonce().await?;

        self.submit_oracle_messages(nonce, encoded).await?;
        Ok(())
    }

    async fn submit_oracle_messages(&mut self, nonce: u64, calldata: Vec<u8>) -> Result<(), Error> {
        match self
            .emitter
            .submit_oracle_messages(nonce, calldata.clone())
            .await
        {
            Ok(receipt) => {
                // TODO: After broadcasting a transaction to eip155:1 and getting a
                // transaction receipt, we should monitor it until it get enough
                // confirmations. It's unclear which component should do this task.

                // Record the DataEdge call
                let data_edge_call: DataEdgeCall = {
                    // FIXME: The only purpose of this scope is to transform a
                    // transaction receipt and an encoded payload into a DataEdgeCall
                    // value. We could refactor it into a dedicated function.
                    let web3::types::TransactionReceipt {
                        transaction_hash,
                        block_hash,
                        block_number,
                        ..
                    } = &receipt;
                    if let (Some(block_hash), Some(block_number)) = (block_hash, block_number) {
                        DataEdgeCall::new(
                            transaction_hash.as_bytes(),
                            nonce,
                            block_number.as_u64(),
                            block_hash.as_bytes(),
                            calldata,
                        )
                    } else {
                        todo!(
                            "The expected block number and hash were not present, \
                                         despite having the transaction receipt at hand. \
                                         We should make an new Error variant out of this."
                        )
                    }
                };

                self.store.insert_data_edge_call(data_edge_call).await?;
            }
            Err(other) => return Err(other.into()),
        }

        Ok(())
    }

    async fn is_new_epoch(
        &self,
        chain_id: &Caip2ChainId,
        block_number: u64,
    ) -> Result<bool, Error> {
        let is_protocol_chain = chain_id == self.config.protocol_chain_client.id();

        let is_new_epoch = match self.epoch_tracker.is_new_epoch(block_number).await {
            Ok(boolean) => boolean,
            Err(EpochTrackerError::PreviousEpochNotFound) => {
                // FIXME: At the moment, we are unable to determine the latest epoch from an
                // empty state. Until the Oracle is capable of reacting upon that, we will
                // consider that we have reached a new epoch in such cases.
                warn!("Failed to determine the previous epoch.");
                true
            }
            Err(other_error) => return Err(other_error.into()),
        };

        Ok(is_protocol_chain && is_new_epoch)
    }
}

fn latest_blocks_to_message(
    latest_blocks: HashMap<&Caip2ChainId, web3::types::U64>,
) -> ee::Message {
    Message::SetBlockNumbersForNextEpoch(
        latest_blocks
            .iter()
            .map(|(chain_id, block_number)| {
                (
                    chain_id.as_str().to_owned(),
                    BlockPtr {
                        number: block_number.as_u64(),
                        hash: [0u8; 32], // FIXME
                    },
                )
            })
            .collect(),
    )
}

fn networks_diff_to_message(diff: NetworksDiff) -> Option<ee::Message> {
    if diff.deletions.is_empty() && diff.insertions.is_empty() {
        None
    } else {
        Some(ee::Message::RegisterNetworks {
            remove: diff.deletions.into_iter().map(|x| x.1 as u64).collect(),
            add: diff
                .insertions
                .into_iter()
                .map(|x| x.0.as_str().to_string())
                .collect(),
        })
    }
}
