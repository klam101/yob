//! Keeping the job database fresh. The worker calls [`run_due_checks`] in a
//! loop; everything it needs (the schedule, what's open) lives in Postgres,
//! so a restart picks up where it left off without refetching.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use futures::{StreamExt, stream};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{
    http::{FetchError, Fetcher},
    model::{NormalizedJob, Platform},
    sources,
};

/// Jobs missing from this many checks in a row are closed. One bad response
/// from a job board can't wipe out a company's jobs.
const MISSES_BEFORE_CLOSE: i32 = 2;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DueCompany {
    pub id: Uuid,
    pub name: String,
    pub platform: String,
    pub board_token: String,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CheckStats {
    pub seen: i32,
    pub new: i32,
    pub closed: i32,
}

/// Claims up to `limit` companies that are due and checks them, `concurrency`
/// at a time. Returns how many were checked (0 means nothing was due).
pub async fn run_due_checks(
    pool: &PgPool,
    fetcher: &Fetcher,
    limit: i64,
    concurrency: usize,
) -> anyhow::Result<usize> {
    let due = claim_due(pool, limit).await?;
    let count = due.len();
    stream::iter(due)
        .for_each_concurrent(concurrency, |company| async move {
            check_and_record(pool, fetcher, &company).await;
        })
        .await;
    Ok(count)
}

/// Takes due companies and pushes their `next_check_at` forward as a lease,
/// so another worker (or this one, after a crash) won't check them twice.
async fn claim_due(pool: &PgPool, limit: i64) -> sqlx::Result<Vec<DueCompany>> {
    sqlx::query_as(
        "UPDATE companies SET next_check_at = now() + interval '20 minutes'
         WHERE id IN (
             SELECT id FROM companies
             WHERE active AND next_check_at <= now()
             ORDER BY next_check_at
             LIMIT $1
             FOR UPDATE SKIP LOCKED
         )
         RETURNING id, name, platform, board_token",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// Checks one company and records the outcome and next check time.
pub async fn check_and_record(pool: &PgPool, fetcher: &Fetcher, company: &DueCompany) {
    let started = Utc::now();
    let result = check_company(pool, fetcher, company).await;

    let recorded = match &result {
        Ok(stats) => {
            tracing::info!(
                company = %company.name, seen = stats.seen, new = stats.new, closed = stats.closed,
                "checked"
            );
            record_success(pool, company.id, started, *stats).await
        }
        Err(err) => {
            tracing::warn!(company = %company.name, error = %err, "check failed");
            record_failure(pool, company.id, started, &err.to_string()).await
        }
    };
    if let Err(e) = recorded {
        tracing::error!(company = %company.name, error = %e, "couldn't record check result");
    }
}

async fn check_company(
    pool: &PgPool,
    fetcher: &Fetcher,
    company: &DueCompany,
) -> anyhow::Result<CheckStats> {
    let platform = Platform::parse(&company.platform)
        .ok_or_else(|| anyhow::anyhow!("unknown platform {}", company.platform))?;
    let token = &company.board_token;

    let open: HashSet<String> = sqlx::query_scalar(
        "SELECT source_job_id FROM jobs WHERE company_id = $1 AND closed_at IS NULL",
    )
    .bind(company.id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .collect();

    // Cheap path (Greenhouse): list ids first, and only download full
    // postings when something new appeared.
    if let Some(ids) = sources::fetch_ids(fetcher, platform, token).await?
        && ids.is_subset(&open)
    {
        let ids: Vec<String> = ids.into_iter().collect();
        let mut tx = pool.begin().await?;
        mark_seen(&mut tx, company.id, &ids).await?;
        let closed = close_missing(&mut tx, company.id, &ids).await?;
        tx.commit().await?;
        return Ok(CheckStats {
            seen: ids.len() as i32,
            new: 0,
            closed,
        });
    }

    let mut jobs = sources::fetch_all(fetcher, platform, token).await?;
    let mut unique = HashSet::new();
    jobs.retain(|j| !j.source_job_id.is_empty() && unique.insert(j.source_job_id.clone()));

    let mut tx = pool.begin().await?;
    let mut new = 0;
    for job in &jobs {
        if upsert_job(&mut tx, company.id, job).await? {
            new += 1;
        }
    }
    let ids: Vec<String> = jobs.iter().map(|j| j.source_job_id.clone()).collect();
    let closed = close_missing(&mut tx, company.id, &ids).await?;
    tx.commit().await?;

    Ok(CheckStats {
        seen: jobs.len() as i32,
        new,
        closed,
    })
}

/// Inserts a new job or refreshes a known one. Returns true when inserted.
/// A new job whose content matches a closed job at the same company counts
/// as a repost; a closed job that reappears under the same id is reopened.
async fn upsert_job(
    tx: &mut Transaction<'_, Postgres>,
    company_id: Uuid,
    job: &NormalizedJob,
) -> sqlx::Result<bool> {
    let salary = job.salary.as_ref();
    sqlx::query_scalar(
        "INSERT INTO jobs (
             company_id, source_job_id, title, department, locations, workplace,
             employment_type, seniority, salary_min, salary_max, salary_currency,
             salary_period, posted_at, apply_url, description_html, description_text,
             content_hash, repost_count
         ) VALUES (
             $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17,
             COALESCE((SELECT max(repost_count) + 1 FROM jobs
                       WHERE company_id = $1 AND content_hash = $17 AND closed_at IS NOT NULL), 0)
         )
         ON CONFLICT (company_id, source_job_id) DO UPDATE SET
             title = EXCLUDED.title,
             department = EXCLUDED.department,
             locations = EXCLUDED.locations,
             workplace = EXCLUDED.workplace,
             employment_type = EXCLUDED.employment_type,
             seniority = EXCLUDED.seniority,
             salary_min = EXCLUDED.salary_min,
             salary_max = EXCLUDED.salary_max,
             salary_currency = EXCLUDED.salary_currency,
             salary_period = EXCLUDED.salary_period,
             apply_url = EXCLUDED.apply_url,
             description_html = EXCLUDED.description_html,
             description_text = EXCLUDED.description_text,
             content_hash = EXCLUDED.content_hash,
             last_seen_at = now(),
             missed_checks = 0,
             repost_count = jobs.repost_count + CASE WHEN jobs.closed_at IS NULL THEN 0 ELSE 1 END,
             closed_at = NULL
         RETURNING (xmax = 0)",
    )
    .bind(company_id)
    .bind(&job.source_job_id)
    .bind(&job.title)
    .bind(&job.department)
    .bind(&job.locations)
    .bind(job.workplace.as_str())
    .bind(job.employment_type.as_str())
    .bind(job.seniority.as_str())
    .bind(salary.map(|s| s.min as i32))
    .bind(salary.map(|s| s.max as i32))
    .bind(salary.map(|s| s.currency.clone()))
    .bind(salary.map(|s| s.period.as_str()))
    .bind(job.posted_at)
    .bind(&job.apply_url)
    .bind(&job.description_html)
    .bind(&job.description_text)
    .bind(&job.content_hash)
    .fetch_one(&mut **tx)
    .await
}

async fn mark_seen(
    tx: &mut Transaction<'_, Postgres>,
    company_id: Uuid,
    ids: &[String],
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE jobs SET last_seen_at = now(), missed_checks = 0
         WHERE company_id = $1 AND closed_at IS NULL AND source_job_id = ANY($2)",
    )
    .bind(company_id)
    .bind(ids)
    .execute(&mut **tx)
    .await
    .map(|_| ())
}

/// Counts a miss for open jobs not in `seen_ids`, closing those that hit
/// [`MISSES_BEFORE_CLOSE`]. Returns how many were closed.
async fn close_missing(
    tx: &mut Transaction<'_, Postgres>,
    company_id: Uuid,
    seen_ids: &[String],
) -> sqlx::Result<i32> {
    let closed: i64 = sqlx::query_scalar(
        "WITH missed AS (
             UPDATE jobs SET
                 missed_checks = missed_checks + 1,
                 closed_at = CASE WHEN missed_checks + 1 >= $3 THEN now() END
             WHERE company_id = $1 AND closed_at IS NULL AND NOT (source_job_id = ANY($2))
             RETURNING closed_at
         )
         SELECT count(*) FILTER (WHERE closed_at IS NOT NULL) FROM missed",
    )
    .bind(company_id)
    .bind(seen_ids)
    .bind(MISSES_BEFORE_CLOSE)
    .fetch_one(&mut **tx)
    .await?;
    Ok(closed as i32)
}

/// Schedules the next check from how actively the company posts:
/// a job posted in the last week → 15 minutes, last month → 6 hours,
/// otherwise daily. ±10% jitter spreads checks out.
async fn record_success(
    pool: &PgPool,
    company_id: Uuid,
    started: DateTime<Utc>,
    stats: CheckStats,
) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "WITH activity AS (
             SELECT max(posted_at) AS latest FROM jobs WHERE company_id = $1 AND closed_at IS NULL
         ), cadence AS (
             SELECT CASE
                 WHEN latest > now() - interval '7 days' THEN 15
                 WHEN latest > now() - interval '30 days' THEN 360
                 ELSE 1440
             END AS minutes FROM activity
         )
         UPDATE companies SET
             last_checked_at = now(),
             last_status = 'ok',
             last_error = NULL,
             consecutive_failures = 0,
             check_interval_min = (SELECT minutes FROM cadence),
             next_check_at = now() + make_interval(mins => (SELECT minutes FROM cadence))
                                     * (0.9 + random() * 0.2)
         WHERE id = $1",
    )
    .bind(company_id)
    .execute(&mut *tx)
    .await?;
    insert_check(&mut tx, company_id, started, "ok", stats, None).await?;
    tx.commit().await
}

