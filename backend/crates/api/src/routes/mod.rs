use axum::{Json, Router, http::HeaderValue, routing::get};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use yob_core::config::Config;

use crate::{openapi::ApiDoc, state::AppState};

mod health;
mod jobs;

/// Every feature adds its routes here with `.merge(feature::routes())`.
pub fn router(state: AppState, config: &Config) -> anyhow::Result<Router> {
    let (router, spec) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(health::routes())
        .merge(jobs::routes())
        .split_for_parts();

    let cors = CorsLayer::new()
        .allow_origin(config.web_origin.parse::<HeaderValue>()?)
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    Ok(router
        .route("/openapi.json", get(move || async move { Json(spec) }))
        .with_state(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http()))
}
