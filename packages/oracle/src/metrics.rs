use prometheus::*;

pub struct Metrics {
    registry: Registry,
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
        Self { registry: r }
    }
}
