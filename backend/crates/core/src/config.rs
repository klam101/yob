use anyhow::{Context, Result};

/// Settings read from the environment (and from `.env` when running locally).
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub api_addr: String,
    pub web_origin: String,
    pub ollama_url: String,
    /// Optional keys for job boards that need one.
    pub feed_keys: FeedKeys,
}

/// API keys for job boards. A board whose key isn't set is skipped.
#[derive(Clone, Default)]
pub struct FeedKeys {
    pub usajobs: Option<UsajobsKey>,
}

/// USAJOBS asks for the key holder's email as the User-Agent. Both come
/// only from the user's own `.env`.
#[derive(Clone)]
pub struct UsajobsKey {
    pub api_key: String,
    pub email: String,
}

// Keys stay out of logs.
impl std::fmt::Debug for FeedKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FeedKeys")
            .field("usajobs", &self.usajobs.is_some())
            .finish()
    }
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // A missing .env is fine: Docker Compose passes real environment variables.
        let _ = dotenvy::dotenv();

        let usajobs = match (set("USAJOBS_API_KEY"), set("USAJOBS_EMAIL")) {
            (Some(api_key), Some(email)) => Some(UsajobsKey { api_key, email }),
            _ => None,
        };
        Ok(Self {
            database_url: required("DATABASE_URL")?,
            redis_url: required("REDIS_URL")?,
            api_addr: optional("API_ADDR", "127.0.0.1:8080"),
            web_origin: optional("WEB_ORIGIN", "http://localhost:3000"),
            ollama_url: optional("OLLAMA_URL", "http://localhost:11434"),
            feed_keys: FeedKeys { usajobs },
        })
    }
}

fn required(key: &str) -> Result<String> {
    std::env::var(key).with_context(|| format!("{key} is not set (see .env.example)"))
}

fn optional(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_owned())
}

/// A variable that is set and not blank (Compose passes unset ones as "").
fn set(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty())
}
