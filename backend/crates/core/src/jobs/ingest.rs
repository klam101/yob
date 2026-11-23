//! Keeping the job database fresh. The worker calls [`run_due_checks`] in a
//! loop; everything it needs (the schedule, what's open) lives in Postgres,
//! so a restart picks up where it left off without refetching.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use futures::{StreamExt, stream};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{
    discover,
    http::{FetchError, Fetcher},
    model::{NormalizedJob, Platform},
    normalize, sources,
};

/// Jobs missing from this many checks in a row are closed. One bad response
/// from a job board can't wipe out a company's jobs.
const MISSES_BEFORE_CLOSE: i32 = 2;

/// Jobs written per INSERT statement.
const UPSERT_BATCH: usize = 500;

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
        // In-scope jobs missing a description (they were out of scope under
        // older rules) count as new, so their details are fetched again.
        "SELECT source_job_id FROM jobs WHERE company_id = $1 AND closed_at IS NULL
           AND NOT (in_scope AND description_text IS NULL)",
    )
    .bind(company.id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .collect();

    // Full listings carry every job; id listings need details for new ids
    // only, so a board with nothing new costs one light request.
    let (seen_ids, mut jobs): (Vec<String>, Vec<NormalizedJob>) =
        match sources::list(fetcher, platform, token).await? {
            sources::Listing::Full(jobs) => {
                (jobs.iter().map(|j| j.source_job_id.clone()).collect(), jobs)
            }
            sources::Listing::Ids(ids) => {
                let new: HashSet<String> = ids.difference(&open).cloned().collect();
                let mut jobs = sources::details(fetcher, platform, token, &new).await?;
                jobs.retain(|j| ids.contains(&j.source_job_id));
                (ids.into_iter().collect(), jobs)
            }
        };
    let mut unique = HashSet::new();
    jobs.retain(|j| !j.source_job_id.is_empty() && unique.insert(j.source_job_id.clone()));

    // Board links inside new postings are leads for discovery.
    let mut leads: Vec<(Platform, String)> = Vec::new();
    for job in jobs.iter().filter(|j| !open.contains(&j.source_job_id)) {
        let text = format!("{} {}", job.apply_url, job.description_html);
        for lead in sources::board_links(&text) {
            let own = lead.0 == platform && lead.1.eq_ignore_ascii_case(token);
            if !own && !leads.contains(&lead) {
                leads.push(lead);
            }
        }
    }

    let mut tx = pool.begin().await?;
    let mut new = 0;
    for chunk in jobs.chunks(UPSERT_BATCH) {
        new += upsert_jobs(&mut tx, company.id, chunk).await?;
    }
    mark_seen(&mut tx, company.id, &seen_ids).await?;
    let closed = close_missing(&mut tx, company.id, &seen_ids).await?;
    tx.commit().await?;

    if !leads.is_empty()
        && let Err(e) = discover::add_candidates(pool, &leads, "link").await
    {
        tracing::warn!(error = %e, "couldn't save discovered boards");
    }

    Ok(CheckStats {
        seen: seen_ids.len() as i32,
        new,
        closed,
    })
}

/// One job as a JSON object for [`upsert_jobs`]. Out-of-scope jobs are kept
/// without their description: enough to recognize them on the next check.
fn job_json(job: &NormalizedJob) -> serde_json::Value {
    let salary = job.salary.as_ref();
    let in_scope = normalize::us_scope(&job.locations);
    serde_json::json!({
        "source_job_id": job.source_job_id,
        "title": job.title,
        "department": job.department,
        "locations": job.locations,
        "workplace": job.workplace.as_str(),
        "employment_type": job.employment_type.as_str(),
        "seniority": job.seniority.as_str(),
        "salary_min": salary.map(|s| s.min),
        "salary_max": salary.map(|s| s.max),
        "salary_currency": salary.map(|s| s.currency.clone()),
        "salary_period": salary.map(|s| s.period.as_str()),
        "salary_source": salary.map(|_| if job.salary_from_text { "text" } else { "structured" }),
        "posted_at": job.posted_at,
        "apply_url": job.apply_url,
        "description_html": in_scope.then_some(&job.description_html),
        "description_text": in_scope.then_some(&job.description_text),
        "content_hash": job.content_hash,
        "in_scope": in_scope,
    })
}

