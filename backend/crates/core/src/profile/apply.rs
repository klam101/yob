//! Updating the profile from a resume's content.
//!
//! `apply_sections` copies the sections the user picks in the "Update
//! profile" dialog. `fill_once` uses it to fill an empty profile from the
//! first resume automatically, so every part of the app that needs a
//! profile (autofill, tailoring, matching) works without another step. That
//! happens only once; after that, a new resume asks with the dialog.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use super::{
    Personal, ProfileError,
    resume::{self, Files},
    resume_content::{self, ResumeContent},
};

/// Which resume filled the profile automatically, for the "check what we
/// read" banner.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct FilledFrom {
    /// None when that resume has since been deleted.
    pub resume_id: Option<String>,
    pub name: String,
    pub filled_at: DateTime<Utc>,
    /// Sections left out because what was read didn't pass validation.
    pub skipped: Vec<String>,
    /// The parser's changes on that resume still waiting to be checked.
    pub open_changes: i64,
    /// The user dismissed the banner.
    pub reviewed: bool,
}

/// Nothing in the profile yet: no name and no entries in any list.
pub async fn is_empty(pool: &PgPool, user_id: Uuid) -> sqlx::Result<bool> {
    let (empty,): (bool,) = sqlx::query_as(
        "SELECT NOT EXISTS (SELECT 1 FROM profiles WHERE user_id = $1 AND (first_name <> '' OR last_name <> ''))
            AND NOT EXISTS (SELECT 1 FROM experiences WHERE user_id = $1)
            AND NOT EXISTS (SELECT 1 FROM educations WHERE user_id = $1)
            AND NOT EXISTS (SELECT 1 FROM projects WHERE user_id = $1)
            AND NOT EXISTS (SELECT 1 FROM skills WHERE user_id = $1)",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(empty)
}

async fn never_filled(pool: &PgPool, user_id: Uuid) -> sqlx::Result<bool> {
    let (never,): (bool,) = sqlx::query_as(
        "SELECT NOT EXISTS (SELECT 1 FROM profiles WHERE user_id = $1 AND filled_at IS NOT NULL)",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(never)
}

/// A part of the profile a resume can update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ProfileSection {
    Personal,
    Education,
    Experience,
    Projects,
    Skills,
}

impl ProfileSection {
    pub const ALL: [ProfileSection; 5] = [
        Self::Personal,
        Self::Education,
        Self::Experience,
        Self::Projects,
        Self::Skills,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Personal => "Personal details",
            Self::Education => "Education",
            Self::Experience => "Work experience",
            Self::Projects => "Projects",
            Self::Skills => "Skills",
        }
    }
}

/// A section that wasn't updated, and why.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SectionFailure {
    pub section: ProfileSection,
    pub message: String,
    /// The field on the resume to fix (`header.email`), when known.
    pub field: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Applied {
    pub updated: Vec<ProfileSection>,
    pub failed: Vec<SectionFailure>,
}

/// A profile field path as the matching resume field, so "Show me" can
/// point at it: `personal.email` -> `header.email`. Entry indices differ
/// after the profile's newest-first sort, so list paths point at the list.
fn resume_field(section: ProfileSection, profile_field: &str) -> Option<String> {
    let f = profile_field
        .strip_prefix("personal.")
        .unwrap_or(profile_field);
    Some(match section {
        ProfileSection::Personal => match f {
            "first_name" | "middle_name" | "last_name" => "header.name".into(),
            "email" => "header.email".into(),
            "phone" => "header.phone".into(),
            "city" | "region" => "header.location".into(),
            "headline" => "header.subtitle".into(),
            "summary" => "summary".into(),
            _ => return None,
        },
        ProfileSection::Education => "educations".into(),
        ProfileSection::Experience => "experiences".into(),
        ProfileSection::Projects => "projects".into(),
        ProfileSection::Skills => "skills".into(),
    })
}

