//! One adapter per hiring platform (not per company). Each turns the
//! platform's public job-board JSON into [`NormalizedJob`]s. Parsing is kept
//! separate from fetching so it can be tested on saved responses.

pub mod ashby;
pub mod bamboohr;
pub mod greenhouse;
pub mod lever;
pub mod recruitee;
pub mod smartrecruiters;
pub mod workable;
pub mod workday;

use std::sync::LazyLock;

use chrono::{DateTime, Utc};
use futures::{StreamExt, stream};
use regex::Regex;

use super::{
    http::{FetchError, Fetcher},
    model::{NormalizedJob, Platform},
    normalize,
};

/// Detail requests in flight per company. The per-host rate limit in
/// `http.rs` still sets the pace.
const DETAIL_CONCURRENCY: usize = 4;

/// What a board's list request returns.
pub enum Listing {
    /// Every open job with full details, in one request (Lever, Ashby,
    /// Workable, Recruitee).
    Full(Vec<NormalizedJob>),
    /// Every open job as a stub: id, title, locations, but no description.
    /// Call [`details`] for the new ones (Greenhouse, Workday,
    /// SmartRecruiters, BambooHR).
    Partial(Vec<NormalizedJob>),
}

impl Listing {
    pub fn len(&self) -> usize {
        match self {
            Self::Full(jobs) | Self::Partial(jobs) => jobs.len(),
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
    use Listing::{Full, Partial};
    Ok(match platform {
        Platform::Greenhouse => Partial(greenhouse::fetch_list(fetcher, token).await?),
        Platform::Lever => Full(lever::fetch_all(fetcher, token).await?),
        Platform::Ashby => Full(ashby::fetch_all(fetcher, token).await?),
        Platform::Workday => Partial(workday::list(fetcher, token).await?),
        Platform::Smartrecruiters => Partial(smartrecruiters::list(fetcher, token).await?),
        Platform::Workable => Full(workable::fetch_all(fetcher, token).await?),
        Platform::Recruitee => Full(recruitee::fetch_all(fetcher, token).await?),
        Platform::Bamboohr => Partial(bamboohr::list(fetcher, token).await?),
    })
}

/// How many jobs a board has open: one request for paged platforms, where
/// the full list can take hundreds.
pub async fn count(
    fetcher: &Fetcher,
    platform: Platform,
    token: &str,
) -> Result<usize, FetchError> {
    match platform {
        Platform::Workday => workday::count(fetcher, token).await,
        Platform::Smartrecruiters => {
            let page = smartrecruiters::fetch_page(fetcher, token, 0).await?;
            Ok(page.total_found.max(page.content.len()))
        }
        _ => Ok(list(fetcher, platform, token).await?.len()),
    }
}

/// Full details for stubs that a [`Listing::Partial`] reported as new. May
/// return more than asked for when the platform only serves the whole board
/// (Greenhouse); the extra jobs are simply refreshed. Jobs that vanished
/// in between are left out; other failures fail the call only if no job
/// could be read.
pub async fn details(
    fetcher: &Fetcher,
    platform: Platform,
    token: &str,
    new: &[NormalizedJob],
) -> Result<Vec<NormalizedJob>, FetchError> {
    if new.is_empty() {
        return Ok(Vec::new());
    }
    match platform {
        Platform::Greenhouse => greenhouse::fetch_all(fetcher, token).await,
        Platform::Workday => {
            each(
                new.iter()
                    .map(|s| workday::detail(fetcher, token, s))
                    .collect(),
            )
            .await
        }
        Platform::Smartrecruiters => {
            each(
                new.iter()
                    .map(|s| smartrecruiters::detail(fetcher, token, s))
                    .collect(),
            )
            .await
        }
        Platform::Bamboohr => {
            each(
                new.iter()
                    .map(|s| bamboohr::detail(fetcher, token, s))
                    .collect(),
            )
            .await
        }
        // Their listings already carry full details.
        Platform::Lever | Platform::Ashby | Platform::Workable | Platform::Recruitee => {
            Ok(Vec::new())
        }
    }
}

/// Runs one detail request per job, a few at a time.
async fn each<Fut>(requests: Vec<Fut>) -> Result<Vec<NormalizedJob>, FetchError>
where
    Fut: Future<Output = Result<NormalizedJob, FetchError>>,
{
    let results: Vec<_> = stream::iter(requests)
        .buffer_unordered(DETAIL_CONCURRENCY)
        .collect()
        .await;
    let mut jobs = Vec::new();
    let mut failure = None;
    for result in results {
        match result {
            Ok(job) => jobs.push(job),
            Err(FetchError::NotFound) => {}
            Err(e) => {
                tracing::debug!(error = %e, "job detail failed; will retry next check");
                failure.get_or_insert(e);
            }
        }
    }
    match failure {
        Some(e) if jobs.is_empty() => Err(e),
        _ => Ok(jobs),
    }
}

/// A job known only from a list: no description yet. Out-of-scope stubs are
/// stored as they are; in-scope ones get [`details`].
pub(crate) fn stub(
    id: String,
    title: String,
    locations: Vec<String>,
    posted_at: DateTime<Utc>,
    apply_url: String,
) -> NormalizedJob {
    NormalizedJob {
        source_job_id: id,
        workplace: normalize::infer_workplace(&title, &locations, ""),
        employment_type: normalize::infer_employment(&title, None),
        seniority: normalize::infer_seniority(&title),
        salary: None,
        salary_from_text: false,
        content_hash: normalize::content_hash(&title, &locations, ""),
        department: None,
        title,
        locations,
        posted_at,
        apply_url,
        description_html: String::new(),
        description_text: String::new(),
    }
}

/// The company's name as its board shows it, if the platform exposes one:
/// Greenhouse's board API, the board page's `<title>` for Lever
/// ("Palantir Technologies") and Ashby ("Ramp Jobs"), or the name in the
/// job data for SmartRecruiters, Workable and Recruitee.
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
        Platform::Smartrecruiters => smartrecruiters::board_name(fetcher, token).await?,
        Platform::Workable => workable::fetch_account(fetcher, token).await.ok()?.name?,
        Platform::Recruitee => recruitee::fetch_offers(fetcher, token)
            .await
            .ok()?
            .offers
            .into_iter()
            .find_map(|o| o.company_name)?,
        // Neither has a reliable name to read; the token is used.
        Platform::Workday | Platform::Bamboohr => return None,
    };
    let name = name.trim().to_owned();
    let generic = [
        "jobs",
        "careers",
        "job board",
        "lever",
        "ashby",
        "greenhouse",
        "workable",
    ];
    (!name.is_empty() && name.len() <= 120 && !generic.contains(&name.to_lowercase().as_str()))
        .then_some(name)
}

