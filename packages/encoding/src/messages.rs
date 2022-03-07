use crate::*;
use std::collections::HashMap;

pub enum Message {
    SetBlockNumbersForNextEpoch(HashMap<String, BlockPtr>),
    CorrectEpochs,
    // TODO: Register Networks should have BlockHash for latest epoch, chainId, networkId and prev N block deltas
    // TODO: RegisterNetworks should be able to remove networks
    RegisterNetworks,
    UpdateVersion,
}

pub enum CompressedMessage {
    SetBlockNumbersForNextEpoch {
        accelerations: Vec<i64>,
        root: Bytes32,
    },
    CorrectEpochs,
    RegisterNetworks,
    UpdateVersion,
}
