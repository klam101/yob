//! USAJOBS Search API (US federal jobs): `GET https://data.usajobs.gov/api/Search`
//! with the user's free API key (`Authorization-Key`) and the key holder's
//! email as the User-Agent, as USAJOBS requires. Only jobs open to the
//! public (any U.S. citizen), 500 per page, newest first; the cursor is the
//! page number.

use chrono::{DateTime, NaiveDateTime, Utc};
use serde::Deserialize;

use super::{FeedPosting, JobParts, Page, text_or_html};
use crate::{
    config::UsajobsKey,
    jobs::{
        http::{FetchError, Fetcher},
        model::{Salary, SalaryPeriod, Workplace},
        normalize,
    },
};

const PAGE: usize = 500;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Response {
    pub search_result: SearchResult,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SearchResult {
    #[serde(default)]
    pub search_result_items: Vec<Item>,
    pub user_area: Option<ResultArea>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ResultArea {
    /// A number written as a string.
    pub number_of_pages: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Item {
    pub matched_object_id: Option<serde_json::Value>,
    pub matched_object_descriptor: Descriptor,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Descriptor {
    #[serde(rename = "PositionID")]
    pub position_id: Option<String>,
    pub position_title: String,
    #[serde(rename = "PositionURI")]
    pub position_uri: String,
    #[serde(rename = "ApplyURI", default)]
    pub apply_uri: Vec<String>,
    #[serde(default)]
    pub position_location: Vec<Location>,
    pub organization_name: Option<String>,
    pub department_name: Option<String>,
    #[serde(default)]
    pub job_category: Vec<Named>,
    #[serde(default)]
    pub position_schedule: Vec<Named>,
    #[serde(default)]
    pub position_offering_type: Vec<Named>,
    pub qualification_summary: Option<String>,
    #[serde(default)]
    pub position_remuneration: Vec<Pay>,
    pub publication_start_date: Option<String>,
    pub user_area: Option<UserArea>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Location {
    pub location_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Named {
    pub name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Pay {
    pub minimum_range: Option<serde_json::Value>,
    pub maximum_range: Option<serde_json::Value>,
    pub rate_interval_code: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UserArea {
    pub details: Option<Details>,
}

/// Text fields may be one string or a list of strings.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Details {
    pub job_summary: Option<serde_json::Value>,
    pub major_duties: Option<serde_json::Value>,
    pub key_requirements: Option<serde_json::Value>,
    pub requirements: Option<serde_json::Value>,
    pub education: Option<serde_json::Value>,
    pub other_information: Option<serde_json::Value>,
    pub telework_eligible: Option<bool>,
    pub remote_indicator: Option<bool>,
}

pub async fn fetch_page(
    fetcher: &Fetcher,
    key: &UsajobsKey,
    cursor: Option<&str>,
) -> Result<Page, FetchError> {
    let page: usize = cursor.and_then(|c| c.parse().ok()).unwrap_or(1);
    let url = format!(
        "https://data.usajobs.gov/api/Search?WhoMayApply=public&ResultsPerPage={PAGE}\
         &Page={page}&SortField=opendate&SortDirection=Desc"
    );
    let headers = [
        ("authorization-key", key.api_key.as_str()),
        ("user-agent", key.email.as_str()),
    ];
    let response: Response = fetcher.get_json_with(&url, &headers).await?;
    Ok(parse(response, page))
}

pub fn parse(response: Response, page: usize) -> Page {
    let result = response.search_result;
    let pages = result
        .user_area
        .and_then(|a| a.number_of_pages)
        .and_then(|n| number(&n))
        .unwrap_or(0.0) as usize;
    let next =
        (page < pages && !result.search_result_items.is_empty()).then(|| (page + 1).to_string());
    Page {
        postings: result
            .search_result_items
            .into_iter()
            .filter_map(posting)
            .collect(),
        next,
    }
}

fn posting(item: Item) -> Option<FeedPosting> {
    let d = item.matched_object_descriptor;
    let external_id = item
        .matched_object_id
        .as_ref()
        .and_then(|v| match v {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Number(n) => Some(n.to_string()),
            _ => None,
        })
        .or(d.position_id.clone())?;

    let mut locations: Vec<String> = Vec::new();
    for name in d
        .position_location
        .iter()
        .filter_map(|l| l.location_name.as_deref())
    {
        let name = name.trim().to_owned();
        if !name.is_empty() && !locations.contains(&name) {
            locations.push(name);
        }
    }
    let details = d.user_area.and_then(|u| u.details);
    let workplace = details.as_ref().and_then(|x| {
        if x.remote_indicator == Some(true) {
            Some(Workplace::Remote)
        } else if x.telework_eligible == Some(true) {
            Some(Workplace::Hybrid)
        } else {
            None
        }
    });
    let html = details_html(details.as_ref(), d.qualification_summary.as_deref());
    // "Full-Time" plus "Permanent", "Temporary" or "Internships".
    let employment = d
        .position_schedule
        .iter()
        .chain(&d.position_offering_type)
        .filter_map(|n| n.name.as_deref())
        .collect::<Vec<_>>()
        .join(" ");

    Some(FeedPosting {
        company: d
            .organization_name
            .or(d.department_name)
            .unwrap_or_default(),
        company_url: None,
        url: d.position_uri.clone(),
        us_eligible: normalize::us_scope(&locations),
        job: JobParts {
            id: external_id.clone(),
            title: d.position_title,
            department: d.job_category.into_iter().find_map(|c| c.name),
            locations,
            workplace,
            employment: Some(employment),
            salary: d.position_remuneration.first().and_then(pay),
            posted_at: d
                .publication_start_date
                .as_deref()
                .and_then(date)
                .unwrap_or_else(Utc::now),
            apply_url: d.apply_uri.into_iter().next().unwrap_or(d.position_uri),
            html,
        }
        .build(),
        external_id,
    })
}

fn number(v: &serde_json::Value) -> Option<f64> {
    match v {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// "PA" is per year, "PH" per hour; daily and other rates are left out.
fn pay(p: &Pay) -> Option<Salary> {
    let period = match p.rate_interval_code.as_deref()? {
        "PA" | "Per Year" => SalaryPeriod::Year,
        "PH" | "Per Hour" => SalaryPeriod::Hour,
        _ => return None,
    };
    let min = number(p.minimum_range.as_ref()?)?;
    let max = number(p.maximum_range.as_ref()?)?;
    (min > 0.0 && max >= min).then(|| Salary {
        min: min.round() as u32,
        max: max.round() as u32,
        currency: "USD".to_owned(),
        period,
    })
}

/// "2026-11-20T00:00:00Z" or "2026-11-20T00:00:00.0000" (UTC).
fn date(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .ok()
        .or_else(|| {
            NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f")
                .ok()
                .map(|d| d.and_utc())
        })
}

/// The announcement's sections as HTML, each under a heading.
fn details_html(details: Option<&Details>, qualifications: Option<&str>) -> String {
    let qualifications = qualifications.map(|q| serde_json::Value::String(q.to_owned()));
    let sections: [(&str, Option<&serde_json::Value>); 6] = [
        ("Summary", details.and_then(|d| d.job_summary.as_ref())),
        ("Duties", details.and_then(|d| d.major_duties.as_ref())),
        (
            "Requirements",
            details.and_then(|d| d.key_requirements.as_ref()),
        ),
        ("Qualifications", qualifications.as_ref()),
        ("Education", details.and_then(|d| d.education.as_ref())),
        (
            "Additional information",
            details.and_then(|d| d.other_information.as_ref().or(d.requirements.as_ref())),
        ),
    ];
    let mut html = String::new();
    for (heading, value) in sections {
        let body = match value {
            Some(serde_json::Value::String(s)) if !s.trim().is_empty() => text_or_html(s),
            Some(serde_json::Value::Array(items)) => {
                let items: Vec<String> = items
                    .iter()
                    .filter_map(|i| i.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| format!("<li>{}</li>", html_escape::encode_text(s.trim())))
                    .collect();
                if items.is_empty() {
                    continue;
                }
                format!("<ul>{}</ul>", items.join(""))
            }
            _ => continue,
        };
        html.push_str(&format!("<h3>{heading}</h3>{body}"));
    }
    html
}
