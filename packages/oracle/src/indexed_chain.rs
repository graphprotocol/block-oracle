use std::time::Duration;

use crate::store::Caip2ChainId;
use crate::transport::JsonRpcExponentialBackoff;
use url::Url;
use web3::types::U64;
use web3::Web3;

#[derive(Debug, Clone)]
pub struct IndexedChain {
    chain_id: Caip2ChainId,
    web3: Web3<JsonRpcExponentialBackoff>,
}

impl IndexedChain {
    pub fn new(chain_id: Caip2ChainId, jrpc_url: Url, retry_wait_time: Duration) -> Self {
        let web3 = Web3::new(JsonRpcExponentialBackoff::new(jrpc_url, retry_wait_time));
        Self { chain_id, web3 }
    }

    pub fn id(&self) -> &Caip2ChainId {
        &self.chain_id
    }

    pub async fn get_latest_block(&self) -> Result<U64, web3::Error> {
        self.web3.eth().block_number().await
    }
}
