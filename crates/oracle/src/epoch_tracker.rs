use crate::Config;
use thiserror::Error;
use tracing::debug;

#[derive(Debug, Error)]
pub enum EpochTrackerError {
    #[error("Failed to determine current epoch.")]
    CantInferPreviousEpoch,
}

/// Tracks current Ethereum mainnet epoch.
pub struct EpochTracker {
    epoch_duration: u64,
}

impl EpochTracker {
    pub fn new(config: &Config) -> Self {
        Self {
            epoch_duration: config.epoch_duration,
        }
    }

    pub async fn is_new_epoch(&self, block_number: u64) -> Result<bool, EpochTrackerError> {
        let previous_block: Option<u64> =
            todo!("obtain the block number from the previous transaction");
        if let Some(block_number_of_last_tx) = previous_block {
            debug!(
                block_number = block_number,
                block_number_of_last_tx = block_number_of_last_tx,
                epoch_duration = self.epoch_duration,
                "Checking (possibly) new epoch."
            );
            Ok(block_number - block_number_of_last_tx >= self.epoch_duration)
        } else {
            Err(EpochTrackerError::CantInferPreviousEpoch)
        }
    }
}
