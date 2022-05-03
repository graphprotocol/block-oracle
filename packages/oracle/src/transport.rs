use backoff::{future::retry, ExponentialBackoff, ExponentialBackoffBuilder};
use secp256k1::SecretKey;
use std::time::Duration;
use tracing::trace;
use url::Url;
use web3::{
    transports::Http,
    types::{SignedTransaction, TransactionParameters, TransactionReceipt, U64},
    Web3,
};

/// A wrapper type around [`Web3`] that supports retries with exponential backoff.
#[derive(Debug, Clone)]
pub struct Transport {
    inner: Web3<Http>,
    retry_strategy: ExponentialBackoff,
}

impl Transport {
    pub fn new(jrpc_url: Url, max_wait_time: Duration) -> Self {
        // Unwrap: URLs were already parsed and are valid.
        let transport = Http::new(jrpc_url.as_str()).expect("failed to create HTTP transport");
        let inner = Web3::new(transport);
        let retry_strategy = ExponentialBackoffBuilder::new()
            .with_max_elapsed_time(Some(max_wait_time))
            .build();
        Self {
            inner,
            retry_strategy,
        }
    }

    pub async fn get_latest_block(&self) -> Result<U64, web3::Error> {
        retry(self.retry_strategy.clone(), || async {
            trace!("Fetching latest blocks");
            self.inner
                .eth()
                .block_number()
                .await
                .map_err(backoff::Error::transient)
        })
        .await
    }

    pub async fn sign_transaction(
        &self,
        tx_object: TransactionParameters,
        private_key: &SecretKey,
    ) -> Result<SignedTransaction, web3::Error> {
        retry(self.retry_strategy.clone(), || async {
            trace!("Signing transaction");
            self.inner
                .accounts()
                .sign_transaction(tx_object.clone(), private_key)
                .await
                .map_err(backoff::Error::transient)
        })
        .await
    }

    pub async fn send_transaction(
        &self,
        signed_transaction: SignedTransaction,
    ) -> Result<TransactionReceipt, web3::Error> {
        retry(self.retry_strategy.clone(), || async {
            trace!("Sending signed transaction");
            self.inner
                .send_raw_transaction_with_confirmation(
                    signed_transaction.raw_transaction.clone(),
                    Duration::from_secs(5), // TODO: set this as a configurable value
                    0,                      // TODO: set this as a configurable value
                )
                .await
                .map_err(backoff::Error::transient)
        })
        .await
    }
}
