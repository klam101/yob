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
//! can show it. The draft is never saved without the user reviewing it.

use std::sync::LazyLock;

use regex::Regex;

use super::{Education, Experience, Link, Profile, Project, Skill, link_kind, normalize_url};

#[derive(Debug, Clone, Default)]
pub struct Parsed {
    pub profile: Profile,
    /// Lines the parser couldn't place, grouped by where they came from.
    pub unsorted: Vec<String>,
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
            let month = to_month(d.as_str(), true);
            let whole = c.get(0).unwrap().range();
            line.replace_range(whole, " ");
            return Dates {
                start: month.clone(),
                end: month,
                found: true,
            };
        }
    }
    Dates {
        start: None,
        end: None,
        found: false,
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
        if !cur.header.is_empty() && !dated && (sentence_like(line) || short_sentence) {
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

fn experience(mut e: Entry, unsorted: &mut Vec<String>) -> Option<Experience> {
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
    let title_idx = parts.iter().position(|p| has_word(p, TITLE_WORDS));
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
    Some(Experience {
        employment_type: employment_type(&format!("{title} {all}")),
        company: tidy_caps(&company),
        title: tidy_caps(&title),
        location,
        start: dates.start,
        end: if dates.found { dates.end } else { None },
        bullets: e.bullets,
    })
}

fn education(mut e: Entry, unsorted: &mut Vec<String>) -> Option<Education> {
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
    let (mut degree, mut field) = (String::new(), String::new());
    let mut minor = None;
    if let Some(i) = parts.iter().position(|p| DEGREE.is_match(p)) {
        let mut d = parts.remove(i);
        // ", Minor in Mathematics" is a detail, not the field.
        if let Some(at) = d.to_lowercase().find("minor") {
            let cut = d[..at]
                .trim_end()
                .trim_end_matches([',', ';', '|'])
                .trim_end()
                .len();
            minor = Some(d[at..].trim().to_owned());
            d.truncate(cut);
        }
        // "Bachelor of Science in Computer Science", "B.S. Computer Science",
        // "B.S., Computer Science"
        if let Some((a, b)) = d.split_once(" in ").or_else(|| d.split_once(", ")) {
            degree = a.trim().to_owned();
            field = b.trim().to_owned();
        } else if let Some(m) = DEGREE.find(&d)
            && m.start() == 0
            && d.len() > m.end() + 2
            && !d[..m.end()].to_lowercase().starts_with("bachelor")
            && !d[..m.end()].to_lowercase().starts_with("master")
        {
            // The regex stops before a final period ("B.S" of "B.S.").
            let end = m.end() + usize::from(d[m.end()..].starts_with('.'));
            degree = d[..end].trim().to_owned();
            field = d[end..]
                .trim_matches(|c: char| c.is_whitespace() || c == ',' || c == '.')
                .to_owned();
        } else {
            degree = d;
        }
    }
    // "Major: X" / "Minor: Y" lines.
    let mut details: Vec<String> = minor.into_iter().collect();
    for p in parts {
        let lower = p.to_lowercase();
        if field.is_empty() && (lower.starts_with("major") || lower.starts_with("concentration")) {
            field = p
                .split_once(':')
                .map(|(_, v)| v.trim().to_owned())
                .unwrap_or(p);
        } else {
            details.push(p);
        }
    }
    details.extend(e.bullets);
    if school.is_empty() {
        if !details.is_empty() {
            unsorted.push(format!("Education: {}", details.join(" | ")));
        }
        return None;
    }
    Some(Education {
        school: tidy_caps(&school),
        degree,
        field_of_study: field,
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

fn project(mut e: Entry, unsorted: &mut Vec<String>) -> Option<Project> {
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
    let mut parts = pieces(&e.header);
    if parts.is_empty() {
        if !e.bullets.is_empty() {
            unsorted.push(format!("Projects: {}", e.bullets.join(" ")));
        }
        return None;
    }
    let name = parts.remove(0);
    let mut skills: Vec<String> = Vec::new();
    let mut role = String::new();
    for p in parts {
        let lower = p.to_lowercase();
        let list = lower
            .strip_prefix("technologies")
            .or_else(|| lower.strip_prefix("tech stack"))
            .or_else(|| lower.strip_prefix("tools"))
            .or_else(|| lower.strip_prefix("built with"))
            .map(|_| {
                p.split_once(':')
                    .map(|(_, v)| v.to_owned())
                    .unwrap_or_default()
            })
            .unwrap_or_else(|| p.clone());
        if list.contains(',') || !role.is_empty() {
            skills.extend(split_items(&list));
        } else {
            role = p;
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
    Some(Project {
        name,
        role,
        url,
        start: dates.start,
        end: if dates.found { dates.end } else { None },
        bullets: e.bullets,
        skills,
    })
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
                (label.trim().to_owned(), rest)
            }
            _ => (String::new(), line.as_str()),
        };
        for name in split_items(items) {
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
fn header(lines: &[&str], profile: &mut Profile, unsorted: &mut Vec<String>) {
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
            if token.is_empty() || token.chars().all(|c| !c.is_alphanumeric()) {
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

/// "QA ENGINEER" -> "QA Engineer": all-caps text in title case, keeping
/// short words that are likely acronyms (QA, IT, LLC). Mixed case is kept.
fn tidy_caps(s: &str) -> String {
    if s.chars().any(char::is_lowercase) || s.chars().filter(|c| c.is_alphabetic()).count() <= 4 {
        return s.to_owned();
    }
    s.split(' ')
        .map(|w| {
            if w.chars().filter(|c| c.is_alphabetic()).count() <= 3 {
                w.to_owned()
            } else {
                title_case(w)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
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

pub fn parse(text: &str) -> Parsed {
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
    let Parsed { profile, unsorted } = &mut parsed;
    let mut coursework: Vec<String> = Vec::new();
    for (section, title, body) in sections {
        match section {
            Section::Header => header(&body, profile, unsorted),
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
                for e in entries(&body) {
                    if let Some(x) = experience(e, unsorted) {
                        profile.experiences.push(x);
                    }
                }
            }
            Section::Education => {
                for e in entries(&body) {
                    // "Relevant Coursework: ..." inside Education.
                    if let Some(x) = education(e, unsorted) {
                        profile.educations.push(x);
                    }
                }
            }
            Section::Projects => {
                for e in entries(&body) {
                    if let Some(x) = project(e, unsorted) {
                        profile.projects.push(x);
                    }
                }
            }
            Section::Skills => profile.skills.extend(skills(&body)),
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
    // Duplicate skills (case-insensitive) keep the first.
    let mut seen = std::collections::HashSet::new();
    profile
        .skills
        .retain(|s| seen.insert(s.name.to_lowercase()));
    parsed
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
