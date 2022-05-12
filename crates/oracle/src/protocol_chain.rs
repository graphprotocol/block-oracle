use crate::{networks::Caip2ChainId, transport::JsonRpcExponentialBackoff};
use futures::future::try_join_all;
use secp256k1::SecretKey;
use std::time::Duration;
use tracing::error;
use url::Url;
use web3::{
    types::{
        SignedTransaction, Transaction, TransactionParameters, TransactionReceipt, H160, H256,
        U256, U64,
    },
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

    /// Scans a block range for relevant transactions.
    ///
    /// Returns a vector of the filtered transactions.
    pub async fn calls_in_block_range(
        &self,
        from_block: U64,
        to_block: U64,
        from_address: H160,
        to_address: H160,
    ) -> Result<Vec<Transaction>, web3::Error> {
        let block_range: Vec<_> = (from_block.as_u64()..=to_block.as_u64()).collect();
        // Prepare all async calls for fetching blocks in range
        let block_futures: Vec<_> = block_range
            .iter()
            .map(|block_number| {
                let block_number: U64 = (*block_number).into();
                self.web3.eth().block(block_number.into())
            })
            .collect();
        // Searching is fallible, so we get a vector of options
        let optional_blocks = try_join_all(block_futures).await?;
        // This will store all transaction hashes found within the fetched blocks
        let mut transaction_hashes: Vec<H256> = Vec::new();
        // Extract the transaction hashes from the the received blocks
        for (opt, block_number) in optional_blocks.into_iter().zip(block_range) {
            if let Some(block) = opt {
                for transaction_hash in block.transactions.into_iter() {
                    transaction_hashes.push(transaction_hash)
                }
            } else {
                error!(%block_number, "Failed to fetch block by number");
            }
        }
        // Prepare the async calls for fetching the full transaction objects
        let transaction_futures: Vec<_> = transaction_hashes
            .iter()
            .map(|transaction_hash| self.web3.eth().transaction((*transaction_hash).into()))
            .collect();
        // Again, searching is fallible, meaning we get back a vector of optional values
        let optional_transactions = try_join_all(transaction_futures).await?;
        // This will hold the filtered transactions that will be returned by thins function
        let mut filtered_transactions: Vec<Transaction> = Vec::new();
        // Iterate over all received transactions and filter the ones we are interested in
        for (opt, transaction_hash) in optional_transactions.into_iter().zip(transaction_hashes) {
            if let Some(transaction) = opt {
                if matches!((transaction.from, transaction.to), (Some(a), Some(b)) if a == from_address && b == to_address)
                {
                    filtered_transactions.push(transaction)
                }
            } else {
                error!(%transaction_hash, "Failed to fetch transaction by hash");
            }
        }

        Ok(filtered_transactions)
    }
}
