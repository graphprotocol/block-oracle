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

pub struct BlockPtr {
    number: u64,
    hash: Bytes32,
}

// TODO: For 'correct' include hash, count, and (if count is nonzero) merkle root

pub struct Transaction {
    pub nonce: u64,
    pub calldata: Vec<u8>,
}

#[async_trait]
pub trait Blockchain {
    type Err;
    async fn submit_oracle_messages(&mut self, transaction: Transaction) -> Result<(), Self::Err>;
}

pub enum ValidationError {
    NetworkMismatch,
}

// Publishes the latest epoch oracle messages.
// First, compresses the message using the latest database state.
// Then, encode the message to a (blockchain) transaction
// Publish that transaction, and if successful, finally commit the update
// to the database within a (db) transaction.
pub async fn publish<Conn, Chain>(
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
    db.transaction(|db| async {
        let compressed = match compress_messages(db, messages).await? {
            Ok(compressed) => compressed,
            Err(e) => return Ok(Err(e)),
        };
        let encoded = encode_messages(&compressed);

        let nonce = db.get_next_nonce().await?;
        db.set_next_nonce(nonce).await?;

        let transaction = Transaction {
            nonce,
            calldata: encoded,
        };

        chain.submit_oracle_messages(transaction).await?;

        Ok(Ok(()))
    })
    .await
}

async fn compress_message<Db>(db: &mut Db, message: &Message) -> DbResult<CompressedMessage, Db>
where
    Db: Database,
{
    match message {
        Message::SetBlockNumbersForNextEpoch(block_ptrs) => {
            // Sort the block pointers by network id.
            let networks = db.get_network_ids().await?;
            if networks.len() != block_ptrs.len() {
                return Ok(Err(ValidationError::NetworkMismatch));
            }

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

            // Get accelerations to accelerations
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

            let root = merkle_root(&merkle_leaves);

            Ok(Ok(CompressedMessage::SetBlockNumbersForNextEpoch {
                accelerations,
                root,
            }))
        }
        _ => todo!(),
    }
}

async fn compress_messages<Db>(
    db: &mut Db,
    messages: &[Message],
) -> DbResult<Vec<CompressedMessage>, Db>
where
    Db: Database,
{
    let mut result = Vec::with_capacity(messages.len());
    for message in messages {
        let message = match compress_message(db, message).await? {
            Ok(m) => m,
            Err(e) => return Ok(Err(e)),
        };
        result.push(message);
    }
    Ok(Ok(result))
}
