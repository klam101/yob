//! Turns resume text into a draft profile with rules, no AI.
//!
//! 1. Contact details (email, phone, links, address, name) from the top.
//! 2. Sections, by known headings ("Work Experience", "Education", ...).
//! 3. Entries in Experience, Education and Projects: split where a new
//!    header block starts; dates, location, title and company read from it.
//! 4. Bullets (by their marker, with wrapped lines joined back).
//! 5. Skills split on commas and friends; "Label: a, b" sets the category.
//!
//! Whatever can't be placed is returned as `unsorted`, so the review screen
//! can show it. What the parser changed from the file's literal text (capitals,
//! a split degree line, a link label taken off a name) is returned as
//! `changes`, for the user to check on the resume's page.

use std::sync::LazyLock;

use regex::Regex;

use super::{
    Education, Experience, Link, LinkKind, Profile, Project, Skill,
    clean::{Fix, FixKind},
    link_kind, normalize_url,
    resume_content::{Change, ChangeKind, SectionKind},
};

#[derive(Debug, Clone, Default)]
pub struct Parsed {
    pub profile: Profile,
    /// Lines the parser couldn't place, grouped by where they came from.
    pub unsorted: Vec<String>,
    /// The order of the sections in the file.
    pub sections: Vec<SectionKind>,
    /// What the parser changed from the file's text, by field path.
    pub changes: Vec<Change>,
}

/// Records a capitals change when tidying changed the text.
fn caps_change(field: &str, raw: &str, tidy: &str, out: &mut Vec<Change>) {
    let raw = raw.trim();
    if !raw.is_empty() && raw != tidy {
        out.push(Change::new(field, ChangeKind::Capitals, raw, tidy).undoable());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    Header,
    Summary,
    Experience,
    Education,
    Skills,
    Projects,
    Coursework,
    /// A known heading we don't store (awards, certifications, ...).
    Other,
}

const HEADINGS: &[(&str, Section)] = &[
    ("summary", Section::Summary),
    ("professional summary", Section::Summary),
    ("career summary", Section::Summary),
    ("summary of qualifications", Section::Summary),
    ("profile", Section::Summary),
    ("professional profile", Section::Summary),
    ("objective", Section::Summary),
    ("career objective", Section::Summary),
    ("about", Section::Summary),
    ("about me", Section::Summary),
    ("experience", Section::Experience),
    ("work experience", Section::Experience),
    ("professional experience", Section::Experience),
    ("relevant experience", Section::Experience),
    ("employment", Section::Experience),
    ("employment history", Section::Experience),
    ("work history", Section::Experience),
    ("career history", Section::Experience),
    ("internships", Section::Experience),
    ("internship experience", Section::Experience),
    ("experience and internships", Section::Experience),
    ("technical experience", Section::Experience),
    ("education", Section::Education),
    ("academic background", Section::Education),
    ("academics", Section::Education),
    ("education and training", Section::Education),
    ("skills", Section::Skills),
    ("technical skills", Section::Skills),
    ("core skills", Section::Skills),
    ("key skills", Section::Skills),
    ("relevant skills", Section::Skills),
    ("core competencies", Section::Skills),
    ("competencies", Section::Skills),
    ("skills and interests", Section::Skills),
    ("skills and tools", Section::Skills),
    ("skills and technologies", Section::Skills),
    ("technologies", Section::Skills),
    ("technical proficiencies", Section::Skills),
    ("tools and technologies", Section::Skills),
    ("languages and technologies", Section::Skills),
    ("projects", Section::Projects),
    ("personal projects", Section::Projects),
    ("academic projects", Section::Projects),
    ("selected projects", Section::Projects),
    ("relevant projects", Section::Projects),
    ("technical projects", Section::Projects),
    ("side projects", Section::Projects),
    ("relevant coursework", Section::Coursework),
    ("coursework", Section::Coursework),
    ("certifications", Section::Other),
    ("certificates", Section::Other),
    ("licenses and certifications", Section::Other),
    ("certifications and licenses", Section::Other),
    ("awards", Section::Other),
    ("honors", Section::Other),
    ("honors and awards", Section::Other),
    ("awards and honors", Section::Other),
    ("achievements", Section::Other),
    ("leadership", Section::Other),
    ("activities", Section::Other),
    ("leadership and activities", Section::Other),
    ("extracurricular activities", Section::Other),
    ("involvement", Section::Other),
    ("volunteer", Section::Other),
    ("volunteering", Section::Other),
    ("volunteer experience", Section::Other),
    ("publications", Section::Other),
    ("interests", Section::Other),
    ("hobbies", Section::Other),
    ("languages", Section::Other),
    ("references", Section::Other),
];

const STATES: &[&str] = &[
    "AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "FL", "GA", "HI", "ID", "IL", "IN", "IA", "KS",
    "KY", "LA", "ME", "MD", "MA", "MI", "MN", "MS", "MO", "MT", "NE", "NV", "NH", "NJ", "NM", "NY",
    "NC", "ND", "OH", "OK", "OR", "PA", "RI", "SC", "SD", "TN", "TX", "UT", "VT", "VA", "WA", "WV",
    "WI", "WY", "DC", "PR",
];

const STATE_NAMES: &[&str] = &[
    "alabama",
    "alaska",
    "arizona",
    "arkansas",
    "california",
    "colorado",
    "connecticut",
    "delaware",
    "florida",
    "georgia",
    "hawaii",
    "idaho",
    "illinois",
    "indiana",
    "iowa",
    "kansas",
    "kentucky",
    "louisiana",
    "maine",
    "maryland",
    "massachusetts",
    "michigan",
    "minnesota",
    "mississippi",
    "missouri",
    "montana",
    "nebraska",
    "nevada",
    "new hampshire",
    "new jersey",
    "new mexico",
    "new york",
    "north carolina",
    "north dakota",
    "ohio",
    "oklahoma",
    "oregon",
    "pennsylvania",
    "rhode island",
    "south carolina",
    "south dakota",
    "tennessee",
    "texas",
    "utah",
    "vermont",
    "virginia",
    "washington",
    "west virginia",
    "wisconsin",
    "wyoming",
    "district of columbia",
    "puerto rico",
    "usa",
    "us",
    "united states",
];

fn is_state(s: &str) -> bool {
    let s = s.trim().trim_end_matches('.');
    STATES.contains(&s) || STATE_NAMES.contains(&s.to_lowercase().as_str())
}

/// Words that mark a line as a job title rather than a company name.
const TITLE_WORDS: &[&str] = &[
    "engineer",
    "developer",
    "programmer",
    "intern",
    "internship",
    "analyst",
    "manager",
    "tester",
    "assistant",
    "specialist",
    "consultant",
    "designer",
    "scientist",
    "lead",
    "director",
    "associate",
    "coordinator",
    "technician",
    "administrator",
    "architect",
    "researcher",
    "representative",
    "officer",
    "teacher",
    "tutor",
    "instructor",
    "fellow",
    "president",
    "founder",
    "co-founder",
    "volunteer",
    "qa",
    "sde",
    "swe",
    "head",
    "advisor",
    "strategist",
    "recruiter",
    "accountant",
    "editor",
    "writer",
    "nurse",
    "cashier",
    "server",
    "clerk",
    "supervisor",
    "owner",
    "member",
    "chair",
    "captain",
    "mentor",
    "trainee",
    "apprentice",
    "agent",
    "operator",
    "producer",
    "executive",
    "vp",
    "cto",
    "ceo",
    "staff",
    "principal",
    "senior",
    "junior",
    "sr",
    "jr",
    "student",
    "contractor",
    "freelancer",
    "barista",
    "associate",
];

const SCHOOL_WORDS: &[&str] = &[
    "university",
    "college",
    "institute",
    "school",
    "academy",
    "polytechnic",
    "community college",
    "universidad",
    "conservatory",
    "seminary",
];

static EMAIL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}").unwrap());
static PHONE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:\+?1[\s.-]?)?\(?\b\d{3}\)?[\s.-]?\d{3}[\s.-]?\d{4}\b").unwrap()
});
static URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(?:https?://)?(?:www\.)?(?:linkedin\.com/[^\s|,;()]+|github\.com/[^\s|,;()]+|[a-z0-9-]+(?:\.[a-z0-9-]+)*\.(?:com|io|dev|me|net|org|app|co|ai|site|page|tech|xyz)(?:/[^\s|,;()]*)?)",
    )
    .unwrap()
});
/// `City, ST` with an optional ZIP, at the end of a token.
static CITY_STATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:(?P<street>.*\d.*?),\s*)?(?P<city>[A-Za-z][A-Za-z .'-]{1,40}),\s*(?P<state>[A-Z]{2})\.?(?:\s+(?P<zip>\d{5})(?:-\d{4})?)?$").unwrap()
});
static ZIP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b\d{5}(?:-\d{4})?\b").unwrap());

