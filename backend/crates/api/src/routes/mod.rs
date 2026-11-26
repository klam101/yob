use axum::{Json, Router, http::HeaderValue, routing::get};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use yob_core::config::Config;

use crate::{openapi::ApiDoc, state::AppState};

pub mod auth;
mod companies;
mod feeds;
mod health;
mod jobs;
mod preferences;
mod profile;
mod suggest;

/// Every feature adds its routes here with `.merge(feature::routes())`.
pub fn router(state: AppState, config: &Config) -> anyhow::Result<Router> {
    let (router, spec) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(health::routes())
        .merge(auth::routes())
        .merge(companies::routes())
        .merge(feeds::routes())
        .merge(jobs::routes())
        .merge(profile::routes())
        .merge(preferences::routes())
        .merge(suggest::routes())
        .split_for_parts();

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(web_origins(&config.web_origin)?))
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    Ok(router
        .route("/openapi.json", get(move || async move { Json(spec) }))
        .with_state(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http()))
}

/// `WEB_ORIGIN` is a comma-separated list. Each `localhost` origin also
/// allows its `127.0.0.1` twin and the other way round, so the app works
/// whichever of the two the browser was opened at.
fn web_origins(setting: &str) -> anyhow::Result<Vec<HeaderValue>> {
    let mut origins: Vec<String> = Vec::new();
    for origin in setting.split(',').map(str::trim).filter(|o| !o.is_empty()) {
        let twin = if origin.contains("://localhost") {
            Some(origin.replace("://localhost", "://127.0.0.1"))
        } else if origin.contains("://127.0.0.1") {
            Some(origin.replace("://127.0.0.1", "://localhost"))
        } else {
            None
        };
        for o in std::iter::once(origin.to_owned()).chain(twin) {
            if !origins.contains(&o) {
                origins.push(o);
            }
        }
    }
    Ok(origins
        .iter()
        .map(|o| o.parse::<HeaderValue>())
        .collect::<Result<_, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::web_origins;

    #[test]
    fn origins_include_the_loopback_twin() {
        let got = web_origins("http://127.0.0.1:3000, http://localhost:3000").unwrap();
        assert_eq!(got, ["http://127.0.0.1:3000", "http://localhost:3000"]);
        let got = web_origins("http://localhost:3000").unwrap();
        assert_eq!(got, ["http://localhost:3000", "http://127.0.0.1:3000"]);
    }
}
