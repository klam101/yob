use anyhow::{Context, Result};

/// Settings read from the environment (and from `.env` when running locally).
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub api_addr: String,
    pub web_origin: String,
    pub ollama_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // A missing .env is fine: Docker Compose passes real environment variables.
        let _ = dotenvy::dotenv();

        Ok(Self {
            database_url: required("DATABASE_URL")?,
            redis_url: required("REDIS_URL")?,
            api_addr: optional("API_ADDR", "127.0.0.1:8080"),
            web_origin: optional("WEB_ORIGIN", "http://localhost:3000"),
            ollama_url: optional("OLLAMA_URL", "http://localhost:11434"),
        })
    }
}

fn required(key: &str) -> Result<String> {
    std::env::var(key).with_context(|| format!("{key} is not set (see .env.example)"))
}

fn optional(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_owned())
}
