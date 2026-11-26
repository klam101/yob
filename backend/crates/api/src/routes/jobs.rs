//! Job feed endpoints. They only read the job database; the worker keeps it
//! fresh (see `yob_core::jobs::ingest`).

use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;
use yob_core::jobs::{
    ApplyProcess, EmploymentType, SalaryPeriod, Seniority, Source, Workplace, extract, functions,
    search,
    store::{self, AnyTags, EduRule, JobFilter, JobRow, JobSort},
};

use crate::{error::ApiError, state::AppState};

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_jobs))
        .routes(routes!(search_companies))
        .routes(routes!(vocabulary))
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
    #[serde(rename = "3w")]
    ThreeWeeks,
    #[serde(rename = "1m")]
    Month,
    #[serde(rename = "2m")]
    TwoMonths,
    #[serde(rename = "3m")]
    ThreeMonths,
    #[serde(rename = "6m")]
    SixMonths,
    #[serde(rename = "1y")]
    Year,
}

impl PostedWithin {
    fn duration(self) -> Duration {
        match self {
            Self::Day => Duration::hours(24),
            Self::ThreeDays => Duration::days(3),
            Self::Week => Duration::weeks(1),
            Self::TwoWeeks => Duration::weeks(2),
            Self::ThreeWeeks => Duration::weeks(3),
            Self::Month => Duration::days(30),
            Self::TwoMonths => Duration::days(60),
            Self::ThreeMonths => Duration::days(90),
            Self::SixMonths => Duration::days(182),
            Self::Year => Duration::days(365),
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
    /// Comma-separated: intern, entry, mid, senior, staff_plus, director_plus.
    pub seniority: Option<String>,
    /// Comma-separated department names.
    pub department: Option<String>,
    /// Comma-separated company names.
    pub company: Option<String>,
    /// Comma-separated hiring platforms or job boards, e.g. greenhouse,workday,himalayas.
    pub source: Option<String>,
    #[param(inline)]
    pub posted_within: Option<PostedWithin>,
    /// Posted on or after this day, e.g. `2026-11-01`.
    pub posted_from: Option<NaiveDate>,
    /// Posted on or before this day.
    pub posted_to: Option<NaiveDate>,
    /// Comma-separated: simple (a short form, no account), account (the
    /// applicant creates an account first, as on Workday).
    pub apply: Option<String>,
    /// Only jobs whose listed pay is per year or per hour.
    #[param(inline)]
    pub pay_period: Option<SalaryPeriod>,
    /// Only jobs that list a salary.
    pub salary_listed: Option<bool>,
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
    /// Comma-separated job functions (see `GET /jobs/vocabulary`), e.g.
    /// `software,data`.
    pub function: Option<String>,
    /// Least years of experience the job asks for, at least this.
    pub years_min: Option<u8>,
    /// ...and at most this (10 or more means no upper limit).
    pub years_max: Option<u8>,
    /// With a years range, keep jobs that don't say.
    pub years_unlisted: Option<bool>,
    /// `ic` (individual contributor) or `manager` (manages people).
    pub role: Option<String>,
    /// Comma-separated `level:state` pairs; a job matches each level named.
    /// Levels: associate, bachelors, masters, doctorate. States: required,
    /// preferred, not (not mentioned). E.g. `bachelors:required,bachelors:not`.
    pub edu: Option<String>,
    /// Comma-separated perks the posting mentions, all of them: pto,
    /// four_day_week, 401k_match, parental_leave, retirement, tuition,
    /// visa_sponsorship, relocation.
    pub perk: Option<String>,
    /// Comma-separated clearance levels, any of them: confidential, secret,
    /// top_secret, ts_sci, public_trust, interim, polygraph, none (no
    /// clearance mentioned).
    pub clearance: Option<String>,
    /// Hide jobs that mention a security clearance.
    pub hide_clearance: Option<bool>,
    /// Comma-separated licenses or certifications, any of them (e.g. `rn,pmp`).
    pub cert: Option<String>,
    /// Comma-separated licenses or certifications to hide.
    pub x_cert: Option<String>,
    /// Hide jobs that mention any license or certification.
    pub no_cert: Option<bool>,
    /// Comma-separated spoken languages, any of them (e.g. `spanish`).
    pub lang: Option<String>,
    /// Comma-separated spoken languages to hide.
    pub x_lang: Option<String>,
    /// Comma-separated groups the posting welcomes, any: veterans, fair_chance.
    pub encourage: Option<String>,
    /// Comma-separated shifts the posting mentions, all of them: day,
    /// evening, night, weekend, holiday, overtime, on_call.
    pub shift: Option<String>,
    /// Comma-separated shifts to hide.
    pub x_shift: Option<String>,
    /// Comma-separated travel levels, any: none, minimal, moderate, extensive.
    pub travel: Option<String>,
    /// Comma-separated skills, any of them (e.g. `rust,react`).
    pub skill: Option<String>,
    /// Comma-separated skills to hide.
    pub x_skill: Option<String>,
    /// Hide staffing and recruiting agencies.
    pub hide_staffing: Option<bool>,
    /// Comma-separated, any: temporary, seasonal.
    pub commit: Option<String>,
    /// Comma-separated graduation years the posting names, any (e.g. `2026`).
    pub grad: Option<String>,
    /// Comma-separated internship starts, any (e.g. `2026-summer`).
    pub start: Option<String>,
    /// Semicolon-separated company industries (from Wikidata), any of them.
    /// Companies without known facts are left out.
    pub industry: Option<String>,
    /// Semicolon-separated industries to hide.
    pub x_industry: Option<String>,
    /// Comma-separated organization types, any: public, private, nonprofit,
    /// government, cooperative.
    pub org_type: Option<String>,
    /// Comma-separated organization types to hide.
    pub x_org_type: Option<String>,
    /// Comma-separated headcount bands, any: startup (1-50), small (51-200),
    /// medium (201-1,000), large (1,001-10,000), enterprise (10,001+).
    pub size: Option<String>,
    /// Founded in or after this year.
    pub founded_min: Option<i16>,
    /// Founded in or before this year.
    pub founded_max: Option<i16>,
    /// Semicolon-separated headquarters countries, any of them.
    pub hq: Option<String>,
    /// Semicolon-separated headquarters countries to hide.
    pub x_hq: Option<String>,
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

/// A job function with its open-job count.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FunctionCount {
    pub value: String,
    pub group: String,
    pub label: String,
    pub count: u32,
}

/// A detail tag (`skill:rust`, `perk:401k_match`...) with its open-job count.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TagCount {
    pub value: String,
    pub label: String,
    pub count: u32,
}

/// The options of the filters read from job details: job functions in
/// display order, and every tag with its label, grouped by prefix.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Vocabulary {
    pub functions: Vec<FunctionCount>,
    pub tags: Vec<TagCount>,
    /// Company industries (from Wikidata), by open jobs.
    pub industries: Vec<FacetCount>,
    /// Headquarters countries, by open jobs.
    pub countries: Vec<FacetCount>,
    /// Companies with open jobs whose facts are known, out of `companies_total`.
    pub companies_known: u32,
    pub companies_total: u32,
}

