//! Turning platform data into consistent fields. Everything here is a pure
//! function so it can be unit-tested without the network or a database.

use std::sync::LazyLock;

use regex::Regex;
use sha2::{Digest, Sha256};

use super::model::{EmploymentType, Salary, SalaryPeriod, Seniority, Workplace};

/// Sanitizes posting HTML (set `escaped` for Greenhouse, which HTML-escapes
/// its content) and returns `(safe_html, plain_text)`.
pub fn clean_html(raw: &str, escaped: bool) -> (String, String) {
    let decoded = if escaped {
        html_escape::decode_html_entities(raw).into_owned()
    } else {
        raw.to_owned()
    };

    let html = ammonia::Builder::default()
        // No remote images or embeds: they'd load third-party content.
        .rm_tags(["img"])
        .link_rel(Some("noopener noreferrer nofollow"))
        .clean(&decoded)
        .to_string();

    (html.clone(), html_to_text(&html))
}

static BLOCK_END: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)</(p|li|h[1-6]|div|ul|ol|tr)>|<br\s*/?>").unwrap());
static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());
static SPACES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[ \t\u{a0}]+").unwrap());
static BLANK_LINES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\n\s*\n+").unwrap());

/// Plain text with line breaks between blocks, for search and AI later.
pub fn html_to_text(html: &str) -> String {
    let with_breaks = BLOCK_END.replace_all(html, "\n");
    let stripped = TAG.replace_all(&with_breaks, "");
    let decoded = html_escape::decode_html_entities(&stripped);
    let spaced = SPACES.replace_all(&decoded, " ");
    let lines: Vec<&str> = spaced.lines().map(str::trim).collect();
    BLANK_LINES
        .replace_all(&lines.join("\n"), "\n\n")
        .trim()
        .to_owned()
}

static WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[a-z0-9+#]+").unwrap());

fn words(s: &str) -> Vec<String> {
    WORD.find_iter(&s.to_lowercase())
        .map(|m| m.as_str().to_owned())
        .collect()
}

/// Seniority from the title. Order matters: "Senior Intern" is an intern,
/// "Staff Engineer" beats "Engineer II".
pub fn infer_seniority(title: &str) -> Seniority {
    let w = words(title);
    let has = |x: &str| w.iter().any(|t| t == x);
    let lower = title.to_lowercase();

    if has("intern") || has("internship") || has("co") && has("op") || lower.contains("co-op") {
        Seniority::Intern
    } else if [
        "staff",
        "principal",
        "distinguished",
        "director",
        "head",
        "vp",
        "chief",
        "fellow",
    ]
    .iter()
    .any(|x| has(x))
        || lower.contains("vice president")
    {
        Seniority::StaffPlus
    } else if has("senior")
        || has("sr")
        || has("lead")
        || has("iii")
        || has("iv")
        // People managers of engineering teams ("Engineering Manager",
        // "Manager, Software Engineering"). Not "Product Manager" or
        // "Account Manager", which are usually mid-level roles.
        || has("manager")
            && ["engineering", "software", "development", "infrastructure"]
                .iter()
                .any(|x| has(x))
    {
        Seniority::Senior
    } else if has("junior")
        || has("jr")
        || has("entry")
        || has("associate")
        || has("graduate")
        || lower.contains("new grad")
        || lower.contains("early career")
        || has("i")
        || has("apprentice")
    {
        Seniority::Entry
    } else {
        Seniority::Mid
    }
}

/// Employment type from a platform hint (Lever's "commitment") or the title.
pub fn infer_employment(title: &str, hint: Option<&str>) -> EmploymentType {
    let text = format!("{} {}", hint.unwrap_or(""), title).to_lowercase();
    if text.contains("intern") || text.contains("co-op") {
        EmploymentType::Internship
    } else if text.contains("part-time") || text.contains("part time") {
        EmploymentType::PartTime
    } else if text.contains("contract")
        || text.contains("temporary")
        || text.contains("temp ")
        || text.contains("freelance")
        || text.contains("fixed term")
    {
        EmploymentType::Contract
    } else {
        EmploymentType::FullTime
    }
}

/// Workplace when the platform doesn't say (Greenhouse): "remote" in a
/// location or the title, then "hybrid" anywhere, otherwise on-site.
pub fn infer_workplace(title: &str, locations: &[String], text: &str) -> Workplace {
    let remote = |s: &str| s.to_lowercase().contains("remote");
    if remote(title) || locations.iter().any(|l| remote(l)) {
        Workplace::Remote
    } else if title.to_lowercase().contains("hybrid")
        || locations
            .iter()
            .any(|l| l.to_lowercase().contains("hybrid"))
        || text.to_lowercase().contains("hybrid")
    {
        Workplace::Hybrid
    } else {
        Workplace::Onsite
    }
}

/// A USD salary range written in the description, e.g. "$120,000 - $150,000",
/// "$55 – $65 per hour" or "$120K to $150K". Returns None when unsure.
pub fn parse_salary(text: &str) -> Option<Salary> {
    static RANGE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)\$\s?(\d{1,3}(?:,\d{3})+|\d+(?:\.\d+)?)\s?(k)?\s*(?:usd\s*)?(?:-|–|—|to)\s*\$?\s?(\d{1,3}(?:,\d{3})+|\d+(?:\.\d+)?)\s?(k)?",
        )
        .unwrap()
    });

    for caps in RANGE.captures_iter(text) {
        let num = |i: usize, k: usize| -> Option<f64> {
            let n: f64 = caps.get(i)?.as_str().replace(',', "").parse().ok()?;
            Some(if caps.get(k).is_some() { n * 1000.0 } else { n })
        };
        let (Some(mut min), Some(max)) = (num(1, 2), num(3, 4)) else {
            continue;
        };
        // "$120-150K": the K applies to both ends.
        if caps.get(2).is_none() && caps.get(4).is_some() && min < 1000.0 {
            min *= 1000.0;
        }
        if min > max {
            continue;
        }
        let after = text[caps.get(0).unwrap().end()..]
            .chars()
            .take(24)
            .collect::<String>()
            .to_lowercase();
        // "$50 - $60 million" is funding, not pay.
        let tail = after.trim_start();
        if tail.starts_with("million") || tail.starts_with("billion") || tail.starts_with("m ") {
            continue;
        }
        let hourly = max < 1000.0 || after.contains("hour") || after.contains("/hr");
        let ok = if hourly {
            (10.0..=500.0).contains(&min) && max <= 500.0
        } else {
            // Above $1M a bare "$" is more likely pesos or another dollar
            // currency than USD. Real seven-figure salaries come from the
            // structured fields (Ashby, Lever), which carry a currency code.
            (20_000.0..=1_000_000.0).contains(&min) && max <= 1_000_000.0
        };
        if ok {
            return Some(Salary {
                min: min.round() as u32,
                max: max.round() as u32,
                currency: "USD".to_owned(),
                period: if hourly {
                    SalaryPeriod::Hour
                } else {
                    SalaryPeriod::Year
                },
            });
        }
    }
    None
}

