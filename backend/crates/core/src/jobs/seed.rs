//! Starter list of companies to track, embedded in the binary. Every entry
//! was verified against the live job board when it was added. Existing rows
//! are left alone, so removing a company in the app sticks.

use serde::Deserialize;
use sqlx::PgPool;

use super::{model::Platform, sources};

#[derive(Deserialize)]
struct Seed {
    name: String,
    platform: Platform,
    token: String,
}

const SEEDS: &str = include_str!("seed_companies.json");

/// Inserts any seed companies that aren't tracked yet. Returns how many were added.
pub async fn seed_companies(pool: &PgPool) -> anyhow::Result<u64> {
    let seeds: Vec<Seed> = serde_json::from_str(SEEDS)?;
    let mut added = 0;
    for s in seeds {
        added += sqlx::query(
            "INSERT INTO companies (name, platform, board_token, careers_url)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (platform, board_token) DO NOTHING",
        )
        .bind(&s.name)
        .bind(s.platform.as_str())
        .bind(&s.token)
        .bind(sources::careers_url(s.platform, &s.token))
        .execute(pool)
        .await?
        .rows_affected();
    }
    Ok(added)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_file_parses_and_has_no_duplicates() {
        let seeds: Vec<Seed> = serde_json::from_str(SEEDS).unwrap();
        let mut keys = std::collections::HashSet::new();
        for s in &seeds {
            assert!(
                keys.insert((s.platform, s.token.to_lowercase())),
                "duplicate seed {}",
                s.token
            );
            assert!(!s.name.trim().is_empty());
        }
    }
}
