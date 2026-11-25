//! Job boards with public APIs: Himalayas, Remotive, Remote OK and (with a
//! key) USAJOBS. Boards mostly repeat jobs we already read from employers'
//! own boards, so every listing is matched first:
//!
//! 1. **Same job at a tracked employer** (same company and title, same
//!    workplace or an overlapping location): the employer's copy wins and
//!    the board becomes an attribution in `job_sources`. A job another board
//!    already added counts too.
//! 2. **Employer tracked, job not found:** the employer's own board is
//!    trusted (the listing is probably closed or retitled), so nothing is
//!    added. The listing is still recorded, and matched again next time.
//! 3. **No match:** the listing becomes a job of its own (`jobs.feed`), under
//!    an inactive "external" company. If the employer's board is tracked
//!    later, the next check matches it and closes the board's copy.
//!
//! Only US-eligible listings with a company, a title and an https link are
//! kept, and known reposting services are dropped.
//!
//! A big board (Himalayas: about 6,000 pages) is read two ways per check:
//! the newest pages until one has nothing new, then the next stretch of a
//! full pass that resumes where the previous check stopped. When a pass
//! reaches the end, listings it didn't see are removed and the board's own
//! jobs without a listing close.

pub mod himalayas;
pub mod remoteok;
pub mod remotive;
pub mod usajobs;

use std::collections::HashSet;

use anyhow::Context;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{
    discover,
    http::{FetchError, Fetcher},
    ingest,
    model::{Feed, NormalizedJob, Salary, Workplace},
    normalize, sources,
};
use crate::config::FeedKeys;

/// How long a claimed board stays reserved for its check.
const LEASE_MINUTES: i32 = 60;
/// Newest pages read per check before the full pass continues.
const HEAD_PAGES: usize = 50;
/// Wait after a board cuts a check off with a 429. Himalayas allows about
/// 100 pages and then refuses for hours (November 2026), so retrying every
/// 30 minutes only spends one request to be refused again.
const RATE_LIMITED_WAIT_MINUTES: i32 = 180;

/// One listing on a job board.
#[derive(Debug, Clone)]
pub struct FeedPosting {
    /// The board's id for the listing.
    pub external_id: String,
    pub company: String,
    /// The company's page on the board, if it has one.
    pub company_url: Option<String>,
    /// The listing's page on the board (the attribution link).
    pub url: String,
    /// Open to people in the US (the board's own country list, or the
    /// location rules).
    pub us_eligible: bool,
    pub job: NormalizedJob,
}

pub struct Page {
    pub postings: Vec<FeedPosting>,
    /// Cursor for the next page; none on the last one.
    pub next: Option<String>,
}

/// The fields a board gives, turned into a [`NormalizedJob`].
pub(crate) struct JobParts {
    pub id: String,
    pub title: String,
    pub department: Option<String>,
    pub locations: Vec<String>,
    /// When the board says; otherwise inferred.
    pub workplace: Option<Workplace>,
    /// Employment type words ("Full Time", "contract").
    pub employment: Option<String>,
    pub salary: Option<Salary>,
    pub posted_at: DateTime<Utc>,
    pub apply_url: String,
    /// The description as the board sends it (HTML, cleaned here).
    pub html: String,
}

impl JobParts {
    pub(crate) fn build(self) -> NormalizedJob {
        let (description_html, description_text) = normalize::clean_html(&self.html, false);
        let title = self.title.split_whitespace().collect::<Vec<_>>().join(" ");
        let workplace = self.workplace.unwrap_or_else(|| {
            normalize::infer_workplace(&title, &self.locations, &description_text)
        });
        let (salary, salary_from_text) = normalize::salary_or_text(self.salary, &description_text);
        NormalizedJob {
            source_job_id: self.id,
            workplace,
            employment_type: normalize::infer_employment(&title, self.employment.as_deref()),
            seniority: normalize::infer_seniority(&title),
            salary,
            salary_from_text,
            content_hash: normalize::content_hash(&title, &self.locations, &description_text),
            department: self.department.filter(|d| !d.trim().is_empty()),
            title,
            locations: self.locations,
            posted_at: self.posted_at,
            apply_url: self.apply_url,
            description_html,
            description_text,
        }
    }
}

/// How often each board is checked and how much of a full pass one check
/// reads.
struct Schedule {
    every_minutes: i32,
    pass_pages: usize,
}

