use lazy_static::lazy_static;
use prometheus::{
    register_histogram_vec_with_registry, register_int_gauge_vec_with_registry, Encoder,
    HistogramVec, IntGaugeVec, Registry, TextEncoder,
};

lazy_static! {
    pub static ref METRICS: Metrics = Metrics::new().expect("failed to create Metrics");
}

#[derive(Debug, Clone)]
pub struct Metrics {
    registry: Registry,
    retries_by_jsonrpc_provider: HistogramVec,
    current_epoch: IntGaugeVec,
}

impl Metrics {
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();

        let retries_by_jsonrpc_provider = register_histogram_vec_with_registry!(
            "retries_by_jsonrpc_provider",
            "Number of JSON-RPC request retries.",
            &["provider"],
            registry
        )?;

        let current_epoch = register_int_gauge_vec_with_registry!(
            "current_epoch",
            "Current Epoch",
            &["source"],
            registry
        )?;

        Ok(Self {
            registry,
            retries_by_jsonrpc_provider,
            current_epoch,
        })
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buffer = vec![];
        TextEncoder::new()
            .encode(&self.registry.gather(), &mut buffer)
            .expect("failed to encode gathered Prometheus metrics");
        buffer
    }

    pub fn set_current_epoch(&self, label: &str, current_epoch: i64) {
        self.current_epoch
            .get_metric_with_label_values(&[label])
            .unwrap()
            .set(current_epoch as i64);
    }
}

pub mod server {
    use super::Metrics;
    use futures::Future;
    use hyper::{
        service::{make_service_fn, service_fn},
        Body, Request, Response, Server,
    };
    use std::{convert::Infallible, net::SocketAddr};
    use tracing::info;

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

    pub fn metrics_server(
        metrics: &'static Metrics,
        port: u16,
    ) -> impl Future<Output = Result<(), hyper::Error>> {
        // TODO: make this configurable
        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        let make_service = make_service_fn(move |_conn| async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                handle_metrics_server_request(req, metrics)
            }))
        });
        info!("Starting metrics server at port {port}");
        Server::bind(&addr).serve(make_service)
    }
}
