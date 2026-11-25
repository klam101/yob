//! A resume's own content: what the file says, cleaned up, and edited on the
//! resume's page. It's separate from the profile. A resume made for one kind
//! of job can differ from the profile, and the profile only changes when the
//! user fills it from a resume.
//!
//! Stored as JSON in `resumes.content`. `CONTENT_VERSION` goes up when the
//! shape changes in a way old rows need converting for.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{
    Education, Experience, Link, LinkKind, Personal, Profile, ProfileError, Project, Skill,
    link_kind, normalize_url, parse::Parsed,
};

/// 2: an education entry's degree, major and minor became `degrees`.
/// 3: each degree became one "X in Y" row (`degree`, `field`); a minor or a
/// second major is its own row ("Minor" in "Business").
pub const CONTENT_VERSION: i32 = 3;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct ResumeContent {
    pub header: ResumeHeader,
    pub summary: String,
    /// The order of the sections on this resume.
    pub sections: Vec<SectionKind>,
    pub educations: Vec<ResumeEducation>,
    pub experiences: Vec<ResumeExperience>,
    pub projects: Vec<Project>,
    pub skills: Vec<Skill>,
    /// What the parser changed and the user hasn't checked yet.
    pub changes: Vec<Change>,
    /// Text the parser couldn't place, for the user to move or dismiss.
    pub unplaced: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct ResumeHeader {
    pub name: String,
    /// A line under the name ("QA engineer").
    pub subtitle: String,
    pub email: String,
    pub phone: String,
    /// "City, ST", or whatever the resume says.
    pub location: String,
    pub tags: Vec<String>,
    pub links: Vec<ResumeLink>,
}

/// A link as the resume shows it: a label ("GitHub", "portfolio") and its
/// address.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct ResumeLink {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SectionKind {
    Education,
    Experience,
    Projects,
    Skills,
}

pub const ALL_SECTIONS: [SectionKind; 4] = [
    SectionKind::Education,
    SectionKind::Experience,
    SectionKind::Projects,
    SectionKind::Skills,
];

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct ResumeEducation {
    pub school: String,
    /// "X in Y" rows: "Bachelor of Science" in "Computer Science", "Minor"
    /// in "Business", "Major" in "Mathematics" for a double major.
    pub degrees: Vec<ResumeDegree>,
    pub gpa: String,
    pub location: String,
    pub start: Option<String>,
    pub end: Option<String>,
    /// One line, like "Dean's List" or "Magna cum laude".
    pub achievement: String,
    pub coursework: Vec<String>,
    /// Honors, activities and anything else under the entry.
    pub bullets: Vec<String>,
    // Version 1 kept one degree, major and minor here. Read so old content
    // loads; `upgrade` moves them into `degrees`. Never written.
    #[serde(skip_serializing)]
    #[schema(ignore)]
    pub degree: String,
    #[serde(skip_serializing)]
    #[schema(ignore)]
    pub field_of_study: String,
    #[serde(skip_serializing)]
    #[schema(ignore)]
    pub minor: String,
}

/// One "X in Y" row of an education entry. `degree` is what it is
/// ("Bachelor of Science", "B.S." as the resume wrote it, "Minor",
/// "Concentration") and `field` what it's in. Both are free text.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct ResumeDegree {
    pub degree: String,
    pub field: String,
    // Version 2 kept lists here. Read so old content loads; `upgrade` turns
    // them into rows. Never written.
    #[serde(skip_serializing)]
    #[schema(ignore)]
    pub majors: Vec<String>,
    #[serde(skip_serializing)]
    #[schema(ignore)]
    pub minors: Vec<String>,
    #[serde(skip_serializing)]
    #[schema(ignore)]
    pub concentrations: Vec<String>,
}

impl ResumeDegree {
    pub fn row(degree: &str, field: &str) -> Self {
        Self {
            degree: degree.to_owned(),
            field: field.to_owned(),
            ..Self::default()
        }
    }

    fn is_blank(&self) -> bool {
        self.degree.trim().is_empty() && self.field.trim().is_empty()
    }
}

/// What a row's first input says it is, for the profile's one-degree
/// entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowKind {
    Degree,
    Major,
    Minor,
    Concentration,
}

fn row_kind(degree: &str) -> RowKind {
    let d = degree.trim().to_lowercase();
    let starts = |words: &[&str]| words.iter().any(|w| d.starts_with(w));
    if starts(&["minor"]) {
        RowKind::Minor
    } else if starts(&["major", "double major", "second major", "co-major"]) {
        RowKind::Major
    } else if starts(&[
        "concentration",
        "specialization",
        "specialisation",
        "emphasis",
        "track",
        "option in",
    ]) {
        RowKind::Concentration
    } else {
        RowKind::Degree
    }
}

