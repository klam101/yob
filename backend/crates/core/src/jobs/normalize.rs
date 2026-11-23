//! Turning platform data into consistent fields. Everything here is a pure
//! function so it can be unit-tested without the network or a database.

use std::sync::LazyLock;

use regex::Regex;
use sha2::{Digest, Sha256};

use super::model::{EmploymentType, Salary, SalaryPeriod, Seniority, Workplace};

/// Sanitizes posting HTML (set `escaped` for Greenhouse, which HTML-escapes
/// its content) and returns `(safe_html, plain_text)`.
pub fn clean_html(raw: &str, escaped: bool) -> (String, String) {
    let decoded = if escaped {
        html_escape::decode_html_entities(raw).into_owned()
    } else {
        raw.to_owned()
    };

    let html = ammonia::Builder::default()
        // No remote images or embeds: they'd load third-party content.
        .rm_tags(["img"])
        .link_rel(Some("noopener noreferrer nofollow"))
        .clean(&decoded)
        .to_string();

    (html.clone(), html_to_text(&html))
}

static BLOCK_END: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)</(p|li|h[1-6]|div|ul|ol|tr)>|<br\s*/?>").unwrap());
static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());
static SPACES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[ \t\u{a0}]+").unwrap());
static BLANK_LINES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\n\s*\n+").unwrap());

/// Plain text with line breaks between blocks, for search and AI later.
pub fn html_to_text(html: &str) -> String {
    let with_breaks = BLOCK_END.replace_all(html, "\n");
    let stripped = TAG.replace_all(&with_breaks, "");
    let decoded = html_escape::decode_html_entities(&stripped);
    let spaced = SPACES.replace_all(&decoded, " ");
    let lines: Vec<&str> = spaced.lines().map(str::trim).collect();
    BLANK_LINES
        .replace_all(&lines.join("\n"), "\n\n")
        .trim()
        .to_owned()
}

static WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[a-z0-9+#]+").unwrap());

fn words(s: &str) -> Vec<String> {
    WORD.find_iter(&s.to_lowercase())
        .map(|m| m.as_str().to_owned())
        .collect()
}

/// Seniority from the title. Order matters: "Senior Intern" is an intern,
/// "Staff Engineer" beats "Engineer II".
pub fn infer_seniority(title: &str) -> Seniority {
    let w = words(title);
    let has = |x: &str| w.iter().any(|t| t == x);
    let lower = title.to_lowercase();

    if has("intern") || has("internship") || has("co") && has("op") || lower.contains("co-op") {
        Seniority::Intern
    } else if [
        "staff",
        "principal",
        "distinguished",
        "director",
        "head",
        "vp",
        "chief",
        "fellow",
    ]
    .iter()
    .any(|x| has(x))
        || lower.contains("vice president")
    {
        Seniority::StaffPlus
    } else if has("senior")
        || has("sr")
        || has("lead")
        || has("iii")
        || has("iv")
        // People managers of engineering teams ("Engineering Manager",
        // "Manager, Software Engineering"). Not "Product Manager" or
        // "Account Manager", which are usually mid-level roles.
        || has("manager")
            && ["engineering", "software", "development", "infrastructure"]
                .iter()
                .any(|x| has(x))
    {
        Seniority::Senior
    } else if has("junior")
        || has("jr")
        || has("entry")
        || has("associate")
        || has("graduate")
        || lower.contains("new grad")
        || lower.contains("early career")
        || has("i")
        || has("apprentice")
    {
        Seniority::Entry
    } else {
        Seniority::Mid
    }
}

/// Employment type from a platform hint (Lever's "commitment") or the title.
pub fn infer_employment(title: &str, hint: Option<&str>) -> EmploymentType {
    let text = format!("{} {}", hint.unwrap_or(""), title).to_lowercase();
    if text.contains("intern") || text.contains("co-op") {
        EmploymentType::Internship
    } else if text.contains("part-time") || text.contains("part time") {
        EmploymentType::PartTime
    } else if text.contains("contract")
        || text.contains("temporary")
        || text.contains("temp ")
        || text.contains("freelance")
        || text.contains("fixed term")
    {
        EmploymentType::Contract
    } else {
        EmploymentType::FullTime
    }
}

/// Workplace when the platform doesn't say (Greenhouse): "remote" in a
/// location or the title, then "hybrid" anywhere, otherwise on-site.
pub fn infer_workplace(title: &str, locations: &[String], text: &str) -> Workplace {
    let remote = |s: &str| s.to_lowercase().contains("remote");
    if remote(title) || locations.iter().any(|l| remote(l)) {
        Workplace::Remote
    } else if title.to_lowercase().contains("hybrid")
        || locations
            .iter()
            .any(|l| l.to_lowercase().contains("hybrid"))
        || text.to_lowercase().contains("hybrid")
    {
        Workplace::Hybrid
    } else {
        Workplace::Onsite
    }
}

/// A USD salary range written in the description, e.g. "$120,000 - $150,000",
/// "$55 – $65 per hour", "$120K to $150K" or "187,500 USD – 364,000 USD"
/// (common on Workday). Returns None when unsure.
pub fn parse_salary(text: &str) -> Option<Salary> {
    static RANGE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)\$\s?(\d{1,3}(?:,\d{3})+|\d+(?:\.\d+)?)\s?(k)?\s*(?:usd\s*)?(?:-|–|—|to)\s*\$?\s?(\d{1,3}(?:,\d{3})+|\d+(?:\.\d+)?)\s?(k)?",
        )
        .unwrap()
    });
    // No dollar sign: "USD" right after the first number is required.
    static USD_RANGE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)\b(\d{1,3}(?:,\d{3})+|\d+(?:\.\d+)?)\s?(k)?\s*usd\s*(?:-|–|—|to)\s*\$?\s?(\d{1,3}(?:,\d{3})+|\d+(?:\.\d+)?)\s?(k)?",
        )
        .unwrap()
    });

    for caps in RANGE
        .captures_iter(text)
        .chain(USD_RANGE.captures_iter(text))
    {
        let num = |i: usize, k: usize| -> Option<f64> {
            let n: f64 = caps.get(i)?.as_str().replace(',', "").parse().ok()?;
            Some(if caps.get(k).is_some() { n * 1000.0 } else { n })
        };
        let (Some(mut min), Some(max)) = (num(1, 2), num(3, 4)) else {
            continue;
        };
        // "$120-150K": the K applies to both ends.
        if caps.get(2).is_none() && caps.get(4).is_some() && min < 1000.0 {
            min *= 1000.0;
        }
        if min > max {
            continue;
        }
        let after = text[caps.get(0).unwrap().end()..]
            .chars()
            .take(24)
            .collect::<String>()
            .to_lowercase();
        // "$50 - $60 million" is funding, not pay.
        let tail = after.trim_start();
        if tail.starts_with("million") || tail.starts_with("billion") || tail.starts_with("m ") {
            continue;
        }
        let hourly = max < 1000.0 || after.contains("hour") || after.contains("/hr");
        let ok = if hourly {
            (10.0..=500.0).contains(&min) && max <= 500.0
        } else {
            // Above $1M a bare "$" is more likely pesos or another dollar
            // currency than USD. Real seven-figure salaries come from the
            // structured fields (Ashby, Lever), which carry a currency code.
            (20_000.0..=1_000_000.0).contains(&min) && max <= 1_000_000.0
        };
        if ok {
            return Some(Salary {
                min: min.round() as u32,
                max: max.round() as u32,
                currency: "USD".to_owned(),
                period: if hourly {
                    SalaryPeriod::Hour
                } else {
                    SalaryPeriod::Year
                },
            });
        }
    }
    None
}

/// Version of the rules in this file. Bump it whenever a rule changes how
/// seniority, scope or a text-parsed salary come out; the worker then
/// re-derives stored jobs (`ingest::renormalize`) without refetching them.
pub const NORMALIZE_VERSION: i32 = 5;

/// Where a single location string points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    Us,
    NonUs,
    /// "Remote", "Anywhere", or a place we don't recognize.
    Unknown,
}

fn alternation(words: &[&str]) -> Regex {
    let escaped: Vec<String> = words.iter().map(|w| regex::escape(w)).collect();
    Regex::new(&format!(r"\b(?:{})\b", escaped.join("|"))).unwrap()
}

/// Explicit US or US-including regions. Wins over everything else, so
/// "Remote - US or Canada" stays in scope.
static US_EXPLICIT: LazyLock<Regex> = LazyLock::new(|| {
    alternation(&[
        "us",
        "usa",
        "u.s",
        "u.s.a",
        "united states",
        "north america",
        "americas",
        "us-remote",
        "remote-us",
        "namer",
        "amer",
        "usca",
    ])
});

