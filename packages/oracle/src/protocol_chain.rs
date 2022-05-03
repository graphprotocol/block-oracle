use crate::{store::Caip2ChainId, transport::Transport};
use secp256k1::SecretKey;
use std::time::Duration;
use url::Url;
use web3::types::{SignedTransaction, TransactionParameters, TransactionReceipt, U64};

#[derive(Debug, Clone)]
pub struct ProtocolChain {
    chain_id: Caip2ChainId,
    transport: Transport,
}
impl ProtocolChain {
    pub fn new(chain_id: Caip2ChainId, jrpc_provider: Url, retry_wait_time: Duration) -> Self {
        let transport = Transport::new(jrpc_provider, retry_wait_time);
        Self {
            chain_id,
            transport,
        }
    }

    pub async fn sign_transaction(
        &self,
        tx_object: TransactionParameters,
        private_key: &SecretKey,
    ) -> Result<SignedTransaction, web3::Error> {
        self.transport
            .sign_transaction(tx_object, private_key)
            .await
    }

    pub async fn send_transaction(
        &self,
        signed_transaction: SignedTransaction,
    ) -> Result<TransactionReceipt, web3::Error> {
        self.transport.send_transaction(signed_transaction).await
    }

    pub async fn get_latest_block(&self) -> Result<U64, web3::Error> {
        self.transport.get_latest_block().await
    }

    /// Get a reference to the protocol chain client's chain id.
    pub fn id(&self) -> &Caip2ChainId {
        &self.chain_id
    }
}
