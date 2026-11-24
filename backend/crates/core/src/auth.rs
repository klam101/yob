//! The login account and sessions.
//!
//! yob has one account. Sign-up only works while no account exists (the
//! first visit creates it); after that there is only login. Sessions are
//! random tokens held in a cookie; Redis stores only their SHA-256 hash, so a
//! Redis dump can't be replayed as a login.

use std::time::Duration;

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use deadpool_redis::redis;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

/// How long a session lasts without use. Each request refreshes it.
pub const SESSION_TTL: Duration = Duration::from_secs(30 * 24 * 60 * 60);
/// Failed logins allowed per email within `LOGIN_WINDOW`.
const MAX_LOGIN_FAILURES: i64 = 5;
const LOGIN_WINDOW: Duration = Duration::from_secs(15 * 60);
pub const MIN_PASSWORD_LEN: usize = 10;

#[derive(Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub email: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("An account already exists. Log in instead.")]
    AccountExists,
    #[error("Wrong email or password.")]
    WrongCredentials,
    #[error("Too many failed attempts. Try again in 15 minutes.")]
    TooManyAttempts,
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("session store unavailable")]
    Redis(#[from] redis::RedisError),
    #[error("session store unavailable")]
    Pool(#[from] deadpool_redis::PoolError),
    #[error("password hashing failed")]
    Hash,
}

pub fn hash_password(password: &str) -> Result<String, AuthError> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|h| h.to_string())
        .map_err(|_| AuthError::Hash)
}

pub fn verify_password(hash: &str, password: &str) -> bool {
    PasswordHash::new(hash).is_ok_and(|parsed| {
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    })
}

/// Trims and lowercases the email and checks both fields.
pub fn validate(email: &str, password: &str) -> Result<String, AuthError> {
    let email = email.trim().to_lowercase();
    let valid_email = email.len() <= 254
        && email
            .split_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.'));
    if !valid_email {
        return Err(AuthError::Invalid("Enter a valid email address.".into()));
    }
    if password.chars().count() < MIN_PASSWORD_LEN {
        return Err(AuthError::Invalid(format!(
            "Use a password of at least {MIN_PASSWORD_LEN} characters."
        )));
    }
    if password.len() > 1024 {
        return Err(AuthError::Invalid("That password is too long.".into()));
    }
    Ok(email)
}

pub async fn account_exists(pool: &PgPool) -> sqlx::Result<bool> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users)")
        .fetch_one(pool)
        .await
}

/// Creates the account, but only if there is none yet. The table lock makes
/// two sign-ups at the same moment safe: the second sees the first's row.
pub async fn create_first_user(
    pool: &PgPool,
    email: &str,
    password: &str,
) -> Result<User, AuthError> {
    let email = validate(email, password)?;
    let hash = hash_password(password)?;
    let mut tx = pool.begin().await?;
    sqlx::query("LOCK TABLE users IN SHARE ROW EXCLUSIVE MODE")
        .execute(&mut *tx)
        .await?;
    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users)")
        .fetch_one(&mut *tx)
        .await?;
    if exists {
        return Err(AuthError::AccountExists);
    }
    let id: Uuid =
        sqlx::query_scalar("INSERT INTO users (email, password_hash) VALUES ($1, $2) RETURNING id")
            .bind(&email)
            .bind(&hash)
            .fetch_one(&mut *tx)
            .await?;
    tx.commit().await?;
    Ok(User { id, email })
}

/// Checks the password, with failed attempts limited per email.
pub async fn authenticate(
    pool: &PgPool,
    redis: &deadpool_redis::Pool,
    email: &str,
    password: &str,
) -> Result<User, AuthError> {
    let email = email.trim().to_lowercase();
    let key = format!("login_failures:{}", hex::encode(Sha256::digest(&email)));
    let mut conn = redis.get().await?;
    let failures: Option<i64> = redis::cmd("GET").arg(&key).query_async(&mut conn).await?;
    if failures.unwrap_or(0) >= MAX_LOGIN_FAILURES {
        return Err(AuthError::TooManyAttempts);
    }

    let row: Option<(Uuid, String)> =
        sqlx::query_as("SELECT id, password_hash FROM users WHERE email = $1")
            .bind(&email)
            .fetch_optional(pool)
            .await?;
    // Hash even when the email is unknown, so timing doesn't reveal it.
    let ok = match &row {
        Some((_, hash)) => verify_password(hash, password),
        None => {
            let _ = hash_password(password);
            false
        }
    };
    if !ok {
        let _: i64 = redis::cmd("INCR").arg(&key).query_async(&mut conn).await?;
        let _: bool = redis::cmd("EXPIRE")
            .arg(&key)
            .arg(LOGIN_WINDOW.as_secs())
            .arg("NX")
            .query_async(&mut conn)
            .await?;
        return Err(AuthError::WrongCredentials);
    }
    let _: i64 = redis::cmd("DEL").arg(&key).query_async(&mut conn).await?;
    let (id, _) = row.expect("checked above");
    Ok(User { id, email })
}

