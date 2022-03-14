mod database;
mod encoding;
mod merkle;
mod messages;
mod varint;

use async_trait::async_trait;
use merkle::{merkle_root, MerkleLeaf};
use {database::*, encoding::*, messages::*};

type Bytes32 = [u8; 32];

type ConnResult<Ok, Conn> = DbResult<Ok, <Conn as Connection>::Database>;

type DbResult<Ok, DB> =
    std::result::Result<std::result::Result<Ok, ValidationError>, <DB as Database>::Error>;

type NetworkId = u64;

#[async_trait]
pub trait Blockchain {
    type Err;
    async fn submit_oracle_messages(&mut self, transaction: Transaction) -> Result<(), Self::Err>;
}

#[derive(Debug)]
pub enum ValidationError {
    NetworkMismatch,
}

// Publishes the latest epoch oracle messages.
// First, compresses the message using the latest database state.
// Then, encode the message to a (blockchain) transaction
// Publish that transaction, and if successful, finally commit the update
// to the database within a (db) transaction.
pub async fn publish<'a, Conn, Chain>(
    db: &Conn,
    messages: &[Message],
    chain: &mut Chain,
) -> ConnResult<(), Conn>
where
    Conn: Connection,
    <Conn as Connection>::Database: Send + Sync,
    Chain: Blockchain + Send + Sync,
    <<Conn as Connection>::Database as Database>::Error: From<<Chain as Blockchain>::Err>,
{
    db.transaction(|mut db| async move {
        let compressed = match compress_messages(&mut db, messages).await? {
            Ok(compressed) => compressed,
            Err(e) => return Ok(Err(e)),
        };
        let encoded = encode_messages(&compressed);

        let nonce = db.get_next_nonce().await?;
        db.set_next_nonce(nonce).await?;

        let transaction = Transaction {
            nonce,
            payload: encoded,
        };

        chain.submit_oracle_messages(transaction).await?;

        Ok(Ok(()))
    })
    .await
}

async fn compress_message<Db>(
    db: &mut Db,
    message: &Message,
    compressed: &mut Vec<CompressedMessage>,
) -> DbResult<(), Db>
where
    Db: Database,
{
    match message {
        Message::SetBlockNumbersForNextEpoch(block_ptrs) => {
            //////////////////////////////////
            // Synchronize the network list //
            //////////////////////////////////
            let mut networks = db.get_network_ids().await?;
            let mut add_networks = Vec::new();
            let mut remove_networks = Vec::new();

            // Removes are processed first. In this way we can re-use ids.
            // Would have been nice to use drain_filter here but it's not
            // stable Rust yet.
            networks.retain(|k, v| {
                if !block_ptrs.contains_key(k) {
                    remove_networks.push(*v);
                    false
                } else {
                    true
                }
            });

            // Process added networks
            for k in block_ptrs.keys() {
                if networks.contains_key(k) {
                    continue;
                }
                add_networks.push(k.to_owned());

                // TODO: Performance: Silly O(N^2) algorithm used here
                // Could use a freelist instead
                let mut unused = 0;
                loop {
                    if !networks.values().any(|&v| v == unused) {
                        break;
                    }
                    unused += 1;
                }

                let prev = networks.insert(k.to_owned(), unused);
                debug_assert!(prev == None);
                db.set_network(
                    unused,
                    Network {
                        block_delta: 0,
                        block_number: 0,
                    },
                )
                .await?;
            }

            if add_networks.len() != 0 || remove_networks.len() != 0 {
                compressed.push(CompressedMessage::RegisterNetworks {
                    add: add_networks,
                    remove: remove_networks,
                });

                db.set_network_ids(networks.clone()).await?;
            }

            ///////////////////////
            // Set Block Numbers //
            ///////////////////////

            // Sort the block pointers by network id.
            let mut by_id = Vec::new();
            for block_ptr in block_ptrs {
                let id = if let Some(id) = networks.get(block_ptr.0) {
                    id
                } else {
                    return Ok(Err(ValidationError::NetworkMismatch));
                };
                by_id.push((*id, block_ptr.1));
            }
            by_id.sort_unstable_by_key(|i| i.0);

            // Get accelerations and merkle leaves based on previous deltas.
            let mut accelerations = Vec::with_capacity(by_id.len());
            let mut merkle_leaves = Vec::with_capacity(by_id.len());
            for (id, ptr) in by_id.into_iter() {
                let mut network = if let Some(network) = db.get_network(id).await? {
                    network
                } else {
                    return Ok(Err(ValidationError::NetworkMismatch));
                };
                let delta = (ptr.number - network.block_number) as i64;
                let acceleration = delta - network.block_delta;
                network.block_number = ptr.number;
                network.block_delta = delta;
                db.set_network(id, network).await?;
                accelerations.push(acceleration);
                merkle_leaves.push(MerkleLeaf {
                    network_id: id,
                    block_hash: ptr.hash,
                    block_number: ptr.number,
                });
            }

            let root = if merkle_leaves.len() != 0 {
                Some(merkle_root(&merkle_leaves))
            } else {
                None
            };

            compressed.push(CompressedMessage::SetBlockNumbersForNextEpoch {
                accelerations,
                root,
            });
        }

        _ => todo!(),
    }

    Ok(Ok(()))
}

async fn compress_messages<Db>(
    db: &mut Db,
    messages: &[Message],
) -> DbResult<Vec<CompressedMessage>, Db>
where
    Db: Database,
{
    let mut result = Vec::new();
    for message in messages {
        match compress_message(db, message, &mut result).await? {
            Ok(()) => {}
            Err(e) => return Ok(Err(e)),
        }
    }
    Ok(Ok(result))
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            database::mocks::*,
            messages::{BlockPtr, Message},
        },
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
            .collect();

        // Add blocks
        for i in 0..4 {
            let nums = networks
                .iter()
                .enumerate()
                .map(|(ni, n)| {
                    (
                        n.to_string(),
                        BlockPtr {
                            number: 300 * (i + (ni as u64)) + i,
                            hash: [1; 32],
                        },
                    )
                })
                .collect();
            messages.push(Message::SetBlockNumbersForNextEpoch(nums));
        }

        let db = MockConnection::new();

        struct MockChain {}

        #[async_trait]
        impl Blockchain for MockChain {
            type Err = Never;
            async fn submit_oracle_messages(
                &mut self,
                transaction: Transaction,
            ) -> Result<(), Self::Err> {
                println!("Len: {}", transaction.payload.len());
                println!("{:?}", transaction);
                Ok(())
            }
        }

        publish(&db, &messages, &mut MockChain {})
            .await
            .unwrap()
            .unwrap();

        // TODO: Add ability to skip epochs? Right now the way to get past this is to
        // just add 80 or so SetBlockNumbers.
    }
}
