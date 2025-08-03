mod merkle;
pub mod messages;
mod serialize;

use merkle::{merkle_root, MerkleLeaf};
use messages::*;
use std::collections::BTreeMap;

pub use messages::{BlockPtr, CompressedMessage, CompressedSetBlockNumbersForNextEpoch, Message};
pub use serialize::serialize_messages;

pub const CURRENT_ENCODING_VERSION: u64 = 0;

/// Something that went wrong when using the [`Encoder`].
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Unsuported encoding version: {0}")]
    UnsupportedEncodingVersion(u64),
    #[error(
        "After updating the encoding version, no more messages can be encoded in the same batch"
    )]
    MessageAfterEncodingVersionChange,
    #[error("Invalid Network ID: {0}")]
    InvalidNetworkId(String),
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Network {
    pub block_number: u64,
    pub block_delta: i64,
    pub array_index: u64,
}

impl Network {
    pub fn new(block_number: u64, block_delta: i64, array_index: u64) -> Self {
        Self {
            block_number,
            block_delta,
            array_index,
        }
    }
}

/// The [`Encoder`]'s job is to take in sequences of high-level [`Message`]s, compress them,
/// perform validation, and spit out bytes.
///
/// # Panics
///
/// The [`Encoder`] should never panic on malformed [`Message`]s, but rather return an [`Error`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Encoder {
    networks: Vec<(String, Network)>,
    encoding_version: u64,
    compressed: Vec<CompressedMessage>,
}

impl Encoder {
    /// Creates a new [`Encoder`] with the specificied initial state.
    pub fn new(encoding_version: u64, networks: Vec<(String, Network)>) -> Result<Self, Error> {
        if encoding_version != CURRENT_ENCODING_VERSION {
            return Err(Error::UnsupportedEncodingVersion(encoding_version));
        }

        Ok(Self {
            encoding_version,
            networks,
            compressed: Vec::new(),
        })
    }

    pub fn network_deltas(&self) -> &[(String, Network)] {
        &self.networks
    }

    /// Gets the network's index from the ID, if the network exists.
    pub fn network_index(&self, network_id: &str) -> Option<NetworkIndex> {
        self.networks
            .iter()
            .enumerate()
            .find(|(_, (id, _))| id == network_id)
            .map(|(i, _)| i as NetworkIndex)
    }

    /// Returns the latest encoding version used by this [`Encoder`].
    pub fn encoding_version(&self) -> u64 {
        self.encoding_version
    }

    /// Compression is a stateful operation. After this call, the [`Encoder`] is
    /// ready to be used again and some of its internal state might have
    /// changed.
    pub fn compress(&mut self, messages: &[Message]) -> Result<Vec<CompressedMessage>, Error> {
        for m in messages {
            self.compress_message(m)?;
        }
        Ok(std::mem::take(&mut self.compressed))
    }

    pub fn encode(&self, compressed: &[CompressedMessage]) -> Vec<u8> {
        let mut bytes = vec![];
        serialize_messages(compressed, &mut bytes);
        bytes
    }

