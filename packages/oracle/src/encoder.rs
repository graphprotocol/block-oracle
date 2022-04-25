use super::store::Network;
use epoch_encoding::{CompressedMessage, Message};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CompressionState {
    pub latest_block_number: u64,
    pub last_emitted_block_number: u64,
    pub block_delta: u64,
}

impl CompressionState {
    pub fn delta(&self) -> u64 {
        self.latest_block_number
            .checked_sub(self.last_emitted_block_number)
            .unwrap()
    }

    pub fn acceleration(&self) -> i64 {
        self.delta() as i64 - self.block_delta as i64
    }
}

/// Encodes and compresses messages.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Encoder {
    networks: Vec<Network>,
}

impl Encoder {
    pub fn new(networks: Vec<Network>) -> Self {
        Self { networks }
    }

    pub fn encode_message(
        &self,
        message: &Message,
        compressed: &mut Vec<CompressedMessage>,
    ) -> CompressedMessage {
        todo!()
    }
}
