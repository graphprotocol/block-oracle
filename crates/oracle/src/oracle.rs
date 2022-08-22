use crate::{
    contracts::Contracts,
    hex_string,
    jrpc_utils::{get_latest_block, get_latest_blocks},
    metrics::METRICS,
    Caip2ChainId, Config, Error, JrpcExpBackoff, JrpcProviderForChain, NetworksDiff, SubgraphQuery,
    SubgraphStateTracker,
};
use epoch_encoding::{self as ee, BlockPtr, Encoder, Message, CURRENT_ENCODING_VERSION};
use std::{cmp::Ordering, collections::BTreeMap};
use tracing::{debug, error, info, warn};

/// The main application in-memory state.
pub struct Oracle {
    config: &'static Config,
    protocol_chain: JrpcProviderForChain<JrpcExpBackoff>,
    indexed_chains: Vec<JrpcProviderForChain<JrpcExpBackoff>>,
    subgraph_state: SubgraphStateTracker<SubgraphQuery>,
    contracts: Contracts<JrpcExpBackoff>,
}

impl Oracle {
    pub fn new(config: &'static Config) -> Self {
        let subgraph_api = SubgraphQuery::new(config.subgraph_url.clone());
        let subgraph_state = SubgraphStateTracker::new(subgraph_api);
        let backoff_max = config.retry_strategy_max_wait_time;
        let protocol_chain = {
            let transport = JrpcExpBackoff::http(
                config.protocol_chain.jrpc_url.clone(),
                config.protocol_chain.id.clone(),
                backoff_max,
            );
            JrpcProviderForChain::new(config.protocol_chain.id.clone(), transport)
        };
        let indexed_chains = config
            .indexed_chains
            .iter()
            .map(|chain| {
                let transport =
                    JrpcExpBackoff::http(chain.jrpc_url.clone(), chain.id.clone(), backoff_max);
                JrpcProviderForChain::new(chain.id.clone(), transport)
            })
            .collect();
        let contracts = Contracts::new(
            &protocol_chain.web3.eth(),
            config.data_edge_address,
            config.epoch_manager_address,
        )
        .expect("Failed to initialize Block Oracle's required contracts");

        Self {
            config,
            protocol_chain,
            indexed_chains,
            subgraph_state,
            contracts,
        }
    }

    /// Runs a new polling iteration and submits new messages to the subgraph,
    /// if necessary.
    pub async fn run(&mut self) -> Result<(), Error> {
        info!("New polling iteration.");
        if self.detect_new_epoch().await? {
            self.handle_new_epoch().await?;
        } else {
            debug!("No epoch change detected.");
        }
        Ok(())
    }

    /// Checks if the Subgraph should consider that the Subgraph is at a previous epoch compared to
    /// the Epoch Manager.
    async fn detect_new_epoch(&mut self) -> Result<bool, Error> {
        // Before anything else, we must get the latest subgraph state
        debug!("Querying the subgraph state...");
        self.subgraph_state.refresh().await;
        self.subgraph_state.result()?;

        // Then we check if there is a new epoch by looking at the current Subgraph state.
        let last_block_number_indexed_by_subgraph = match self.is_new_epoch().await? {
            // If the Subgraph is uninitialized, we should skip the freshness and epoch check
            // and return `true`, indicating that the Oracle should send a message.
            //
            // Otherwise this could lead to a deadlock in which the Oracle never sends any
            // message to the Subgraph while waiting for it to be initialized.
            NewEpochCheck::SubgraphIsUninitialized => return Ok(true),

            // The Subgraph is at the same epoch as the Epoch Manager.
            NewEpochCheck::SameEpoch => return Ok(false),

            // The Subgraph is at a previous epoch than the Epoch Manager, but we still need to
            // check if the former is fresh.
            NewEpochCheck::PreviousEpoch {
                subgraph_latest_indexed_block,
            } => subgraph_latest_indexed_block,
        };

        let protocol_chain_current_block = get_latest_block(self.protocol_chain.web3.clone())
            .await
            .map_err(Error::BadJrpcProtocolChain)?;
        debug!(
            number = protocol_chain_current_block.number,
            hash = hex::encode(protocol_chain_current_block.hash).as_str(),
            "Got the latest block from the protocol chain."
        );

        let is_fresh = freshness::subgraph_is_fresh(
            last_block_number_indexed_by_subgraph.into(),
            protocol_chain_current_block.number.into(),
            self.protocol_chain.clone(),
            self.config.owner_address,
            self.config.data_edge_address,
            self.config.freshness_threshold,
        )
        .await
        .map_err(Error::BadJrpcProtocolChain)?;
        if !is_fresh {
            error!("Subgraph is not fresh");
            Err(Error::SubgraphNotFresh)
        } else {
            Ok(true)
        }
    }