    fn compress_message(&mut self, message: &Message) -> Result<(), Error> {
        // After updating the encoding version, no more messages can be encoded
        // in the same batch.
        if let Some(CompressedMessage::UpdateVersion { .. }) = self.compressed.last() {
            return Err(Error::MessageAfterEncodingVersionChange);
        }

        match message {
            Message::SetBlockNumbersForNextEpoch(block_ptrs) => {
                // There are separate cases for empty sets and non-empty sets.
                if block_ptrs.is_empty() {
                    self.compress_empty_block_ptrs();
                } else {
                    self.compress_block_ptrs(block_ptrs.clone())?;
                }
            }
            Message::RegisterNetworks { remove, add } => {
                for index in remove {
                    self.remove_network(*index);
                }
                for id in add {
                    self.add_network(id);
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
                if *version_number != CURRENT_ENCODING_VERSION {
                    return Err(Error::UnsupportedEncodingVersion(*version_number));
                }

                self.encoding_version = *version_number;
                self.compressed.push(CompressedMessage::UpdateVersion {
                    version_number: *version_number,
                });
            }
            Message::Reset => {
                self.networks.clear();
                self.compressed.push(CompressedMessage::Reset);
            }
            Message::RegisterNetworksAndAliases { remove, add } => {
                for index in remove {
                    self.remove_network(*index);
                }
                for (id, _) in add {
                    self.add_network(id);
                }

                self.compressed
                    .push(CompressedMessage::RegisterNetworksAndAliases {
                        remove: remove.clone(),
                        add: add.clone(),
                    });
            }
            Message::ChangePermissions {
                address,
                valid_through,
                permissions,
            } => {
                self.compressed.push(CompressedMessage::ChangePermissions {
                    address: *address,
                    valid_through: *valid_through,
                    permissions: permissions
                        .iter()
                        .map(|x| Message::str_to_u64(x.as_str()))
                        .collect(),
                });
            }
            Message::CorrectLastEpoch {
                network_id,
                block_number,
                merkle_root,
            } => {
                self.compressed.push(CompressedMessage::CorrectLastEpoch {
                    network_id: *network_id,
                    block_number: *block_number,
                    merkle_root: *merkle_root,
                });
            }
        };
        Ok(())
    }

    fn add_network(&mut self, id: &str) {
        self.networks.push((id.to_string(), Network::default()));
    }

    /// Remove a network from [`Encoder.networks`].
    ///
    /// Removal occurs by position, based on the `array_index` field of the target element.
    fn remove_network(&mut self, network_index: NetworkIndex) {
        let position = self
            .networks
            .iter()
            .position(|(_, network)| network.array_index == network_index)
            .unwrap_or_else(|| {
                panic!("Failed to find the a network with array_index equal to {network_index}")
            });
        self.networks.remove(position);
    }

    /// Takes in some network data by network ID and turns it into a [`Vec`] with the correct
    /// network indices.
    fn sort_network_data_by_index<T>(
        &self,
        chain_data: &BTreeMap<String, T>,
    ) -> Result<Vec<T>, Error>
    where
        T: Clone,
    {
        let mut sorted: Vec<(NetworkIndex, T)> = chain_data
            .iter()
            .map(|(id, data)| {
                Ok((
                    self.network_index(id)
                        .ok_or_else(|| Error::InvalidNetworkId(id.to_string()))?,
                    data.clone(),
                ))
            })
            .collect::<Result<Vec<(NetworkIndex, T)>, Error>>()?;
        // Sort by network index.
        sorted.sort_by(|(i, _), (j, _)| i.cmp(j));
        // Now remove the network index, which is implied by element positioning within the vector.
        Ok(sorted.into_iter().map(|(_, x)| x).collect())
    }

    fn compress_block_ptrs(
        &mut self,
        mut block_ptrs: BTreeMap<String, BlockPtr>,
    ) -> Result<(), Error> {
        for network in &self.networks {
            if !block_ptrs.contains_key(&network.0) {
                block_ptrs.insert(
                    network.0.clone(),
                    BlockPtr::new(network.1.block_number, [0; 32]),
                );
            }
        }

        // Prepare to get accelerations and merkle leaves based on previous deltas.
        let mut accelerations = Vec::with_capacity(block_ptrs.len());
        let mut merkle_leaves = Vec::with_capacity(block_ptrs.len());

        // Sort the block pointers by network index.
        let sorted_block_ptrs = self.sort_network_data_by_index(&block_ptrs)?;

        for (i, ptr) in sorted_block_ptrs.into_iter().enumerate() {
            let network_data = &self.networks[i].1;

            let delta = ptr.number as i64 - network_data.block_number as i64;
            let acceleration = delta - network_data.block_delta;

            let current_network = &mut self.networks[i].1;
            current_network.block_number = ptr.number;
            current_network.block_delta = delta;

            accelerations.push(acceleration);
            merkle_leaves.push(MerkleLeaf {
                network_index: i as NetworkIndex,
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

        Ok(())
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
    };

    #[test]
    fn include_skipped_blocks() {
        let networks = vec![
            ("A:1".to_string(), Network::new(0, 0, 0)),
            ("B:2".to_string(), Network::new(100, 0, 1)),
        ];
        let mut encoder = Encoder::new(CURRENT_ENCODING_VERSION, networks).unwrap();
        let block_updates = vec![("A:1".to_string(), BlockPtr::new(1, [0; 32]))];
        let compressed = encoder
            .compress(&[Message::SetBlockNumbersForNextEpoch(
                block_updates.into_iter().collect(),
            )])
            .unwrap();

        let accelerations = compressed
            .last()
            .unwrap()
            .as_non_empty_block_numbers()
            .unwrap()
            .0;
        assert_eq!(accelerations, [1, 0]);
    }

    #[test]
    fn block_numbers_increase() {
        let networks = vec![
            ("A:1".to_string(), Network::new(0, 0, 0)),
            ("B:2".to_string(), Network::new(100, 0, 1)),
        ];
        let mut encoder = Encoder::new(CURRENT_ENCODING_VERSION, networks).unwrap();

        let block_updates = vec![
            ("A:1".to_string(), BlockPtr::new(1, [0; 32])),
            ("B:2".to_string(), BlockPtr::new(250, [0; 32])),
        ];
        let compressed = encoder
            .compress(&[Message::SetBlockNumbersForNextEpoch(
                block_updates.into_iter().collect(),
            )])
            .unwrap();

        let accelerations = compressed
            .last()
            .unwrap()
            .as_non_empty_block_numbers()
            .unwrap()
            .0;
        assert_eq!(accelerations, [1, 150]);
    }

    #[test]
    fn pipeline() {
        let mut messages = Vec::new();

        // Skip some empty epochs
        for _ in 0..20 {
            messages.push(Message::SetBlockNumbersForNextEpoch(BTreeMap::new()));
        }

        let networks: Vec<_> = ["A:1991", "B:2kl", "C:190", "D:18818"]
            .iter()
            .map(|i| i.to_string())
            .enumerate()
            .map(|(i, s)| {
                (
                    s,
                    Network {
                        block_number: 0,
                        block_delta: 0,
                        array_index: i as u64,
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

        let mut engine = Encoder::new(0, networks).unwrap();
        engine.compress(&messages[..]).unwrap();

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

    #[test]
    fn register_networks_changes_state() {
        let mut encoder = Encoder::new(CURRENT_ENCODING_VERSION, vec![]).unwrap();
        let networks_before = encoder.networks.clone();

        encoder
            .compress(&[Message::RegisterNetworks {
                remove: vec![],
                add: vec!["foo:bar".to_string()],
            }])
            .unwrap();

        let networks_after = encoder.networks.clone();

        assert_ne!(networks_before, networks_after);
    }

    #[test]
    fn set_block_numbers_changes_state() {
        let mut encoder = Encoder::new(
            CURRENT_ENCODING_VERSION,
            vec![("foo:bar".to_string(), Network::new(42, 0, 0))],
        )
        .unwrap();
        let networks_before = encoder.networks.clone();

        encoder
            .compress(&[Message::SetBlockNumbersForNextEpoch(
                vec![("foo:bar".to_string(), BlockPtr::new(42, [0; 32]))]
                    .into_iter()
                    .collect(),
            )])
            .unwrap();

        // We didn't update any block numbers.
        assert_eq!(networks_before, encoder.networks);

        encoder
            .compress(&[Message::SetBlockNumbersForNextEpoch(
                vec![("foo:bar".to_string(), BlockPtr::new(1337, [0; 32]))]
                    .into_iter()
                    .collect(),
            )])
            .unwrap();

        // We did update block numbers, this time around.
        assert_ne!(networks_before, encoder.networks);
        assert_ne!(encoder.networks.last().unwrap().1.block_delta, 0);
    }

    #[test]
    fn change_permissions_message() {
        let mut encoder = Encoder::new(CURRENT_ENCODING_VERSION, vec![]).unwrap();

        let test_permissions = vec![
            "RegisterNetworksAndAliasesMessage".to_string(),
            "CorrectEpochsMessage".to_string(),
        ];

        let result_permissions = vec![6, 1];

        let compressed = encoder
            .compress(&[Message::ChangePermissions {
                address: [1u8; 20],
                valid_through: 123u64,
                permissions: test_permissions.clone(),
            }])
            .unwrap();

        assert_eq!(compressed.len(), 1);

        match &compressed[0] {
            CompressedMessage::ChangePermissions {
                address,
                valid_through,
                permissions,
            } => {
                assert_eq!(*address, [1u8; 20]);
                assert_eq!(*valid_through, 123u64);
                assert_eq!(*permissions, result_permissions);
            }
            _ => panic!("Expected ChangePermissions message"),
        }
    }
}
