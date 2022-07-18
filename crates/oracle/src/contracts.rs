use anyhow::Context;
use secp256k1::SecretKey;
use tracing::info;
use web3::{
    api::Eth,
    contract::{Contract, Options},
    ethabi::{Address, Bytes},
    types::{H256, U256},
    Transport,
};

const DATA_EDGE_CONTRACT_FUNCTION_NAME: &'static str = "crossChainEpochOracle";

pub struct Contracts<T>
where
    T: Clone + Transport,
{
    data_edge: Contract<T>,
    epoch_manager: Contract<T>,
}

impl<T> Contracts<T>
where
    T: Clone + Transport,
{
    pub fn new(
        eth: &Eth<T>,
        data_edge_address: Address,
        epoch_manager_address: Address,
    ) -> anyhow::Result<Self> {
        let data_edge = Contracts::new_contract("abi/DataEdge.json", eth, data_edge_address)?;
        let epoch_manager =
            Contracts::new_contract("abi/EpochManager.json", eth, epoch_manager_address)?;
        Ok(Self {
            data_edge,
            epoch_manager,
        })
    }

    fn new_contract(abi_file: &str, eth: &Eth<T>, address: Address) -> anyhow::Result<Contract<T>> {
        let json = std::fs::read_to_string(abi_file)
            .context("Failed to read ABI JSON file for at {abi_file}")?;

        Contract::from_json(eth.clone(), address, json.as_ref())
            .context("Failed to create contract for ABI JSON file {abi_file}")
    }

    pub async fn query_current_epoch(&self) -> Result<u64, web3::contract::Error> {
        let epoch_number: U256 = self
            .epoch_manager
            .query("currentEpoch", (), None, Default::default(), None)
            .await?;
        Ok(epoch_number.as_u64())
    }

    pub async fn submit_call(
        &self,
        payload: Vec<u8>,
        owner_private_key: &SecretKey,
    ) -> Result<H256, web3::contract::Error> {
        let payload = Bytes::from(payload);
        let transaction_hash = self
            .data_edge
            .signed_call(
                DATA_EDGE_CONTRACT_FUNCTION_NAME,
                (payload,),
                Options::default(),
                owner_private_key,
            )
            .await?;
        info!(?transaction_hash, "Sent transaction");
        Ok(transaction_hash)
    }
}
