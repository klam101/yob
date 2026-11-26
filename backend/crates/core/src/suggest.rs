//! Type-ahead suggestions for profile and resume fields: cities, schools,
//! companies and job titles. The browser shows a few at a time and asks
//! again as the user types, so lists of any size work.
//!
//! - Cities and schools are fixed lists shipped with the app (`data/`, built
//!   from US government data; see `data/README.md`), searched in memory.
//! - Companies and job titles come from our own job data. The worker rebuilds
//!   them into `suggest_terms` (`rebuild`), searched with the trigram index.
//!
//! Matching is the same everywhere: the text's start first, then every typed
//! word starting a word ("sof eng" -> "Software Engineer"), then the text
//! anywhere; ties go to the more common entry. When too few match, close
//! spellings fill in ("clemsn" -> "Clemson University").

use std::{collections::HashMap, sync::LazyLock};

use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SuggestKind {
    /// "City, ST", plus "Remote".
    Location,
    School,
    Company,
    Title,
    /// The job feed's search box: job titles (with related ones, "coder" ->
    /// "Software Developer"), then companies. Each says which it is.
    Search,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Suggestion {
    pub value: String,
    /// A second line, like a school's "Clemson, SC" or a company's "42 open
    /// jobs".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// What it is, when a list mixes kinds (`search`: title or company).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<SuggestKind>,
}

impl Suggestion {
    fn new(value: String, detail: Option<String>) -> Self {
        Self {
            value,
            detail,
            kind: None,
        }
    }
}

/// At most this many come back.
pub const MAX_LIMIT: usize = 20;

/// Lowercase words of letters and digits, apostrophes dropped ("Bachelor's"
/// -> "bachelors"), everything else a space.
pub fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c == '\'' || c == '’' {
            continue;
        }
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.ends_with(' ') && !out.is_empty() {
            out.push(' ');
        }
    }
    out.trim_end().to_owned()
}

/// How well `entry` (folded) matches the folded query: 0 starts with it,
/// 1 every typed word starts a word, 2 contains it. None: no match.
fn rank(entry: &str, query: &str, tokens: &[&str]) -> Option<u8> {
    if entry.starts_with(query) {
        Some(0)
    } else if tokens
        .iter()
        .all(|t| entry.split(' ').any(|w| w.starts_with(t)))
    {
        Some(1)
    } else if entry.contains(query) {
        Some(2)
    } else {
        None
    }
}

/// Trigram similarity (like Postgres' `pg_trgm`), for close spellings.
fn similarity(a: &str, b: &str) -> f32 {
    let grams = |s: &str| -> std::collections::HashSet<[char; 3]> {
        let mut set = std::collections::HashSet::new();
        for w in s.split(' ').filter(|w| !w.is_empty()) {
            let cs: Vec<char> = format!("  {w} ").chars().collect();
            for g in cs.windows(3) {
                set.insert([g[0], g[1], g[2]]);
            }
        }
        set
    };
    let (a, b) = (grams(a), grams(b));
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let common = a.intersection(&b).count() as f32;
    common / (a.len() + b.len()) as f32 * 2.0
}

// --- fixed lists ----------------------------------------------------------------

struct Entry {
    value: String,
    detail: Option<String>,
    /// Folded value plus any aliases ("ucla").
    search: String,
    weight: i64,
}

/// Splits one CSV line (quotes allowed, no line breaks inside a field).
fn csv_line(line: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                chars.next();
                out.last_mut().unwrap().push('"');
            }
            '"' => quoted = !quoted,
            ',' if !quoted => out.push(String::new()),
            c => out.last_mut().unwrap().push(c),
        }
    }
    out
}

fn rows(csv: &str) -> impl Iterator<Item = Vec<String>> + '_ {
    csv.lines()
        .skip(1)
        .filter(|l| !l.trim().is_empty())
        .map(csv_line)
}

static PLACES: LazyLock<Vec<Entry>> = LazyLock::new(|| {
    let mut out = vec![Entry {
        value: "Remote".into(),
        detail: None,
        search: "remote".into(),
        weight: i64::MAX,
    }];
    for r in rows(include_str!("../data/us_places.csv")) {
        let [name, state, pop] = &r[..] else { continue };
        let value = format!("{name}, {state}");
        out.push(Entry {
            search: fold(&value),
            value,
            detail: None,
            weight: pop.parse().unwrap_or(0),
        });
    }
    out
});