/// Backs off after failures: 15 min, 30, 60… capped at a day.
async fn record_failure(
    pool: &PgPool,
    company_id: Uuid,
    started: DateTime<Utc>,
    error: &str,
) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE companies SET
             last_checked_at = now(),
             last_status = 'error',
             last_error = $2,
             consecutive_failures = consecutive_failures + 1,
             next_check_at = now() + least(
                 interval '24 hours',
                 interval '15 minutes' * power(2, least(consecutive_failures, 10))
             )
         WHERE id = $1",
    )
    .bind(company_id)
    .bind(error)
    .execute(&mut *tx)
    .await?;
    insert_check(
        &mut tx,
        company_id,
        started,
        "error",
        CheckStats::default(),
        Some(error),
    )
    .await?;
    tx.commit().await
}

async fn insert_check(
    tx: &mut Transaction<'_, Postgres>,
    company_id: Uuid,
    started: DateTime<Utc>,
    status: &str,
    stats: CheckStats,
    error: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO company_checks (company_id, started_at, status, jobs_seen, jobs_new, jobs_closed, error)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(company_id)
    .bind(started)
    .bind(status)
    .bind(stats.seen)
    .bind(stats.new)
    .bind(stats.closed)
    .bind(error)
    .execute(&mut **tx)
    .await
    .map(|_| ())
}

/// Clears descriptions of jobs closed more than 90 days ago (titles, dates and
/// hashes stay for repost history) and trims old check history.
pub async fn prune(pool: &PgPool) -> sqlx::Result<u64> {
    let cleared = sqlx::query(
        "UPDATE jobs SET description_html = NULL, description_text = NULL
         WHERE closed_at < now() - interval '90 days' AND description_text IS NOT NULL",
    )
    .execute(pool)
    .await?
    .rows_affected();
    sqlx::query("DELETE FROM company_checks WHERE started_at < now() - interval '30 days'")
        .execute(pool)
        .await?;
    Ok(cleared)
}

/// Verifies a board exists and returns how many jobs it has open.
pub async fn probe_board(
    fetcher: &Fetcher,
    platform: Platform,
    token: &str,
) -> Result<usize, FetchError> {
    match sources::fetch_ids(fetcher, platform, token).await? {
        Some(ids) => Ok(ids.len()),
        None => Ok(sources::fetch_all(fetcher, platform, token).await?.len()),
    }
}
