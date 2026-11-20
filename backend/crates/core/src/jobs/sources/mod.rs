//! One adapter per hiring platform (not per company). Each turns the
//! platform's public job-board JSON into [`NormalizedJob`]s. Parsing is kept
//! separate from fetching so it can be tested on saved responses.

pub mod ashby;
pub mod greenhouse;
pub mod lever;

use std::{collections::HashSet, sync::LazyLock};

use chrono::{DateTime, Utc};
use regex::Regex;

use super::{
    http::{FetchError, Fetcher},
    model::{NormalizedJob, Platform},
};

/// All open jobs on a company's board, normalized.
pub async fn fetch_all(
    fetcher: &Fetcher,
    platform: Platform,
    token: &str,
) -> Result<Vec<NormalizedJob>, FetchError> {
    match platform {
        Platform::Greenhouse => greenhouse::fetch_all(fetcher, token).await,
        Platform::Lever => lever::fetch_all(fetcher, token).await,
        Platform::Ashby => ashby::fetch_all(fetcher, token).await,
    }
}

/// Just the open job ids, when the platform has a cheap way to list them
/// (Greenhouse). `None` means "fetch everything instead".
pub async fn fetch_ids(
    fetcher: &Fetcher,
    platform: Platform,
    token: &str,
) -> Result<Option<HashSet<String>>, FetchError> {
    match platform {
        Platform::Greenhouse => greenhouse::fetch_ids(fetcher, token).await.map(Some),
        Platform::Lever | Platform::Ashby => Ok(None),
    }
}

/// Recognizes a public job-board URL and returns its platform and token, e.g.
/// `https://boards.greenhouse.io/stripe` → (Greenhouse, "stripe").
pub fn parse_board_url(input: &str) -> Option<(Platform, String)> {
    let input = input.trim();
    let with_scheme = if input.contains("://") {
        input.to_owned()
    } else {
        format!("https://{input}")
    };
    let url = url::Url::parse(&with_scheme).ok()?;
    let host = url.host_str()?.to_lowercase();
    let mut segments = url.path_segments()?.filter(|s| !s.is_empty());

    let (platform, token) = match host.as_str() {
        "boards.greenhouse.io" | "job-boards.greenhouse.io" | "job-boards.eu.greenhouse.io" => {
            let first = segments.next()?;
            // Embedded boards use ?for=<token>.
            if first == "embed" {
                let token = url.query_pairs().find(|(k, _)| k == "for")?.1.into_owned();
                (Platform::Greenhouse, token)
            } else {
                (Platform::Greenhouse, first.to_owned())
            }
        }
        "boards-api.greenhouse.io" => {
            // /v1/boards/<token>/jobs
            let parts: Vec<&str> = segments.collect();
            let i = parts.iter().position(|p| *p == "boards")?;
            (Platform::Greenhouse, parts.get(i + 1)?.to_string())
        }
        "jobs.lever.co" | "jobs.eu.lever.co" => (Platform::Lever, segments.next()?.to_owned()),
        "api.lever.co" => {
            let parts: Vec<&str> = segments.collect();
            let i = parts.iter().position(|p| *p == "postings")?;
            (Platform::Lever, parts.get(i + 1)?.to_string())
        }
        "jobs.ashbyhq.com" => (Platform::Ashby, segments.next()?.to_owned()),
        "api.ashbyhq.com" => (Platform::Ashby, segments.next_back()?.to_owned()),
        _ => return None,
    };
    let token = token.trim().to_owned();
    (!token.is_empty()).then_some((platform, token))
}

/// The public careers page for a board, shown on the Job sources page.
pub fn careers_url(platform: Platform, token: &str) -> String {
    match platform {
        Platform::Greenhouse => format!("https://job-boards.greenhouse.io/{token}"),
        Platform::Lever => format!("https://jobs.lever.co/{token}"),
        Platform::Ashby => format!("https://jobs.ashbyhq.com/{token}"),
    }
}

pub(crate) fn parse_time(s: Option<&str>) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s?)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

/// Strips internal codes some companies put on departments ("8611 Security").
pub(crate) fn clean_department(name: Option<&str>) -> Option<String> {
    static CODE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*\d+\s*[-–:]?\s*").unwrap());
    let name = CODE.replace(name?.trim(), "").trim().to_owned();
    (!name.is_empty()).then_some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn board_urls() {
        let cases = [
            (
                "https://boards.greenhouse.io/stripe",
                Platform::Greenhouse,
                "stripe",
            ),
            (
                "job-boards.greenhouse.io/figma/jobs/123",
                Platform::Greenhouse,
                "figma",
            ),
            (
                "https://boards.greenhouse.io/embed/job_board?for=airbnb",
                Platform::Greenhouse,
                "airbnb",
            ),
            (
                "https://boards-api.greenhouse.io/v1/boards/discord/jobs",
                Platform::Greenhouse,
                "discord",
            ),
            (
                "https://jobs.lever.co/palantir/abc-123",
                Platform::Lever,
                "palantir",
            ),
            (
                "https://api.lever.co/v0/postings/spotify?mode=json",
                Platform::Lever,
                "spotify",
            ),
            ("https://jobs.ashbyhq.com/ramp", Platform::Ashby, "ramp"),
            (
                "https://api.ashbyhq.com/posting-api/job-board/notion",
                Platform::Ashby,
                "notion",
            ),
        ];
        for (url, platform, token) in cases {
            assert_eq!(
                parse_board_url(url),
                Some((platform, token.to_owned())),
                "{url}"
            );
        }
        assert_eq!(parse_board_url("https://example.com/careers"), None);
        assert_eq!(parse_board_url("https://jobs.lever.co/"), None);
    }

    #[test]
    fn departments() {
        assert_eq!(
            clean_department(Some("8611 Security Analytics")).as_deref(),
            Some("Security Analytics")
        );
        assert_eq!(
            clean_department(Some("Engineering")).as_deref(),
            Some("Engineering")
        );
        assert_eq!(clean_department(Some("  ")), None);
    }
}
