//! Job feed endpoints. They only read the job database; the worker keeps it
//! fresh (see `yob_core::jobs::ingest`).

use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;
use yob_core::jobs::{
    EmploymentType, SalaryPeriod, Seniority, Source, Workplace, search,
    store::{self, JobFilter, JobRow, JobSort},
};

use crate::{error::ApiError, state::AppState};

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_jobs))
        .routes(routes!(search_companies))
        .routes(routes!(get_job))
}

// --- types -----------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MatchLabel {
    Strong,
    Good,
    Fair,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Sort {
    /// Newest first until fit scores exist.
    #[default]
    Recommended,
    Newest,
    Salary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, ToSchema)]
pub enum PostedWithin {
    #[serde(rename = "24h")]
    Day,
    #[serde(rename = "3d")]
    ThreeDays,
    #[serde(rename = "1w")]
    Week,
    #[serde(rename = "2w")]
    TwoWeeks,
    #[serde(rename = "1m")]
    Month,
    #[serde(rename = "3m")]
    ThreeMonths,
}

impl PostedWithin {
    fn duration(self) -> Duration {
        match self {
            Self::Day => Duration::hours(24),
            Self::ThreeDays => Duration::days(3),
            Self::Week => Duration::weeks(1),
            Self::TwoWeeks => Duration::weeks(2),
            Self::Month => Duration::days(30),
            Self::ThreeMonths => Duration::days(90),
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Company {
    pub name: String,
    pub domain: Option<String>,
    /// The company's public job board.
    pub careers_url: String,
    /// Empty until company data is added.
    pub industries: Vec<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Salary {
    pub min: u32,
    pub max: u32,
    pub currency: String,
    pub period: SalaryPeriod,
}

/// How well a job fits the user's profile. Absent until a resume is added.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Match {
    /// 0 to 100.
    pub score: u8,
    pub label: MatchLabel,
    /// Two or three short reasons, best first.
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JobSummary {
    pub id: String,
    pub title: String,
    pub company: Company,
    pub locations: Vec<String>,
    pub workplace: Workplace,
    pub employment_type: EmploymentType,
    pub seniority: Seniority,
    pub department: Option<String>,
    pub salary: Option<Salary>,
    /// The date the company says it posted the job.
    pub posted_at: DateTime<Utc>,
    /// When yob first saw the job; more trustworthy than `posted_at`.
    pub first_seen_at: DateTime<Utc>,
    /// The last check that found the job still open.
    pub last_seen_at: DateTime<Utc>,
    /// Set once the job disappeared from the company's board.
    pub closed_at: Option<DateTime<Utc>>,
    /// Times this job closed and reappeared.
    pub repost_count: u32,
    /// Open more than 60 days or reposted more than twice.
    pub possibly_stale: bool,
    /// The posting says it won't sponsor a work visa.
    pub no_sponsorship: bool,
    /// The posting is open only to U.S. citizens or security-clearance holders.
    pub citizens_only: bool,
    /// The employer's hiring platform, or the job board when no employer
    /// copy is tracked.
    pub source: Source,
    pub apply_url: String,
    #[serde(rename = "match")]
    pub fit: Option<Match>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MatchBreakdown {
    pub experience: u8,
    pub skill: u8,
    pub education: u8,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct KeywordGap {
    /// Keywords from the job that the profile covers.
    pub matched: Vec<String>,
    /// Keywords from the job the profile doesn't mention.
    pub missing: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct JobDetail {
    pub job: JobSummary,
    /// One-paragraph plain-language summary. Absent until AI extraction exists.
    pub summary: Option<String>,
    /// Sanitized HTML of the full posting; empty for long-closed jobs.
    pub description_html: String,
    /// Extracted skills; empty until AI extraction exists.
    pub skills: Vec<String>,
    pub yoe_min: Option<u8>,
    pub education: Option<String>,
    /// Extra detail about the workplace arrangement, e.g. in-office days.
    pub workplace_note: Option<String>,
    pub breakdown: Option<MatchBreakdown>,
    pub keywords: Option<KeywordGap>,
    /// Job boards that list this job, each linking to its listing. Boards
    /// like Remotive and Remote OK require this credit and link.
    pub listings: Vec<Listing>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Listing {
    pub source: Source,
    /// The job's page on the board.
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FacetCount {
    pub value: String,
    pub count: u32,
}

/// Options for filter menus, counted over all open jobs (refreshed every few
/// minutes).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Facets {
    /// The biggest departments.
    pub departments: Vec<FacetCount>,
    /// The companies with the most jobs, plus any selected in `company`.
    /// Search the rest with `GET /jobs/companies`.
    pub companies: Vec<FacetCount>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct CompanySearch {
    /// Part of a company name.
    pub q: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct JobsPage {
    pub items: Vec<JobSummary>,
    /// Jobs matching the filters, across all pages.
    pub total: u32,
    /// Distinct companies among the matching jobs.
    pub company_count: u32,
    pub page: u32,
    pub page_size: u32,
    pub facets: Facets,
}

/// Filters for the job feed. List filters take comma-separated values,
/// e.g. `workplace=remote,hybrid`.
#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct JobsQuery {
    /// Matches title, department and company.
    pub q: Option<String>,
    /// Comma-separated job titles; a job matches when its title has every
    /// word of any one of them, e.g. `software engineer,data engineer`.
    pub title: Option<String>,
    /// Semicolon-separated places, e.g. `Austin, TX; Texas; Remote`. A state
    /// matches by name or code. When `workplace` includes remote, remote
    /// jobs match too.
    pub location: Option<String>,
    /// Comma-separated: onsite, hybrid, remote.
    pub workplace: Option<String>,
    /// Comma-separated: full_time, part_time, contract, internship.
    pub employment_type: Option<String>,
    /// Comma-separated: intern, entry, mid, senior, staff_plus.
    pub seniority: Option<String>,
    /// Comma-separated department names.
    pub department: Option<String>,
    /// Comma-separated company names.
    pub company: Option<String>,
    /// Comma-separated hiring platforms or job boards, e.g. greenhouse,workday,himalayas.
    pub source: Option<String>,
    #[param(inline)]
    pub posted_within: Option<PostedWithin>,
    /// Minimum yearly salary; jobs without a listed salary are excluded
    /// unless `include_unlisted_salary` is set.
    pub salary_min: Option<u32>,
    /// With `salary_min`, keep jobs that don't list a salary.
    pub include_unlisted_salary: Option<bool>,
    /// Comma-separated words or phrases; jobs whose title contains any of
    /// them as whole words are hidden, e.g. `manager,sales`.
    pub exclude_title: Option<String>,
    /// Comma-separated company names to hide.
    pub exclude_company: Option<String>,
    /// Hide jobs that say they won't sponsor a work visa.
    pub hide_no_sponsorship: Option<bool>,
    /// Hide jobs open only to U.S. citizens or security-clearance holders.
    pub hide_citizens_only: Option<bool>,
    /// Hide jobs flagged as possibly stale.
    pub hide_stale: Option<bool>,
    #[param(inline)]
    pub sort: Option<Sort>,
    /// 1-based page number.
    pub page: Option<u32>,
    /// Defaults to 20, at most 100.
    pub page_size: Option<u32>,
}

// --- handlers --------------------------------------------------------------

/// Lists open jobs, filtered, sorted and paginated.
#[utoipa::path(
    get,
    path = "/jobs",
    tag = "jobs",
    params(JobsQuery),
    responses(
        (status = OK, body = JobsPage),
        (status = BAD_REQUEST, body = ApiError),
    )
)]
async fn list_jobs(
    State(state): State<AppState>,
    Query(query): Query<JobsQuery>,
) -> Result<Json<JobsPage>, ApiError> {
    let filter = to_filter(&query)?;
    let sort = match query.sort.unwrap_or_default() {
        Sort::Recommended => JobSort::Recommended,
        Sort::Newest => JobSort::Newest,
        Sort::Salary => JobSort::Salary,
    };
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);
    let page = query.page.unwrap_or(1).clamp(1, 10_000);

    let result = store::list_jobs(&state.db, &filter, sort, page, page_size).await?;
    let mut facets = cached_facets(&state).await?;

    // Keep selected companies in the menu even when they're not in the top list.
    let missing: Vec<String> = filter
        .company
        .iter()
        .filter(|name| {
            !facets
                .companies
                .iter()
                .any(|c| c.value.to_lowercase() == **name)
        })
        .cloned()
        .collect();
    if !missing.is_empty() {
        let extra = store::company_counts(&state.db, None, &missing, missing.len() as i64).await?;
        facets.companies.extend(extra.into_iter().map(facet));
    }

    Ok(Json(JobsPage {
        items: result.rows.into_iter().map(summary).collect(),
        total: result.total as u32,
        company_count: result.company_count as u32,
        page,
        page_size,
        facets,
    }))
}

/// Companies whose name contains `q`, with open-job counts, for the Company
/// filter's search box. At most 30, biggest first.
#[utoipa::path(
    get,
    path = "/jobs/companies",
    tag = "jobs",
    params(CompanySearch),
    responses((status = OK, body = Vec<FacetCount>))
)]
async fn search_companies(
    State(state): State<AppState>,
    Query(query): Query<CompanySearch>,
) -> Result<Json<Vec<FacetCount>>, ApiError> {
    let q = query.q.trim();
    if q.is_empty() {
        return Ok(Json(Vec::new()));
    }
    let rows = store::company_counts(&state.db, Some(q), &[], 30).await?;
    Ok(Json(rows.into_iter().map(facet).collect()))
}

fn facet(c: store::Count) -> FacetCount {
    FacetCount {
        value: c.value,
        count: c.count.max(0) as u32,
    }
}

const FACETS_KEY: &str = "cache:facets:v1";
const FACETS_TTL_SECS: u64 = 300;

/// Facets take a few hundred milliseconds to count over every open job and
/// are the same for every request, so they're cached in Redis for five
/// minutes. Without Redis they're simply computed each time.
async fn cached_facets(state: &AppState) -> Result<Facets, ApiError> {
    use deadpool_redis::redis::AsyncCommands;

    let mut conn = state.redis.get().await.ok();
    if let Some(conn) = conn.as_mut()
        && let Ok(Some(json)) = conn.get::<_, Option<String>>(FACETS_KEY).await
        && let Ok(facets) = serde_json::from_str::<Facets>(&json)
    {
        return Ok(facets);
    }

    let (departments, companies) = store::facets(&state.db).await?;
    let facets = Facets {
        departments: departments.into_iter().map(facet).collect(),
        companies: companies.into_iter().map(facet).collect(),
    };
    if let Some(conn) = conn.as_mut()
        && let Ok(json) = serde_json::to_string(&facets)
    {
        let _: Result<(), _> = conn.set_ex(FACETS_KEY, json, FACETS_TTL_SECS).await;
    }
    Ok(facets)
}

/// One job with its full description. Closed jobs are still returned, with
/// `closed_at` set, so saved jobs can show that they've expired.
#[utoipa::path(
    get,
    path = "/jobs/{id}",
    tag = "jobs",
    params(("id" = String, Path, description = "Job id")),
    responses(
        (status = OK, body = JobDetail),
        (status = NOT_FOUND, body = ApiError),
    )
)]
async fn get_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<JobDetail>, ApiError> {
    let not_found = || ApiError::not_found(format!("No job with id {id}"));
    let uuid = Uuid::parse_str(&id).map_err(|_| not_found())?;
    let row = store::get_job(&state.db, uuid)
        .await?
        .ok_or_else(not_found)?;
    let listings = store::job_listings(&state.db, uuid)
        .await?
        .into_iter()
        .filter_map(|l| {
            Some(Listing {
                source: Source::parse(&l.feed)?,
                url: l.url,
            })
        })
        .collect();

    Ok(Json(JobDetail {
        job: summary(row.job),
        summary: None,
        description_html: row.description_html.unwrap_or_default(),
        skills: Vec::new(),
        yoe_min: None,
        education: None,
        workplace_note: None,
        breakdown: None,
        keywords: None,
        listings,
    }))
}

// --- mapping ---------------------------------------------------------------

fn summary(r: JobRow) -> JobSummary {
    let salary = match (r.salary_min, r.salary_max) {
        (Some(min), Some(max)) => Some(Salary {
            min: min.max(0) as u32,
            max: max.max(0) as u32,
            currency: r.salary_currency.unwrap_or_else(|| "USD".to_owned()),
            period: r
                .salary_period
                .as_deref()
                .and_then(SalaryPeriod::parse)
                .unwrap_or(SalaryPeriod::Year),
        }),
        _ => None,
    };
    JobSummary {
        id: r.id.to_string(),
        title: r.title,
        company: Company {
            name: r.company_name,
            domain: r.company_domain,
            careers_url: r.company_careers_url,
            industries: Vec::new(),
            description: None,
        },
        locations: r.locations,
        workplace: Workplace::parse(&r.workplace).unwrap_or(Workplace::Onsite),
        employment_type: EmploymentType::parse(&r.employment_type)
            .unwrap_or(EmploymentType::FullTime),
        seniority: Seniority::parse(&r.seniority).unwrap_or(Seniority::Mid),
        department: r.department,
        salary,
        posted_at: r.posted_at,
        first_seen_at: r.first_seen_at,
        last_seen_at: r.last_seen_at,
        closed_at: r.closed_at,
        repost_count: r.repost_count.max(0) as u32,
        possibly_stale: r.possibly_stale,
        no_sponsorship: r.no_sponsorship,
        citizens_only: r.citizens_only,
        source: Source::parse(&r.source).unwrap_or(Source::Greenhouse),
        apply_url: r.apply_url,
        fit: None,
    }
}

fn to_filter(q: &JobsQuery) -> Result<JobFilter, ApiError> {
    Ok(JobFilter {
        q: trimmed(&q.q),
        titles: split_trimmed(&q.title),
        locations: search::places(q.location.as_deref().unwrap_or("")),
        workplace: parse_list(&q.workplace, "workplace", Workplace::parse)?,
        employment_type: parse_list(&q.employment_type, "employment_type", EmploymentType::parse)?,
        seniority: parse_list(&q.seniority, "seniority", Seniority::parse)?,
        department: split_lower(&q.department),
        company: split_lower(&q.company),
        source: parse_list(&q.source, "source", Source::parse)?,
        posted_after: q.posted_within.map(|w| Utc::now() - w.duration()),
        salary_min: q.salary_min.map(|v| v.min(i32::MAX as u32) as i32),
        include_unlisted_salary: q.include_unlisted_salary.unwrap_or(false),
        hide_stale: q.hide_stale.unwrap_or(false),
        exclude_title: split_trimmed(&q.exclude_title),
        exclude_company: split_lower(&q.exclude_company),
        hide_no_sponsorship: q.hide_no_sponsorship.unwrap_or(false),
        hide_citizens_only: q.hide_citizens_only.unwrap_or(false),
    })
}

fn trimmed(v: &Option<String>) -> Option<String> {
    v.as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

/// A comma-separated list, trimmed, at most 30 entries.
fn split_trimmed(v: &Option<String>) -> Vec<String> {
    v.as_deref()
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .take(30)
        .map(str::to_owned)
        .collect()
}

fn split_lower(v: &Option<String>) -> Vec<String> {
    v.as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Parses a comma-separated list of enum names.
fn parse_list<T>(
    v: &Option<String>,
    field: &str,
    parse: fn(&str) -> Option<T>,
) -> Result<Vec<T>, ApiError> {
    split_lower(v)
        .into_iter()
        .map(|token| {
            parse(&token)
                .ok_or_else(|| ApiError::bad_request(format!("Unknown {field} value: {token}")))
        })
        .collect()
}
