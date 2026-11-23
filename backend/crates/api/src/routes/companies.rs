//! Job sources: the companies whose job boards the worker checks, and the
//! discovery of new ones.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;
use yob_core::jobs::{
    FetchError, Platform, discover, ingest,
    sources::{self, parse_board_url},
    store::{self, CompanyRow, CompanyStatus},
};

use crate::{error::ApiError, state::AppState};

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_companies, add_company))
        .routes(routes!(check_company))
        .routes(routes!(discovery_stats))
        .routes(routes!(add_candidates))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CompanySource {
    pub id: String,
    pub name: String,
    pub platform: Platform,
    pub board_token: String,
    pub careers_url: String,
    pub active: bool,
    /// Open jobs in the US or remote.
    pub open_jobs: u32,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub next_check_at: DateTime<Utc>,
    /// "ok" or "error" after the first check; absent before it.
    pub last_status: Option<String>,
    pub last_error: Option<String>,
}

impl From<CompanyRow> for CompanySource {
    fn from(r: CompanyRow) -> Self {
        Self {
            id: r.id.to_string(),
            name: r.name,
            platform: Platform::parse(&r.platform).unwrap_or(Platform::Greenhouse),
            board_token: r.board_token,
            careers_url: r.careers_url,
            active: r.active,
            open_jobs: r.open_jobs.max(0) as u32,
            last_checked_at: r.last_checked_at,
            next_check_at: r.next_check_at,
            last_status: r.last_status,
            last_error: r.last_error,
        }
    }
}

/// Totals over every tracked company.
#[derive(Debug, Serialize, ToSchema)]
pub struct SourcesSummary {
    pub companies: u32,
    pub open_jobs: u32,
    /// Companies whose last check failed.
    pub errors: u32,
    /// Companies not checked yet.
    pub pending: u32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CompaniesPage {
    pub items: Vec<CompanySource>,
    /// Companies matching the search and status, across all pages.
    pub total: u32,
    pub page: u32,
    pub page_size: u32,
    pub summary: SourcesSummary,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum StatusFilter {
    #[default]
    All,
    Errors,
    Pending,
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct CompaniesQuery {
    /// Matches company name or board token.
    pub q: Option<String>,
    #[param(inline)]
    pub status: Option<StatusFilter>,
    /// 1-based page number.
    pub page: Option<u32>,
    /// Defaults to 50, at most 200.
    pub page_size: Option<u32>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AddCompany {
    /// A public job-board URL, e.g. https://boards.greenhouse.io/stripe,
    /// https://jobs.lever.co/palantir or https://jobs.ashbyhq.com/ramp.
    pub url: String,
    /// Display name; defaults to the name on the board.
    pub name: Option<String>,
}

/// Tracked companies, by name, with open-job counts and check status.
#[utoipa::path(
    get,
    path = "/companies",
    tag = "companies",
    params(CompaniesQuery),
    responses((status = OK, body = CompaniesPage))
)]
async fn list_companies(
    State(state): State<AppState>,
    Query(query): Query<CompaniesQuery>,
) -> Result<Json<CompaniesPage>, ApiError> {
    let page_size = query.page_size.unwrap_or(50).clamp(1, 200);
    let page = query.page.unwrap_or(1).clamp(1, 100_000);
    let status = match query.status.unwrap_or_default() {
        StatusFilter::All => CompanyStatus::All,
        StatusFilter::Errors => CompanyStatus::Errors,
        StatusFilter::Pending => CompanyStatus::Pending,
    };
    let result =
        store::list_companies(&state.db, query.q.as_deref(), status, page, page_size).await?;
    let n = |v: i64| v.max(0) as u32;
    Ok(Json(CompaniesPage {
        items: result.rows.into_iter().map(Into::into).collect(),
        total: n(result.total),
        page,
        page_size,
        summary: SourcesSummary {
            companies: n(result.summary.companies),
            open_jobs: n(result.summary.open_jobs),
            errors: n(result.summary.errors),
            pending: n(result.summary.pending),
        },
    }))
}

/// Starts tracking a company from its job-board URL. The board is fetched
/// once to make sure it exists; the worker imports its jobs within seconds.
#[utoipa::path(
    post,
    path = "/companies",
    tag = "companies",
    request_body = AddCompany,
    responses(
        (status = CREATED, body = CompanySource),
        (status = BAD_REQUEST, body = ApiError, description = "Not a supported job-board URL"),
        (status = NOT_FOUND, body = ApiError, description = "The board doesn't exist"),
        (status = CONFLICT, body = ApiError, description = "Already tracked"),
        (status = BAD_GATEWAY, body = ApiError, description = "The job board couldn't be reached"),
    )
)]
async fn add_company(
    State(state): State<AppState>,
    Json(body): Json<AddCompany>,
) -> Result<(StatusCode, Json<CompanySource>), ApiError> {
    let (platform, token) = parse_board_url(&body.url).ok_or_else(|| {
        ApiError::bad_request(
            "Paste a job-board link from Greenhouse, Lever, Ashby, Workday, SmartRecruiters, \
             Workable, Recruitee or BambooHR, like https://jobs.lever.co/company",
        )
    })?;

    if store::find_company(&state.db, platform, &token)
        .await?
        .is_some()
    {
        return Err(ApiError::conflict(format!(
            "{token} on {} is already tracked",
            platform.as_str()
        )));
    }

    match ingest::probe_board(&state.fetcher, platform, &token).await {
        Ok(_) => {}
        Err(FetchError::NotFound) => {
            return Err(ApiError::not_found(format!(
                "No {} job board named \"{token}\"",
                platform.as_str()
            )));
        }
        Err(e) => {
            return Err(ApiError::bad_gateway(format!(
                "Couldn't reach the job board: {e}"
            )));
        }
    }

    let given = body
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_owned);
    let name = match given {
        Some(name) => name,
        None => sources::board_name(&state.fetcher, platform, &token)
            .await
            .unwrap_or_else(|| sources::display_name(&token)),
    };
    let id = store::insert_company(&state.db, &name, platform, &token).await?;
    let row = store::get_company(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::not_found("Company vanished after insert"))?;
    Ok((StatusCode::CREATED, Json(row.into())))
}

