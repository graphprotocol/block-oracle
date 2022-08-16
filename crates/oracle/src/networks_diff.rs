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
        let mut deleted_ids = HashSet::new();
        for network in old.iter() {
            if !new.contains(&network.id) {
                deletions.insert(network.id.clone(), network.array_index);
                deleted_ids.insert(network.array_index);
            }
        }

        let mut insertions = HashMap::new();
        let old_network_names: HashSet<_> = old.iter().map(|network| &network.id).collect();
        for network_name in new.into_iter() {
            if !old_network_names.contains(&network_name) {
                // We use 0 as a temporary ID.
                insertions.insert(network_name, 0);
            }
        }

        // Now we can assign IDs to the newly-inserted networks. We want to
        // recycle IDs as much as possible, so the lowest nonnegative number
        // that's not currently assigned must be used.
        let mut next_candidate_id = 0;
        for (_, id) in insertions.iter_mut() {
            loop {
                let id_is_free_to_use = deleted_ids.contains(&next_candidate_id)
                    || !old
                        .iter()
                        .any(|network| network.array_index == next_candidate_id);

                if id_is_free_to_use {
                    *id = next_candidate_id;
                    next_candidate_id += 1;
                    break;
                }
            }
        }

        Self {
            deletions,
            insertions,
        }
    }
}
