use crate::Config;
use thiserror::Error;
use tracing::{debug, error};

#[derive(Debug, Error)]
pub enum EpochTrackerError {
    #[error("Failed to determine current epoch")]
    PreviousEpochNotFound,
}

impl crate::MainLoopFlow for EpochTrackerError {
    fn instruction(&self) -> crate::OracleControlFlow {
        use std::ops::ControlFlow::*;
        use EpochTrackerError::*;
        match self {
            error @ PreviousEpochNotFound => {
                error!("{error}");
                Continue(None)
            }
        }
    }
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
        // FIXME
        if let Some(block_number_of_last_tx) = Some(0) {
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
