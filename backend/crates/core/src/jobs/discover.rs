//! Finding more companies. Adapters are per platform, so coverage grows by
//! finding more company job boards. Leads ("candidates") come from:
//!
//! - the Common Crawl URL index ([`run_common_crawl`]), walked slowly in the
//!   background and resumed where it stopped;
//! - board links inside job postings we already read (`ingest`);
//! - URLs pasted on the Job sources page or sent with `dev.sh discover`.
//!
//! Each candidate is verified once through the rate-limited fetcher
//! ([`verify`]) and becomes a company only if it has open jobs.

use std::{collections::HashSet, time::Duration};

use chrono::{DateTime, Utc};

use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    http::{FetchError, Fetcher},
    ingest,
    model::Platform,
    sources, store,
};

/// URL patterns searched in each crawl.
pub const PATTERNS: &[&str] = &[
    "boards.greenhouse.io/*",
    "job-boards.greenhouse.io/*",
    "jobs.lever.co/*",
    "jobs.ashbyhq.com/*",
    "jobs.smartrecruiters.com/*",
    "apply.workable.com/*",
    "*.recruitee.com",
    "*.bamboohr.com",
    "*.myworkdaysite.com",
    // Largest last: millions of Workday job pages, from a few thousand tenants.
    "*.myworkdayjobs.com",
];
/// How many of the newest crawls to read. Older crawls add few live boards.
const CRAWLS: usize = 3;
/// Pause between index pages, to stay a polite client of a shared service.
const PAGE_PAUSE: Duration = Duration::from_secs(3);
/// Waits between attempts when the index is overloaded (it often is).
const BACKOFF: &[u64] = &[15, 30, 60, 120, 240];
/// After a full pass, look for a new crawl this often.
const ROUND_PAUSE: Duration = Duration::from_secs(24 * 60 * 60);
/// After a pass that failed (index down), try again this soon.
const RETRY_PAUSE: Duration = Duration::from_secs(30 * 60);

/// Saves leads as pending candidates, skipping boards already tracked or
/// already known. Returns how many were new.
pub async fn add_candidates(
    pool: &PgPool,
    boards: &[(Platform, String)],
    source: &str,
) -> sqlx::Result<u64> {
    let mut seen = HashSet::new();
    let (platforms, tokens): (Vec<String>, Vec<String>) = boards
        .iter()
        .filter(|(p, t)| sources::plausible_token(t) && seen.insert((*p, t.to_lowercase())))
        .map(|(p, t)| (p.as_str().to_owned(), t.clone()))
        .unzip();
    if tokens.is_empty() {
        return Ok(0);
    }
    Ok(sqlx::query(
        "INSERT INTO company_candidates (platform, token, source)
         SELECT b.platform, b.token, $3 FROM unnest($1::text[], $2::text[]) AS b(platform, token)
         WHERE NOT EXISTS (
             SELECT 1 FROM companies c
             WHERE c.platform = b.platform AND lower(c.board_token) = lower(b.token)
         )
         ON CONFLICT (platform, lower(token)) DO NOTHING",
    )
    .bind(platforms)
    .bind(tokens)
    .bind(source)
    .execute(pool)
    .await?
    .rows_affected())
}

#[derive(sqlx::FromRow)]
pub struct Candidate {
    id: i64,
    platform: String,
    pub token: String,
}

