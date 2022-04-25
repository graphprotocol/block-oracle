use url::Url;
use web3::{transports::Http, types::U64, Web3};

use crate::store::Caip2ChainId;

#[derive(Debug, Clone)]
pub struct IndexedChain {
    chain_id: Caip2ChainId,
    client: Web3<Http>,
}

impl IndexedChain {
    pub fn new(chain_id: Caip2ChainId, jrpc_url: Url) -> Self {
        // Unwrap: URLs were already parsed and are valid.
        let transport = Http::new(jrpc_url.as_str()).expect("failed to create HTTP transport");
        let client = Web3::new(transport);
        Self { chain_id, client }
    }

    pub fn id(&self) -> &Caip2ChainId {
        &self.chain_id
    }

    pub async fn get_latest_block(&self) -> Result<U64, web3::Error> {
        self.client.eth().block_number().await
    }
}
