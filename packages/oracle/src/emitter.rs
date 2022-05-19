use crate::{config::Config, protocol_chain::ProtocolChain};
use secp256k1::SecretKey;
use thiserror::Error;
use tiny_keccak::{Hasher, Keccak};
use tracing::{debug, info};
use web3::types::{Bytes, TransactionParameters, H160, U256};

const METHOD_SIGNATURE: &'static str = "crossChainEpochOracle(bytes)";

#[derive(Debug, Error)]
pub enum EmitterError {
    #[error(transparent)]
    Web3(#[from] web3::Error),
}

/// Responsible for receiving the encoded payload, constructing and signing the
/// transactions to Ethereum Mainnet.
pub struct Emitter<'a> {
    client: &'a ProtocolChain,
    contract_address: H160,
    owner_private_key: SecretKey,
    owner_address: H160,
}

impl<'a> Emitter<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self {
            client: &config.protocol_chain,
            contract_address: config.contract_address,
            owner_private_key: config.owner_private_key,
            owner_address: config.owner_address,
        }
    }

    pub async fn submit_oracle_messages(
        &mut self,
        call_data: Vec<u8>,
    ) -> Result<web3::types::TransactionReceipt, EmitterError> {
        let nonce = self.client.get_latest_nonce(self.owner_address).await? + 1;

        let call_data_with_identifier = {
            let mut identifier = function_identifier().to_vec();
            identifier.extend(call_data);
            identifier
        };

        let tx_object = TransactionParameters {
            to: Some(self.contract_address),
            value: U256::zero(),
            nonce: Some(nonce.into()),
            data: Bytes::from(call_data_with_identifier),
            ..Default::default()
        };
        let signed = self
            .client
            .sign_transaction(tx_object, &self.owner_private_key)
            .await?;
        debug!(hash = ?signed.transaction_hash, nonce = %nonce, "Signed transaction.");
        let receipt = self.client.send_transaction(signed).await?;
        info!(hash = ?receipt.transaction_hash, nonce = %nonce, "Sent transaction.");
        Ok(receipt)
    }
}

fn function_identifier() -> [u8; 4] {
    let mut buff = [0u8; 4];
    let mut sponge = Keccak::v256();
    sponge.update(METHOD_SIGNATURE.as_bytes());
    sponge.finalize(&mut buff);
    buff
}

#[test]
fn test_function_identifier() {
    /// The first four bytes of [`METHOD_SIGNATURE`]'s Keccak hash.
    const EXPECTED_HEX: &'static str = "a1dce332";
    let actual_bytes = function_identifier();
    let actual_hex = hex::encode(actual_bytes);
    assert_eq!(EXPECTED_HEX, actual_hex);
}
