use crate::{Config, Store};
use thiserror::Error;
use tracing::debug;

#[derive(Debug, Error)]
pub enum EpochTrackerError {
    #[error("Store error")]
    StoreError,
    #[error("Failed to determine current epoch. No previous epoch was found in local storage.")]
    PreviousEpochNotFound,
}

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

    pub async fn is_new_epoch(&self, block_number: u64) -> Result<bool, EpochTrackerError> {
        if let Some(block_number_of_last_tx) = self.store.block_number_of_last_epoch().await {
            debug!(
                block_number = block_number,
                block_number_of_last_tx = block_number_of_last_tx,
                epoch_duration = self.epoch_duration,
                "Checking (possibly) new epoch."
            );
            Ok(block_number - block_number_of_last_tx >= self.epoch_duration)
        } else {
            Err(EpochTrackerError::PreviousEpochNotFound)
        }
    }
}
