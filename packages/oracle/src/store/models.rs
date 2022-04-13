use sqlx::types::chrono;

pub type Id = u64;
pub type EncodingVersion = u64;
pub type BlockNumber = u64;
pub type Timestamp = chrono::DateTime<chrono::Utc>;
pub type Nonce = u64;

pub struct WithId<T, I = Id> {
    pub id: I,
    pub data: T,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Caip2ChainId {
    chain_id: String,
}

impl Caip2ChainId {
    const SEPARATOR: char = ':';

    pub fn parse(chain_id: &str) -> Option<Self> {
        let split = chain_id.split(Self::SEPARATOR).collect::<Vec<&str>>();

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

    pub fn into_string(self) -> String {
        self.chain_id
    }

    pub fn as_str(&self) -> &str {
        &self.chain_id
    }

    pub fn namespace_part(&self) -> &str {
        self.chain_id.split_once(Self::SEPARATOR).unwrap().0
    }

    pub fn reference_part(&self) -> &str {
        self.chain_id.split_once(Self::SEPARATOR).unwrap().1
    }
}

pub struct Network {
    pub name: Caip2ChainId,
    pub latest_block_number: Option<u64>,
    pub latest_block_hash: Option<Vec<u8>>,
    pub latest_block_delta: Option<i64>,
    pub introduced_with: Id,
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