    /// Checks if the Subgraph epoch is behind the Epoch Manager's current epoch.
    ///
    /// Returns a pair of values indicating: 1) if there is a new epoch; and 2) the latest block
    /// number indexed by the subgraph. Returns `None` if the Subgraph is not initialized.
    async fn is_new_epoch(&self) -> Result<NewEpochCheck, Error> {
        use NewEpochCheck::*;
        let (subgraph_latest_indexed_block, subgraph_latest_epoch) = {
            match self
                .subgraph_state
                .result()?
                .and_then(|(block, state)| state.latest_epoch_number.map(|en| (*block, en)))
            {
                Some(state) => state,
                None => {
                    warn!("The subgraph state is uninitialized");
                    return Ok(SubgraphIsUninitialized);
                }
            }
        };
        debug!("Subgraph is at epoch {subgraph_latest_epoch}");
        METRICS.set_current_epoch("subgraph", subgraph_latest_epoch as i64);
        let manager_current_epoch = self.contracts.query_current_epoch().await?;
        match subgraph_latest_epoch.cmp(&manager_current_epoch) {
            Ordering::Less => Ok(PreviousEpoch {
                subgraph_latest_indexed_block,
            }),
            Ordering::Equal => Ok(SameEpoch),
            Ordering::Greater => Err(Error::EpochManagerBehindSubgraph {
                manager: manager_current_epoch,
                subgraph: subgraph_latest_epoch,
            }),
        }
    }

    async fn handle_new_epoch(&mut self) -> Result<(), Error> {
        info!("Entering a new epoch.");

        info!("Collecting latest block information from all indexed chains.");
        let latest_blocks_res = get_latest_blocks(&self.indexed_chains).await;
        let latest_blocks = latest_blocks_res
            .iter()
            .filter_map(|(chain_id, res)| match res {
                Ok(block) => Some((chain_id.clone(), *block)),
                Err(e) => {
                    warn!(
                        chain_id = chain_id.as_str(),
                        error = e.to_string().as_str(),
                        "Failed to get latest block from chain. Skipping."
                    );
                    None
                }
            })
            .collect();

        let payload = self.produce_next_payload(latest_blocks)?;
        let tx_hash = self
            .contracts
            .submit_call(payload, &self.config.owner_private_key)
            .await
            .map_err(Error::CantSubmitTx)?;
        info!(
            tx_hash = tx_hash.to_string().as_str(),
            "Contract call submitted successfully."
        );

        // TODO: After broadcasting a transaction to the protocol chain and getting a transaction
        // receipt, we should monitor it until it get enough confirmations. It's unclear which
        // component should do this task.

        Ok(())
    }

