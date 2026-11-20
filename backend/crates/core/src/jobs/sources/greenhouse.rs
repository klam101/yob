//! Greenhouse public job board API:
//! `GET https://boards-api.greenhouse.io/v1/boards/{token}/jobs[?content=true]`.
//! The plain list is light (ids, titles, locations); `content=true` adds the
//! description (HTML-escaped) and departments for every job in one request.
//! Greenhouse doesn't say remote/hybrid or full/part time, so those are inferred.

use std::collections::HashSet;

use chrono::Utc;
use serde::Deserialize;

use super::{clean_department, parse_time};
use crate::jobs::{
    http::{FetchError, Fetcher},
    model::NormalizedJob,
    normalize,
};

#[derive(Deserialize)]
pub struct Board {
    pub jobs: Vec<Job>,
}

#[derive(Deserialize)]
pub struct Job {
    pub id: u64,
    pub title: String,
    pub absolute_url: String,
    pub location: Option<Named>,
    pub first_published: Option<String>,
    pub updated_at: Option<String>,
    pub content: Option<String>,
    #[serde(default)]
    pub departments: Vec<Named>,
}

#[derive(Deserialize)]
pub struct Named {
    pub name: Option<String>,
}

fn url(token: &str, content: bool) -> String {
    let base = format!("https://boards-api.greenhouse.io/v1/boards/{token}/jobs");
    if content {
        format!("{base}?content=true")
    } else {
        base
    }
}

pub async fn fetch_ids(fetcher: &Fetcher, token: &str) -> Result<HashSet<String>, FetchError> {
    let board: Board = fetcher.get_json(&url(token, false)).await?;
    Ok(board.jobs.into_iter().map(|j| j.id.to_string()).collect())
}

pub async fn fetch_all(fetcher: &Fetcher, token: &str) -> Result<Vec<NormalizedJob>, FetchError> {
    let board: Board = fetcher.get_json(&url(token, true)).await?;
    Ok(parse(board))
}

pub fn parse(board: Board) -> Vec<NormalizedJob> {
    board.jobs.into_iter().map(normalize_job).collect()
}

fn normalize_job(j: Job) -> NormalizedJob {
    let (description_html, description_text) =
        normalize::clean_html(j.content.as_deref().unwrap_or(""), true);
    let locations: Vec<String> = j
        .location
        .and_then(|l| l.name)
        .map(|n| n.trim().to_owned())
        .filter(|n| !n.is_empty())
        .into_iter()
        .collect();
    let department = clean_department(j.departments.first().and_then(|d| d.name.as_deref()));
    let posted_at = parse_time(j.first_published.as_deref())
        .or_else(|| parse_time(j.updated_at.as_deref()))
        .unwrap_or_else(Utc::now);

    NormalizedJob {
        source_job_id: j.id.to_string(),
        workplace: normalize::infer_workplace(&j.title, &locations, &description_text),
        employment_type: normalize::infer_employment(&j.title, None),
        seniority: normalize::infer_seniority(&j.title),
        salary: normalize::parse_salary(&description_text),
        content_hash: normalize::content_hash(&j.title, &locations, &description_text),
        title: j.title.trim().to_owned(),
        department,
        locations,
        posted_at,
        apply_url: j.absolute_url,
        description_html,
        description_text,
    }
}
