//! The local Ollama server: which models yob uses and whether they're ready.
//! Ollama runs as the native Windows app, outside Docker.

use std::time::Duration;

use serde::Deserialize;

/// Embeds jobs and the profile for matching.
pub const EMBEDDING_MODEL: &str = "qwen3-embedding:4b";
/// Resume parsing, fit scoring, tailoring and form answers.
pub const TEXT_MODEL: &str = "qwen3:8b";

pub const MODELS: [&str; 2] = [EMBEDDING_MODEL, TEXT_MODEL];

#[derive(Debug, Clone, Default)]
pub struct Status {
    pub reachable: bool,
    /// Models yob needs that haven't been pulled.
    pub missing_models: Vec<String>,
}

#[derive(Deserialize)]
struct Tags {
    models: Vec<Tag>,
}

#[derive(Deserialize)]
struct Tag {
    name: String,
}

/// Asks Ollama which models it has. Unreachable within 2 seconds counts as
/// down, with every model missing.
pub async fn status(base_url: &str) -> Status {
    let client = reqwest::Client::new();
    let url = format!("{}/api/tags", base_url.trim_end_matches('/'));
    let tags = async {
        client
            .get(url)
            .timeout(Duration::from_secs(2))
            .send()
            .await?
            .error_for_status()?
            .json::<Tags>()
            .await
    }
    .await;
    match tags {
        Ok(tags) => Status {
            reachable: true,
            missing_models: missing(
                &tags
                    .models
                    .iter()
                    .map(|t| t.name.as_str())
                    .collect::<Vec<_>>(),
            ),
        },
        Err(_) => Status {
            reachable: false,
            missing_models: MODELS.iter().map(|m| (*m).to_owned()).collect(),
        },
    }
}

/// Ollama names untagged models `name:latest`.
fn missing(have: &[&str]) -> Vec<String> {
    MODELS
        .iter()
        .filter(|m| !have.iter().any(|h| h == *m))
        .map(|m| (*m).to_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_models_not_pulled() {
        assert_eq!(
            missing(&["qwen3:8b", "llama3:latest"]),
            vec![EMBEDDING_MODEL]
        );
        assert!(missing(&["qwen3-embedding:4b", "qwen3:8b"]).is_empty());
    }
}