const MONTH: &str = r"(?:jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|june?|july?|aug(?:ust)?|sept?(?:ember)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?)\.?";
const SEASON: &str = r"(?:spring|summer|fall|autumn|winter)";

fn point() -> String {
    format!(
        r"(?:{MONTH},?\s+'?\d{{2,4}}|{SEASON},?\s+\d{{4}}|\d{{1,2}}/\d{{4}}|\d{{1,2}}/\d{{2}}\b|\d{{4}}[-/.](?:1[0-2]|0?[1-9])\b|(?:19|20)\d{{2}})"
    )
}

static RANGE: LazyLock<Regex> = LazyLock::new(|| {
    let p = point();
    Regex::new(&format!(
        r"(?i)(?P<start>{p})\s*(?:-|–|—|−|to|→|⟶|~|until|thru|through)\s*(?P<end>{p}|present|current|now|today|ongoing)"
    ))
    .unwrap()
});
static SINGLE: LazyLock<Regex> = LazyLock::new(|| {
    let p = point();
    Regex::new(&format!(r"(?i)(?:(?P<expected>expected|anticipated|graduat(?:ed|ing|ion)|class of)\s*:?\s*)?(?P<date>{p})")).unwrap()
});
static GPA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:gpa|grade point average)\s*[:\-]?\s*(?P<a>\d\.\d{1,2})(?:\s*/\s*(?P<b>\d(?:\.\d{1,2})?))?|(?P<c>\d\.\d{1,2})\s*/\s*(?P<d>4(?:\.0{1,2})?)\b").unwrap()
});
static DEGREE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:bachelor|master|doctor|associate|ph\.?\s?d|m\.?b\.?a|b\.?\s?s\.?c?|m\.?\s?s\.?c?|b\.?\s?a\.?|m\.?\s?a\.?|b\.?\s?eng|m\.?\s?eng|b\.?f\.?a|m\.?f\.?a|j\.?d|m\.?d|diploma|certificate|a\.?a\.?s?|a\.?s\.?|high school)\b").unwrap()
});
static SPLIT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s+[|•·◦▪●]\s+|\s+[–—]\s+|\s+-\s+|\s*\|\s*|\t+|\s{3,}").unwrap());
static ITEM_SPLIT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s*[,;|•·●▪◦]\s*|\t+|\s{3,}").unwrap());
static LOCATION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:remote|hybrid|on-?site|(?:[A-Za-z][A-Za-z .'-]{1,40}),\s*(?:[A-Z]{2}|[A-Za-z ]{4,20})(?:\s*\((?:remote|hybrid|on-?site)\))?|remote\s*\(.*\)|.*\(remote\))$").unwrap()
});

