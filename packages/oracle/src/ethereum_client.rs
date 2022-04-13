use super::CONFIG;
use async_trait::async_trait;
use epoch_encoding::{Blockchain, Transaction};
use web3::{
    transports::Http,
    types::{Bytes, TransactionParameters, U256},
};

/// Responsible for receiving the encodede payload, constructing and signing the
/// transactions to Ethereum Mainnet.
pub struct EthereumClient {
    client: web3::Web3<Http>,
}

impl EthereumClient {
    pub fn new(transport: web3::transports::Http) -> Self {
        let client = web3::Web3::new(transport);
        Self { client }
    }
}

#[async_trait]
impl Blockchain for EthereumClient {
    type Err = String;

    async fn submit_oracle_messages(&mut self, transaction: Transaction) -> Result<(), Self::Err> {
        let tx_object = TransactionParameters {
            to: Some(CONFIG.contract_address.clone()),
            value: U256::zero(),
            nonce: Some(transaction.nonce.into()),
            data: Bytes::from(transaction.payload),
            ..Default::default()
        };
        let private_key = CONFIG.owner_private_key.clone();
        let signed = self
            .client
            .accounts()
            .sign_transaction(tx_object, &private_key)
            .await
            .unwrap();

        self.client
            .eth()
            .send_raw_transaction(signed.raw_transaction)
            .await
            .unwrap();

        Ok(())
    }
}
