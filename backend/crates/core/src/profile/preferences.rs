//! Search preferences from the onboarding questionnaire: what jobs the user
//! wants. They are the feed's default filters and are only ever changed by
//! the user; the resume never touches them.
//!
//! Work authorization is asked in the questionnaire too, but it's stored in
//! `application_answers` (the same answers autofill uses), so there is one
//! copy of it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use super::{ProfileError, invalid, short};
use crate::jobs::{EmploymentType, Seniority, Workplace};

/// Entries allowed in each list.
const MAX_LIST: usize = 30;

/// Company headcount bands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CompanySize {
    /// 1 to 50 people.
    Startup,
    /// 51 to 200.
    Small,
    /// 201 to 1,000.
    Medium,
    /// 1,001 to 10,000.
    Large,
    /// More than 10,000.
    Enterprise,
}

impl CompanySize {
    fn as_str(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
            Self::Enterprise => "enterprise",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        [
            Self::Startup,
            Self::Small,
            Self::Medium,
            Self::Large,
            Self::Enterprise,
        ]
        .into_iter()
        .find(|v| v.as_str() == s)
    }
}

/// The work authorization questions, shared with `application_answers`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct WorkAuthorization {
    pub authorized_us: Option<bool>,
    pub needs_sponsorship: Option<bool>,
    pub us_citizen: Option<bool>,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct Preferences {
    /// Job titles to look for, e.g. "Software Engineer". Any of them matches.
    pub titles: Vec<String>,
    pub seniority: Vec<Seniority>,
    pub employment_types: Vec<EmploymentType>,
    pub workplaces: Vec<Workplace>,
    /// Cities or states, e.g. "Austin, TX" or "Texas". Remote jobs are shown
    /// regardless when `workplaces` includes remote.
    pub locations: Vec<String>,
    /// Minimum yearly salary in USD.
    pub salary_min: Option<u32>,
    /// With `salary_min`, still show jobs that don't list a salary.
    #[serde(default = "yes")]
    pub include_unlisted_salary: bool,
    /// Not used by the feed until company data is collected.
    pub company_sizes: Vec<CompanySize>,
    /// Not used by the feed until company data is collected.
    pub industries: Vec<String>,
    /// Deal-breakers: never show these companies.
    pub excluded_companies: Vec<String>,
    /// Deal-breakers: hide titles with these words, e.g. "manager".
    pub excluded_title_words: Vec<String>,
    pub work_authorization: WorkAuthorization,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            titles: Vec::new(),
            seniority: Vec::new(),
            employment_types: Vec::new(),
            workplaces: Vec::new(),
            locations: Vec::new(),
            salary_min: None,
            include_unlisted_salary: true,
            company_sizes: Vec::new(),
            industries: Vec::new(),
            excluded_companies: Vec::new(),
            excluded_title_words: Vec::new(),
            work_authorization: WorkAuthorization::default(),
        }
    }
}

/// Where the user is with the questionnaire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Onboarding {
    /// Never seen it: the feed sends the user there first.
    NotStarted,
    /// Chose "Skip for now".
    Skipped,
    Done,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SearchPreferences {
    pub onboarding: Onboarding,
    pub preferences: Preferences,
    pub updated_at: Option<DateTime<Utc>>,
}

// --- validation --------------------------------------------------------------

/// Trims, drops empties and case-insensitive duplicates, checks lengths.
fn texts(field: &str, items: &[String], max_len: usize) -> Result<Vec<String>, ProfileError> {
    let mut out: Vec<String> = Vec::new();
    for item in items {
        let v = short(field, item)?;
        if v.chars().count() > max_len {
            return Err(invalid(format!(
                "{field}: \"{v}\" is too long (at most {max_len} characters)."
            )));
        }
        if !v.is_empty() && !out.iter().any(|o| o.eq_ignore_ascii_case(&v)) {
            out.push(v);
        }
    }
    if out.len() > MAX_LIST {
        return Err(invalid(format!("{field}: at most {MAX_LIST} entries.")));
    }
    Ok(out)
}

fn unique<T: PartialEq + Copy>(items: &[T]) -> Vec<T> {
    let mut out = Vec::new();
    for &i in items {
        if !out.contains(&i) {
            out.push(i);
        }
    }
    out
}