fn schedule(feed: Feed) -> Schedule {
    match feed {
        // Himalayas allows roughly 100 pages before refusing for hours
        // (November 2026), so most checks end early with a 429 and the next
        // waits RATE_LIMITED_WAIT_MINUTES. The newest pages are read first, so
        // new jobs keep arriving; a full pass takes days.
        Feed::Himalayas => Schedule {
            every_minutes: 30,
            pass_pages: 150,
        },
        // Remotive asks for at most 4 calls a day.
        Feed::Remotive => Schedule {
            every_minutes: 360,
            pass_pages: 1,
        },
        Feed::Remoteok => Schedule {
            every_minutes: 120,
            pass_pages: 1,
        },
        // About 40 pages of 500.
        Feed::Usajobs => Schedule {
            every_minutes: 360,
            pass_pages: 100,
        },
    }
}

/// Boards that can run: USAJOBS only with its key.
pub fn enabled(keys: &FeedKeys) -> Vec<Feed> {
    Feed::ALL
        .iter()
        .copied()
        .filter(|f| *f != Feed::Usajobs || keys.usajobs.is_some())
        .collect()
}

/// Adds a row for every board and marks which can run. Called on startup.
pub async fn sync(pool: &PgPool, keys: &FeedKeys) -> sqlx::Result<()> {
    let all: Vec<&str> = Feed::ALL.iter().map(|f| f.as_str()).collect();
    let on: Vec<&str> = enabled(keys).iter().map(|f| f.as_str()).collect();
    sqlx::query(
        "INSERT INTO feeds (id, active) SELECT id, id = ANY($2) FROM unnest($1::text[]) AS id
         ON CONFLICT (id) DO UPDATE SET active = EXCLUDED.active",
    )
    .bind(&all)
    .bind(&on)
    .execute(pool)
    .await
    .map(|_| ())
}

/// Takes the board that's most overdue, if any, with a lease like
/// [`ingest::claim_due`].
pub async fn claim_due(pool: &PgPool) -> sqlx::Result<Option<Feed>> {
    let id: Option<String> = sqlx::query_scalar(
        "UPDATE feeds SET next_check_at = now() + make_interval(mins => $1)
         WHERE id = (
             SELECT id FROM feeds WHERE active AND next_check_at <= now()
             ORDER BY next_check_at LIMIT 1 FOR UPDATE SKIP LOCKED
         )
         RETURNING id",
    )
    .bind(LEASE_MINUTES)
    .fetch_optional(pool)
    .await?;
    Ok(id.as_deref().and_then(Feed::parse))
}

pub async fn fetch_page(
    fetcher: &Fetcher,
    keys: &FeedKeys,
    feed: Feed,
    cursor: Option<&str>,
) -> anyhow::Result<Page> {
    Ok(match feed {
        Feed::Himalayas => himalayas::fetch_page(fetcher, cursor).await?,
        Feed::Remotive => remotive::fetch(fetcher).await?,
        Feed::Remoteok => remoteok::fetch(fetcher).await?,
        Feed::Usajobs => {
            let key = keys
                .usajobs
                .as_ref()
                .context("USAJOBS_API_KEY and USAJOBS_EMAIL aren't set")?;
            usajobs::fetch_page(fetcher, key, cursor).await?
        }
    })
}

#[derive(Debug, Default, Clone, Copy)]
pub struct FeedStats {
    pub pages: usize,
    /// Listings read.
    pub seen: usize,
    /// US-eligible listings that passed the quality checks.
    pub kept: usize,
    /// Jobs added that no employer board has.
    pub new: usize,
    /// Listings matched to a job we already had.
    pub matched: usize,
    /// Listings at a tracked employer whose board doesn't have the job.
    pub skipped: usize,
    /// Set when a full pass finished: the board's jobs it closed.
    pub closed: Option<usize>,
    /// The board rate-limited us partway; the pass resumes next check.
    pub rate_limited: bool,
}

/// A 429 after at least one page isn't a failure: progress is saved every
/// page, so the check stops there and the next one carries on.
fn rate_limited(err: &anyhow::Error) -> bool {
    matches!(
        err.downcast_ref::<FetchError>(),
        Some(FetchError::RateLimited(_))
    )
}

