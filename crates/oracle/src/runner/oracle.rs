use crate::runner::jrpc_utils::{get_latest_block, get_latest_blocks, JrpcExpBackoff};
use crate::runner::{hex_string, Error};
use crate::{
    Caip2ChainId, Config, Error, JrpcExpBackoff, JrpcProviderForChain, NetworksDiff,
    SubgraphStateTracker,
};
use epoch_encoding::{BlockPtr, Encoder, Message, CURRENT_ENCODING_VERSION};
use tracing::{debug, error, info, warn};

/// The main application in-memory state.
pub struct Oracle {
    config: Config,
    protocol_chain: JrpcProviderForChain<JrpcExpBackoff>,
    indexed_chains: Vec<JrpcProviderForChain<JrpcExpBackoff>>,
    contracts: Contracts<JrpcExpBackoff>,
}

impl Oracle {
    pub fn new(config: Config) -> Self {
        let subgraph_state = SubgraphStateTracker::new(config.subgraph_url.clone());
}

impl Oracle {
    pub fn new(config: &'static Config) -> Self {
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
            contracts,
        }
    }

    /// Runs a new polling iteration and submits new messages to the subgraph,
    /// if necessary.
    pub async fn run(&mut self) -> Result<(), Error> {
        info!("New polling iteration.");

        // Before anything else, we must get the latest subgraph state
        debug!("Querying the subgraph state...");
        let subgraph_state =
            query_subgraph(&self.config.subgraph_url, &self.config.bearer_token).await?;

        if self.detect_new_epoch(&subgraph_state).await? {
            self.handle_new_epoch(&subgraph_state).await?;
        } else {
            debug!("No epoch change detected.");
        }
        Ok(())
    }

    /// Checks if the Subgraph should consider that the Subgraph is at a previous epoch compared to
    /// the Epoch Manager.
    async fn detect_new_epoch(&self, subgraph_state: &SubgraphState) -> Result<bool, Error> {
        // Then we check if there is a new epoch by looking at the current Subgraph state.
        let last_block_number_indexed_by_subgraph = match self.is_new_epoch(subgraph_state).await? {
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
    async fn is_new_epoch(&self, subgraph_state: &SubgraphState) -> Result<NewEpochCheck, Error> {
        use NewEpochCheck::*;
        let (subgraph_latest_indexed_block, subgraph_latest_epoch) = {
            match subgraph_state
                .global_state
                .as_ref()
                .and_then(|gs| gs.latest_epoch_number)
            {
                Some(epoch_num) => (subgraph_state.last_indexed_block_number, epoch_num),
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

    async fn handle_new_epoch(&mut self, subgraph_state: &SubgraphState) -> Result<(), Error> {
        info!("Entering a new epoch.");
        info!("Collecting latest block information from all indexed chains.");

        self.query_owner_eth_balance().await?;

        let latest_blocks_res = get_latest_blocks(&self.indexed_chains).await;
        let latest_blocks = latest_blocks_res
            .iter()
            .filter_map(|(chain_id, res)| -> Option<(Caip2ChainId, BlockPtr)> {
                match res {
                    Ok(block) => {
                        METRICS.set_latest_block_number(
                            chain_id.as_str(),
                            "jrpc",
                            block.number as i64,
                        );
                        Some((chain_id.clone(), *block))
                    }
                    Err(e) => {
                        warn!(
                            chain_id = chain_id.as_str(),
                            error = e.to_string().as_str(),
                            "Failed to get latest block from chain. Skipping."
                        );
                        None
                    }
                }
            })
            .collect();

        let payload = self.produce_next_payload(&subgraph_state, latest_blocks)?;
        let tx_hash = self
            .contracts
            .submit_call(payload, &self.config.owner_private_key)
            .await
            .map_err(Error::CantSubmitTx)?;
        METRICS.set_last_sent_message();
        info!(
            tx_hash = tx_hash.to_string().as_str(),
        let payload = self.produce_next_payload(latest_blocks)?;
        // component should do this task.

        Ok(())
    }

    /// Queries the Protocol Chain for the current balance of the Owner's account.
    ///
    /// Used for monitoring and logging.
    async fn query_owner_eth_balance(&self) -> Result<usize, Error> {
        let balance = self
            .protocol_chain
            .web3
            .eth()
            .balance(self.config.owner_address, None)
            .await
            .map_err(Error::BadJrpcProtocolChain)?
            .as_usize();
        info!("Owner ETH Balance is {} gwei", balance);
        METRICS.set_wallet_balance(balance as i64);
        Ok(balance)
    }
}

        let mut messages = vec![];

    // We're not interested in unregistered networks. So we isolate them into a separate
    // collection, log them, and finally discard them.
    let mut ignored_networks = Vec::new();
    for chain_id in latest_blocks.keys().cloned() {
        if !registered_networks
            .iter()
            .any(|network| network.id == chain_id)
        {
            ignored_networks.push(chain_id);
        }
    }
    if !ignored_networks.is_empty() {
        warn!(
            ignored_networks = ?ignored_networks,
            "Multiple networks present in the configuration file are not registered"
        {
            ignored_networks.push(chain_id);
        }
    }
    if !ignored_networks.is_empty() {
        warn!(
            ignored_networks = ?ignored_networks,
            "Multiple networks present in the configuration file are not registered"
        );

            return Err(Error::MalconfiguredIndexedChains(networks_diff));
        }
        let registered_networks = registered_networks(&self.subgraph_state);
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

    /// Queries the Protocol Chain for the current balance of the Owner's account.
    ///
    /// Used for monitoring and logging.
    async fn query_owner_eth_balance(&self) -> Result<usize, Error> {
        let balance = self
            .protocol_chain
            .web3
            .eth()
            .balance(self.config.owner_address, None)
            .await
            .map_err(Error::BadJrpcProtocolChain)?
            .as_usize();
        info!("Owner ETH Balance is {} gwei", balance);
        METRICS.set_wallet_balance(balance as i64);
        Ok(balance)
    }
}

fn registered_networks(subgraph_state: &SubgraphStateTracker) -> Vec<crate::subgraph::Network> {
    if let Some(gs) = subgraph_state
        .result()
        .and_then(|state| state.global_state.as_ref())
    {
        gs.networks.clone()
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

    /// The Epoch Subgraph is at a previous epoch than the Epoch Manager.
    PreviousEpoch { subgraph_latest_indexed_block: u64 },
    /// The Epoch Subgraph is at the same epoch as the Epoch Manager.
    SameEpoch,
}


}

fn latest_blocks_to_message(latest_blocks: BTreeMap<Caip2ChainId, BlockPtr>) -> ee::Message {
    Message::SetBlockNumbersForNextEpoch(
        latest_blocks
            .into_iter()
            .map(|(chain_id, block_ptr)| (chain_id.as_str().to_owned(), block_ptr))
            .collect(),