/// Inserts new jobs and refreshes changed ones in one statement. Returns how
/// many were inserted. Unchanged open jobs are skipped (`mark_seen` covers
/// them), so a re-check doesn't rewrite every description.
///
/// A new job whose content matches a closed job at the same company counts
/// as a repost; a closed job that reappears under the same id is reopened.
async fn upsert_jobs(
    tx: &mut Transaction<'_, Postgres>,
    company_id: Uuid,
    jobs: &[NormalizedJob],
) -> sqlx::Result<i32> {
    let batch = serde_json::Value::Array(jobs.iter().map(job_json).collect()).to_string();
    let inserted: Vec<bool> = sqlx::query_scalar(
        "INSERT INTO jobs (
             company_id, source_job_id, title, department, locations, workplace,
             employment_type, seniority, salary_min, salary_max, salary_currency,
             salary_period, salary_source, posted_at, apply_url, description_html,
             description_text, content_hash, in_scope, normalize_version, repost_count
         )
         SELECT $1, x.source_job_id, x.title, x.department, x.locations, x.workplace,
                x.employment_type, x.seniority, x.salary_min, x.salary_max, x.salary_currency,
                x.salary_period, x.salary_source, x.posted_at, x.apply_url, x.description_html,
                x.description_text, x.content_hash, x.in_scope, $3,
                COALESCE((SELECT max(p.repost_count) + 1 FROM jobs p
                          WHERE p.company_id = $1 AND p.content_hash = x.content_hash
                            AND p.closed_at IS NOT NULL), 0)
         FROM jsonb_to_recordset($2::jsonb) AS x(
             source_job_id text, title text, department text, locations text[],
             workplace text, employment_type text, seniority text, salary_min integer,
             salary_max integer, salary_currency text, salary_period text,
             salary_source text, posted_at timestamptz, apply_url text,
             description_html text, description_text text, content_hash text,
             in_scope boolean
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
             salary_source = EXCLUDED.salary_source,
             apply_url = EXCLUDED.apply_url,
             description_html = EXCLUDED.description_html,
             description_text = EXCLUDED.description_text,
             content_hash = EXCLUDED.content_hash,
             in_scope = EXCLUDED.in_scope,
             normalize_version = EXCLUDED.normalize_version,
             last_seen_at = now(),
             missed_checks = 0,
             repost_count = jobs.repost_count + CASE WHEN jobs.closed_at IS NULL THEN 0 ELSE 1 END,
             closed_at = NULL
         WHERE jobs.closed_at IS NOT NULL
            OR (jobs.description_text IS NULL AND EXCLUDED.description_text IS NOT NULL)
            OR jobs.content_hash IS DISTINCT FROM EXCLUDED.content_hash
            OR jobs.normalize_version IS DISTINCT FROM EXCLUDED.normalize_version
            OR (jobs.title, jobs.department, jobs.locations, jobs.workplace, jobs.employment_type,
                jobs.salary_min, jobs.salary_max, jobs.salary_currency, jobs.apply_url)
               IS DISTINCT FROM
               (EXCLUDED.title, EXCLUDED.department, EXCLUDED.locations, EXCLUDED.workplace,
                EXCLUDED.employment_type, EXCLUDED.salary_min, EXCLUDED.salary_max,
                EXCLUDED.salary_currency, EXCLUDED.apply_url)
         RETURNING (xmax = 0)",
    )
    .bind(company_id)
    .bind(batch)
    .bind(normalize::NORMALIZE_VERSION)
    .fetch_all(&mut **tx)
    .await?;
    Ok(inserted.into_iter().filter(|&new| new).count() as i32)
}