/// Checks one board and records the outcome and next check time.
pub async fn check_and_record(pool: &PgPool, fetcher: &Fetcher, keys: &FeedKeys, feed: Feed) {
    let result = check(pool, fetcher, keys, feed).await;
    let recorded = match &result {
        Ok(s) => {
            tracing::info!(
                feed = feed.as_str(), pages = s.pages, seen = s.seen, kept = s.kept, new = s.new,
                matched = s.matched, skipped = s.skipped, closed = ?s.closed,
                rate_limited = s.rate_limited, "checked job board"
            );
            sqlx::query(
                "UPDATE feeds SET last_checked_at = now(), last_status = 'ok', last_error = NULL,
                     consecutive_failures = 0, next_check_at = now() + make_interval(mins => $2)
                 WHERE id = $1",
            )
            .bind(feed.as_str())
            .bind(if s.rate_limited {
                RATE_LIMITED_WAIT_MINUTES.max(schedule(feed).every_minutes)
            } else {
                schedule(feed).every_minutes
            })
            .execute(pool)
            .await
        }
        Err(err) => {
            tracing::warn!(feed = feed.as_str(), error = %err, "job board check failed");
            sqlx::query(
                "UPDATE feeds SET last_checked_at = now(), last_status = 'error', last_error = $2,
                     consecutive_failures = consecutive_failures + 1,
                     next_check_at = now() + least(
                         interval '24 hours',
                         interval '15 minutes' * power(2, least(consecutive_failures, 10)))
                 WHERE id = $1",
            )
            .bind(feed.as_str())
            .bind(format!("{err:#}"))
            .execute(pool)
            .await
        }
    };
    if let Err(e) = recorded {
        tracing::error!(feed = feed.as_str(), error = %e, "couldn't record job board check");
    }
}

async fn check(
    pool: &PgPool,
    fetcher: &Fetcher,
    keys: &FeedKeys,
    feed: Feed,
) -> anyhow::Result<FeedStats> {
    let started = Utc::now();
    let (pass_cursor, pass_started): (Option<String>, Option<DateTime<Utc>>) =
        sqlx::query_as("SELECT sweep_cursor, sweep_started_at FROM feeds WHERE id = $1")
            .bind(feed.as_str())
            .fetch_one(pool)
            .await?;
    let mut known: HashSet<String> =
        sqlx::query_scalar("SELECT external_id FROM job_sources WHERE feed = $1")
            .bind(feed.as_str())
            .fetch_all(pool)
            .await?
            .into_iter()
            .collect();
    let mut stats = FeedStats::default();

    // The newest pages, until one has nothing new.
    let mut next: Option<String> = None;
    for _ in 0..HEAD_PAGES {
        let page = match fetch_page(fetcher, keys, feed, next.as_deref()).await {
            Ok(page) => page,
            Err(e) if stats.pages > 0 && rate_limited(&e) => {
                stats.rate_limited = true;
                return Ok(stats);
            }
            Err(e) => return Err(e),
        };
        stats.pages += 1;
        let fresh = save(pool, feed, page.postings, &mut known, &mut stats).await?;
        match page.next {
            // The whole board fit in this read: that's a full pass.
            None => {
                stats.closed = Some(finish_pass(pool, feed, started).await?);
                return Ok(stats);
            }
            Some(cursor) => next = Some(cursor),
        }
        if fresh == 0 {
            break;
        }
    }

    // The next stretch of the full pass. A new pass starts with this check
    // and skips the pages just read.
    let (mut cursor, pass_started) = match pass_cursor {
        Some(c) => (Some(c), pass_started.unwrap_or(started)),
        None => (next, started),
    };
    for _ in 0..schedule(feed).pass_pages {
        let page = match fetch_page(fetcher, keys, feed, cursor.as_deref()).await {
            Ok(page) => page,
            Err(e) if stats.pages > 0 && rate_limited(&e) => {
                stats.rate_limited = true;
                break;
            }
            Err(e) => return Err(e),
        };
        stats.pages += 1;
        save(pool, feed, page.postings, &mut known, &mut stats).await?;
        cursor = page.next;
        if cursor.is_none() {
            stats.closed = Some(finish_pass(pool, feed, pass_started).await?);
            break;
        }
        // Saved every page, so an error or restart resumes here.
        sqlx::query("UPDATE feeds SET sweep_cursor = $2, sweep_started_at = $3 WHERE id = $1")
            .bind(feed.as_str())
            .bind(&cursor)
            .bind(pass_started)
            .execute(pool)
            .await?;
    }
    Ok(stats)
}

