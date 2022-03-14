use crate::*;
use std::collections::HashMap;

#[derive(Debug)]
pub struct BlockPtr {
    pub number: u64,
    pub hash: Bytes32,
}

#[derive(Debug)]
pub struct Transaction {
    pub nonce: u64,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub enum Message {
    // TODO: Consider specifying epoch number here?
    SetBlockNumbersForNextEpoch(HashMap<String, BlockPtr>),
    // TODO: include hash, count, and (if count is nonzero) merkle root
    CorrectEpochs,
    UpdateVersion,
}

#[derive(Debug)]
pub enum CompressedMessage {
    SetBlockNumbersForNextEpoch {
        accelerations: Vec<i64>,
        root: Option<Bytes32>,
    },
    CorrectEpochs,
    RegisterNetworks {
        // Remove is by index
        remove: Vec<u64>,
        // Add is by name
        add: Vec<String>,
    },
    UpdateVersion,
}
