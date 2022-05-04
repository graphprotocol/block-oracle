use backoff::future::retry;
use backoff::{ExponentialBackoff, ExponentialBackoffBuilder};
use futures::TryFutureExt;
use jsonrpc_core::{Call, Value};
use std::sync::Arc;
use std::time::Duration;
use std::{future::Future, pin::Pin};
use tracing::trace;
use url::Url;
use web3::{transports::Http, RequestId};

#[derive(Debug, Clone)]
pub struct JsonRpcExponentialBackoff {
    inner: Arc<Http>,
    strategy: ExponentialBackoff,
}

impl JsonRpcExponentialBackoff {
    pub fn new(jrpc_url: Url, max_wait: Duration) -> Self {
        let strategy = ExponentialBackoffBuilder::new()
            .with_max_elapsed_time(Some(max_wait))
            .build();
        // Unwrap: URLs were already parsed and are valid.
        let client = Http::new(jrpc_url.as_str()).expect("failed to create HTTP transport");
        let inner = Arc::new(client);
        Self { inner, strategy }
    }
}

impl web3::Transport for JsonRpcExponentialBackoff {
    type Out = Pin<Box<dyn Future<Output = web3::error::Result<Value>>>>;

    fn prepare(&self, method: &str, params: Vec<Value>) -> (RequestId, Call) {
        self.inner.prepare(method, params)
    }

    fn send(&self, id: RequestId, request: Call) -> Self::Out {
        let strategy = self.strategy.clone();
        let http = Arc::clone(&self.inner);
        let op = move || {
            trace!(?id, ?request, "Sending JRPC call");
            http.send(id, request.clone())
                .map_err(backoff::Error::transient)
        };
        Box::pin(retry(strategy, op))
    }
}
