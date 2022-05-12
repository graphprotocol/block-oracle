use super::models;
use models::{Caip2ChainId, WithId};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Store {
    // nothing here
}

impl Store {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn last_encoding_version(&self) -> Option<models::Id> {
        todo!()
    }

    pub async fn insert_encoding_version(
        &self,
        version: models::Id,
        data_edge_call_id: models::Id,
    ) -> models::Id {
        todo!()
    }

    pub async fn sync_networks(&self, networks: Vec<Caip2ChainId>) {
        let networks: HashSet<Caip2ChainId> = networks.into_iter().collect();
        let current_networks: HashMap<Caip2ChainId, models::Id> = self
            .networks()
            .await
            .into_iter()
            .map(|n| (n.data.name, n.id))
            .collect();
        let networks_to_delete: Vec<Caip2ChainId> = current_networks
            .iter()
            .filter(|n| !networks.contains(&n.0))
            .map(|x| x.0.clone())
            .collect();
        let networks_to_insert: Vec<Caip2ChainId> = networks
            .iter()
            .filter(|chain_id| !current_networks.contains_key(*chain_id))
            .cloned()
            .collect();
    }

    pub async fn delete_network(&self, network_id: models::Id) {
        todo!()
    }

    pub async fn insert_network(&self, network: WithId<models::Network>) -> models::Id {
        todo!()
    }

    pub async fn update_network_block_info_by_id(&self, id: models::Id, latest_block_number: u64) {
        todo!()
    }

    pub async fn update_network_block_info(
        &self,
        caip2: &Caip2ChainId,
        latest_block_number: u64,
    ) -> () {
        todo!()
    }

    pub async fn network_by_id(&self, id: models::Id) -> Option<WithId<models::Network>> {
        todo!()
    }

    pub async fn networks(&self) -> Vec<WithId<models::Network>> {
        todo!()
    }

    pub async fn insert_data_edge_call(&self, call: models::DataEdgeCall) -> models::Id {
        todo!()
    }

    pub async fn write_num_confirmations(&self, call_id: models::Id, num_confirmations: u64) {
        todo!()
    }

    pub async fn block_number_of_last_epoch(&self) -> Option<u64> {
        todo!()
    }

    pub async fn last_nonce(&self) -> Option<models::Nonce> {
        todo!()
    }

    pub async fn next_nonce(&self) -> models::Nonce {
        todo!()
    }
}
