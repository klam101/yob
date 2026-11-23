//! Background worker. For now: keeps the job database fresh by checking
//! company job boards on the schedule stored in Postgres. Later also
//! embeddings, fit scoring and document generation.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Context;
use tokio::{sync::Semaphore, task::JoinSet};
use yob_core::{
    config::Config,
    jobs::{Fetcher, discover, ingest, seed},
    shutdown, telemetry,
};

/// Companies (or discovered boards) checked at once. Per-host rate limits
/// in Redis keep any one job board from being hit too hard.
const SLOTS: usize = 6;
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
    // Apply changed parsing rules to stored jobs, without refetching them.
    let renormalized = ingest::renormalize(&db)
        .await
        .context("re-normalizing stored jobs")?;
    if renormalized > 0 {
        tracing::info!(jobs = renormalized, "re-derived fields of stored jobs");
    }

    // Finds new company boards slowly in the background; progress is saved
    // per page, so stopping it at any point is safe.
    let crawl = tokio::spawn(discover::run_common_crawl(db.clone(), fetcher.clone()));
    tracing::info!("worker started; checking companies as they come due");

    let stop = shutdown::signal();
    tokio::pin!(stop);
    let mut last_prune: Option<Instant> = None;
    // Each check runs as its own task holding one slot, so a slow board (a
    // large Workday tenant takes minutes) never holds up the others.
    let slots = Arc::new(Semaphore::new(SLOTS));
    let mut running = JoinSet::new();

    loop {
        while running.try_join_next().is_some() {}
        if last_prune.is_none_or(|t| t.elapsed() >= PRUNE_EVERY) {
            match ingest::prune(&db).await {
                Ok(n) => tracing::info!(cleared = n, "pruned old closed jobs"),
                Err(e) => tracing::warn!(error = %e, "prune failed"),
            }
            last_prune = Some(Instant::now());
        }

        let slot = tokio::select! {
            () = &mut stop => break,
            slot = slots.clone().acquire_owned() => slot?,
        };

        // Due companies come first; spare slots verify discovered boards.
        match ingest::claim_due(&db, 1).await {
            Ok(mut due) if !due.is_empty() => {
                let company = due.remove(0);
                let (db, fetcher) = (db.clone(), fetcher.clone());
                running.spawn(async move {
                    ingest::check_and_record(&db, &fetcher, &company).await;
                    drop(slot);
                });
                continue;
            }
            Ok(_) => {}
            Err(e) => tracing::error!(error = %e, "couldn't claim due companies"),
        }
        match discover::claim_candidates(&db, 1).await {
            Ok(mut due) if !due.is_empty() => {
                let candidate = due.remove(0);
                let (db, fetcher) = (db.clone(), fetcher.clone());
                running.spawn(async move {
                    discover::verify(&db, &fetcher, &candidate).await;
                    drop(slot);
                });
                continue;
            }
            Ok(_) => {}
            Err(e) => tracing::error!(error = %e, "couldn't claim discovered boards"),
        }

        // Nothing due: free the slot and wait a little.
        drop(slot);
        tokio::select! {
            () = &mut stop => break,
            () = tokio::time::sleep(IDLE) => {}
        }
    }

    // A check cut off here rolls back its transaction; its lease expires and
    // the company is checked again later.
    running.abort_all();
    crawl.abort();
    tracing::info!("worker stopped");
    Ok(())
}
