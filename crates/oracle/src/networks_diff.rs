use crate::{models::Caip2ChainId, Config};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct NetworksDiff {
    pub deletions: HashMap<Caip2ChainId, u32>,
    pub insertions: HashMap<Caip2ChainId, u32>,
}

impl NetworksDiff {
    pub fn calculate(subgraph_networks: HashMap<Caip2ChainId, u32>, config: &Config) -> Self {
        let new = config
            .indexed_chains
            .iter()
            .map(|c| c.id().clone())
            .collect();
        Self::diff(subgraph_networks, new)
    }

    fn diff(old: HashMap<Caip2ChainId, u32>, new: Vec<Caip2ChainId>) -> Self {
        // Turn `new` into a `HashSet` to easily check for the presence of
        // items.
        let new: HashSet<Caip2ChainId> = new.into_iter().collect();

        let mut deletions = HashMap::new();
        let mut deleted_ids = HashSet::new();
        for (network_name, id) in old.iter() {
            if !new.contains(network_name) {
                deletions.insert(network_name.clone(), *id);
                deleted_ids.insert(*id);
            }
        }

        let mut insertions = HashMap::new();
        for network_name in new.into_iter() {
            if !old.contains_key(&network_name) {
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
                    || !old.iter().any(|(_, id)| *id == next_candidate_id);

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
