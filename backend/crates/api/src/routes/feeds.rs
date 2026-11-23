//! Job boards with public APIs (Himalayas, Remotive, Remote OK, USAJOBS):
//! their check status and what their listings added. The worker reads them
//! (see `yob_core::jobs::feeds`).

use axum::{Json, extract::State};
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use yob_core::jobs::{Feed, store};

use crate::{error::ApiError, state::AppState};

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(list_feeds))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FeedStatus {
    pub id: Feed,
    /// False when the board needs an API key that isn't set in `.env`.
    pub enabled: bool,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub next_check_at: DateTime<Utc>,
    /// "ok" or "error" after the first check; absent before it.
    pub last_status: Option<String>,
    pub last_error: Option<String>,
    /// When the last full pass over the board finished.
    pub last_pass_at: Option<DateTime<Utc>>,
    /// A full pass is part way through (big boards take several checks).
    pub passing: bool,
    /// Open jobs (US + remote) that only this board lists.
    pub own_jobs: u32,
    /// Listings matched to a job we already had from an employer's board.
    pub matched: u32,
    /// Listings at tracked employers whose own board doesn't have the job;
    /// not shown, since the employer's board is trusted.
    pub skipped: u32,
}

/// Every job board, with its check status and listing counts.
#[utoipa::path(
    get,
    path = "/feeds",
    tag = "feeds",
    responses((status = OK, body = Vec<FeedStatus>))
)]
async fn list_feeds(State(state): State<AppState>) -> Result<Json<Vec<FeedStatus>>, ApiError> {
    let n = |v: i64| v.max(0) as u32;
    let rows = store::list_feeds(&state.db).await?;
    Ok(Json(
        rows.into_iter()
            .filter_map(|r| {
                Some(FeedStatus {
                    id: Feed::parse(&r.id)?,
                    enabled: r.active,
                    last_checked_at: r.last_checked_at,
                    next_check_at: r.next_check_at,
                    last_status: r.last_status,
                    last_error: r.last_error,
                    last_pass_at: r.last_pass_at,
                    passing: r.passing,
                    own_jobs: n(r.own_jobs),
                    matched: n(r.matched),
                    skipped: n(r.skipped),
                })
            })
            .collect(),
    ))
}