/// Countries and regions other than the US. "Georgia", "Lebanon", "Panama",
/// "Jersey" and similar are left out: they're also US states or towns.
static FOREIGN_REGION: LazyLock<Regex> = LazyLock::new(|| {
    alternation(&[
        "canada",
        "mexico",
        "brazil",
        "brasil",
        "argentina",
        "chile",
        "colombia",
        "peru",
        "uruguay",
        "costa rica",
        "guatemala",
        "puerto rico",
        "dominican republic",
        "united kingdom",
        "uk",
        "u.k",
        "england",
        "scotland",
        "wales",
        "northern ireland",
        "ireland",
        "france",
        "germany",
        "deutschland",
        "spain",
        "españa",
        "portugal",
        "italy",
        "netherlands",
        "the netherlands",
        "holland",
        "belgium",
        "luxembourg",
        "switzerland",
        "austria",
        "denmark",
        "sweden",
        "norway",
        "finland",
        "iceland",
        "estonia",
        "latvia",
        "lithuania",
        "poland",
        "czechia",
        "czech republic",
        "slovakia",
        "hungary",
        "romania",
        "bulgaria",
        "greece",
        "croatia",
        "serbia",
        "slovenia",
        "ukraine",
        "turkey",
        "türkiye",
        "cyprus",
        "malta",
        "israel",
        "egypt",
        "morocco",
        "nigeria",
        "kenya",
        "ghana",
        "south africa",
        "uae",
        "united arab emirates",
        "saudi arabia",
        "qatar",
        "india",
        "pakistan",
        "bangladesh",
        "sri lanka",
        "nepal",
        "china",
        "hong kong",
        "taiwan",
        "japan",
        "south korea",
        "korea",
        "singapore",
        "malaysia",
        "indonesia",
        "philippines",
        "thailand",
        "vietnam",
        "australia",
        "new zealand",
        "emea",
        "apac",
        "latam",
        "europe",
        "eu",
        "asia",
        "africa",
        "oceania",
        "middle east",
        "latin america",
        "south america",
        "central america",
        "european union",
        "kuwait",
        "cambodia",
        // Not "jordan", "lebanon", "georgia" or "panama": West Jordan UT,
        // Lebanon PA, the state, Panama City FL. Their capitals are cities below.
        "viet nam",
        "oman",
        "bahrain",
        "iraq",
        "iran",
        "tunisia",
        "algeria",
        "ethiopia",
        "tanzania",
        "uganda",
        "rwanda",
        "senegal",
        "côte d'ivoire",
        "cameroon",
        "zimbabwe",
        "zambia",
        "mozambique",
        "angola",
        "kazakhstan",
        "uzbekistan",
        "azerbaijan",
        "armenia",
        "mongolia",
        "myanmar",
        "macau",
        "macao",
        "ecuador",
        "bolivia",
        "paraguay",
        "venezuela",
        "honduras",
        "el salvador",
        "nicaragua",
        "jamaica",
        "trinidad",
        "belarus",
        "moldova",
        "bosnia",
        "albania",
        "north macedonia",
        "montenegro",
        "kosovo",
        "anz",
        "dach",
        "nordics",
        "benelux",
    ])
});

/// Canadian province codes after a comma ("Toronto, ON"), and ISO country
/// codes some boards use ("Mohali, IND", "Burnaby, CAN", "GBR - LON Metro").
/// Uppercase only, so "Can be remote" stays unknown.
static FOREIGN_CODE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?:,\s*(?:ON|BC|QC|AB|MB|NS|NB|NL|PE|SK)\b)|(?:^|,|\s-)\s*(?:IND|CAN|GBR|DEU|FRA|CRI|BRA|MEX|AUS|NZL|IRL|NZ)\b",
    )
    .unwrap()
});

/// US state codes after a comma or dash ("Austin, TX", "Hybrid - NY").
static US_STATE_CODE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?:,|\s-)\s*(?:AL|AK|AZ|AR|CA|CO|CT|DE|DC|FL|GA|HI|ID|IL|IN|IA|KS|KY|LA|ME|MD|MA|MI|MN|MS|MO|MT|NE|NV|NH|NJ|NM|NY|NC|ND|OH|OK|OR|PA|RI|SC|SD|TN|TX|UT|VT|VA|WA|WV|WI|WY)\b",
    )
    .unwrap()
});

