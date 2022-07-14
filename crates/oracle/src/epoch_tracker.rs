use crate::Config;
use thiserror::Error;
use tracing::{debug, error};

#[derive(Debug, Error)]
pub enum EpochTrackerError {
    #[error("Failed to determine current epoch")]
    PreviousEpochNotFound,
    #[error("Previous epoch block number ({0}) is higher than chain head ({1})")]
    ImpossibleLatestBlock(u64, u64),
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
            error @ ImpossibleLatestBlock(..) => {
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

    pub async fn is_new_epoch(
        &self,
        latest_block_number: u64,
        previous_epoch_block_number: u64,
    ) -> Result<bool, EpochTrackerError> {
        debug!(
            latest_block_number,
            previous_epoch_block_number,
            epoch_duration = self.epoch_duration,
            "Checking (possibly) new epoch."
        );
        let block_distance = previous_epoch_block_number
            .checked_sub(latest_block_number)
            .ok_or(EpochTrackerError::ImpossibleLatestBlock(
                previous_epoch_block_number,
                latest_block_number,
            ))?;
        Ok(block_distance >= self.epoch_duration)
    }
}
