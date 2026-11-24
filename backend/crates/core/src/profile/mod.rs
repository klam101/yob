//! The profile: the facts every resume, cover letter and autofill answer is
//! built from. One table per section; lists are one row per entry.
//!
//! The profile is filled by hand or from an uploaded resume (`parse`), which
//! produces a draft the user reviews before anything is saved.

pub mod crypto;
pub mod parse;
pub mod resume;

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
    pub last_name: String,
    pub email: String,
    pub phone: String,
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
}

#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error("{0}")]
    Invalid(String),
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

fn dates(
    what: &str,
    start: &Option<String>,
    end: &Option<String>,
) -> Result<(Option<NaiveDate>, Option<NaiveDate>), ProfileError> {
    let parse = |s: &Option<String>| -> Result<Option<NaiveDate>, ProfileError> {
        match s.as_deref().map(str::trim) {
            None | Some("") => Ok(None),
            Some(v) => parse_month(v)
                .map(Some)
                .ok_or_else(|| invalid(format!("{what}: dates must look like 2024-05."))),
        }
    };
    let (s, e) = (parse(start)?, parse(end)?);
    if let (Some(s), Some(e)) = (s, e)
        && e < s
    {
        return Err(invalid(format!(
            "{what}: the end date is before the start date."
        )));
    }
    Ok((s, e))
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

pub async fn load(pool: &PgPool, key: &ProfileKey, user_id: Uuid) -> Result<Profile, ProfileError> {
    let personal = sqlx::query_as::<_, (String, String, String, String, String, String, String, String, String, String, String)>(
        "SELECT first_name, last_name, email, phone, street, city, region, postal_code, country, headline, summary
         FROM profiles WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .map(|r| Personal {
        first_name: r.0,
        last_name: r.1,
        email: r.2,
        phone: r.3,
        street: r.4,
        city: r.5,
        region: r.6,
        postal_code: r.7,
        country: r.8,
        headline: r.9,
        summary: r.10,
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

    type EduRow = (
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
        "SELECT school, degree, field_of_study, gpa, location, start_date, end_date, details
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
        gpa: r.3,
        location: r.4,
        start: month(r.5),
        end: month(r.6),
        details: r.7,
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
        Option<NaiveDate>,
        Option<NaiveDate>,
        Vec<String>,
        Vec<String>,
    );
    let projects = sqlx::query_as::<_, ProjRow>(
        "SELECT name, role, url, start_date, end_date, bullets, skills
         FROM projects WHERE user_id = $1 ORDER BY position",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| Project {
        name: r.0,
        role: r.1,
        url: r.2,
        start: month(r.3),
        end: month(r.4),
        bullets: r.5,
        skills: r.6,
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
    let email = short("Email", &p.email)?;
    if !email.is_empty() && !email.contains('@') {
        return Err(invalid("Enter a valid email address."));
    }
    let p = Personal {
        first_name: short("First name", &p.first_name)?,
        last_name: short("Last name", &p.last_name)?,
        email,
        phone: short("Phone", &p.phone)?,
        street: short("Street", &p.street)?,
        city: short("City", &p.city)?,
        region: short("State", &p.region)?,
        postal_code: short("ZIP code", &p.postal_code)?,
        country: short("Country", &p.country)?,
        headline: short("Headline", &p.headline)?,
        summary: long("Summary", &p.summary)?,
    };
    sqlx::query(
        "INSERT INTO profiles (user_id, first_name, last_name, email, phone, street, city, region,
                               postal_code, country, headline, summary, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, now())
         ON CONFLICT (user_id) DO UPDATE SET
             first_name = EXCLUDED.first_name, last_name = EXCLUDED.last_name,
             email = EXCLUDED.email, phone = EXCLUDED.phone, street = EXCLUDED.street,
             city = EXCLUDED.city, region = EXCLUDED.region, postal_code = EXCLUDED.postal_code,
             country = EXCLUDED.country, headline = EXCLUDED.headline, summary = EXCLUDED.summary,
             updated_at = now()",
    )
    .bind(user_id)
    .bind(&p.first_name)
    .bind(&p.last_name)
    .bind(&p.email)
    .bind(&p.phone)
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

pub async fn save_educations(
    pool: &PgPool,
    user_id: Uuid,
    items: &[Education],
) -> Result<Vec<Education>, ProfileError> {
    count("Education", items.len())?;
    let mut tx = pool.begin().await?;
    clear(&mut tx, "educations", user_id).await?;
    let mut out = Vec::new();
    for (i, e) in items.iter().enumerate() {
        let school = short("School", &e.school)?;
        if school.is_empty() {
            return Err(invalid("Each education entry needs a school."));
        }
        let (start, end) = dates(&school, &e.start, &e.end)?;
        let e = Education {
            school,
            degree: short("Degree", &e.degree)?,
            field_of_study: short("Field of study", &e.field_of_study)?,
            gpa: short("GPA", &e.gpa)?,
            location: short("Location", &e.location)?,
            start: month(start),
            end: month(end),
            details: long("Details", &e.details)?,
        };
        sqlx::query(
            "INSERT INTO educations (user_id, position, school, degree, field_of_study, gpa, location,
                                     start_date, end_date, details)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(user_id)
        .bind(i as i32)
        .bind(&e.school)
        .bind(&e.degree)
        .bind(&e.field_of_study)
        .bind(&e.gpa)
        .bind(&e.location)
        .bind(start)
        .bind(end)
        .bind(&e.details)
        .execute(&mut *tx)
        .await?;
        out.push(e);
    }
    tx.commit().await?;
    Ok(out)
}

pub async fn save_experiences(
    pool: &PgPool,
    user_id: Uuid,
    items: &[Experience],
) -> Result<Vec<Experience>, ProfileError> {
    count("Experience", items.len())?;
    let mut tx = pool.begin().await?;
    clear(&mut tx, "experiences", user_id).await?;
    let mut out = Vec::new();
    for (i, e) in items.iter().enumerate() {
        let company = short("Company", &e.company)?;
        let title = short("Title", &e.title)?;
        if company.is_empty() && title.is_empty() {
            return Err(invalid("Each experience needs a company or a title."));
        }
        let what = if company.is_empty() { &title } else { &company };
        let (start, end) = dates(what, &e.start, &e.end)?;
        let e = Experience {
            bullets: bullets(what, &e.bullets)?,
            company,
            title,
            location: short("Location", &e.location)?,
            employment_type: short("Employment type", &e.employment_type)?,
            start: month(start),
            end: month(end),
        };
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
        out.push(e);
    }
    tx.commit().await?;
    Ok(out)
}

pub async fn save_projects(
    pool: &PgPool,
    user_id: Uuid,
    items: &[Project],
) -> Result<Vec<Project>, ProfileError> {
    count("Projects", items.len())?;
    let mut tx = pool.begin().await?;
    clear(&mut tx, "projects", user_id).await?;
    let mut out = Vec::new();
    for (i, p) in items.iter().enumerate() {
        let name = short("Project name", &p.name)?;
        if name.is_empty() {
            return Err(invalid("Each project needs a name."));
        }
        let (start, end) = dates(&name, &p.start, &p.end)?;
        let p = Project {
            bullets: bullets(&name, &p.bullets)?,
            skills: p
                .skills
                .iter()
                .map(|s| short("Skill", s))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect(),
            role: short("Role", &p.role)?,
            url: normalize_url(&p.url)?,
            start: month(start),
            end: month(end),
            name,
        };
        sqlx::query(
            "INSERT INTO projects (user_id, position, name, role, url, start_date, end_date, bullets, skills)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(user_id)
        .bind(i as i32)
        .bind(&p.name)
        .bind(&p.role)
        .bind(&p.url)
        .bind(start)
        .bind(end)
        .bind(&p.bullets)
        .bind(&p.skills)
        .execute(&mut *tx)
        .await?;
        out.push(p);
    }
    tx.commit().await?;
    Ok(out)
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
        assert!(dates("x", &Some("2024-05".into()), &Some("2023-01".into())).is_err());
        assert!(dates("x", &Some("2024-05".into()), &None).is_ok());
        assert!(dates("x", &Some("".into()), &None).unwrap().0.is_none());
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