const VOCABULARY_KEY: &str = "cache:vocabulary:v2";
const VOCABULARY_TTL_SECS: u64 = 1800;

/// Job functions and detail tags with open-job counts (cached 30 minutes),
/// for the All filters dialog.
#[utoipa::path(
    get,
    path = "/jobs/vocabulary",
    tag = "jobs",
    responses((status = OK, body = Vocabulary))
)]
async fn vocabulary(State(state): State<AppState>) -> Result<Json<Vocabulary>, ApiError> {
    use deadpool_redis::redis::AsyncCommands;

    let mut conn = state.redis.get().await.ok();
    if let Some(conn) = conn.as_mut()
        && let Ok(Some(json)) = conn.get::<_, Option<String>>(VOCABULARY_KEY).await
        && let Ok(v) = serde_json::from_str::<Vocabulary>(&json)
    {
        return Ok(Json(v));
    }

    let (function_counts, tag_counts) = store::detail_counts(&state.db).await?;
    let company = store::company_fact_counts(&state.db).await?;
    let count = |counts: &[store::Count], value: &str| {
        counts
            .iter()
            .find(|c| c.value == value)
            .map_or(0, |c| c.count.max(0) as u32)
    };
    let v = Vocabulary {
        functions: functions::FUNCTIONS
            .iter()
            .map(|f| FunctionCount {
                value: f.value.to_owned(),
                group: f.group.to_owned(),
                label: f.label.to_owned(),
                count: count(&function_counts, f.value),
            })
            .collect(),
        tags: extract::vocabulary()
            .into_iter()
            .map(|t| TagCount {
                value: t.value.to_owned(),
                label: t.label.to_owned(),
                count: count(&tag_counts, t.value),
            })
            .collect(),
        industries: company.industries.into_iter().map(facet).collect(),
        countries: company.countries.into_iter().map(facet).collect(),
        companies_known: company.known.max(0) as u32,
        companies_total: company.total.max(0) as u32,
    };
    if let Some(conn) = conn.as_mut()
        && let Ok(json) = serde_json::to_string(&v)
    {
        let _: Result<(), _> = conn.set_ex(VOCABULARY_KEY, json, VOCABULARY_TTL_SECS).await;
    }
    Ok(Json(v))
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
    let (tags_any, tags_all, tags_none) = tag_filters(q)?;
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
        posted_from: q.posted_from,
        posted_to: q.posted_to,
        apply: parse_list(&q.apply, "apply", ApplyProcess::parse)?,
        pay_period: q.pay_period,
        salary_listed: q.salary_listed.unwrap_or(false),
        salary_min: q.salary_min.map(|v| v.min(i32::MAX as u32) as i32),
        include_unlisted_salary: q.include_unlisted_salary.unwrap_or(false),
        hide_stale: q.hide_stale.unwrap_or(false),
        exclude_title: split_trimmed(&q.exclude_title),
        exclude_company: split_lower(&q.exclude_company),
        hide_no_sponsorship: q.hide_no_sponsorship.unwrap_or(false),
        hide_citizens_only: q.hide_citizens_only.unwrap_or(false),
        functions: split_lower(&q.function),
        years_min: q.years_min.map(i16::from),
        years_max: q.years_max.filter(|&m| m < 10).map(i16::from),
        years_unlisted: q.years_unlisted.unwrap_or(false),
        tags_any,
        tags_all,
        tags_none,
        edu: edu_rules(&q.edu)?,
        hide_staffing: q.hide_staffing.unwrap_or(false),
        industries: split_semi(&q.industry),
        exclude_industries: split_semi(&q.x_industry),
        org_types: org_types(&q.org_type)?,
        exclude_org_types: org_types(&q.x_org_type)?,
        sizes: split_lower(&q.size)
            .into_iter()
            .map(|band| {
                size_range(&band)
                    .ok_or_else(|| ApiError::bad_request(format!("Unknown size value: {band}")))
            })
            .collect::<Result<_, _>>()?,
        founded_min: q.founded_min,
        founded_max: q.founded_max,
        hq_countries: split_semi(&q.hq),
        exclude_hq_countries: split_semi(&q.x_hq),
    })
}