/// Claims up to `limit` candidates that are due for a check, with a
/// 20-minute lease. The worker verifies them with [`verify`] when it has a
/// free slot and no company is due.
pub async fn claim_candidates(pool: &PgPool, limit: i64) -> sqlx::Result<Vec<Candidate>> {
    sqlx::query_as(
        "UPDATE company_candidates SET next_check_at = now() + interval '20 minutes'
         WHERE id IN (
             SELECT id FROM company_candidates
             WHERE status IN ('pending', 'rejected') AND next_check_at <= now()
             ORDER BY next_check_at
             LIMIT $1
             FOR UPDATE SKIP LOCKED
         )
         RETURNING id, platform, token",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// Probes one candidate board: a company if it has open jobs, otherwise
/// rejected for 30 days.
pub async fn verify(pool: &PgPool, fetcher: &Fetcher, c: &Candidate) {
    if let Err(e) = verify_one(pool, fetcher, c).await {
        tracing::warn!(token = %c.token, error = %e, "couldn't record candidate result");
    }
}

async fn verify_one(pool: &PgPool, fetcher: &Fetcher, c: &Candidate) -> sqlx::Result<()> {
    let Some(platform) = Platform::parse(&c.platform) else {
        return reject(pool, c.id, "unknown platform", None).await;
    };
    match ingest::probe_board(fetcher, platform, &c.token).await {
        Ok(0) => reject(pool, c.id, "no open jobs", Some(0)).await,
        Ok(jobs) => {
            let company_id = match store::find_company(pool, platform, &c.token).await? {
                Some(id) => id,
                None => {
                    let name = sources::board_name(fetcher, platform, &c.token)
                        .await
                        .unwrap_or_else(|| sources::display_name(&c.token));
                    let id = store::insert_company(pool, &name, platform, &c.token).await?;
                    tracing::info!(company = %name, platform = %c.platform, jobs, "discovered company");
                    id
                }
            };
            mark_added(pool, c.id, company_id, jobs as i32).await
        }
        // A missing board, or a token that isn't one.
        Err(e @ (FetchError::NotFound | FetchError::Invalid(_))) => {
            reject(pool, c.id, &e.to_string(), None).await
        }
        // Rate limits and outages: try again in an hour.
        Err(e) => sqlx::query(
            "UPDATE company_candidates
                 SET error = $2, next_check_at = now() + interval '1 hour'
                 WHERE id = $1",
        )
        .bind(c.id)
        .bind(e.to_string())
        .execute(pool)
        .await
        .map(|_| ()),
    }
}

async fn mark_added(pool: &PgPool, id: i64, company_id: Uuid, jobs: i32) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE company_candidates
         SET status = 'added', checked_at = now(), jobs_found = $3, error = NULL, company_id = $2
         WHERE id = $1",
    )
    .bind(id)
    .bind(company_id)
    .bind(jobs)
    .execute(pool)
    .await
    .map(|_| ())
}

/// Not a live board (yet). Boards that open roles later get another look
/// in 30 days.
async fn reject(pool: &PgPool, id: i64, error: &str, jobs: Option<i32>) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE company_candidates
         SET status = 'rejected', checked_at = now(), jobs_found = $3, error = $2,
             next_check_at = now() + interval '30 days'
         WHERE id = $1",
    )
    .bind(id)
    .bind(error)
    .bind(jobs)
    .execute(pool)
    .await
    .map(|_| ())
}

// --- Common Crawl -----------------------------------------------------------

#[derive(Deserialize)]
struct Crawl {
    id: String,
    #[serde(rename = "cdx-api")]
    cdx_api: String,
}

#[derive(Deserialize)]
struct PageCount {
    pages: i32,
}

#[derive(Deserialize)]
struct Capture {
    url: String,
}

/// Runs forever: reads the newest crawls' URL indexes for job-board links,
/// saving progress after every page, then waits for the next crawl.
pub async fn run_common_crawl(pool: PgPool, fetcher: Fetcher) {
    loop {
        let pause = match crawl_round(&pool, &fetcher).await {
            Ok(found) => {
                tracing::info!(found, "Common Crawl discovery is up to date");
                ROUND_PAUSE
            }
            Err(e) => {
                tracing::warn!(error = %e, "Common Crawl discovery paused; will resume");
                RETRY_PAUSE
            }
        };
        tokio::time::sleep(pause).await;
    }
}

async fn crawl_round(pool: &PgPool, fetcher: &Fetcher) -> anyhow::Result<u64> {
    let body = cc_get(fetcher, "https://index.commoncrawl.org/collinfo.json")
        .await?
        .ok_or_else(|| anyhow::anyhow!("crawl list not found"))?;
    let crawls: Vec<Crawl> = serde_json::from_str(&body)?;

    let mut found = 0;
    for crawl in crawls.iter().take(CRAWLS) {
        for pattern in PATTERNS {
            found += crawl_pattern(pool, fetcher, crawl, pattern).await?;
        }
    }
    Ok(found)
}

