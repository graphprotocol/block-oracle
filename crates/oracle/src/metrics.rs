use futures::Future;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use prometheus::{Encoder, HistogramOpts, HistogramVec, IntGaugeVec, Registry, TextEncoder};
use std::{convert::Infallible, net::SocketAddr};

#[derive(Debug, Clone)]
pub struct Metrics {
    registry: Registry,
    retries_by_jsonrpc_provider: HistogramVec,
    current_epoch: IntGaugeVec,
}

impl Metrics {
    pub fn encode(&self) -> Vec<u8> {
        let mut buffer = vec![];
        TextEncoder::new()
            .encode(&self.registry.gather(), &mut buffer)
            .expect("failed to encode gathered Prometheus metrics");
        buffer
    }
}

impl Default for Metrics {
    fn default() -> Self {
        let registry = Registry::new();

        let retries_by_jsonrpc_provider = HistogramVec::new(
            HistogramOpts::new(
                "retries_by_jsonrpc_provider",
                "Number of JSON-RPC request retries.",
            ),
            &["jsonrpc_provider"],
        )
        .expect("failed to create metric");
        registry
            .register(Box::new(retries_by_jsonrpc_provider.clone()))
            .expect("failed to register Prometheus metric");

        let current_epoch = IntGaugeVec::new("current_epoch", "Epoch Manager Current Epoch")
            .expect("failed to create metric");
        registry
            .register(Box::new(current_epoch.clone()))
            .expect("failed to register metric");

        Self {
            registry,
            retries_by_jsonrpc_provider,
            current_epoch,
        }
    }
}

async fn handle_metrics_server_request(
    _req: Request<Body>,
    metrics: &'static Metrics,
) -> Result<Response<Body>, Infallible> {
    let encoded = metrics.encode();
    let body = Body::from(encoded);
    let response = Response::builder()
        .body(body)
        .expect("failed to build response body with Prometheus encoded metrics");
    Ok(response)
}

pub fn metrics_server(metrics: &'static Metrics) -> impl Future<Output = Result<(), hyper::Error>> {
    // TODO: make this configurable
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let make_service = make_service_fn(move |_conn| async move {
        Ok::<_, Infallible>(service_fn(move |req| {
            handle_metrics_server_request(req, metrics)
        }))
    });
    Server::bind(&addr).serve(make_service)
}