/// Stable fingerprint of a posting's content. A new job id with the same hash
/// as a closed job at the same company is a repost.
pub fn content_hash(title: &str, locations: &[String], text: &str) -> String {
    let mut h = Sha256::new();
    h.update(words(title).join(" "));
    h.update("\n");
    h.update(locations.join("|").to_lowercase());
    h.update("\n");
    h.update(words(text).join(" "));
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seniority_from_titles() {
        let cases = [
            (
                "Software Engineering Intern, Summer 2027",
                Seniority::Intern,
            ),
            ("Senior Backend Engineer", Seniority::Senior),
            ("Sr. Data Scientist", Seniority::Senior),
            (
                "Staff Software Engineer, Infrastructure",
                Seniority::StaffPlus,
            ),
            ("Director of Engineering", Seniority::StaffPlus),
            ("Software Engineer I", Seniority::Entry),
            ("Software Engineer, Early Career (SF)", Seniority::Entry),
            ("New Grad Software Engineer", Seniority::Entry),
            ("Junior QA Analyst", Seniority::Entry),
            ("Software Engineer II", Seniority::Mid),
            ("Backend Engineer", Seniority::Mid),
            ("Software Engineer III", Seniority::Senior),
            (
                "Manager, Software Engineering (Code Standards)",
                Seniority::Senior,
            ),
            ("Engineering Manager, Payments", Seniority::Senior),
            ("Product Manager", Seniority::Mid),
            ("Account Manager", Seniority::Mid),
        ];
        for (title, want) in cases {
            assert_eq!(infer_seniority(title), want, "{title}");
        }
    }

    #[test]
    fn employment_from_hints() {
        assert_eq!(
            infer_employment("Engineer", Some("Full-time")),
            EmploymentType::FullTime
        );
        assert_eq!(
            infer_employment("Engineer", Some("Contract")),
            EmploymentType::Contract
        );
        assert_eq!(
            infer_employment("Data Intern", None),
            EmploymentType::Internship
        );
        assert_eq!(
            infer_employment("Barista (Part-Time)", None),
            EmploymentType::PartTime
        );
        assert_eq!(infer_employment("Engineer", None), EmploymentType::FullTime);
    }

    #[test]
    fn workplace_inference() {
        let loc = |s: &str| vec![s.to_owned()];
        assert_eq!(
            infer_workplace("Engineer", &loc("Remote - US"), ""),
            Workplace::Remote
        );
        assert_eq!(
            infer_workplace("Engineer", &loc("New York, NY"), "This is a hybrid role"),
            Workplace::Hybrid
        );
        assert_eq!(
            infer_workplace("Engineer", &loc("Austin, TX"), "In office daily"),
            Workplace::Onsite
        );
    }

    #[test]
    fn salary_ranges() {
        let s = parse_salary("The base pay range is $120,000 - $150,000 per year.").unwrap();
        assert_eq!(
            (s.min, s.max, s.period),
            (120_000, 150_000, SalaryPeriod::Year)
        );

        let s = parse_salary("Pay: $55 – $65 per hour").unwrap();
        assert_eq!((s.min, s.max, s.period), (55, 65, SalaryPeriod::Hour));

        let s = parse_salary("$120K to $150K + equity").unwrap();
        assert_eq!((s.min, s.max), (120_000, 150_000));

        let s = parse_salary("Range: $120-150K").unwrap();
        assert_eq!((s.min, s.max), (120_000, 150_000));

        assert!(parse_salary("We raised $50 - $60 million").is_none());
        assert!(parse_salary("No numbers here").is_none());
        assert!(parse_salary("$5 - $8").is_none(), "too low to be a wage");
        assert!(
            parse_salary("$900,000 - $1,254,000 MXN").is_none(),
            "seven-figure \"$\" amounts are probably not USD"
        );
    }

    #[test]
    fn html_cleaning() {
        let (html, text) = clean_html(
            "&lt;h2&gt;About&lt;/h2&gt;&lt;p&gt;We &amp;amp; you&lt;/p&gt;&lt;script&gt;x()&lt;/script&gt;&lt;img src=x&gt;",
            true,
        );
        assert!(html.contains("<h2>About</h2>"));
        assert!(!html.contains("script") && !html.contains("<img"));
        assert_eq!(text, "About\nWe & you");
    }

    #[test]
    fn hash_ignores_formatting() {
        let locs = vec!["NYC".to_owned()];
        assert_eq!(
            content_hash("Software Engineer", &locs, "Build  things!"),
            content_hash("software engineer", &locs, "build things")
        );
        assert_ne!(
            content_hash("Software Engineer", &locs, "a"),
            content_hash("Data Engineer", &locs, "a")
        );
    }
}
