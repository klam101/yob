//! Workable public widget API:
//! `GET https://apply.workable.com/api/v1/widget/accounts/{account}?details=true`.
//! One request returns the company name and every published job with its
//! description. Workable says only "telecommuting" (remote) or not.

use chrono::{NaiveDate, Utc};
use serde::Deserialize;

use super::clean_department;
use crate::jobs::{
    http::{FetchError, Fetcher},
    model::{NormalizedJob, Workplace},
    normalize,
};

#[derive(Deserialize)]
pub struct Account {
    pub name: Option<String>,
    #[serde(default)]
    pub jobs: Vec<Job>,
}

#[derive(Deserialize)]
pub struct Job {
    pub title: String,
    pub shortcode: String,
    pub employment_type: Option<String>,
    pub telecommuting: Option<bool>,
    pub department: Option<String>,
    pub url: Option<String>,
    pub published_on: Option<String>,
    pub created_at: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    #[serde(default)]
    pub locations: Vec<Location>,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct Location {
    pub city: Option<String>,
    pub region: Option<String>,
    pub country: Option<String>,
    pub hidden: Option<bool>,
}

pub async fn fetch_account(fetcher: &Fetcher, account: &str) -> Result<Account, FetchError> {
    let url = format!("https://apply.workable.com/api/v1/widget/accounts/{account}?details=true");
    fetcher.get_json(&url).await
}

pub async fn fetch_all(fetcher: &Fetcher, account: &str) -> Result<Vec<NormalizedJob>, FetchError> {
    Ok(parse(fetch_account(fetcher, account).await?))
}

pub fn parse(account: Account) -> Vec<NormalizedJob> {
    account.jobs.into_iter().map(normalize_job).collect()
}

fn place(
    city: &Option<String>,
    region: &Option<String>,
    country: &Option<String>,
) -> Option<String> {
    let parts: Vec<&str> = [city, region, country]
        .into_iter()
        .flatten()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

fn normalize_job(j: Job) -> NormalizedJob {
    let (description_html, description_text) =
        normalize::clean_html(j.description.as_deref().unwrap_or(""), false);

    let mut locations: Vec<String> = Vec::new();
    for l in j.locations.iter().filter(|l| l.hidden != Some(true)) {
        if let Some(p) = place(&l.city, &l.region, &l.country)
            && !locations.contains(&p)
        {
            locations.push(p);
        }
    }
    if locations.is_empty() {
        locations.extend(place(&j.city, &j.state, &j.country));
    }

    let workplace = if j.telecommuting == Some(true) {
        Workplace::Remote
    } else {
        normalize::infer_workplace(&j.title, &locations, &description_text)
    };
    let (salary, salary_from_text) = normalize::salary_or_text(None, &description_text);
    let posted_at = j
        .published_on
        .as_deref()
        .or(j.created_at.as_deref())
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|d| d.and_utc())
        .unwrap_or_else(Utc::now);
    let title = j.title.trim().to_owned();

    NormalizedJob {
        workplace,
        employment_type: normalize::infer_employment(&title, j.employment_type.as_deref()),
        seniority: normalize::infer_seniority(&title),
        salary,
        salary_from_text,
        content_hash: normalize::content_hash(&title, &locations, &description_text),
        posted_at,
        apply_url: j
            .url
            .unwrap_or_else(|| format!("https://apply.workable.com/j/{}", j.shortcode)),
        department: clean_department(j.department.as_deref()),
        source_job_id: j.shortcode,
        title,
        locations,
        description_html,
        description_text,
    }
}