/// Lowercase, `&` as "and", letters and spaces only. Letter-spaced headings
/// ("E X P E R I E N C E") are joined back up.
fn heading_key(line: &str) -> String {
    // "P R O F E S S I O N A L   E X P E R I E N C E": letters one space
    // apart, words three spaces apart.
    let spaced = line.split_whitespace().count() > 3
        && line.split_whitespace().all(|w| w.chars().count() == 1);
    let joined = if spaced {
        line.split("  ")
            .map(|w| w.split_whitespace().collect::<String>())
            .filter(|w| !w.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        line.to_owned()
    };
    joined
        .to_lowercase()
        .replace('&', " and ")
        .chars()
        .map(|c| if c.is_alphabetic() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn heading(line: &str) -> Option<Section> {
    let key = heading_key(line);
    if key.chars().count() > 40 {
        return None;
    }
    HEADINGS.iter().find(|(h, _)| *h == key).map(|(_, s)| *s)
}

fn bullet_marker(c: char) -> bool {
    matches!(
        c,
        '•' | '·'
            | '●'
            | '○'
            | '▪'
            | '■'
            | '◦'
            | '‣'
            | '⁃'
            | '–'
            | '—'
            | '-'
            | '*'
            | '>'
            | '✓'
            | '➢'
            | '►'
            | '▸'
            | '❖'
            | '◆'
            | '♦'
            | '→'
    ) || ('\u{E000}'..='\u{F8FF}').contains(&c)
}

/// The text of a bullet line without its marker.
fn bullet_text(line: &str) -> Option<String> {
    let mut chars = line.chars();
    let first = chars.next()?;
    if !bullet_marker(first) {
        return None;
    }
    let rest = chars.as_str().trim_start();
    // "-2024" or "--" aren't bullets.
    if rest.is_empty() || (first == '-' && !line[1..].starts_with(' ')) {
        return None;
    }
    Some(rest.to_owned())
}

fn sentence_like(s: &str) -> bool {
    let n = s.chars().count();
    n > 90 || (ends_sentence(s) && n > 45)
}

fn ends_sentence(s: &str) -> bool {
    s.trim_end().ends_with(['.', '!', '?', ';'])
}

fn starts_lower(s: &str) -> bool {
    s.chars()
        .next()
        .is_some_and(|c| c.is_lowercase() || c == '(' || c == '&' || c == ',')
}

fn has_date(s: &str) -> bool {
    RANGE.is_match(s)
        || SINGLE
            .find(s)
            .is_some_and(|m| !m.as_str().chars().all(|c| c.is_ascii_digit()) || s.trim().len() < 12)
}

fn month_number(word: &str) -> Option<u32> {
    let w = word.trim_end_matches('.').to_lowercase();
    let m = match w.get(..3)? {
        "jan" => 1,
        "feb" => 2,
        "mar" => 3,
        "apr" => 4,
        "may" => 5,
        "jun" => 6,
        "jul" => 7,
        "aug" => 8,
        "sep" => 9,
        "oct" => 10,
        "nov" => 11,
        "dec" => 12,
        "spr" => 3,
        "sum" => 6,
        "fal" | "aut" => 9,
        "win" => 12,
        _ => return None,
    };
    Some(m)
}

fn full_year(y: &str) -> Option<i32> {
    let y = y.trim_start_matches('\'');
    let n: i32 = y.parse().ok()?;
    match y.len() {
        4 if (1950..=2100).contains(&n) => Some(n),
        // '24 -> 2024; anything above next year's two digits is 19xx.
        2 => Some(if n <= 40 { 2000 + n } else { 1900 + n }),
        _ => None,
    }
}

/// One date point to `YYYY-MM`. A bare year is January for a start and
/// December for an end.
fn to_month(s: &str, is_end: bool) -> Option<String> {
    let s = s.trim().trim_end_matches(',');
    let lower = s.to_lowercase();
    if matches!(
        lower.as_str(),
        "present" | "current" | "now" | "today" | "ongoing"
    ) {
        return None;
    }
    let (y, m) = if let Some((a, b)) = s.split_once('/') {
        let (a, b) = (a.trim(), b.trim());
        if a.len() == 4 {
            (full_year(a)?, b.parse().ok()?)
        } else {
            (full_year(b)?, a.parse().ok()?)
        }
    } else if s.len() == 7 && (s.as_bytes()[4] == b'-' || s.as_bytes()[4] == b'.')
        || (s.len() == 6 && s.as_bytes()[4] == b'-')
    {
        (full_year(&s[..4])?, s[5..].parse().ok()?)
    } else {
        let parts: Vec<&str> = s
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|p| !p.is_empty())
            .collect();
        match parts.as_slice() {
            [y] => (full_year(y)?, if is_end { 12 } else { 1 }),
            [m, y] => (full_year(y)?, month_number(m)?),
            _ => return None,
        }
    };
    if !(1..=12).contains(&m) {
        return None;
    }
    Some(format!("{y:04}-{m:02}"))
}

/// Dates found in an entry: (start, end, is current, text with dates removed).
struct Dates {
    start: Option<String>,
    end: Option<String>,
    found: bool,
    /// The text, when a year alone was read as May ("Class of 2026").
    assumed_may: Option<String>,
}

/// Finds a range (or a single date) in the header lines and removes it.
fn take_dates(lines: &mut [String]) -> Dates {
    for line in lines.iter_mut() {
        if let Some(c) = RANGE.captures(line) {
            let start = to_month(&c["start"], false);
            let end = to_month(&c["end"], true);
            let whole = c.get(0).unwrap().range();
            line.replace_range(whole, " ");
            return Dates {
                start,
                end,
                found: true,
                assumed_may: None,
            };
        }
    }
    for line in lines.iter_mut() {
        if let Some(c) = SINGLE.captures(line) {
            let d = c.name("date").unwrap();
            // A lone four-digit number inside longer text might be anything.
            if d.as_str().chars().all(|ch| ch.is_ascii_digit())
                && line.trim().len() > 30
                && c.name("expected").is_none()
            {
                continue;
            }
            let bare_year = d.as_str().chars().all(|ch| ch.is_ascii_digit());
            // "Class of 2026", "Expected 2026": US degrees mostly finish in May.
            let may = bare_year && c.name("expected").is_some();
            let month = if may {
                Some(format!("{}-05", d.as_str()))
            } else {
                to_month(d.as_str(), true)
            };
            let assumed_may = may.then(|| c.get(0).unwrap().as_str().trim().to_owned());
            let whole = c.get(0).unwrap().range();
            line.replace_range(whole, " ");
            return Dates {
                start: month.clone(),
                end: month,
                found: true,
                assumed_may,
            };
        }
    }
    Dates {
        start: None,
        end: None,
        found: false,
        assumed_may: None,
    }
}

/// Splits header text into its pieces ("Title | Company", "Company — City").
fn pieces(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .flat_map(|l| SPLIT.split(l).map(str::to_owned).collect::<Vec<_>>())
        .map(|p| {
            p.trim_matches(|c: char| {
                c.is_whitespace() || matches!(c, ',' | '|' | '–' | '—' | '-' | '(' | ')' | ':')
            })
            .to_owned()
        })
        .filter(|p| !p.is_empty())
        .flat_map(|p| {
            // "Clemson University, Clemson, SC" -> school and location.
            let mut commas = p.rmatch_indices(',').map(|(i, _)| i);
            let (_, second) = (commas.next(), commas.next());
            match second {
                Some(i) if is_location(p[i + 1..].trim()) && !is_location(&p) => {
                    vec![p[..i].trim().to_owned(), p[i + 1..].trim().to_owned()]
                }
                _ => match split_trailing_city(&p) {
                    Some((rest, loc)) => vec![rest, loc],
                    None => vec![p],
                },
            }
        })
        .collect()
}

/// First words of multi-word US city names ("Fort Collins", "San Jose").
const CITY_PREFIXES: &[&str] = &[
    "fort",
    "ft.",
    "saint",
    "st.",
    "san",
    "santa",
    "los",
    "las",
    "new",
    "north",
    "south",
    "east",
    "west",
    "salt",
    "grand",
    "palo",
    "mountain",
    "kansas",
    "baton",
    "cedar",
    "coral",
    "ann",
    "long",
    "el",
    "jersey",
    "oklahoma",
    "sioux",
    "colorado",
    "green",
    "little",
    "bowling",
    "corpus",
    "des",
    "rock",
    "silver",
    "overland",
    "college",
    "virginia",
    "daly",
    "round",
    "sugar",
    "port",
    "lake",
    "bay",
    "rio",
    "mount",
    "mt.",
    "hot",
    "glen",
    "sandy",
    "cape",
    "eau",
    "elk",
    "royal",
    "lees",
    "chapel",
    "boca",
    "winter",
    "palm",
    "half",
    "cherry",
    "chula",
    "costa",
    "huntington",
    "thousand",
    "rancho",
    "redwood",
    "menlo",
    "foster",
    "union",
    "culver",
    "beverly",
    "newport",
    "playa",
    "del",
];
/// Last words of multi-word city names ("Salt Lake City", "Palm Springs").
const CITY_SUFFIXES: &[&str] = &[
    "city", "beach", "springs", "falls", "park", "heights", "hills", "island", "valley", "creek",
    "rapids", "lake", "harbor", "grove", "point", "village", "station",
];

/// "Fourth Coffee Denver, CO" (two columns joined by PDF text extraction)
/// -> ("Fourth Coffee", "Denver, CO"). How many words the city has comes
/// from common city name patterns; the review screen catches the rest.
fn split_trailing_city(p: &str) -> Option<(String, String)> {
    let (head, st) = p.rsplit_once(',')?;
    let st_clean = ZIP.replace_all(st.split('(').next().unwrap_or(""), "");
    if !is_state(&st_clean) {
        return None;
    }
    let words: Vec<&str> = head.split_whitespace().collect();
    if words.len() < 2 || words.iter().any(|w| w.chars().any(|c| c.is_ascii_digit())) {
        return None;
    }
    let lower = |w: &str| w.to_lowercase();
    let mut n = 1;
    if CITY_SUFFIXES.contains(&lower(words[words.len() - 1]).as_str()) && words.len() > n {
        n += 1;
    }
    while n < words.len()
        && n < 3
        && CITY_PREFIXES.contains(&lower(words[words.len() - n - 1]).as_str())
    {
        n += 1;
    }
    if n >= words.len() {
        return None;
    }
    let rest = words[..words.len() - n].join(" ");
    let city = words[words.len() - n..].join(" ");
    Some((rest, format!("{city},{st}").trim().to_owned()))
}

fn is_location(p: &str) -> bool {
    if !LOCATION.is_match(p) {
        return false;
    }
    let lower = p.to_lowercase();
    if lower.starts_with("remote")
        || lower.contains("(remote")
        || lower == "hybrid"
        || lower.starts_with("on")
    {
        return true;
    }
    // "City, ST" or "City, Statename", optionally with "(Remote)" or a ZIP.
    match p.rsplit_once(',') {
        Some((city, st)) => {
            let st = st.split('(').next().unwrap_or("");
            let st = ZIP.replace_all(st, "");
            is_state(&st)
                && city.split_whitespace().count() <= 4
                && !city.chars().any(|c| c.is_ascii_digit())
        }
        None => false,
    }
}

fn has_word(p: &str, words: &[&str]) -> bool {
    let lower = p.to_lowercase();
    let tokens: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric() && c != '-')
        .filter(|t| !t.is_empty())
        .collect();
    words.iter().any(|w| {
        if w.contains(' ') {
            lower.contains(w)
        } else {
            tokens.contains(w)
        }
    })
}

fn employment_type(text: &str) -> String {
    let lower = text.to_lowercase();
    for (needle, label) in [
        ("internship", "Internship"),
        ("intern", "Internship"),
        ("part-time", "Part-time"),
        ("part time", "Part-time"),
        ("contract", "Contract"),
        ("freelance", "Freelance"),
        ("full-time", "Full-time"),
        ("full time", "Full-time"),
    ] {
        if lower
            .split(|c: char| !c.is_alphanumeric() && c != '-')
            .any(|t| t == needle)
            || (needle.contains(' ') && lower.contains(needle))
        {
            return label.to_owned();
        }
    }
    String::new()
}

/// A group of lines: the non-bullet header lines, then bullets.
#[derive(Debug, Default)]
struct Entry {
    header: Vec<String>,
    bullets: Vec<String>,
    header_has_date: bool,
}

/// Splits a section's lines into entries.
fn entries(lines: &[&str]) -> Vec<Entry> {
    let mut out: Vec<Entry> = Vec::new();
    let mut cur = Entry::default();
    let mut last_was_bullet = false;
    for (i, raw) in lines.iter().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let next = lines[i + 1..]
            .iter()
            .map(|l| l.trim())
            .find(|l| !l.is_empty())
            .unwrap_or("");
        if let Some(b) = bullet_text(line) {
            cur.bullets.push(b);
            last_was_bullet = true;
            continue;
        }
        let dated = has_date(line);
        // A wrapped bullet line continues the bullet before it.
        if last_was_bullet && !dated {
            let prev = cur.bullets.last().map(String::as_str).unwrap_or("");
            let header_like = has_date(next) && line.chars().count() <= 70;
            if starts_lower(line)
                || (!ends_sentence(prev) && !header_like && line.chars().count() > 25)
            {
                let b = cur.bullets.last_mut().unwrap();
                b.push(' ');
                b.push_str(line);
                continue;
            }
        }
        // Resumes without bullet markers: a sentence under a header is a bullet.
        let short_sentence = cur.header_has_date
            && cur.header.len() >= 2
            && ends_sentence(line)
            && line.split_whitespace().count() >= 4;
        // A long line that wraps onto a lowercase line is a sentence too.
        let wraps = cur.header_has_date && line.chars().count() > 50 && starts_lower(next);
        if !cur.header.is_empty() && !dated && (sentence_like(line) || short_sentence || wraps) {
            cur.bullets.push(line.to_owned());
            last_was_bullet = true;
            continue;
        }
        // A header line after bullets, or a second dated header block, starts
        // a new entry.
        let starts_new =
            !cur.bullets.is_empty() || (cur.header_has_date && dated && cur.header.len() >= 2);
        if starts_new && (!cur.header.is_empty() || !cur.bullets.is_empty()) {
            out.push(std::mem::take(&mut cur));
        }
        cur.header_has_date |= dated;
        cur.header.push(line.to_owned());
        last_was_bullet = false;
    }
    if !cur.header.is_empty() || !cur.bullets.is_empty() {
        out.push(cur);
    }

    // Resumes without bullet markers: long sentence-like header lines are
    // really bullets.
    for e in &mut out {
        if e.bullets.is_empty() && e.header.len() > 2 {
            let keep = e
                .header
                .iter()
                .position(|l| sentence_like(l))
                .unwrap_or(e.header.len());
            let moved: Vec<String> = e.header.drain(keep..).collect();
            e.bullets = moved;
        }
    }
    out
}

