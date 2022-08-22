use crate::metrics::METRICS;
use anyhow::Context;
use secp256k1::SecretKey;
use tracing::{debug, info, trace};
use web3::{
    api::Eth,
    contract::{Contract, Options},
    ethabi::Address,
    types::{H256, U256},
    Transport,
};

static EPOCH_MANAGER_ABI: &[u8] = include_bytes!("abi/EpochManager.json");
static DATA_EDGE_ABI: &[u8] = include_bytes!("abi/DataEdge.json");

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
        let data_edge = Contracts::new_contract(DATA_EDGE_ABI, eth, data_edge_address)?;
        let epoch_manager = Contracts::new_contract(EPOCH_MANAGER_ABI, eth, epoch_manager_address)?;
        Ok(Self {
            data_edge,
            epoch_manager,
        })
    }

    fn new_contract(abi: &[u8], eth: &Eth<T>, address: Address) -> anyhow::Result<Contract<T>> {
        Contract::from_json(eth.clone(), address, abi)
            .with_context(|| "Failed to create contract".to_string())
    }

    pub async fn query_current_epoch(&self) -> Result<u64, web3::contract::Error> {
        trace!("Querying the Epoch Manager for the current epoch");
        let epoch_number: U256 = self
            .epoch_manager
            .query("currentEpoch", (), None, Default::default(), None)
            .await?;
        let current_epoch = epoch_number.as_u64();
        debug!("Epoch Manager is at epoch {current_epoch}");
        METRICS.set_current_epoch("manager", current_epoch as i64);
        Ok(current_epoch)
    }

    pub async fn submit_call(
        &self,
        payload: Vec<u8>,
        owner_private_key: &SecretKey,
    ) -> Result<H256, web3::contract::Error> {
        let transaction_hash = self
            .data_edge
            .signed_call(
                "crossChainEpochOracle",
                (payload,),
                Options::default(),
                owner_private_key,
            )
            .await?;
        info!(?transaction_hash, "Sent transaction");
        Ok(transaction_hash)
    }
}
