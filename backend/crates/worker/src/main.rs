//! Background worker: feed checks, embeddings, fit scoring, document
//! generation. For now it only connects to Postgres and Redis and waits.

use anyhow::Context;
use deadpool_redis::redis;
use yob_core::{config::Config, shutdown, telemetry};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    telemetry::init();
    let config = Config::from_env()?;

    let db = yob_db::connect(&config.database_url)
        .await
        .context("connecting to Postgres")?;
    yob_db::migrate(&db).await.context("running migrations")?;

    let redis = deadpool_redis::Config::from_url(&config.redis_url)
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .context("creating Redis pool")?;
    let mut conn = redis.get().await.context("connecting to Redis")?;
    redis::cmd("PING")
        .query_async::<String>(&mut conn)
        .await
        .context("pinging Redis")?;
    drop(conn);

    tracing::info!("worker connected to Postgres and Redis; no jobs registered yet, idling");
    shutdown::signal().await;
    Ok(())
}
