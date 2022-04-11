mod store;

use epoch_encoding;
use std::collections::HashMap;
use std::io;
use store::models::Caip2ChainId;
pub use store::Store;

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

fn main() -> io::Result<()> {
    Ok(())
}