/// Sets a new password (the `reset-password` command). Ends every session.
pub async fn set_password(
    pool: &PgPool,
    redis: &deadpool_redis::Pool,
    email: &str,
    password: &str,
) -> Result<(), AuthError> {
    let email = validate(email, password)?;
    let hash = hash_password(password)?;
    let id: Option<Uuid> = sqlx::query_scalar(
        "UPDATE users SET password_hash = $2, updated_at = now() WHERE email = $1 RETURNING id",
    )
    .bind(&email)
    .bind(&hash)
    .fetch_optional(pool)
    .await?;
    let id = id.ok_or_else(|| AuthError::Invalid(format!("No account with email {email}.")))?;
    end_all_sessions(redis, id).await
}

fn session_key(token: &str) -> String {
    format!("session:{}", hex::encode(Sha256::digest(token.as_bytes())))
}

fn user_sessions_key(user_id: Uuid) -> String {
    format!("user_sessions:{user_id}")
}

/// Starts a session and returns the token for the cookie.
pub async fn create_session(
    redis: &deadpool_redis::Pool,
    user_id: Uuid,
) -> Result<String, AuthError> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| AuthError::Hash)?;
    let token = hex::encode(bytes);
    let key = session_key(&token);
    let mut conn = redis.get().await?;
    let _: () = redis::pipe()
        .cmd("SET")
        .arg(&key)
        .arg(user_id.to_string())
        .arg("EX")
        .arg(SESSION_TTL.as_secs())
        .ignore()
        .cmd("SADD")
        .arg(user_sessions_key(user_id))
        .arg(&key)
        .ignore()
        .cmd("EXPIRE")
        .arg(user_sessions_key(user_id))
        .arg(SESSION_TTL.as_secs())
        .ignore()
        .query_async(&mut conn)
        .await?;
    Ok(token)
}

/// The user a session token belongs to, refreshing its expiry.
pub async fn session_user(
    pool: &PgPool,
    redis: &deadpool_redis::Pool,
    token: &str,
) -> Result<Option<User>, AuthError> {
    if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Ok(None);
    }
    let key = session_key(token);
    let mut conn = redis.get().await?;
    let id: Option<String> = redis::cmd("GETEX")
        .arg(&key)
        .arg("EX")
        .arg(SESSION_TTL.as_secs())
        .query_async(&mut conn)
        .await?;
    let Some(id) = id.and_then(|s| Uuid::parse_str(&s).ok()) else {
        return Ok(None);
    };
    let email: Option<String> = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(email.map(|email| User { id, email }))
}

pub async fn end_session(redis: &deadpool_redis::Pool, token: &str) -> Result<(), AuthError> {
    let mut conn = redis.get().await?;
    let _: i64 = redis::cmd("DEL")
        .arg(session_key(token))
        .query_async(&mut conn)
        .await?;
    Ok(())
}

async fn end_all_sessions(redis: &deadpool_redis::Pool, user_id: Uuid) -> Result<(), AuthError> {
    let mut conn = redis.get().await?;
    let set = user_sessions_key(user_id);
    let keys: Vec<String> = redis::cmd("SMEMBERS")
        .arg(&set)
        .query_async(&mut conn)
        .await?;
    let mut pipe = redis::pipe();
    for key in keys {
        pipe.cmd("DEL").arg(key).ignore();
    }
    pipe.cmd("DEL").arg(set).ignore();
    let _: () = pipe.query_async(&mut conn).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_round_trip() {
        let hash = hash_password("correct horse battery").unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_password(&hash, "correct horse battery"));
        assert!(!verify_password(&hash, "correct horse battery!"));
        assert!(!verify_password("not a hash", "x"));
    }

    #[test]
    fn validation() {
        assert_eq!(
            validate("  Me@Example.COM ", "long enough pw").unwrap(),
            "me@example.com"
        );
        assert!(validate("no-at-sign", "long enough pw").is_err());
        assert!(validate("a@b", "long enough pw").is_err());
        assert!(validate("a@b.co", "short").is_err());
    }
}
