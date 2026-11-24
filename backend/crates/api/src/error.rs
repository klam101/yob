use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;

/// Error body returned by every endpoint on failure.
#[derive(Debug, Serialize, ToSchema)]
pub struct ApiError {
    #[serde(skip)]
    status: StatusCode,
    pub message: String,
}

impl ApiError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, message)
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, message)
    }

    pub fn too_many_requests(message: impl Into<String>) -> Self {
        Self::new(StatusCode::TOO_MANY_REQUESTS, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
    }

    /// Something we depend on (a job board) failed.
    pub fn bad_gateway(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_GATEWAY, message)
    }
}

/// Database errors are logged in full and reported without internals.
impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!(error = %err, "database error");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "Database error")
    }
}

impl From<yob_core::auth::AuthError> for ApiError {
    fn from(err: yob_core::auth::AuthError) -> Self {
        use yob_core::auth::AuthError as E;
        match err {
            E::AccountExists => Self::conflict(err.to_string()),
            E::WrongCredentials => Self::unauthorized(err.to_string()),
            E::TooManyAttempts => Self::too_many_requests(err.to_string()),
            E::Invalid(_) => Self::bad_request(err.to_string()),
            E::Database(e) => e.into(),
            E::Redis(_) | E::Pool(_) | E::Hash => {
                tracing::error!(error = %err, "auth error");
                Self::internal("Login is unavailable right now")
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self)).into_response()
    }
}
