//! Lever public postings API:
//! `GET https://api.lever.co/v0/postings/{company}?mode=json`.
//! One request returns every posting with its description, workplace type and
//! (sometimes) salary range. Lever rate-limits quickly; see `http.rs`.

use chrono::{DateTime, Utc};
use serde::Deserialize;

use super::clean_department;
use crate::jobs::{
    http::{FetchError, Fetcher},
    model::{NormalizedJob, Salary, SalaryPeriod, Workplace},
    normalize,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Posting {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub categories: Categories,
    pub created_at: Option<i64>,
    pub description: Option<String>,
    #[serde(default)]
    pub lists: Vec<List>,
    pub additional: Option<String>,
    pub hosted_url: Option<String>,
    pub apply_url: Option<String>,
    pub workplace_type: Option<String>,
    pub salary_range: Option<SalaryRange>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Categories {
    pub commitment: Option<String>,
    pub department: Option<String>,
    pub team: Option<String>,
    pub location: Option<String>,
    #[serde(default)]
    pub all_locations: Vec<String>,
}

#[derive(Deserialize)]
pub struct List {
    pub text: Option<String>,
    pub content: Option<String>,
}

#[derive(Deserialize)]
pub struct SalaryRange {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub currency: Option<String>,
    pub interval: Option<String>,
}

pub async fn fetch_all(fetcher: &Fetcher, company: &str) -> Result<Vec<NormalizedJob>, FetchError> {
    let url = format!("https://api.lever.co/v0/postings/{company}?mode=json");
    let postings: Vec<Posting> = fetcher.get_json(&url).await?;
    Ok(parse(postings))
}

pub fn parse(postings: Vec<Posting>) -> Vec<NormalizedJob> {
    postings.into_iter().map(normalize_job).collect()
}

fn normalize_job(p: Posting) -> NormalizedJob {
    // The posting body is split into an intro, titled lists and a closing part.
    let mut raw = p.description.unwrap_or_default();
    for list in &p.lists {
        if let Some(title) = &list.text {
            raw.push_str(&format!("<h3>{}</h3>", html_escape::encode_text(title)));
        }
        raw.push_str(&format!(
            "<ul>{}</ul>",
            list.content.as_deref().unwrap_or("")
        ));
    }
    raw.push_str(p.additional.as_deref().unwrap_or(""));
    let (description_html, description_text) = normalize::clean_html(&raw, false);

    let mut locations = p.categories.all_locations.clone();
    if locations.is_empty() {
        locations.extend(p.categories.location.clone());
    }

    let workplace = match p.workplace_type.as_deref() {
        Some("remote") => Workplace::Remote,
        Some("hybrid") => Workplace::Hybrid,
        Some("onsite") => Workplace::Onsite,
        _ => normalize::infer_workplace(&p.text, &locations, &description_text),
    };

    let salary = p
        .salary_range
        .as_ref()
        .and_then(salary_from_range)
        .or_else(|| normalize::parse_salary(&description_text));

    let department = clean_department(
        p.categories
            .department
            .as_deref()
            .or(p.categories.team.as_deref()),
    );

    NormalizedJob {
        source_job_id: p.id,
        workplace,
        employment_type: normalize::infer_employment(&p.text, p.categories.commitment.as_deref()),
        seniority: normalize::infer_seniority(&p.text),
        salary,
        content_hash: normalize::content_hash(&p.text, &locations, &description_text),
        posted_at: p
            .created_at
            .and_then(DateTime::from_timestamp_millis)
            .unwrap_or_else(Utc::now),
        apply_url: p.hosted_url.or(p.apply_url).unwrap_or_default(),
        title: p.text.trim().to_owned(),
        department,
        locations,
        description_html,
        description_text,
    }
}

fn salary_from_range(r: &SalaryRange) -> Option<Salary> {
    let (min, max) = (r.min?, r.max?);
    let interval = r.interval.as_deref().unwrap_or("").to_lowercase();
    let (period, factor) = if interval.contains("hour") {
        (SalaryPeriod::Hour, 1.0)
    } else if interval.contains("month") {
        (SalaryPeriod::Year, 12.0)
    } else {
        (SalaryPeriod::Year, 1.0)
    };
    (min > 0.0 && max >= min).then(|| Salary {
        min: (min * factor).round() as u32,
        max: (max * factor).round() as u32,
        currency: r.currency.clone().unwrap_or_else(|| "USD".to_owned()),
        period,
    })
}
