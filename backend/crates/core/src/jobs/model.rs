//! Platform-neutral job types. Every adapter produces [`NormalizedJob`]s, and
//! enums are stored in Postgres as their snake_case names.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Defines a string-backed enum: serde and the database use the same
/// snake_case names, and `as_str`/`parse` convert between them.
/// The test below checks that each `$text` matches serde's name.
macro_rules! text_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        $(#[$meta])*
        // Every $text is the snake_case form of its variant, so one
        // rename_all keeps serde, the OpenAPI schema and Postgres in agreement.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
        #[serde(rename_all = "snake_case")]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            pub const ALL: &[Self] = &[$(Self::$variant),+];

            pub fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $text),+ }
            }

            pub fn parse(s: &str) -> Option<Self> {
                match s { $($text => Some(Self::$variant),)+ _ => None }
            }
        }
    };
}

text_enum!(
    /// The hiring platform a company's job board runs on.
    Platform {
        Greenhouse => "greenhouse",
        Lever => "lever",
        Ashby => "ashby",
        Workday => "workday",
        Smartrecruiters => "smartrecruiters",
        Workable => "workable",
        Recruitee => "recruitee",
        Bamboohr => "bamboohr",
    }
);
text_enum!(
    /// A job board with a public API. Its jobs are matched against the
    /// employers' own boards (see `jobs::feeds`).
    Feed {
        Himalayas => "himalayas",
        Remotive => "remotive",
        Remoteok => "remoteok",
        Usajobs => "usajobs",
    }
);
text_enum!(
    /// Where a job came from: an employer's hiring platform, or a job board
    /// when no employer copy is tracked. The feed's Source filter.
    Source {
        Greenhouse => "greenhouse",
        Lever => "lever",
        Ashby => "ashby",
        Workday => "workday",
        Smartrecruiters => "smartrecruiters",
        Workable => "workable",
        Recruitee => "recruitee",
        Bamboohr => "bamboohr",
        Himalayas => "himalayas",
        Remotive => "remotive",
        Remoteok => "remoteok",
        Usajobs => "usajobs",
    }
);
text_enum!(Workplace { Onsite => "onsite", Hybrid => "hybrid", Remote => "remote" });
text_enum!(EmploymentType {
    FullTime => "full_time",
    PartTime => "part_time",
    Contract => "contract",
    Internship => "internship",
});
text_enum!(Seniority {
    Intern => "intern",
    Entry => "entry",
    Mid => "mid",
    Senior => "senior",
    StaffPlus => "staff_plus",
});
text_enum!(SalaryPeriod { Year => "year", Hour => "hour" });

#[derive(Debug, Clone, PartialEq)]
pub struct Salary {
    pub min: u32,
    pub max: u32,
    pub currency: String,
    pub period: SalaryPeriod,
}

/// One job, read from any platform and normalized to a single shape.
#[derive(Debug, Clone)]
pub struct NormalizedJob {
    /// The platform's own id for the job, unique within a company.
    pub source_job_id: String,
    pub title: String,
    pub department: Option<String>,
    pub locations: Vec<String>,
    pub workplace: Workplace,
    pub employment_type: EmploymentType,
    pub seniority: Seniority,
    pub salary: Option<Salary>,
    /// The salary was parsed from the description, not a platform field.
    pub salary_from_text: bool,
    pub posted_at: DateTime<Utc>,
    pub apply_url: String,
    /// Sanitized HTML, safe to render.
    pub description_html: String,
    pub description_text: String,
    /// Hash of title, locations and description text, used to spot reposts.
    pub content_hash: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check<T: Serialize + Copy>(all: &[T], as_str: fn(T) -> &'static str) {
        for v in all {
            assert_eq!(serde_json::to_value(v).unwrap(), as_str(*v));
        }
    }

    #[test]
    fn database_names_match_serde() {
        check(Platform::ALL, Platform::as_str);
        check(Feed::ALL, Feed::as_str);
        check(Source::ALL, Source::as_str);
        check(Workplace::ALL, Workplace::as_str);
        check(EmploymentType::ALL, EmploymentType::as_str);
        check(Seniority::ALL, Seniority::as_str);
        check(SalaryPeriod::ALL, SalaryPeriod::as_str);
    }

    #[test]
    fn every_platform_and_feed_is_a_source() {
        let names = Platform::ALL
            .iter()
            .map(|p| p.as_str())
            .chain(Feed::ALL.iter().map(|f| f.as_str()));
        for name in names {
            assert!(Source::parse(name).is_some(), "{name} missing from Source");
        }
        assert_eq!(Source::ALL.len(), Platform::ALL.len() + Feed::ALL.len());
    }
}
