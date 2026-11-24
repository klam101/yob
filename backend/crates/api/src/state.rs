use yob_core::{config::ProfileKey, jobs::Fetcher, profile::resume::Files};
use yob_db::PgPool;

/// Shared handles passed to every request handler.
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: deadpool_redis::Pool,
    /// For verifying job boards when a company is added. Shares the
    /// per-host rate limit with the worker through Redis.
    pub fetcher: Fetcher,
    pub ollama_url: String,
    /// Encrypts the equal-employment answers.
    pub profile_key: ProfileKey,
    /// Uploaded resumes and generated documents.
    pub files: Files,
}
