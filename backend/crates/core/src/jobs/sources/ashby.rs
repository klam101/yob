//! Ashby public job board API:
//! `GET https://api.ashbyhq.com/posting-api/job-board/{name}?includeCompensation=true`.
//! One request returns every listed job with workplace type, employment type
//! and structured compensation.

use chrono::Utc;
use serde::Deserialize;

use super::{clean_department, parse_time};
use crate::jobs::{
    http::{FetchError, Fetcher},
    model::{EmploymentType, NormalizedJob, Salary, SalaryPeriod, Workplace},
    normalize,
};

#[derive(Deserialize)]
pub struct Board {
    pub jobs: Vec<Job>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    pub title: String,
    pub department: Option<String>,
    pub team: Option<String>,
    pub employment_type: Option<String>,
    pub location: Option<String>,
    #[serde(default)]
    pub secondary_locations: Vec<SecondaryLocation>,
    pub published_at: Option<String>,
    pub is_listed: Option<bool>,
    pub is_remote: Option<bool>,
    pub workplace_type: Option<String>,
    pub job_url: Option<String>,
    pub apply_url: Option<String>,
    pub description_html: Option<String>,
    pub compensation: Option<Compensation>,
}

#[derive(Deserialize)]
pub struct SecondaryLocation {
    pub location: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Compensation {
    #[serde(default)]
    pub summary_components: Vec<Component>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Component {
    pub compensation_type: Option<String>,
    pub interval: Option<String>,
    pub currency_code: Option<String>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
}

pub async fn fetch_all(fetcher: &Fetcher, name: &str) -> Result<Vec<NormalizedJob>, FetchError> {
    let url =
        format!("https://api.ashbyhq.com/posting-api/job-board/{name}?includeCompensation=true");
    let board: Board = fetcher.get_json(&url).await?;
    Ok(parse(board))
}

pub fn parse(board: Board) -> Vec<NormalizedJob> {
    board
        .jobs
        .into_iter()
        .filter(|j| j.is_listed != Some(false))
        .map(normalize_job)
        .collect()
}

fn normalize_job(j: Job) -> NormalizedJob {
    let (description_html, description_text) =
        normalize::clean_html(j.description_html.as_deref().unwrap_or(""), false);

    let mut locations: Vec<String> = j.location.iter().map(|l| l.trim().to_owned()).collect();
    for s in &j.secondary_locations {
        if let Some(l) = s.location.as_deref().map(str::trim)
            && !l.is_empty()
            && !locations.iter().any(|x| x == l)
        {
            locations.push(l.to_owned());
        }
    }

    let workplace = match j.workplace_type.as_deref() {
        Some("Remote") => Workplace::Remote,
        Some("Hybrid") => Workplace::Hybrid,
        Some("OnSite") => Workplace::Onsite,
        _ if j.is_remote == Some(true) => Workplace::Remote,
        _ => normalize::infer_workplace(&j.title, &locations, &description_text),
    };

    let employment_type = match j.employment_type.as_deref() {
        Some("FullTime") => EmploymentType::FullTime,
        Some("PartTime") => EmploymentType::PartTime,
        Some("Intern") => EmploymentType::Internship,
        Some("Contract" | "Temporary") => EmploymentType::Contract,
        _ => normalize::infer_employment(&j.title, None),
    };

    let salary = j
        .compensation
        .as_ref()
        .and_then(salary_from_components)
        .or_else(|| normalize::parse_salary(&description_text));

    NormalizedJob {
        source_job_id: j.id,
        workplace,
        employment_type,
        seniority: normalize::infer_seniority(&j.title),
        salary,
        content_hash: normalize::content_hash(&j.title, &locations, &description_text),
        posted_at: parse_time(j.published_at.as_deref()).unwrap_or_else(Utc::now),
        apply_url: j.job_url.or(j.apply_url).unwrap_or_default(),
        department: clean_department(j.department.as_deref().or(j.team.as_deref())),
        title: j.title.trim().to_owned(),
        locations,
        description_html,
        description_text,
    }
}

fn salary_from_components(c: &Compensation) -> Option<Salary> {
    let s = c
        .summary_components
        .iter()
        .find(|s| s.compensation_type.as_deref() == Some("Salary"))?;
    let (min, max) = (s.min_value?, s.max_value?);
    let interval = s.interval.as_deref().unwrap_or("");
    let (period, factor) = match interval {
        "1 HOUR" => (SalaryPeriod::Hour, 1.0),
        "1 MONTH" => (SalaryPeriod::Year, 12.0),
        "1 YEAR" => (SalaryPeriod::Year, 1.0),
        _ => return None,
    };
    (min > 0.0 && max >= min).then(|| Salary {
        min: (min * factor).round() as u32,
        max: (max * factor).round() as u32,
        currency: s.currency_code.clone().unwrap_or_else(|| "USD".to_owned()),
        period,
    })
}