/// One experience entry. Changes are recorded with paths inside the entry
/// (`title`).
fn experience(
    mut e: Entry,
    unsorted: &mut Vec<String>,
    changes: &mut Vec<Change>,
) -> Option<Experience> {
    let dates = take_dates(&mut e.header);
    let all = e.header.join(" ");
    let mut parts = pieces(&e.header);
    let location = parts
        .iter()
        .position(|p| is_location(p))
        .map(|i| parts.remove(i))
        .unwrap_or_default();
    // "Title at Company"
    if parts.len() == 1
        && let Some((t, c)) = parts[0].split_once(" at ")
    {
        parts = vec![t.trim().to_owned(), c.trim().to_owned()];
    }
    // The piece most like a title: one ending in a title word ("Business
    // Intelligence Assistant") beats one merely containing one ("Student
    // Consulting Group").
    let title_score = |p: &str| {
        let last = p
            .split(|c: char| !c.is_alphanumeric() && c != '-')
            .rfind(|w| !w.is_empty())
            .unwrap_or("")
            .to_lowercase();
        if TITLE_WORDS.contains(&last.as_str()) {
            2
        } else {
            u8::from(has_word(p, TITLE_WORDS))
        }
    };
    let title_idx = (0..parts.len())
        .filter(|&i| title_score(&parts[i]) > 0)
        .max_by_key(|&i| (title_score(&parts[i]), std::cmp::Reverse(i)));
    let (title, company) = match (title_idx, parts.len()) {
        (_, 0) => (String::new(), String::new()),
        (Some(t), _) => {
            let title = parts.remove(t);
            (
                title,
                if parts.is_empty() {
                    String::new()
                } else {
                    parts.remove(0)
                },
            )
        }
        // No title word: the more common layout is company first.
        (None, 1) => (String::new(), parts.remove(0)),
        (None, _) => {
            let company = parts.remove(0);
            (parts.remove(0), company)
        }
    };
    // "NIWC Atlantic - Clemson Capstone": pieces left over from the
    // company's own line are part of its name.
    let mut company = company;
    if !company.is_empty()
        && let Some(line) = e.header.iter().find(|l| l.contains(company.as_str()))
    {
        let mut i = 0;
        while i < parts.len() {
            if line.contains(parts[i].as_str())
                && (title.is_empty() || !line.contains(title.as_str()))
            {
                let piece = parts.remove(i);
                let (a, b) = (line.find(company.as_str()), line.find(piece.as_str()));
                company = if b < a {
                    format!("{piece} - {company}")
                } else {
                    format!("{company} - {piece}")
                };
            } else {
                i += 1;
            }
        }
    }
    if !parts.is_empty() {
        unsorted.push(format!(
            "Experience ({}): {}",
            if company.is_empty() { &title } else { &company },
            parts.join(" | ")
        ));
    }
    if title.is_empty() && company.is_empty() {
        if !e.bullets.is_empty() || !all.trim().is_empty() {
            unsorted.push(
                format!("Experience: {} {}", all.trim(), e.bullets.join(" "))
                    .trim()
                    .to_owned(),
            );
        }
        return None;
    }
    let (tidy_company, tidy_title) = (tidy_caps(&company), tidy_caps(&title));
    caps_change("company", &company, &tidy_company, changes);
    caps_change("title", &title, &tidy_title, changes);
    Some(Experience {
        employment_type: employment_type(&format!("{title} {all}")),
        company: tidy_company,
        title: tidy_title,
        location,
        start: dates.start,
        end: if dates.found { dates.end } else { None },
        bullets: e.bullets,
    })
}

/// One education entry; changes as in `experience`.
fn education(
    mut e: Entry,
    unsorted: &mut Vec<String>,
    changes: &mut Vec<Change>,
) -> Option<Education> {
    let mut gpa = String::new();
    for line in e.header.iter_mut().chain(e.bullets.iter_mut()) {
        if gpa.is_empty()
            && let Some(c) = GPA.captures(line)
        {
            let (a, b) = match (c.name("a"), c.name("c")) {
                (Some(a), _) => (a.as_str(), c.name("b").map(|m| m.as_str())),
                (None, Some(cc)) => (cc.as_str(), c.name("d").map(|m| m.as_str())),
                _ => continue,
            };
            gpa = match b {
                Some(b) => format!("{a}/{b}"),
                None => a.to_owned(),
            };
            let r = c.get(0).unwrap().range();
            line.replace_range(r, " ");
        }
    }
    e.bullets.retain(|b| {
        !b.trim()
            .trim_matches(|c: char| !c.is_alphanumeric())
            .is_empty()
    });
    let dates = take_dates(&mut e.header);
    let mut parts = pieces(&e.header);
    let location = parts
        .iter()
        .position(|p| is_location(p))
        .map(|i| parts.remove(i))
        .unwrap_or_default();
    let school = parts
        .iter()
        .position(|p| has_word(p, SCHOOL_WORDS))
        .or(if parts.is_empty() { None } else { Some(0) })
        .map(|i| parts.remove(i))
        .unwrap_or_default();
    let (mut degree, mut field, mut minor) = (String::new(), String::new(), String::new());
    // The degree line as the file had it, to record (and undo) the split.
    let mut degree_raw = String::new();
    // The degree line is taken whole from the header, as `pieces` would
    // split it at the comma before the minor.
    if let Some(i) = e
        .header
        .iter()
        .position(|l| DEGREE.is_match(l) && !has_word(l, SCHOOL_WORDS))
    {
        degree_raw = e.header[i].split_whitespace().collect::<Vec<_>>().join(" ");
        (degree, field, minor) = degree_line(&e.header[i]);
        let words = degree_words(&e.header[i]);
        parts.retain(|p| !degree_words(p).iter().all(|w| words.contains(w)));
    } else if let Some(i) = parts.iter().position(|p| DEGREE.is_match(p)) {
        degree_raw = parts.remove(i);
        (degree, field, minor) = degree_line(&degree_raw);
    }
    // "Major: X" / "Minor: Y" lines.
    let mut details: Vec<String> = Vec::new();
    for p in parts {
        let lower = p.to_lowercase();
        let value = || {
            p.split_once(':')
                .map(|(_, v)| v.trim().to_owned())
                .unwrap_or_else(|| p.clone())
        };
        if field.is_empty() && (lower.starts_with("major") || lower.starts_with("concentration")) {
            field = value();
        } else if minor.is_empty() && lower.starts_with("minor") {
            minor = value()
                .trim_start_matches(|c: char| c.is_whitespace())
                .trim_start_matches("in ")
                .trim_start_matches("Minor ")
                .to_owned();
        } else {
            details.push(p);
        }
    }
    details.extend(e.bullets.iter().map(|b| coursework_line(b)));
    if school.is_empty() {
        if !details.is_empty() {
            unsorted.push(format!("Education: {}", details.join(" | ")));
        }
        return None;
    }
    let (field, minor) = (tidy_caps(&field), tidy_caps(&minor));
    let tidy_school = tidy_caps(&school);
    caps_change("school", &school, &tidy_school, changes);
    if !degree_raw.is_empty() && (!field.is_empty() || !minor.is_empty() || degree != degree_raw) {
        let after = [
            degree.clone(),
            field.clone(),
            if minor.is_empty() {
                String::new()
            } else {
                format!("Minor in {minor}")
            },
        ]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
        let mut c = Change::new("degree", ChangeKind::DegreeSplit, &degree_raw, &after);
        c.undo = [
            ("degree", degree_raw.as_str()),
            ("field_of_study", ""),
            ("minor", ""),
        ]
        .iter()
        .map(|(f, v)| super::resume_content::FieldValue {
            field: (*f).to_owned(),
            value: (*v).to_owned(),
        })
        .collect();
        changes.push(c);
    }
    if let (Some(text), Some(end)) = (&dates.assumed_may, &dates.end) {
        changes.push(Change::new("end", ChangeKind::ClassOfMay, text, end));
    }
    Some(Education {
        school: tidy_school,
        degree,
        field_of_study: field,
        minor,
        gpa,
        location,
        start: if dates.start != dates.end {
            dates.start
        } else {
            None
        },
        end: dates.end,
        details: details.join("\n"),
    })
}

