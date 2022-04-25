use crate::{store::Caip2ChainId, Config, Error, Store};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct NetworksDiff {
    pub deletions: Vec<Caip2ChainId>,
    pub insertions: HashMap<Caip2ChainId, u32>,
}

impl NetworksDiff {
    pub async fn calculate(store: &Store, config: &Config) -> Result<Self, Error> {
        let old = store
            .networks()
            .await?
            .into_iter()
            .map(|n| (n.data.name, n.id))
            .collect();
        let new = config.networks();

        Ok(Self::diff(old, new))
    }

    fn diff(old: HashMap<Caip2ChainId, u32>, new: Vec<Caip2ChainId>) -> Self {
        // Turn `new` into a `HashSet` to easily check for the presence of
        // items.
        let new: HashSet<Caip2ChainId> = new.into_iter().collect();

        // Removes are processed first. In this way we can re-use IDs.
        let mut deletions = vec![];
        let mut deleted_ids = HashSet::new();
        for (network_name, id) in old.iter() {
            if !new.contains(network_name) {
                deletions.push(network_name.clone());
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