/// Ends a full pass: removes listings it didn't see and closes the board's
/// own jobs that have no listing left. Returns how many jobs closed.
async fn finish_pass(pool: &PgPool, feed: Feed, started: DateTime<Utc>) -> sqlx::Result<usize> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM job_sources WHERE feed = $1 AND last_seen_at < $2")
        .bind(feed.as_str())
        .bind(started)
        .execute(&mut *tx)
        .await?;
    let closed = sqlx::query(
        "UPDATE jobs j SET closed_at = now()
         WHERE j.feed = $1 AND j.closed_at IS NULL
           AND NOT EXISTS (SELECT 1 FROM job_sources s WHERE s.job_id = j.id AND s.feed = $1)",
    )
    .bind(feed.as_str())
    .execute(&mut *tx)
    .await?
    .rows_affected();
    sqlx::query(
        "UPDATE feeds SET sweep_cursor = NULL, sweep_started_at = NULL, last_pass_at = now()
         WHERE id = $1",
    )
    .bind(feed.as_str())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(closed as usize)
}

/// Companies that repost other employers' jobs; their listings are dropped
/// (the real employer's copy is better, or another board has it).
const REPOSTERS: &[&str] = &["lifelancer", "jobgether"];

/// The quality bar for a listing: open to US workers, with a company, a
/// title and an https link, and not from a reposting service.
pub fn keep(p: &FeedPosting) -> bool {
    let company = p.company.trim().to_lowercase();
    p.us_eligible
        && !company.is_empty()
        && !p.job.title.is_empty()
        && p.url.starts_with("https://")
        && p.job.apply_url.starts_with("https://")
        && !REPOSTERS.contains(&company.as_str())
}

/// Stores one page of listings. Returns how many were new to this board.
async fn save(
    pool: &PgPool,
    feed: Feed,
    postings: Vec<FeedPosting>,
    known: &mut HashSet<String>,
    stats: &mut FeedStats,
) -> anyhow::Result<usize> {
    stats.seen += postings.len();
    let mut ids = HashSet::new();
    let kept: Vec<FeedPosting> = postings
        .into_iter()
        .filter(|p| keep(p) && ids.insert(p.external_id.clone()))
        .collect();
    stats.kept += kept.len();
    if kept.is_empty() {
        return Ok(0);
    }

    // Board links in new listings are leads for discovery.
    let mut leads = Vec::new();
    let fresh: Vec<&FeedPosting> = kept
        .iter()
        .filter(|p| !known.contains(&p.external_id))
        .collect();
    for p in &fresh {
        for lead in sources::board_links(&format!("{} {}", p.job.apply_url, p.job.description_html))
        {
            if !leads.contains(&lead) {
                leads.push(lead);
            }
        }
    }
    let fresh = fresh.len();

    let outcome = merge(pool, feed, &kept).await?;
    stats.new += outcome.new;
    stats.matched += outcome.matched;
    stats.skipped += outcome.skipped;
    known.extend(kept.into_iter().map(|p| p.external_id));

    if !leads.is_empty()
        && let Err(e) = discover::add_candidates(pool, &leads, "link").await
    {
        tracing::warn!(error = %e, "couldn't save discovered boards");
    }
    Ok(fresh)
}

#[derive(Debug, Default)]
struct Merged {
    new: usize,
    matched: usize,
    skipped: usize,
}

/// The board's own jobs use this id, so two boards never collide.
fn own_id(feed: Feed, external_id: &str) -> String {
    format!("{}:{external_id}", feed.as_str())
}

/// The external company's key: the matching key of its name, or the
/// lowercased name when that's empty (a name in another script).
const EXTERNAL_TOKEN: &str = "COALESCE(NULLIF(company_key(x.company), ''), lower(x.company))";

