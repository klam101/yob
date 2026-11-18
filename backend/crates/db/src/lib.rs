//! Postgres connection pool and migrations. Queries live next to the module
//! that owns the table, in `yob-core`; this crate only provides the plumbing.

use std::time::Duration;

pub use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
}

/// Applies any pending migrations from `crates/db/migrations`. Safe to call
/// from both the API and the worker: sqlx holds a Postgres lock while it runs.
pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

/// Round-trips a trivial query; used by health checks.
pub async fn ping(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1").execute(pool).await.map(|_| ())
}
