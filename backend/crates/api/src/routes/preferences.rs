//! Search preferences from the onboarding questionnaire. They become the
//! job feed's default filters. Every route needs a session.

use axum::{Json, extract::State};
use utoipa_axum::{router::OpenApiRouter, routes};
use yob_core::profile::preferences::{self, Preferences, SearchPreferences};

use super::auth::CurrentUser;
use crate::{error::ApiError, state::AppState};

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_preferences, put_preferences))
        .routes(routes!(skip_preferences))
}

/// The saved preferences and whether the questionnaire was done, skipped or
/// never started.
#[utoipa::path(
    get,
    path = "/preferences",
    tag = "preferences",
    responses(
        (status = OK, body = SearchPreferences),
        (status = UNAUTHORIZED, body = ApiError),
    )
)]
async fn get_preferences(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<SearchPreferences>, ApiError> {
    Ok(Json(preferences::load(&state.db, user.id).await?))
}

/// Saves every answer and marks the questionnaire done. Work authorization
/// is saved with the application answers, which autofill also uses.
#[utoipa::path(
    put,
    path = "/preferences",
    tag = "preferences",
    request_body = Preferences,
    responses(
        (status = OK, body = SearchPreferences),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
    )
)]
async fn put_preferences(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<Preferences>,
) -> Result<Json<SearchPreferences>, ApiError> {
    Ok(Json(preferences::save(&state.db, user.id, &body).await?))
}

/// "Skip for now": stops the feed from sending the user to the
/// questionnaire. Anything saved before is kept.
#[utoipa::path(
    post,
    path = "/preferences/skip",
    tag = "preferences",
    responses(
        (status = OK, body = SearchPreferences),
        (status = UNAUTHORIZED, body = ApiError),
    )
)]
async fn skip_preferences(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<SearchPreferences>, ApiError> {
    Ok(Json(preferences::skip(&state.db, user.id).await?))
}
