//! Job feed endpoints.
//!
//! STUB: these serve fictional jobs from `fixtures.rs`, filtered and sorted in
//! memory, so the frontend can be built against the real API shape. Real
//! ingestion (Greenhouse, Lever, Ashby) replaces the fixtures later; the types
//! here are the contract and should stay stable.

mod fixtures;

use std::collections::{BTreeMap, BTreeSet};

use axum::{
    Json,
    extract::{Path, Query},
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{error::ApiError, state::AppState};

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_jobs))
        .routes(routes!(get_job))
}

// --- types -----------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Workplace {
    Onsite,
    Hybrid,
    Remote,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EmploymentType {
    FullTime,
    PartTime,
    Contract,
    Internship,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Seniority {
    Intern,
    Entry,
    Mid,
    Senior,
    StaffPlus,
}

/// The hiring platform a job was read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Greenhouse,
    Lever,
    Ashby,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SalaryPeriod {
    Year,
    Hour,
}

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
    pub domain: String,
    pub industries: Vec<String>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Salary {
    pub min: u32,
    pub max: u32,
    pub currency: String,
    pub period: SalaryPeriod,
}

impl Salary {
    /// Yearly equivalent of the top of the range, for sorting and filtering.
    fn annual_max(&self) -> u32 {
        match self.period {
            SalaryPeriod::Year => self.max,
            SalaryPeriod::Hour => self.max * 2080,
        }
    }
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
    pub department: String,
    pub salary: Option<Salary>,
    /// The date the company says it posted the job.
    pub posted_at: DateTime<Utc>,
    /// When yob first saw the job; more trustworthy than `posted_at`.
    pub first_seen_at: DateTime<Utc>,
    /// Times this job closed and reappeared.
    pub repost_count: u32,
    /// Open more than 60 days or reposted more than twice.
    pub possibly_stale: bool,
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
    /// One-paragraph plain-language summary of the role.
    pub summary: String,
    /// Sanitized HTML of the full posting.
    pub description_html: String,
    pub skills: Vec<String>,
    pub yoe_min: Option<u8>,
    pub education: Option<String>,
    /// Extra detail about the workplace arrangement, e.g. in-office days.
    pub workplace_note: Option<String>,
    pub breakdown: Option<MatchBreakdown>,
    pub keywords: Option<KeywordGap>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FacetCount {
    pub value: String,
    pub count: u32,
}

/// Options for filter menus, counted over all open jobs.
#[derive(Debug, Serialize, ToSchema)]
pub struct Facets {
    pub departments: Vec<FacetCount>,
    pub companies: Vec<FacetCount>,
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
    /// Matches title, company, department and skills.
    pub q: Option<String>,
    /// Matches any of the job's locations, e.g. "New York".
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
    /// Comma-separated: greenhouse, lever, ashby.
    pub source: Option<String>,
    #[param(inline)]
    pub posted_within: Option<PostedWithin>,
    /// Minimum yearly salary; jobs without a listed salary are excluded.
    pub salary_min: Option<u32>,
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
async fn list_jobs(Query(query): Query<JobsQuery>) -> Result<Json<JobsPage>, ApiError> {
    let filters = Filters::parse(&query)?;
    let all = fixtures::jobs();

    let mut matching: Vec<&JobSummary> = all
        .iter()
        .filter(|d| filters.matches(&d.job, &d.skills))
        .map(|d| &d.job)
        .collect();

    match query.sort.unwrap_or_default() {
        Sort::Recommended => matching.sort_by_key(|j| {
            (
                std::cmp::Reverse(j.fit.as_ref().map_or(0, |m| m.score)),
                std::cmp::Reverse(j.posted_at),
            )
        }),
        Sort::Newest => matching.sort_by_key(|j| std::cmp::Reverse(j.posted_at)),
        Sort::Salary => matching
            .sort_by_key(|j| std::cmp::Reverse(j.salary.as_ref().map_or(0, Salary::annual_max))),
    }

    let total = matching.len() as u32;
    let company_count = matching
        .iter()
        .map(|j| j.company.name.as_str())
        .collect::<BTreeSet<_>>()
        .len() as u32;
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);
    let page = query.page.unwrap_or(1).max(1);
    let items = matching
        .into_iter()
        .skip(((page - 1) * page_size) as usize)
        .take(page_size as usize)
        .cloned()
        .collect();

    Ok(Json(JobsPage {
        items,
        total,
        company_count,
        page,
        page_size,
        facets: facets(all),
    }))
}

/// One job with its full description and match details.
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
async fn get_job(Path(id): Path<String>) -> Result<Json<JobDetail>, ApiError> {
    fixtures::jobs()
        .iter()
        .find(|d| d.job.id == id)
        .cloned()
        .map(Json)
        .ok_or_else(|| ApiError::not_found(format!("No job with id {id}")))
}

// --- filtering -------------------------------------------------------------

struct Filters {
    q: Option<String>,
    location: Option<String>,
    workplace: Vec<Workplace>,
    employment_type: Vec<EmploymentType>,
    seniority: Vec<Seniority>,
    department: Vec<String>,
    company: Vec<String>,
    source: Vec<Source>,
    posted_after: Option<DateTime<Utc>>,
    salary_min: Option<u32>,
    hide_stale: bool,
}

impl Filters {
    fn parse(q: &JobsQuery) -> Result<Self, ApiError> {
        Ok(Self {
            q: lower(&q.q),
            location: lower(&q.location),
            workplace: parse_list(&q.workplace, "workplace")?,
            employment_type: parse_list(&q.employment_type, "employment_type")?,
            seniority: parse_list(&q.seniority, "seniority")?,
            department: split_lower(&q.department),
            company: split_lower(&q.company),
            source: parse_list(&q.source, "source")?,
            posted_after: q.posted_within.map(|w| Utc::now() - w.duration()),
            salary_min: q.salary_min,
            hide_stale: q.hide_stale.unwrap_or(false),
        })
    }

    fn matches(&self, j: &JobSummary, skills: &[String]) -> bool {
        let any = |list: &[String], value: &str| {
            list.is_empty() || list.iter().any(|v| v == &value.to_lowercase())
        };

        if let Some(q) = &self.q {
            let hit = j.title.to_lowercase().contains(q)
                || j.company.name.to_lowercase().contains(q)
                || j.department.to_lowercase().contains(q)
                || skills.iter().any(|s| s.to_lowercase().contains(q));
            if !hit {
                return false;
            }
        }
        if let Some(loc) = &self.location
            && !j.locations.iter().any(|l| l.to_lowercase().contains(loc))
        {
            return false;
        }
        if let Some(after) = self.posted_after
            && j.posted_at < after
        {
            return false;
        }
        if let Some(min) = self.salary_min
            && j.salary.as_ref().is_none_or(|s| s.annual_max() < min)
        {
            return false;
        }
        (self.workplace.is_empty() || self.workplace.contains(&j.workplace))
            && (self.employment_type.is_empty()
                || self.employment_type.contains(&j.employment_type))
            && (self.seniority.is_empty() || self.seniority.contains(&j.seniority))
            && (self.source.is_empty() || self.source.contains(&j.source))
            && any(&self.department, &j.department)
            && any(&self.company, &j.company.name)
            && !(self.hide_stale && j.possibly_stale)
    }
}

fn lower(v: &Option<String>) -> Option<String> {
    v.as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_lowercase)
}

fn split_lower(v: &Option<String>) -> Vec<String> {
    v.as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Parses a comma-separated list of enum values using their serde names.
fn parse_list<T: DeserializeOwned>(v: &Option<String>, field: &str) -> Result<Vec<T>, ApiError> {
    split_lower(v)
        .into_iter()
        .map(|token| {
            serde_json::from_value(serde_json::Value::String(token.clone()))
                .map_err(|_| ApiError::bad_request(format!("Unknown {field} value: {token}")))
        })
        .collect()
}

fn facets(all: &[JobDetail]) -> Facets {
    let count = |key: fn(&JobSummary) -> &str| {
        let mut counts = BTreeMap::<&str, u32>::new();
        for d in all {
            *counts.entry(key(&d.job)).or_default() += 1;
        }
        counts
            .into_iter()
            .map(|(value, count)| FacetCount {
                value: value.to_owned(),
                count,
            })
            .collect()
    };
    Facets {
        departments: count(|j| &j.department),
        companies: count(|j| &j.company.name),
    }
}