/// Matches listings to jobs we have, adds the rest as the board's own jobs,
/// and records every listing (see the module docs).
async fn merge(pool: &PgPool, feed: Feed, postings: &[FeedPosting]) -> sqlx::Result<Merged> {
    let mut tx = pool.begin().await?;

    let candidates = serde_json::Value::Array(
        postings
            .iter()
            .enumerate()
            .map(|(i, p)| {
                serde_json::json!({
                    "i": i,
                    "company": p.company.trim(),
                    "title": p.job.title,
                    "workplace": p.job.workplace.as_str(),
                    "locations": p.job.locations,
                })
            })
            .collect(),
    );
    // For each listing: the employer's (or another board's) copy, and
    // whether the employer's own board is tracked at all.
    let matches: Vec<(i32, Option<Uuid>, bool)> = sqlx::query_as(
        "WITH x AS (
             SELECT x.*, company_key(x.company) AS k
             FROM jsonb_to_recordset($1::jsonb) AS x(
                 i integer, company text, title text, workplace text, locations text[])
         )
         SELECT x.i,
             (SELECT j.id FROM companies c JOIN jobs j ON j.company_id = c.id
              WHERE company_key(c.name) = x.k AND j.closed_at IS NULL AND j.in_scope
                AND title_key(j.title) = title_key(x.title)
                AND j.feed IS DISTINCT FROM $2
                AND (j.workplace = x.workplace OR j.locations && x.locations)
              ORDER BY (j.feed IS NULL) DESC, j.first_seen_at, j.id
              LIMIT 1),
             EXISTS (SELECT 1 FROM companies c
                     WHERE company_key(c.name) = x.k AND c.platform <> 'external' AND c.active)
         FROM x WHERE x.k <> ''",
    )
    .bind(candidates.to_string())
    .bind(feed.as_str())
    .fetch_all(&mut *tx)
    .await?;

    let mut matched: Vec<Option<Uuid>> = vec![None; postings.len()];
    let mut employer = vec![false; postings.len()];
    for (i, job_id, tracked) in matches {
        if let Some(i) = usize::try_from(i).ok().filter(|&i| i < postings.len()) {
            matched[i] = job_id;
            employer[i] = tracked;
        }
    }
    let own: Vec<usize> = (0..postings.len())
        .filter(|&i| matched[i].is_none() && !employer[i])
        .collect();
    let mut merged = Merged {
        matched: matched.iter().filter(|m| m.is_some()).count(),
        skipped: (0..postings.len())
            .filter(|&i| matched[i].is_none() && employer[i])
            .count(),
        new: 0,
    };

    if !own.is_empty() {
        merged.new = upsert_own(&mut tx, feed, own.iter().map(|&i| &postings[i])).await?;
    }

    // Every listing, pointing at its job: the match, the board's own job,
    // or none (employer tracked, job not there).
    let listings = serde_json::Value::Array(
        postings
            .iter()
            .enumerate()
            .map(|(i, p)| {
                serde_json::json!({
                    "external_id": p.external_id,
                    "url": p.url,
                    "job_id": matched[i],
                    "own": own.contains(&i),
                    "company": p.company.trim(),
                    "source_job_id": own_id(feed, &p.external_id),
                })
            })
            .collect(),
    );
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO job_sources (feed, external_id, job_id, url)
         SELECT $1, x.external_id, COALESCE(x.job_id, o.id), x.url
         FROM jsonb_to_recordset($2::jsonb) AS x(
             external_id text, url text, job_id uuid, own boolean, company text,
             source_job_id text)
         LEFT JOIN companies c ON x.own AND c.platform = 'external'
             AND c.board_token = {EXTERNAL_TOKEN}
         LEFT JOIN jobs o ON o.company_id = c.id AND o.source_job_id = x.source_job_id
         ON CONFLICT (feed, external_id) DO UPDATE SET
             job_id = EXCLUDED.job_id, url = EXCLUDED.url, last_seen_at = now()"
    )))
    .bind(feed.as_str())
    .bind(listings.to_string())
    .execute(&mut *tx)
    .await?;

    // The board's own copies of listings that now match an employer's job
    // (or whose employer is tracked) close: the employer's board wins.
    let replaced: Vec<String> = (0..postings.len())
        .filter(|i| !own.contains(i))
        .map(|i| own_id(feed, &postings[i].external_id))
        .collect();
    if !replaced.is_empty() {
        sqlx::query(
            "UPDATE jobs SET closed_at = now()
             WHERE feed = $1 AND closed_at IS NULL AND source_job_id = ANY($2)",
        )
        .bind(feed.as_str())
        .bind(&replaced)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(merged)
}