static SCHOOLS: LazyLock<Vec<Entry>> = LazyLock::new(|| {
    rows(include_str!("../data/us_schools.csv"))
        .filter_map(|r| {
            let [name, city, state, alias, rank] = &r[..] else {
                return None;
            };
            Some(Entry {
                search: fold(&format!("{name} {alias}")),
                value: name.clone(),
                detail: Some(format!("{city}, {state}")),
                weight: rank.parse().unwrap_or(0),
            })
        })
        .collect()
});

fn search_list(list: &[Entry], query: &str, limit: usize) -> Vec<Suggestion> {
    let q = fold(query);
    let take = |e: &Entry| Suggestion::new(e.value.clone(), e.detail.clone());
    if q.is_empty() {
        let mut all: Vec<&Entry> = list.iter().collect();
        all.sort_by_key(|e| std::cmp::Reverse(e.weight));
        return all.into_iter().take(limit).map(take).collect();
    }
    let tokens: Vec<&str> = q.split(' ').collect();
    let mut hits: Vec<(u8, &Entry)> = list
        .iter()
        .filter_map(|e| rank(&e.search, &q, &tokens).map(|r| (r, e)))
        .collect();
    hits.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(b.1.weight.cmp(&a.1.weight))
            .then(a.1.value.len().cmp(&b.1.value.len()))
    });
    let mut out: Vec<Suggestion> = hits.iter().take(limit).map(|(_, e)| take(e)).collect();
    // Too few: close spellings.
    if out.len() < limit && q.chars().count() >= 4 {
        let mut close: Vec<(f32, &Entry)> = list
            .iter()
            .filter(|e| {
                !out.iter()
                    .any(|s| s.value == e.value && s.detail == e.detail)
            })
            .map(|e| (similarity(&e.search, &q), e))
            .filter(|(s, _)| *s >= 0.35)
            .collect();
        close.sort_by(|a, b| b.0.total_cmp(&a.0).then(b.1.weight.cmp(&a.1.weight)));
        out.extend(
            close
                .into_iter()
                .take(limit - out.len())
                .map(|(_, e)| take(e)),
        );
    }
    out
}

/// The detail of the list entry named `value` (the biggest one when two
/// campuses share a name).
fn detail_for(list: &[Entry], value: &str) -> Option<String> {
    let f = fold(value);
    list.iter()
        .filter(|e| fold(&e.value) == f)
        .max_by_key(|e| e.weight)
        .and_then(|e| e.detail.clone())
}

// --- terms from the job data -------------------------------------------------------

/// Values of one kind in `suggest_terms` matching `query`, with their weight
/// (open jobs), best first.
async fn search_terms(
    pool: &PgPool,
    kind: &str,
    query: &str,
    limit: usize,
) -> sqlx::Result<Vec<(String, i32)>> {
    let q = fold(query);
    let limit = limit as i64;
    if limit == 0 {
        return Ok(Vec::new());
    }
    let rows: Vec<(String, i32)> = if q.is_empty() {
        sqlx::query_as(
            "SELECT value, weight FROM suggest_terms WHERE kind = $1 ORDER BY weight DESC, value LIMIT $2",
        )
        .bind(kind)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        let tokens: Vec<&str> = q.split(' ').collect();
        let longest = tokens.iter().max_by_key(|t| t.len()).copied().unwrap_or("");
        // Every typed word starts a word; the longest one's `LIKE` uses the
        // trigram index. Values starting with the text come first.
        let mut found: Vec<(String, i32)> = sqlx::query_as(
            "SELECT value, weight FROM suggest_terms
             WHERE kind = $1
               AND search LIKE '%' || $2 || '%'
               AND (SELECT bool_and(search LIKE '% ' || t || '%') FROM unnest($3::text[]) t)
             ORDER BY (search LIKE ' ' || $4 || '%') DESC, weight DESC, length(value)
             LIMIT $5",
        )
        .bind(kind)
        .bind(longest)
        .bind(&tokens)
        .bind(&q)
        .bind(limit)
        .fetch_all(pool)
        .await?;
        if (found.len() as i64) < limit && q.len() >= 4 {
            let close: Vec<(String, i32)> = sqlx::query_as(
                "SELECT value, weight FROM suggest_terms
                 WHERE kind = $1 AND search % $2
                 ORDER BY similarity(search, $2) DESC, weight DESC
                 LIMIT $3",
            )
            .bind(kind)
            .bind(&q)
            .bind(limit)
            .fetch_all(pool)
            .await?;
            for c in close {
                if (found.len() as i64) < limit && !found.iter().any(|f| f.0 == c.0) {
                    found.push(c);
                }
            }
        }
        found
    };
    Ok(rows)
}