/// Updates the chosen profile sections from a resume's content.
///
/// - Personal details merge: what the resume has replaces the saved value,
///   and what it doesn't have (street, ZIP, phone type…) is kept. Links are
///   added by address; saved ones stay.
/// - Every other section is replaced, with the usual validation and order.
///
/// A section that doesn't pass validation is left as it was and reported;
/// the others are still saved. Database errors stop and are returned.
pub async fn apply_sections(
    pool: &PgPool,
    user_id: Uuid,
    content: &ResumeContent,
    sections: &[ProfileSection],
) -> Result<Applied, ProfileError> {
    let p = resume_content::to_profile(content);
    let mut out = Applied::default();
    for &section in ProfileSection::ALL.iter().filter(|s| sections.contains(s)) {
        let result: Result<(), ProfileError> = match section {
            ProfileSection::Personal => {
                let (saved, saved_links) = super::load_personal(pool, user_id).await?;
                let merged = merge_personal(&saved, &p.personal);
                match super::save_personal(pool, user_id, &merged).await {
                    Ok(_) => {
                        let mut links = saved_links;
                        for l in &p.links {
                            let key = |u: &str| {
                                u.to_lowercase()
                                    .trim_end_matches('/')
                                    .replace("://www.", "://")
                            };
                            if !links.iter().any(|s| key(&s.url) == key(&l.url)) {
                                links.push(l.clone());
                            }
                        }
                        super::save_links(pool, user_id, &links).await.map(drop)
                    }
                    Err(e) => Err(e),
                }
            }
            ProfileSection::Education => super::save_educations(pool, user_id, &p.educations)
                .await
                .map(drop),
            ProfileSection::Experience => super::save_experiences(pool, user_id, &p.experiences)
                .await
                .map(drop),
            ProfileSection::Projects => super::save_projects(pool, user_id, &p.projects)
                .await
                .map(drop),
            ProfileSection::Skills => super::save_skills(pool, user_id, &p.skills).await.map(drop),
        };
        match result {
            Ok(()) => out.updated.push(section),
            Err(ProfileError::Invalid(message)) => out.failed.push(SectionFailure {
                section,
                message,
                field: resume_field(section, ""),
            }),
            Err(ProfileError::Field { field, message }) => out.failed.push(SectionFailure {
                section,
                message,
                field: resume_field(section, &field),
            }),
            Err(e) => return Err(e),
        }
    }
    Ok(out)
}

/// The resume's personal details over the saved ones, keeping saved values
/// the resume doesn't have.
fn merge_personal(saved: &Personal, from: &Personal) -> Personal {
    let pick = |new: &str, old: &str| if new.trim().is_empty() { old } else { new }.to_owned();
    // The name is one unit: a resume with a name replaces all of it.
    let has_name = !from.first_name.is_empty() || !from.last_name.is_empty();
    // So is the place: a new city with an old ZIP would be wrong.
    let has_place = !from.city.is_empty() || !from.region.is_empty();
    Personal {
        first_name: if has_name {
            from.first_name.clone()
        } else {
            saved.first_name.clone()
        },
        middle_name: if has_name {
            from.middle_name.clone()
        } else {
            saved.middle_name.clone()
        },
        last_name: if has_name {
            from.last_name.clone()
        } else {
            saved.last_name.clone()
        },
        email: pick(&from.email, &saved.email),
        phone: pick(&from.phone, &saved.phone),
        phone_type: saved.phone_type.clone(),
        street: if has_place && from.city != saved.city {
            String::new()
        } else {
            saved.street.clone()
        },
        city: pick(&from.city, &saved.city),
        region: pick(&from.region, &saved.region),
        postal_code: if has_place && from.city != saved.city {
            String::new()
        } else {
            saved.postal_code.clone()
        },
        country: pick(&from.country, &saved.country),
        headline: pick(&from.headline, &saved.headline),
        summary: pick(&from.summary, &saved.summary),
    }
}