    fn produce_next_payload(
        &self,
        latest_blocks: BTreeMap<Caip2ChainId, BlockPtr>,
    ) -> Result<Vec<u8>, Error> {
        let registered_networks = registered_networks(&self.subgraph_state);

        let mut messages = vec![];

        // First, we need to make sure that there are no pending
        // `RegisterNetworks` messages.
        let networks_diff = { NetworksDiff::calculate(&registered_networks, self.config) };
        info!(
            created = networks_diff.insertions.len(),
            deleted = networks_diff.deletions.len(),
            "Performed indexed chain diffing."
        );

        if !networks_diff.is_empty() {
            return Err(Error::MalconfiguredIndexedChains(networks_diff));
        }

        messages.push(latest_blocks_to_message(latest_blocks));

        let available_networks: Vec<(String, epoch_encoding::Network)> = {
            registered_networks
                .into_iter()
                .map(|network| (network.id.as_str().to_owned(), network.into()))
                .collect()
        };

        debug!(
            messages = ?messages,
            networks = ?available_networks,
            messages_count = messages.len(),
            networks_count = available_networks.len(),
            "Compressing message(s)."
        );

        let mut compression_engine = Encoder::new(CURRENT_ENCODING_VERSION, available_networks)
            .expect("Can't prepare for encoding because something went wrong.");
        let compression_engine_initially = compression_engine.clone();

        let compressed = compression_engine
            .compress(&messages[..])
            .unwrap_or_else(|error| {
                panic!("Encoding failed. Messages {:?}. Error: {}", messages, error)
            });
        debug!(
            compressed = ?compressed,
            networks = ?compression_engine.network_deltas(),
            "Successfully compressed message(s)."
        );
        let encoded = compression_engine.encode(&compressed);
        debug!(
            encoded = hex_string(&encoded).as_str(),
            "Successfully encoded message(s)."
        );

        assert_ne!(
            compression_engine, compression_engine_initially,
            "The encoder has identical internal state compared to what \
            it had before these new messages. This is a bug!"
        );

        Ok(encoded)
    }
}

fn registered_networks(
    subgraph_state: &SubgraphStateTracker<SubgraphQuery>,
) -> Vec<crate::subgraph::Network> {
    if let Ok(Some(state)) = subgraph_state.result() {
        state.1.networks.clone()
    } else {
        // The subgraph is uninitialized, so there's no registered networks at all.
        vec![]
    }
}

fn latest_blocks_to_message(latest_blocks: BTreeMap<Caip2ChainId, BlockPtr>) -> ee::Message {
    Message::SetBlockNumbersForNextEpoch(
        latest_blocks
            .into_iter()
            .map(|(chain_id, block_ptr)| (chain_id.as_str().to_owned(), block_ptr))
            .collect(),
    )
}

mod freshness {
    use crate::{jrpc_utils::calls_in_block_range, models::JrpcProviderForChain};
    use tracing::{debug, trace};
    use web3::types::{H160, U64};

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
    pub async fn subgraph_is_fresh<T>(
        subgraph_latest_block: U64,
        current_block: U64,
        protocol_chain: JrpcProviderForChain<T>,
        owner_address: H160,
        contract_address: H160,
        freshness_threshold: u64,
    ) -> web3::Result<bool>
    where
        T: web3::Transport,
    {
        // If this ever happens, then there must be a serious bug in the code
        if subgraph_latest_block > current_block {
            return Ok(true);
        }
        let block_distance = (current_block - subgraph_latest_block).as_u64();
        if block_distance == 0 {
            return Ok(true);
        } else if block_distance > freshness_threshold {
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
        let calls = calls_in_block_range(
            protocol_chain.web3,
            subgraph_latest_block.as_u64()..=current_block.as_u64(),
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

/// Used inside the 'Oracle::is_new_epoch' method to return information about the Epoch Subgraph
/// current state.
enum NewEpochCheck {
    /// The Epoch Subgraph has no initial state.
    SubgraphIsUninitialized,
    /// The Epoch Subgraph is at a previous epoch than the Epoch Manager.
    PreviousEpoch { subgraph_latest_indexed_block: u64 },
    /// The Epoch Subgraph is at the same epoch as the Epoch Manager.
    SameEpoch,
}
