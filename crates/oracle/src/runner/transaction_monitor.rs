use crate::config::TransactionMonitoringOptions;
use std::collections::HashSet;
use web3::{
    error::Error as Web3Error,
    signing::Key,
    types::{Address, Bytes, TransactionReceipt, TransactionRequest, H256},
    Transport, Web3,
};

#[derive(thiserror::Error, Debug)]
pub enum TransactionMonitorError {
    #[error("failed to determine default values for crafting the transaction")]
    Startup(#[source] Web3Error),
}

pub struct TransactionMonitor<T: Transport, K: Key> {
    client: Web3<T>,
    signing_key: K,

    /// The unsingned transaction that we want to broadcast.
    /// We keep it around so we can control its `nonce` and `gas_price` values.
    transaction_request: TransactionRequest,

    /// Holds the hashes of previously sent transactions, so it can check if any of them got anyt
    /// confirmations.
    sent_transaction_hashes: HashSet<H256>,

    options: TransactionMonitoringOptions,
}

impl<T: Transport, K: Key> TransactionMonitor<T, K> {
    pub async fn new(
        client: Web3<T>,
        signing_key: K,
        contract_address: Address,
        calldata: Bytes,
        options: TransactionMonitoringOptions,
    ) -> Result<Self, TransactionMonitorError> {
        let from = signing_key.address();

        let (nonce, gas_price) = futures::future::try_join(
            client.eth().transaction_count(from, None),
            client.eth().gas_price(),
        )
        .await
        .map_err(TransactionMonitorError::Startup)?;

        let transaction_request = TransactionRequest {
            from,
            to: Some(contract_address),
            gas: todo!("should we set a fixed gas limit?"),
            gas_price: Some(gas_price),
            data: Some(calldata),
            nonce: Some(nonce),
            max_fee_per_gas: todo!("how should we set this?"),
            max_priority_fee_per_gas: todo!("how should we set this?"),
            ..Default::default()
        };

        Ok(Self {
            client,
            transaction_request,
            signing_key,
            options,
            sent_transaction_hashes: Default::default(),
        })
    }

    /// It is possible that previously sent transactions are included in a block while we are trying
    /// to rebroadcast the original transaction.
    ///
    /// If this function detects any confirmation the TransactionManager should abort its ongoing
    /// operations and return the transaction hash of the confirmed transaction to the Oracle.
    async fn check_previously_sent_transactions(
        &self,
    ) -> Result<TransactionReceipt, TransactionMonitorError> {
        todo!()
    }

    /// Broadcasts the transaction and waits for its confirmation.
    ///
    /// It will bump the gas price and retry if the transaction takes too long to confirm.
    /// While doing so, it will also check if previously sent transactions were confirmed.
    ///
    /// This function will return an error if we exhaust its maximum retries attempts.
    pub async fn execute_transaction(&self) -> Result<TransactionReceipt, TransactionMonitorError> {
        todo!()
    }
}
