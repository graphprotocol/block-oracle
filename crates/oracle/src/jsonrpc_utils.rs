use backoff::future::retry;
use backoff::{ExponentialBackoff, ExponentialBackoffBuilder};
use epoch_encoding::BlockPtr;
use futures::future::try_join_all;
use futures::TryFutureExt;
use jsonrpc_core::{Call, Value};
use std::time::Duration;
use std::{future::Future, pin::Pin};
use tracing::{error, trace};
use url::Url;
use web3::types::{Transaction, H160, H256, U64};
use web3::{transports::Http, RequestId};
use web3::{Transport, Web3};

/// A wrapper around [`web3::Transport`] that retries JSON-RPC calls on failure.
#[derive(Debug, Clone)]
pub struct JrpcExpBackoff<T = Http> {
    inner: T,
    strategy: ExponentialBackoff,
}

impl<T> JrpcExpBackoff<T> {
    pub fn new(transport: T, max_wait: Duration) -> Self {
        let strategy = ExponentialBackoffBuilder::new()
            .with_max_elapsed_time(Some(max_wait))
            .build();

        Self {
            inner: transport,
            strategy,
        }
    }
}

impl JrpcExpBackoff {
    pub fn http(jrpc_url: Url, max_wait: Duration) -> Self {
        // Unwrap: URLs were already parsed and are valid.
        let client = Http::new(jrpc_url.as_str()).expect("failed to create HTTP transport");
        Self::new(client, max_wait)
    }
}

impl<T> web3::Transport for JrpcExpBackoff<T>
where
    T: web3::Transport + 'static,
{
    type Out = Pin<Box<dyn Future<Output = web3::error::Result<Value>>>>;

    fn prepare(&self, method: &str, params: Vec<Value>) -> (RequestId, Call) {
        self.inner.prepare(method, params)
    }

    fn send(&self, id: RequestId, request: Call) -> Self::Out {
        let strategy = self.strategy.clone();
        let transport = self.inner.clone();
        let op = move || {
            trace!(?id, ?request, "Sending JRPC call");
            transport
                .send(id, request.clone())
                .map_err(backoff::Error::transient)
        };
        Box::pin(retry(strategy, op))
    }
}

pub async fn get_latest_block<T>(web3: Web3<T>) -> web3::Result<BlockPtr>
where
    T: Transport,
{
    let block_num = web3.eth().block_number().await?;
    let block_id = web3::types::BlockId::Number(block_num.into());
    let block = web3
        .eth()
        .block(block_id)
        .await?
        // We were just told that's the latest block number, so it wouldn't
        // make sense for this to fail. How can it *not* find a block with
        // that block number?
        .expect("Invalid block number");

    // Same thing here. We expect data to be consistent across multiple
    // JSON-RPC calls.
    if block.number != Some(block_num) {
        error!(
            block_num1 = ?block_num,
            block_num2 = ?block.number,
            "The JSON-RPC provider is responding to queries with inconsistent data. This is most likely a bug."
        );
    }
    assert_eq!(block.number, Some(block_num));
    assert!(block.hash.is_some());

    Ok(BlockPtr {
        number: block_num.as_u64(),
        hash: block.hash.unwrap().into(),
    })
}

/// Scans a block range for relevant transactions.
///
/// Returns a vector of the filtered transactions.
pub async fn calls_in_block_range<T>(
    web3: Web3<T>,
    from_block: U64,
    to_block: U64,
    from_address: H160,
    to_address: H160,
) -> Result<Vec<Transaction>, web3::Error>
where
    T: Transport,
{
    let block_range: Vec<_> = (from_block.as_u64()..=to_block.as_u64()).collect();
    // Prepare all async calls for fetching blocks in range
    let block_futures: Vec<_> = block_range
        .iter()
        .map(|block_number| {
            let block_number: U64 = (*block_number).into();
            web3.eth().block(block_number.into())
        })
        .collect();
    // Searching is fallible, so we get a vector of options
    let optional_blocks = try_join_all(block_futures).await?;
    // This will store all transaction hashes found within the fetched blocks
    let mut transaction_hashes: Vec<H256> = Vec::new();
    // Extract the transaction hashes from the the received blocks
    for (opt, block_number) in optional_blocks.into_iter().zip(block_range) {
        if let Some(block) = opt {
            for transaction_hash in block.transactions.into_iter() {
                transaction_hashes.push(transaction_hash)
            }
        } else {
            error!(%block_number, "Failed to fetch block by number");
        }
    }
    // Prepare the async calls for fetching the full transaction objects
    let transaction_futures: Vec<_> = transaction_hashes
        .iter()
        .map(|transaction_hash| web3.eth().transaction((*transaction_hash).into()))
        .collect();
    // Again, searching is fallible, meaning we get back a vector of optional values
    let optional_transactions = try_join_all(transaction_futures).await?;
    // This will hold the filtered transactions that will be returned by thins function
    let mut filtered_transactions: Vec<Transaction> = Vec::new();
    // Iterate over all received transactions and filter the ones we are interested in
    for (opt, transaction_hash) in optional_transactions.into_iter().zip(transaction_hashes) {
        if let Some(transaction) = opt {
            if matches!((transaction.from, transaction.to), (Some(a), Some(b)) if a == from_address && b == to_address)
            {
                filtered_transactions.push(transaction)
            }
        } else {
            error!(%transaction_hash, "Failed to fetch transaction by hash");
        }
    }

    Ok(filtered_transactions)
}
