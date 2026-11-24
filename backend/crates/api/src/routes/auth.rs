//! Login for the single account. Sign-up works only until the account
//! exists. The session token lives in an HttpOnly cookie; the browser reaches
//! the API through the web app's `/api` proxy, so the cookie is first-party.

use axum::{
    Json,
    extract::{FromRequestParts, State},
    http::{StatusCode, request::Parts},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use yob_core::auth::{self, User};

use crate::{error::ApiError, state::AppState};

const COOKIE: &str = "yob_session";

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(status))
        .routes(routes!(signup))
        .routes(routes!(login))
        .routes(routes!(logout))
        .routes(routes!(me))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Me {
    pub id: String,
    pub email: String,
}

impl From<&User> for Me {
    fn from(u: &User) -> Self {
        Self {
            id: u.id.to_string(),
            email: u.email.clone(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthStatus {
    /// False until the first visit creates the account.
    pub account_exists: bool,
    /// The logged-in user, if any.
    pub user: Option<Me>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct Credentials {
    pub email: String,
    pub password: String,
}

/// The logged-in user. Handlers that take it answer 401 without a session.
pub struct CurrentUser(pub User);

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar
            .get(COOKIE)
            .map(|c| c.value().to_owned())
            .ok_or_else(|| ApiError::unauthorized("Log in to continue"))?;
        auth::session_user(&state.db, &state.redis, &token)
            .await?
            .map(CurrentUser)
            .ok_or_else(|| ApiError::unauthorized("Your session ended. Log in again."))
    }
}

fn session_cookie(token: String) -> Cookie<'static> {
    Cookie::build((COOKIE, token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(
            auth::SESSION_TTL
                .try_into()
                .expect("session TTL fits a cookie max-age"),
        )
        .build()
}

/// Whether the account exists and who is logged in.
#[utoipa::path(get, path = "/auth/status", tag = "auth", responses((status = OK, body = AuthStatus)))]
async fn status(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<AuthStatus>, ApiError> {
    let user = match jar.get(COOKIE) {
        Some(c) => auth::session_user(&state.db, &state.redis, c.value()).await?,
        None => None,
    };
    Ok(Json(AuthStatus {
        account_exists: user.is_some() || auth::account_exists(&state.db).await?,
        user: user.as_ref().map(Me::from),
    }))
}

/// Creates the account. Only works while no account exists.
#[utoipa::path(
    post,
    path = "/auth/signup",
    tag = "auth",
    request_body = Credentials,
    responses(
        (status = CREATED, body = Me),
        (status = BAD_REQUEST, body = ApiError),
        (status = CONFLICT, body = ApiError, description = "An account already exists"),
    )
)]
async fn signup(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<Credentials>,
) -> Result<(StatusCode, CookieJar, Json<Me>), ApiError> {
    let user = auth::create_first_user(&state.db, &body.email, &body.password).await?;
    let token = auth::create_session(&state.redis, user.id).await?;
    tracing::info!("account created");
    Ok((
        StatusCode::CREATED,
        jar.add(session_cookie(token)),
        Json(Me::from(&user)),
    ))
}

/// Logs in and sets the session cookie.
#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "auth",
    request_body = Credentials,
    responses(
        (status = OK, body = Me),
        (status = UNAUTHORIZED, body = ApiError),
        (status = TOO_MANY_REQUESTS, body = ApiError),
    )
)]
async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<Credentials>,
) -> Result<(CookieJar, Json<Me>), ApiError> {
    let user = auth::authenticate(&state.db, &state.redis, &body.email, &body.password).await?;
    let token = auth::create_session(&state.redis, user.id).await?;
    Ok((jar.add(session_cookie(token)), Json(Me::from(&user))))
}

/// Ends the session.
#[utoipa::path(post, path = "/auth/logout", tag = "auth", responses((status = NO_CONTENT)))]
async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(StatusCode, CookieJar), ApiError> {
    if let Some(c) = jar.get(COOKIE) {
        auth::end_session(&state.redis, c.value()).await?;
    }
    Ok((
        StatusCode::NO_CONTENT,
        jar.remove(Cookie::build(COOKIE).path("/")),
    ))
}

/// The logged-in user.
#[utoipa::path(
    get,
    path = "/auth/me",
    tag = "auth",
    responses((status = OK, body = Me), (status = UNAUTHORIZED, body = ApiError))
)]
async fn me(CurrentUser(user): CurrentUser) -> Json<Me> {
    Json(Me::from(&user))
}
