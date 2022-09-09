use lazy_static::lazy_static;
use prometheus::{
    register_gauge_with_registry, register_histogram_vec_with_registry,
    register_int_counter_vec_with_registry, register_int_gauge_vec_with_registry,
    register_int_gauge_with_registry, Encoder, Gauge, HistogramVec, IntCounterVec, IntGauge,
    IntGaugeVec, Registry, TextEncoder,
};
use std::time::UNIX_EPOCH;
use tracing::{debug, error, info};
use warp::{http::Response, Filter};

lazy_static! {
    pub static ref METRICS: Metrics = Metrics::new().expect("failed to create Metrics");
}

#[derive(Debug, Clone)]
pub struct Metrics {
    registry: Registry,
    jrpc_request_duration_seconds: HistogramVec,
    jrpc_failure: IntCounterVec,
    current_epoch: IntGaugeVec,
    last_sent_message: Gauge,
    latest_block_number: IntGaugeVec,
    wallet_balance: IntGauge,
    subgraph_indexing_errors: IntGauge,
    subgraph_last_payload_health: IntGauge,
    subgraph_last_payload_block_number: IntGauge,
}

impl Metrics {
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();

        let jrpc_request_duration_seconds = register_histogram_vec_with_registry!(
            "epoch_block_oracle_jrpc_request_duration_seconds",
            "JSON RPC Request Duration",
            &["network"],
            registry
        )?;

        let jrpc_failure = register_int_counter_vec_with_registry!(
            "epoch_block_oracle_jrpc_failure_total",
            "JSON RPC Request Failure",
            &["network"],
            registry
        )?;

        let current_epoch = register_int_gauge_vec_with_registry!(
            "epoch_block_oracle_current_epoch",
            "Current Epoch",
            &["source"],
            registry
        )?;

        let last_sent_message = register_gauge_with_registry!(
            "epoch_block_oracle_last_sent_message",
            "Last Sent Message",
            registry
        )?;

        let latest_block_number = register_int_gauge_vec_with_registry!(
            "epoch_block_oracle_latest_block_number",
            "Latest Block Number",
            &["network", "source"],
            registry
        )?;

        let wallet_balance = register_int_gauge_with_registry!(
            "epoch_block_oracle_eth_balance",
            "Owner's ETH Balance",
            registry
        )?;

        let subgraph_indexing_errors = register_int_gauge_with_registry!(
            "epoch_block_oracle_subgraph_health",
            "Epoch Subgraph Indexing Errors",
            registry
        )?;

        let subgraph_last_payload_health = register_int_gauge_with_registry!(
            "epoch_block_oracle_subgraph_last_payload_health",
            "Epoch Subgraph Last Payload Health",
            registry
        )?;

        let subgraph_last_payload_block_number = register_int_gauge_with_registry!(
            "epoch_block_oracle_subgraph_last_payload_block_number",
            "Epoch Subgraph Last Payload Block Number",
            registry
        )?;

        Ok(Self {
            registry,
            jrpc_request_duration_seconds,
            jrpc_failure,
            current_epoch,
            last_sent_message,
            latest_block_number,
            wallet_balance,
            subgraph_indexing_errors,
            subgraph_last_payload_health,
            subgraph_last_payload_block_number,
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

    pub fn set_last_sent_message(&self) {
        let now = UNIX_EPOCH.elapsed().unwrap().as_secs_f64();
        self.last_sent_message.set(now);
    }

    pub fn set_jrpc_request_duration(&self, network: &str, duration: std::time::Duration) {
        let seconds = duration.as_secs_f64();
        self.jrpc_request_duration_seconds
            .get_metric_with_label_values(&[network])
            .unwrap()
            .observe(seconds)
    }

    pub fn set_latest_block_number(&self, network: &str, source: &str, block_number: i64) {
        self.latest_block_number
            .get_metric_with_label_values(&[network, source])
            .unwrap()
            .set(block_number)
    }

    pub fn set_wallet_balance(&self, balance: i64) {
        self.wallet_balance.set(balance)
    }

    pub fn set_subgraph_indexing_errors(&self, error: bool) {
        self.subgraph_indexing_errors.set(error as i64)
    }

    pub fn set_subgraph_last_payload_health(&self, healthy: bool, block_number: i64) {
        if healthy {
            debug!("Latest Epoch Subgraph payload at block #{block_number} is valid");
        } else {
            error!("Latest Epoch Subgraph payload at block #{block_number} is invalid");
        }
        self.subgraph_last_payload_health.set(healthy as i64);
        self.subgraph_last_payload_block_number.set(block_number)
    }

    pub fn track_jrpc_failure(&self, network: &str) {
        self.jrpc_failure
            .get_metric_with_label_values(&[network])
            .unwrap()
            .inc();
    }
}

pub async fn metrics_server(metrics: &'static Metrics, port: u16) {
    info!("Starting metrics server at port {port}/metrics");
    let endpoint = warp::path("metrics").map(|| {
        Response::builder()
            .header("Content-Type", "text/plain")
            .body(metrics.encode())
    });
    warp::serve(endpoint).run(([127, 0, 0, 1], port)).await;
}
