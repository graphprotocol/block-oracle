use crate::config::TransactionMonitoringOptions;
use either::Either;
use std::collections::HashSet;
use tokio::time::{timeout, Duration};
use web3::{
    api::{Accounts, Namespace},
    error::Error as Web3Error,
    signing::{Key, SecretKeyRef},
    types::{Address, Bytes, TransactionParameters, TransactionReceipt, H256, U256},
    Transport, Web3,
};

#[derive(thiserror::Error, Debug)]
pub enum TransactionMonitorError {
    #[error("failed to determine default values for crafting the transaction: {0}")]
    Startup(#[source] Web3Error),
    #[error("failed to sign the transaction parameters: {0}")]
    Signing(#[source] Web3Error),
    #[error("failed to send a signed transaction: {0}")]
    Provider(#[source] Web3Error),
    #[error("failed to send transaction after exhausting all retries")]
    BroadcastFailure,
}

pub struct TransactionMonitor<'a, T: Transport> {
    client: Web3<T>,
    signing_key: SecretKeyRef<'a>,

    /// The unsingned transaction that we want to broadcast.
    /// We keep it around so we can control its `nonce` and `gas_price` values.
    transaction_parameters: TransactionParameters,

    /// Holds the hashes of previously sent transactions, so it can check if any of them got anyt
    /// confirmations.
    sent_transaction_hashes: HashSet<H256>,

    options: TransactionMonitoringOptions,
}

impl<'a, T: Transport> TransactionMonitor<'a, T> {
    pub async fn new(
        client: Web3<T>,
        signing_key: SecretKeyRef<'a>,
        contract_address: Address,
        calldata: Bytes,
        options: TransactionMonitoringOptions,
    ) -> Result<TransactionMonitor<'a, T>, TransactionMonitorError> {
        let from = signing_key.address();

        let (nonce, gas_price) = futures::future::try_join(
            client.eth().transaction_count(from, None),
            client.eth().gas_price(),
        )
        .await
        .map_err(TransactionMonitorError::Startup)?;

        let transaction_parameters = TransactionParameters {
            to: Some(contract_address),
            gas: options.gas_limit.into(),
            gas_price: Some(gas_price),
            data: calldata,
            nonce: Some(nonce),
            max_fee_per_gas: options.max_fee_per_gas.map(Into::into),
            max_priority_fee_per_gas: options.max_priority_fee_per_gas.map(Into::into),

            ..Default::default()
        };

        Ok(Self {
            client,
            transaction_parameters,
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
    async fn check_previously_sent_transactions(&self) {
        todo!()
    }

    /// Attempts to sign and broadcast a transaction, returing its receipt on success.
    /// This function has two error types:
    /// - the generalist `web3::error:Error, and
    /// - the hash of the transaction that we given up waiting for it to be confirmed.
    async fn send_transaction_and_wait_for_confirmation(
        &self,
        transaction_parameters: TransactionParameters,
    ) -> Result<TransactionReceipt, Either<Web3Error, H256>> {
        // Sign the transaction
        let signed_transaction = Accounts::new(self.client.transport().clone())
            .sign_transaction(transaction_parameters, &*self.signing_key)
            .await
            .map_err(Either::Left)?;

        let transaction_hash = signed_transaction.transaction_hash;

        // Wrap the transaction broadcast in a tokio::timeout future
        let send_transaction_future = web3::confirm::send_raw_transaction_with_confirmation(
            self.client.transport().clone(),
            signed_transaction.raw_transaction,
            Duration::from_secs(self.options.poll_interval_in_seconds),
            self.options.confirmations,
        );
        let with_timeout = timeout(
            Duration::from_secs(self.options.confirmation_timeout_in_seconds),
            send_transaction_future,
        );

        match with_timeout.await {
            Ok(Ok(receipt)) => Ok(receipt),
            Ok(Err(web3_error)) => Err(Either::Left(web3_error)),
            Err(_timed_out) => Err(Either::Right(transaction_hash)),
        }
    }

    /// Broadcasts the transaction and waits for its confirmation.
    ///
    /// It will bump the gas price and retry if the transaction takes too long to confirm.
    /// While doing so, it will also check if previously sent transactions were confirmed.
    ///
    /// This function will return an error if we exhaust its maximum retries attempts.
    pub async fn execute_transaction(&self) -> Result<TransactionReceipt, TransactionMonitorError> {
        let mut retries = self.options.max_retries;

        let mut sent_transactions = HashSet::new();
        let mut transaction_parameters = self.transaction_parameters.clone();

        while retries > 0 {
            match self
                .send_transaction_and_wait_for_confirmation(transaction_parameters.clone())
                .await
            {
                Ok(receipt) => return Ok(receipt),
                Err(Either::Left(web3_error)) => {
                    // This means that we failed handling the transaction and got a provider error
                    // before the timeout.
                    return Err(TransactionMonitorError::Provider(web3_error));
                }
                Err(Either::Right(transaction_hash)) => {
                    // This means that we timed out waiting for the transaction to be confirmed.
                    sent_transactions.insert(transaction_hash);
                    transaction_parameters.gas_price.as_mut().map(|gas| {
                        *gas = bump_gas(*gas, self.options.gas_percentual_increase)
                            .expect("gas_price calculation won't overflow a 256-bit number")
                    });
                    retries -= 1;
                }
            };
        }

        // At this point, we have exhausted all retry attempts
        Err(TransactionMonitorError::BroadcastFailure)
    }
}

fn bump_gas(gas_price: U256, percentual_increase: u32) -> Option<U256> {
    let factor = U256::from(100 + percentual_increase);
    let denominator = U256::from(100);
    gas_price.checked_mul(factor)?.checked_div(denominator)
}

#[test]
fn test_bump_gas() {
    let input: U256 = 1000.into();
    let percentual_increase: u32 = 25;
    let expected: U256 = 1250.into();
    let output = bump_gas(input, percentual_increase);
    assert_eq!(output, Some(expected));
}
