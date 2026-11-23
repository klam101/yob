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

/// What a board's list request returns.
pub enum Listing {
    /// Every open job with full details, in one request (Lever, Ashby).
    Full(Vec<NormalizedJob>),
    /// Only the open job ids; call [`details`] for the new ones (Greenhouse).
    Ids(HashSet<String>),
}

impl Listing {
    pub fn len(&self) -> usize {
        match self {
            Self::Full(jobs) => jobs.len(),
            Self::Ids(ids) => ids.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The open jobs on a company's board, as cheaply as the platform allows.
pub async fn list(
    fetcher: &Fetcher,
    platform: Platform,
    token: &str,
) -> Result<Listing, FetchError> {
    Ok(match platform {
        Platform::Greenhouse => Listing::Ids(greenhouse::fetch_ids(fetcher, token).await?),
        Platform::Lever => Listing::Full(lever::fetch_all(fetcher, token).await?),
        Platform::Ashby => Listing::Full(ashby::fetch_all(fetcher, token).await?),
    })
}

/// Full details for jobs that a [`Listing::Ids`] reported as new. May return
/// more than asked for when the platform only serves the whole board
/// (Greenhouse); the extra jobs are simply refreshed.
pub async fn details(
    fetcher: &Fetcher,
    platform: Platform,
    token: &str,
    new_ids: &HashSet<String>,
) -> Result<Vec<NormalizedJob>, FetchError> {
    if new_ids.is_empty() {
        return Ok(Vec::new());
    }
    match platform {
        Platform::Greenhouse => greenhouse::fetch_all(fetcher, token).await,
        // Their listings already carry full details.
        Platform::Lever | Platform::Ashby => Ok(Vec::new()),
    }
}

/// The company's name as its board shows it, if the platform exposes one:
/// Greenhouse's board API, or the board page's `<title>` for Lever
/// ("Palantir Technologies") and Ashby ("Ramp Jobs").
pub async fn board_name(fetcher: &Fetcher, platform: Platform, token: &str) -> Option<String> {
    #[derive(serde::Deserialize)]
    struct GreenhouseBoard {
        name: Option<String>,
    }
    static TITLE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?is)<title[^>]*>(.*?)</title>").unwrap());

    let name = match platform {
        Platform::Greenhouse => {
            let url = format!("https://boards-api.greenhouse.io/v1/boards/{token}");
            fetcher.get_json::<GreenhouseBoard>(&url).await.ok()?.name?
        }
        Platform::Lever | Platform::Ashby => {
            let page = fetcher.get_text(&careers_url(platform, token)).await.ok()?;
            let title = TITLE.captures(&page)?.get(1)?.as_str();
            let title = html_escape::decode_html_entities(title.trim()).into_owned();
            title
                .strip_suffix(" Jobs")
                .or_else(|| title.strip_suffix(" jobs"))
                .unwrap_or(&title)
                .trim()
                .to_owned()
        }
    };
    let name = name.trim().to_owned();
    let generic = [
        "jobs",
        "careers",
        "job board",
        "lever",
        "ashby",
        "greenhouse",
    ];
    (!name.is_empty() && name.len() <= 120 && !generic.contains(&name.to_lowercase().as_str()))
        .then_some(name)
}

/// A readable name from a board token: "open-ai" → "Open Ai". Used when the
/// platform has no name to offer.
pub fn display_name(token: &str) -> String {
    token
        .split(['-', '_', '.'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether a token from a crawled or linked URL can be a real board name.
/// Filters out file names and site paths before they cost a probe.
pub fn plausible_token(token: &str) -> bool {
    static SHAPE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^[A-Za-z0-9][A-Za-z0-9_-]{0,79}$").unwrap());
    const NOT_BOARDS: &[&str] = &[
        "api", "v0", "v1", "static", "assets", "embed", "jobs", "search", "login", "privacy",
        "terms", "cdn",
    ];
    SHAPE.is_match(token) && !NOT_BOARDS.contains(&token.to_lowercase().as_str())
}

/// Job-board links inside a description or apply URL, as (platform, token).
/// Lets every job we read point us to more companies at no extra cost.
pub fn board_links(text: &str) -> Vec<(Platform, String)> {
    static LINK: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"(?i)https?://(?:boards|job-boards|boards-api)\.greenhouse\.io/[^\s"'<>)]+|https?://(?:jobs|api)\.lever\.co/[^\s"'<>)]+|https?://jobs\.ashbyhq\.com/[^\s"'<>)]+"#,
        )
        .unwrap()
    });
    let mut found: Vec<(Platform, String)> = Vec::new();
    for m in LINK.find_iter(text) {
        if let Some((platform, token)) =
            parse_board_url(m.as_str().trim_end_matches(['.', ',', ';', ':', '!', '?']))
            && plausible_token(&token)
            && !found
                .iter()
                .any(|(p, t)| *p == platform && t.eq_ignore_ascii_case(&token))
        {
            found.push((platform, token));
        }
    }
    found
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
    fn links_in_text() {
        let text = r#"Apply at <a href="https://boards.greenhouse.io/acme/jobs/1?gh_src=x">here</a>,
            see https://jobs.lever.co/Beta-Co/abc and https://jobs.lever.co/beta-co/def,
            or https://jobs.ashbyhq.com/gamma. Not https://jobs.lever.co/robots.txt."#;
        assert_eq!(
            board_links(text),
            vec![
                (Platform::Greenhouse, "acme".to_owned()),
                (Platform::Lever, "Beta-Co".to_owned()),
                (Platform::Ashby, "gamma".to_owned()),
            ]
        );
    }

    #[test]
    fn names_from_tokens() {
        assert_eq!(display_name("open-ai"), "Open Ai");
        assert!(plausible_token("stripe"));
        assert!(!plausible_token("robots.txt"));
        assert!(!plausible_token("embed"));
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
