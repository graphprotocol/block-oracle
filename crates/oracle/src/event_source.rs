use crate::indexed_chain::IndexedChain;
use crate::Config;
use crate::{models::Caip2ChainId, protocol_chain::ProtocolChain};
use epoch_encoding::BlockPtr;
use futures::{
    stream::{FuturesUnordered, StreamExt},
    FutureExt,
};
use std::collections::HashSet;
use std::collections::{hash_map::Entry, HashMap};
use thiserror::Error;
use tracing::error;
use web3::types::U64;

#[derive(Error, Debug)]
pub enum EventSourceError {
    #[error("Failed to poll chain for its latest block")]
    GetLatestBlocksForChain(#[source] web3::Error, Caip2ChainId),
    #[error("Received a JSON RPC result twice for the same chain")]
    DuplicateChainResult(Caip2ChainId),
    #[error("Missed block pointers for a subset of indexed chains")]
    MissingChains(Vec<Caip2ChainId>),
}

impl crate::MainLoopFlow for EventSourceError {
    fn instruction(&self) -> crate::OracleControlFlow {
        use std::ops::ControlFlow::*;
        use EventSourceError::*;
        match self {
            error @ GetLatestBlocksForChain(cause, chain) => {
                error!(%cause, %chain, "{error}");
                Continue(None)
            }
            error @ DuplicateChainResult(duplicated_chain) => {
                error!(%duplicated_chain, "{error}");
                Continue(None)
            }
            error @ MissingChains(missing_chains) => {
                let missing_chains = crate::error_handling::format_slice(&missing_chains);
                error!(%missing_chains, "{error}");
                Continue(None)
            }
        }
    }
}

/// Actively listens for new blocks and reorgs from registered blockchains. Also, it checks the
/// number of confirmations for transactions sent to the DataEdge contract.
#[derive(Debug, Clone)]
pub struct EventSource {
    protocol_chain: ProtocolChain,
    indexed_chains: Vec<IndexedChain>,
}

impl EventSource {
    pub fn new(config: &Config) -> Self {
        Self {
            protocol_chain: config.protocol_chain.clone(),
            indexed_chains: config.indexed_chains.clone(),
        }
    }

    pub async fn get_latest_blocks(
        &self,
    ) -> Result<HashMap<&Caip2ChainId, BlockPtr>, EventSourceError> {
        let mut block_ptr_per_chain: HashMap<&Caip2ChainId, BlockPtr> = HashMap::new();

        let mut tasks = self
            .indexed_chains
            .iter()
            .map(|indexed_chain| {
                indexed_chain
                    .get_latest_block()
                    .map(|block| (indexed_chain.id(), block))
            })
            .collect::<FuturesUnordered<_>>();

        while let Some((chain_id, jrpc_call_result)) = tasks.next().await {
            match jrpc_call_result {
                Ok(block_ptr) => {
                    match block_ptr_per_chain.entry(chain_id) {
                        Entry::Occupied(_) => {
                            return Err(EventSourceError::DuplicateChainResult(chain_id.clone()))
                        }
                        Entry::Vacant(slot) => slot.insert(block_ptr),
                    };
                }
                Err(json_rpc_error) => {
                    return Err(EventSourceError::GetLatestBlocksForChain(
                        json_rpc_error,
                        chain_id.clone(),
                    ))
                }
            }
        }
        // check if we missed any chain
        if block_ptr_per_chain.len() != self.indexed_chains.len() {
            let missing = {
                let current: HashSet<&Caip2ChainId> = block_ptr_per_chain.keys().cloned().collect();
                let expected: HashSet<&Caip2ChainId> =
                    self.indexed_chains.iter().map(|chain| chain.id()).collect();
                current
                    .intersection(&expected)
                    .map(|&x| x.clone())
                    .collect::<Vec<_>>()
            };
            return Err(EventSourceError::MissingChains(missing));
        }

        Ok(block_ptr_per_chain)
    }

    /// Pools the latest block from the protocol chain.
    pub async fn get_latest_protocol_chain_block(&self) -> Result<U64, EventSourceError> {
        let block_number =
            self.protocol_chain
                .get_latest_block()
                .await
                .map_err(|json_rpc_error| {
                    EventSourceError::GetLatestBlocksForChain(
                        json_rpc_error,
                        self.protocol_chain.id().clone(),
                    )
                })?;
        Ok(block_number)
    }
}