/// A readable name from a board token: "open-ai" → "Open Ai". Used when the
/// platform has no name to offer. For Workday's `tenant/wdN/site`, the tenant.
pub fn display_name(token: &str) -> String {
    let token = token.split('/').next().unwrap_or(token);
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
/// Filters out file names and site paths before they cost a probe. Workday
/// tokens have three parts (`tenant/wdN/site`), each checked the same way.
pub fn plausible_token(token: &str) -> bool {
    static SHAPE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^[A-Za-z0-9][A-Za-z0-9_-]{0,79}(/[A-Za-z0-9][A-Za-z0-9_-]{0,79}){0,2}$")
            .unwrap()
    });
    const NOT_BOARDS: &[&str] = &[
        "api", "v0", "v1", "static", "assets", "embed", "jobs", "search", "login", "privacy",
        "terms", "cdn", "www", "app", "help", "support", "blog", "docs", "careers",
    ];
    SHAPE.is_match(token) && !NOT_BOARDS.contains(&token.to_lowercase().as_str())
}

/// Job-board links inside a description or apply URL, as (platform, token).
/// Lets every job we read point us to more companies at no extra cost.
pub fn board_links(text: &str) -> Vec<(Platform, String)> {
    static LINK: LazyLock<Regex> = LazyLock::new(|| {
        let hosts = [
            r"(?:boards|job-boards|boards-api)\.greenhouse\.io",
            r"(?:jobs|api)\.lever\.co",
            r"jobs\.ashbyhq\.com",
            r"[a-z0-9-]+\.wd\d+\.myworkdayjobs\.com",
            r"wd\d+\.myworkdaysite\.com",
            r"(?:jobs|careers)\.smartrecruiters\.com",
            r"apply\.workable\.com",
            r"[a-z0-9-]+\.recruitee\.com",
            r"[a-z0-9-]+\.bamboohr\.com",
        ];
        Regex::new(&format!(
            r#"(?i)https?://(?:{})/[^\s"'<>)]*"#,
            hosts.join("|")
        ))
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
        "jobs.smartrecruiters.com" | "careers.smartrecruiters.com" => {
            let first = segments.next()?;
            // One-click apply links: /oneclick-ui/company/<id>/publication/…
            let id = if first == "oneclick-ui" {
                segments.skip_while(|s| *s != "company").nth(1)?
            } else {
                first
            };
            (Platform::Smartrecruiters, id.to_owned())
        }
        "api.smartrecruiters.com" => {
            let parts: Vec<&str> = segments.collect();
            let i = parts.iter().position(|p| *p == "companies")?;
            (Platform::Smartrecruiters, parts.get(i + 1)?.to_string())
        }
        "apply.workable.com" => {
            let parts: Vec<&str> = segments.collect();
            match parts.first()? {
                // Job shortlinks (/j/<code>) don't name the account.
                &"j" => return None,
                &"api" => {
                    let i = parts.iter().position(|p| *p == "accounts")?;
                    (Platform::Workable, parts.get(i + 1)?.to_string())
                }
                account => (Platform::Workable, account.to_string()),
            }
        }
        h if h.ends_with(".myworkdayjobs.com") => {
            // <tenant>.wd5.myworkdayjobs.com/[en-US/]<site>/…, or the API
            // path /wday/cxs/<tenant>/<site>/….
            let labels: Vec<&str> = h.split('.').collect();
            let [tenant, wd, _, _] = labels[..] else {
                return None;
            };
            let parts: Vec<&str> = segments.filter(|s| !is_locale(s)).collect();
            let site = match parts.first()? {
                &"wday" => parts.get(3)?,
                site => site,
            };
            if !wd.starts_with("wd") || matches!(*site, "job" | "login" | "wday") {
                return None;
            }
            (Platform::Workday, format!("{tenant}/{wd}/{site}"))
        }
        h if h.ends_with(".myworkdaysite.com") => {
            // wd3.myworkdaysite.com/[en-US/]recruiting/<tenant>/<site>/…
            let wd = h.split('.').next()?;
            let parts: Vec<&str> = segments.filter(|s| !is_locale(s)).collect();
            let i = parts.iter().position(|p| *p == "recruiting")?;
            let (tenant, site) = (parts.get(i + 1)?, parts.get(i + 2)?);
            if !wd.starts_with("wd") {
                return None;
            }
            (
                Platform::Workday,
                format!("{}/{wd}/{site}", tenant.to_lowercase()),
            )
        }
        h if h.ends_with(".recruitee.com") => (Platform::Recruitee, subdomain(h)?),
        h if h.ends_with(".bamboohr.com") => (Platform::Bamboohr, subdomain(h)?),
        _ => return None,
    };
    let token = token.trim().to_owned();
    (!token.is_empty()).then_some((platform, token))
}

/// `acme` from `acme.recruitee.com`; the platform's own hosts don't count.
fn subdomain(host: &str) -> Option<String> {
    let labels: Vec<&str> = host.split('.').collect();
    let [name, _, _] = labels[..] else {
        return None;
    };
    const OWN: &[&str] = &[
        "www",
        "app",
        "api",
        "help",
        "support",
        "blog",
        "docs",
        "status",
        "marketplace",
        "partners",
        "careers",
        "jobs",
        "static",
        "cdn",
    ];
    (!OWN.contains(&name)).then(|| name.to_owned())
}

/// "en-US", "fr-CA": Workday puts a locale before the site name.
fn is_locale(segment: &str) -> bool {
    let b = segment.as_bytes();
    b.len() == 5
        && b[2] == b'-'
        && b[..2].iter().all(u8::is_ascii_lowercase)
        && b[3..].iter().all(u8::is_ascii_uppercase)
}

/// The public careers page for a board, shown on the Job sources page.
pub fn careers_url(platform: Platform, token: &str) -> String {
    match platform {
        Platform::Greenhouse => format!("https://job-boards.greenhouse.io/{token}"),
        Platform::Lever => format!("https://jobs.lever.co/{token}"),
        Platform::Ashby => format!("https://jobs.ashbyhq.com/{token}"),
        Platform::Workday => workday::Site::parse(token)
            .map(|s| s.careers_url())
            .unwrap_or_default(),
        Platform::Smartrecruiters => format!("https://jobs.smartrecruiters.com/{token}"),
        Platform::Workable => format!("https://apply.workable.com/{token}/"),
        Platform::Recruitee => format!("https://{token}.recruitee.com/"),
        Platform::Bamboohr => format!("https://{token}.bamboohr.com/careers"),
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
    fn more_platform_urls() {
        let cases = [
            (
                "https://nvidia.wd5.myworkdayjobs.com/NVIDIAExternalCareerSite/job/US-CA-Santa-Clara/Engineer_JR1",
                Platform::Workday,
                "nvidia/wd5/NVIDIAExternalCareerSite",
            ),
            (
                "https://acme.wd1.myworkdayjobs.com/en-US/External",
                Platform::Workday,
                "acme/wd1/External",
            ),
            (
                "https://acme.wd1.myworkdayjobs.com/wday/cxs/acme/External/jobs",
                Platform::Workday,
                "acme/wd1/External",
            ),
            (
                "https://wd3.myworkdaysite.com/en-US/recruiting/Acme/Careers/job/1",
                Platform::Workday,
                "acme/wd3/Careers",
            ),
            (
                "https://jobs.smartrecruiters.com/ServiceNow/744000163098369-data-analyst",
                Platform::Smartrecruiters,
                "ServiceNow",
            ),
            (
                "https://jobs.smartrecruiters.com/oneclick-ui/company/Visa/publication/abc",
                Platform::Smartrecruiters,
                "Visa",
            ),
            (
                "https://api.smartrecruiters.com/v1/companies/BoschGroup/postings",
                Platform::Smartrecruiters,
                "BoschGroup",
            ),
            (
                "https://apply.workable.com/huggingface/j/81B46579FE/",
                Platform::Workable,
                "huggingface",
            ),
            (
                "https://apply.workable.com/api/v1/widget/accounts/huggingface",
                Platform::Workable,
                "huggingface",
            ),
            (
                "https://bunq.recruitee.com/o/copywriter-3",
                Platform::Recruitee,
                "bunq",
            ),
            (
                "https://rei.bamboohr.com/careers/42",
                Platform::Bamboohr,
                "rei",
            ),
        ];
        for (url, platform, token) in cases {
            assert_eq!(
                parse_board_url(url),
                Some((platform, token.to_owned())),
                "{url}"
            );
        }
        for url in [
            "https://apply.workable.com/j/81B46579FE",
            "https://www.bamboohr.com/pricing",
            "https://acme.myworkdayjobs.com/External",
            "https://acme.wd5.myworkdayjobs.com/",
        ] {
            assert_eq!(parse_board_url(url), None, "{url}");
        }
        assert!(plausible_token("nvidia/wd5/NVIDIAExternalCareerSite"));
        assert!(!plausible_token("a/b/c/d"));
        assert_eq!(display_name("nvidia/wd5/External"), "Nvidia");
        assert_eq!(
            board_links(
                "Apply: https://acme.wd1.myworkdayjobs.com/External/job/1 or https://rei.bamboohr.com/careers."
            ),
            vec![
                (Platform::Workday, "acme/wd1/External".to_owned()),
                (Platform::Bamboohr, "rei".to_owned()),
            ]
        );
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
