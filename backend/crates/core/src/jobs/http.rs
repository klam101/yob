//! HTTP for job boards: one shared client, a per-host rate limit in Redis,
//! and retries with backoff on 429, 5xx and non-JSON responses.

use std::{fmt, time::Duration};

use deadpool_redis::redis;
use serde::de::DeserializeOwned;

const USER_AGENT: &str = "yob/0.1 (personal job search)";
const MAX_ATTEMPTS: u32 = 3;

/// Requests per second allowed for each job-board host. Lever rate-limits
/// quickly, so it gets the lowest budget.
fn per_second(host: &str) -> i64 {
    match host {
        "boards-api.greenhouse.io" => 4,
        "api.ashbyhq.com" => 2,
        "api.lever.co" | "jobs.lever.co" => 1,
        "jobs.ashbyhq.com" => 2,
        // Shared, often overloaded public index; discovery also pauses
        // between pages.
        "index.commoncrawl.org" => 1,
        _ => 1,
    }
}

#[derive(Debug)]
pub enum FetchError {
    /// The board doesn't exist (wrong token, or the company left the platform).
    NotFound,
    RateLimited(Option<Duration>),
    Server(u16),
    /// The response wasn't the JSON we expected.
    Invalid(String),
    Network(String),
}

impl FetchError {
    fn retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimited(_) | Self::Server(_) | Self::Network(_)
        )
    }
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "job board not found (404)"),
            Self::RateLimited(_) => write!(f, "rate limited by the job board"),
            Self::Server(code) => write!(f, "job board returned HTTP {code}"),
            Self::Invalid(msg) => write!(f, "unexpected response: {msg}"),
            Self::Network(msg) => write!(f, "network error: {msg}"),
        }
    }
}

impl std::error::Error for FetchError {}

#[derive(Clone)]
pub struct Fetcher {
    client: reqwest::Client,
    redis: deadpool_redis::Pool,
}

impl Fetcher {
    pub fn new(redis: deadpool_redis::Pool) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(90))
            .connect_timeout(Duration::from_secs(15))
            .build()?;
        Ok(Self { client, redis })
    }

    /// GETs `url` and parses the JSON body, waiting for the host's rate limit
    /// and retrying transient failures.
    pub async fn get_json<T: DeserializeOwned>(&self, url: &str) -> Result<T, FetchError> {
        self.request(url, "application/json", MAX_ATTEMPTS, |body| {
            serde_json::from_str(&body).map_err(|e| {
                // Lever answers some throttled requests with an HTML page.
                if body.trim_start().starts_with('<') {
                    FetchError::RateLimited(None)
                } else {
                    FetchError::Invalid(e.to_string())
                }
            })
        })
        .await
    }

    /// GETs `url` as text (an HTML page), with the same limits and retries.
    pub async fn get_text(&self, url: &str) -> Result<String, FetchError> {
        self.request(url, "*/*", MAX_ATTEMPTS, Ok).await
    }

    /// One attempt only, for callers with their own, slower retry schedule
    /// (Common Crawl, which is often overloaded).
    pub async fn get_text_once(&self, url: &str) -> Result<String, FetchError> {
        self.request(url, "*/*", 1, Ok).await
    }

    async fn request<T>(
        &self,
        url: &str,
        accept: &str,
        attempts: u32,
        parse: impl Fn(String) -> Result<T, FetchError>,
    ) -> Result<T, FetchError> {
        let host = reqwest::Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_owned))
            .unwrap_or_default();

        let mut attempt = 0;
        loop {
            attempt += 1;
            self.wait_for_turn(&host).await;
            match self.try_get(url, accept).await.and_then(&parse) {
                Ok(value) => return Ok(value),
                Err(err) if err.retryable() && attempt < attempts => {
                    let wait = match &err {
                        FetchError::RateLimited(Some(d)) => (*d).min(Duration::from_secs(60)),
                        _ => Duration::from_secs(2u64.pow(attempt)),
                    };
                    tracing::debug!(%host, attempt, ?wait, error = %err, "retrying");
                    tokio::time::sleep(wait).await;
                }
                Err(err) => return Err(err),
            }
        }
    }

    async fn try_get(&self, url: &str, accept: &str) -> Result<String, FetchError> {
        let resp = self
            .client
            .get(url)
            .header(reqwest::header::ACCEPT, accept)
            .send()
            .await
            .map_err(|e| FetchError::Network(e.without_url().to_string()))?;

        let status = resp.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(FetchError::NotFound);
        }
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = resp
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .map(Duration::from_secs);
            return Err(FetchError::RateLimited(retry_after));
        }
        if status.is_server_error() {
            return Err(FetchError::Server(status.as_u16()));
        }
        if !status.is_success() {
            return Err(FetchError::Invalid(format!("HTTP {}", status.as_u16())));
        }

        resp.text()
            .await
            .map_err(|e| FetchError::Network(e.without_url().to_string()))
    }

    /// Fixed one-second windows per host, counted in Redis so every worker
    /// (and the API's "check now") shares the budget. If Redis is down, falls
    /// back to a short pause rather than failing the check.
    async fn wait_for_turn(&self, host: &str) {
        let limit = per_second(host);
        loop {
            let now = chrono::Utc::now();
            let key = format!("ratelimit:{host}:{}", now.timestamp());
            let count = async {
                let mut conn = self.redis.get().await.ok()?;
                let n: i64 = redis::cmd("INCR")
                    .arg(&key)
                    .query_async(&mut conn)
                    .await
                    .ok()?;
                if n == 1 {
                    let _: Result<(), _> = redis::cmd("EXPIRE")
                        .arg(&key)
                        .arg(2)
                        .query_async(&mut conn)
                        .await;
                }
                Some(n)
            }
            .await;

            match count {
                Some(n) if n <= limit => return,
                Some(_) => {
                    let ms_left = 1000 - i64::from(now.timestamp_subsec_millis());
                    tokio::time::sleep(Duration::from_millis(ms_left.max(1) as u64 + 5)).await;
                }
                None => {
                    tokio::time::sleep(Duration::from_millis(1000 / limit as u64)).await;
                    return;
                }
            }
        }
    }
}
