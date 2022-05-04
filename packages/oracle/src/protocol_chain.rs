use crate::{store::Caip2ChainId, transport::JsonRpcExponentialBackoff};
use secp256k1::SecretKey;
use std::time::Duration;
use url::Url;
use web3::{
    types::{SignedTransaction, TransactionParameters, TransactionReceipt, U64},
    Web3,
};

#[derive(Debug, Clone)]
pub struct ProtocolChain {
    chain_id: Caip2ChainId,
    web3: Web3<JsonRpcExponentialBackoff>,
}
impl ProtocolChain {
    pub fn new(chain_id: Caip2ChainId, jrpc_url: Url, retry_wait_time: Duration) -> Self {
        let web3 = Web3::new(JsonRpcExponentialBackoff::new(jrpc_url, retry_wait_time));
        Self { chain_id, web3 }
    }

    pub async fn sign_transaction(
        &self,
        tx_object: TransactionParameters,
        private_key: &SecretKey,
    ) -> Result<SignedTransaction, web3::Error> {
        self.web3
            .accounts()
            .sign_transaction(tx_object, private_key)
            .await
    }

    pub async fn send_transaction(
        &self,
        signed_transaction: SignedTransaction,
    ) -> Result<TransactionReceipt, web3::Error> {
        self.web3
            .send_raw_transaction_with_confirmation(
                signed_transaction.raw_transaction,
                Duration::from_secs(5), // TODO: set this as a configurable value
                0,                      // TODO: set this as a configurable value
            )
            .await
    }

    pub async fn get_latest_block(&self) -> Result<U64, web3::Error> {
        self.web3.eth().block_number().await
    }

    /// Get a reference to the protocol chain client's chain id.
    pub fn id(&self) -> &Caip2ChainId {
        &self.chain_id
    }
}