/// Adds or refreshes the board's own jobs, creating their external companies
/// first. Returns how many jobs were new.
async fn upsert_own<'a>(
    tx: &mut Transaction<'_, Postgres>,
    feed: Feed,
    postings: impl Iterator<Item = &'a FeedPosting>,
) -> sqlx::Result<usize> {
    let rows: Vec<serde_json::Value> = postings
        .map(|p| {
            let mut row = ingest::job_json(&p.job);
            row["source_job_id"] = own_id(feed, &p.external_id).into();
            row["company"] = p.company.trim().into();
            row["company_url"] = p
                .company_url
                .clone()
                .unwrap_or_else(|| p.url.clone())
                .into();
            row
        })
        .collect();
    let batch = serde_json::Value::Array(rows).to_string();

    sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO companies (name, platform, board_token, careers_url, active)
         SELECT DISTINCT ON (token) company, 'external', token, company_url, false
         FROM (SELECT x.company, x.company_url, {EXTERNAL_TOKEN} AS token
               FROM jsonb_to_recordset($1::jsonb) AS x(company text, company_url text)) s
         ON CONFLICT (platform, board_token) DO UPDATE SET name = EXCLUDED.name
         WHERE companies.name IS DISTINCT FROM EXCLUDED.name"
    )))
    .bind(&batch)
    .execute(&mut **tx)
    .await?;

    let inserted: Vec<bool> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "INSERT INTO jobs (
             company_id, source_job_id, feed, title, department, locations, workplace,
             employment_type, seniority, salary_min, salary_max, salary_currency,
             salary_period, salary_source, posted_at, apply_url, description_html,
             description_text, content_hash, in_scope, no_sponsorship, citizens_only, normalize_version
         )
         SELECT c.id, x.source_job_id, $2, x.title, x.department, x.locations, x.workplace,
                x.employment_type, x.seniority, x.salary_min, x.salary_max, x.salary_currency,
                x.salary_period, x.salary_source, x.posted_at, x.apply_url, x.description_html,
                x.description_text, x.content_hash, x.in_scope, x.no_sponsorship, x.citizens_only, $3
         FROM jsonb_to_recordset($1::jsonb) AS x(
             company text, source_job_id text, title text, department text, locations text[],
             workplace text, employment_type text, seniority text, salary_min integer,
             salary_max integer, salary_currency text, salary_period text,
             salary_source text, posted_at timestamptz, apply_url text,
             description_html text, description_text text, content_hash text,
             in_scope boolean, no_sponsorship boolean, citizens_only boolean
         )
         JOIN companies c ON c.platform = 'external' AND c.board_token = {EXTERNAL_TOKEN}
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
             no_sponsorship = EXCLUDED.no_sponsorship,
             citizens_only = EXCLUDED.citizens_only,
             normalize_version = EXCLUDED.normalize_version,
             last_seen_at = now(),
             repost_count = jobs.repost_count + CASE WHEN jobs.closed_at IS NULL THEN 0 ELSE 1 END,
             closed_at = NULL
         WHERE jobs.closed_at IS NOT NULL
            OR jobs.last_seen_at < now() - interval '1 hour'
            OR jobs.content_hash IS DISTINCT FROM EXCLUDED.content_hash
            OR jobs.normalize_version IS DISTINCT FROM EXCLUDED.normalize_version
            OR (jobs.title, jobs.locations, jobs.salary_min, jobs.salary_max, jobs.apply_url)
               IS DISTINCT FROM
               (EXCLUDED.title, EXCLUDED.locations, EXCLUDED.salary_min, EXCLUDED.salary_max,
                EXCLUDED.apply_url)
         RETURNING (xmax = 0)"
    )))
    .bind(&batch)
    .bind(feed.as_str())
    .bind(normalize::NORMALIZE_VERSION)
    .fetch_all(&mut **tx)
    .await?;
    Ok(inserted.into_iter().filter(|&new| new).count())
}

/// Board text sometimes arrives UTF-8 encoded twice ("MÃ¼nchen",
/// "donâ€™t", "Å\u{8c}URA" for "ŌURA"), sometimes mixed with correct text.
/// Each run of non-ASCII characters is repaired on its own when its bytes
/// decode cleanly as UTF-8 (an encoded character never contains ASCII
/// bytes); real text ("São Paulo", a lone "’") never does, so it stays.
pub fn fix_mojibake(s: &str) -> String {
    if s.is_ascii() {
        return s.to_owned();
    }
    let mut out = String::with_capacity(s.len());
    let mut run = String::new();
    let flush = |run: &mut String, out: &mut String| {
        let bytes: Option<Vec<u8>> = run.chars().map(cp1252_byte).collect();
        match bytes.and_then(|b| String::from_utf8(b).ok()) {
            Some(fixed) => out.push_str(&fixed),
            None => out.push_str(run),
        }
        run.clear();
    };
    for c in s.chars() {
        if c.is_ascii() {
            flush(&mut run, &mut out);
            out.push(c);
        } else {
            run.push(c);
        }
    }
    flush(&mut run, &mut out);
    out
}

