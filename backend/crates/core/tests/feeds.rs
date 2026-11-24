//! Job board parsing against trimmed real responses in `tests/data/`
//! (descriptions shortened). Remotive's third job was set to "Europe" by
//! hand; the USAJOBS sample follows its documented response shape, with a
//! second job written by hand for hourly pay and remote work.

use chrono::{Datelike, TimeZone, Utc};
use yob_core::jobs::{
    EmploymentType, SalaryPeriod, Workplace,
    feeds::{self, himalayas, remoteok, remotive, usajobs},
    normalize,
};

fn read(name: &str) -> String {
    std::fs::read_to_string(format!("{}/tests/data/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

#[test]
fn himalayas_page() {
    let response: himalayas::Response = serde_json::from_str(&read("himalayas.json")).unwrap();
    let page = himalayas::parse(response);
    assert!(page.next.is_some(), "a cursor for the next page");
    assert_eq!(page.postings.len(), 3);

    let us = &page.postings[0];
    assert_eq!(us.company, "Seer Interactive");
    assert_eq!(
        us.company_url.as_deref(),
        Some("https://himalayas.app/companies/seer-interactive")
    );
    assert!(us.us_eligible);
    assert_eq!(us.job.locations, ["Remote - United States"]);
    assert_eq!(us.job.workplace, Workplace::Remote);
    assert_eq!(us.job.employment_type, EmploymentType::PartTime);
    let salary = us.job.salary.as_ref().unwrap();
    assert_eq!((salary.min, salary.max), (50, 60));
    assert_eq!(salary.period, SalaryPeriod::Hour);
    assert!(!us.job.salary_from_text);
    assert_eq!(us.job.posted_at.year(), 2026);
    assert!(
        us.url
            .starts_with("https://himalayas.app/companies/seer-interactive/jobs/")
    );
    assert!(!us.job.description_text.is_empty());
    assert!(feeds::keep(us));

    // Restricted to the UK: not for US workers.
    let uk = &page.postings[1];
    assert!(!uk.us_eligible);
    assert!(!feeds::keep(uk));

    // No restriction: anywhere. The id is the guid's number.
    let anywhere = &page.postings[2];
    assert!(anywhere.us_eligible);
    assert_eq!(anywhere.job.locations, ["Remote"]);
    assert_eq!(anywhere.external_id, "5257184543");
    assert_eq!(anywhere.job.employment_type, EmploymentType::Contract);
}

#[test]
fn himalayas_last_page() {
    let response: himalayas::Response =
        serde_json::from_str(r#"{"jobs": [], "nextCursor": "abc"}"#).unwrap();
    assert!(himalayas::parse(response).next.is_none());
}

#[test]
fn remotive_jobs() {
    let response: remotive::Response = serde_json::from_str(&read("remotive.json")).unwrap();
    let page = remotive::parse(response);
    assert!(page.next.is_none(), "one request holds every job");
    assert_eq!(page.postings.len(), 3);

    let us = &page.postings[0];
    assert_eq!(us.external_id, "2091177");
    assert_eq!(us.company, "TELUS Digital");
    assert_eq!(us.job.locations, ["Remote - USA"]);
    assert!(us.us_eligible);
    assert_eq!(us.job.employment_type, EmploymentType::PartTime);
    let salary = us.job.salary.as_ref().unwrap();
    assert_eq!(
        (salary.min, salary.max, salary.period),
        (14, 14, SalaryPeriod::Hour)
    );
    assert_eq!(
        us.job.posted_at,
        Utc.with_ymd_and_hms(2026, 11, 19, 14, 58, 42).unwrap()
    );
    assert_eq!(us.url, us.job.apply_url);

    assert!(page.postings[1].us_eligible, "worldwide");
    assert!(!page.postings[2].us_eligible, "Europe only");
}

#[test]
fn remoteok_jobs() {
    let items: Vec<serde_json::Value> = serde_json::from_str(&read("remoteok.json")).unwrap();
    let page = remoteok::parse(items);
    assert_eq!(page.postings.len(), 3, "the legal notice is skipped");

    let us = &page.postings[0];
    assert_eq!(us.external_id, "1137977");
    assert_eq!(us.company, "Reddit");
    assert_eq!(us.job.title, "Product Manager, Developer Ecosystem");
    assert!(us.us_eligible);
    assert!(us.url.starts_with("https://remoteOK.com/remote-jobs/"));
    assert_eq!(us.job.posted_at.year(), 2026);

    assert!(!page.postings[1].us_eligible, "Europe");
    // Double-encoded text is repaired, and a German city is out of scope.
    let munich = &page.postings[2];
    assert_eq!(munich.job.locations, ["Remote - München"]);
    assert!(!munich.us_eligible);
}

#[test]
fn usajobs_page() {
    let response: usajobs::Response = serde_json::from_str(&read("usajobs.json")).unwrap();
    let page = usajobs::parse(response, 1);
    assert_eq!(page.next.as_deref(), Some("2"));
    assert_eq!(page.postings.len(), 2);

    let it = &page.postings[0];
    assert_eq!(it.external_id, "21947200");
    assert_eq!(it.company, "Space and Naval Warfare Systems Command");
    assert_eq!(
        it.job.department.as_deref(),
        Some("Information Technology Management")
    );
    assert_eq!(
        it.job.locations,
        ["Point Loma Complex, San Diego, California"]
    );
    assert!(it.us_eligible);
    assert_eq!(it.job.workplace, Workplace::Hybrid, "telework eligible");
    assert_eq!(it.job.employment_type, EmploymentType::FullTime);
    let salary = it.job.salary.as_ref().unwrap();
    assert_eq!((salary.min, salary.max), (92_108, 119_746));
    assert_eq!(salary.period, SalaryPeriod::Year);
    assert!(it.job.apply_url.contains("PostingChannelID=RESTAPI"));
    assert_eq!(
        it.url,
        "https://www.usajobs.gov/GetJob/ViewDetails/21947200"
    );
    assert_eq!(
        it.job.posted_at,
        Utc.with_ymd_and_hms(2026, 11, 20, 0, 0, 0).unwrap()
    );
    let html = &it.job.description_html;
    assert!(html.contains("<h3>Duties</h3><ul><li>Monitor network security tools.</li>"));
    assert!(html.contains("<p>Protect Navy networks.</p><p>This is a direct hire position.</p>"));
    assert!(
        !html.contains("<h3>Education</h3>"),
        "empty sections are left out"
    );

    let remote = &page.postings[1];
    assert_eq!(remote.job.workplace, Workplace::Remote);
    assert_eq!(remote.job.employment_type, EmploymentType::PartTime);
    let salary = remote.job.salary.as_ref().unwrap();
    assert_eq!(
        (salary.min, salary.max, salary.period),
        (22, 28, SalaryPeriod::Hour)
    );
    assert!(
        remote
            .job
            .description_html
            .contains("Answer calls &amp; schedule visits.")
    );

    // The last page has no next cursor.
    let response: usajobs::Response = serde_json::from_str(&read("usajobs.json")).unwrap();
    assert!(usajobs::parse(response, 2).next.is_none());
}

#[test]
fn us_scope_of_board_locations() {
    for (loc, ok) in [
        ("Remote - US Only", true),
        ("Remote - Worldwide", true),
        ("Remote - LATAM, Europe, USA, Canada, APAC", true),
        ("Remote - Canada Only", false),
        ("Remote - EMEA", false),
    ] {
        assert_eq!(normalize::us_scope(&[loc.to_owned()]), ok, "{loc}");
    }
}
