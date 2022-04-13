use std::collections::HashMap;

pub struct Encoder {}

//use epoch_encoding::{CompressedMessage, Message};
//
//use crate::store::Caip2ChainId;
//
//#[derive(Debug, Clone, PartialEq, Eq, Hash)]
//pub struct CompressionState {
//    pub latest_block_number: u64,
//    pub last_emitted_block_number: u64,
//    pub block_delta: u64,
//}
//
//impl CompressionState {
//    pub fn delta(&self) -> u64 {
//        self.latest_block_number
//            .checked_sub(self.last_emitted_block_number)
//            .unwrap()
//    }
//
//    pub fn acceleration(&self) -> i64 {
//        self.delta() as i64 - self.block_delta as i64
//    }
//}
//
///// Encodes and compresses messages.
//#[derive(Debug, Clone)]
//#[non_exhaustive]
//pub struct Encoder {
//    pub compression_state: HashMap<Caip2ChainId, CompressionState>,
//}
//
//impl Encoder {
//    pub fn encode_message(
//        &self,
//        message: &Message,
//        compressed: &mut Vec<CompressedMessage>,
//    ) -> CompressedMessage {
//        match message {
//            Message::SetBlockNumbersForNextEpoch(block_ptrs) => {
//                // There are separate cases for empty sets and non-empty sets.
//                // If we have an empty set we may need to extend the last message.
//                if block_ptrs.len() == 0 {
//                    let count = loop {
//                        match compressed.last_mut() {
//                            Some(CompressedMessage::SetBlockNumbersForNextEpoch(
//                                CompressedSetBlockNumbersForNextEpoch::Empty { count },
//                            )) => break count,
//                            _ => {
//                                compressed.push(CompressedMessage::SetBlockNumbersForNextEpoch(
//                                    CompressedSetBlockNumbersForNextEpoch::Empty { count: 0 },
//                                ));
//                            }
//                        }
//                    };
//                    *count += 1;
//                } else {
//                    // Sort the block pointers by network id.
//                    let mut by_id = Vec::new();
//                    for block_ptr in block_ptrs {
//                        let id = if let Some(id) = networks.get(block_ptr.0) {
//                            id
//                        } else {
//                            return Ok(Err(ValidationError::NetworkMismatch));
//                        };
//                        by_id.push((*id, block_ptr.1));
//                    }
//                    by_id.sort_unstable_by_key(|i| i.0);
//
//                    // Get accelerations and merkle leaves based on previous deltas.
//                    let mut accelerations = Vec::with_capacity(by_id.len());
//                    let mut merkle_leaves = Vec::with_capacity(by_id.len());
//                    for (id, ptr) in by_id.into_iter() {
//                        let mut network = if let Some(network) = db.get_network(id).await? {
//                            network
//                        } else {
//                            return Ok(Err(ValidationError::NetworkMismatch));
//                        };
//                        let delta = (ptr.number - network.block_number) as i64;
//                        let acceleration = delta - network.block_delta;
//                        network.block_number = ptr.number;
//                        network.block_delta = delta;
//                        db.set_network(id, network).await?;
//                        accelerations.push(acceleration);
//                        merkle_leaves.push(MerkleLeaf {
//                            network_id: id,
//                            block_hash: ptr.hash,
//                            block_number: ptr.number,
//                        });
//                    }
//
//                    let root = merkle_root(&merkle_leaves);
//
//                    compressed.push(CompressedMessage::SetBlockNumbersForNextEpoch(
//                        CompressedSetBlockNumbersForNextEpoch::NonEmpty {
//                            accelerations,
//                            root,
//                        },
//                    ));
//                }
//            }
//
//            _ => todo!(),
//        }
//
//        Ok(Ok(()))
//    }
//}
//
