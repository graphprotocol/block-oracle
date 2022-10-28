use crate::metrics::METRICS;
use anyhow::Context;
use secp256k1::SecretKey;
use std::time::Duration;
use tracing::{debug, info, trace};
use web3::{
    api::Eth,
    contract::{tokens::Tokenize, Contract},
    ethabi::Address,
    types::{TransactionReceipt, U256},
    Transport,
};

static EPOCH_MANAGER_ABI: &[u8] = include_bytes!("abi/EpochManager.json");
static EPOCH_MANAGER_FUNCTION_NAME: &str = "currentEpoch";
static DATA_EDGE_ABI: &[u8] = include_bytes!("abi/DataEdge.json");
static DATA_EDGE_FUNCTION_NAME: &str = "crossChainEpochOracle";

pub struct Contracts<T>
where
    T: Clone + Transport,
{
    data_edge: Contract<T>,
    epoch_manager: Contract<T>,
    confirmation_timeout: Duration,
}

impl<T> Contracts<T>
where
    T: Clone + Transport,
{
    pub fn new(
        eth: &Eth<T>,
        data_edge_address: Address,
        epoch_manager_address: Address,
        confirmation_timeout: Duration,
    ) -> anyhow::Result<Self> {
        let data_edge = Contracts::new_contract(DATA_EDGE_ABI, eth, data_edge_address)?;
        let epoch_manager = Contracts::new_contract(EPOCH_MANAGER_ABI, eth, epoch_manager_address)?;
        Ok(Self {
            data_edge,
            epoch_manager,
            confirmation_timeout,
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
            .query(
                EPOCH_MANAGER_FUNCTION_NAME,
                (),
                None,
                Default::default(),
                None,
            )
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
    ) -> Result<TransactionReceipt, web3::contract::Error> {
        info!("Sending transaction and waiting for confirmations",);
        let transaction_receipt: TransactionReceipt = todo!("transaction monitor here");
        info!(?transaction_receipt.transaction_hash, "Transaction confirmed");
        Ok(transaction_receipt)
    }

    pub fn abi_encode_data_edge_payload(
        &self,
        params: impl Tokenize,
    ) -> Result<Vec<u8>, web3::ethabi::Error> {
        self.data_edge
            .abi()
            .function(DATA_EDGE_FUNCTION_NAME)
            .and_then(|function| function.encode_input(&params.into_tokens()))
    }
}
