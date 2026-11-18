use tracing_subscriber::{EnvFilter, fmt};

/// Logs to stdout, filtered by `RUST_LOG` (defaults to `info`).
pub fn init() {
    fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
}
