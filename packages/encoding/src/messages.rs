use crate::NetworkId;
use std::collections::HashMap;

pub type Bytes32 = [u8; 32];

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BlockPtr {
    pub number: u64,
    pub hash: Bytes32,
}

#[derive(Debug, Clone)]
pub enum Message {
    // TODO: Consider specifying epoch number here?
    SetBlockNumbersForNextEpoch(HashMap<String, BlockPtr>),
    RegisterNetworks {
        // Remove is by index
        remove: Vec<NetworkId>,
        // Add is by name
        add: Vec<String>,
    },
    // TODO: include hash, count, and (if count is nonzero) merkle root
    CorrectEpochs,
    UpdateVersion,
    Reset,
}

#[derive(Debug)]
pub enum CompressedMessage {
    SetBlockNumbersForNextEpoch(CompressedSetBlockNumbersForNextEpoch),
    CorrectEpochs,
    RegisterNetworks { remove: Vec<u64>, add: Vec<String> },
    UpdateVersion,
    Reset,
}

#[derive(Debug)]
pub enum CompressedSetBlockNumbersForNextEpoch {
    Empty {
        count: u64,
    },
    NonEmpty {
        accelerations: Vec<i64>,
        root: Bytes32,
    },
}