/// A user name shown for a link ("klam101"): one word found inside one of
/// the file's link addresses.
fn is_link_text(token: &str, links: &[String]) -> bool {
    let t = token.trim().to_lowercase();
    t.len() >= 3
        && !t.contains(char::is_whitespace)
        && links.iter().any(|l| l.to_lowercase().contains(&t))
}

fn is_link_label(s: &str) -> bool {
    super::clean::LINK_LABELS.contains(&s.trim().to_lowercase().as_str())
}

/// A project, and whether its header ended in a link label ("GitHub",
/// "Demo") whose address is only in the file's links.
fn project(
    mut e: Entry,
    unsorted: &mut Vec<String>,
    changes: &mut Vec<Change>,
) -> Option<(Project, bool)> {
    let dates = take_dates(&mut e.header);
    let mut url = String::new();
    for line in e.header.iter_mut() {
        if url.is_empty()
            && let Some(m) = URL.find(line)
            && let Ok(u) = normalize_url(m.as_str())
        {
            url = u;
            let r = m.range();
            line.replace_range(r, " ");
        }
    }
    // "Sunset | GitHub | Demo" -> "Sunset", labeled.
    let mut labeled = false;
    let first_raw = e
        .header
        .first()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .unwrap_or_default();
    if let Some(first) = e.header.first_mut() {
        loop {
            let trimmed = first.trim_end_matches(|c: char| {
                c.is_whitespace() || matches!(c, '|' | '-' | '–' | '—' | ',' | '·' | '•')
            });
            match trimmed.rsplit_once(char::is_whitespace) {
                // Only after a separator or a wide gap (where an icon was),
                // so a project called "Personal Website" keeps its name.
                Some((head, last))
                    if is_link_label(last)
                        && (head.ends_with(char::is_whitespace)
                            || head
                                .trim_end()
                                .ends_with(['|', '-', '–', '—', '·', '•', ','])) =>
                {
                    labeled = true;
                    *first = head.to_owned();
                }
                _ => {
                    *first = trimmed.to_owned();
                    break;
                }
            }
        }
    }
    let mut parts = pieces(&e.header);
    parts.retain(|p| {
        let label = is_link_label(p);
        labeled |= label;
        !label
    });
    if parts.is_empty() {
        if !e.bullets.is_empty() {
            unsorted.push(format!("Projects: {}", e.bullets.join(" ")));
        }
        return None;
    }
    let name = parts.remove(0);
    let mut skills: Vec<String> = Vec::new();
    let mut role = String::new();
    let mut summary = String::new();
    for p in parts {
        let lower = p.to_lowercase();
        let labeled_list = lower
            .strip_prefix("technologies")
            .or_else(|| lower.strip_prefix("tech stack"))
            .or_else(|| lower.strip_prefix("tools"))
            .or_else(|| lower.strip_prefix("built with"))
            .map(|_| {
                p.split_once(':')
                    .map(|(_, v)| v.to_owned())
                    .unwrap_or_default()
            });
        if let Some(list) = labeled_list {
            skills.extend(split_items(&list));
        } else if p.contains(',') {
            skills.extend(split_items(&p));
        } else if role.is_empty() && has_word(&p, TITLE_WORDS) {
            role = p;
        } else if summary.is_empty() {
            // "ONLINE BANKING PLATFORM": the line under the name.
            summary = tidy_caps(&p);
            caps_change("summary", &p, &summary, changes);
        } else if role.is_empty() {
            role = p;
        } else {
            skills.extend(split_items(&p));
        }
    }
    // A "Technologies: ..." bullet is the stack, not an accomplishment.
    e.bullets.retain(|b| {
        let lower = b.to_lowercase();
        for prefix in [
            "technologies:",
            "tech stack:",
            "tools:",
            "built with:",
            "stack:",
        ] {
            if let Some(rest) = lower.strip_prefix(prefix) {
                let start = b.len() - rest.len();
                skills.extend(split_items(&b[start..]));
                return false;
            }
        }
        true
    });
    let tidy_name = tidy_caps(&name);
    if labeled && !first_raw.is_empty() && first_raw != name {
        changes.push(Change::new("name", ChangeKind::LinkLabel, &first_raw, &tidy_name).undoable());
    } else {
        caps_change("name", &name, &tidy_name, changes);
    }
    Some((
        Project {
            name: tidy_name,
            summary,
            role,
            url,
            start: dates.start,
            end: if dates.found { dates.end } else { None },
            bullets: e.bullets,
            skills,
        },
        labeled,
    ))
}

fn split_items(s: &str) -> Vec<String> {
    ITEM_SPLIT
        .split(s)
        .map(|t| {
            t.trim_matches(|c: char| c.is_whitespace() || matches!(c, '.' | ':' | '-' | '–'))
                .to_owned()
        })
        .filter(|t| !t.is_empty() && t.chars().count() <= 50 && t.split_whitespace().count() <= 5)
        .collect()
}

/// Skill group names, longest first so "Programming Languages" wins over
/// "Languages".
const SKILL_CATEGORIES: &[&str] = &[
    "programming languages",
    "frameworks and libraries",
    "frameworks & libraries",
    "libraries and frameworks",
    "tools and technologies",
    "tools & technologies",
    "developer tools",
    "operating systems",
    "soft skills",
    "methodologies",
    "certifications",
    "technologies",
    "frameworks",
    "libraries",
    "databases",
    "platforms",
    "languages",
    "software",
    "concepts",
    "testing",
    "devops",
    "cloud",
    "tools",
    "other",
];

/// A skills line whose label is in its own column, without a colon
/// ("Languages C/C++, C#, Python"): a known group name at the start, or
/// extra leading words on the first item ("AI-Assisted Development Ollama,
/// Claude, Cursor" -> "AI-Assisted Development").
fn category_without_colon(line: &str) -> (String, String) {
    if !line.contains(',') {
        return (String::new(), line.to_owned());
    }
    let lower = line.to_lowercase();
    for c in SKILL_CATEGORIES {
        if let Some(rest) = lower.strip_prefix(c)
            && rest.starts_with(char::is_whitespace)
        {
            let at = line.len() - rest.len();
            return (line[..at].trim().to_owned(), line[at..].trim().to_owned());
        }
    }
    let items: Vec<&str> = line.split(',').map(str::trim).collect();
    let mut others: Vec<usize> = items[1..]
        .iter()
        .map(|i| i.split_whitespace().count())
        .collect();
    others.sort_unstable();
    let median = others.get(others.len() / 2).copied().unwrap_or(1).max(1);
    let first: Vec<&str> = items[0].split_whitespace().collect();
    if others.len() >= 2 && first.len() >= median + 2 {
        let cut = first.len() - median;
        let category = first[..cut].join(" ");
        let comma = line.find(',').unwrap_or(line.len());
        let rest = format!("{}{}", first[cut..].join(" "), &line[comma..]);
        return (category, rest);
    }
    (String::new(), line.to_owned())
}

fn skills(lines: &[&str]) -> Vec<Skill> {
    let mut out = Vec::new();
    for raw in lines {
        let line = bullet_text(raw.trim()).unwrap_or_else(|| raw.trim().to_owned());
        if line.is_empty() {
            continue;
        }
        // "Languages: Rust, Python" -> category "Languages".
        let (category, items) = match line.split_once(':') {
            Some((label, rest))
                if label.split_whitespace().count() <= 4 && !label.contains(',') =>
            {
                (label.trim().to_owned(), rest.to_owned())
            }
            _ => category_without_colon(&line),
        };
        for name in split_items(&items) {
            out.push(Skill {
                name,
                category: category.clone(),
            });
        }
    }
    out
}