fn plain(rows: Vec<(String, i32)>) -> Vec<Suggestion> {
    rows.into_iter()
        .map(|(v, _)| Suggestion::new(v, None))
        .collect()
}

// --- related titles ----------------------------------------------------------------

/// Words people search with, and the job titles they mean. Rule-based for
/// now: "coder" finds "Software Developer". (Matching by meaning with
/// embeddings comes with the matching stage.)
const RELATED: &[(&[&str], &[&str])] = &[
    (
        &[
            "coder",
            "programmer",
            "developer",
            "dev",
            "swe",
            "software dev",
            "computer programmer",
        ],
        &["software engineer", "software developer"],
    ),
    (
        &["sde"],
        &["software development engineer", "software engineer"],
    ),
    (
        &["tester", "qa", "quality"],
        &[
            "qa engineer",
            "quality assurance engineer",
            "test engineer",
            "software test engineer",
        ],
    ),
    (
        &["frontend", "front end", "front-end"],
        &[
            "frontend engineer",
            "front end engineer",
            "front end developer",
        ],
    ),
    (
        &["backend", "back end", "back-end"],
        &["backend engineer", "back end engineer", "backend developer"],
    ),
    (
        &["fullstack", "full stack"],
        &[
            "full stack engineer",
            "full stack developer",
            "full stack software engineer",
        ],
    ),
    (
        &["ml", "machine learning", "ai"],
        &[
            "machine learning engineer",
            "ai engineer",
            "applied scientist",
        ],
    ),
    (
        &["devops", "sre", "reliability"],
        &[
            "site reliability engineer",
            "devops engineer",
            "platform engineer",
        ],
    ),
    (
        &["pm"],
        &["product manager", "project manager", "program manager"],
    ),
    (
        &["ux", "ui"],
        &["product designer", "ux designer", "ui ux designer"],
    ),
    (
        &["data"],
        &["data analyst", "data scientist", "data engineer"],
    ),
    (&["rn"], &["registered nurse"]),
    (&["cna"], &["certified nursing assistant"]),
    (
        &["cpa"],
        &["accountant", "staff accountant", "senior accountant"],
    ),
    (
        &["it", "helpdesk", "help desk"],
        &[
            "it support specialist",
            "help desk technician",
            "it technician",
        ],
    ),
    (
        &["intern"],
        &["software engineer intern", "software engineering intern"],
    ),
];

/// Titles related to what's typed (folded), most related first. A key
/// matches when the text is the start of it ("prog" -> "programmer") or the
/// text has it as a word ("senior swe").
pub fn related_titles(query: &str) -> Vec<&'static str> {
    let q = fold(query);
    if q.chars().count() < 2 {
        return Vec::new();
    }
    let words: Vec<&str> = q.split(' ').collect();
    let mut out: Vec<&'static str> = Vec::new();
    for (keys, titles) in RELATED {
        let hit = keys.iter().any(|k| {
            let k = fold(k);
            (q.chars().count() >= 3 && k.starts_with(&q)) || k == q || words.contains(&k.as_str())
        });
        if hit {
            for t in *titles {
                if !out.contains(t) {
                    out.push(t);
                }
            }
        }
    }
    out
}

