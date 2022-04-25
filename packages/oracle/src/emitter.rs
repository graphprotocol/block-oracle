use crate::{store::Caip2ChainId, Config};
use secp256k1::SecretKey;
use web3::{
    transports::Http,
    types::{Bytes, TransactionParameters, H160, U256},
};

/// Responsible for receiving the encodede payload, constructing and signing the
/// transactions to Ethereum Mainnet.
pub struct Emitter {
    client: web3::Web3<Http>,
    contract_address: H160,
    owner_private_key: SecretKey,
}

impl Emitter {
    pub fn new(config: &Config) -> web3::Result<Self> {
        let transport = web3::transports::Http::new(
            config
                .jrpc_providers
                .get(&Caip2ChainId::ethereum_mainnet())
                .expect("Ethereum mainnet provider not found")
                .as_str(),
        )?;
        Ok(Self {
            client: web3::Web3::new(transport),
            contract_address: config.contract_address,
            owner_private_key: config.owner_private_key,
        })
    }

    pub async fn submit_oracle_messages(
        &mut self,
        nonce: u64,
        calldata: Vec<u8>,
    ) -> web3::Result<()> {
        let tx_object = TransactionParameters {
            to: Some(self.contract_address.clone()),
            value: U256::zero(),
            nonce: Some(nonce.into()),
            data: Bytes::from(calldata),
            ..Default::default()
        };
        let private_key = self.owner_private_key.clone();
        let signed = self
            .client
            .accounts()
            .sign_transaction(tx_object, &private_key)
            .await?;

        self.client
            .eth()
            .send_raw_transaction(signed.raw_transaction)
            .await?;

        Ok(())
    }
}