/// Name, contact details, links and a headline from the lines above the
/// first section.
fn header(lines: &[&str], links: &[String], profile: &mut Profile, unsorted: &mut Vec<String>) {
    // A link wrapped onto the next line ("linkedin.com/in/jane-" + "doe").
    let mut joined: Vec<String> = Vec::new();
    for raw in lines {
        let line = raw.trim();
        if let Some(prev) = joined.last_mut()
            && (prev.ends_with('-') || prev.ends_with('/'))
            && prev
                .split_whitespace()
                .last()
                .is_some_and(|t| t.contains(".com/") || t.contains("://"))
            && !line.is_empty()
            && !line.contains(' ')
        {
            prev.push_str(line);
            continue;
        }
        joined.push(line.to_owned());
    }
    let p = &mut profile.personal;
    let mut leftovers = Vec::new();
    for raw in &joined {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let mut rest = line.to_owned();
        if p.email.is_empty()
            && let Some(m) = EMAIL.find(&rest)
        {
            p.email = m.as_str().to_owned();
        }
        rest = EMAIL.replace_all(&rest, " ").into_owned();
        for m in URL.find_iter(&rest.clone()) {
            if let Ok(url) = normalize_url(m.as_str())
                && !profile
                    .links
                    .iter()
                    .any(|l| l.url.eq_ignore_ascii_case(&url))
            {
                profile.links.push(Link {
                    kind: link_kind(&url),
                    url,
                });
            }
        }
        rest = URL.replace_all(&rest, " ").into_owned();
        if p.phone.is_empty()
            && let Some(m) = PHONE.find(&rest)
        {
            p.phone = m.as_str().trim().to_owned();
        }
        rest = PHONE.replace_all(&rest, " ").into_owned();

        for token in SPLIT.split(&rest).map(|t| {
            t.trim_matches(|c: char| c.is_whitespace() || matches!(c, ',' | '|' | '•' | '·'))
        }) {
            // "portfolio", "GitHub": a link's label; the address comes from
            // the file's links.
            if token.is_empty()
                || token.chars().all(|c| !c.is_alphanumeric())
                || is_link_label(token)
                || is_link_text(token, links)
            {
                continue;
            }
            if p.city.is_empty()
                && let Some(c) = CITY_STATE.captures(token)
                && STATES.contains(&&c["state"])
            {
                p.city = c["city"].trim().to_owned();
                p.region = c["state"].to_owned();
                p.postal_code = c
                    .name("zip")
                    .map(|m| m.as_str().to_owned())
                    .unwrap_or_default();
                p.street = c
                    .name("street")
                    .map(|m| m.as_str().trim().to_owned())
                    .unwrap_or_default();
                p.country = "United States".into();
                continue;
            }
            leftovers.push(token.to_owned());
        }
    }
    // The name: the first leftover of 2 to 4 capitalized words without digits.
    if let Some(i) = leftovers.iter().position(|t| {
        let name = t.split(',').next().unwrap_or(t);
        let words: Vec<&str> = name.split_whitespace().collect();
        (2..=4).contains(&words.len())
            && !t.chars().any(|c| c.is_ascii_digit())
            && words
                .iter()
                .all(|w| w.chars().next().is_some_and(char::is_uppercase))
    }) {
        let full = leftovers.remove(i);
        // "Jane Doe, PMP" -> "Jane Doe"
        let full = full.split(',').next().unwrap_or(&full).trim().to_owned();
        let mut words = full.split_whitespace();
        let first = words.next().unwrap_or_default();
        p.first_name = title_case(first);
        p.last_name = words.map(title_case).collect::<Vec<_>>().join(" ");
    }
    // A street address on its own ("907 Main Street") next to a ZIP.
    if p.street.is_empty()
        && let Some(i) = leftovers.iter().position(|t| {
            t.chars().next().is_some_and(|c| c.is_ascii_digit())
                && t.split_whitespace().count() >= 3
                && t.chars().any(char::is_alphabetic)
        })
    {
        let t = leftovers.remove(i);
        if let Some(z) = ZIP.find(&t)
            && p.postal_code.is_empty()
        {
            p.postal_code = z.as_str().to_owned();
        }
        p.street = ZIP
            .replace_all(&t, "")
            .trim_matches(|c: char| c.is_whitespace() || c == ',')
            .to_owned();
    }
    // A short line under the name is usually a headline ("QA Engineer").
    if let Some(i) = leftovers.iter().position(|t| {
        (1..=6).contains(&t.split_whitespace().count()) && !t.chars().any(|c| c.is_ascii_digit())
    }) {
        p.headline = leftovers.remove(i);
    }
    for t in leftovers {
        unsorted.push(format!("Contact: {t}"));
    }
}

/// Words written in lowercase inside a title ("Bachelor of Science").
const SMALL_WORDS: &[&str] = &[
    "of", "in", "and", "the", "for", "at", "to", "a", "an", "with", "on", "or", "by", "&", "via",
];

/// Short everyday words, title-cased even though most words of three
/// letters or fewer in an all-caps title are acronyms ("MAP" -> "Map").
const SHORT_WORDS: &[&str] = &[
    "app", "map", "web", "art", "new", "big", "bot", "car", "day", "job", "key", "lab", "law",
    "pay", "sea", "sun", "top", "use", "way", "toy", "tax", "tea", "pet", "box", "hub", "kit",
    "net", "pro", "tag", "cup", "gym", "bar", "bus", "fun", "end", "all", "one", "two", "six",
    "ten", "our", "old", "red", "my", "go", "run", "air", "low", "sky", "bay", "aid", "fit",
];

/// Words kept in capitals.
const ACRONYMS: &[&str] = &[
    "GPA", "QA", "QE", "AI", "ML", "UI", "UX", "IT", "MVP", "AWS", "GCP", "SQL", "API", "LLC",
    "LLP", "CEO", "CTO", "CFO", "COO", "VP", "HR", "USA", "US", "U.S.", "NASA", "IBM", "MBA",
    "PHD", "B.S.", "B.A.", "M.S.", "M.A.", "BS", "BA", "MS", "MA", "II", "III", "IV", "R&D",
    "SDET", "SRE", "DBA", "ERP", "CRM", "GIS", "IOS", "SAAS", "POS", "STEM", "NLP", "CS", "CE",
    "EE", "ECE", "DEVOPS", "CI/CD", "ETL", "BI", "PM", "TA", "RA",
];

