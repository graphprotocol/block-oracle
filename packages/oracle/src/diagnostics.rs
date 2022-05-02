use std::env::set_var;
use tracing_subscriber::{
    filter::{EnvFilter, LevelFilter},
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

pub fn init_logging(log_level: LevelFilter) {
    set_var("RUST_LOG", "block_oracle=trace");

    let filter = EnvFilter::builder()
        .with_default_directive(log_level.into())
        .from_env_lossy();

    let stdout = fmt::layer()
        .without_time()
        .with_target(false)
        .with_writer(std::io::stdout);

    tracing_subscriber::registry()
        .with(filter)
        .with(stdout)
        .init();
}
