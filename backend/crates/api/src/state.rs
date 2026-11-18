use yob_db::PgPool;

/// Shared handles passed to every request handler.
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: deadpool_redis::Pool,
}
