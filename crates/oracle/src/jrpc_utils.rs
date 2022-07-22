use crate::{Caip2ChainId, JrpcProviderForChain};
use backoff::{future::retry, ExponentialBackoff, ExponentialBackoffBuilder};
use epoch_encoding::BlockPtr;
use futures::{future::try_join_all, TryFutureExt};
use futures::{
    stream::{FuturesUnordered, StreamExt},
    FutureExt,
};
use jsonrpc_core::{Call, Value};
use std::collections::BTreeMap;
use std::ops::RangeInclusive;
use std::{future::Future, pin::Pin, time::Duration};
use tracing::trace;
use url::Url;
use web3::types::{BlockId, BlockNumber, Transaction, H160, U64};
use web3::{transports::Http, RequestId, Transport, Web3};

/// A wrapper around [`web3::Transport`] that retries JSON-RPC calls on failure.
#[derive(Debug, Clone)]
pub struct JrpcExpBackoff<T = Http> {
    inner: T,
    strategy: ExponentialBackoff,
    network: Caip2ChainId,
}

impl<T> JrpcExpBackoff<T> {
    pub fn new(transport: T, network: Caip2ChainId, max_wait: Duration) -> Self {
        let strategy = ExponentialBackoffBuilder::new()
            .with_max_elapsed_time(Some(max_wait))
            .build();

        Self {
            inner: transport,
            strategy,
            network,
        }
    }
}

impl JrpcExpBackoff {
    pub fn http(jrpc_url: Url, network: Caip2ChainId, max_wait: Duration) -> Self {
        // Unwrap: URLs were already parsed and are valid.
        let client = Http::new(jrpc_url.as_str()).expect("failed to create HTTP transport");
        Self::new(client, network, max_wait)
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
        let network = self.network.clone();
        let op = move || {
            trace!(?id, ?request, %network, "Sending JRPC call");
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
    let block = web3
        .eth()
        // We're asking for the latest head of the blockchain.
        .block(BlockId::Number(BlockNumber::Latest))
        .await?
        .ok_or_else(|| web3::Error::InvalidResponse("No latest block found".to_string()))?;

    match (block.number, block.hash) {
        (Some(number), Some(hash)) => Ok(BlockPtr {
            number: number.as_u64(),
            hash: hash.0,
        }),
        _ => Err(web3::Error::InvalidResponse(
            "The latest block is missing a number or hash".to_string(),
        )),
    }
}

/// Fetches the latest available block number and hash from all `chains`.
pub async fn get_latest_blocks<T>(
    chains: &[JrpcProviderForChain<T>],
) -> BTreeMap<Caip2ChainId, web3::Result<BlockPtr>>
where
    T: web3::Transport,
{
    let mut tasks = chains
        .iter()
        .cloned()
        .map(|chain| get_latest_block(chain.web3).map(|block| (chain.chain_id, block)))
        .collect::<FuturesUnordered<_>>();

    let mut block_ptr_per_chain = BTreeMap::new();
    while let Some((chain_id, jrpc_call_result)) = tasks.next().await {
        block_ptr_per_chain.insert(chain_id, jrpc_call_result);
    }

    assert!(block_ptr_per_chain.len() == chains.len());
    block_ptr_per_chain
}

/// Scans a block range for relevant transactions.
///
/// Returns a vector of the filtered transactions.
pub async fn calls_in_block_range<T>(
    web3: Web3<T>,
    block_range: RangeInclusive<u64>,
    from_address: H160,
    to_address: H160,
) -> web3::Result<Vec<Transaction>>
where
    T: Transport,
{
    let block_numbers: Vec<u64> = block_range.collect();
    // Prepare all async calls for fetching blocks in range.
    let block_futures = block_numbers
        .iter()
        .map(|block_number| web3.eth().block_with_txs(U64::from(*block_number).into()));

    // Searching is fallible, so we get a vector of options.
    let blocks = try_join_all(block_futures).await?;

    let mut txs = vec![];
    for (i, block_opt) in blocks.into_iter().enumerate() {
        let block_number = block_numbers[i];
        let block = block_opt.ok_or_else(|| {
            web3::Error::InvalidResponse(format!(
                "Block {} not found during range scan",
                block_number
            ))
        })?;
        txs.extend_from_slice(&block.transactions);
    }

    txs.retain(|tx| tx.from == Some(from_address) && tx.to == Some(to_address));
    Ok(txs)
}