/// Asks the worker to check this company now (within about 15 seconds).
#[utoipa::path(
    post,
    path = "/companies/{id}/check",
    tag = "companies",
    params(("id" = String, Path, description = "Company id")),
    responses(
        (status = ACCEPTED, body = CompanySource),
        (status = NOT_FOUND, body = ApiError),
    )
)]
async fn check_company(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<CompanySource>), ApiError> {
    let not_found = || ApiError::not_found(format!("No company with id {id}"));
    let uuid = Uuid::parse_str(&id).map_err(|_| not_found())?;
    if !store::request_check(&state.db, uuid).await? {
        return Err(not_found());
    }
    let row = store::get_company(&state.db, uuid)
        .await?
        .ok_or_else(not_found)?;
    Ok((StatusCode::ACCEPTED, Json(row.into())))
}

// --- discovery ---------------------------------------------------------------

/// Progress of company discovery.
#[derive(Debug, Serialize, ToSchema)]
pub struct DiscoveryStats {
    /// Boards found but not verified yet.
    pub pending: u32,
    /// Boards verified and now tracked.
    pub added: u32,
    /// Boards that were missing or had no open jobs (retried after 30 days).
    pub rejected: u32,
    pub added_last_week: u32,
    /// Common Crawl index pages read so far.
    pub index_pages_read: u32,
    /// Absent until the size of every index is known.
    pub index_pages_total: Option<u32>,
    pub last_index_read_at: Option<DateTime<Utc>>,
}

/// How discovery is going: candidates found, verified and rejected, and how
/// far the Common Crawl scan has got.
#[utoipa::path(
    get,
    path = "/discovery",
    tag = "companies",
    responses((status = OK, body = DiscoveryStats))
)]
async fn discovery_stats(State(state): State<AppState>) -> Result<Json<DiscoveryStats>, ApiError> {
    let s = discover::stats(&state.db).await?;
    let n = |v: i64| v.max(0) as u32;
    Ok(Json(DiscoveryStats {
        pending: n(s.pending),
        added: n(s.added),
        rejected: n(s.rejected),
        added_last_week: n(s.added_last_week),
        index_pages_read: n(s.index_pages_read),
        index_pages_total: s.index_pages_total.map(n),
        last_index_read_at: s.last_index_read_at,
    }))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AddCandidates {
    /// Job-board URLs, e.g. from a list of companies you're interested in.
    pub urls: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CandidatesAdded {
    /// New boards queued for verification.
    pub queued: u32,
    /// URLs recognized as job boards (some may already be tracked or queued).
    pub recognized: u32,
    /// URLs that aren't Greenhouse, Lever or Ashby boards.
    pub unrecognized: Vec<String>,
}

/// Queues many job-board URLs at once. The worker verifies them in the
/// background and adds the ones with open jobs.
#[utoipa::path(
    post,
    path = "/discovery/candidates",
    tag = "companies",
    request_body = AddCandidates,
    responses(
        (status = OK, body = CandidatesAdded),
        (status = BAD_REQUEST, body = ApiError, description = "More than 10,000 URLs"),
    )
)]
async fn add_candidates(
    State(state): State<AppState>,
    Json(body): Json<AddCandidates>,
) -> Result<Json<CandidatesAdded>, ApiError> {
    if body.urls.len() > 10_000 {
        return Err(ApiError::bad_request("Send at most 10,000 URLs at a time"));
    }
    let mut boards = Vec::new();
    let mut unrecognized = Vec::new();
    for url in body.urls.iter().map(|u| u.trim()).filter(|u| !u.is_empty()) {
        match parse_board_url(url) {
            Some(board) => boards.push(board),
            None => unrecognized.push(url.to_owned()),
        }
    }
    let queued = discover::add_candidates(&state.db, &boards, "manual").await?;
    Ok(Json(CandidatesAdded {
        queued: queued as u32,
        recognized: boards.len() as u32,
        unrecognized,
    }))
}
