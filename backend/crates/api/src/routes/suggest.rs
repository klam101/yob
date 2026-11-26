//! Type-ahead suggestions for profile and resume fields (`core::suggest`).

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use utoipa::IntoParams;
use utoipa_axum::{router::OpenApiRouter, routes};
use yob_core::{
    profile,
    suggest::{self, SuggestKind, Suggestion},
};

use crate::{error::ApiError, routes::auth::CurrentUser, state::AppState};

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(get_suggestions))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct SuggestQuery {
    /// What's typed so far; empty gives the most common entries.
    #[serde(default)]
    pub q: String,
    /// How many to return (1 to 20, default 8).
    pub limit: Option<usize>,
}

/// Suggestions for a field as the user types: cities ("City, ST"),
/// schools, companies or job titles, best first. Values already in the
/// user's profile that match come first.
#[utoipa::path(
    get,
    path = "/suggest/{kind}",
    tag = "suggest",
    params(("kind" = SuggestKind, Path, description = "Which list"), SuggestQuery),
    responses((status = OK, body = Vec<Suggestion>), (status = UNAUTHORIZED, body = ApiError))
)]
async fn get_suggestions(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(kind): Path<SuggestKind>,
    Query(query): Query<SuggestQuery>,
) -> Result<Json<Vec<Suggestion>>, ApiError> {
    let q: String = query.q.chars().take(100).collect();
    let p = profile::load(&state.db, &state.profile_key, user.id).await?;
    let mut own: Vec<String> = match kind {
        SuggestKind::Location => {
            let me = &p.personal;
            let home = [me.city.as_str(), me.region.as_str()]
                .iter()
                .filter(|s| !s.is_empty())
                .copied()
                .collect::<Vec<_>>()
                .join(", ");
            std::iter::once(home)
                .chain(p.experiences.iter().map(|x| x.location.clone()))
                .chain(p.educations.iter().map(|e| e.location.clone()))
                .collect()
        }
        SuggestKind::School => p.educations.iter().map(|e| e.school.clone()).collect(),
        SuggestKind::Company => p.experiences.iter().map(|x| x.company.clone()).collect(),
        SuggestKind::Title => p.experiences.iter().map(|x| x.title.clone()).collect(),
    };
    own.retain(|v| !v.trim().is_empty());
    let out = suggest::suggest(&state.db, kind, &q, query.limit.unwrap_or(8), &own).await?;
    Ok(Json(out))
}
