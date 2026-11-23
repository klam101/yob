//! SmartRecruiters public Posting API (no key needed):
//! - List: `GET https://api.smartrecruiters.com/v1/companies/{id}/postings`,
//!   100 per page, with title, location, department and employment type.
//! - Detail: `GET …/postings/{postingId}` adds the job ad text, one per new job.
//!
//! An unknown company id answers with an empty list, not a 404.

use chrono::Utc;
use serde::Deserialize;

use super::{clean_department, parse_time, stub};
use crate::jobs::{
    http::{FetchError, Fetcher},
    model::{NormalizedJob, Workplace},
    normalize,
};

const PAGE: usize = 100;
const MAX_JOBS: usize = 25_000;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    #[serde(default)]
    pub total_found: usize,
    #[serde(default)]
    pub content: Vec<Posting>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Posting {
    pub id: String,
    pub name: String,
    pub released_date: Option<String>,
    #[serde(default)]
    pub location: Location,
    pub department: Option<Label>,
    pub type_of_employment: Option<Label>,
    pub company: Option<Company>,
    pub posting_url: Option<String>,
    pub job_ad: Option<JobAd>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub full_location: Option<String>,
    pub city: Option<String>,
    pub region: Option<String>,
    pub country: Option<String>,
    pub remote: Option<bool>,
    pub hybrid: Option<bool>,
}

#[derive(Deserialize)]
pub struct Label {
    pub label: Option<String>,
}

#[derive(Deserialize)]
pub struct Company {
    pub name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobAd {
    pub sections: Sections,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sections {
    pub company_description: Option<Section>,
    pub job_description: Option<Section>,
    pub qualifications: Option<Section>,
    pub additional_information: Option<Section>,
}

#[derive(Deserialize)]
pub struct Section {
    pub title: Option<String>,
    pub text: Option<String>,
}

pub async fn fetch_page(
    fetcher: &Fetcher,
    company: &str,
    offset: usize,
) -> Result<Page, FetchError> {
    let url = format!(
        "https://api.smartrecruiters.com/v1/companies/{company}/postings?limit={PAGE}&offset={offset}"
    );
    fetcher.get_json(&url).await
}

/// Every open job as a stub (no description).
pub async fn list(fetcher: &Fetcher, company: &str) -> Result<Vec<NormalizedJob>, FetchError> {
    let mut postings = Vec::new();
    loop {
        let page = fetch_page(fetcher, company, postings.len()).await?;
        let got = page.content.len();
        postings.extend(page.content);
        if got < PAGE || postings.len() >= page.total_found.min(MAX_JOBS) {
            break;
        }
    }
    Ok(parse_list(company, postings))
}

pub fn parse_list(company: &str, postings: Vec<Posting>) -> Vec<NormalizedJob> {
    postings
        .into_iter()
        .map(|p| {
            let mut job = stub(
                p.id.clone(),
                p.name.trim().to_owned(),
                locations(&p.location),
                parse_time(p.released_date.as_deref()).unwrap_or_else(Utc::now),
                format!("https://jobs.smartrecruiters.com/{company}/{}", p.id),
            );
            fill_fields(&mut job, &p);
            job
        })
        .collect()
}

/// The full job for a stub from [`list`].
pub async fn detail(
    fetcher: &Fetcher,
    company: &str,
    stub: &NormalizedJob,
) -> Result<NormalizedJob, FetchError> {
    let url = format!(
        "https://api.smartrecruiters.com/v1/companies/{company}/postings/{}",
        stub.source_job_id
    );
    let posting: Posting = fetcher.get_json(&url).await?;
    Ok(parse_detail(stub, posting))
}

pub fn parse_detail(stub: &NormalizedJob, p: Posting) -> NormalizedJob {
    let mut raw = String::new();
    if let Some(ad) = &p.job_ad {
        let s = &ad.sections;
        // The role first; the company blurb goes last.
        for section in [
            &s.job_description,
            &s.qualifications,
            &s.additional_information,
            &s.company_description,
        ]
        .into_iter()
        .flatten()
        {
            let Some(text) = section.text.as_deref().filter(|t| !t.trim().is_empty()) else {
                continue;
            };
            if let Some(title) = &section.title {
                raw.push_str(&format!("<h3>{}</h3>", html_escape::encode_text(title)));
            }
            raw.push_str(text);
        }
    }
    let (description_html, description_text) = normalize::clean_html(&raw, false);

    let mut job = stub.clone();
    let locations = locations(&p.location);
    if !locations.is_empty() {
        job.locations = locations;
    }
    job.title = p.name.trim().to_owned();
    fill_fields(&mut job, &p);
    if job.workplace == Workplace::Onsite && p.location.hybrid.is_none() {
        job.workplace = normalize::infer_workplace(&job.title, &job.locations, &description_text);
    }
    (job.salary, job.salary_from_text) = normalize::salary_or_text(None, &description_text);
    job.content_hash = normalize::content_hash(&job.title, &job.locations, &description_text);
    if let Some(url) = p.posting_url {
        job.apply_url = url;
    }
    job.description_html = description_html;
    job.description_text = description_text;
    job
}

/// Fields present on both the list and the detail.
fn fill_fields(job: &mut NormalizedJob, p: &Posting) {
    job.workplace = if p.location.remote == Some(true) {
        Workplace::Remote
    } else if p.location.hybrid == Some(true) {
        Workplace::Hybrid
    } else {
        Workplace::Onsite
    };
    job.department = clean_department(p.department.as_ref().and_then(|d| d.label.as_deref()));
    job.employment_type = normalize::infer_employment(
        &job.title,
        p.type_of_employment
            .as_ref()
            .and_then(|t| t.label.as_deref()),
    );
    if let Some(t) = parse_time(p.released_date.as_deref()) {
        job.posted_at = t;
    }
}

fn locations(l: &Location) -> Vec<String> {
    let full = l.full_location.clone().or_else(|| {
        let parts: Vec<&str> = [&l.city, &l.region, &l.country]
            .into_iter()
            .flatten()
            .map(String::as_str)
            .filter(|s| !s.is_empty())
            .collect();
        (!parts.is_empty()).then(|| parts.join(", "))
    });
    full.map(|f| f.trim().to_owned())
        .filter(|f| !f.is_empty())
        .into_iter()
        .collect()
}

/// The company's display name, from any one of its postings.
pub async fn board_name(fetcher: &Fetcher, company: &str) -> Option<String> {
    let page = fetch_page(fetcher, company, 0).await.ok()?;
    page.content.into_iter().find_map(|p| p.company?.name)
}
