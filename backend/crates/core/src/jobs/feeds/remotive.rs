//! Remotive public API: `GET https://remotive.com/api/remote-jobs`. One
//! request returns every job it shares (a small selection, delayed 24 hours).
//! Remotive asks for at most 4 calls a day, a link back to each job's page
//! and credit as the source.

use std::sync::LazyLock;

use chrono::{NaiveDateTime, Utc};
use regex::Regex;
use serde::Deserialize;

use super::{FeedPosting, JobParts, Page};
use crate::jobs::{
    http::{FetchError, Fetcher},
    model::{Salary, SalaryPeriod, Workplace},
    normalize,
};

#[derive(Deserialize)]
pub struct Response {
    #[serde(default)]
    pub jobs: Vec<Job>,
}

#[derive(Deserialize)]
pub struct Job {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub company_name: Option<String>,
    pub category: Option<String>,
    pub job_type: Option<String>,
    pub publication_date: Option<String>,
    pub candidate_required_location: Option<String>,
    pub salary: Option<String>,
    pub description: Option<String>,
}

pub async fn fetch(fetcher: &Fetcher) -> Result<Page, FetchError> {
    let response: Response = fetcher
        .get_json("https://remotive.com/api/remote-jobs")
        .await?;
    Ok(parse(response))
}

pub fn parse(response: Response) -> Page {
    Page {
        postings: response.jobs.into_iter().map(posting).collect(),
        next: None,
    }
}

/// The salary field: a range ("$170k - $200k", "$14 - $20/hour"), or since
/// it's a field of its own, a single amount ("$14/hour", "$120k").
fn salary(text: &str) -> Option<Salary> {
    static SINGLE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)^\$\s?(\d[\d,]*(?:\.\d+)?)\s?(k)?\s*(?:/\s*|per\s+)?(hour|hr|year|yr)?$")
            .unwrap()
    });
    if let Some(range) = normalize::parse_salary(text) {
        return Some(range);
    }
    let caps = SINGLE.captures(text.trim())?;
    let mut amount: f64 = caps[1].replace(',', "").parse().ok()?;
    if caps.get(2).is_some() {
        amount *= 1000.0;
    }
    let hourly = match caps.get(3).map(|m| m.as_str().to_lowercase()) {
        Some(p) => p.starts_with('h'),
        None => amount < 300.0,
    };
    let plausible = if hourly {
        (7.0..=500.0).contains(&amount)
    } else {
        (10_000.0..=2_000_000.0).contains(&amount)
    };
    plausible.then(|| Salary {
        min: amount.round() as u32,
        max: amount.round() as u32,
        currency: "USD".to_owned(),
        period: if hourly {
            SalaryPeriod::Hour
        } else {
            SalaryPeriod::Year
        },
    })
}

fn posting(j: Job) -> FeedPosting {
    let locations = match j.candidate_required_location.as_deref().map(str::trim) {
        Some(l) if !l.is_empty() => vec![format!("Remote - {l}")],
        _ => vec!["Remote".to_owned()],
    };
    let posted_at = j
        .publication_date
        .as_deref()
        .and_then(|d| NaiveDateTime::parse_from_str(d, "%Y-%m-%dT%H:%M:%S").ok())
        .map(|d| d.and_utc())
        .unwrap_or_else(Utc::now);
    let external_id = j.id.to_string();

    FeedPosting {
        company: j.company_name.unwrap_or_default(),
        company_url: None,
        url: j.url.clone(),
        us_eligible: normalize::us_scope(&locations),
        job: JobParts {
            id: external_id.clone(),
            title: j.title,
            department: j.category,
            locations,
            workplace: Some(Workplace::Remote),
            // "full_time", "part_time", "contract", "freelance", "internship".
            employment: j.job_type.map(|t| t.replace('_', " ")),
            // A field of its own ("$170k - $200k", "$14/hour").
            salary: j.salary.as_deref().and_then(salary),
            posted_at,
            apply_url: j.url,
            html: j.description.unwrap_or_default(),
        }
        .build(),
        external_id,
    }
}
