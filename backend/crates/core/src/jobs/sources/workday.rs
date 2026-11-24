//! Workday careers sites, read through the JSON their own pages load.
//! A board token is `tenant/wdN/site`, from a careers URL like
//! `https://nvidia.wd5.myworkdayjobs.com/NVIDIAExternalCareerSite`.
//!
//! - List: `POST https://{tenant}.{wdN}.myworkdayjobs.com/wday/cxs/{tenant}/{site}/jobs`,
//!   20 jobs per page, with title, one location line and "Posted 3 Days Ago".
//! - Detail: `GET …/wday/cxs/{tenant}/{site}{externalPath}`, one per new job.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::Deserialize;

use super::stub;
use crate::jobs::{
    http::{FetchError, Fetcher},
    model::{NormalizedJob, Workplace},
    normalize,
};

/// Workday refuses pages larger than 20.
const PAGE: usize = 20;
/// A safety stop for very large tenants.
const MAX_JOBS: usize = 25_000;

/// A parsed `tenant/wdN/site` token.
pub struct Site<'a> {
    pub tenant: &'a str,
    pub wd: &'a str,
    pub site: &'a str,
}

impl<'a> Site<'a> {
    pub fn parse(token: &'a str) -> Option<Self> {
        let mut parts = token.split('/');
        let (tenant, wd, site) = (parts.next()?, parts.next()?, parts.next()?);
        let valid = !tenant.is_empty() && wd.starts_with("wd") && !site.is_empty();
        (valid && parts.next().is_none()).then_some(Self { tenant, wd, site })
    }

    fn host(&self) -> String {
        format!("https://{}.{}.myworkdayjobs.com", self.tenant, self.wd)
    }

    fn api(&self) -> String {
        format!("{}/wday/cxs/{}/{}", self.host(), self.tenant, self.site)
    }

