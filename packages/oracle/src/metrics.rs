use prometheus::*;

#[derive(Debug, Clone)]
pub struct Metrics {
    registry: Registry,

    pub db_queries: Counter,
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

        let db_queries = Counter::new("db_queries", "Number of database queries").unwrap();
        let retries_by_jsonrpc_provider = HistogramVec::new(
            HistogramOpts::new(
                "retries_by_jsonrpc_provider",
                "Number of JSON-RPC request retries.",
            ),
            &["jsonrpc_provider"],
        )
        .unwrap();

        r.register(Box::new(db_queries.clone())).unwrap();
        r.register(Box::new(retries_by_jsonrpc_provider.clone()))
            .unwrap();

        Self {
            registry: r,
            db_queries,
            retries_by_jsonrpc_provider,
        }
    }
}
