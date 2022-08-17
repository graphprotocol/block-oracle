use crate::{models::Caip2ChainId, subgraph::Network, Config};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct NetworksDiff {
    pub deletions: HashMap<Caip2ChainId, u64>,
    pub insertions: HashSet<Caip2ChainId>,
}

impl NetworksDiff {
    pub fn calculate(subgraph_networks: &[Network], config: &Config) -> Self {
        let new = config.indexed_chains.iter().map(|c| c.id.clone()).collect();
        Self::diff(subgraph_networks, new)
    }

    fn diff(old: &[Network], new: HashSet<Caip2ChainId>) -> Self {
        let mut deletions = HashMap::new();
        let mut deleted_indices = HashSet::new();
        for network in old.iter() {
            if !new.contains(&network.id) {
                deletions.insert(network.id.clone(), network.array_index);
                deleted_indices.insert(network.array_index);
            }
        }

        let old_network_names: HashSet<_> =
            old.iter().map(|network| &network.id).cloned().collect();
        let insertions = new.difference(&old_network_names).cloned().collect();

        Self {
            deletions,
            insertions,
        }
    }
}