static US_PLACE: LazyLock<Regex> = LazyLock::new(|| {
    alternation(&[
        "alabama",
        "alaska",
        "arizona",
        "arkansas",
        "california",
        "colorado",
        "connecticut",
        "delaware",
        "district of columbia",
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
        // Cities and areas that often appear without a state.
        "nyc",
        "sf",
        "san francisco",
        "bay area",
        "sf bay area",
        "silicon valley",
        "los angeles",
        "seattle",
        "austin",
        "boston",
        "chicago",
        "denver",
        "atlanta",
        "miami",
        "dallas",
        "houston",
        "philadelphia",
        "phoenix",
        "san diego",
        "san jose",
        "portland",
        "salt lake city",
        "nashville",
        "raleigh",
        "pittsburgh",
        "detroit",
        "minneapolis",
        "palo alto",
        "mountain view",
        "menlo park",
        "sunnyvale",
        "redwood city",
        "santa clara",
        "brooklyn",
        "manhattan",
        "jersey city",
        "boulder",
        "irvine",
        "san mateo",
        "cambridge, ma",
        "washington dc",
        "washington, d.c",
        "d.c",
        "dc",
        "charlotte",
        "columbus",
        "st. louis",
        "kansas city",
        "baltimore",
        "tampa",
        "orlando",
        "las vegas",
        "sacramento",
        "oakland",
    ])
});

/// Big non-US cities that often appear without a country. Checked after the
/// US lists, so "Dublin, OH" and "Paris, TX" stay American.
static FOREIGN_CITY: LazyLock<Regex> = LazyLock::new(|| {
    alternation(&[
        "amman",
        "beirut",
        "tbilisi",
        "panama city, panama",
        "london",
        "paris",
        "berlin",
        "munich",
        "hamburg",
        "frankfurt",
        "amsterdam",
        "dublin",
        "toronto",
        "vancouver",
        "montreal",
        "montréal",
        "ottawa",
        "calgary",
        "edmonton",
        "sydney",
        "melbourne",
        "brisbane",
        "auckland",
        "tokyo",
        "osaka",
        "seoul",
        "beijing",
        "shanghai",
        "shenzhen",
        "taipei",
        "manila",
        "jakarta",
        "bangkok",
        "ho chi minh city",
        "hanoi",
        "kuala lumpur",
        "bangalore",
        "bengaluru",
        "hyderabad",
        "pune",
        "mumbai",
        "delhi",
        "new delhi",
        "gurgaon",
        "gurugram",
        "noida",
        "chennai",
        "kolkata",
        "tel aviv",
        "são paulo",
        "sao paulo",
        "rio de janeiro",
        "mexico city",
        "ciudad de méxico",
        "guadalajara",
        "buenos aires",
        "bogotá",
        "bogota",
        "medellín",
        "medellin",
        "santiago",
        "lima",
        "madrid",
        "barcelona",
        "lisbon",
        "porto",
        "milan",
        "rome",
        "zurich",
        "zürich",
        "geneva",
        "vienna",
        "brussels",
        "copenhagen",
        "stockholm",
        "oslo",
        "helsinki",
        "warsaw",
        "krakow",
        "kraków",
        "wroclaw",
        "prague",
        "budapest",
        "bucharest",
        "sofia",
        "athens",
        "istanbul",
        "kyiv",
        "belgrade",
        "zagreb",
        "tallinn",
        "riga",
        "vilnius",
        "edinburgh",
        "manchester",
        "glasgow",
        "belfast",
        "cape town",
        "johannesburg",
        "lagos",
        "nairobi",
        "cairo",
        "dubai",
        "abu dhabi",
        "riyadh",
        "doha",
        // Provinces here, not with the countries: "Ontario, CA" is in California.
        "ontario",
        "british columbia",
        "alberta",
        "quebec",
        "québec",
        "manitoba",
        "nova scotia",
        "saskatchewan",
        "malmö",
        "malmo",
        "reykjavík",
        "reykjavik",
        "cork",
        "canberra",
        "wellington",
        "burnaby",
        "cdmx",
        "aveiro",
        "waterloo",
    ])
});

