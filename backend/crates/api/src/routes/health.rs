use axum::{Json, extract::State, http::StatusCode};
use deadpool_redis::redis;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::state::AppState;

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(health))
}

#[derive(Serialize, ToSchema)]
pub struct Health {
    /// "ok" when every dependency answered, otherwise "degraded".
    status: String,
    database: bool,
    redis: bool,
    /// Whether the local Ollama app answers. Not needed for the job feed, so
    /// it doesn't affect `status`.
    ollama: bool,
    /// Models yob needs that Ollama hasn't pulled (`./scripts/dev.sh ollama`).
    missing_models: Vec<String>,
}

/// Checks that the API can reach Postgres, Redis and Ollama.
#[utoipa::path(
    get,
    path = "/health",
    tag = "system",
    responses(
        (status = OK, body = Health),
        (status = SERVICE_UNAVAILABLE, body = Health),
    )
)]
async fn health(State(state): State<AppState>) -> (StatusCode, Json<Health>) {
    let database = db_ping(&state).await;
    let redis = redis_ping(&state).await;
    let ollama = yob_core::ollama::status(&state.ollama_url).await;
    let ok = database && redis;

    let code = if ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    let status = if ok { "ok" } else { "degraded" }.to_owned();
    (
        code,
        Json(Health {
            status,
            database,
            redis,
            ollama: ollama.reachable,
            missing_models: ollama.missing_models,
        }),
    )
}

async fn db_ping(state: &AppState) -> bool {
    yob_db::ping(&state.db).await.is_ok()
}

async fn redis_ping(state: &AppState) -> bool {
    let Ok(mut conn) = state.redis.get().await else {
        return false;
    };
    redis::cmd("PING")
        .query_async::<String>(&mut conn)
        .await
        .is_ok()
}
