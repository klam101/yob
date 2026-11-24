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

    if std::env::args().nth(1).as_deref() == Some("reset-password") {
        return reset_password(&db, &redis).await;
    }

    let fetcher = yob_core::jobs::Fetcher::new(redis.clone())?;
    let profile_key = config.profile_key.clone().context(
        "PROFILE_ENCRYPTION_KEY is not set. Run ./scripts/dev.sh up (or setup) to add one to .env",
    )?;
    let files = yob_core::profile::resume::Files::local(&config.files_dir)
        .with_context(|| format!("opening file storage at {}", config.files_dir))?;
    let app = routes::router(
        state::AppState {
            db,
            redis,
            fetcher,
            ollama_url: config.ollama_url.clone(),
            profile_key,
            files,
        },
        &config,
    )?;

    let listener = tokio::net::TcpListener::bind(&config.api_addr)
        .await
        .with_context(|| format!("binding {}", config.api_addr))?;
    tracing::info!("API listening on http://{}", config.api_addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown::signal())
        .await?;
    Ok(())
}

/// `api reset-password`: reads the email, then the new password, one per line
/// on stdin (`./scripts/dev.sh reset-password` prompts for them). Ends every
/// session for that account.
async fn reset_password(db: &yob_db::PgPool, redis: &deadpool_redis::Pool) -> anyhow::Result<()> {
    let mut lines = std::io::stdin().lines();
    let email = lines.next().context("expected the email on stdin")??;
    let password = lines
        .next()
        .context("expected the new password on stdin")??;
    yob_core::auth::set_password(db, redis, &email, &password)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!(
        "Password updated for {}. Every session was logged out.",
        email.trim()
    );
    Ok(())
}