    pub fn careers_url(&self) -> String {
        format!("{}/{}", self.host(), self.site)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    /// Only reliable on the first page; later pages may say 0.
    #[serde(default)]
    pub total: usize,
    #[serde(default)]
    pub job_postings: Vec<Posting>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Posting {
    pub title: Option<String>,
    pub external_path: Option<String>,
    /// One location ("US, CA, Santa Clara") or a count ("3 Locations").
    pub locations_text: Option<String>,
    pub posted_on: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Detail {
    pub job_posting_info: PostingInfo,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostingInfo {
    pub title: String,
    pub job_description: Option<String>,
    pub location: Option<String>,
    #[serde(default)]
    pub additional_locations: Vec<String>,
    pub posted_on: Option<String>,
    /// The posting date, "2026-11-23".
    pub start_date: Option<String>,
    pub time_type: Option<String>,
    pub remote_type: Option<String>,
    pub external_url: Option<String>,
}

fn invalid_token() -> FetchError {
    FetchError::Invalid("Workday boards look like tenant/wdN/site".to_owned())
}

/// How many jobs are open, from the first page alone.
pub async fn count(fetcher: &Fetcher, token: &str) -> Result<usize, FetchError> {
    let site = Site::parse(token).ok_or_else(invalid_token)?;
    let body = serde_json::json!({
        "appliedFacets": {}, "limit": PAGE, "offset": 0, "searchText": "",
    });
    let page: Page = fetcher
        .post_json(&format!("{}/jobs", site.api()), &body)
        .await?;
    Ok(page.total.max(page.job_postings.len()))
}

/// Every open job as a stub (no description). Ids are the job's path.
pub async fn list(fetcher: &Fetcher, token: &str) -> Result<Vec<NormalizedJob>, FetchError> {
    let site = Site::parse(token).ok_or_else(invalid_token)?;
    let url = format!("{}/jobs", site.api());
    let mut postings = Vec::new();
    let mut total = None;
    loop {
        let body = serde_json::json!({
            "appliedFacets": {}, "limit": PAGE, "offset": postings.len(), "searchText": "",
        });
        let page: Page = fetcher.post_json(&url, &body).await?;
        let got = page.job_postings.len();
        let total = *total.get_or_insert(page.total);
        postings.extend(page.job_postings);
        if got < PAGE || postings.len() >= total.min(MAX_JOBS) {
            break;
        }
    }
    Ok(parse_list(&site, postings, Utc::now()))
}

pub fn parse_list(site: &Site, postings: Vec<Posting>, now: DateTime<Utc>) -> Vec<NormalizedJob> {
    postings
        .into_iter()
        .filter_map(|p| {
            let path = p.external_path?;
            let title = p.title?.trim().to_owned();
            let locations = p
                .locations_text
                .map(|l| l.trim().to_owned())
                .filter(|l| !l.is_empty() && !is_location_count(l))
                .into_iter()
                .collect::<Vec<_>>();
            let apply_url = format!("{}{}", site.careers_url(), path);
            let posted = posted_ago(p.posted_on.as_deref(), now);
            Some(stub(path, title, locations, posted, apply_url))
        })
        .collect()
}

/// "3 Locations": the real list only comes with the detail.
fn is_location_count(text: &str) -> bool {
    text.split_once(' ').is_some_and(|(n, rest)| {
        n.parse::<u32>().is_ok() && rest.to_lowercase().starts_with("location")
    })
}

/// The full job for a stub from [`list`].
pub async fn detail(
    fetcher: &Fetcher,
    token: &str,
    stub: &NormalizedJob,
) -> Result<NormalizedJob, FetchError> {
    let site = Site::parse(token).ok_or_else(invalid_token)?;
    let url = format!("{}{}", site.api(), stub.source_job_id);
    let detail: Detail = fetcher.get_json(&url).await?;
    Ok(parse_detail(stub, detail, Utc::now()))
}

pub fn parse_detail(stub: &NormalizedJob, d: Detail, now: DateTime<Utc>) -> NormalizedJob {
    let p = d.job_posting_info;
    let (description_html, description_text) =
        normalize::clean_html(p.job_description.as_deref().unwrap_or(""), false);

    let mut locations: Vec<String> = Vec::new();
    for l in p.location.iter().chain(&p.additional_locations) {
        let l = l.trim();
        if !l.is_empty() && !locations.iter().any(|x| x == l) {
            locations.push(l.to_owned());
        }
    }
    if locations.is_empty() {
        locations = stub.locations.clone();
    }

    let workplace = match p.remote_type.as_deref().map(str::to_lowercase) {
        Some(r) if r.contains("remote") => Workplace::Remote,
        Some(r) if r.contains("hybrid") => Workplace::Hybrid,
        _ => normalize::infer_workplace(&p.title, &locations, &description_text),
    };

    let posted_at = p
        .start_date
        .as_deref()
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|d| d.and_utc().min(now))
        .unwrap_or_else(|| posted_ago(p.posted_on.as_deref(), now));

    let (salary, salary_from_text) = normalize::salary_or_text(None, &description_text);
    let title = p.title.trim().to_owned();
    NormalizedJob {
        source_job_id: stub.source_job_id.clone(),
        department: None,
        workplace,
        employment_type: normalize::infer_employment(&title, p.time_type.as_deref()),
        seniority: normalize::infer_seniority(&title),
        salary,
        salary_from_text,
        content_hash: normalize::content_hash(&title, &locations, &description_text),
        posted_at,
        apply_url: p.external_url.unwrap_or_else(|| stub.apply_url.clone()),
        title,
        locations,
        description_html,
        description_text,
    }
}

/// "Posted Today", "Posted Yesterday", "Posted 3 Days Ago", "Posted 30+ Days Ago".
pub fn posted_ago(text: Option<&str>, now: DateTime<Utc>) -> DateTime<Utc> {
    let text = text.unwrap_or("").to_lowercase();
    let days = if text.contains("yesterday") {
        1
    } else {
        text.split_whitespace()
            .find_map(|w| w.trim_end_matches('+').parse::<i64>().ok())
            .unwrap_or(0)
    };
    now - Duration::days(days)
}
