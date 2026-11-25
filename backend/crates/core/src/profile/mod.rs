//! The profile: the facts every resume, cover letter and autofill answer is
//! built from. One table per section; lists are one row per entry.
//!
//! The profile is filled by hand or from an uploaded resume (`parse`), which
//! produces a draft the user reviews before anything is saved.

pub mod apply;
pub mod clean;
pub mod crypto;
pub mod parse;
pub mod preferences;
pub mod resume;
pub mod resume_content;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Transaction};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::config::ProfileKey;

/// Entries allowed per list section.
const MAX_ITEMS: usize = 60;
const SHORT: usize = 200;
const LONG: usize = 5_000;
const BULLET: usize = 1_500;
const MAX_BULLETS: usize = 30;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct Personal {
    pub first_name: String,
    pub middle_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: String,
    /// "mobile", "home" or "work"; empty when not given.
    pub phone_type: String,
    pub street: String,
    pub city: String,
    /// State or province.
    pub region: String,
    pub postal_code: String,
    pub country: String,
    /// A one-line title, e.g. "QA engineer".
    pub headline: String,
    pub summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum LinkKind {
    Linkedin,
    Github,
    Portfolio,
    Other,
}

impl LinkKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Linkedin => "linkedin",
            Self::Github => "github",
            Self::Portfolio => "portfolio",
            Self::Other => "other",
        }
    }

    fn parse(s: &str) -> Self {
        match s {
            "linkedin" => Self::Linkedin,
            "github" => Self::Github,
            "portfolio" => Self::Portfolio,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Link {
    pub kind: LinkKind,
    pub url: String,
}

/// Dates are months, written `YYYY-MM` (what `<input type="month">` uses).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct Education {
    pub school: String,
    pub degree: String,
    pub field_of_study: String,
    pub minor: String,
    pub gpa: String,
    pub location: String,
    pub start: Option<String>,
    pub end: Option<String>,
    pub details: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct Experience {
    pub company: String,
    pub title: String,
    pub location: String,
    pub employment_type: String,
    pub start: Option<String>,
    /// None with a start date means "Present".
    pub end: Option<String>,
    pub bullets: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct Project {
    pub name: String,
    /// A one-line description ("Online banking platform").
    pub summary: String,
    pub role: String,
    pub url: String,
    pub start: Option<String>,
    pub end: Option<String>,
    pub bullets: Vec<String>,
    pub skills: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct Skill {
    pub name: String,
    /// e.g. "Languages", "Tools". Empty when the resume didn't group them.
    pub category: String,
}

/// Equal-employment answers. Stored encrypted; empty means not provided.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct Eeo {
    pub gender: String,
    pub race: String,
    pub hispanic_latino: String,
    pub veteran: String,
    pub disability: String,
    pub lgbtq: String,
    pub sexual_orientation: String,
    pub pronouns: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct Answers {
    /// Countries you want to work in.
    pub work_countries: Vec<String>,
    pub us_citizen: Option<bool>,
    pub authorized_us: Option<bool>,
    pub needs_sponsorship: Option<bool>,
    /// e.g. "Citizen", "Green card", "F-1 OPT".
    pub authorization_basis: String,
    pub eeo: Eeo,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct Profile {
    pub personal: Personal,
    pub links: Vec<Link>,
    pub educations: Vec<Education>,
    pub experiences: Vec<Experience>,
    pub projects: Vec<Project>,
    pub skills: Vec<Skill>,
    pub answers: Answers,
    /// Set when a resume filled the profile automatically (read-only; it's
    /// ignored when saving).
    pub filled_from: Option<apply::FilledFrom>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error("{0}")]
    Invalid(String),
    /// A problem with one field, so the editor can show it next to that
    /// field. `field` is a path like `experiences.2.end` (the index is the
    /// entry's place in the list that was sent).
    #[error("{message}")]
    Field { field: String, message: String },
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(
        "couldn't decrypt the saved equal-employment answers (was PROFILE_ENCRYPTION_KEY changed?)"
    )]
    Decrypt,
}

fn invalid(msg: impl Into<String>) -> ProfileError {
    ProfileError::Invalid(msg.into())
}

fn field_error(field: impl Into<String>, message: impl Into<String>) -> ProfileError {
    ProfileError::Field {
        field: field.into(),
        message: message.into(),
    }
}

// --- validation --------------------------------------------------------------

/// Trims, collapses inner whitespace runs in one-line fields, and checks length.
fn short(field: &str, value: &str) -> Result<String, ProfileError> {
    let v = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if v.chars().count() > SHORT {
        return Err(invalid(format!(
            "{field} is too long (at most {SHORT} characters)."
        )));
    }
    Ok(v)
}

fn long(field: &str, value: &str) -> Result<String, ProfileError> {
    let v = value.trim().to_owned();
    if v.chars().count() > LONG {
        return Err(invalid(format!(
            "{field} is too long (at most {LONG} characters)."
        )));
    }
    Ok(v)
}

fn bullets(field: &str, items: &[String]) -> Result<Vec<String>, ProfileError> {
    let out: Vec<String> = items
        .iter()
        .map(|b| b.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|b| !b.is_empty())
        .collect();
    if out.len() > MAX_BULLETS {
        return Err(invalid(format!(
            "{field} has more than {MAX_BULLETS} bullets."
        )));
    }
    if out.iter().any(|b| b.chars().count() > BULLET) {
        return Err(invalid(format!(
            "A bullet in {field} is too long (at most {BULLET} characters)."
        )));
    }
    Ok(out)
}

fn count(section: &str, n: usize) -> Result<(), ProfileError> {
    if n > MAX_ITEMS {
        return Err(invalid(format!(
            "{section} has more than {MAX_ITEMS} entries."
        )));
    }
    Ok(())
}

/// `YYYY-MM` to the first day of that month.
pub fn parse_month(s: &str) -> Option<NaiveDate> {
    let (y, m) = s.trim().split_once('-')?;
    let (y, m) = (y.parse::<i32>().ok()?, m.parse::<u32>().ok()?);
    if !(1900..=2100).contains(&y) {
        return None;
    }
    NaiveDate::from_ymd_opt(y, m, 1)
}

fn month(d: Option<NaiveDate>) -> Option<String> {
    d.map(|d| d.format("%Y-%m").to_string())
}

/// Checks an entry's dates. `path` is the entry's field path
/// (`experiences.0`). A start date in the future is only allowed where it
/// makes sense (a degree you're about to start).
fn dates(
    path: &str,
    start: &Option<String>,
    end: &Option<String>,
    future_start: bool,
) -> Result<(Option<NaiveDate>, Option<NaiveDate>), ProfileError> {
    let parse = |which: &str, s: &Option<String>| -> Result<Option<NaiveDate>, ProfileError> {
        match s.as_deref().map(str::trim) {
            None | Some("") => Ok(None),
            Some(v) => parse_month(v).map(Some).ok_or_else(|| {
                field_error(format!("{path}.{which}"), "Pick a month, like 2024-05.")
            }),
        }
    };
    let (s, e) = (parse("start", start)?, parse("end", end)?);
    if let (Some(s), Some(e)) = (s, e)
        && e < s
    {
        return Err(field_error(
            format!("{path}.end"),
            "The end date is before the start date.",
        ));
    }
    if !future_start
        && let Some(s) = s
        && s > chrono::Utc::now().date_naive()
    {
        return Err(field_error(
            format!("{path}.start"),
            "The start date is in the future.",
        ));
    }
    Ok((s, e))
}

/// Newest first: current entries (a start and no end), then by end date,
/// then by start date. Entries without dates go last, in the order given.
fn newest_first<T>(items: &mut [T], key: impl Fn(&T) -> (Option<NaiveDate>, Option<NaiveDate>)) {
    items.sort_by(|a, b| {
        let rank = |t: &T| {
            let (s, e) = key(t);
            match (s, e) {
                (Some(_), None) => (2, NaiveDate::MAX, s),
                (_, Some(e)) => (1, e, s),
                (None, None) => (0, NaiveDate::MIN, None),
            }
        };
        rank(b).cmp(&rank(a))
    });
}

static EMAIL_FORMAT: std::sync::LazyLock<regex::Regex> =
    std::sync::LazyLock::new(|| regex::Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]{2,}$").unwrap());
static GPA_FORMAT: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(r"^\d{1,2}(\.\d{1,3})?(\s*/\s*\d{1,3}(\.\d{1,3})?)?$").unwrap()
});

fn check_phone(phone: &str) -> Result<(), ProfileError> {
    let digits = phone.chars().filter(char::is_ascii_digit).count();
    if !phone.is_empty()
        && (!(7..=15).contains(&digits)
            || !phone
                .chars()
                .all(|c| c.is_ascii_digit() || " +-().x".contains(c)))
    {
        return Err(field_error(
            "personal.phone",
            "Enter a phone number, like (555) 555-0100.",
        ));
    }
    Ok(())
}

/// Adds `https://` when the scheme is missing; only http(s) links are kept.
pub fn normalize_url(raw: &str) -> Result<String, ProfileError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(String::new());
    }
    let with_scheme = if raw.contains("://") {
        raw.to_owned()
    } else {
        format!("https://{raw}")
    };
    match url::Url::parse(&with_scheme) {
        Ok(u) if matches!(u.scheme(), "http" | "https") && u.host_str().is_some() => {
            if with_scheme.len() > 500 {
                return Err(invalid("That link is too long."));
            }
            Ok(with_scheme)
        }
        _ => Err(invalid(format!("\"{raw}\" isn't a web link."))),
    }
}

