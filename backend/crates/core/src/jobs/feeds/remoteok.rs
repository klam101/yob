//! Remote OK public API: `GET https://remoteok.com/api`. One request returns
//! the latest jobs (about 100); the first element is a legal notice. Remote
//! OK asks for a followed link back to each job's page and credit as the
//! source. Some text arrives double-encoded ("MÃ¼nchen").

use chrono::{DateTime, Utc};
use serde::Deserialize;

use super::{FeedPosting, JobParts, Page, fix_mojibake};
use crate::jobs::{
    http::{FetchError, Fetcher},
    model::{Salary, SalaryPeriod, Workplace},
    normalize,
};

#[derive(Deserialize)]
pub struct Job {
    /// A string in practice; a number is accepted too.
    pub id: serde_json::Value,
    pub epoch: Option<i64>,
    pub company: Option<String>,
    pub position: Option<String>,
    pub location: Option<String>,
    pub salary_min: Option<f64>,
    pub salary_max: Option<f64>,
    pub description: Option<String>,
    pub url: Option<String>,
}

pub async fn fetch(fetcher: &Fetcher) -> Result<Page, FetchError> {
    let items: Vec<serde_json::Value> = fetcher.get_json("https://remoteok.com/api").await?;
    Ok(parse(items))
}

pub fn parse(items: Vec<serde_json::Value>) -> Page {
    let postings = items
        .into_iter()
        // The legal notice (and anything else odd) has no id or position.
        .filter_map(|item| serde_json::from_value::<Job>(item).ok())
        .filter_map(posting)
        .collect();
    Page {
        postings,
        next: None,
    }
}

fn posting(j: Job) -> Option<FeedPosting> {
    let external_id = match &j.id {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => return None,
    };
    let title = fix_mojibake(j.position.as_deref()?);
    let url = j.url?;
    let locations = match j.location.as_deref().map(fix_mojibake) {
        Some(l) if !l.trim().is_empty() => vec![format!("Remote - {}", l.trim())],
        _ => vec!["Remote".to_owned()],
    };
    let salary = match (j.salary_min, j.salary_max) {
        (Some(min), Some(max)) if min > 0.0 && max >= min => Some(Salary {
            min: min.round() as u32,
            max: max.round() as u32,
            currency: "USD".to_owned(),
            period: SalaryPeriod::Year,
        }),
        _ => None,
    };

    Some(FeedPosting {
        company: j.company.as_deref().map(fix_mojibake).unwrap_or_default(),
        company_url: None,
        url: url.clone(),
        us_eligible: normalize::us_scope(&locations),
        job: JobParts {
            id: external_id.clone(),
            title,
            department: None,
            locations,
            workplace: Some(Workplace::Remote),
            employment: None,
            salary,
            posted_at: j
                .epoch
                .and_then(|s| DateTime::from_timestamp(s, 0))
                .unwrap_or_else(Utc::now),
            apply_url: url,
            html: fix_mojibake(j.description.as_deref().unwrap_or("")),
        }
        .build(),
        external_id,
    })
}
