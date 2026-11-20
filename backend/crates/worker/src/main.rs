//! Background worker. For now: keeps the job database fresh by checking
//! company job boards on the schedule stored in Postgres. Later also
//! embeddings, fit scoring and document generation.

use std::time::{Duration, Instant};

use anyhow::Context;
use yob_core::{
    config::Config,
    jobs::{Fetcher, ingest, seed},
    shutdown, telemetry,
};

/// Companies claimed per round, and how many are checked at once.
const BATCH: i64 = 8;
const CONCURRENCY: usize = 4;
/// Pause when nothing is due. "Check now" in the app waits at most this long.
const IDLE: Duration = Duration::from_secs(15);
const PRUNE_EVERY: Duration = Duration::from_secs(24 * 60 * 60);

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
    let fetcher = Fetcher::new(redis)?;

    let added = seed::seed_companies(&db)
        .await
        .context("seeding companies")?;
    if added > 0 {
        tracing::info!(added, "added seed companies");
    }
    tracing::info!("worker started; checking companies as they come due");

    let stop = shutdown::signal();
    tokio::pin!(stop);
    let mut last_prune: Option<Instant> = None;

    loop {
        if last_prune.is_none_or(|t| t.elapsed() >= PRUNE_EVERY) {
            match ingest::prune(&db).await {
                Ok(n) => tracing::info!(cleared = n, "pruned old closed jobs"),
                Err(e) => tracing::warn!(error = %e, "prune failed"),
            }
            last_prune = Some(Instant::now());
        }

        // A check cut off by shutdown rolls back its transaction; its lease
        // expires and the company is checked again later.
        let checked = tokio::select! {
            () = &mut stop => break,
            result = ingest::run_due_checks(&db, &fetcher, BATCH, CONCURRENCY) => result,
        };

        let idle = match checked {
            Ok(0) => true,
            Ok(_) => false,
            Err(e) => {
                tracing::error!(error = %e, "couldn't claim due companies");
                true
            }
        };
        if idle {
            tokio::select! {
                () = &mut stop => break,
                () = tokio::time::sleep(IDLE) => {}
            }
        }
    }

    tracing::info!("worker stopped");
    Ok(())
}
