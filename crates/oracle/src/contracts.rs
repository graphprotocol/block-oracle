use crate::{
    config::TransactionMonitoringOptions,
    metrics::METRICS,
    transaction_monitor::{TransactionMonitor, TransactionMonitorError},
};
use anyhow::Context;
use secp256k1::SecretKey;
use tracing::{debug, info, trace};
use web3::{
    api::Eth,
    contract::{tokens::Tokenize, Contract},
    ethabi::Address,
    signing::SecretKeyRef,
    types::{TransactionReceipt, U256},
    Transport, Web3,
};

static EPOCH_MANAGER_ABI: &[u8] = include_bytes!("abi/EpochManager.json");
static EPOCH_MANAGER_FUNCTION_NAME: &str = "currentEpoch";
static DATA_EDGE_ABI: &[u8] = include_bytes!("abi/DataEdge.json");
static DATA_EDGE_FUNCTION_NAME: &str = "crossChainEpochOracle";

#[derive(thiserror::Error, Debug)]
pub enum ContractError {
    #[error(transparent)]
    TransactionMonitor(#[from] TransactionMonitorError),
    #[error(transparent)]
    AbiEncode(#[from] web3::ethabi::Error),
}

pub struct Contracts<T>
where
    T: Clone + Transport,
{
    client: Web3<T>,
    data_edge: Contract<T>,
    epoch_manager: Contract<T>,
    transaction_monitoring_options: TransactionMonitoringOptions,
}

impl<T> Contracts<T>
where
    T: Clone + Transport,
{
    pub fn new(
        client: Web3<T>,
        data_edge_address: Address,
        epoch_manager_address: Address,
        transaction_monitoring_options: TransactionMonitoringOptions,
    ) -> anyhow::Result<Self> {
        let data_edge = Contracts::new_contract(DATA_EDGE_ABI, &client.eth(), data_edge_address)?;
        let epoch_manager =
            Contracts::new_contract(EPOCH_MANAGER_ABI, &client.eth(), epoch_manager_address)?;
        Ok(Self {
            client,
            data_edge,
            epoch_manager,
            transaction_monitoring_options,
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
    ) -> Result<TransactionReceipt, ContractError> {
        info!("Sending transaction to DataEdge");

        let transaction_receipt: TransactionReceipt = {
            let calldata: web3::types::Bytes =
                self.abi_encode_data_edge_payload((payload,))?.into();

            let transaction_monitor = TransactionMonitor::new(
                self.client.clone(),
                SecretKeyRef::new(owner_private_key),
                self.data_edge.address(),
                calldata,
                self.transaction_monitoring_options,
            )
            .await?;
            transaction_monitor.execute_transaction().await?
        };

        info!(?transaction_receipt.transaction_hash, "Transaction confirmed");
        Ok(transaction_receipt)
    }

    fn abi_encode_data_edge_payload(
        &self,
        params: impl Tokenize,
    ) -> Result<Vec<u8>, web3::ethabi::Error> {
        self.data_edge
            .abi()
            .function(DATA_EDGE_FUNCTION_NAME)
            .and_then(|function| function.encode_input(&params.into_tokens()))
    }
}
