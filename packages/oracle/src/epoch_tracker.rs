use crate::{Config, Store};

/// Tracks current Ethereum mainnet epoch.
pub struct EpochTracker {
    store: Store,
    epoch_duration: u64,
}

impl EpochTracker {
    pub fn new(store: &Store, config: &Config) -> Self {
        Self {
            store: store.clone(),
            epoch_duration: config.epoch_duration,
        }
    }

    pub async fn is_new_epoch(&self, block_number: u64) -> sqlx::Result<bool> {
        let block_number_of_last_tx = self.store.block_number_of_last_tx().await?;
        Ok(block_number - block_number_of_last_tx >= self.epoch_duration)
    }
}
