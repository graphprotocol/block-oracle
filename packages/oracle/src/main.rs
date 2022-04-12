mod config;
mod store;

use async_trait::async_trait;
use epoch_encoding::{self, Blockchain, Transaction};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::io;
use store::models::Caip2ChainId;
use web3::transports::Http;
use web3::types::{Bytes, TransactionParameters, U256};

pub use store::Store;

lazy_static! {
    pub static ref CONFIG: config::Config = config::Config::parse();
}

/// Actively listens for new blocks and reorgs from registered blockchains. Also, it checks the
/// number of confirmations for transactions sent to the DataEdge contract.
type EventSource = ();

/// Responsible for receiving the encodede payload, constructing and signing the transactions to
/// Ethereum Mainnet.
type EthereumClient = ();

/// Encodes and compresses messages.
type Encoder = ();

/// Tracks current Ethereum mainnet epoch.
type EpochTracker = ();

// -------------

type BlockChainState = ();

/// The main application in-memory state
struct Oracle {
    // -- components --
    store: Store,
    event_source: EventSource,
    encoder: Encoder,
    ethereum_client: EthereumClient,
    epoch_tracker: EpochTracker,

    // -- data --
    state_by_blockchain: HashMap<Caip2ChainId, BlockChainState>,
}

pub struct Web3JsonRpc {
    client: web3::Web3<Http>,
}

impl Web3JsonRpc {
    pub fn new(transport: web3::transports::Http) -> Self {
        let client = web3::Web3::new(transport);
        Self { client }
    }
}

#[async_trait]
impl Blockchain for Web3JsonRpc {
    type Err = String;

    async fn submit_oracle_messages(&mut self, transaction: Transaction) -> Result<(), Self::Err> {
        let tx_object = TransactionParameters {
            to: Some(CONFIG.contract_address.clone()),
            value: U256::zero(),
            nonce: Some(transaction.nonce.into()),
            data: Bytes::from(transaction.payload),
            ..Default::default()
        };
        let private_key = CONFIG.owner_private_key.clone();
        let signed = self
            .client
            .accounts()
            .sign_transaction(tx_object, &private_key)
            .await
            .unwrap();

        self.client
            .eth()
            .send_raw_transaction(signed.raw_transaction)
            .await
            .unwrap();

        Ok(())
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // Immediately dereference `CONFIG` to trigger `lazy_static` initialization.
    let _ = &*CONFIG;
    let json_rpc = Web3JsonRpc::new(Http::new("http://localhost:8545").unwrap());
    Ok(())
}