/// Records that open jobs are still listed. Rows touched in the last hour
/// are skipped, so frequent checks don't rewrite the whole table.
async fn mark_seen(
    tx: &mut Transaction<'_, Postgres>,
    company_id: Uuid,
    ids: &[String],
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE jobs SET last_seen_at = now(), missed_checks = 0
         WHERE company_id = $1 AND closed_at IS NULL AND source_job_id = ANY($2)
           AND (missed_checks > 0 OR last_seen_at < now() - interval '1 hour')",
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
    Ok(sources::list(fetcher, platform, token).await?.len())
}

#[derive(sqlx::FromRow)]
struct StoredJob {
    id: Uuid,
    title: String,
    locations: Vec<String>,
    description_text: Option<String>,
    salary_source: Option<String>,
}

/// Re-derives fields from stored jobs after the rules in `normalize.rs`
/// change (see [`normalize::NORMALIZE_VERSION`]): seniority, US scope, and
/// salaries that were parsed from text. Structured salaries are left alone.
/// Returns how many jobs were updated.
pub async fn renormalize(pool: &PgPool) -> sqlx::Result<u64> {
    let mut updated = 0;
    loop {
        let rows: Vec<StoredJob> = sqlx::query_as(
            "SELECT id, title, locations, description_text, salary_source FROM jobs
             WHERE normalize_version < $1 LIMIT 1000",
        )
        .bind(normalize::NORMALIZE_VERSION)
        .fetch_all(pool)
        .await?;
        if rows.is_empty() {
            return Ok(updated);
        }

        let batch: Vec<serde_json::Value> = rows
            .iter()
            .map(|r| {
                let text = r.description_text.as_deref().unwrap_or("");
                let in_scope = normalize::us_scope(&r.locations);
                // Re-parse salaries that came from text, or jobs with none
                // yet, as long as the text is still stored.
                let resalary = r.salary_source.as_deref() != Some("structured")
                    && r.description_text.is_some();
                let salary = resalary.then(|| normalize::parse_salary(text)).flatten();
                serde_json::json!({
                    "id": r.id,
                    "seniority": normalize::infer_seniority(&r.title).as_str(),
                    "in_scope": in_scope,
                    "resalary": resalary,
                    "salary_min": salary.as_ref().map(|s| s.min),
                    "salary_max": salary.as_ref().map(|s| s.max),
                    "salary_currency": salary.as_ref().map(|s| s.currency.clone()),
                    "salary_period": salary.as_ref().map(|s| s.period.as_str()),
                    "salary_source": salary.as_ref().map(|_| "text"),
                })
            })
            .collect();

        updated += sqlx::query(
            "UPDATE jobs j SET
                 seniority = x.seniority,
                 in_scope = x.in_scope,
                 normalize_version = $2,
                 salary_min = CASE WHEN x.resalary THEN x.salary_min ELSE j.salary_min END,
                 salary_max = CASE WHEN x.resalary THEN x.salary_max ELSE j.salary_max END,
                 salary_currency = CASE WHEN x.resalary THEN x.salary_currency ELSE j.salary_currency END,
                 salary_period = CASE WHEN x.resalary THEN x.salary_period ELSE j.salary_period END,
                 salary_source = CASE WHEN x.resalary THEN x.salary_source ELSE j.salary_source END,
                 description_html = CASE WHEN x.in_scope THEN j.description_html END,
                 description_text = CASE WHEN x.in_scope THEN j.description_text END
             FROM jsonb_to_recordset($1::jsonb) AS x(
                 id uuid, seniority text, in_scope boolean, resalary boolean,
                 salary_min integer, salary_max integer, salary_currency text,
                 salary_period text, salary_source text
             )
             WHERE j.id = x.id",
        )
        .bind(serde_json::Value::Array(batch).to_string())
        .bind(normalize::NORMALIZE_VERSION)
        .execute(pool)
        .await?
        .rows_affected();
    }
}
