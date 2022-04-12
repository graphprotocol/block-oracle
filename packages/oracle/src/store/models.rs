use sqlx::types::chrono::{self};

pub type Id = i64;
pub type EncodingVersion = u32;
pub type BlockNumber = u64;
pub type Timestamp = chrono::DateTime<chrono::Utc>;

pub struct Caip2ChainId {
    chain_id: String,
}

impl Caip2ChainId {
    const SEPARATOR: char = ':';

    pub fn parse(&self, chain_id: &str) -> Option<Self> {
        let split = self.chain_id.split(Self::SEPARATOR).collect::<Vec<&str>>();

        let is_ascii_alphanumberic_or_hyphen =
            |s: &str| s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');

        if split.len() == 2
            && split[0].len() >= 3
            && split[0].len() <= 8
            && is_ascii_alphanumberic_or_hyphen(split[0])
            && split[1].len() >= 1
            && split[1].len() <= 32
            && is_ascii_alphanumberic_or_hyphen(split[1])
        {
            Some(Self {
                chain_id: chain_id.to_string(),
            })
        } else {
            None
        }
    }

    pub fn as_str(&self) -> &str {
        &self.chain_id
    }

    pub fn namespace_part(&self) -> &str {
        self.chain_id.split_once(':').unwrap().0
    }

    pub fn reference_part(&self) -> &str {
        self.chain_id.split_once(':').unwrap().1
    }
}

pub struct Network {
    chain_id: Caip2ChainId,
}

pub struct DataEdgeCall {
    pub tx_hash: Vec<u8>,
    pub nonce: u64,
    pub num_confirmations: u64,
    pub num_confirmations_last_checked_at: Timestamp,
    pub block_number: BlockNumber,
    pub block_hash: Vec<u8>,
    pub payload: Vec<u8>,
}
