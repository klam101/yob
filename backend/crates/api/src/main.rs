use anyhow::Context;
use yob_core::{config::Config, shutdown, telemetry};

mod error;
mod openapi;
mod routes;
mod state;

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

    let fetcher = yob_core::jobs::Fetcher::new(redis.clone())?;
    let app = routes::router(state::AppState { db, redis, fetcher }, &config)?;

    let listener = tokio::net::TcpListener::bind(&config.api_addr)
        .await
        .with_context(|| format!("binding {}", config.api_addr))?;
    tracing::info!("API listening on http://{}", config.api_addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown::signal())
        .await?;
    Ok(())
}
