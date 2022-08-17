use crate::{models::Caip2ChainId, subgraph::Network, Config};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct NetworksDiff {
    pub deletions: HashMap<Caip2ChainId, u64>,
    pub insertions: HashMap<Caip2ChainId, u64>,
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

        let mut insertions = HashMap::new();
        let old_network_names: HashSet<_> = old.iter().map(|network| &network.id).collect();
        for network_name in new.into_iter() {
            if !old_network_names.contains(&network_name) {
                // We use 0 as a temporary index.
                insertions.insert(network_name, 0);
            }
        }

        // Now we can assign indices to the newly-inserted networks. We want to
        // recycle indices as much as possible, so the lowest nonnegative number
        // that's not currently assigned must be used.
        let mut next_candidate_index = 0;
        for (_, index) in insertions.iter_mut() {
            loop {
                let index_is_free_to_use = deleted_indices.contains(&next_candidate_index)
                    || !old
                        .iter()
                        .any(|network| network.array_index == next_candidate_index);

                if index_is_free_to_use {
                    *index = next_candidate_index;
                    break;
                }
                next_candidate_index += 1;
            }
        }

        Self {
            deletions,
            insertions,
        }
    }
}
