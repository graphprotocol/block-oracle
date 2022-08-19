use std::{convert::Infallible, net::SocketAddr};

use futures::Future;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use prometheus::{Encoder, HistogramOpts, HistogramVec, Registry, TextEncoder};

#[derive(Debug, Clone)]
pub struct Metrics {
    registry: Registry,

    pub retries_by_jsonrpc_provider: HistogramVec,
}

impl Metrics {
    pub fn serve(&self) -> Vec<u8> {
        let mut buffer = vec![];
        TextEncoder::new()
            .encode(&self.registry.gather(), &mut buffer)
            .unwrap();
        buffer
    }
}

impl Default for Metrics {
    fn default() -> Self {
        let r = Registry::new();

        let retries_by_jsonrpc_provider = HistogramVec::new(
            HistogramOpts::new(
                "retries_by_jsonrpc_provider",
                "Number of JSON-RPC request retries.",
            ),
            &["jsonrpc_provider"],
        )
        .unwrap();

        r.register(Box::new(retries_by_jsonrpc_provider.clone()))
            .unwrap();

        Self {
            registry: r,
            retries_by_jsonrpc_provider,
        }
    }
}

async fn handle(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new(Body::from("Hello World")))
}

pub fn metrics_server(
    _metrics: &'static Metrics,
) -> impl Future<Output = Result<(), hyper::Error>> {
    // TODO: make this configurable
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let make_service = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle)) });
    Server::bind(&addr).serve(make_service)
}
