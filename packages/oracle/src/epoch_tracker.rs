use crate::{Config, Store};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EpochTrackerError {
    #[error("Store error: {0}")]
    Sqlx(#[from] sqlx::error::Error),
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
        if let Some(block_number_of_last_tx) = self.store.block_number_of_last_epoch().await? {
            Ok(block_number - block_number_of_last_tx >= self.epoch_duration)
        } else {
            Err(EpochTrackerError::PreviousEpochNotFound)
        }
    }
}
