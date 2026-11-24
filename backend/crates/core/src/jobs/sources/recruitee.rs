//! Recruitee public careers API: `GET https://{company}.recruitee.com/api/offers/`.
//! One request returns every published offer with its description,
//! requirements, remote/hybrid flags and (sometimes) a salary range.

use chrono::{NaiveDateTime, Utc};
use serde::Deserialize;

use super::clean_department;
use crate::jobs::{
    http::{FetchError, Fetcher},
    model::{NormalizedJob, Salary, SalaryPeriod, Workplace},
    normalize,
};

#[derive(Deserialize)]
pub struct Offers {
    #[serde(default)]
    pub offers: Vec<Offer>,
}

#[derive(Deserialize)]
pub struct Offer {
    pub id: u64,
    pub title: String,
    pub status: Option<String>,
    pub company_name: Option<String>,
    pub careers_url: Option<String>,
    pub location: Option<String>,
    #[serde(default)]
    pub locations: Vec<Location>,
    pub remote: Option<bool>,
    pub hybrid: Option<bool>,
    pub on_site: Option<bool>,
    pub employment_type_code: Option<String>,
    pub department: Option<String>,
    pub published_at: Option<String>,
    pub created_at: Option<String>,
    pub description: Option<String>,
    pub requirements: Option<String>,
    pub salary: Option<OfferSalary>,
}

#[derive(Deserialize)]
pub struct Location {
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
}

/// Amounts arrive as numbers or strings, depending on the account.
#[derive(Deserialize)]
pub struct OfferSalary {
    pub min: Option<serde_json::Value>,
    pub max: Option<serde_json::Value>,
    pub period: Option<String>,
    pub currency: Option<String>,
}

pub async fn fetch_offers(fetcher: &Fetcher, company: &str) -> Result<Offers, FetchError> {
    fetcher
        .get_json(&format!("https://{company}.recruitee.com/api/offers/"))
        .await
}

pub async fn fetch_all(fetcher: &Fetcher, company: &str) -> Result<Vec<NormalizedJob>, FetchError> {
    Ok(parse(fetch_offers(fetcher, company).await?))
}

pub fn parse(offers: Offers) -> Vec<NormalizedJob> {
    offers
        .offers
        .into_iter()
        .filter(|o| o.status.as_deref().is_none_or(|s| s == "published"))
        .map(normalize_job)
        .collect()
}

fn normalize_job(o: Offer) -> NormalizedJob {
    let mut raw = o.description.clone().unwrap_or_default();
    if let Some(req) = o.requirements.as_deref().filter(|r| !r.trim().is_empty()) {
        raw.push_str("<h3>Requirements</h3>");
        raw.push_str(req);
    }
    let (description_html, description_text) = normalize::clean_html(&raw, false);

    let mut locations: Vec<String> = Vec::new();
    for l in &o.locations {
        let parts: Vec<&str> = [&l.city, &l.state, &l.country]
            .into_iter()
            .flatten()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        let place = parts.join(", ");
        if !place.is_empty() && !locations.contains(&place) {
            locations.push(place);
        }
    }
    if locations.is_empty() {
        locations.extend(
            o.location
                .as_deref()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(str::to_owned),
        );
    }

    let title = o.title.trim().to_owned();
    let workplace = match (o.remote, o.hybrid, o.on_site) {
        (Some(true), _, _) => Workplace::Remote,
        (_, Some(true), _) => Workplace::Hybrid,
        (_, _, Some(true)) => Workplace::Onsite,
        _ => normalize::infer_workplace(&title, &locations, &description_text),
    };
    let hint = o
        .employment_type_code
        .as_deref()
        .map(|c| c.replace('_', " "));
    let (salary, salary_from_text) = normalize::salary_or_text(
        o.salary.as_ref().and_then(structured_salary),
        &description_text,
    );
    let posted_at = o
        .published_at
        .as_deref()
        .or(o.created_at.as_deref())
        .and_then(|t| NaiveDateTime::parse_from_str(t, "%Y-%m-%d %H:%M:%S UTC").ok())
        .map(|t| t.and_utc())
        .unwrap_or_else(Utc::now);

    NormalizedJob {
        source_job_id: o.id.to_string(),
        workplace,
        employment_type: normalize::infer_employment(&title, hint.as_deref()),
        seniority: normalize::infer_seniority(&title),
        salary,
        salary_from_text,
        content_hash: normalize::content_hash(&title, &locations, &description_text),
        posted_at,
        apply_url: o.careers_url.unwrap_or_default(),
        department: clean_department(o.department.as_deref()),
        title,
        locations,
        description_html,
        description_text,
    }
}

fn amount(v: &Option<serde_json::Value>) -> Option<f64> {
    match v.as_ref()? {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.replace([',', ' '], "").parse().ok(),
        _ => None,
    }
}

fn structured_salary(s: &OfferSalary) -> Option<Salary> {
    let (min, max) = (amount(&s.min)?, amount(&s.max)?);
    let (period, factor) = match s.period.as_deref()? {
        "hour" => (SalaryPeriod::Hour, 1.0),
        "month" => (SalaryPeriod::Year, 12.0),
        "year" => (SalaryPeriod::Year, 1.0),
        _ => return None,
    };
    (min > 0.0 && max >= min).then(|| Salary {
        min: (min * factor).round() as u32,
        max: (max * factor).round() as u32,
        currency: s.currency.clone().unwrap_or_else(|| "USD".to_owned()),
        period,
    })
}