/// Fills an empty, never-filled profile from a resume (the given one, or
/// the primary one). Returns whether it did.
///
/// The marker is claimed first in one statement, so two uploads at the same
/// moment can't both fill. Every section is then saved through
/// `apply_sections`; one that doesn't pass validation is left out and
/// listed for the banner.
pub async fn fill_once(
    pool: &PgPool,
    files: &Files,
    user_id: Uuid,
    resume_id: Option<Uuid>,
) -> Result<bool, ProfileError> {
    if !never_filled(pool, user_id).await? || !is_empty(pool, user_id).await? {
        return Ok(false);
    }
    let resume_id = match resume_id {
        Some(id) => id,
        None => {
            let primary: Option<(Uuid,)> =
                sqlx::query_as("SELECT id FROM resumes WHERE user_id = $1 AND is_primary LIMIT 1")
                    .bind(user_id)
                    .fetch_optional(pool)
                    .await?;
            match primary {
                Some((id,)) => id,
                None => return Ok(false),
            }
        }
    };
    let content = resume::content(pool, files, user_id, resume_id)
        .await
        .map_err(|e| ProfileError::Invalid(format!("Couldn't read the resume: {e}")))?;
    let Some(content) = content else {
        return Ok(false);
    };

    // Claim: only while never filled and still without a name.
    let claimed: Option<(Uuid,)> = sqlx::query_as(
        "INSERT INTO profiles (user_id, filled_from_resume, filled_at)
         VALUES ($1, $2, now())
         ON CONFLICT (user_id) DO UPDATE
             SET filled_from_resume = EXCLUDED.filled_from_resume, filled_at = EXCLUDED.filled_at
             WHERE profiles.filled_at IS NULL
               AND profiles.first_name = '' AND profiles.last_name = ''
         RETURNING user_id",
    )
    .bind(user_id)
    .bind(resume_id)
    .fetch_optional(pool)
    .await?;
    if claimed.is_none() {
        return Ok(false);
    }

    let applied = apply_sections(pool, user_id, &content, &ProfileSection::ALL).await?;
    let skipped: Vec<String> = applied
        .failed
        .iter()
        .map(|f| f.section.label().to_owned())
        .collect();
    sqlx::query("UPDATE profiles SET fill_skipped = $2 WHERE user_id = $1")
        .bind(user_id)
        .bind(&skipped)
        .execute(pool)
        .await?;
    tracing::info!(skipped = ?skipped, "filled the profile from the first resume");
    Ok(true)
}

/// Which resume filled the profile, if one did.
pub async fn filled_from(pool: &PgPool, user_id: Uuid) -> sqlx::Result<Option<FilledFrom>> {
    type Row = (
        Option<Uuid>,
        Option<String>,
        DateTime<Utc>,
        Vec<String>,
        bool,
        Option<i32>,
    );
    let row = sqlx::query_as::<_, Row>(
        "SELECT p.filled_from_resume, r.name, p.filled_at, p.fill_skipped, p.fill_reviewed,
                jsonb_array_length(COALESCE(r.content -> 'changes', '[]'::jsonb))
         FROM profiles p LEFT JOIN resumes r ON r.id = p.filled_from_resume
         WHERE p.user_id = $1 AND p.filled_at IS NOT NULL",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| FilledFrom {
        resume_id: r.0.map(|id| id.to_string()),
        name: r.1.unwrap_or_else(|| "a deleted resume".into()),
        filled_at: r.2,
        skipped: r.3,
        reviewed: r.4,
        open_changes: r.5.unwrap_or(0).into(),
    }))
}

/// Hides the "check what we read" banner.
pub async fn mark_reviewed(pool: &PgPool, user_id: Uuid) -> sqlx::Result<()> {
    sqlx::query("UPDATE profiles SET fill_reviewed = true WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn personal_merge_keeps_what_the_resume_lacks() {
        let saved = Personal {
            first_name: "Old".into(),
            last_name: "Name".into(),
            street: "1 Main St".into(),
            city: "Clemson".into(),
            region: "SC".into(),
            postal_code: "29631".into(),
            phone_type: "mobile".into(),
            headline: "QA engineer".into(),
            ..Default::default()
        };
        let from = Personal {
            first_name: "Kevin".into(),
            last_name: "Lam".into(),
            email: "k@example.com".into(),
            city: "Clemson".into(),
            region: "SC".into(),
            ..Default::default()
        };
        let m = merge_personal(&saved, &from);
        assert_eq!(
            (m.first_name.as_str(), m.last_name.as_str()),
            ("Kevin", "Lam")
        );
        assert_eq!(m.email, "k@example.com");
        // Same city: the street and ZIP still apply.
        assert_eq!(
            (m.street.as_str(), m.postal_code.as_str()),
            ("1 Main St", "29631")
        );
        assert_eq!(m.phone_type, "mobile");
        assert_eq!(m.headline, "QA engineer");
        // A different city drops the old street and ZIP.
        let moved = merge_personal(
            &saved,
            &Personal {
                city: "Austin".into(),
                region: "TX".into(),
                ..from
            },
        );
        assert_eq!(
            (moved.street.as_str(), moved.postal_code.as_str()),
            ("", "")
        );
        assert_eq!(moved.city, "Austin");
    }

    #[test]
    fn failures_point_at_the_resume_field() {
        assert_eq!(
            resume_field(ProfileSection::Personal, "personal.email").as_deref(),
            Some("header.email")
        );
        assert_eq!(
            resume_field(ProfileSection::Experience, "experiences.2.end").as_deref(),
            Some("experiences")
        );
        assert_eq!(
            resume_field(ProfileSection::Personal, "personal.street"),
            None
        );
    }
}