/// The feed's search box: titles (direct matches, then related ones), then
/// companies with their open-job counts.
async fn search_box(pool: &PgPool, query: &str) -> sqlx::Result<Vec<Suggestion>> {
    const TITLES: usize = 6;
    const COMPANIES: usize = 3;
    if fold(query).is_empty() {
        return Ok(Vec::new());
    }
    // Related titles that exist in the job data, at most 3 of the 6 slots.
    let wanted: Vec<String> = related_titles(query)
        .iter()
        .map(|t| format!(" {}", fold(t)))
        .collect();
    let related: Vec<(String, i32)> = if wanted.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as(
            "SELECT value, weight FROM suggest_terms
             WHERE kind = 'title' AND search = ANY($1)
             ORDER BY array_position($1, search)
             LIMIT 3",
        )
        .bind(&wanted)
        .fetch_all(pool)
        .await?
    };
    let direct = search_terms(pool, "title", query, TITLES).await?;
    // Related titles the direct matches don't already include keep their
    // slots at the end; direct matches fill the rest.
    let shown: std::collections::HashSet<String> = direct.iter().map(|(v, _)| fold(v)).collect();
    let related: Vec<String> = related
        .into_iter()
        .map(|(v, _)| v)
        .filter(|v| !shown.contains(&fold(v)))
        .collect();
    let mut out: Vec<Suggestion> = direct
        .into_iter()
        .take(TITLES - related.len())
        .map(|(v, _)| Suggestion {
            kind: Some(SuggestKind::Title),
            ..Suggestion::new(v, None)
        })
        .collect();
    out.extend(related.into_iter().map(|v| Suggestion {
        kind: Some(SuggestKind::Title),
        ..Suggestion::new(v, Some("Related".into()))
    }));
    for (v, n) in search_terms(pool, "company", query, COMPANIES).await? {
        out.push(Suggestion {
            kind: Some(SuggestKind::Company),
            ..Suggestion::new(v, Some(open_jobs(n)))
        });
    }
    Ok(out)
}

/// "1 open job", "1,234 open jobs".
fn open_jobs(n: i32) -> String {
    let digits = n.max(0).to_string();
    let mut with_commas = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            with_commas.push(',');
        }
        with_commas.push(c);
    }
    format!("{with_commas} open job{}", if n == 1 { "" } else { "s" })
}

/// Suggestions for `query`, best first. `own` are the user's own values for
/// this kind of field (from their profile); matching ones come first, so a
/// company we don't track still shows up.
pub async fn suggest(
    pool: &PgPool,
    kind: SuggestKind,
    query: &str,
    limit: usize,
    own: &[String],
) -> sqlx::Result<Vec<Suggestion>> {
    if kind == SuggestKind::Search {
        return search_box(pool, query).await;
    }
    let limit = limit.clamp(1, MAX_LIMIT);
    let q = fold(query);
    let tokens: Vec<&str> = q.split(' ').filter(|t| !t.is_empty()).collect();
    let mut out: Vec<Suggestion> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for v in own {
        let f = fold(v);
        if !f.is_empty() && (q.is_empty() || rank(&f, &q, &tokens).is_some()) && seen.insert(f) {
            // A school from the list keeps its city ("Clemson, SC"), so it
            // shows under the name and fills the location when picked.
            let detail = match kind {
                SuggestKind::School => detail_for(&SCHOOLS, v),
                _ => None,
            };
            out.push(Suggestion::new(v.clone(), detail));
        }
    }
    out.truncate(3.min(limit));
    let rest = match kind {
        SuggestKind::Location => search_list(&PLACES, query, limit),
        SuggestKind::School => search_list(&SCHOOLS, query, limit),
        SuggestKind::Company => plain(search_terms(pool, "company", query, limit).await?),
        SuggestKind::Title => plain(search_terms(pool, "title", query, limit).await?),
        SuggestKind::Search => unreachable!("handled above"),
    };
    for s in rest {
        if out.len() >= limit {
            break;
        }
        if seen.insert(fold(&s.value)) {
            out.push(s);
        }
    }
    Ok(out)
}

// --- rebuilding from the job data ------------------------------------------------------

/// Titles seen fewer times than this are too rare to suggest.
const MIN_TITLE_COUNT: i64 = 3;

