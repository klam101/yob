use yob_core::jobs::Fetcher;
use yob_db::PgPool;

/// Shared handles passed to every request handler.
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: deadpool_redis::Pool,
    /// For verifying job boards when a company is added. Shares the
    /// per-host rate limit with the worker through Redis.
    pub fetcher: Fetcher,
}
