//! What people type into the feed's filters, turned into match rules.
//! Pure functions, so `store` only has to bind the results.
//!
//! - Locations: "Austin, TX", "Texas", "NY", "Remote" or any text. A state
//!   matches by name or code, so "Austin, TX" also finds "Austin, Texas".
//! - Titles: every word of a title must appear as a whole word, in any
//!   order, so "software engineer" finds "Engineer, Software".
//! - Excluded title words: whole words, so "intern" doesn't hide "Internal".

/// US states and DC: (code, name).
const STATES: &[(&str, &str)] = &[
    ("AL", "Alabama"),
    ("AK", "Alaska"),
    ("AZ", "Arizona"),
    ("AR", "Arkansas"),
    ("CA", "California"),
    ("CO", "Colorado"),
    ("CT", "Connecticut"),
    ("DE", "Delaware"),
    ("DC", "District of Columbia"),
    ("FL", "Florida"),
    ("GA", "Georgia"),
    ("HI", "Hawaii"),
    ("ID", "Idaho"),
    ("IL", "Illinois"),
    ("IN", "Indiana"),
    ("IA", "Iowa"),
    ("KS", "Kansas"),
    ("KY", "Kentucky"),
    ("LA", "Louisiana"),
    ("ME", "Maine"),
    ("MD", "Maryland"),
    ("MA", "Massachusetts"),
    ("MI", "Michigan"),
    ("MN", "Minnesota"),
    ("MS", "Mississippi"),
    ("MO", "Missouri"),
    ("MT", "Montana"),
    ("NE", "Nebraska"),
    ("NV", "Nevada"),
    ("NH", "New Hampshire"),
    ("NJ", "New Jersey"),
    ("NM", "New Mexico"),
    ("NY", "New York"),
    ("NC", "North Carolina"),
    ("ND", "North Dakota"),
    ("OH", "Ohio"),
    ("OK", "Oklahoma"),
    ("OR", "Oregon"),
    ("PA", "Pennsylvania"),
    ("RI", "Rhode Island"),
    ("SC", "South Carolina"),
    ("SD", "South Dakota"),
    ("TN", "Tennessee"),
    ("TX", "Texas"),
    ("UT", "Utah"),
    ("VT", "Vermont"),
    ("VA", "Virginia"),
    ("WA", "Washington"),
    ("WV", "West Virginia"),
    ("WI", "Wisconsin"),
    ("WY", "Wyoming"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct State {
    pub code: &'static str,
    pub name: &'static str,
}

impl State {
    /// "TX" or "Texas", any case.
    pub fn find(s: &str) -> Option<Self> {
        let s = s.trim().trim_end_matches('.');
        STATES
            .iter()
            .find(|(code, name)| code.eq_ignore_ascii_case(s) || name.eq_ignore_ascii_case(s))
            .map(|&(code, name)| Self { code, name })
    }

    /// Postgres regex (case-sensitive) for the code as its own token, so
    /// "TX" matches "Austin, TX" and "US-TX-Austin" but not "Textron".
    pub fn code_pattern(&self) -> String {
        format!("(^|[^A-Za-z]){}([^A-Za-z]|$)", self.code)
    }
}

/// One place from the Location filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    /// Remote jobs.
    Remote,
    /// Anywhere in a state.
    State(State),
    /// A city or any other text, optionally with a state.
    Text { text: String, state: Option<State> },
}

/// The Location filter holds several places separated by `;`
/// ("Austin, TX; Denver, CO; Remote"). At most 20 are used.
pub fn places(raw: &str) -> Vec<Place> {
    raw.split(';').filter_map(place).take(20).collect()
}

pub fn place(entry: &str) -> Option<Place> {
    let entry = entry.split_whitespace().collect::<Vec<_>>().join(" ");
    if entry.is_empty() {
        return None;
    }
    let lower = entry.to_lowercase();
    if matches!(
        lower.as_str(),
        "remote" | "anywhere" | "remote us" | "remote, us" | "remote - us"
    ) {
        return Some(Place::Remote);
    }
    if let Some(state) = State::find(&entry) {
        return Some(Place::State(state));
    }
    // "Austin, TX", "Austin, Texas", "Austin, TX, USA".
    let parts: Vec<&str> = entry.split(',').map(str::trim).collect();
    if parts.len() >= 2
        && !parts[0].is_empty()
        && let Some(state) = State::find(parts[1])
    {
        return Some(Place::Text {
            text: parts[0].to_owned(),
            state: Some(state),
        });
    }
    Some(Place::Text {
        text: entry,
        state: None,
    })
}

/// Words that don't decide whether a title fits.
const TITLE_STOPWORDS: &[&str] = &[
    "and", "or", "of", "the", "a", "an", "for", "in", "at", "to", "&", "-", "/", "|",
];

/// The words of a title, lowercased: "Sr. Software Engineer (Backend)" ->
/// ["sr", "software", "engineer", "backend"]. Keeps `+`, `#` and `.` inside
/// words, for "C++", "C#" and ".NET".
pub fn title_words(title: &str) -> Vec<String> {
    title
        .to_lowercase()
        .split(|c: char| !(c.is_alphanumeric() || matches!(c, '+' | '#' | '.')))
        .map(|w| w.trim_matches('.'))
        .filter(|w| !w.is_empty() && !TITLE_STOPWORDS.contains(w))
        .map(str::to_owned)
        .collect()
}

/// Escapes a literal for a Postgres regular expression. In Postgres, a
/// backslash before a letter or digit starts a class escape, so only other
/// characters are escaped.
fn pg_regex_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if !c.is_alphanumeric() && c != ' ' {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Case-insensitive Postgres regex (`~*`) for a word or phrase as whole
/// words, with an optional plural "s": "engineer" matches "Engineers" but
/// "intern" doesn't match "Internal".
pub fn whole_word_pattern(words: &[String]) -> String {
    let alternatives: Vec<String> = words
        .iter()
        .map(|w| w.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|w| !w.is_empty())
        .map(|w| pg_regex_escape(&w).replace(' ', "[^a-z0-9]+"))
        .collect();
    format!("(^|[^a-z0-9])({})s?([^a-z0-9]|$)", alternatives.join("|"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_from_text() {
        let tx = State::find("tx").unwrap();
        assert_eq!(tx.name, "Texas");
        assert_eq!(place("Remote"), Some(Place::Remote));
        assert_eq!(place(" Texas "), Some(Place::State(tx)));
        assert_eq!(
            place("Austin, TX"),
            Some(Place::Text {
                text: "Austin".into(),
                state: Some(tx)
            })
        );
        assert_eq!(
            place("Austin, Texas, USA"),
            Some(Place::Text {
                text: "Austin".into(),
                state: Some(tx)
            })
        );
        assert_eq!(
            place("Bay Area"),
            Some(Place::Text {
                text: "Bay Area".into(),
                state: None
            })
        );
        assert_eq!(place("  "), None);
        assert_eq!(places("Austin, TX; ; Remote").len(), 2);
    }

    #[test]
    fn words_and_patterns() {
        assert_eq!(
            title_words("Sr. Software Engineer (Backend) & C++"),
            ["sr", "software", "engineer", "backend", "c++"]
        );
        assert_eq!(title_words(".NET Developer"), ["net", "developer"]);
        assert_eq!(
            whole_word_pattern(&["c++".into(), "account  executive".into()]),
            r"(^|[^a-z0-9])(c\+\+|account[^a-z0-9]+executive)s?([^a-z0-9]|$)"
        );
        assert_eq!(
            State::find("NY").unwrap().code_pattern(),
            "(^|[^A-Za-z])NY([^A-Za-z]|$)"
        );
    }
}