static PARENS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*[(\[]([^)\]]*)[)\]]").unwrap());
static NOISE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(?:remote|hybrid|on-?site|in-?office|united states|usa?|u\.s|full[- ]time|part[- ]time|contract|temporary|temp|req|job id|relocation)\b|(?-i:\b[A-Z]{2}\b)|\d|,",
    )
    .unwrap()
});
static REQ_ID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\s*(?:#\s*\d[\w-]*|\b(?:j?r|req)[-_ ]?\d{3,}\b)").unwrap());

/// A job posting's title as a suggestion: without locations, work types and
/// requisition numbers ("Software Engineer (Remote - US) R12345" ->
/// "Software Engineer"). None when nothing title-like is left.
pub fn clean_title(raw: &str) -> Option<String> {
    let mut t = raw.split(" | ").next().unwrap_or(raw).to_owned();
    t = REQ_ID.replace_all(&t, "").into_owned();
    // Parentheses that hold a place or a work type go; "(Backend)" stays.
    t = PARENS
        .replace_all(&t, |c: &regex::Captures| {
            if NOISE.is_match(&c[1]) {
                String::new()
            } else {
                c[0].to_owned()
            }
        })
        .into_owned();
    // " - Remote", " – Austin, TX": from the first dash whose rest is a
    // place or a work type.
    let dash = [" - ", " – ", " — "]
        .iter()
        .flat_map(|sep| {
            t.match_indices(sep)
                .map(|(i, _)| (i, sep.len()))
                .collect::<Vec<_>>()
        })
        .filter(|(i, len)| NOISE.is_match(&t[i + len..]))
        .min_by_key(|(i, _)| *i);
    if let Some((i, _)) = dash {
        t.truncate(i);
    }
    // Then trailing ", Remote", ": Austin, TX" (", Payments" stays).
    while let Some(i) = [", ", ": "].iter().filter_map(|sep| t.rfind(sep)).max() {
        let rest = t[i + 2..].trim();
        if rest.is_empty() || NOISE.is_match(rest) {
            t.truncate(i);
        } else {
            break;
        }
    }
    let t = t
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(|c: char| !c.is_alphanumeric() && c != ')' && c != '+' && c != '#')
        .to_owned();
    let letters = t.chars().filter(|c| c.is_alphabetic()).count();
    if letters < 2 || t.chars().count() > 80 {
        return None;
    }
    Some(t)
}

/// Groups `(text, count)` rows by their folded form: one entry each, shown
/// the way it was written most often, weighted by the total.
fn group(rows: impl IntoIterator<Item = (String, i64)>) -> Vec<(String, String, i64)> {
    let mut by: HashMap<String, (HashMap<String, i64>, i64)> = HashMap::new();
    for (text, n) in rows {
        let key = fold(&text);
        if key.is_empty() {
            continue;
        }
        let e = by.entry(key).or_default();
        *e.0.entry(text).or_default() += n;
        e.1 += n;
    }
    by.into_iter()
        .map(|(key, (forms, total))| {
            let best = forms
                .into_iter()
                .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
                .map(|(t, _)| t)
                .unwrap_or_default();
            (best, format!(" {key}"), total)
        })
        .collect()
}

async fn replace(pool: &PgPool, kind: &str, terms: &[(String, String, i64)]) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM suggest_terms WHERE kind = $1")
        .bind(kind)
        .execute(&mut *tx)
        .await?;
    for chunk in terms.chunks(5_000) {
        let values: Vec<&str> = chunk.iter().map(|t| t.0.as_str()).collect();
        let search: Vec<&str> = chunk.iter().map(|t| t.1.as_str()).collect();
        let weight: Vec<i32> = chunk
            .iter()
            .map(|t| t.2.min(i32::MAX as i64) as i32)
            .collect();
        sqlx::query(
            "INSERT INTO suggest_terms (kind, value, search, weight)
             SELECT $1, * FROM unnest($2::text[], $3::text[], $4::int[])
             ON CONFLICT (kind, search) DO NOTHING",
        )
        .bind(kind)
        .bind(&values)
        .bind(&search)
        .bind(&weight)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await
}

/// Rebuilds the title and company suggestions from the job data. Returns
/// (titles, companies) stored.
pub async fn rebuild(pool: &PgPool) -> sqlx::Result<(usize, usize)> {
    let titles: Vec<(String, i64)> = sqlx::query_as(
        "SELECT title, count(*) FROM jobs WHERE in_scope AND title <> '' GROUP BY title",
    )
    .fetch_all(pool)
    .await?;
    let titles: Vec<_> = group(
        titles
            .into_iter()
            .filter_map(|(t, n)| clean_title(&t).map(|t| (t, n))),
    )
    .into_iter()
    .filter(|t| t.2 >= MIN_TITLE_COUNT)
    .collect();
    replace(pool, "title", &titles).await?;

    let companies: Vec<(String, i64)> = sqlx::query_as(
        "SELECT c.name, count(j.id)
         FROM companies c
         LEFT JOIN jobs j ON j.company_id = c.id AND j.closed_at IS NULL AND j.in_scope
         WHERE c.name <> ''
         GROUP BY c.name",
    )
    .fetch_all(pool)
    .await?;
    let companies = group(companies);
    replace(pool, "company", &companies).await?;
    Ok((titles.len(), companies.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folding() {
        assert_eq!(
            fold("Bachelor's  of Science (BS)"),
            "bachelors of science bs"
        );
        assert_eq!(fold("  São Paulo, SP "), "são paulo sp");
    }

    #[test]
    fn cities_rank_by_start_then_size() {
        let r = search_list(&PLACES, "sa", 8);
        let v: Vec<&str> = r.iter().map(|s| s.value.as_str()).collect();
        assert_eq!(v[0], "San Antonio, TX");
        assert!(v.contains(&"San Diego, CA"), "{v:?}");
        // "Santa Rosa" by its words, in either state.
        let r = search_list(&PLACES, "santa ros", 8);
        assert!(r.iter().any(|s| s.value == "Santa Rosa, CA"));
        // The state narrows it.
        let r = search_list(&PLACES, "clemson sc", 8);
        assert_eq!(r[0].value, "Clemson, SC");
        assert_eq!(search_list(&PLACES, "", 3)[0].value, "Remote");
    }

    #[test]
    fn schools_by_name_alias_and_typo() {
        let r = search_list(&SCHOOLS, "clem", 8);
        assert_eq!(r[0].value, "Clemson University");
        assert_eq!(r[0].detail.as_deref(), Some("Clemson, SC"));
        let r = search_list(&SCHOOLS, "ucla", 8);
        assert_eq!(r[0].value, "University of California-Los Angeles");
        let r = search_list(&SCHOOLS, "clemsn university", 8);
        assert!(r.iter().any(|s| s.value == "Clemson University"), "{r:?}");
    }

    #[test]
    fn related_titles_by_word() {
        let r = related_titles("coder");
        assert_eq!(&r[..2], ["software engineer", "software developer"]);
        assert!(
            related_titles("prog").contains(&"software developer"),
            "prefix of programmer"
        );
        assert!(related_titles("senior swe").contains(&"software engineer"));
        assert!(related_titles("Front-End").contains(&"frontend engineer"));
        assert!(related_titles("c").is_empty());
        assert!(related_titles("plumber").is_empty());
    }

    #[test]
    fn open_job_counts() {
        assert_eq!(open_jobs(1), "1 open job");
        assert_eq!(open_jobs(42), "42 open jobs");
        assert_eq!(open_jobs(1234), "1,234 open jobs");
        assert_eq!(open_jobs(1234567), "1,234,567 open jobs");
    }

    #[test]
    fn own_school_keeps_its_city() {
        assert_eq!(
            detail_for(&SCHOOLS, "Clemson University").as_deref(),
            Some("Clemson, SC")
        );
        assert_eq!(
            detail_for(&SCHOOLS, "clemson university").as_deref(),
            Some("Clemson, SC")
        );
        assert_eq!(detail_for(&SCHOOLS, "Not A Real School"), None);
    }

    #[test]
    fn titles_lose_places_and_ids() {
        let c = |s: &str| clean_title(s);
        assert_eq!(
            c("Software Engineer (Remote - US)").as_deref(),
            Some("Software Engineer")
        );
        assert_eq!(
            c("Software Engineer - Remote").as_deref(),
            Some("Software Engineer")
        );
        assert_eq!(
            c("Data Analyst – Austin, TX").as_deref(),
            Some("Data Analyst")
        );
        assert_eq!(
            c("Nurse | Night Shift | Req 12345").as_deref(),
            Some("Nurse")
        );
        assert_eq!(c("QA Engineer R123456").as_deref(), Some("QA Engineer"));
        assert_eq!(
            c("Software Engineer (Backend)").as_deref(),
            Some("Software Engineer (Backend)")
        );
        assert_eq!(
            c("Senior Software Engineer, Payments").as_deref(),
            Some("Senior Software Engineer, Payments")
        );
        assert_eq!(c("C++ Developer").as_deref(), Some("C++ Developer"));
        assert_eq!(c("12345"), None);
    }

    #[test]
    fn grouping_keeps_the_common_spelling() {
        let g = group([
            ("Software Engineer".to_owned(), 10),
            ("software engineer".to_owned(), 2),
            ("Data Analyst".to_owned(), 1),
        ]);
        let se = g.iter().find(|t| t.1 == " software engineer").unwrap();
        assert_eq!((se.0.as_str(), se.2), ("Software Engineer", 12));
    }
}