/// Where one location string points. Order matters; see the statics above.
pub fn classify_place(location: &str) -> Place {
    let lower = location.to_lowercase();
    if US_EXPLICIT.is_match(&lower) {
        Place::Us
    } else if FOREIGN_REGION.is_match(&lower.replace("new mexico", ""))
        || FOREIGN_CODE.is_match(location)
    {
        Place::NonUs
    } else if US_STATE_CODE.is_match(location) || US_PLACE.is_match(&lower) {
        Place::Us
    } else if FOREIGN_CITY.is_match(&lower) {
        Place::NonUs
    } else {
        Place::Unknown
    }
}

/// Whether a job belongs in the feed: US or remote for US workers. Out only
/// when every location points outside the US. Unknown places ("Remote",
/// a small town) count as in, so US jobs are never hidden by a gap in the lists.
pub fn us_scope(locations: &[String]) -> bool {
    locations.is_empty() || locations.iter().any(|l| classify_place(l) != Place::NonUs)
}

/// A platform's structured salary if it has one, otherwise one parsed from
/// the description. The flag says it came from text (and can be re-derived).
pub fn salary_or_text(structured: Option<Salary>, text: &str) -> (Option<Salary>, bool) {
    match structured {
        Some(s) => (Some(s), false),
        None => {
            let parsed = parse_salary(text);
            let from_text = parsed.is_some();
            (parsed, from_text)
        }
    }
}