/// Cleans what the user entered. `;` separates places in the feed's
/// Location filter and `,` separates list values, so neither may appear
/// where it would split an entry.
pub fn validate(p: &Preferences) -> Result<Preferences, ProfileError> {
    let titles = texts("Job titles", &p.titles, 100)?;
    if titles.iter().any(|t| t.contains(',')) {
        return Err(invalid(
            "Job titles can't contain commas; add each title separately.",
        ));
    }
    let locations = texts("Locations", &p.locations, 100)?;
    if locations.iter().any(|l| l.contains(';')) {
        return Err(invalid(
            "Locations can't contain semicolons; add each place separately.",
        ));
    }
    let excluded_title_words = texts("Excluded title words", &p.excluded_title_words, 60)?;
    if excluded_title_words.iter().any(|t| t.contains(',')) {
        return Err(invalid(
            "Excluded title words can't contain commas; add each separately.",
        ));
    }
    let excluded_companies = texts("Excluded companies", &p.excluded_companies, 120)?;
    if excluded_companies.iter().any(|t| t.contains(',')) {
        return Err(invalid("Excluded companies can't contain commas."));
    }
    if let Some(s) = p.salary_min
        && s > 2_000_000
    {
        return Err(invalid(
            "The salary floor looks too high (it's yearly, in USD).",
        ));
    }
    Ok(Preferences {
        titles,
        seniority: unique(&p.seniority),
        employment_types: unique(&p.employment_types),
        workplaces: unique(&p.workplaces),
        locations,
        salary_min: p.salary_min.filter(|&s| s > 0),
        include_unlisted_salary: p.include_unlisted_salary,
        company_sizes: unique(&p.company_sizes),
        industries: texts("Industries", &p.industries, 80)?,
        excluded_companies,
        excluded_title_words,
        work_authorization: p.work_authorization.clone(),
    })
}

// --- storage -----------------------------------------------------------------

type Row = (
    Vec<String>,
    Vec<String>,
    Vec<String>,
    Vec<String>,
    Vec<String>,
    Option<i32>,
    bool,
    Vec<String>,
    Vec<String>,
    Vec<String>,
    Vec<String>,
    Option<DateTime<Utc>>,
    DateTime<Utc>,
);

fn parse_all<T>(items: Vec<String>, parse: fn(&str) -> Option<T>) -> Vec<T> {
    items.iter().filter_map(|s| parse(s)).collect()
}

