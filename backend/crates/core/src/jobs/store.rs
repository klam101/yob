//! Read queries for the API: the feed, one job, companies. Filters are built
//! with bound parameters only; nothing from the request is spliced into SQL.

use chrono::{DateTime, Utc};
// `AssertSqlSafe` below only ever wraps our own constant SQL fragments joined
// together (SELECT_JOB, STALE_SQL, SELECT_COMPANY); request data is always bound.
use sqlx::{AssertSqlSafe, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::model::{EmploymentType, Platform, Seniority, Source, Workplace};
use super::search::{self, Place};

/// "Possibly stale": open more than 60 days (by the earlier of the company's
/// posted date and when we first saw it) or reposted more than twice.
const STALE_SQL: &str =
    "(LEAST(j.posted_at, j.first_seen_at) < now() - interval '60 days' OR j.repost_count > 2)";

#[derive(Debug, Default, Clone)]
pub struct JobFilter {
    pub q: Option<String>,
    /// Any of these titles; see [`search::title_words`].
    pub titles: Vec<String>,
    /// Any of these places. Remote jobs pass too when `workplace` includes
    /// remote, so "Austin, TX" plus Remote means remote or in Austin.
    pub locations: Vec<Place>,
    pub workplace: Vec<Workplace>,
    pub employment_type: Vec<EmploymentType>,
    pub seniority: Vec<Seniority>,
    /// Lowercased department names.
    pub department: Vec<String>,
    /// Lowercased company names.
    pub company: Vec<String>,
    /// Where the job came from: the employer's platform, or a job board.
    pub source: Vec<Source>,
    pub posted_after: Option<DateTime<Utc>>,
    /// Minimum yearly salary.
    pub salary_min: Option<i32>,
    /// With `salary_min`, keep jobs that don't list a salary.
    pub include_unlisted_salary: bool,
    pub hide_stale: bool,
    /// Deal-breakers: titles containing any of these words or phrases.
    pub exclude_title: Vec<String>,
    /// Deal-breakers: lowercased company names.
    pub exclude_company: Vec<String>,
    /// Hide jobs that say they won't sponsor a visa.
    pub hide_no_sponsorship: bool,
    /// Hide jobs open only to U.S. citizens or clearance holders.
    pub hide_citizens_only: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum JobSort {
    /// Newest until fit scores exist.
    #[default]
    Recommended,
    Newest,
    Salary,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct JobRow {
    pub id: Uuid,
    pub title: String,
    pub department: Option<String>,
    pub locations: Vec<String>,
    pub workplace: String,
    pub employment_type: String,
    pub seniority: String,
    pub salary_min: Option<i32>,
    pub salary_max: Option<i32>,
    pub salary_currency: Option<String>,
    pub salary_period: Option<String>,
    pub posted_at: DateTime<Utc>,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub repost_count: i32,
    pub possibly_stale: bool,
    pub apply_url: String,
    pub no_sponsorship: bool,
    pub citizens_only: bool,
    pub company_name: String,
    pub company_domain: Option<String>,
    pub company_careers_url: String,
    /// The employer's platform, or the job board for jobs only a board lists.
    pub source: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct JobDetailRow {
    #[sqlx(flatten)]
    pub job: JobRow,
    pub description_html: Option<String>,
    pub description_text: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Count {
    pub value: String,
    pub count: i64,
}

const SELECT_JOB: &str = "SELECT j.id, j.title, j.department, j.locations, j.workplace,
       j.employment_type, j.seniority, j.salary_min, j.salary_max, j.salary_currency,
       j.salary_period, j.posted_at, j.first_seen_at, j.last_seen_at, j.closed_at,
       j.repost_count, j.apply_url, j.no_sponsorship, j.citizens_only, c.name AS company_name, c.domain AS company_domain,
       c.careers_url AS company_careers_url, COALESCE(j.feed, c.platform) AS source";

fn push_where(qb: &mut QueryBuilder<Postgres>, f: &JobFilter) {
    qb.push(
        " FROM jobs j JOIN companies c ON c.id = j.company_id WHERE j.closed_at IS NULL AND j.in_scope",
    );

    if let Some(q) = &f.q {
        let like = format!("%{}%", escape_like(q));
        qb.push(" AND (j.search @@ websearch_to_tsquery('english', ")
            .push_bind(q.clone())
            .push(") OR j.title ILIKE ")
            .push_bind(like.clone())
            .push(" OR c.name ILIKE ")
            .push_bind(like)
            .push(")");
    }
    if !f.titles.is_empty() {
        push_titles(qb, &f.titles);
    }
    if !f.exclude_title.is_empty() {
        qb.push(" AND j.title !~* ")
            .push_bind(search::whole_word_pattern(&f.exclude_title));
    }
    if !f.locations.is_empty() {
        push_locations(qb, &f.locations, f.workplace.contains(&Workplace::Remote));
    }
    if !f.workplace.is_empty() {
        qb.push(" AND j.workplace = ANY(")
            .push_bind(names(&f.workplace, Workplace::as_str))
            .push(")");
    }
    if !f.employment_type.is_empty() {
        qb.push(" AND j.employment_type = ANY(")
            .push_bind(names(&f.employment_type, EmploymentType::as_str))
            .push(")");
    }
    if !f.seniority.is_empty() {
        qb.push(" AND j.seniority = ANY(")
            .push_bind(names(&f.seniority, Seniority::as_str))
            .push(")");
    }
    if !f.source.is_empty() {
        qb.push(" AND COALESCE(j.feed, c.platform) = ANY(")
            .push_bind(names(&f.source, Source::as_str))
            .push(")");
    }
    if !f.department.is_empty() {
        qb.push(" AND lower(j.department) = ANY(")
            .push_bind(f.department.clone())
            .push(")");
    }
    if !f.company.is_empty() {
        qb.push(" AND lower(c.name) = ANY(")
            .push_bind(f.company.clone())
            .push(")");
    }
    if !f.exclude_company.is_empty() {
        qb.push(" AND lower(c.name) <> ALL(")
            .push_bind(f.exclude_company.clone())
            .push(")");
    }
    if f.hide_no_sponsorship {
        qb.push(" AND NOT j.no_sponsorship");
    }
    if f.hide_citizens_only {
        qb.push(" AND NOT j.citizens_only");
    }
    if let Some(after) = f.posted_after {
        qb.push(" AND j.posted_at >= ").push_bind(after);
    }
    if let Some(min) = f.salary_min {
        qb.push(" AND (j.salary_annual_max >= ").push_bind(min);
        if f.include_unlisted_salary {
            qb.push(" OR j.salary_annual_max IS NULL");
        }
        qb.push(")");
    }
    if f.hide_stale {
        qb.push(" AND NOT ").push(STALE_SQL);
    }
}

/// Any of the titles: each title's words must all appear in the job title.
fn push_titles(qb: &mut QueryBuilder<Postgres>, titles: &[String]) {
    let titles: Vec<Vec<String>> = titles
        .iter()
        .map(|t| search::title_words(t))
        .filter(|w| !w.is_empty())
        .take(20)
        .collect();
    if titles.is_empty() {
        return;
    }
    qb.push(" AND (");
    for (i, words) in titles.iter().enumerate() {
        qb.push(if i == 0 { "(" } else { " OR (" });
        for (k, word) in words.iter().enumerate() {
            if k > 0 {
                qb.push(" AND ");
            }
            qb.push("j.title ~* ")
                .push_bind(search::whole_word_pattern(std::slice::from_ref(word)));
        }
        qb.push(")");
    }
    qb.push(")");
}

/// Any of the places, or any remote job when `remote_ok`.
fn push_locations(qb: &mut QueryBuilder<Postgres>, places: &[Place], remote_ok: bool) {
    qb.push(" AND (false");
    if remote_ok {
        qb.push(" OR j.workplace = 'remote'");
    }
    for place in places {
        match place {
            Place::Remote => {
                qb.push(" OR j.workplace = 'remote'");
            }
            Place::State(state) => {
                qb.push(" OR EXISTS (SELECT 1 FROM unnest(j.locations) AS l WHERE l ILIKE ")
                    .push_bind(format!("%{}%", escape_like(state.name)))
                    .push(" OR l ~ ")
                    .push_bind(state.code_pattern())
                    .push(")");
            }
            Place::Text { text, state } => {
                qb.push(" OR EXISTS (SELECT 1 FROM unnest(j.locations) AS l WHERE l ILIKE ")
                    .push_bind(format!("%{}%", escape_like(text)));
                // "Portland, OR" shouldn't match "Portland, ME"; a location
                // that names no state code ("Austin", "Austin, Texas") passes.
                if let Some(state) = state {
                    qb.push(" AND (l ILIKE ")
                        .push_bind(format!("%{}%", escape_like(state.name)))
                        .push(" OR l ~ ")
                        .push_bind(state.code_pattern())
                        .push(r" OR l !~ ',\s*[A-Z]{2}([^A-Za-z]|$)')");
                }
                qb.push(")");
            }
        }
    }
    qb.push(")");
}

/// Enum values as the strings stored in Postgres, for `= ANY($n)`.
fn names<T: Copy>(values: &[T], as_str: fn(T) -> &'static str) -> Vec<String> {
    values.iter().map(|v| as_str(*v).to_owned()).collect()
}

fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

pub struct JobPage {
    pub rows: Vec<JobRow>,
    pub total: i64,
    pub company_count: i64,
}

pub async fn list_jobs(
    pool: &PgPool,
    filter: &JobFilter,
    sort: JobSort,
    page: u32,
    page_size: u32,
) -> sqlx::Result<JobPage> {
    let mut count = QueryBuilder::new("SELECT count(*), count(DISTINCT c.id)");
    push_where(&mut count, filter);
    let (total, company_count): (i64, i64) = count.build_query_as().fetch_one(pool).await?;

    let mut qb = QueryBuilder::new(SELECT_JOB);
    qb.push(", ").push(STALE_SQL).push(" AS possibly_stale");
    push_where(&mut qb, filter);
    qb.push(match sort {
        JobSort::Recommended | JobSort::Newest => " ORDER BY j.posted_at DESC, j.id",
        JobSort::Salary => " ORDER BY j.salary_annual_max DESC NULLS LAST, j.posted_at DESC, j.id",
    });
    qb.push(" LIMIT ")
        .push_bind(i64::from(page_size))
        .push(" OFFSET ")
        .push_bind(i64::from(page.saturating_sub(1)) * i64::from(page_size));
    let rows = qb.build_query_as().fetch_all(pool).await?;

    Ok(JobPage {
        rows,
        total,
        company_count,
    })
}

/// One job, open or closed (a saved job may have closed since).
pub async fn get_job(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<JobDetailRow>> {
    let sql = format!(
        "{SELECT_JOB}, {STALE_SQL} AS possibly_stale, j.description_html, j.description_text
         FROM jobs j JOIN companies c ON c.id = j.company_id WHERE j.id = $1"
    );
    sqlx::query_as(AssertSqlSafe(sql))
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// A job board listing of a job: the board (a `Feed` name) and its page.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Listing {
    pub feed: String,
    pub url: String,
}

/// Where a job board lists this job, oldest listing first.
pub async fn job_listings(pool: &PgPool, job_id: Uuid) -> sqlx::Result<Vec<Listing>> {
    sqlx::query_as(
        "SELECT feed, url FROM job_sources WHERE job_id = $1 ORDER BY first_seen_at, feed",
    )
    .bind(job_id)
    .fetch_all(pool)
    .await
}

/// Status of one job board for the Job sources page.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FeedRow {
    pub id: String,
    /// False when the board needs an API key that isn't set.
    pub active: bool,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub next_check_at: DateTime<Utc>,
    pub last_status: Option<String>,
    pub last_error: Option<String>,
    pub last_pass_at: Option<DateTime<Utc>>,
    /// A full pass is part way through.
    pub passing: bool,
    /// Open in-scope jobs only this board lists.
    pub own_jobs: i64,
    /// Listings matched to a job from an employer's board (or another job board).
    pub matched: i64,
    /// Listings at tracked employers whose own board doesn't have the job.
    pub skipped: i64,
}

pub async fn list_feeds(pool: &PgPool) -> sqlx::Result<Vec<FeedRow>> {
    sqlx::query_as(
        "SELECT f.id, f.active, f.last_checked_at, f.next_check_at, f.last_status, f.last_error,
                f.last_pass_at, f.sweep_cursor IS NOT NULL AS passing,
                (SELECT count(*) FROM jobs j
                 WHERE j.feed = f.id AND j.closed_at IS NULL AND j.in_scope) AS own_jobs,
                (SELECT count(*) FROM job_sources s JOIN jobs j ON j.id = s.job_id
                 WHERE s.feed = f.id AND j.feed IS DISTINCT FROM f.id) AS matched,
                (SELECT count(*) FROM job_sources s
                 WHERE s.feed = f.id AND s.job_id IS NULL) AS skipped
         FROM feeds f ORDER BY f.id",
    )
    .fetch_all(pool)
    .await
}

/// Companies listed in the Company filter before the user searches.
pub const TOP_COMPANIES: i64 = 200;

/// Filter-menu options with open-job counts: the 60 biggest departments and
/// the [`TOP_COMPANIES`] companies with the most jobs. The same for every
/// request, so the API caches them.
pub async fn facets(pool: &PgPool) -> sqlx::Result<(Vec<Count>, Vec<Count>)> {
    let departments = sqlx::query_as(
        "SELECT department AS value, count(*) AS count FROM jobs
         WHERE closed_at IS NULL AND in_scope AND department IS NOT NULL
         GROUP BY department ORDER BY count(*) DESC, department LIMIT 60",
    )
    .fetch_all(pool)
    .await?;
    let companies = sqlx::query_as(
        "SELECT c.name AS value, count(*) AS count FROM jobs j JOIN companies c ON c.id = j.company_id
         WHERE j.closed_at IS NULL AND j.in_scope
         GROUP BY c.name ORDER BY count(*) DESC, c.name LIMIT $1",
    )
    .bind(TOP_COMPANIES)
    .fetch_all(pool)
    .await?;
    Ok((departments, companies))
}

/// Open-job counts for companies matching `q` (for the Company filter's
/// search box), or for exactly `names` (lowercased; selected companies that
/// aren't in the top list).
pub async fn company_counts(
    pool: &PgPool,
    q: Option<&str>,
    names: &[String],
    limit: i64,
) -> sqlx::Result<Vec<Count>> {
    let like = q.map(|q| format!("%{}%", escape_like(q.trim())));
    sqlx::query_as(
        "SELECT c.name AS value, count(j.id) AS count
         FROM companies c JOIN jobs j ON j.company_id = c.id AND j.closed_at IS NULL AND j.in_scope
         WHERE ($1::text IS NULL OR c.name ILIKE $1) AND (cardinality($2::text[]) = 0 OR lower(c.name) = ANY($2))
         GROUP BY c.name ORDER BY count(j.id) DESC, c.name LIMIT $3",
    )
    .bind(like)
    .bind(names)
    .bind(limit)
    .fetch_all(pool)
    .await
}

// --- companies -------------------------------------------------------------

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CompanyRow {
    pub id: Uuid,
    pub name: String,
    pub platform: String,
    pub board_token: String,
    pub careers_url: String,
    pub active: bool,
    pub open_jobs: i64,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub next_check_at: DateTime<Utc>,
    pub last_status: Option<String>,
    pub last_error: Option<String>,
}

const SELECT_COMPANY: &str = "SELECT c.id, c.name, c.platform, c.board_token, c.careers_url, c.active,
       (SELECT count(*) FROM jobs j WHERE j.company_id = c.id AND j.closed_at IS NULL AND j.in_scope) AS open_jobs,
       c.last_checked_at, c.next_check_at, c.last_status, c.last_error
       FROM companies c";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CompanyStatus {
    #[default]
    All,
    Errors,
    /// Not checked yet.
    Pending,
}

/// Totals for the Job sources page, over every company.
#[derive(Debug, Clone, Copy, Default, sqlx::FromRow)]
pub struct SourcesSummary {
    pub companies: i64,
    pub open_jobs: i64,
    pub errors: i64,
    pub pending: i64,
}

pub struct CompanyPage {
    pub rows: Vec<CompanyRow>,
    /// Companies matching the search and status, across all pages.
    pub total: i64,
    pub summary: SourcesSummary,
}

/// One page of companies, by name, filtered by a name or token search and status.
pub async fn list_companies(
    pool: &PgPool,
    q: Option<&str>,
    status: CompanyStatus,
    page: u32,
    page_size: u32,
) -> sqlx::Result<CompanyPage> {
    // Employers known only from job boards aren't checked, so they're left out.
    const WHERE: &str = " WHERE c.platform <> 'external'
         AND ($1::text IS NULL OR c.name ILIKE $1 OR c.board_token ILIKE $1)
         AND ($2 = 'all' OR ($2 = 'errors' AND c.last_status = 'error')
              OR ($2 = 'pending' AND c.last_status IS NULL))";
    let like = q
        .map(str::trim)
        .filter(|q| !q.is_empty())
        .map(|q| format!("%{}%", escape_like(q)));
    let status = match status {
        CompanyStatus::All => "all",
        CompanyStatus::Errors => "errors",
        CompanyStatus::Pending => "pending",
    };

    let total: i64 = sqlx::query_scalar(AssertSqlSafe(format!(
        "SELECT count(*) FROM companies c{WHERE}"
    )))
    .bind(&like)
    .bind(status)
    .fetch_one(pool)
    .await?;
    let rows = sqlx::query_as(AssertSqlSafe(format!(
        "{SELECT_COMPANY}{WHERE} ORDER BY lower(c.name), c.id LIMIT $3 OFFSET $4"
    )))
    .bind(&like)
    .bind(status)
    .bind(i64::from(page_size))
    .bind(i64::from(page.saturating_sub(1)) * i64::from(page_size))
    .fetch_all(pool)
    .await?;
    let summary = sqlx::query_as(
        "SELECT (SELECT count(*) FROM companies WHERE platform <> 'external') AS companies,
                (SELECT count(*) FROM jobs WHERE closed_at IS NULL AND in_scope) AS open_jobs,
                (SELECT count(*) FROM companies WHERE last_status = 'error' AND platform <> 'external') AS errors,
                (SELECT count(*) FROM companies WHERE last_status IS NULL AND platform <> 'external') AS pending",
    )
    .fetch_one(pool)
    .await?;
    Ok(CompanyPage {
        rows,
        total,
        summary,
    })
}

pub async fn get_company(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<CompanyRow>> {
    sqlx::query_as(AssertSqlSafe(format!("{SELECT_COMPANY} WHERE c.id = $1")))
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn find_company(
    pool: &PgPool,
    platform: Platform,
    token: &str,
) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar(
        "SELECT id FROM companies WHERE platform = $1 AND lower(board_token) = lower($2)",
    )
    .bind(platform.as_str())
    .bind(token)
    .fetch_optional(pool)
    .await
}

/// Adds a company to track; it's due immediately.
pub async fn insert_company(
    pool: &PgPool,
    name: &str,
    platform: Platform,
    token: &str,
) -> sqlx::Result<Uuid> {
    sqlx::query_scalar(
        "INSERT INTO companies (name, platform, board_token, careers_url)
         VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(name)
    .bind(platform.as_str())
    .bind(token)
    .bind(super::sources::careers_url(platform, token))
    .fetch_one(pool)
    .await
}

/// Makes a company due now; the worker picks it up within a few seconds.
pub async fn request_check(pool: &PgPool, id: Uuid) -> sqlx::Result<bool> {
    let updated = sqlx::query(
        "UPDATE companies SET next_check_at = now(), active = true
             WHERE id = $1 AND platform <> 'external'",
    )
    .bind(id)
    .execute(pool)
    .await?
    .rows_affected();
    Ok(updated > 0)
}
