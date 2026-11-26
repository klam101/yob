//! Company facts from Wikidata (free, CC0): industry, organization type,
//! headcount, founding year and headquarters country, for the feed's
//! Company filters.
//!
//! Companies are matched by name, since we don't know their websites: the
//! English label or alias, among entities that look like organizations (they
//! have an industry, headcount, stock listing, website or headquarters, and
//! aren't people). When several match, the one with the most Wikipedia
//! articles wins. Big companies match well; many startups aren't in
//! Wikidata, and the filters leave them out when used.
//!
//! Only company names are sent, in batches, at most one request every two
//! seconds, with the app's generic User-Agent.

use std::{collections::HashMap, sync::LazyLock, time::Duration};

use regex::Regex;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use super::http::{FetchError, Fetcher};

const ENDPOINT: &str = "https://query.wikidata.org/sparql";
/// Names per query.
const BATCH: usize = 40;
const PAUSE: Duration = Duration::from_secs(2);
/// How long facts (or a miss) are kept before checking again.
const REFRESH_DAYS: i32 = 30;

/// Legal-form words dropped from the end of a name ("Acme, Inc." -> "Acme").
static SUFFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)[,\s]+(inc\.?|incorporated|llc|l\.l\.c\.|ltd\.?|limited|corp\.?|corporation|co\.|plc|gmbh|s\.a\.|ag|l\.p\.|lp|llp|pbc|holdings)$").unwrap()
});
static PARENS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*\([^)]*\)").unwrap());

static INNER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\(([^)]*)\)").unwrap());

fn without_suffixes(name: &str) -> String {
    let mut short = name.trim().to_owned();
    loop {
        let next = SUFFIX.replace(&short, "").trim().to_owned();
        if next == short || next.is_empty() {
            return short;
        }
        short = next;
    }
}

/// The names a company may go by in Wikidata, most exact first. For
/// "AIS (Applied Information Sciences)" that's the full name, then the
/// spelled-out one; a bare acronym is too ambiguous to look up.
pub fn name_variants(name: &str) -> Vec<String> {
    let mut out = vec![name.trim().to_owned()];
    let mut push = |v: String| {
        if !v.is_empty() && !out.contains(&v) {
            out.push(v);
        }
    };
    let inner = INNER.captures(name).map(|c| c[1].trim().to_owned());
    if let Some(inner) = inner.filter(|i| i.contains(' ')) {
        push(without_suffixes(&inner));
    } else {
        push(without_suffixes(&PARENS.replace_all(name.trim(), "")));
    }
    out
}