pub async fn load(pool: &PgPool, user_id: Uuid) -> Result<SearchPreferences, ProfileError> {
    let row: Option<Row> = sqlx::query_as(
        "SELECT titles, seniority, employment_types, workplaces, locations, salary_min,
                include_unlisted_salary, company_sizes, industries, excluded_companies,
                excluded_title_words, completed_at, updated_at
         FROM search_preferences WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    let auth: Option<(Option<bool>, Option<bool>, Option<bool>)> = sqlx::query_as(
        "SELECT authorized_us, needs_sponsorship, us_citizen
         FROM application_answers WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    let work_authorization = auth
        .map(
            |(authorized_us, needs_sponsorship, us_citizen)| WorkAuthorization {
                authorized_us,
                needs_sponsorship,
                us_citizen,
            },
        )
        .unwrap_or_default();

    let Some(r) = row else {
        return Ok(SearchPreferences {
            onboarding: Onboarding::NotStarted,
            preferences: Preferences {
                work_authorization,
                ..Preferences::default()
            },
            updated_at: None,
        });
    };
    Ok(SearchPreferences {
        onboarding: if r.11.is_some() {
            Onboarding::Done
        } else {
            Onboarding::Skipped
        },
        preferences: Preferences {
            titles: r.0,
            seniority: parse_all(r.1, Seniority::parse),
            employment_types: parse_all(r.2, EmploymentType::parse),
            workplaces: parse_all(r.3, Workplace::parse),
            locations: r.4,
            salary_min: r.5.map(|s| s.max(0) as u32),
            include_unlisted_salary: r.6,
            company_sizes: parse_all(r.7, CompanySize::parse),
            industries: r.8,
            excluded_companies: r.9,
            excluded_title_words: r.10,
            work_authorization,
        },
        updated_at: Some(r.12),
    })
}

/// Saves the answers and marks the questionnaire done. Work authorization
/// goes to `application_answers`, leaving its other answers alone.
pub async fn save(
    pool: &PgPool,
    user_id: Uuid,
    p: &Preferences,
) -> Result<SearchPreferences, ProfileError> {
    let p = validate(p)?;
    let names = |v: Vec<&'static str>| v.into_iter().map(str::to_owned).collect::<Vec<_>>();

    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO search_preferences (
             user_id, titles, seniority, employment_types, workplaces, locations, salary_min,
             include_unlisted_salary, company_sizes, industries, excluded_companies,
             excluded_title_words, completed_at, updated_at
         )
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, now(), now())
         ON CONFLICT (user_id) DO UPDATE SET
             titles = EXCLUDED.titles, seniority = EXCLUDED.seniority,
             employment_types = EXCLUDED.employment_types, workplaces = EXCLUDED.workplaces,
             locations = EXCLUDED.locations, salary_min = EXCLUDED.salary_min,
             include_unlisted_salary = EXCLUDED.include_unlisted_salary,
             company_sizes = EXCLUDED.company_sizes, industries = EXCLUDED.industries,
             excluded_companies = EXCLUDED.excluded_companies,
             excluded_title_words = EXCLUDED.excluded_title_words,
             completed_at = COALESCE(search_preferences.completed_at, now()),
             updated_at = now()",
    )
    .bind(user_id)
    .bind(&p.titles)
    .bind(names(p.seniority.iter().map(|s| s.as_str()).collect()))
    .bind(names(
        p.employment_types.iter().map(|s| s.as_str()).collect(),
    ))
    .bind(names(p.workplaces.iter().map(|s| s.as_str()).collect()))
    .bind(&p.locations)
    .bind(p.salary_min.map(|s| s as i32))
    .bind(p.include_unlisted_salary)
    .bind(names(p.company_sizes.iter().map(|s| s.as_str()).collect()))
    .bind(&p.industries)
    .bind(&p.excluded_companies)
    .bind(&p.excluded_title_words)
    .execute(&mut *tx)
    .await?;

    let a = &p.work_authorization;
    sqlx::query(
        "INSERT INTO application_answers (user_id, authorized_us, needs_sponsorship, us_citizen, updated_at)
         VALUES ($1, $2, $3, $4, now())
         ON CONFLICT (user_id) DO UPDATE SET
             authorized_us = EXCLUDED.authorized_us,
             needs_sponsorship = EXCLUDED.needs_sponsorship,
             us_citizen = EXCLUDED.us_citizen,
             updated_at = now()",
    )
    .bind(user_id)
    .bind(a.authorized_us)
    .bind(a.needs_sponsorship)
    .bind(a.us_citizen)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    load(pool, user_id).await
}

/// "Skip for now": the feed stops sending the user to the questionnaire.
/// Answers saved earlier are kept.
pub async fn skip(pool: &PgPool, user_id: Uuid) -> Result<SearchPreferences, ProfileError> {
    sqlx::query(
        "INSERT INTO search_preferences (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING",
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    load(pool, user_id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_lists() {
        let p = Preferences {
            titles: vec![
                " Software  Engineer ".into(),
                "software engineer".into(),
                "".into(),
            ],
            seniority: vec![Seniority::Mid, Seniority::Mid],
            salary_min: Some(0),
            ..Preferences::default()
        };
        let v = validate(&p).unwrap();
        assert_eq!(v.titles, ["Software Engineer"]);
        assert_eq!(v.seniority, [Seniority::Mid]);
        assert_eq!(v.salary_min, None);
        assert!(v.include_unlisted_salary);

        let bad = |p: Preferences| validate(&p).is_err();
        assert!(bad(Preferences {
            titles: vec!["Engineer, Software".into()],
            ..Preferences::default()
        }));
        assert!(bad(Preferences {
            locations: vec!["Austin; Denver".into()],
            ..Preferences::default()
        }));
        assert!(bad(Preferences {
            salary_min: Some(50_000_000),
            ..Preferences::default()
        }));
    }

    #[test]
    fn missing_flag_defaults_to_including_unlisted() {
        let p: Preferences = serde_json::from_str(r#"{"titles":["Designer"]}"#).unwrap();
        assert!(p.include_unlisted_salary);
    }
}
