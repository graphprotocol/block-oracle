use crate::{config::Config, protocol_chain::ProtocolChain};
use secp256k1::SecretKey;
use thiserror::Error;
use tracing::{debug, info};
use web3::types::{Bytes, TransactionParameters, H160, U256};

#[derive(Debug, Error)]
pub enum EmitterError {
    #[error(transparent)]
    Web3(#[from] web3::Error),
}

/// Responsible for receiving the encodede payload, constructing and signing the
/// transactions to Ethereum Mainnet.
pub struct Emitter<'a> {
    client: &'a ProtocolChain,
    contract_address: H160,
    owner_private_key: SecretKey,
}

impl<'a> Emitter<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self {
            client: &config.protocol_chain,
            contract_address: config.contract_address,
            owner_private_key: config.owner_private_key,
        }
    }

    pub async fn submit_oracle_messages(
        &mut self,
        nonce: u64,
        calldata: Vec<u8>,
    ) -> Result<web3::types::TransactionReceipt, EmitterError> {
        let tx_object = TransactionParameters {
            to: Some(self.contract_address),
            value: U256::zero(),
            nonce: Some(nonce.into()),
            data: Bytes::from(calldata),
            ..Default::default()
        };
        let signed = self
            .client
            .sign_transaction(tx_object, &self.owner_private_key)
            .await?;
        debug!(hash = ?signed.transaction_hash, nonce = nonce, "Signed transaction.");
        let receipt = self.client.send_transaction(signed).await?;
        info!(hash = ?receipt.transaction_hash, nonce = nonce, "Sent transaction.");
        Ok(receipt)
    }
}