/// The byte a character came from when bytes were read as Windows-1252
/// (or as Latin-1, which keeps 0x80–0x9F as control characters).
fn cp1252_byte(c: char) -> Option<u8> {
    const HIGH: [(char, u8); 27] = [
        ('€', 0x80),
        ('‚', 0x82),
        ('ƒ', 0x83),
        ('„', 0x84),
        ('…', 0x85),
        ('†', 0x86),
        ('‡', 0x87),
        ('ˆ', 0x88),
        ('‰', 0x89),
        ('Š', 0x8A),
        ('‹', 0x8B),
        ('Œ', 0x8C),
        ('Ž', 0x8E),
        ('‘', 0x91),
        ('’', 0x92),
        ('“', 0x93),
        ('”', 0x94),
        ('•', 0x95),
        ('–', 0x96),
        ('—', 0x97),
        ('˜', 0x98),
        ('™', 0x99),
        ('š', 0x9A),
        ('›', 0x9B),
        ('œ', 0x9C),
        ('ž', 0x9E),
        ('Ÿ', 0x9F),
    ];
    match u32::from(c) {
        n @ 0..=0xFF => Some(n as u8),
        _ => HIGH.iter().find(|(ch, _)| *ch == c).map(|(_, b)| *b),
    }
}

/// A text field as HTML: kept as is when it already has tags, otherwise
/// escaped, with blank lines as paragraphs and line breaks kept.
pub fn text_or_html(s: &str) -> String {
    if s.contains('<') && s.contains('>') {
        return s.to_owned();
    }
    s.split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| {
            format!(
                "<p>{}</p>",
                html_escape::encode_text(p).replace('\n', "<br>")
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mojibake() {
        assert_eq!(fix_mojibake("MÃ¼nchen"), "München");
        assert_eq!(fix_mojibake("donâ€™t"), "don’t");
        // Latin-1 reading: the C1 control character stands for byte 0x8C.
        assert_eq!(fix_mojibake("Å\u{8c}URA"), "ŌURA");
        assert_eq!(fix_mojibake("Weâ\u{80}\u{99}re"), "We’re");
        assert_eq!(fix_mojibake("São Paulo"), "São Paulo");
        assert_eq!(fix_mojibake("Plain text"), "Plain text");
        // Not double-encoded: a real "Ã" stays.
        assert_eq!(fix_mojibake("Ã and 日本"), "Ã and 日本");
        // Correct and double-encoded text in one string.
        assert_eq!(
            fix_mojibake("It’s Å\u{8c}URA, São Paulo"),
            "It’s ŌURA, São Paulo"
        );
        assert_eq!(fix_mojibake("office.Â\u{a0}Next"), "office.\u{a0}Next");
    }

    #[test]
    fn text_paragraphs() {
        assert_eq!(
            text_or_html("a < b\n\nline 1\nline 2"),
            "<p>a &lt; b</p><p>line 1<br>line 2</p>"
        );
        assert_eq!(text_or_html("<p>Already</p>"), "<p>Already</p>");
    }

    fn posting(company: &str, url: &str, us: bool) -> FeedPosting {
        FeedPosting {
            external_id: "1".into(),
            company: company.into(),
            company_url: None,
            url: url.into(),
            us_eligible: us,
            job: JobParts {
                id: "1".into(),
                title: "Engineer".into(),
                department: None,
                locations: vec!["Remote".into()],
                workplace: None,
                employment: None,
                salary: None,
                posted_at: Utc::now(),
                apply_url: url.into(),
                html: String::new(),
            }
            .build(),
        }
    }

    #[test]
    fn quality_checks() {
        assert!(keep(&posting("Acme", "https://example.com/1", true)));
        assert!(!keep(&posting("Acme", "https://example.com/1", false)));
        assert!(!keep(&posting(" ", "https://example.com/1", true)));
        assert!(!keep(&posting("Acme", "http://example.com/1", true)));
        assert!(!keep(&posting("Lifelancer", "https://example.com/1", true)));
    }
}