/// Lowercase letters and digits only ("Zoom Info, Inc." -> "zoominfoinc").
fn squash(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// One word, very short, or all capitals: many organizations share it.
fn ambiguous(name: &str) -> bool {
    let letters: String = name.chars().filter(|c| c.is_alphabetic()).collect();
    !name.trim().contains(' ')
        || squash(name).len() <= 4
        || letters.chars().all(|c| c.is_uppercase())
}

/// The main part of a website's host: "https://www.zoominfo.com/" ->
/// "zoominfo", "https://www.ft.co.uk" -> "ft".
fn site_label(url: &str) -> Option<String> {
    let host = url::Url::parse(url).ok()?.host_str()?.to_ascii_lowercase();
    let parts: Vec<&str> = host.trim_start_matches("www.").split('.').collect();
    let n = parts.len();
    let label = match n {
        0 | 1 => return None,
        _ if n >= 3
            && parts[n - 1].len() == 2
            && ["co", "com", "org", "net", "ac"].contains(&parts[n - 2]) =>
        {
            parts[n - 3]
        }
        _ => parts[n - 2],
    };
    Some(label.to_owned())
}

/// Whether a candidate is the company. Names of several words are taken as
/// is. An ambiguous one ("Hatch", "DSI") needs the entity's website to match
/// the name or the job-board token (stripe.com for "Stripe" or board
/// `stripe`), or, without a website, a very well-known entity.
fn confident(name: &str, token: &str, f: &Facts) -> bool {
    if !ambiguous(name) {
        return true;
    }
    let (name, token) = (squash(name), squash(token));
    // Acronyms ("TMS", "NPE") match too many sites; only famous ones count.
    if name.len() <= 4 {
        return f.sitelinks >= 20;
    }
    match f.website.as_deref().and_then(site_label) {
        Some(label) => {
            let close = |s: &str| {
                s.len() >= 3
                    && (label == s
                        || (s.len() >= 5 && (label.starts_with(s) || s.starts_with(&label))))
            };
            close(&name) || close(&token)
        }
        None => f.sitelinks >= 20,
    }
}

/// Escapes a string for a SPARQL literal.
fn literal(s: &str) -> String {
    let escaped: String = s
        .chars()
        .filter(|c| !c.is_control())
        .flat_map(|c| match c {
            '"' => vec!['\\', '"'],
            '\\' => vec!['\\', '\\'],
            c => vec![c],
        })
        .collect();
    format!("\"{escaped}\"@en")
}

fn query(names: &[String]) -> String {
    let values: Vec<String> = names.iter().map(|n| literal(n)).collect();
    format!(
        r#"SELECT ?name ?item (SAMPLE(?sl) AS ?sitelinks)
  (GROUP_CONCAT(DISTINCT ?industryLabel; separator="|") AS ?industries)
  (MAX(?emp) AS ?employees) (MIN(YEAR(?inception)) AS ?founded)
  (SAMPLE(?countryLabel) AS ?country) (COUNT(DISTINCT ?exchange) AS ?listed)
  (GROUP_CONCAT(DISTINCT STRAFTER(STR(?type), "entity/"); separator="|") AS ?types)
  (SAMPLE(?site) AS ?website)
WHERE {{
  VALUES ?name {{ {} }}
  {{ ?item rdfs:label ?name }} UNION {{ ?item skos:altLabel ?name }}
  ?item wikibase:sitelinks ?sl .
  FILTER EXISTS {{ ?item wdt:P452|wdt:P1128|wdt:P414|wdt:P856|wdt:P159 [] }}
  FILTER NOT EXISTS {{ ?item wdt:P31 wd:Q5 }}
  OPTIONAL {{ ?item wdt:P452 ?ind . ?ind rdfs:label ?industryLabel FILTER(LANG(?industryLabel) = "en") }}
  OPTIONAL {{ ?item wdt:P1128 ?emp }}
  OPTIONAL {{ ?item wdt:P571 ?inception }}
  OPTIONAL {{ ?item wdt:P159/wdt:P17|wdt:P17 ?cty . ?cty rdfs:label ?countryLabel FILTER(LANG(?countryLabel) = "en") }}
  OPTIONAL {{ ?item wdt:P414 ?exchange }}
  OPTIONAL {{ ?item wdt:P31 ?type }}
  OPTIONAL {{ ?item wdt:P856 ?site }}
}}
GROUP BY ?name ?item"#,
        values.join(" ")
    )
}

#[derive(Debug, Deserialize)]
struct Answer {
    results: Results,
}

#[derive(Debug, Deserialize)]
struct Results {
    bindings: Vec<HashMap<String, Value>>,
}

#[derive(Debug, Deserialize)]
struct Value {
    value: String,
}

/// What Wikidata says about one company.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Facts {
    pub wikidata_id: String,
    pub industries: Vec<String>,
    pub org_type: &'static str,
    pub employees: Option<i32>,
    pub founded: Option<i16>,
    pub hq_country: Option<String>,
    website: Option<String>,
    sitelinks: i64,
}

/// Instance-of classes that mark an organization type.
const NONPROFIT: &[&str] = &[
    "Q163740",   // nonprofit organization
    "Q157031",   // foundation
    "Q3918",     // university
    "Q23002054", // private not-for-profit educational institution
    "Q1664720",  // institute
    "Q31855",    // research institute
    "Q708676",   // charitable organization
    "Q48204",    // voluntary association
];
const GOVERNMENT: &[&str] = &[
    "Q327333",    // government agency
    "Q2659904",   // government organization
    "Q192350",    // ministry
    "Q105543609", // federal agency of the United States
    "Q1802419",   // US federal executive department
];
const COOPERATIVE: &[&str] = &["Q4539", "Q2006431"];

fn org_type(listed: bool, types: &[&str]) -> &'static str {
    let any = |set: &[&str]| types.iter().any(|t| set.contains(t));
    if listed {
        "public"
    } else if any(GOVERNMENT) {
        "government"
    } else if any(NONPROFIT) {
        "nonprofit"
    } else if any(COOPERATIVE) {
        "cooperative"
    } else {
        "private"
    }
}

