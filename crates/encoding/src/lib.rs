mod encoding;
mod merkle;
pub mod messages;

use std::collections::HashMap;

use merkle::{merkle_root, MerkleLeaf};
use messages::*;

pub use encoding::encode_messages;
pub use messages::{BlockPtr, CompressedMessage, Message};

pub type NetworkId = u64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Network {
    pub block_number: u64,
    pub block_delta: i64,
}

#[derive(Debug)]
#[non_exhaustive]
pub struct CompressionEngine {
    network_data: Vec<(String, Network)>,
    network_ids_by_name: HashMap<String, NetworkId>,

    pub network_data_updates: HashMap<NetworkId, Network>,
    pub compressed: Vec<CompressedMessage>,
}

impl CompressionEngine {
    pub fn new(available_networks: Vec<(String, Network)>) -> Self {
        let mut network_ids_by_name = HashMap::with_capacity(available_networks.len());
        let mut network_data = Vec::with_capacity(available_networks.len());

        for (i, (name, network)) in available_networks.into_iter().enumerate() {
            network_ids_by_name.insert(name.clone(), i as NetworkId);
            network_data.push((name, network));
        }

        Self {
            network_data,
            network_ids_by_name,
            network_data_updates: HashMap::new(),
            compressed: Vec::new(),
        }
    }

    pub fn compress_messages(&mut self, messages: &[Message]) {
        for message in messages {
            self.compress_message(message);
        }
    }

    fn compress_message(&mut self, message: &Message) {
        match message {
            Message::SetBlockNumbersForNextEpoch(block_ptrs) => {
                // There are separate cases for empty sets and non-empty sets.
                if block_ptrs.is_empty() {
                    self.compress_empty_block_ptrs();
                } else {
                    self.compress_block_ptrs(block_ptrs);
                }
            }
            Message::RegisterNetworks { remove, add } => {
                // TODO: removals.
                for added in add {
                    self.network_data.push((
                        added.clone(),
                        Network {
                            block_delta: 0,
                            block_number: 0,
                        },
                    ));
                    self.network_ids_by_name
                        .insert(added.clone(), self.network_data.len() as NetworkId - 1);
                }

                self.compressed.push(CompressedMessage::RegisterNetworks {
                    remove: remove.clone(),
                    add: add.clone(),
                });
            }
            Message::CorrectEpochs { data_by_network_id } => {
                self.compressed.push(CompressedMessage::CorrectEpochs {
                    data_by_network_id: data_by_network_id.clone(),
                });
            }
            Message::UpdateVersion { version_number } => {
                self.compressed.push(CompressedMessage::UpdateVersion {
                    version_number: *version_number,
                });
            }
            Message::Reset => {
                self.compressed.push(CompressedMessage::Reset);
            }
        }
    }

    fn compress_block_ptrs(&mut self, block_ptrs: &HashMap<String, BlockPtr>) {
        // Sort the block pointers by network id.
        let block_ptrs_by_id: HashMap<NetworkId, BlockPtr> = block_ptrs
            .iter()
            .map(|(network_name, block_ptr)| {
                let network_id = self.network_ids_by_name[network_name];
                (network_id, *block_ptr)
            })
            .collect();

        // Get accelerations and merkle leaves based on previous deltas.
        let mut accelerations = Vec::with_capacity(block_ptrs.len());
        let mut merkle_leaves = Vec::with_capacity(block_ptrs.len());
        for (id, ptr) in block_ptrs_by_id.into_iter() {
            let network_data = &self.network_data[id as usize].1;
            let delta = (ptr.number - network_data.block_number) as i64;
            let acceleration = delta - network_data.block_delta;

            let new_network_data = Network {
                block_number: ptr.number,
                block_delta: delta,
            };
            self.network_data_updates.insert(id, new_network_data);

            accelerations.push(acceleration);
            merkle_leaves.push(MerkleLeaf {
                network_id: id,
                block_hash: ptr.hash,
                block_number: ptr.number,
            });
        }

        let root = merkle_root(&merkle_leaves);

        self.compressed
            .push(CompressedMessage::SetBlockNumbersForNextEpoch(
                CompressedSetBlockNumbersForNextEpoch::NonEmpty {
                    accelerations,
                    root,
                },
            ));
    }

    fn compress_empty_block_ptrs(&mut self) {
        // If we have an empty set we may need to extend the last message.
        if let Some(CompressedMessage::SetBlockNumbersForNextEpoch(
            CompressedSetBlockNumbersForNextEpoch::Empty { count },
        )) = self.compressed.last_mut()
        {
            *count += 1
        } else {
            self.compressed
                .push(CompressedMessage::SetBlockNumbersForNextEpoch(
                    CompressedSetBlockNumbersForNextEpoch::Empty { count: 1 },
                ));
        }
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::messages::{BlockPtr, Message},
        never::Never,
        std::collections::HashMap,
        tokio::test,
    };

    #[test]
    async fn pipeline() {
        let mut messages = Vec::new();

        // Skip some empty epochs
        for _ in 0..20 {
            messages.push(Message::SetBlockNumbersForNextEpoch(HashMap::new()));
        }

        let networks: Vec<_> = ["A:1991", "B:2kl", "C:190", "D:18818"]
            .iter()
            .map(|i| i.to_string())
            .map(|s| {
                (
                    s,
                    Network {
                        block_number: 0,
                        block_delta: 0,
                    },
                )
            })
            .collect();

        // Add blocks
        for i in 0..4 {
            let nums = networks
                .iter()
                .enumerate()
                .map(|(network_i, (name, _network))| {
                    (
                        name.to_string(),
                        BlockPtr {
                            number: 300 * (i + (network_i as u64)) + i,
                            hash: [1; 32],
                        },
                    )
                })
                .collect();
            messages.push(Message::SetBlockNumbersForNextEpoch(nums));
        }

        let mut engine = CompressionEngine::new(networks);
        engine.compress_messages(&messages[..]);

        assert!(matches!(
            engine.compressed[0],
            CompressedMessage::SetBlockNumbersForNextEpoch(
                CompressedSetBlockNumbersForNextEpoch::Empty { count: 20 }
            )
        ));
        assert!(matches!(
            engine.compressed.last().unwrap(),
            CompressedMessage::SetBlockNumbersForNextEpoch(
                CompressedSetBlockNumbersForNextEpoch::NonEmpty { .. }
            )
        ));

        // TODO: Add ability to skip epochs? Right now the way to get past this is to
        // just add 80 or so SetBlockNumbers.
    }
}
