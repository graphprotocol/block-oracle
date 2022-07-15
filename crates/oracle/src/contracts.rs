use anyhow::Context;
use web3::{api::Eth, contract::Contract, ethabi::Address, Transport};

use crate::{Config, JrpcProviderForChain};

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
}