/// Stable fingerprint of a posting's content. A new job id with the same hash
/// as a closed job at the same company is a repost.
pub fn content_hash(title: &str, locations: &[String], text: &str) -> String {
    let mut h = Sha256::new();
    h.update(words(title).join(" "));
    h.update("\n");
    h.update(locations.join("|").to_lowercase());
    h.update("\n");
    h.update(words(text).join(" "));
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seniority_from_titles() {
        let cases = [
            (
                "Software Engineering Intern, Summer 2027",
                Seniority::Intern,
            ),
            ("Senior Backend Engineer", Seniority::Senior),
            ("Sr. Data Scientist", Seniority::Senior),
            (
                "Staff Software Engineer, Infrastructure",
                Seniority::StaffPlus,
            ),
            ("Director of Engineering", Seniority::StaffPlus),
            ("Software Engineer I", Seniority::Entry),
            ("Software Engineer, Early Career (SF)", Seniority::Entry),
            ("New Grad Software Engineer", Seniority::Entry),
            ("Junior QA Analyst", Seniority::Entry),
            ("Software Engineer II", Seniority::Mid),
            ("Backend Engineer", Seniority::Mid),
            ("Software Engineer III", Seniority::Senior),
            (
                "Manager, Software Engineering (Code Standards)",
                Seniority::Senior,
            ),
            ("Engineering Manager, Payments", Seniority::Senior),
            ("Product Manager", Seniority::Mid),
            ("Account Manager", Seniority::Mid),
        ];
        for (title, want) in cases {
            assert_eq!(infer_seniority(title), want, "{title}");
        }
    }

    #[test]
    fn employment_from_hints() {
        assert_eq!(
            infer_employment("Engineer", Some("Full-time")),
            EmploymentType::FullTime
        );
        assert_eq!(
            infer_employment("Engineer", Some("Contract")),
            EmploymentType::Contract
        );
        assert_eq!(
            infer_employment("Data Intern", None),
            EmploymentType::Internship
        );
        assert_eq!(
            infer_employment("Barista (Part-Time)", None),
            EmploymentType::PartTime
        );
        assert_eq!(infer_employment("Engineer", None), EmploymentType::FullTime);
    }

    #[test]
    fn workplace_inference() {
        let loc = |s: &str| vec![s.to_owned()];
        assert_eq!(
            infer_workplace("Engineer", &loc("Remote - US"), ""),
            Workplace::Remote
        );
        assert_eq!(
            infer_workplace("Engineer", &loc("New York, NY"), "This is a hybrid role"),
            Workplace::Hybrid
        );
        assert_eq!(
            infer_workplace("Engineer", &loc("Austin, TX"), "In office daily"),
            Workplace::Onsite
        );
    }

    #[test]
    fn salary_ranges() {
        let s = parse_salary("The base pay range is $120,000 - $150,000 per year.").unwrap();
        assert_eq!(
            (s.min, s.max, s.period),
            (120_000, 150_000, SalaryPeriod::Year)
        );

        let s = parse_salary("Pay: $55 – $65 per hour").unwrap();
        assert_eq!((s.min, s.max, s.period), (55, 65, SalaryPeriod::Hour));

        let s = parse_salary("ranges are:\n187,500 USD – 364,000 USD for Level 4").unwrap();
        assert_eq!(
            (s.min, s.max, s.period),
            (187_500, 364_000, SalaryPeriod::Year)
        );
        assert!(parse_salary("Teams of 120,000 to 150,000 USD-based users").is_none());
        let s = parse_salary("$120K to $150K + equity").unwrap();
        assert_eq!((s.min, s.max), (120_000, 150_000));

        let s = parse_salary("Range: $120-150K").unwrap();
        assert_eq!((s.min, s.max), (120_000, 150_000));

        assert!(parse_salary("We raised $50 - $60 million").is_none());
        assert!(parse_salary("No numbers here").is_none());
        assert!(parse_salary("$5 - $8").is_none(), "too low to be a wage");
        assert!(
            parse_salary("$900,000 - $1,254,000 MXN").is_none(),
            "seven-figure \"$\" amounts are probably not USD"
        );
    }

    #[test]
    fn places() {
        let cases = [
            ("San Francisco, CA", Place::Us),
            ("New York, NY", Place::Us),
            ("Remote - US", Place::Us),
            ("US-Remote", Place::Us),
            ("Remote, North America", Place::Us),
            ("Remote - US or Canada", Place::Us),
            ("United States", Place::Us),
            ("Hybrid - NYC", Place::Us),
            ("Austin", Place::Us),
            ("Albuquerque, New Mexico", Place::Us),
            ("Dublin, OH", Place::Us),
            ("Washington, D.C.", Place::Us),
            ("Remote", Place::Unknown),
            ("Anywhere", Place::Unknown),
            ("London, United Kingdom", Place::NonUs),
            ("London", Place::NonUs),
            ("Toronto, ON", Place::NonUs),
            ("Toronto, ON, CA", Place::NonUs),
            ("Remote - Canada", Place::NonUs),
            ("Remote, EMEA", Place::NonUs),
            ("Bengaluru", Place::NonUs),
            ("Mexico City", Place::NonUs),
            ("Berlin, Germany", Place::NonUs),
            ("Dublin, Ireland", Place::NonUs),
            ("Remote - European Union", Place::NonUs),
            ("Mohali, IND", Place::NonUs),
            ("GBR - LON Metro", Place::NonUs),
            ("Vancouver, British Columbia", Place::NonUs),
            ("Ontario, CA", Place::Us),
            // Workday's "Country - City" and "US, ST, City" forms.
            ("Jordan - Amman", Place::NonUs),
            ("Lebanon - Beirut", Place::NonUs),
            ("Viet Nam - Ho Chi Minh", Place::NonUs),
            ("Oman - Muscat", Place::NonUs),
            ("US, CA, Santa Clara", Place::Us),
            ("USA.VA.Reston", Place::Us),
            ("Vancouver, Washington, United States of America", Place::Us),
            ("West Jordan, UT", Place::Us),
            ("Lebanon, PA", Place::Us),
            ("NAMER", Place::Us),
            ("Can be remote", Place::Unknown),
            ("Hybrid", Place::Unknown),
        ];
        for (loc, want) in cases {
            assert_eq!(classify_place(loc), want, "{loc}");
        }
    }

    #[test]
    fn scope() {
        let locs = |xs: &[&str]| xs.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(us_scope(&locs(&[])));
        assert!(us_scope(&locs(&["Remote"])));
        assert!(us_scope(&locs(&["London", "New York, NY"])));
        assert!(!us_scope(&locs(&["London", "Paris, France"])));
        assert!(!us_scope(&locs(&["Remote - Canada"])));
    }

    #[test]
    fn html_cleaning() {
        let (html, text) = clean_html(
            "&lt;h2&gt;About&lt;/h2&gt;&lt;p&gt;We &amp;amp; you&lt;/p&gt;&lt;script&gt;x()&lt;/script&gt;&lt;img src=x&gt;",
            true,
        );
        assert!(html.contains("<h2>About</h2>"));
        assert!(!html.contains("script") && !html.contains("<img"));
        assert_eq!(text, "About\nWe & you");
    }

    #[test]
    fn hash_ignores_formatting() {
        let locs = vec!["NYC".to_owned()];
        assert_eq!(
            content_hash("Software Engineer", &locs, "Build  things!"),
            content_hash("software engineer", &locs, "build things")
        );
        assert_ne!(
            content_hash("Software Engineer", &locs, "a"),
            content_hash("Data Engineer", &locs, "a")
        );
    }
}
