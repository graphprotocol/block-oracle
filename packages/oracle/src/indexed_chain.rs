use std::time::Duration;

use crate::store::Caip2ChainId;
use crate::transport::Transport;
use url::Url;
use web3::types::U64;

#[derive(Debug, Clone)]
pub struct IndexedChain {
    chain_id: Caip2ChainId,
    transport: Transport,
}

impl IndexedChain {
    pub fn new(chain_id: Caip2ChainId, jrpc_url: Url, retry_wait_time: Duration) -> Self {
        let transport = Transport::new(jrpc_url, retry_wait_time);
        Self {
            chain_id,
            transport,
        }
    }

    pub fn id(&self) -> &Caip2ChainId {
        &self.chain_id
    }

    pub async fn get_latest_block(&self) -> Result<U64, web3::Error> {
        self.transport.get_latest_block().await
    }
}