pub fn link_kind(url: &str) -> LinkKind {
    let host = url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_lowercase))
        .unwrap_or_default();
    if host.ends_with("linkedin.com") {
        LinkKind::Linkedin
    } else if host.ends_with("github.com") {
        LinkKind::Github
    } else {
        LinkKind::Portfolio
    }
}

// --- loading -----------------------------------------------------------------

/// Saved personal details and links.
pub async fn load_personal(pool: &PgPool, user_id: Uuid) -> sqlx::Result<(Personal, Vec<Link>)> {
    type PersonalRow = (
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
    );
    let personal = sqlx::query_as::<_, PersonalRow>(
        "SELECT first_name, middle_name, last_name, email, phone, phone_type, street, city, region,
                postal_code, country, headline, summary
         FROM profiles WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .map(|r| Personal {
        first_name: r.0,
        middle_name: r.1,
        last_name: r.2,
        email: r.3,
        phone: r.4,
        phone_type: r.5,
        street: r.6,
        city: r.7,
        region: r.8,
        postal_code: r.9,
        country: r.10,
        headline: r.11,
        summary: r.12,
    })
    .unwrap_or_default();

    let links = sqlx::query_as::<_, (String, String)>(
        "SELECT kind, url FROM profile_links WHERE user_id = $1 ORDER BY position",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(kind, url)| Link {
        kind: LinkKind::parse(&kind),
        url,
    })
    .collect();
    Ok((personal, links))
}

pub async fn load(pool: &PgPool, key: &ProfileKey, user_id: Uuid) -> Result<Profile, ProfileError> {
    let (personal, links) = load_personal(pool, user_id).await?;

    type EduRow = (
        String,
        String,
        String,
        String,
        String,
        String,
        Option<NaiveDate>,
        Option<NaiveDate>,
        String,
    );
    let educations = sqlx::query_as::<_, EduRow>(
        "SELECT school, degree, field_of_study, minor, gpa, location, start_date, end_date, details
         FROM educations WHERE user_id = $1 ORDER BY position",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| Education {
        school: r.0,
        degree: r.1,
        field_of_study: r.2,
        minor: r.3,
        gpa: r.4,
        location: r.5,
        start: month(r.6),
        end: month(r.7),
        details: r.8,
    })
    .collect();

    type ExpRow = (
        String,
        String,
        String,
        String,
        Option<NaiveDate>,
        Option<NaiveDate>,
        Vec<String>,
    );
    let experiences = sqlx::query_as::<_, ExpRow>(
        "SELECT company, title, location, employment_type, start_date, end_date, bullets
         FROM experiences WHERE user_id = $1 ORDER BY position",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| Experience {
        company: r.0,
        title: r.1,
        location: r.2,
        employment_type: r.3,
        start: month(r.4),
        end: month(r.5),
        bullets: r.6,
    })
    .collect();

    type ProjRow = (
        String,
        String,
        String,
        String,
        Option<NaiveDate>,
        Option<NaiveDate>,
        Vec<String>,
        Vec<String>,
    );
    let projects = sqlx::query_as::<_, ProjRow>(
        "SELECT name, summary, role, url, start_date, end_date, bullets, skills
         FROM projects WHERE user_id = $1 ORDER BY position",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| Project {
        name: r.0,
        summary: r.1,
        role: r.2,
        url: r.3,
        start: month(r.4),
        end: month(r.5),
        bullets: r.6,
        skills: r.7,
    })
    .collect();

    let skills = sqlx::query_as::<_, (String, String)>(
        "SELECT name, category FROM skills WHERE user_id = $1 ORDER BY position",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(name, category)| Skill { name, category })
    .collect();

    type AnsRow = (
        Vec<String>,
        Option<bool>,
        Option<bool>,
        Option<bool>,
        String,
        Option<Vec<u8>>,
        Option<Vec<u8>>,
    );
    let answers = match sqlx::query_as::<_, AnsRow>(
        "SELECT work_countries, us_citizen, authorized_us, needs_sponsorship, authorization_basis,
                eeo_nonce, eeo_ciphertext
         FROM application_answers WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    {
        None => Answers::default(),
        Some(r) => {
            let eeo = match (r.5, r.6) {
                (Some(nonce), Some(ct)) => {
                    let plain = crypto::decrypt(key, &nonce, &ct).ok_or(ProfileError::Decrypt)?;
                    serde_json::from_slice(&plain).map_err(|_| ProfileError::Decrypt)?
                }
                _ => Eeo::default(),
            };
            Answers {
                work_countries: r.0,
                us_citizen: r.1,
                authorized_us: r.2,
                needs_sponsorship: r.3,
                authorization_basis: r.4,
                eeo,
            }
        }
    };

    Ok(Profile {
        personal,
        links,
        educations,
        experiences,
        projects,
        skills,
        answers,
        filled_from: apply::filled_from(pool, user_id).await?,
    })
}

