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

    pub async fn networks(&self) -> Vec<WithId<models::Network>> {
        todo!()
    }

    pub async fn block_number_of_last_epoch(&self) -> Option<u64> {
        todo!()
    }

    pub async fn next_nonce(&self) -> models::Nonce {
        todo!()
    }
}
