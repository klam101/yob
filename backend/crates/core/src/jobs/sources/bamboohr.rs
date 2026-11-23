//! BambooHR careers pages, read through the JSON the page itself loads:
//! - List: `GET https://{company}.bamboohr.com/careers/list`, with title,
//!   city and state, department and remote flag.
//! - Detail: `GET …/careers/{id}/detail` adds the description, country,
//!   posting date and compensation, one per new job.
//!
//! A missing company redirects to bamboohr.com, which the fetcher reports
//! as not found.

use chrono::{NaiveDate, Utc};
use serde::Deserialize;

use super::{clean_department, stub};
use crate::jobs::{
    http::{FetchError, Fetcher},
    model::{NormalizedJob, Workplace},
    normalize,
};

#[derive(Deserialize)]
pub struct List {
    #[serde(default)]
    pub result: Vec<Opening>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Opening {
    pub id: String,
    pub job_opening_name: String,
    pub department_label: Option<String>,
    pub employment_status_label: Option<String>,
    #[serde(default)]
    pub location: Place,
    #[serde(default)]
    pub ats_location: Place,
    pub is_remote: Option<bool>,
    pub location_type: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    pub city: Option<String>,
    pub state: Option<String>,
    pub province: Option<String>,
    pub country: Option<String>,
    pub address_country: Option<String>,
}

#[derive(Deserialize)]
pub struct DetailResponse {
    pub result: DetailResult,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailResult {
    pub job_opening: DetailOpening,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailOpening {
    pub job_opening_name: String,
    pub job_opening_share_url: Option<String>,
    pub department_label: Option<String>,
    pub employment_status_label: Option<String>,
    #[serde(default)]
    pub location: Place,
    #[serde(default)]
    pub ats_location: Place,
    pub is_remote: Option<bool>,
    pub location_type: Option<String>,
    pub description: Option<String>,
    /// Free text ("$60,000 - $75,000 / year") or null.
    pub compensation: Option<serde_json::Value>,
    pub date_posted: Option<String>,
}

pub async fn list(fetcher: &Fetcher, company: &str) -> Result<Vec<NormalizedJob>, FetchError> {
    let url = format!("https://{company}.bamboohr.com/careers/list");
    let list: List = fetcher.get_json(&url).await?;
    Ok(parse_list(company, list))
}

pub fn parse_list(company: &str, list: List) -> Vec<NormalizedJob> {
    list.result
        .into_iter()
        .map(|o| {
            let title = o.job_opening_name.trim().to_owned();
            let locations = place(&o.location, &o.ats_location).into_iter().collect();
            let mut job = stub(
                o.id.clone(),
                title,
                locations,
                Utc::now(),
                format!("https://{company}.bamboohr.com/careers/{}", o.id),
            );
            job.workplace =
                workplace(o.is_remote, o.location_type.as_deref()).unwrap_or(Workplace::Onsite);
            job.department = department(o.department_label.as_deref());
            job.employment_type =
                normalize::infer_employment(&job.title, o.employment_status_label.as_deref());
            job
        })
        .collect()
}

pub async fn detail(
    fetcher: &Fetcher,
    company: &str,
    stub: &NormalizedJob,
) -> Result<NormalizedJob, FetchError> {
    let url = format!(
        "https://{company}.bamboohr.com/careers/{}/detail",
        stub.source_job_id
    );
    let detail: DetailResponse = fetcher.get_json(&url).await?;
    Ok(parse_detail(stub, detail.result.job_opening))
}

pub fn parse_detail(stub: &NormalizedJob, o: DetailOpening) -> NormalizedJob {
    let (description_html, description_text) =
        normalize::clean_html(o.description.as_deref().unwrap_or(""), false);
    let title = o.job_opening_name.trim().to_owned();
    let locations: Vec<String> = match place(&o.location, &o.ats_location) {
        Some(p) => vec![p],
        None => stub.locations.clone(),
    };
    let workplace = workplace(o.is_remote, o.location_type.as_deref())
        .unwrap_or_else(|| normalize::infer_workplace(&title, &locations, &description_text));

    let compensation = match &o.compensation {
        Some(serde_json::Value::String(s)) => s.clone(),
        _ => String::new(),
    };
    let structured = normalize::parse_salary(&compensation);
    let (salary, salary_from_text) = normalize::salary_or_text(structured, &description_text);

    let posted_at = o
        .date_posted
        .as_deref()
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|d| d.and_utc())
        .unwrap_or(stub.posted_at);

    NormalizedJob {
        source_job_id: stub.source_job_id.clone(),
        workplace,
        employment_type: normalize::infer_employment(&title, o.employment_status_label.as_deref()),
        seniority: normalize::infer_seniority(&title),
        salary,
        salary_from_text,
        content_hash: normalize::content_hash(&title, &locations, &description_text),
        posted_at,
        apply_url: o
            .job_opening_share_url
            .unwrap_or_else(|| stub.apply_url.clone()),
        department: department(o.department_label.as_deref()),
        title,
        locations,
        description_html,
        description_text,
    }
}

/// "Raleigh, North Carolina, United States". "All Locations" is dropped.
fn place(location: &Place, ats: &Place) -> Option<String> {
    let pick = |a: &Option<String>, b: &Option<String>| {
        a.as_deref()
            .or(b.as_deref())
            .map(str::trim)
            .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("all locations"))
            .map(str::to_owned)
    };
    let parts: Vec<String> = [
        pick(&location.city, &ats.city),
        pick(&location.state, &ats.state).or_else(|| pick(&location.province, &ats.province)),
        pick(&location.address_country, &ats.country),
    ]
    .into_iter()
    .flatten()
    .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

/// BambooHR's location type: 0 on-site, 1 remote, 2 hybrid.
fn workplace(is_remote: Option<bool>, location_type: Option<&str>) -> Option<Workplace> {
    match (is_remote, location_type) {
        (Some(true), _) | (_, Some("1")) => Some(Workplace::Remote),
        (_, Some("2")) => Some(Workplace::Hybrid),
        (_, Some("0")) => Some(Workplace::Onsite),
        _ => None,
    }
}

/// Some companies use the department field for "All Departments".
fn department(label: Option<&str>) -> Option<String> {
    clean_department(label).filter(|d| !d.eq_ignore_ascii_case("all departments"))
}