// --- saving ------------------------------------------------------------------
//
// Each section saves as a whole: its rows are replaced in one transaction, in
// the order given. Lists are short, so this keeps ordering and editing simple.

async fn clear(tx: &mut Transaction<'_, Postgres>, table: &str, user_id: Uuid) -> sqlx::Result<()> {
    // `table` is one of our own constants.
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DELETE FROM {table} WHERE user_id = $1"
    )))
    .bind(user_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn save_personal(
    pool: &PgPool,
    user_id: Uuid,
    p: &Personal,
) -> Result<Personal, ProfileError> {
    let one = |name: &str, label: &str, v: &str| -> Result<String, ProfileError> {
        short(label, v).map_err(|e| field_error(format!("personal.{name}"), e.to_string()))
    };
    let p = Personal {
        first_name: one("first_name", "First name", &p.first_name)?,
        middle_name: one("middle_name", "Middle name", &p.middle_name)?,
        last_name: one("last_name", "Last name", &p.last_name)?,
        email: one("email", "Email", &p.email)?,
        phone: one("phone", "Phone", &p.phone)?,
        phone_type: one("phone_type", "Phone type", &p.phone_type)?.to_lowercase(),
        street: one("street", "Street", &p.street)?,
        city: one("city", "City", &p.city)?,
        region: one("region", "State", &p.region)?,
        postal_code: one("postal_code", "ZIP code", &p.postal_code)?,
        country: one("country", "Country", &p.country)?,
        headline: one("headline", "Headline", &p.headline)?,
        summary: long("Summary", &p.summary)
            .map_err(|e| field_error("personal.summary", e.to_string()))?,
    };
    if p.first_name.is_empty() {
        return Err(field_error("personal.first_name", "Enter your first name."));
    }
    if p.last_name.is_empty() {
        return Err(field_error("personal.last_name", "Enter your last name."));
    }
    if p.email.is_empty() {
        return Err(field_error("personal.email", "Enter your email address."));
    }
    if !EMAIL_FORMAT.is_match(&p.email) {
        return Err(field_error(
            "personal.email",
            "Enter an email address like name@example.com.",
        ));
    }
    check_phone(&p.phone)?;
    if !matches!(p.phone_type.as_str(), "" | "mobile" | "home" | "work") {
        return Err(field_error(
            "personal.phone_type",
            "Phone type is mobile, home or work.",
        ));
    }
    sqlx::query(
        "INSERT INTO profiles (user_id, first_name, middle_name, last_name, email, phone, phone_type,
                               street, city, region, postal_code, country, headline, summary,
                               updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, now())
         ON CONFLICT (user_id) DO UPDATE SET
             first_name = EXCLUDED.first_name, middle_name = EXCLUDED.middle_name,
             last_name = EXCLUDED.last_name, email = EXCLUDED.email, phone = EXCLUDED.phone,
             phone_type = EXCLUDED.phone_type, street = EXCLUDED.street, city = EXCLUDED.city,
             region = EXCLUDED.region, postal_code = EXCLUDED.postal_code,
             country = EXCLUDED.country, headline = EXCLUDED.headline, summary = EXCLUDED.summary,
             updated_at = now()",
    )
    .bind(user_id)
    .bind(&p.first_name)
    .bind(&p.middle_name)
    .bind(&p.last_name)
    .bind(&p.email)
    .bind(&p.phone)
    .bind(&p.phone_type)
    .bind(&p.street)
    .bind(&p.city)
    .bind(&p.region)
    .bind(&p.postal_code)
    .bind(&p.country)
    .bind(&p.headline)
    .bind(&p.summary)
    .execute(pool)
    .await?;
    Ok(p)
}

