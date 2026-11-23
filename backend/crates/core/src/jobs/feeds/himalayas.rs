//! Himalayas public jobs API: `GET https://himalayas.app/jobs/api?limit=20&cursor=…`.
//! About 120,000 remote jobs, newest first, 20 per page (the most it
//! allows), paged with the `nextCursor` of each answer. Each job says which
//! countries may apply (`locationRestrictions`, empty for anywhere). Jobs
//! link to their Himalayas page; there's no direct employer link.

use chrono::{DateTime, Utc};
use serde::Deserialize;

use super::{FeedPosting, JobParts, Page};
use crate::jobs::{
    http::{FetchError, Fetcher},
    model::{Salary, SalaryPeriod, Workplace},
};

const PAGE: usize = 20;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    #[serde(default)]
    pub jobs: Vec<Job>,
    pub next_cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub title: String,
    pub company_name: Option<String>,
    pub company_slug: Option<String>,
    pub employment_type: Option<String>,
    pub min_salary: Option<f64>,
    pub max_salary: Option<f64>,
    pub salary_period: Option<String>,
    pub currency: Option<String>,
    #[serde(default)]
    pub location_restrictions: Vec<String>,
    #[serde(default)]
    pub parent_categories: Vec<String>,
    pub description: Option<String>,
    pub pub_date: Option<i64>,
    pub application_link: Option<String>,
    pub guid: String,
}

pub async fn fetch_page(fetcher: &Fetcher, cursor: Option<&str>) -> Result<Page, FetchError> {
    let limit = PAGE.to_string();
    let mut params = vec![("limit", limit.as_str())];
    params.extend(cursor.map(|c| ("cursor", c)));
    let url = url::Url::parse_with_params("https://himalayas.app/jobs/api", &params)
        .map_err(|e| FetchError::Invalid(e.to_string()))?;
    let response: Response = fetcher.get_json(url.as_str()).await?;
    Ok(parse(response))
}

pub fn parse(response: Response) -> Page {
    // An empty page ends the feed even if a cursor came back.
    let next = response.next_cursor.filter(|_| !response.jobs.is_empty());
    Page {
        postings: response.jobs.into_iter().map(posting).collect(),
        next,
    }
}

fn posting(j: Job) -> FeedPosting {
    let restrictions = &j.location_restrictions;
    let us = restrictions.iter().any(|r| r.starts_with("United States"));
    // The country list decides scope here, not the location rules: a
    // restriction to "Anguilla" isn't in them.
    let locations = if us {
        vec!["Remote - United States".to_owned()]
    } else if restrictions.is_empty() {
        vec!["Remote".to_owned()]
    } else {
        restrictions
            .iter()
            .take(3)
            .map(|r| format!("Remote - {r}"))
            .collect()
    };
    let us_eligible = us || restrictions.is_empty();

    let external_id = j
        .guid
        .rsplit_once('-')
        .map(|(_, id)| id)
        .filter(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()))
        .unwrap_or(&j.guid)
        .to_owned();
    let salary = salary(&j);
    let posted_at = j
        .pub_date
        .and_then(|s| DateTime::from_timestamp(s, 0))
        .unwrap_or_else(Utc::now);

    FeedPosting {
        company: j.company_name.unwrap_or_default(),
        company_url: j
            .company_slug
            .map(|s| format!("https://himalayas.app/companies/{s}")),
        url: j.guid.clone(),
        us_eligible,
        job: JobParts {
            id: external_id.clone(),
            title: j.title,
            department: j.parent_categories.into_iter().next(),
            locations,
            workplace: Some(Workplace::Remote),
            employment: j.employment_type,
            salary,
            posted_at,
            apply_url: j.application_link.unwrap_or(j.guid),
            html: j.description.unwrap_or_default(),
        }
        .build(),
        external_id,
    }
}

/// Structured pay when a currency is given; monthly pay as yearly.
fn salary(j: &Job) -> Option<Salary> {
    let currency = j.currency.clone()?;
    let (min, max) = match (j.min_salary, j.max_salary) {
        (Some(min), Some(max)) => (min, max),
        (Some(v), None) | (None, Some(v)) => (v, v),
        (None, None) => return None,
    };
    let (period, factor) = match j.salary_period.as_deref() {
        Some("annual") | None => (SalaryPeriod::Year, 1.0),
        Some("monthly") => (SalaryPeriod::Year, 12.0),
        Some("hourly") => (SalaryPeriod::Hour, 1.0),
        Some(_) => return None,
    };
    let (min, max) = (min * factor, max * factor);
    (min > 0.0 && max >= min).then(|| Salary {
        min: min.round() as u32,
        max: max.round() as u32,
        currency,
        period,
    })
}