/// "QA ENGINEER" -> "QA Engineer": all-caps text in title case. Small words
/// are lowercase except at the start, known acronyms stay in capitals, and
/// other words of three letters or fewer stay as they are (likely acronyms).
/// Text that already has lowercase letters is kept.
fn tidy_caps(s: &str) -> String {
    if s.chars().any(char::is_lowercase) || s.chars().filter(|c| c.is_alphabetic()).count() <= 4 {
        return s.to_owned();
    }
    s.split(' ')
        .enumerate()
        .map(|(i, w)| {
            let upper = w.to_uppercase();
            let bare = upper.trim_matches(|c: char| matches!(c, ',' | ';' | ':' | '(' | ')'));
            if ACRONYMS.contains(&bare) {
                match bare {
                    "IOS" => w.replace("IOS", "iOS"),
                    "SAAS" => w.replace("SAAS", "SaaS"),
                    "PHD" => w.replace("PHD", "PhD"),
                    "DEVOPS" => w.replace("DEVOPS", "DevOps"),
                    _ => w.to_owned(),
                }
            } else if i > 0 && SMALL_WORDS.contains(&w.to_lowercase().as_str()) {
                w.to_lowercase()
            } else if w.chars().filter(|c| c.is_alphabetic()).count() <= 3
                && !SHORT_WORDS.contains(&w.to_lowercase().as_str())
            {
                w.to_owned()
            } else {
                // "AI-ASSISTED" -> "AI-Assisted", "FULL-STACK" -> "Full-Stack"
                w.split('-').map(title_case).collect::<Vec<_>>().join("-")
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

static MINOR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:^|[,;|]\s*|\s+)(?:(?:with\s+)?an?\s+)?minors?\s*(?:in|:|-)?\s+(?P<a>[^,;|]+)|(?:^|[,;|]\s*)(?P<b>[^,;|]+?)\s+minor\b").unwrap()
});

/// Splits "Bachelor of Science in Computer Science, Minor Business
/// Administration" into degree, field and minor.
fn degree_line(line: &str) -> (String, String, String) {
    let mut d = tidy_caps(line.trim());
    let mut minor = String::new();
    if let Some(c) = MINOR.captures(&d) {
        minor = c
            .name("a")
            .or(c.name("b"))
            .map(|m| m.as_str().trim().to_owned())
            .unwrap_or_default();
        let r = c.get(0).unwrap().range();
        d.replace_range(r, "");
    }
    let d = d
        .trim()
        .trim_end_matches([',', ';', '|', '-'])
        .trim()
        .to_owned();
    let lower = d.to_lowercase();
    // "Bachelor of Science in Computer Science", "B.S. in X", "B.S., X"
    let split = lower
        .find(" in ")
        .map(|i| (i, 4))
        .or_else(|| lower.find(", ").map(|i| (i, 2)));
    let (degree, field) = if let Some((i, n)) = split {
        (d[..i].trim().to_owned(), d[i + n..].trim().to_owned())
    } else if let Some(m) = DEGREE.find(&d)
        && m.start() == 0
        && d.len() > m.end() + 2
        && !lower.starts_with("bachelor")
        && !lower.starts_with("master")
        && !lower.starts_with("associate")
        && !lower.starts_with("doctor")
    {
        // "B.S. Computer Science": the regex stops before a final period.
        let end = m.end() + usize::from(d[m.end()..].starts_with('.'));
        (
            d[..end].trim().to_owned(),
            d[end..]
                .trim_matches(|c: char| c.is_whitespace() || c == ',' || c == '.')
                .to_owned(),
        )
    } else {
        (d, String::new())
    };
    (degree, field, minor)
}

/// Lowercase words of a piece of text, to tell which pieces came from a line.
fn degree_words(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

/// "Coursework - A, B" and "Relevant Coursework: A, B" -> "Coursework: A, B".
fn coursework_line(line: &str) -> String {
    let lower = line.to_lowercase();
    for label in [
        "relevant coursework",
        "related coursework",
        "coursework",
        "courses",
    ] {
        if let Some(rest) = lower.strip_prefix(label) {
            let rest = &line[line.len() - rest.len()..];
            let rest = rest.trim_start_matches(|c: char| {
                c.is_whitespace() || matches!(c, ':' | '-' | '–' | '—')
            });
            if !rest.is_empty() {
                let label = &line[..label.len()];
                return format!("{label}: {rest}");
            }
        }
    }
    line.to_owned()
}

/// "JANE" -> "Jane"; mixed case is kept ("McDonald").
fn title_case(w: &str) -> String {
    if w.chars()
        .filter(|c| c.is_alphabetic())
        .all(char::is_uppercase)
        && w.chars().count() > 1
    {
        let mut c = w.chars();
        let first = c.next().unwrap();
        first
            .to_uppercase()
            .chain(c.as_str().to_lowercase().chars())
            .collect()
    } else {
        w.to_owned()
    }
}

/// Parses resume text. `links` are the targets of the file's own links
/// (`resume::extract`), used for addresses the text only labels.
pub fn parse(text: &str, links: &[String]) -> Parsed {
    let (text, fixes) = super::clean::clean_with_report(text);
    let lines: Vec<&str> = text.lines().collect();
    let mut sections: Vec<(Section, String, Vec<&str>)> =
        vec![(Section::Header, String::new(), Vec::new())];
    for line in &lines {
        match heading(line.trim()) {
            Some(s) => sections.push((s, line.trim().to_owned(), Vec::new())),
            None => sections.last_mut().unwrap().2.push(line),
        }
    }

    let mut parsed = Parsed::default();
    let Parsed {
        profile,
        unsorted,
        sections: order,
        changes,
    } = &mut parsed;
    let seen_section = |s: SectionKind, order: &mut Vec<SectionKind>| {
        if !order.contains(&s) {
            order.push(s);
        }
    };
    let mut coursework: Vec<String> = Vec::new();
    let mut labeled_projects: Vec<bool> = Vec::new();
    for (section, title, body) in sections {
        match section {
            Section::Header => header(&body, links, profile, unsorted),
            Section::Summary => {
                let text = body
                    .iter()
                    .map(|l| bullet_text(l.trim()).unwrap_or_else(|| l.trim().to_owned()))
                    .filter(|l| !l.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");
                if !text.is_empty() {
                    if !profile.personal.summary.is_empty() {
                        profile.personal.summary.push('\n');
                    }
                    profile.personal.summary.push_str(&text);
                }
            }
            Section::Experience => {
                seen_section(SectionKind::Experience, order);
                for e in entries(&body) {
                    let mut c = Vec::new();
                    if let Some(x) = experience(e, unsorted, &mut c) {
                        let at = format!("experiences.{}", profile.experiences.len());
                        changes.extend(c.into_iter().map(|c| c.under(&at)));
                        profile.experiences.push(x);
                    }
                }
            }
            Section::Education => {
                seen_section(SectionKind::Education, order);
                for e in entries(&body) {
                    let mut c = Vec::new();
                    if let Some(x) = education(e, unsorted, &mut c) {
                        let at = format!("educations.{}", profile.educations.len());
                        changes.extend(c.into_iter().map(|c| c.under(&at)));
                        profile.educations.push(x);
                    }
                }
            }
            Section::Projects => {
                seen_section(SectionKind::Projects, order);
                for e in entries(&body) {
                    let mut c = Vec::new();
                    if let Some((x, labeled)) = project(e, unsorted, &mut c) {
                        let at = format!("projects.{}", profile.projects.len());
                        changes.extend(c.into_iter().map(|c| c.under(&at)));
                        profile.projects.push(x);
                        labeled_projects.push(labeled);
                    }
                }
            }
            Section::Skills => {
                seen_section(SectionKind::Skills, order);
                profile.skills.extend(skills(&body));
            }
            Section::Coursework => coursework.extend(
                body.iter()
                    .map(|l| l.trim().to_owned())
                    .filter(|l| !l.is_empty()),
            ),
            Section::Other => {
                let text: Vec<String> = body
                    .iter()
                    .map(|l| l.trim().to_owned())
                    .filter(|l| !l.is_empty())
                    .collect();
                if !text.is_empty() {
                    unsorted.push(format!("{title}: {}", text.join(" | ")));
                }
            }
        }
    }
    if !coursework.is_empty() {
        let line = format!("Relevant coursework: {}", coursework.join(", "));
        match profile.educations.first_mut() {
            Some(e) => {
                if !e.details.is_empty() {
                    e.details.push('\n');
                }
                e.details.push_str(&line);
            }
            None => unsorted.push(line),
        }
    }
    place_links(profile, &labeled_projects, links, unsorted, changes);
    // Duplicate skills (case-insensitive) keep the first.
    let mut seen = std::collections::HashSet::new();
    profile
        .skills
        .retain(|s| seen.insert(s.name.to_lowercase()));
    attach_fixes(profile, &fixes, changes);
    parsed
}

/// Fields with text, by path, for matching cleanup fixes to them.
fn fields(p: &Profile) -> Vec<(String, &str, bool)> {
    // (path, value, is a bullet or other long text)
    let mut out: Vec<(String, &str, bool)> = Vec::new();
    for (i, l) in p.links.iter().enumerate() {
        out.push((format!("header.links.{i}.url"), l.url.as_str(), false));
    }
    for (i, x) in p.experiences.iter().enumerate() {
        out.push((format!("experiences.{i}.title"), x.title.as_str(), false));
        out.push((
            format!("experiences.{i}.company"),
            x.company.as_str(),
            false,
        ));
        for (j, b) in x.bullets.iter().enumerate() {
            out.push((format!("experiences.{i}.bullets.{j}"), b.as_str(), true));
        }
    }
    for (i, e) in p.educations.iter().enumerate() {
        out.push((format!("educations.{i}.school"), e.school.as_str(), false));
        out.push((format!("educations.{i}.degree"), e.degree.as_str(), false));
        out.push((
            format!("educations.{i}.field_of_study"),
            e.field_of_study.as_str(),
            false,
        ));
        out.push((format!("educations.{i}.minor"), e.minor.as_str(), false));
    }
    for (i, x) in p.projects.iter().enumerate() {
        out.push((format!("projects.{i}.name"), x.name.as_str(), false));
        out.push((format!("projects.{i}.summary"), x.summary.as_str(), false));
        for (j, b) in x.bullets.iter().enumerate() {
            out.push((format!("projects.{i}.bullets.{j}"), b.as_str(), true));
        }
    }
    out
}

/// Turns the cleanup's line fixes into changes on the fields they touched.
/// A field that already has a change (a capitals fix, say) gets no second
/// one for the same text.
fn attach_fixes(profile: &Profile, fixes: &[Fix], changes: &mut Vec<Change>) {
    let lower = |s: &str| {
        s.to_lowercase()
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_start_matches("www.")
            .to_owned()
    };
    let fields = fields(profile);
    for fix in fixes {
        for word in &fix.words {
            let w = lower(word);
            for (path, value, long) in &fields {
                let v = lower(value);
                let hit = match fix.kind {
                    // Whole words in titles and names; bullets would match
                    // every "quality" in a sentence.
                    FixKind::SmallCaps => {
                        !long && v.split(|c: char| !c.is_alphanumeric()).any(|t| t == w)
                    }
                    FixKind::JoinedWord => v.contains(&w),
                    FixKind::IconLink => v == w || v.ends_with(&w),
                };
                // The entry a path is in: "educations.0" of "educations.0.minor".
                let entry = |f: &str| f.splitn(3, '.').take(2).collect::<Vec<_>>().join(".");
                let covered = changes.iter().any(|c| {
                    &c.field == path
                        || (fix.kind == FixKind::SmallCaps && entry(&c.field) == entry(path))
                });
                if !hit || covered {
                    continue;
                }
                let kind = match fix.kind {
                    FixKind::SmallCaps => ChangeKind::SmallCaps,
                    FixKind::JoinedWord => ChangeKind::JoinedWord,
                    FixKind::IconLink => ChangeKind::IconLink,
                };
                changes.push(Change::new(path.clone(), kind, &fix.before, &fix.after));
            }
        }
    }
}

/// A link without its scheme, `www.` and trailing slash, to compare links.
fn link_key(url: &str) -> String {
    let lower = url.trim().to_lowercase();
    let rest = lower.split_once("://").map_or(lower.as_str(), |(_, r)| r);
    rest.trim_start_matches("www.")
        .trim_end_matches('/')
        .to_owned()
}

/// Letters and digits only, to match a repository to a project name.
fn squash(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Puts the file's own links where they belong. They come in reading order,
/// so links before the first project link are in the header.
///
/// - `mailto:` and `tel:` fill the email and phone when the text had none.
/// - LinkedIn profiles and GitHub accounts become profile links.
/// - A link whose last path part matches a project's name is that project's.
/// - Other header links: the first is the portfolio.
/// - The rest go, in order, to projects whose header showed a link label
///   ("GitHub", "Demo"); anything left over is listed as not placed.
fn place_links(
    profile: &mut Profile,
    labeled_projects: &[bool],
    links: &[String],
    unsorted: &mut Vec<String>,
    changes: &mut Vec<Change>,
) {
    // Links the file's text spelled out; anything added after came only from
    // the file's link annotations.
    let in_text = profile.links.len();
    let from_file = |field: String, url: &str| {
        let mut c = Change::new(field.clone(), ChangeKind::LinkFromFile, "", url);
        c.undo = vec![super::resume_content::FieldValue {
            field,
            value: String::new(),
        }];
        c
    };
    let mut in_header = true;
    let mut leftover: Vec<String> = Vec::new();
    for raw in links {
        let lower = raw.to_lowercase();
        if let Some(addr) = lower.strip_prefix("mailto:") {
            let addr = addr.split('?').next().unwrap_or("");
            if profile.personal.email.is_empty() && EMAIL.is_match(addr) {
                profile.personal.email = raw[7..7 + addr.len()].to_owned();
            }
            continue;
        }
        if let Some(num) = lower.strip_prefix("tel:") {
            if profile.personal.phone.is_empty() && PHONE.is_match(num) {
                profile.personal.phone = raw[4..].to_owned();
            }
            continue;
        }
        let Ok(url) = normalize_url(raw) else {
            continue;
        };
        let key = link_key(&url);
        // Already found in the text: keep the file's exact address.
        if let Some(l) = profile.links.iter_mut().find(|l| link_key(&l.url) == key) {
            l.url = url;
            continue;
        }
        if profile.projects.iter().any(|p| link_key(&p.url) == key) {
            continue;
        }
        let path: Vec<&str> = key.split('/').skip(1).filter(|p| !p.is_empty()).collect();
        let host = key.split('/').next().unwrap_or("");
        let kind = link_kind(&url);
        let account = match kind {
            LinkKind::Linkedin => path.first() == Some(&"in") && path.len() == 2,
            LinkKind::Github => path.len() == 1,
            _ => false,
        };
        if account {
            if !profile.links.iter().any(|l| l.kind == kind) {
                profile.links.push(Link { kind, url });
            }
            continue;
        }
        // By name: github.com/user/sunset -> the project "Sunset".
        let last = path.last().map_or_else(
            || squash(host.split('.').next().unwrap_or("")),
            |p| squash(p),
        );
        if let Some(i) = profile
            .projects
            .iter()
            .position(|p| p.url.is_empty() && !last.is_empty() && squash(&p.name) == last)
        {
            changes.push(from_file(format!("projects.{i}.url"), &url));
            profile.projects[i].url = url;
            in_header = false;
            continue;
        }
        if kind == LinkKind::Github {
            in_header = false;
        }
        if in_header && !profile.links.iter().any(|l| l.kind == LinkKind::Portfolio) {
            profile.links.push(Link {
                kind: LinkKind::Portfolio,
                url,
            });
            continue;
        }
        leftover.push(url);
    }
    let mut leftover = leftover.into_iter();
    for (i, (p, labeled)) in profile
        .projects
        .iter_mut()
        .zip(labeled_projects)
        .enumerate()
    {
        if *labeled && p.url.is_empty() {
            match leftover.next() {
                Some(url) => {
                    changes.push(from_file(format!("projects.{i}.url"), &url));
                    p.url = url;
                }
                None => break,
            }
        }
    }
    for url in leftover {
        unsorted.push(format!("Link: {url}"));
    }
    for (i, l) in profile.links.iter().enumerate().skip(in_text) {
        changes.push(from_file(format!("header.links.{i}.url"), &l.url));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_points() {
        assert_eq!(to_month("Jan 2024", false).as_deref(), Some("2024-01"));
        assert_eq!(
            to_month("September 2021", false).as_deref(),
            Some("2021-09")
        );
        assert_eq!(to_month("Sept. 2021", false).as_deref(), Some("2021-09"));
        assert_eq!(to_month("08/2022", false).as_deref(), Some("2022-08"));
        assert_eq!(to_month("2022-08", false).as_deref(), Some("2022-08"));
        assert_eq!(to_month("2022", false).as_deref(), Some("2022-01"));
        assert_eq!(to_month("2022", true).as_deref(), Some("2022-12"));
        assert_eq!(to_month("Summer 2025", false).as_deref(), Some("2025-06"));
        assert_eq!(to_month("May '24", false).as_deref(), Some("2024-05"));
        assert_eq!(to_month("Present", true), None);
    }

    #[test]
    fn ranges() {
        for (text, start, end) in [
            ("Jan 2024 – Present", Some("2024-01"), None),
            ("January 2024 - Dec 2025", Some("2024-01"), Some("2025-12")),
            ("01/2024 – 05/2025", Some("2024-01"), Some("2025-05")),
            ("2022-08 ⟶ 2026-05", Some("2022-08"), Some("2026-05")),
            ("2022 – 2024", Some("2022-01"), Some("2024-12")),
            ("Jun 2023 to Aug 2023", Some("2023-06"), Some("2023-08")),
        ] {
            let mut lines = vec![format!("Acme Corp {text}")];
            let d = take_dates(&mut lines);
            assert!(d.found, "{text}");
            assert_eq!(d.start.as_deref(), start, "{text}");
            assert_eq!(d.end.as_deref(), end, "{text}");
            assert_eq!(lines[0].trim(), "Acme Corp", "{text}");
        }
    }

    #[test]
    fn headings_and_bullets() {
        assert_eq!(heading("WORK EXPERIENCE"), Some(Section::Experience));
        assert_eq!(heading("Skills & Interests:"), Some(Section::Skills));
        assert_eq!(heading("E X P E R I E N C E"), Some(Section::Experience));
        assert_eq!(heading("Education Coordinator"), None);
        assert_eq!(
            bullet_text("• Built things").as_deref(),
            Some("Built things")
        );
        assert_eq!(
            bullet_text("- Built things").as_deref(),
            Some("Built things")
        );
        assert_eq!(
            bullet_text("\u{F0B7} Built things").as_deref(),
            Some("Built things")
        );
        assert_eq!(bullet_text("-2024"), None);
        assert_eq!(bullet_text("Built things"), None);
    }
}
