use crate::{jsonrpc_utils::JsonRpcExponentialBackoff, store::Caip2ChainId};
use epoch_encoding::BlockPtr;
use std::time::Duration;
use tracing::error;
use url::Url;
use web3::Web3;

#[derive(Debug, Clone)]
pub struct IndexedChain {
    chain_id: Caip2ChainId,
    web3: Web3<JsonRpcExponentialBackoff>,
}

impl IndexedChain {
    pub fn new(chain_id: Caip2ChainId, jrpc_url: Url, retry_wait_time: Duration) -> Self {
        let web3 = Web3::new(JsonRpcExponentialBackoff::http(jrpc_url, retry_wait_time));
        Self { chain_id, web3 }
    }

    pub fn id(&self) -> &Caip2ChainId {
        &self.chain_id
    }

    pub async fn get_latest_block(&self) -> web3::Result<BlockPtr> {
        let block_num = self.web3.eth().block_number().await?;
        let block_id = web3::types::BlockId::Number(block_num.into());
        let block = self
            .web3
            .eth()
            .block(block_id)
            .await?
            // We were just told that's the latest block number, so it wouldn't
            // make sense for this to fail. How can it *not* find a block with
            // that block number?
            .expect("Invalid block number");

        // Same thing here. We expect data to be consistent across multiple
        // JSON-RPC calls.
        if block.number != Some(block_num) {
            error!(
                block_num1 = ?block_num,
                block_num2 = ?block.number,
                "The JSON-RPC provider is responding to queries with inconsistent data. This is most likely a bug."
            );
        }
        assert_eq!(block.number, Some(block_num));
        assert!(block.hash.is_some());

        Ok(BlockPtr {
            number: block_num.as_u64(),
            hash: block.hash.unwrap().into(),
        })
    }
}
