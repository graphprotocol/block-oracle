use crate::{store::Caip2ChainId, transport::JsonRpcExponentialBackoff};
use secp256k1::SecretKey;
use std::time::Duration;
use url::Url;
use web3::{
    types::{SignedTransaction, TransactionParameters, TransactionReceipt, H160, U256, U64},
    Web3,
};

#[derive(Debug, Clone)]
pub struct ProtocolChain {
    chain_id: Caip2ChainId,
    web3: Web3<JsonRpcExponentialBackoff>,
    transaction_confirmation_poll_interval_in_seconds: u64,
    transaction_confirmation_count: usize,
}
impl ProtocolChain {
    pub fn new(
        chain_id: Caip2ChainId,
        jrpc_url: Url,
        retry_wait_time: Duration,
        transaction_confirmation_poll_interval_in_seconds: u64,
        transaction_confirmation_count: usize,
    ) -> Self {
        let web3 = Web3::new(JsonRpcExponentialBackoff::new(jrpc_url, retry_wait_time));
        Self {
            chain_id,
            web3,
            transaction_confirmation_poll_interval_in_seconds,
            transaction_confirmation_count,
        }
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

    pub async fn get_latest_nonce(&self, address: H160) -> Result<U256, web3::Error> {
        self.web3.eth().transaction_count(address, None).await
    }

    /// Get a reference to the protocol chain client's chain id.
    pub fn id(&self) -> &Caip2ChainId {
        &self.chain_id
    }

    pub async fn calls_in_block_range(
        &self,
        from_block: U64,
        to_block: U64,
        from_address: H160,
        to_address: H160,
    ) -> Result<Vec<()>, web3::Error> {
        todo!()
    }
}
