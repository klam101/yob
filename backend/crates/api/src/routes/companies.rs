//! Job sources: the companies whose job boards the worker checks.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;
use yob_core::jobs::{
    FetchError, Platform, ingest,
    sources::parse_board_url,
    store::{self, CompanyRow},
};

use crate::{error::ApiError, state::AppState};

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_companies, add_company))
        .routes(routes!(check_company))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CompanySource {
    pub id: String,
    pub name: String,
    pub platform: Platform,
    pub board_token: String,
    pub careers_url: String,
    pub active: bool,
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

#[derive(Debug, Deserialize, ToSchema)]
pub struct AddCompany {
    /// A public job-board URL, e.g. https://boards.greenhouse.io/stripe,
    /// https://jobs.lever.co/palantir or https://jobs.ashbyhq.com/ramp.
    pub url: String,
    /// Display name; defaults to one derived from the board.
    pub name: Option<String>,
}

/// Every tracked company with its open-job count and check status.
#[utoipa::path(
    get,
    path = "/companies",
    tag = "companies",
    responses((status = OK, body = Vec<CompanySource>))
)]
async fn list_companies(
    State(state): State<AppState>,
) -> Result<Json<Vec<CompanySource>>, ApiError> {
    let rows = store::list_companies(&state.db).await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
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
        (status = BAD_REQUEST, body = ApiError, description = "Not a Greenhouse, Lever or Ashby board URL"),
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
            "Paste a Greenhouse, Lever or Ashby job-board link, like https://jobs.lever.co/company",
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

    let name = body
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| display_name(&token));
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

/// "acme-robotics" → "Acme Robotics".
fn display_name(token: &str) -> String {
    token
        .split(['-', '_', '.'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}
