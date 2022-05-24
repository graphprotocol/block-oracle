mod merkle;
pub mod messages;
mod serialize;

use merkle::{merkle_root, MerkleLeaf};
use messages::*;
use std::collections::HashMap;

pub use messages::{BlockPtr, CompressedMessage, Message};
pub use serialize::serialize_messages;

pub const CURRENT_ENCODING_VERSION: u64 = 0;

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Network {
    pub block_number: u64,
    pub block_delta: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Encoder {
    networks: Vec<(String, Network)>,
    encoding_version: u64,
    compressed: Vec<CompressedMessage>,
}

impl Encoder {
    pub fn new(encoding_version: u64, networks: Vec<(String, Network)>) -> Self {
        Self {
            encoding_version,
            networks,
            compressed: Vec::new(),
        }
    }

    /// Returns the latest encoding version used by this [`Encoder`].
    pub fn encoding_version(&self) -> u64 {
        self.encoding_version
    }

    /// Encoding is a stateful operation. After this call, the [`Encoder`] is
    /// ready to be used again and some of its internal state might have
    /// changed.
    pub fn encode(&mut self, messages: &[Message]) -> Vec<u8> {
        for m in messages {
            self.compress(m);
        }
        self.serialize()
    }

    fn serialize(&mut self) -> Vec<u8> {
        let mut bytes = vec![];
        serialize_messages(&self.compressed, &mut bytes);
        self.compressed.clear();
        bytes
    }

    fn compress(&mut self, message: &Message) {
        // After updating the encoding version, no more messages can be encoded
        // in the same batch.
        assert!(!matches!(
            self.compressed.last(),
            Some(CompressedMessage::UpdateVersion { .. })
        ));

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
                for id in remove {
                    self.remove_network(*id);
                }
                for name in add {
                    self.add_network(name);
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
                self.encoding_version = *version_number;
                self.compressed.push(CompressedMessage::UpdateVersion {
                    version_number: *version_number,
                });
            }
            Message::Reset => {
                self.networks.clear();
                self.compressed.push(CompressedMessage::Reset);
            }
        }
    }

    fn add_network(&mut self, name: &str) {
        self.networks.push((name.to_string(), Network::default()));
    }

    fn remove_network(&mut self, i: NetworkId) {
        self.networks.swap_remove(i as usize);
    }

    fn compress_block_ptrs(&mut self, block_ptrs: &HashMap<String, BlockPtr>) {
        // Sort the block pointers by network id.
        let block_ptrs_by_id: HashMap<NetworkId, BlockPtr> = block_ptrs
            .iter()
            .map(|(network_name, block_ptr)| {
                let network_id = self
                    .networks
                    .iter()
                    .position(|(s, _)| s == network_name)
                    .expect(format!("Network named '{}' not found", network_name).as_str());
                (network_id as NetworkId, *block_ptr)
            })
            .collect();

        // Get accelerations and merkle leaves based on previous deltas.
        let mut accelerations = Vec::with_capacity(block_ptrs.len());
        let mut merkle_leaves = Vec::with_capacity(block_ptrs.len());
        for (id, ptr) in block_ptrs_by_id.into_iter() {
            let network_data = &self.networks[id as usize].1;
            let delta = (ptr.number - network_data.block_number) as i64;
            let acceleration = delta - network_data.block_delta;

            self.networks[id as usize].1 = Network {
                block_number: ptr.number,
                block_delta: delta,
            };

            accelerations.push(acceleration);
            merkle_leaves.push(MerkleLeaf {
                network_id: id,
                block_hash: ptr.hash,
                block_number: ptr.number,
            });
        }

        self.compressed
            .push(CompressedMessage::SetBlockNumbersForNextEpoch(
                CompressedSetBlockNumbersForNextEpoch::NonEmpty {
                    accelerations,
                    root: merkle_root(&merkle_leaves),
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

        let mut engine = Encoder::new(0, networks);
        engine.encode(&messages[..]);

        // FIXME
        //assert!(matches!(
        //    engine.compressed[0],
        //    CompressedMessage::SetBlockNumbersForNextEpoch(
        //        CompressedSetBlockNumbersForNextEpoch::Empty { count: 20 }
        //    )
        //));
        //assert!(matches!(
        //    engine.compressed.last().unwrap(),
        //    CompressedMessage::SetBlockNumbersForNextEpoch(
        //        CompressedSetBlockNumbersForNextEpoch::NonEmpty { .. }
        //    )
        //));

        // TODO: Add ability to skip epochs? Right now the way to get past this is to
        // just add 80 or so SetBlockNumbers.
    }
}