pub async fn save_links(
    pool: &PgPool,
    user_id: Uuid,
    links: &[Link],
) -> Result<Vec<Link>, ProfileError> {
    count("Links", links.len())?;
    let mut clean = Vec::new();
    for l in links {
        let url = normalize_url(&l.url)?;
        if !url.is_empty() {
            clean.push(Link { kind: l.kind, url });
        }
    }
    let mut tx = pool.begin().await?;
    clear(&mut tx, "profile_links", user_id).await?;
    for (i, l) in clean.iter().enumerate() {
        sqlx::query(
            "INSERT INTO profile_links (user_id, position, kind, url) VALUES ($1, $2, $3, $4)",
        )
        .bind(user_id)
        .bind(i as i32)
        .bind(l.kind.as_str())
        .bind(&l.url)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(clean)
}

/// Checks a one-line field of a list entry, reporting errors at `path.name`.
fn entry_field(path: &str, name: &str, label: &str, v: &str) -> Result<String, ProfileError> {
    short(label, v).map_err(|e| field_error(format!("{path}.{name}"), e.to_string()))
}

/// Saved newest first, whatever order they were sent in.
pub async fn save_educations(
    pool: &PgPool,
    user_id: Uuid,
    items: &[Education],
) -> Result<Vec<Education>, ProfileError> {
    count("Education", items.len())?;
    let mut clean = Vec::new();
    for (i, e) in items.iter().enumerate() {
        let path = format!("educations.{i}");
        let f = |name: &str, label: &str, v: &str| entry_field(&path, name, label, v);
        let school = f("school", "School", &e.school)?;
        if school.is_empty() {
            return Err(field_error(
                format!("{path}.school"),
                "Enter the school's name.",
            ));
        }
        let gpa = f("gpa", "GPA", &e.gpa)?;
        if !gpa.is_empty() && !GPA_FORMAT.is_match(&gpa) {
            return Err(field_error(
                format!("{path}.gpa"),
                "Write the GPA as a number, like 3.5 or 3.50/4.0.",
            ));
        }
        let (start, end) = dates(&path, &e.start, &e.end, true)?;
        clean.push((
            Education {
                school,
                degree: f("degree", "Degree", &e.degree)?,
                field_of_study: f("field_of_study", "Field of study", &e.field_of_study)?,
                minor: f("minor", "Minor", &e.minor)?,
                gpa,
                location: f("location", "Location", &e.location)?,
                start: month(start),
                end: month(end),
                details: long("Details", &e.details)
                    .map_err(|err| field_error(format!("{path}.details"), err.to_string()))?,
            },
            start,
            end,
        ));
    }
    newest_first(&mut clean, |c| (c.1, c.2));
    let mut tx = pool.begin().await?;
    clear(&mut tx, "educations", user_id).await?;
    for (i, (e, start, end)) in clean.iter().enumerate() {
        sqlx::query(
            "INSERT INTO educations (user_id, position, school, degree, field_of_study, minor, gpa,
                                     location, start_date, end_date, details)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(user_id)
        .bind(i as i32)
        .bind(&e.school)
        .bind(&e.degree)
        .bind(&e.field_of_study)
        .bind(&e.minor)
        .bind(&e.gpa)
        .bind(&e.location)
        .bind(start)
        .bind(end)
        .bind(&e.details)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(clean.into_iter().map(|c| c.0).collect())
}

/// Saved newest first, whatever order they were sent in.
pub async fn save_experiences(
    pool: &PgPool,
    user_id: Uuid,
    items: &[Experience],
) -> Result<Vec<Experience>, ProfileError> {
    count("Experience", items.len())?;
    let mut clean = Vec::new();
    for (i, e) in items.iter().enumerate() {
        let path = format!("experiences.{i}");
        let f = |name: &str, label: &str, v: &str| entry_field(&path, name, label, v);
        let company = f("company", "Company", &e.company)?;
        let title = f("title", "Title", &e.title)?;
        if company.is_empty() {
            return Err(field_error(
                format!("{path}.company"),
                "Enter the company's name.",
            ));
        }
        if title.is_empty() {
            return Err(field_error(
                format!("{path}.title"),
                "Enter your job title.",
            ));
        }
        let (start, end) = dates(&path, &e.start, &e.end, false)?;
        clean.push((
            Experience {
                bullets: bullets(&company, &e.bullets)
                    .map_err(|err| field_error(format!("{path}.bullets"), err.to_string()))?,
                company,
                title,
                location: f("location", "Location", &e.location)?,
                employment_type: f("employment_type", "Job type", &e.employment_type)?,
                start: month(start),
                end: month(end),
            },
            start,
            end,
        ));
    }
    newest_first(&mut clean, |c| (c.1, c.2));
    let mut tx = pool.begin().await?;
    clear(&mut tx, "experiences", user_id).await?;
    for (i, (e, start, end)) in clean.iter().enumerate() {
        sqlx::query(
            "INSERT INTO experiences (user_id, position, company, title, location, employment_type,
                                      start_date, end_date, bullets)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(user_id)
        .bind(i as i32)
        .bind(&e.company)
        .bind(&e.title)
        .bind(&e.location)
        .bind(&e.employment_type)
        .bind(start)
        .bind(end)
        .bind(&e.bullets)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(clean.into_iter().map(|c| c.0).collect())
}

/// Saved in the order given: projects are ordered by hand.
pub async fn save_projects(
    pool: &PgPool,
    user_id: Uuid,
    items: &[Project],
) -> Result<Vec<Project>, ProfileError> {
    count("Projects", items.len())?;
    let mut clean = Vec::new();
    for (i, p) in items.iter().enumerate() {
        let path = format!("projects.{i}");
        let f = |name: &str, label: &str, v: &str| entry_field(&path, name, label, v);
        let name = f("name", "Project name", &p.name)?;
        if name.is_empty() {
            return Err(field_error(
                format!("{path}.name"),
                "Enter the project's name.",
            ));
        }
        let (start, end) = dates(&path, &p.start, &p.end, false)?;
        clean.push((
            Project {
                bullets: bullets(&name, &p.bullets)
                    .map_err(|err| field_error(format!("{path}.bullets"), err.to_string()))?,
                skills: p
                    .skills
                    .iter()
                    .map(|s| f("skills", "Skill", s))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .collect(),
                summary: f("summary", "Summary", &p.summary)?,
                role: f("role", "Role", &p.role)?,
                url: normalize_url(&p.url)
                    .map_err(|err| field_error(format!("{path}.url"), err.to_string()))?,
                start: month(start),
                end: month(end),
                name,
            },
            start,
            end,
        ));
    }
    let mut tx = pool.begin().await?;
    clear(&mut tx, "projects", user_id).await?;
    for (i, (p, start, end)) in clean.iter().enumerate() {
        sqlx::query(
            "INSERT INTO projects (user_id, position, name, summary, role, url, start_date, end_date,
                                   bullets, skills)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(user_id)
        .bind(i as i32)
        .bind(&p.name)
        .bind(&p.summary)
        .bind(&p.role)
        .bind(&p.url)
        .bind(start)
        .bind(end)
        .bind(&p.bullets)
        .bind(&p.skills)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(clean.into_iter().map(|c| c.0).collect())
}

pub async fn save_skills(
    pool: &PgPool,
    user_id: Uuid,
    items: &[Skill],
) -> Result<Vec<Skill>, ProfileError> {
    if items.len() > 300 {
        return Err(invalid("At most 300 skills."));
    }
    // Case-insensitive duplicates keep the first spelling.
    let mut seen = std::collections::HashSet::new();
    let mut clean = Vec::new();
    for s in items {
        let name = short("Skill", &s.name)?;
        if !name.is_empty() && seen.insert(name.to_lowercase()) {
            clean.push(Skill {
                name,
                category: short("Skill category", &s.category)?,
            });
        }
    }
    let mut tx = pool.begin().await?;
    clear(&mut tx, "skills", user_id).await?;
    for (i, s) in clean.iter().enumerate() {
        sqlx::query(
            "INSERT INTO skills (user_id, position, name, category) VALUES ($1, $2, $3, $4)",
        )
        .bind(user_id)
        .bind(i as i32)
        .bind(&s.name)
        .bind(&s.category)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(clean)
}

pub async fn save_answers(
    pool: &PgPool,
    key: &ProfileKey,
    user_id: Uuid,
    a: &Answers,
) -> Result<Answers, ProfileError> {
    let e = &a.eeo;
    let eeo = Eeo {
        gender: short("Gender", &e.gender)?,
        race: short("Race", &e.race)?,
        hispanic_latino: short("Hispanic or Latino", &e.hispanic_latino)?,
        veteran: short("Veteran status", &e.veteran)?,
        disability: short("Disability", &e.disability)?,
        lgbtq: short("LGBTQ+", &e.lgbtq)?,
        sexual_orientation: short("Sexual orientation", &e.sexual_orientation)?,
        pronouns: short("Pronouns", &e.pronouns)?,
    };
    let a = Answers {
        work_countries: a
            .work_countries
            .iter()
            .map(|c| short("Country", c))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter(|c| !c.is_empty())
            .take(50)
            .collect(),
        us_citizen: a.us_citizen,
        authorized_us: a.authorized_us,
        needs_sponsorship: a.needs_sponsorship,
        authorization_basis: short("Authorization basis", &a.authorization_basis)?,
        eeo,
    };
    let (nonce, ct) = if a.eeo == Eeo::default() {
        (None, None)
    } else {
        let plain = serde_json::to_vec(&a.eeo).expect("plain struct serializes");
        let (n, c) = crypto::encrypt(key, &plain);
        (Some(n), Some(c))
    };
    sqlx::query(
        "INSERT INTO application_answers (user_id, work_countries, us_citizen, authorized_us,
                                          needs_sponsorship, authorization_basis, eeo_nonce,
                                          eeo_ciphertext, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now())
         ON CONFLICT (user_id) DO UPDATE SET
             work_countries = EXCLUDED.work_countries, us_citizen = EXCLUDED.us_citizen,
             authorized_us = EXCLUDED.authorized_us, needs_sponsorship = EXCLUDED.needs_sponsorship,
             authorization_basis = EXCLUDED.authorization_basis, eeo_nonce = EXCLUDED.eeo_nonce,
             eeo_ciphertext = EXCLUDED.eeo_ciphertext, updated_at = now()",
    )
    .bind(user_id)
    .bind(&a.work_countries)
    .bind(a.us_citizen)
    .bind(a.authorized_us)
    .bind(a.needs_sponsorship)
    .bind(&a.authorization_basis)
    .bind(nonce)
    .bind(ct)
    .execute(pool)
    .await?;
    Ok(a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn months() {
        assert_eq!(parse_month("2024-05"), NaiveDate::from_ymd_opt(2024, 5, 1));
        assert_eq!(parse_month("2024-13"), None);
        assert_eq!(parse_month("May 2024"), None);
        assert!(dates("x", &Some("2024-05".into()), &Some("2023-01".into()), false).is_err());
        assert!(dates("x", &Some("2024-05".into()), &None, false).is_ok());
        assert!(
            dates("x", &Some("".into()), &None, false)
                .unwrap()
                .0
                .is_none()
        );
        match dates("experiences.1", &Some("2099-01".into()), &None, false) {
            Err(ProfileError::Field { field, .. }) => assert_eq!(field, "experiences.1.start"),
            other => panic!("{other:?}"),
        }
        assert!(dates("educations.0", &Some("2099-01".into()), &None, true).is_ok());
    }

    #[test]
    fn newest_first_order() {
        let d = parse_month;
        let mut items = vec![
            ("none", None, None),
            ("old", d("2019-01"), d("2020-01")),
            ("current", d("2023-01"), None),
            ("recent", d("2021-01"), d("2024-06")),
        ];
        newest_first(&mut items, |t| (t.1, t.2));
        let names: Vec<&str> = items.iter().map(|t| t.0).collect();
        assert_eq!(names, ["current", "recent", "old", "none"]);
    }

    #[test]
    fn formats() {
        assert!(EMAIL_FORMAT.is_match("a.b@example.com"));
        assert!(!EMAIL_FORMAT.is_match("a@b"));
        for ok in ["3.5", "3.50/4.0", "3.8 / 4", "92/100"] {
            assert!(GPA_FORMAT.is_match(ok), "{ok}");
        }
        assert!(!GPA_FORMAT.is_match("great"));
        assert!(check_phone("(555) 555-0100").is_ok());
        assert!(check_phone("+1 555.555.0100").is_ok());
        assert!(check_phone("call me").is_err());
    }

    #[test]
    fn urls() {
        assert_eq!(
            normalize_url("linkedin.com/in/someone").unwrap(),
            "https://linkedin.com/in/someone"
        );
        assert_eq!(normalize_url("  ").unwrap(), "");
        assert!(normalize_url("javascript:alert(1)").is_err());
        assert!(normalize_url("ftp://x.com").is_err());
        assert_eq!(
            link_kind("https://www.linkedin.com/in/x"),
            LinkKind::Linkedin
        );
        assert_eq!(link_kind("https://github.com/x"), LinkKind::Github);
        assert_eq!(link_kind("https://me.dev"), LinkKind::Portfolio);
    }

    #[test]
    fn bullet_cleanup() {
        let b = bullets("Job", &["  Did   a thing ".into(), "".into(), " ".into()]).unwrap();
        assert_eq!(b, vec!["Did a thing"]);
    }
}