impl ResumeEducation {
    /// Nothing filled in (an empty degree row doesn't count).
    fn is_blank(&self) -> bool {
        let Self {
            school,
            degrees,
            gpa,
            location,
            start,
            end,
            achievement,
            coursework,
            bullets,
            ..
        } = self;
        school.is_empty()
            && degrees.iter().all(ResumeDegree::is_blank)
            && gpa.is_empty()
            && location.is_empty()
            && start.is_none()
            && end.is_none()
            && achievement.is_empty()
            && coursework.is_empty()
            && bullets.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct ResumeExperience {
    pub company: String,
    pub title: String,
    pub location: String,
    pub employment_type: String,
    pub start: Option<String>,
    /// None with a start date means "Present".
    pub end: Option<String>,
    /// A line about the role or the team, above the bullets.
    pub summary: String,
    pub bullets: Vec<String>,
}

/// Something the parser changed from what the file literally said, for the
/// user to check.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Change {
    /// The field it's about, as a path: `experiences.0.title`.
    pub field: String,
    pub kind: ChangeKind,
    pub before: String,
    pub after: String,
    /// Values that put the field(s) back as the file had them. Empty when
    /// there's nothing to undo (the old text was the broken one).
    #[serde(default)]
    pub undo: Vec<FieldValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct FieldValue {
    pub field: String,
    /// A string for text fields, a list for lists (`majors`).
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    /// ALL CAPS turned into title case.
    Capitals,
    /// A LaTeX small-caps font's lowercase "i" fixed ("QUALiTY").
    SmallCaps,
    /// A word hyphenated across two lines joined back ("deliv-" "ering").
    JoinedWord,
    /// An icon in front of a user name read as that account's address.
    IconLink,
    /// A link label ("GitHub") taken off a project's name.
    LinkLabel,
    /// One line split into degree, major and minor.
    DegreeSplit,
    /// "Class of 2026" with no month read as May.
    ClassOfMay,
    /// An address found only in the file's link, not in its text.
    LinkFromFile,
}

impl Change {
    pub fn new(field: impl Into<String>, kind: ChangeKind, before: &str, after: &str) -> Self {
        Self {
            field: field.into(),
            kind,
            before: before.to_owned(),
            after: after.to_owned(),
            undo: Vec::new(),
        }
    }

    /// Undo by putting `before` back into the field itself.
    pub fn undoable(mut self) -> Self {
        self.undo = vec![FieldValue {
            field: self.field.clone(),
            value: self.before.clone().into(),
        }];
        self
    }

    /// The same change about an entry inside a list: `title` -> `experiences.2.title`.
    pub fn under(mut self, prefix: &str) -> Self {
        let add = |f: &str| format!("{prefix}.{f}");
        self.field = add(&self.field);
        for u in &mut self.undo {
            u.field = add(&u.field);
        }
        self
    }
}

fn label_for(kind: LinkKind) -> &'static str {
    match kind {
        LinkKind::Linkedin => "LinkedIn",
        LinkKind::Github => "GitHub",
        LinkKind::Portfolio => "Portfolio",
        LinkKind::Other => "Link",
    }
}

/// "Coursework: A, B" (any of the labels `parse::coursework_line` writes)
/// to the course names.
fn coursework(line: &str) -> Option<Vec<String>> {
    let (label, rest) = line.split_once(':')?;
    let label = label.trim().to_lowercase();
    if !matches!(
        label.as_str(),
        "coursework" | "relevant coursework" | "related coursework" | "courses"
    ) {
        return None;
    }
    Some(
        rest.split([',', ';'])
            .map(|c| c.trim().trim_end_matches('.').to_owned())
            .filter(|c| !c.is_empty())
            .collect(),
    )
}

/// Brings content saved by an older `CONTENT_VERSION` up to date, keeping
/// the user's edits, and moves the parser's changes that point at what moved.
/// Safe to run on current content (it does nothing).
///
/// - Version 1 had one degree, major and minor per education entry.
/// - Version 2 had degree blocks with lists of majors, minors and
///   concentrations. Each block becomes rows: the degree in its majors, then
///   "Minor" in each minor and "Concentration" in each concentration.
pub fn upgrade(c: &mut ResumeContent) {
    for (i, e) in c.educations.iter_mut().enumerate() {
        // Version 1 -> 2: the entry's own fields become its first block.
        let degree = std::mem::take(&mut e.degree);
        let field = std::mem::take(&mut e.field_of_study);
        let minor = std::mem::take(&mut e.minor);
        if e.degrees.is_empty() {
            let list = |v: &str| {
                if v.trim().is_empty() {
                    Vec::new()
                } else {
                    vec![v.trim().to_owned()]
                }
            };
            // Every entry gets a row to fill in, even an empty one.
            e.degrees.push(ResumeDegree {
                degree,
                majors: list(&field),
                minors: list(&minor),
                ..ResumeDegree::default()
            });
        }

        // Version 2 -> 3: blocks become rows. `rows[m]` is where block m's
        // first row, minors and concentrations landed, for the change paths.
        struct Moved {
            first: usize,
            minor: Option<usize>,
            concentration: Option<usize>,
        }
        let mut rows = Vec::new();
        let mut moved = Vec::new();
        for d in std::mem::take(&mut e.degrees) {
            let first = rows.len();
            let field = if d.field.is_empty() {
                d.majors.join(", ")
            } else {
                d.field
            };
            rows.push(ResumeDegree::row(&d.degree, &field));
            let minor = (!d.minors.is_empty()).then_some(rows.len());
            rows.extend(d.minors.iter().map(|m| ResumeDegree::row("Minor", m)));
            let concentration = (!d.concentrations.is_empty()).then_some(rows.len());
            rows.extend(
                d.concentrations
                    .iter()
                    .map(|m| ResumeDegree::row("Concentration", m)),
            );
            moved.push(Moved {
                first,
                minor,
                concentration,
            });
        }
        e.degrees = rows;

        let at = format!("educations.{i}.");
        // An old path inside this entry to today's, and whether it held a
        // list (its undo value is then joined into text).
        let move_path = |f: &str| -> Option<(String, bool)> {
            let rest = f.strip_prefix(&at)?;
            let rest = match rest {
                "degree" => "degrees.0.degree",
                "field_of_study" => "degrees.0.majors",
                "minor" => "degrees.0.minors",
                r => r,
            };
            let mut parts = rest.split('.');
            let (Some("degrees"), Some(m), Some(name), None) =
                (parts.next(), parts.next(), parts.next(), parts.next())
            else {
                return None;
            };
            let to = moved.get(m.parse::<usize>().ok()?)?;
            let (row, name, list) = match name {
                "degree" => (to.first, "degree", false),
                "field" => (to.first, "field", false),
                "majors" => (to.first, "field", true),
                "minors" => (to.minor.unwrap_or(to.first), "field", true),
                "concentrations" => (to.concentration.unwrap_or(to.first), "field", true),
                _ => return None,
            };
            Some((format!("{at}degrees.{row}.{name}"), list))
        };
        for ch in &mut c.changes {
            if let Some((to, _)) = move_path(&ch.field) {
                ch.field = to;
            }
            if ch.kind == ChangeKind::DegreeSplit && ch.field.starts_with(&at) {
                // Undo puts the original line back as the only row.
                ch.undo = vec![FieldValue {
                    field: format!("{at}degrees"),
                    value: serde_json::json!([{ "degree": ch.before, "field": "" }]),
                }];
                continue;
            }
            for u in &mut ch.undo {
                if let Some((to, is_list)) = move_path(&u.field) {
                    u.field = to;
                    if is_list && let Some(items) = u.value.as_array() {
                        let text: Vec<&str> = items.iter().filter_map(|v| v.as_str()).collect();
                        u.value = text.join(", ").into();
                    }
                }
            }
        }
    }
}

/// The parsed draft as a resume's content.
pub fn from_parsed(parsed: &Parsed) -> ResumeContent {
    let mut c = from_parsed_v1(parsed);
    upgrade(&mut c);
    c
}

/// The parse in version 1's shape (one degree per entry); `upgrade` turns it
/// into the current one, so the parser's change paths move in one place.
fn from_parsed_v1(parsed: &Parsed) -> ResumeContent {
    let p = &parsed.profile;
    let me = &p.personal;
    let location = [me.city.as_str(), me.region.as_str()]
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(", ");
    let mut sections = parsed.sections.clone();
    for s in ALL_SECTIONS {
        if !sections.contains(&s) {
            sections.push(s);
        }
    }
    ResumeContent {
        header: ResumeHeader {
            name: [
                me.first_name.as_str(),
                me.middle_name.as_str(),
                me.last_name.as_str(),
            ]
            .iter()
            .filter(|s| !s.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(" "),
            subtitle: me.headline.clone(),
            email: me.email.clone(),
            phone: me.phone.clone(),
            location,
            tags: Vec::new(),
            links: p
                .links
                .iter()
                .map(|l| ResumeLink {
                    label: label_for(l.kind).into(),
                    url: l.url.clone(),
                })
                .collect(),
        },
        summary: me.summary.clone(),
        sections,
        educations: p
            .educations
            .iter()
            .map(|e| {
                let mut courses = Vec::new();
                let mut bullets = Vec::new();
                for line in e.details.lines().map(str::trim).filter(|l| !l.is_empty()) {
                    match coursework(line) {
                        Some(c) => courses.extend(c),
                        None => bullets.push(line.to_owned()),
                    }
                }
                ResumeEducation {
                    school: e.school.clone(),
                    degrees: Vec::new(),
                    gpa: e.gpa.clone(),
                    location: e.location.clone(),
                    start: e.start.clone(),
                    end: e.end.clone(),
                    achievement: String::new(),
                    coursework: courses,
                    bullets,
                    degree: e.degree.clone(),
                    field_of_study: e.field_of_study.clone(),
                    minor: e.minor.clone(),
                }
            })
            .collect(),
        experiences: p
            .experiences
            .iter()
            .map(|x| ResumeExperience {
                company: x.company.clone(),
                title: x.title.clone(),
                location: x.location.clone(),
                employment_type: x.employment_type.clone(),
                start: x.start.clone(),
                end: x.end.clone(),
                summary: String::new(),
                bullets: x.bullets.clone(),
            })
            .collect(),
        projects: p.projects.clone(),
        skills: p.skills.clone(),
        changes: parsed.changes.clone(),
        unplaced: parsed.unsorted.clone(),
    }
}

/// One profile education entry's degree, built from rows.
#[derive(Debug, Default)]
struct ProfileDegree {
    degree: String,
    majors: Vec<String>,
    minors: Vec<String>,
    concentrations: Vec<String>,
}

/// An entry's rows as the profile's degrees, one per degree row. A "Major",
/// "Minor" or "Concentration" row adds to the degree above it (or the first
/// one, when it comes before any degree). At least one degree comes back.
fn profile_degrees(rows: &[ResumeDegree]) -> Vec<ProfileDegree> {
    let mut out: Vec<ProfileDegree> = Vec::new();
    // Rows before the first degree, for that degree.
    let mut early = ProfileDegree::default();
    for r in rows.iter().filter(|r| !r.is_blank()) {
        let field = r.field.trim().to_owned();
        let kind = row_kind(&r.degree);
        if kind == RowKind::Degree {
            let mut d = ProfileDegree {
                degree: r.degree.trim().to_owned(),
                ..Default::default()
            };
            if !field.is_empty() {
                d.majors.push(field);
            }
            if out.is_empty() {
                d.majors.append(&mut early.majors);
                d.minors.append(&mut early.minors);
                d.concentrations.append(&mut early.concentrations);
            }
            out.push(d);
            continue;
        }
        if field.is_empty() {
            continue;
        }
        let d = out.last_mut().unwrap_or(&mut early);
        match kind {
            RowKind::Major => d.majors.push(field),
            RowKind::Minor => d.minors.push(field),
            _ => d.concentrations.push(field),
        }
    }
    if out.is_empty() {
        out.push(early);
    }
    out
}

/// "Kevin Y. Lam" -> ("Kevin", "Y.", "Lam"); one word is a first name.
fn split_name(name: &str) -> (String, String, String) {
    let words: Vec<&str> = name.split_whitespace().collect();
    match words.as_slice() {
        [] => Default::default(),
        [one] => ((*one).to_owned(), String::new(), String::new()),
        [first, last] => ((*first).to_owned(), String::new(), (*last).to_owned()),
        [first, middle @ .., last] => ((*first).to_owned(), middle.join(" "), (*last).to_owned()),
    }
}

/// The content as a profile, for "Fill profile from this resume". Answers
/// (work authorization, equal employment) aren't on a resume and stay empty.
pub fn to_profile(c: &ResumeContent) -> Profile {
    let h = &c.header;
    let (first_name, middle_name, last_name) = split_name(&h.name);
    let (city, region) = match h.location.rsplit_once(',') {
        Some((city, region)) => (city.trim().to_owned(), region.trim().to_owned()),
        None => (h.location.trim().to_owned(), String::new()),
    };
    Profile {
        personal: Personal {
            first_name,
            middle_name,
            last_name,
            email: h.email.clone(),
            phone: h.phone.clone(),
            city,
            region,
            country: if h.location.is_empty() {
                String::new()
            } else {
                "United States".into()
            },
            headline: h.subtitle.clone(),
            summary: c.summary.clone(),
            ..Personal::default()
        },
        links: h
            .links
            .iter()
            .filter_map(|l| {
                let url = normalize_url(&l.url).ok().filter(|u| !u.is_empty())?;
                Some(Link {
                    kind: link_kind(&url),
                    url,
                })
            })
            .collect(),
        // The profile has one degree per education entry (application forms
        // ask for it that way), so each degree becomes its own entry.
        educations: c
            .educations
            .iter()
            .filter(|e| !e.is_blank())
            .flat_map(|e| {
                let mut details: Vec<String> = std::iter::once(e.achievement.clone())
                    .filter(|a| !a.trim().is_empty())
                    .chain(e.bullets.iter().cloned())
                    .collect();
                if !e.coursework.is_empty() {
                    details.push(format!("Coursework: {}", e.coursework.join(", ")));
                }
                profile_degrees(&e.degrees)
                    .into_iter()
                    .map(|d| {
                        let mut details = details.clone();
                        if !d.concentrations.is_empty() {
                            details.push(format!("Concentration: {}", d.concentrations.join(", ")));
                        }
                        Education {
                            school: e.school.clone(),
                            degree: d.degree,
                            field_of_study: d.majors.join(", "),
                            minor: d.minors.join(", "),
                            gpa: e.gpa.clone(),
                            location: e.location.clone(),
                            start: e.start.clone(),
                            end: e.end.clone(),
                            details: details.join("\n"),
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect(),
        experiences: c
            .experiences
            .iter()
            .filter(|x| **x != ResumeExperience::default())
            .map(|x| Experience {
                company: x.company.clone(),
                title: x.title.clone(),
                location: x.location.clone(),
                employment_type: x.employment_type.clone(),
                start: x.start.clone(),
                end: x.end.clone(),
                bullets: if x.summary.trim().is_empty() {
                    x.bullets.clone()
                } else {
                    std::iter::once(x.summary.clone())
                        .chain(x.bullets.iter().cloned())
                        .collect()
                },
            })
            .collect(),
        projects: c
            .projects
            .iter()
            .filter(|p| **p != Project::default())
            .cloned()
            .collect(),
        skills: c.skills.clone(),
        answers: Default::default(),
        filled_from: None,
    }
}

// --- validation --------------------------------------------------------------

const SHORT: usize = 200;
const LONG: usize = 5_000;
const BULLET: usize = 1_500;
const MAX_ITEMS: usize = 60;
const MAX_BULLETS: usize = 30;

fn err(field: impl Into<String>, message: impl Into<String>) -> ProfileError {
    ProfileError::Field {
        field: field.into(),
        message: message.into(),
    }
}

/// One line, trimmed and with inner runs of spaces collapsed.
fn line(field: &str, v: &str) -> Result<String, ProfileError> {
    let v = v.split_whitespace().collect::<Vec<_>>().join(" ");
    if v.chars().count() > SHORT {
        return Err(err(field, format!("Keep this under {SHORT} characters.")));
    }
    Ok(v)
}

fn text(field: &str, v: &str) -> Result<String, ProfileError> {
    let v = v.trim().to_owned();
    if v.chars().count() > LONG {
        return Err(err(field, format!("Keep this under {LONG} characters.")));
    }
    Ok(v)
}

fn list(
    field: &str,
    items: &[String],
    max_len: usize,
    max: usize,
) -> Result<Vec<String>, ProfileError> {
    let out: Vec<String> = items
        .iter()
        .map(|b| b.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|b| !b.is_empty())
        .collect();
    if out.len() > max {
        return Err(err(field, format!("At most {max} items here.")));
    }
    if out.iter().any(|b| b.chars().count() > max_len) {
        return Err(err(
            field,
            format!("Keep each item under {max_len} characters."),
        ));
    }
    Ok(out)
}

fn month(field: &str, v: &Option<String>) -> Result<Option<String>, ProfileError> {
    match v.as_deref().map(str::trim) {
        None | Some("") => Ok(None),
        Some(m) => super::parse_month(m)
            .map(|d| Some(d.format("%Y-%m").to_string()))
            .ok_or_else(|| err(field, "Pick a month, like 2024-05.")),
    }
}

fn range(
    path: &str,
    start: &Option<String>,
    end: &Option<String>,
) -> Result<(Option<String>, Option<String>), ProfileError> {
    let s = month(&format!("{path}.start"), start)?;
    let e = month(&format!("{path}.end"), end)?;
    if let (Some(s), Some(e)) = (&s, &e)
        && e < s
    {
        return Err(err(
            format!("{path}.end"),
            "The end date is before the start date.",
        ));
    }
    Ok((s, e))
}

fn count(path: &str, n: usize) -> Result<(), ProfileError> {
    if n > MAX_ITEMS {
        return Err(err(path, format!("At most {MAX_ITEMS} entries.")));
    }
    Ok(())
}

/// Checks and tidies a resume's content before it's saved. The order of
/// entries is the user's and is kept. Errors name the field
/// (`experiences.1.end`). Empty entries and links keep their places, so field paths (the parser's
/// changes point at them) stay valid; `to_profile` leaves them out.
pub fn validate(c: &ResumeContent) -> Result<ResumeContent, ProfileError> {
    let h = &c.header;
    let mut links = Vec::new();
    for (i, l) in h.links.iter().enumerate() {
        let path = format!("header.links.{i}");
        let url = normalize_url(&l.url).map_err(|e| err(format!("{path}.url"), e.to_string()))?;
        let label = line(&format!("{path}.label"), &l.label)?;
        // Kept even when empty: indices are field paths for the parser's
        // changes, so rows never shift on save.
        links.push(ResumeLink { label, url });
    }
    let email = line("header.email", &h.email)?;
    if !email.is_empty() && !(email.contains('@') && email.contains('.')) {
        return Err(err(
            "header.email",
            "Enter an email address like name@example.com.",
        ));
    }
    let header = ResumeHeader {
        name: line("header.name", &h.name)?,
        subtitle: line("header.subtitle", &h.subtitle)?,
        email,
        phone: line("header.phone", &h.phone)?,
        location: line("header.location", &h.location)?,
        tags: list("header.tags", &h.tags, 60, 30)?,
        links,
    };

    let mut sections: Vec<SectionKind> = Vec::new();
    for s in c.sections.iter().chain(ALL_SECTIONS.iter()) {
        if !sections.contains(s) {
            sections.push(*s);
        }
    }

    count("educations", c.educations.len())?;
    let mut educations = Vec::new();
    for (i, e) in c.educations.iter().enumerate() {
        let path = format!("educations.{i}");
        let f = |name: &str, v: &str| line(&format!("{path}.{name}"), v);
        let (start, end) = range(&path, &e.start, &e.end)?;
        if e.degrees.len() > 20 {
            return Err(err(
                format!("{path}.degrees"),
                "At most 20 rows per school.",
            ));
        }
        let mut degrees = Vec::new();
        for (j, d) in e.degrees.iter().enumerate() {
            let at = format!("{path}.degrees.{j}");
            let degree = line(&format!("{at}.degree"), &d.degree)?;
            if degree.chars().count() > 100 {
                return Err(err(
                    format!("{at}.degree"),
                    "Keep this under 100 characters.",
                ));
            }
            // Empty rows keep their places, like empty entries.
            degrees.push(ResumeDegree::row(
                &degree,
                &line(&format!("{at}.field"), &d.field)?,
            ));
        }
        let ed = ResumeEducation {
            school: f("school", &e.school)?,
            degrees,
            gpa: f("gpa", &e.gpa)?,
            location: f("location", &e.location)?,
            start,
            end,
            achievement: f("achievement", &e.achievement)?,
            coursework: list(&format!("{path}.coursework"), &e.coursework, 100, 40)?,
            bullets: list(&format!("{path}.bullets"), &e.bullets, BULLET, MAX_BULLETS)?,
            ..Default::default()
        };
        educations.push(ed);
    }

    count("experiences", c.experiences.len())?;
    let mut experiences = Vec::new();
    for (i, x) in c.experiences.iter().enumerate() {
        let path = format!("experiences.{i}");
        let f = |name: &str, v: &str| line(&format!("{path}.{name}"), v);
        let (start, end) = range(&path, &x.start, &x.end)?;
        let ex = ResumeExperience {
            company: f("company", &x.company)?,
            title: f("title", &x.title)?,
            location: f("location", &x.location)?,
            employment_type: f("employment_type", &x.employment_type)?,
            start,
            end,
            summary: text(&format!("{path}.summary"), &x.summary)?,
            bullets: list(&format!("{path}.bullets"), &x.bullets, BULLET, MAX_BULLETS)?,
        };
        experiences.push(ex);
    }

    count("projects", c.projects.len())?;
    let mut projects = Vec::new();
    for (i, p) in c.projects.iter().enumerate() {
        let path = format!("projects.{i}");
        let f = |name: &str, v: &str| line(&format!("{path}.{name}"), v);
        let (start, end) = range(&path, &p.start, &p.end)?;
        let pr = Project {
            name: f("name", &p.name)?,
            summary: f("summary", &p.summary)?,
            role: f("role", &p.role)?,
            url: normalize_url(&p.url).map_err(|e| err(format!("{path}.url"), e.to_string()))?,
            start,
            end,
            bullets: list(&format!("{path}.bullets"), &p.bullets, BULLET, MAX_BULLETS)?,
            skills: list(&format!("{path}.skills"), &p.skills, 60, 40)?,
        };
        projects.push(pr);
    }

    if c.skills.len() > 300 {
        return Err(err("skills", "At most 300 skills."));
    }
    let mut seen = std::collections::HashSet::new();
    let mut skills = Vec::new();
    for s in &c.skills {
        let name = line("skills", &s.name)?;
        if !name.is_empty() && seen.insert(name.to_lowercase()) {
            skills.push(Skill {
                name,
                category: line("skills", &s.category)?,
            });
        }
    }

    Ok(ResumeContent {
        header,
        summary: text("summary", &c.summary)?,
        sections,
        educations,
        experiences,
        projects,
        skills,
        changes: c.changes.iter().take(200).cloned().collect(),
        unplaced: c.unplaced.iter().take(100).cloned().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed() -> Parsed {
        let mut p = Parsed::default();
        p.profile.personal.first_name = "Morgan".into();
        p.profile.personal.last_name = "Ellis".into();
        p.profile.personal.city = "Columbus".into();
        p.profile.personal.region = "OH".into();
        p.profile.links.push(Link {
            kind: LinkKind::Github,
            url: "https://github.com/mellis-example".into(),
        });
        p.profile.educations.push(Education {
            school: "Lakeside State University".into(),
            details: "Dean's list\nCoursework: Databases, Data Mining".into(),
            ..Default::default()
        });
        p.sections = vec![SectionKind::Experience, SectionKind::Education];
        p
    }

    #[test]
    fn from_parsed_splits_coursework_and_orders_sections() {
        let c = from_parsed(&parsed());
        assert_eq!(c.header.name, "Morgan Ellis");
        assert_eq!(c.header.location, "Columbus, OH");
        assert_eq!(c.header.links[0].label, "GitHub");
        assert_eq!(c.educations[0].coursework, ["Databases", "Data Mining"]);
        assert_eq!(c.educations[0].bullets, ["Dean's list"]);
        assert_eq!(
            c.sections,
            [
                SectionKind::Experience,
                SectionKind::Education,
                SectionKind::Projects,
                SectionKind::Skills
            ]
        );
    }

    #[test]
    fn round_trip_to_profile() {
        let p = to_profile(&from_parsed(&parsed()));
        assert_eq!(
            (
                p.personal.first_name.as_str(),
                p.personal.last_name.as_str()
            ),
            ("Morgan", "Ellis")
        );
        assert_eq!(
            (p.personal.city.as_str(), p.personal.region.as_str()),
            ("Columbus", "OH")
        );
        assert_eq!(p.links[0].kind, LinkKind::Github);
        assert_eq!(
            p.educations[0].details,
            "Dean's list\nCoursework: Databases, Data Mining"
        );
        assert_eq!(
            split_name("Kevin Y. Lam"),
            ("Kevin".into(), "Y.".into(), "Lam".into())
        );
    }

    #[test]
    fn validation_names_fields_and_keeps_order() {
        let mut c = from_parsed(&parsed());
        c.experiences = vec![
            ResumeExperience {
                company: "B".into(),
                start: Some("2020-01".into()),
                ..Default::default()
            },
            ResumeExperience {
                company: "A".into(),
                start: Some("2024-01".into()),
                end: Some("2023-01".into()),
                ..Default::default()
            },
        ];
        match validate(&c) {
            Err(ProfileError::Field { field, .. }) => assert_eq!(field, "experiences.1.end"),
            other => panic!("{other:?}"),
        }
        c.experiences[1].end = None;
        c.experiences.push(ResumeExperience::default());
        c.header.links.insert(0, ResumeLink::default());
        let v = validate(&c).unwrap();
        // Kept in the user's order and places, the empty entry and link too.
        assert_eq!(
            v.experiences
                .iter()
                .map(|x| x.company.as_str())
                .collect::<Vec<_>>(),
            ["B", "A", ""]
        );
        assert_eq!(v.header.links[0].url, "");
        assert_eq!(v.header.links[1].label, "GitHub");
        // The profile leaves the empty ones out.
        let p = to_profile(&v);
        assert_eq!(p.experiences.len(), 2);
        assert_eq!(p.links.len(), 1);
        c.header.links.remove(0);
        c.header.links[0].url = "javascript:alert(1)".into();
        match validate(&c) {
            Err(ProfileError::Field { field, .. }) => assert_eq!(field, "header.links.0.url"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn version_1_content_upgrades_in_place() {
        // As stored by version 1: one degree, major and minor per entry, and
        // the parser's degree-split change pointing at them.
        let json = serde_json::json!({
            "educations": [{
                "school": "Lakeside State University",
                "degree": "Bachelor of Science",
                "field_of_study": "Information Systems",
                "minor": "Statistics",
                "coursework": ["Databases"]
            }],
            "changes": [{
                "field": "educations.0.degree",
                "kind": "degree_split",
                "before": "BACHELOR OF SCIENCE IN INFORMATION SYSTEMS, MINOR STATISTICS",
                "after": "Bachelor of Science · Information Systems · Minor in Statistics",
                "undo": [
                    {"field": "educations.0.degree", "value": "BACHELOR OF SCIENCE IN INFORMATION SYSTEMS, MINOR STATISTICS"},
                    {"field": "educations.0.field_of_study", "value": ""},
                    {"field": "educations.0.minor", "value": ""}
                ]
            }]
        });
        let mut c: ResumeContent = serde_json::from_value(json).unwrap();
        upgrade(&mut c);
        let e = &c.educations[0];
        assert_eq!(
            e.degrees,
            [
                ResumeDegree::row("Bachelor of Science", "Information Systems"),
                ResumeDegree::row("Minor", "Statistics"),
            ]
        );
        assert_eq!(e.coursework, ["Databases"], "other edits are kept");
        let ch = &c.changes[0];
        assert_eq!(ch.field, "educations.0.degrees.0.degree");
        assert_eq!(
            ch.undo,
            [FieldValue {
                field: "educations.0.degrees".into(),
                value: serde_json::json!([{
                    "degree": "BACHELOR OF SCIENCE IN INFORMATION SYSTEMS, MINOR STATISTICS",
                    "field": ""
                }]),
            }]
        );
        // The old fields are never written back.
        let out = serde_json::to_value(&c).unwrap();
        assert!(out["educations"][0].get("field_of_study").is_none());
        assert!(out["educations"][0]["degrees"][0].get("majors").is_none());
        // Running it again changes nothing.
        let before = c.clone();
        upgrade(&mut c);
        assert_eq!(c, before);
    }

    #[test]
    fn version_2_blocks_become_rows() {
        let json = serde_json::json!({
            "educations": [{
                "school": "Lakeside State University",
                "degrees": [
                    {"degree": "Minor", "majors": ["Business"], "minors": [], "concentrations": []},
                    {"degree": "", "majors": [], "minors": [], "concentrations": []},
                    {"degree": "Bachelor of Science", "majors": ["Computer Science", "Mathematics"],
                     "minors": ["Statistics"], "concentrations": ["Machine Learning"]}
                ]
            }],
            "changes": [
                {"field": "educations.0.degrees.2.degree", "kind": "capitals",
                 "before": "BACHELOR OF SCIENCE", "after": "Bachelor of Science",
                 "undo": [{"field": "educations.0.degrees.2.degree", "value": "BACHELOR OF SCIENCE"}]},
                {"field": "educations.0.degrees.2.minors", "kind": "capitals",
                 "before": "STATISTICS", "after": "Statistics",
                 "undo": [{"field": "educations.0.degrees.2.minors", "value": ["STATISTICS"]}]}
            ]
        });
        let mut c: ResumeContent = serde_json::from_value(json).unwrap();
        upgrade(&mut c);
        assert_eq!(
            c.educations[0].degrees,
            [
                ResumeDegree::row("Minor", "Business"),
                ResumeDegree::row("", ""),
                ResumeDegree::row("Bachelor of Science", "Computer Science, Mathematics"),
                ResumeDegree::row("Minor", "Statistics"),
                ResumeDegree::row("Concentration", "Machine Learning"),
            ]
        );
        assert_eq!(c.changes[0].field, "educations.0.degrees.2.degree");
        assert_eq!(c.changes[1].field, "educations.0.degrees.3.field");
        assert_eq!(c.changes[1].undo[0].field, "educations.0.degrees.3.field");
        assert_eq!(c.changes[1].undo[0].value, "STATISTICS");
        let before = c.clone();
        upgrade(&mut c);
        assert_eq!(c, before);
    }

    #[test]
    fn each_degree_becomes_a_profile_entry() {
        let mut c = from_parsed(&parsed());
        c.educations[0].degrees = vec![
            ResumeDegree::row("Minor", "Business"),
            ResumeDegree::row("Bachelor of Science", "Computer Science"),
            ResumeDegree::row("Major", "Mathematics"),
            ResumeDegree::row("Concentration", "Artificial Intelligence"),
            ResumeDegree::row("Master of Science", "Robotics"),
            ResumeDegree::default(),
        ];
        let p = to_profile(&validate(&c).unwrap());
        assert_eq!(p.educations.len(), 2);
        assert_eq!(
            p.educations[0].field_of_study,
            "Computer Science, Mathematics"
        );
        assert_eq!(p.educations[0].minor, "Business");
        assert!(
            p.educations[0]
                .details
                .contains("Concentration: Artificial Intelligence")
        );
        assert_eq!(
            (
                p.educations[1].degree.as_str(),
                p.educations[1].school.as_str()
            ),
            ("Master of Science", "Lakeside State University")
        );
    }

    #[test]
    fn change_paths() {
        let c = Change::new("title", ChangeKind::Capitals, "QA TESTER", "QA Tester")
            .undoable()
            .under("experiences.2");
        assert_eq!(c.field, "experiences.2.title");
        assert_eq!(c.undo[0].field, "experiences.2.title");
        assert_eq!(c.undo[0].value, "QA TESTER");
        // A parsed resume's entries all start with a degree row.
        let parsed = from_parsed(&parsed());
        assert_eq!(parsed.educations[0].degrees.len(), 1);
        // With no degree rows, an entry still becomes one profile entry.
        let mut c = parsed;
        c.educations[0].degrees = vec![ResumeDegree::row("Minor", "Business")];
        let p = to_profile(&c);
        assert_eq!(p.educations.len(), 1);
        assert_eq!(
            (
                p.educations[0].degree.as_str(),
                p.educations[0].minor.as_str()
            ),
            ("", "Business")
        );
    }
}