async fn crawl_pattern(
    pool: &PgPool,
    fetcher: &Fetcher,
    crawl: &Crawl,
    pattern: &str,
) -> anyhow::Result<u64> {
    sqlx::query(
        "INSERT INTO discovery_progress (crawl, pattern) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(&crawl.id)
    .bind(pattern)
    .execute(pool)
    .await?;
    let (pages, mut next_page): (Option<i32>, i32) = sqlx::query_as(
        "SELECT pages, next_page FROM discovery_progress WHERE crawl = $1 AND pattern = $2",
    )
    .bind(&crawl.id)
    .bind(pattern)
    .fetch_one(pool)
    .await?;

    let pages = match pages {
        Some(p) => p,
        None => {
            let url = format!(
                "{}?url={pattern}&output=json&showNumPages=true",
                crawl.cdx_api
            );
            let pages = match cc_get(fetcher, &url).await? {
                Some(body) => serde_json::from_str::<PageCount>(&body)?.pages,
                None => 0,
            };
            sqlx::query(
                "UPDATE discovery_progress SET pages = $3, updated_at = now()
                 WHERE crawl = $1 AND pattern = $2",
            )
            .bind(&crawl.id)
            .bind(pattern)
            .bind(pages)
            .execute(pool)
            .await?;
            tokio::time::sleep(PAGE_PAUSE).await;
            pages
        }
    };

    let mut found = 0;
    while next_page < pages {
        let url = format!(
            "{}?url={pattern}&output=json&fl=url&page={next_page}",
            crawl.cdx_api
        );
        let boards = match cc_get(fetcher, &url).await? {
            Some(body) => boards_in_index_page(&body),
            None => Vec::new(),
        };
        let added = add_candidates(pool, &boards, "commoncrawl").await?;
        found += added;
        next_page += 1;
        sqlx::query(
            "UPDATE discovery_progress
             SET next_page = $3, found = found + $4, updated_at = now()
             WHERE crawl = $1 AND pattern = $2",
        )
        .bind(&crawl.id)
        .bind(pattern)
        .bind(next_page)
        .bind(added as i32)
        .execute(pool)
        .await?;
        tracing::debug!(crawl = %crawl.id, pattern, page = next_page, pages, added, "index page read");
        tokio::time::sleep(PAGE_PAUSE).await;
    }
    Ok(found)
}

/// Distinct boards in one page of index results (one JSON object per line).
pub fn boards_in_index_page(body: &str) -> Vec<(Platform, String)> {
    let mut seen = HashSet::new();
    body.lines()
        .filter_map(|line| serde_json::from_str::<Capture>(line).ok())
        .filter_map(|c| sources::parse_board_url(&c.url))
        .filter(|(p, t)| sources::plausible_token(t) && seen.insert((*p, t.to_lowercase())))
        .collect()
}

/// GET with Common Crawl's slow retry schedule. `None` means 404 ("no
/// captures"), which is an answer, not a failure.
async fn cc_get(fetcher: &Fetcher, url: &str) -> Result<Option<String>, FetchError> {
    let mut attempt = 0;
    loop {
        match fetcher.get_text_once(url).await {
            Ok(body) => return Ok(Some(body)),
            Err(FetchError::NotFound) => return Ok(None),
            Err(e) => {
                let Some(wait) = BACKOFF.get(attempt) else {
                    return Err(e);
                };
                tracing::debug!(url, error = %e, wait, "Common Crawl busy; retrying");
                tokio::time::sleep(Duration::from_secs(*wait)).await;
                attempt += 1;
            }
        }
    }
}

// --- stats -------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct DiscoveryStats {
    pub pending: i64,
    pub added: i64,
    pub rejected: i64,
    pub added_last_week: i64,
    pub index_pages_read: i64,
    /// Null until every page count is known.
    pub index_pages_total: Option<i64>,
    pub last_index_read_at: Option<DateTime<Utc>>,
}

pub async fn stats(pool: &PgPool) -> sqlx::Result<DiscoveryStats> {
    let (pending, added, rejected, added_last_week): (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT count(*) FILTER (WHERE status = 'pending'),
                count(*) FILTER (WHERE status = 'added'),
                count(*) FILTER (WHERE status = 'rejected'),
                count(*) FILTER (WHERE status = 'added' AND checked_at > now() - interval '7 days')
         FROM company_candidates",
    )
    .fetch_one(pool)
    .await?;
    let (read, total, unknown, last): (Option<i64>, Option<i64>, i64, Option<DateTime<Utc>>) =
        sqlx::query_as(
            "SELECT sum(next_page)::bigint, sum(pages)::bigint,
                    count(*) FILTER (WHERE pages IS NULL), max(updated_at)
             FROM discovery_progress",
        )
        .fetch_one(pool)
        .await?;
    Ok(DiscoveryStats {
        pending,
        added,
        rejected,
        added_last_week,
        index_pages_read: read.unwrap_or(0),
        index_pages_total: if unknown == 0 { total } else { None },
        last_index_read_at: last,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_page_lines() {
        let body = r#"{"url": "https://jobs.lever.co/acme/1234-abcd"}
{"url": "https://jobs.lever.co/acme/5678"}
{"url": "https://jobs.lever.co/robots.txt"}
{"url": "https://boards.greenhouse.io/embed/job_board?for=beta"}
not json"#;
        assert_eq!(
            boards_in_index_page(body),
            vec![
                (Platform::Lever, "acme".to_owned()),
                (Platform::Greenhouse, "beta".to_owned()),
            ]
        );
    }
}