/// A `;`-separated list, trimmed, at most 30 entries (values may hold commas).
fn split_semi(v: &Option<String>) -> Vec<String> {
    v.as_deref()
        .unwrap_or("")
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .take(30)
        .map(str::to_owned)
        .collect()
}

fn org_types(v: &Option<String>) -> Result<Vec<String>, ApiError> {
    const ALL: [&str; 5] = [
        "public",
        "private",
        "nonprofit",
        "government",
        "cooperative",
    ];
    split_lower(v)
        .into_iter()
        .map(|t| {
            if ALL.contains(&t.as_str()) {
                Ok(t)
            } else {
                Err(ApiError::bad_request(format!(
                    "Unknown org_type value: {t}"
                )))
            }
        })
        .collect()
}

/// Headcount band -> inclusive range (the preferences' company sizes).
fn size_range(band: &str) -> Option<(i32, i32)> {
    Some(match band {
        "startup" => (1, 50),
        "small" => (51, 200),
        "medium" => (201, 1_000),
        "large" => (1_001, 10_000),
        "enterprise" => (10_001, i32::MAX),
        _ => return None,
    })
}

/// `perk=pto,tuition` -> `["perk:pto", "perk:tuition"]`. Values are slugs;
/// anything else is dropped.
fn tags(v: &Option<String>, prefix: &str) -> Vec<String> {
    split_lower(v)
        .into_iter()
        .filter(|t| {
            t.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        })
        .take(40)
        .map(|t| format!("{prefix}:{t}"))
        .collect()
}

