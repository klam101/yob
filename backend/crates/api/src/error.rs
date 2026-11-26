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
    /// The field the message is about, as a path like `experiences.2.end`,
    /// when the error is about one field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

impl ApiError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
            field: None,
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

impl From<yob_core::profile::ProfileError> for ApiError {
    fn from(err: yob_core::profile::ProfileError) -> Self {
        use yob_core::profile::ProfileError as E;
        match err {
            E::Invalid(m) => Self::bad_request(m),
            E::Field { field, message } => Self {
                field: Some(field),
                ..Self::bad_request(message)
            },
            E::Database(e) => e.into(),
            E::Decrypt => {
                tracing::error!(error = %err, "profile");
                Self::internal(err.to_string())
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self)).into_response()
    }
}