/// The entities found for each name, best known first.
fn parse(answer: Answer) -> HashMap<String, Vec<Facts>> {
    let mut found: HashMap<String, Vec<Facts>> = HashMap::new();
    for row in answer.results.bindings {
        let get = |k: &str| {
            row.get(k)
                .map(|v| v.value.as_str())
                .filter(|v| !v.is_empty())
        };
        let (Some(name), Some(item)) = (get("name"), get("item")) else {
            continue;
        };
        let types: Vec<&str> = get("types")
            .map(|t| t.split('|').collect())
            .unwrap_or_default();
        let facts = Facts {
            wikidata_id: item.rsplit('/').next().unwrap_or(item).to_owned(),
            industries: get("industries")
                .map(|v| v.split('|').map(str::to_owned).collect())
                .unwrap_or_default(),
            org_type: org_type(get("listed").is_some_and(|n| n != "0"), &types),
            employees: get("employees")
                .and_then(|v| v.parse::<f64>().ok())
                .filter(|n| (1.0..5e6).contains(n))
                .map(|n| n as i32),
            founded: get("founded")
                .and_then(|v| v.parse::<i32>().ok())
                .filter(|y| (1600..=2100).contains(y))
                .map(|y| y as i16),
            hq_country: get("country").map(str::to_owned),
            website: get("website").map(str::to_owned),
            sitelinks: get("sitelinks").and_then(|v| v.parse().ok()).unwrap_or(0),
        };
        found.entry(name.to_owned()).or_default().push(facts);
    }
    for list in found.values_mut() {
        list.sort_by_key(|f| std::cmp::Reverse(f.sitelinks));
    }
    found
}

/// The company's entity: by its most exact name, the best-known candidate
/// we're confident about.
fn pick(found: &HashMap<String, Vec<Facts>>, name: &str, token: &str) -> Option<Facts> {
    name_variants(name).iter().find_map(|n| {
        found
            .get(n)?
            .iter()
            .find(|f| confident(n, token, f))
            .cloned()
    })
}

#[derive(sqlx::FromRow)]
struct Due {
    id: Uuid,
    name: String,
    board_token: String,
}

/// Looks up facts for companies with open jobs that haven't been checked in
/// a while, `limit` at most. Returns (checked, matched).
pub async fn refresh(
    pool: &PgPool,
    fetcher: &Fetcher,
    limit: i64,
) -> anyhow::Result<(usize, usize)> {
    let due: Vec<Due> = sqlx::query_as(
        "SELECT c.id, c.name, c.board_token FROM companies c
         WHERE (c.facts_checked_at IS NULL OR c.facts_checked_at < now() - make_interval(days => $1))
           AND EXISTS (SELECT 1 FROM jobs j WHERE j.company_id = c.id AND j.closed_at IS NULL AND j.in_scope)
         ORDER BY c.facts_checked_at NULLS FIRST, c.id
         LIMIT $2",
    )
    .bind(REFRESH_DAYS)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let (mut checked, mut matched) = (0, 0);
    for chunk in due.chunks(BATCH) {
        let names: Vec<String> = {
            let mut all: Vec<String> = chunk.iter().flat_map(|c| name_variants(&c.name)).collect();
            all.sort();
            all.dedup();
            all
        };
        let url = url::Url::parse_with_params(
            ENDPOINT,
            &[("query", query(&names)), ("format", "json".into())],
        )?;
        let answer: Answer = match fetcher.get_json(url.as_str()).await {
            Ok(a) => a,
            // Rate limited or down: stop here and try again next round.
            Err(e @ (FetchError::RateLimited(_) | FetchError::Server(_))) => {
                tracing::warn!(error = ?e, "Wikidata unavailable; company facts paused");
                break;
            }
            Err(e) => {
                tracing::warn!(error = ?e, "Wikidata query failed; skipping a batch");
                mark_checked(pool, chunk.iter().map(|c| c.id).collect()).await?;
                checked += chunk.len();
                tokio::time::sleep(PAUSE).await;
                continue;
            }
        };
        let found = parse(answer);
        for company in chunk {
            match pick(&found, &company.name, &company.board_token) {
                Some(f) => {
                    save(pool, company.id, &f).await?;
                    matched += 1;
                }
                None => mark_checked(pool, vec![company.id]).await?,
            }
        }
        checked += chunk.len();
        tokio::time::sleep(PAUSE).await;
    }
    Ok((checked, matched))
}