/// Any of the values; `none` means "no tag of this kind", given as `without`.
fn any_or_none(v: &Option<String>, prefix: &str, without: &[String]) -> Option<AnyTags> {
    let all = tags(v, prefix);
    if all.is_empty() {
        return None;
    }
    let none = format!("{prefix}:none");
    Some(AnyTags {
        or_without: if all.contains(&none) {
            without.to_vec()
        } else {
            Vec::new()
        },
        tags: all.into_iter().filter(|t| *t != none).collect(),
    })
}

type TagFilters = (Vec<AnyTags>, Vec<String>, Vec<String>);

/// The tag filters, as (any-of groups, all-of, none-of).
fn tag_filters(q: &JobsQuery) -> Result<TagFilters, ApiError> {
    let mut any = Vec::new();
    let mut all = Vec::new();
    let mut none = Vec::new();
    let travel_levels: Vec<String> = ["minimal", "moderate", "extensive"]
        .iter()
        .map(|t| format!("travel:{t}"))
        .collect();
    any.extend(any_or_none(
        &q.clearance,
        "clearance",
        &["clearance:any".to_owned()],
    ));
    any.extend(any_or_none(&q.travel, "travel", &travel_levels));
    for (v, prefix) in [
        (&q.cert, "cert"),
        (&q.lang, "lang"),
        (&q.encourage, "encourage"),
        (&q.skill, "skill"),
        (&q.commit, "commit"),
        (&q.grad, "grad"),
        (&q.start, "start"),
    ] {
        let t = tags(v, prefix);
        if !t.is_empty() {
            any.push(AnyTags {
                tags: t,
                or_without: Vec::new(),
            });
        }
    }
    all.extend(tags(&q.perk, "perk"));
    all.extend(tags(&q.shift, "shift"));
    for (v, prefix) in [
        (&q.x_cert, "cert"),
        (&q.x_lang, "lang"),
        (&q.x_shift, "shift"),
        (&q.x_skill, "skill"),
    ] {
        none.extend(tags(v, prefix));
    }
    if q.hide_clearance == Some(true) {
        none.push("clearance:any".into());
    }
    if q.no_cert == Some(true) {
        none.push("cert:any".into());
    }
    match q.role.as_deref().map(str::trim) {
        None | Some("") => {}
        Some("manager") => all.push("role:manager".into()),
        Some("ic") => none.push("role:manager".into()),
        Some(other) => {
            return Err(ApiError::bad_request(format!(
                "Unknown role value: {other}"
            )));
        }
    }
    Ok((any, all, none))
}

/// `edu=bachelors:required,bachelors:preferred` -> one rule per level.
fn edu_rules(v: &Option<String>) -> Result<Vec<EduRule>, ApiError> {
    let mut rules: Vec<EduRule> = Vec::new();
    for pair in split_lower(v) {
        let bad = || ApiError::bad_request(format!("Unknown edu value: {pair}"));
        let (level, state) = pair.split_once(':').ok_or_else(bad)?;
        if !["associate", "bachelors", "masters", "doctorate"].contains(&level) {
            return Err(bad());
        }
        let i = match rules.iter().position(|r| r.level == level) {
            Some(i) => i,
            None => {
                rules.push(EduRule {
                    level: level.to_owned(),
                    ..EduRule::default()
                });
                rules.len() - 1
            }
        };
        match state {
            "required" => rules[i].required = true,
            "preferred" => rules[i].preferred = true,
            "not" => rules[i].not_mentioned = true,
            _ => return Err(bad()),
        }
    }
    Ok(rules)
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
