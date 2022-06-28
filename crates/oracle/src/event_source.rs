use crate::jsonrpc_utils::get_latest_block;
use crate::{Caip2ChainId, Config, JrpcExpBackoff, JrpcProviderForChain};
use epoch_encoding::BlockPtr;
use futures::{
    stream::{FuturesUnordered, StreamExt},
    FutureExt,
};
use std::collections::HashMap;
use thiserror::Error;
use tracing::error;

#[derive(Error, Debug)]
pub enum EventSourceError {
    #[error("Failed to poll chain for its latest block")]
    GetLatestBlocksForChain(#[source] web3::Error, Caip2ChainId),
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
        }
    }
}

/// Actively listens for new blocks and reorgs from registered blockchains. Also, it checks the
/// number of confirmations for transactions sent to the DataEdge contract.
#[derive(Debug, Clone)]
pub struct EventSource {
    pub protocol_chain: JrpcProviderForChain<JrpcExpBackoff>,
    pub indexed_chains: Vec<JrpcProviderForChain<JrpcExpBackoff>>,
}

impl EventSource {
    pub fn new(config: &Config) -> Self {
        let backoff_max = config.retry_strategy_max_wait_time;
        let protocol_chain = {
            let transport =
                JrpcExpBackoff::http(config.protocol_chain.jrpc_url.clone(), backoff_max);
            JrpcProviderForChain::new(config.protocol_chain.id.clone(), transport)
        };
        let indexed_chains = config
            .indexed_chains
            .iter()
            .map(|chain| {
                let transport = JrpcExpBackoff::http(chain.jrpc_url.clone(), backoff_max);
                JrpcProviderForChain::new(chain.id.clone(), transport)
            })
            .collect();

        Self {
            protocol_chain,
            indexed_chains,
        }
    }

    pub async fn get_latest_blocks(
        &self,
    ) -> Result<HashMap<Caip2ChainId, BlockPtr>, EventSourceError> {
        let mut block_ptr_per_chain: HashMap<Caip2ChainId, BlockPtr> = HashMap::new();
        let mut tasks = self
            .indexed_chains
            .iter()
            .cloned()
            .map(|chain| get_latest_block(chain.web3).map(|block| (chain.chain_id, block)))
            .collect::<FuturesUnordered<_>>();

        while let Some((chain_id, jrpc_call_result)) = tasks.next().await {
            assert!(!block_ptr_per_chain.contains_key(&chain_id));

            let block_ptr = jrpc_call_result
                .map_err(|err| EventSourceError::GetLatestBlocksForChain(err, chain_id.clone()))?;
            block_ptr_per_chain.insert(chain_id, block_ptr);
        }

        assert!(block_ptr_per_chain.len() == self.indexed_chains.len());

        Ok(block_ptr_per_chain)
    }

    /// Pools the latest block from the protocol chain.
    pub async fn get_latest_protocol_chain_block(&self) -> Result<BlockPtr, EventSourceError> {
        match get_latest_block(self.protocol_chain.web3.clone()).await {
            Ok(block) => Ok(block),
            Err(e) => Err(EventSourceError::GetLatestBlocksForChain(
                e,
                self.protocol_chain.chain_id.clone(),
            )),
        }
    }
}
