//! Adapter parsing against trimmed real responses saved in `tests/data/`
//! (two jobs per platform, descriptions shortened). The second Lever job has
//! a `salaryRange` added by hand to cover that field.

use chrono::{Datelike, TimeZone, Utc};
use yob_core::jobs::{
    EmploymentType, Platform, SalaryPeriod, Workplace, normalize,
    sources::{
        ashby, bamboohr, greenhouse, lever, parse_board_url, recruitee, smartrecruiters, workable,
        workday,
    },
};

#[test]
fn workday_list_and_detail() {
    let site = workday::Site::parse("nvidia/wd5/NVIDIAExternalCareerSite").unwrap();
    let page: workday::Page = serde_json::from_str(&read("workday_list.json")).unwrap();
    let now = Utc.with_ymd_and_hms(2026, 11, 22, 12, 0, 0).unwrap();
    let stubs = workday::parse_list(&site, page.job_postings, now);
    assert_eq!(stubs.len(), 3);

    let us = &stubs[0];
    assert!(us.source_job_id.starts_with("/job/"));
    assert_eq!(us.locations, ["US, CA, Remote"]);
    assert!(normalize::us_scope(&us.locations));
    assert!(
        us.apply_url
            .starts_with("https://nvidia.wd5.myworkdayjobs.com/NVIDIAExternalCareerSite/job/")
    );
    assert!(us.description_text.is_empty(), "stubs have no description");
    // "2 Locations" is a count, not a place: details decide.
    assert!(stubs[1].locations.is_empty());
    // Foreign-only jobs skip the detail request.
    assert!(
        !normalize::us_scope(&stubs[2].locations),
        "{:?}",
        stubs[2].locations
    );

    let detail: workday::Detail = serde_json::from_str(&read("workday_detail.json")).unwrap();
    let job = workday::parse_detail(us, detail, now);
    assert_eq!(job.source_job_id, us.source_job_id);
    assert!(job.description_html.contains('<') && !job.description_text.is_empty());
    assert_eq!(job.employment_type, EmploymentType::FullTime);
    assert_eq!(job.workplace, Workplace::Remote);
    assert_eq!(job.posted_at.day(), 19, "startDate is the posting date");
}

#[test]
fn workday_posted_text() {
    let now = Utc.with_ymd_and_hms(2026, 11, 22, 12, 0, 0).unwrap();
    let days = |s: &str| (now - workday::posted_ago(Some(s), now)).num_days();
    assert_eq!(days("Posted Today"), 0);
    assert_eq!(days("Posted Yesterday"), 1);
    assert_eq!(days("Posted 3 Days Ago"), 3);
    assert_eq!(days("Posted 30+ Days Ago"), 30);
}

#[test]
fn smartrecruiters_list_and_detail() {
    let page: smartrecruiters::Page =
        serde_json::from_str(&read("smartrecruiters_list.json")).unwrap();
    let stubs = smartrecruiters::parse_list("ServiceNow", page.content);
    assert_eq!(stubs.len(), 2);
    assert!(normalize::us_scope(&stubs[0].locations));
    assert!(
        !normalize::us_scope(&stubs[1].locations),
        "{:?}",
        stubs[1].locations
    );
    assert_eq!(stubs[0].workplace, Workplace::Remote);
    assert!(stubs[0].department.is_some());

    let detail: smartrecruiters::Posting =
        serde_json::from_str(&read("smartrecruiters_detail.json")).unwrap();
    let job = smartrecruiters::parse_detail(&stubs[0], detail);
    assert!(job.description_html.contains("<h3>"));
    assert!(
        job.apply_url
            .starts_with("https://jobs.smartrecruiters.com/ServiceNow/")
    );
    assert_eq!(job.workplace, Workplace::Remote);
}

#[test]
fn workable_account() {
    let account: workable::Account = serde_json::from_str(&read("workable.json")).unwrap();
    assert_eq!(account.name.as_deref(), Some("Hugging Face"));
    let jobs = workable::parse(account);
    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].workplace, Workplace::Remote);
    assert_eq!(jobs[1].locations, ["New York, New York, United States"]);
    assert!(normalize::us_scope(&jobs[1].locations));
    assert!(!normalize::us_scope(&jobs[0].locations));
    assert!(
        jobs[0]
            .apply_url
            .starts_with("https://apply.workable.com/j/")
    );
    assert!(!jobs[0].description_text.is_empty());
}

#[test]
fn recruitee_offers() {
    let offers: recruitee::Offers = serde_json::from_str(&read("recruitee.json")).unwrap();
    let jobs = recruitee::parse(offers);
    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].workplace, Workplace::Hybrid);
    assert!(jobs[0].locations.len() > 1, "every location is kept");
    assert!(!normalize::us_scope(&jobs[0].locations));
    assert!(jobs[0].posted_at.year() >= 2026);

    let us = &jobs[1];
    assert!(normalize::us_scope(&us.locations));
    assert_eq!(us.workplace, Workplace::Remote);
    let salary = us.salary.as_ref().expect("salary strings are parsed");
    assert_eq!(
        (salary.min, salary.max, salary.period),
        (90_000, 120_000, SalaryPeriod::Year)
    );
    assert!(!us.salary_from_text);
}

#[test]
fn bamboohr_list_and_detail() {
    let list: bamboohr::List = serde_json::from_str(&read("bamboohr_list.json")).unwrap();
    let stubs = bamboohr::parse_list("rei", list);
    assert_eq!(stubs.len(), 3);
    // "All Locations" and "All Departments" aren't real values.
    assert!(stubs[0].locations.is_empty());
    assert_eq!(stubs[0].department, None);
    assert_eq!(stubs[1].locations, ["Raleigh, North Carolina"]);
    assert_eq!(stubs[1].apply_url, "https://rei.bamboohr.com/careers/42");

    let detail: bamboohr::DetailResponse =
        serde_json::from_str(&read("bamboohr_detail.json")).unwrap();
    let job = bamboohr::parse_detail(&stubs[1], detail.result.job_opening);
    assert_eq!(job.locations, ["Raleigh, North Carolina, United States"]);
    assert_eq!(job.posted_at.year(), 2023);
    let salary = job.salary.as_ref().expect("compensation is parsed");
    assert_eq!((salary.min, salary.max), (70_000, 85_000));
    assert!(!job.description_text.is_empty());
}

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
        (Platform::Workday, "nvidia/wd5/NVIDIAExternalCareerSite"),
        (Platform::Smartrecruiters, "ServiceNow"),
        (Platform::Workable, "huggingface"),
        (Platform::Recruitee, "bunq"),
        (Platform::Bamboohr, "rei"),
    ] {
        let url = yob_core::jobs::sources::careers_url(platform, token);
        assert_eq!(
            parse_board_url(&url),
            Some((platform, token.to_owned())),
            "{url}"
        );
    }
}
