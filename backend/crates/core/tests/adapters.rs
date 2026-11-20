//! Adapter parsing against trimmed real responses saved in `tests/data/`
//! (two jobs per platform, descriptions shortened). The second Lever job has
//! a `salaryRange` added by hand to cover that field.

use yob_core::jobs::{
    EmploymentType, Platform, SalaryPeriod, Workplace,
    sources::{ashby, greenhouse, lever, parse_board_url},
};

fn read(name: &str) -> String {
    std::fs::read_to_string(format!("{}/tests/data/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

#[test]
fn greenhouse_board() {
    let board: greenhouse::Board = serde_json::from_str(&read("greenhouse.json")).unwrap();
    let jobs = greenhouse::parse(board);
    assert_eq!(jobs.len(), 2);

    let j = &jobs[0];
    assert!(!j.source_job_id.is_empty());
    assert_eq!(j.title, "Abuse Investigator");
    assert_eq!(j.locations.len(), 1);
    assert!(j.apply_url.starts_with("https://"));
    // Escaped HTML is decoded and sanitized.
    assert!(j.description_html.contains('<') && !j.description_html.contains("&lt;"));
    assert!(!j.description_text.contains('<'));
    // Department codes are stripped ("8611 Security Analytics").
    assert_eq!(j.department.as_deref(), Some("Security Analytics"));
    assert_eq!(j.content_hash.len(), 64);
}

#[test]
fn lever_postings() {
    let postings: Vec<lever::Posting> = serde_json::from_str(&read("lever.json")).unwrap();
    let jobs = lever::parse(postings);
    assert_eq!(jobs.len(), 2);

    let j = &jobs[0];
    assert!(!j.locations.is_empty());
    assert!(matches!(
        j.workplace,
        Workplace::Onsite | Workplace::Hybrid | Workplace::Remote
    ));
    assert_eq!(j.employment_type, EmploymentType::FullTime);
    assert!(j.apply_url.starts_with("https://jobs.lever.co/"));
    // Lists are folded into the description as headed sections.
    assert!(j.description_html.contains("<h3>"));

    let salary = jobs[1].salary.as_ref().expect("salaryRange is parsed");
    assert_eq!(
        (salary.min, salary.max, salary.period),
        (120_000, 150_000, SalaryPeriod::Year)
    );
}

#[test]
fn ashby_board() {
    let board: ashby::Board = serde_json::from_str(&read("ashby.json")).unwrap();
    let jobs = ashby::parse(board);
    assert_eq!(jobs.len(), 2);

    let j = &jobs[0];
    assert_eq!(j.title, j.title.trim(), "titles are trimmed");
    let salary = j
        .salary
        .as_ref()
        .expect("structured compensation is parsed");
    assert_eq!(salary.period, SalaryPeriod::Year);
    assert!(salary.min >= 20_000 && salary.max >= salary.min);
    assert!(j.apply_url.starts_with("https://jobs.ashbyhq.com/"));
}

#[test]
fn board_url_round_trip() {
    for (platform, token) in [
        (Platform::Greenhouse, "stripe"),
        (Platform::Lever, "palantir"),
        (Platform::Ashby, "ramp"),
    ] {
        let url = yob_core::jobs::sources::careers_url(platform, token);
        assert_eq!(
            parse_board_url(&url),
            Some((platform, token.to_owned())),
            "{url}"
        );
    }
}