async fn save(pool: &PgPool, id: Uuid, f: &Facts) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE companies SET wikidata_id = $2, industries = $3, org_type = $4, employees = $5,
             founded = $6, hq_country = $7, facts_checked_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .bind(&f.wikidata_id)
    .bind(&f.industries)
    .bind(f.org_type)
    .bind(f.employees)
    .bind(f.founded)
    .bind(&f.hq_country)
    .execute(pool)
    .await?;
    Ok(())
}

async fn mark_checked(pool: &PgPool, ids: Vec<Uuid>) -> sqlx::Result<()> {
    sqlx::query("UPDATE companies SET facts_checked_at = now() WHERE id = ANY($1)")
        .bind(ids)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variants() {
        assert_eq!(name_variants("Stripe"), ["Stripe"]);
        assert_eq!(name_variants("Acme, Inc."), ["Acme, Inc.", "Acme"]);
        assert_eq!(
            name_variants("Globex Corporation"),
            ["Globex Corporation", "Globex"]
        );
        assert_eq!(name_variants("Initech (US)"), ["Initech (US)", "Initech"]);
        assert_eq!(
            name_variants("AIS (Applied Information Sciences)"),
            [
                "AIS (Applied Information Sciences)",
                "Applied Information Sciences"
            ]
        );
        assert_eq!(
            name_variants("Umbrella Holdings, LLC"),
            ["Umbrella Holdings, LLC", "Umbrella"]
        );
    }

    #[test]
    fn ambiguous_names_need_a_matching_website() {
        let with = |site: Option<&str>, sitelinks| Facts {
            website: site.map(str::to_owned),
            sitelinks,
            ..Facts::default()
        };
        assert!(confident(
            "Groupon",
            "groupon",
            &with(Some("https://www.groupon.com/"), 30)
        ));
        assert!(confident(
            "Stripe",
            "stripe",
            &with(Some("https://stripe.com"), 50)
        ));
        // The board token confirms a renamed site.
        assert!(confident(
            "Meta",
            "facebook",
            &with(Some("https://www.facebook.com"), 90)
        ));
        assert!(!confident(
            "TMS",
            "tmsllc",
            &with(Some("https://tms.sm"), 2)
        ));
        assert!(!confident(
            "DSI",
            "dsi",
            &with(Some("https://www.digitalsports.ca"), 3)
        ));
        assert!(!confident("Hatch", "hatch", &with(None, 4)));
        assert!(confident("Hatch", "hatch", &with(None, 25)));
        assert!(confident(
            "Financial Times",
            "ft",
            &with(Some("https://www.ft.com"), 60)
        ));
        assert_eq!(site_label("https://www.ft.co.uk/x").as_deref(), Some("ft"));
    }

    #[test]
    fn query_escapes_names() {
        let q = query(&["Ben \"Jerry\" Co".to_owned()]);
        assert!(q.contains(r#""Ben \"Jerry\" Co"@en"#), "{q}");
    }

    #[test]
    fn picks_the_best_known_entity() {
        let answer: Answer = serde_json::from_value(serde_json::json!({
            "results": { "bindings": [
                { "name": {"value": "Acme"}, "item": {"value": "http://www.wikidata.org/entity/Q1"},
                  "sitelinks": {"value": "3"}, "industries": {"value": "retail"}, "listed": {"value": "0"},
                  "types": {"value": "Q4830453"} },
                { "name": {"value": "Acme"}, "item": {"value": "http://www.wikidata.org/entity/Q2"},
                  "sitelinks": {"value": "40"}, "industries": {"value": "software|cloud computing"},
                  "employees": {"value": "12000"}, "founded": {"value": "1999"},
                  "country": {"value": "United States"}, "listed": {"value": "1"}, "types": {"value": "Q4830453"} },
                { "name": {"value": "Red Cross"}, "item": {"value": "http://www.wikidata.org/entity/Q3"},
                  "sitelinks": {"value": "5"}, "listed": {"value": "0"}, "types": {"value": "Q163740|Q43229"} }
            ]}
        }))
        .unwrap();
        let found = parse(answer);
        let acme = &found["Acme"][0];
        assert_eq!(acme.wikidata_id, "Q2");
        assert_eq!(acme.industries, ["software", "cloud computing"]);
        assert_eq!(acme.org_type, "public");
        assert_eq!(acme.employees, Some(12000));
        assert_eq!(acme.founded, Some(1999));
        assert_eq!(found["Red Cross"][0].org_type, "nonprofit");
    }
}
