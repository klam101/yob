//! Details read from a job's title and description by rules, for the feed's
//! filters: education, perks, clearance, licenses, languages, shifts,
//! travel, skills, years of experience and more. They're stored as
//! namespaced tags (`jobs.tags`, e.g. `edu:bachelors:required`,
//! `skill:rust`), plus `min_years` and the job function.
//!
//! Rules are conservative: only clear wording counts, and a job that says
//! nothing gets no tag. Equal-opportunity boilerplate ("...protected
//! veterans, individuals with disabilities...") doesn't count as
//! encouraging anyone to apply. A local model may fill gaps later.
//!
//! Changing a rule changes stored data: bump `normalize::NORMALIZE_VERSION`
//! so the worker re-reads stored jobs.

use std::sync::LazyLock;

use regex::{Regex, RegexSet};
use serde::Serialize;
use utoipa::ToSchema;

use super::{functions, normalize};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Extracted {
    pub tags: Vec<String>,
    pub min_years: Option<i16>,
    pub job_function: &'static str,
}

/// A tag the feed can filter on, with its label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
pub struct TagInfo {
    /// The full tag, e.g. `skill:rust`.
    pub value: &'static str,
    pub label: &'static str,
}

// --- vocabularies --------------------------------------------------------------

/// (tag, label, pattern over lowercased text).
type Rule = (&'static str, &'static str, &'static str);

const PERKS: &[Rule] = &[
    (
        "perk:pto",
        "Generous time off",
        r"unlimited (?:pto|paid time off|vacation|time off)|flexible (?:pto|time off|vacation)|generous (?:pto|paid time off|vacation|time off)|(?:1[5-9]|[2-4]\d)\+? ?(?:days|vacation days) (?:of )?(?:pto|paid time off|paid vacation|vacation)|(?:[3-9]|1\d) weeks? (?:of )?(?:pto|paid time off|paid vacation|vacation)",
    ),
    (
        "perk:four_day_week",
        "Four-day work week",
        r"(?:4|four)[- ]day (?:work ?week|week)|32[- ]hour (?:work ?)?week",
    ),
    (
        "perk:401k_match",
        "401(k) match",
        r"401 ?\(?k\)?[^.\n]{0,80}match|match(?:ing|es)?[^.\n]{0,60}401 ?\(?k\)?",
    ),
    (
        "perk:retirement",
        "Retirement plan",
        r"401 ?\(?k\)?|403 ?\(?b\)?|457 ?\(?b\)?|retirement (?:plan|savings|benefits?|account|contribution)|pension",
    ),
    (
        "perk:parental_leave",
        "Paid parental leave",
        r"paid (?:parental|maternity|paternity|family|caregiver|baby bonding) leave|(?:\d+|six|eight|ten|twelve|sixteen|twenty) weeks? (?:of )?(?:fully )?(?:paid )?(?:parental|maternity|paternity|family|bonding) leave|(?:generous|fully[- ]paid) (?:parental|family|maternity) leave",
    ),
    (
        "perk:tuition",
        "Tuition help",
        r"tuition (?:reimbursement|assistance|support|benefits?|coverage)|education(?:al)? (?:reimbursement|assistance|stipend)|student loan (?:repayment|assistance|support|contribution)",
    ),
    (
        "perk:visa_sponsorship",
        "Visa sponsorship",
        r"(?:will|can|able to|happy to|we|does|do) (?:provide |offer )?(?:visa |h-?1b )?sponsor(?:ship)?\b|visa sponsorship (?:is |will be )?(?:available|provided|offered|possible|supported)|sponsorship (?:is |will be )?(?:available|provided|offered)|h-?1b (?:sponsorship|transfers?) (?:is |are )?(?:available|welcome|offered|supported|considered)|open to (?:visa |h-?1b )?sponsor",
    ),
    (
        "perk:relocation",
        "Relocation help",
        r"relocation (?:assistance|package|support|bonus|stipend|benefits?|reimbursement|allowance|is (?:available|provided|offered))|(?:assist|help) with relocation|relocation (?:will|can|may) be (?:provided|offered|available)|relocation (?:offered|available|provided)",
    ),
];

/// Perks that a negative sentence ("not able to sponsor", "no relocation")
/// cancels.
const NEGATABLE: &[&str] = &["perk:visa_sponsorship", "perk:relocation"];

const CLEARANCES: &[Rule] = &[
    (
        "clearance:ts_sci",
        "TS/SCI",
        r"ts ?/ ?sci|ts-sci|topsecret ?/ ?sci|topsecret sci|\bsci (?:clearance|eligib|access)",
    ),
    (
        "clearance:top_secret",
        "Top Secret",
        r"topsecret|\bts (?:clearance|level)",
    ),
    (
        "clearance:secret",
        "Secret",
        r"\bsecret\b[^.\n]{0,25}clearance|clearance[^.\n]{0,25}\bsecret\b|\bsecret[- ]level|\bactive secret\b",
    ),
    (
        "clearance:confidential",
        "Confidential",
        r"confidential (?:security )?clearance",
    ),
    ("clearance:public_trust", "Public Trust", r"public trust"),
    (
        "clearance:interim",
        "Interim clearance",
        r"interim (?:security |secret |topsecret |ts )?clearance|interim (?:secret|topsecret)",
    ),
    (
        "clearance:polygraph",
        "Polygraph",
        r"polygraph|\bci poly|full[- ]scope poly|\bfs poly",
    ),
    (
        "clearance:any",
        "Any clearance",
        r"security clearance|clearance (?:is )?required|active clearance|dod clearance|\bdoe [ql] clearance|\b[ql] clearance|government clearance|federal clearance",
    ),
];

const ENCOURAGE: &[TagInfo] = &[
    TagInfo {
        value: "encourage:veterans",
        label: "Military veterans",
    },
    TagInfo {
        value: "encourage:fair_chance",
        label: "Fair chance (records considered)",
    },
];

const SHIFTS: &[Rule] = &[
    (
        "shift:day",
        "Day shift",
        r"\bday shifts?\b|\b1st shift|\bfirst shift|morning shifts?",
    ),
    (
        "shift:evening",
        "Evening shift",
        r"evening shifts?|\b2nd shift|\bsecond shift|swing shifts?|afternoon shifts?",
    ),
    (
        "shift:night",
        "Night shift",
        r"night shifts?|overnight shifts?|\b3rd shift|\bthird shift|graveyard|work(?:ing)? (?:nights|overnights)|nights? (?:and|&) weekends",
    ),
    (
        "shift:weekend",
        "Weekends",
        r"weekends?\b[^.\n]{0,40}(?:required|availab|must|as needed|rotation|rotating)|(?:work|available|availability|including|includes)[^.\n]{0,40}weekends?|every other weekend|nights? (?:and|&) weekends",
    ),
    (
        "shift:holiday",
        "Holidays",
        r"holidays?[^.\n]{0,30}(?:required|as needed|availab|must|rotation)|(?:work|available|availability)[^.\n]{0,30}holidays",
    ),
    (
        "shift:overtime",
        "Overtime",
        r"(?:work|available for|availability for|willing to work|mandatory|required to work)[^.\n]{0,25}overtime|overtime (?:as needed|may be required|is required|required|as required)",
    ),
    ("shift:on_call", "On-call", r"on[- ]call"),
];

const TRAVEL: &[TagInfo] = &[
    TagInfo {
        value: "travel:minimal",
        label: "Minimal travel",
    },
    TagInfo {
        value: "travel:moderate",
        label: "Moderate travel",
    },
    TagInfo {
        value: "travel:extensive",
        label: "Extensive travel",
    },
];

const COMMIT: &[TagInfo] = &[
    TagInfo {
        value: "commit:temporary",
        label: "Temporary",
    },
    TagInfo {
        value: "commit:seasonal",
        label: "Seasonal",
    },
];

const MISC: &[TagInfo] = &[
    TagInfo {
        value: "role:manager",
        label: "People manager",
    },
    TagInfo {
        value: "staffing",
        label: "Staffing agency",
    },
];

const EDUCATION: &[TagInfo] = &[
    TagInfo {
        value: "edu:associate:required",
        label: "Associate's required",
    },
    TagInfo {
        value: "edu:associate:preferred",
        label: "Associate's preferred",
    },
    TagInfo {
        value: "edu:bachelors:required",
        label: "Bachelor's required",
    },
    TagInfo {
        value: "edu:bachelors:preferred",
        label: "Bachelor's preferred",
    },
    TagInfo {
        value: "edu:masters:required",
        label: "Master's required",
    },
    TagInfo {
        value: "edu:masters:preferred",
        label: "Master's preferred",
    },
    TagInfo {
        value: "edu:doctorate:required",
        label: "Doctorate required",
    },
    TagInfo {
        value: "edu:doctorate:preferred",
        label: "Doctorate preferred",
    },
];

/// Degree levels, lowest first, with their patterns.
const DEGREES: &[(&str, &str)] = &[
    (
        "associate",
        r"\bassociate'?s? (?:degree|of (?:arts|science|applied))|\ba\.?a\.?s\.? degree|\b(?:2|two)[- ]year degree",
    ),
    (
        "bachelors",
        r"\bbachelor'?s?\b|\bb\.?s\.?(?: in| degree|/)|\bb\.?a\.?(?: in| degree|/)|\bbsc\b|\bundergraduate degree|\b(?:4|four)[- ]year (?:college |university )?degree|\bbs/ms\b|\bba/bs\b|\bbs/ba\b|\bbachelor",
    ),
    (
        "masters",
        r"\bmaster'?s?\b (?:degree|of|in)\b|\bmasters?\b ?(?:/|or) ?ph\.?d|\bm\.?s\.?(?: in| degree|/)|\bmba\b|\bm\.?eng\b|\bgraduate degree|\bms/phd\b|\bbs/ms\b|\bmsc\b|\bm\.a\.(?: in| degree)|\bmph\b|\bmsw\b|\bmsn\b|\bmpa\b",
    ),
    (
        "doctorate",
        r"\bph\.? ?d\b|\bdoctorate\b|\bdoctoral\b|\bdoctor of\b|\bms/phd\b|\bpharm\.? ?d\b|\bdnp\b|\bpsy\.? ?d\b|juris doctor|\bj\.d\.",
    ),
];

const CERTS: &[Rule] = &[
    (
        "cert:rn",
        "RN (Registered Nurse)",
        r"\brn\b|registered nurse",
    ),
    (
        "cert:lpn",
        "LPN / LVN",
        r"\blpn\b|\blvn\b|licensed (?:practical|vocational) nurse",
    ),
    (
        "cert:cna",
        "CNA",
        r"\bcna\b|certified nurs(?:e|ing) (?:aide|assistant)",
    ),
    (
        "cert:np",
        "Nurse Practitioner (NP)",
        r"nurse practitioner|\baprn\b|\bfnp\b|\bpmhnp\b",
    ),
    (
        "cert:pa",
        "Physician Assistant (PA-C)",
        r"\bpa-c\b|physician assistant",
    ),
    (
        "cert:md",
        "Medical license (MD/DO)",
        r"medical licen[sc]e|board[- ]eligible|board[- ]certified (?:in|or|/|internal|family|pediatric|psychiatr|emergency|general|anesthes|radiolog|ob)|\b(?:md|do) licen[sc]e",
    ),
    ("cert:bls", "BLS", r"\bbls\b|basic life support"),
    (
        "cert:acls",
        "ACLS",
        r"\bacls\b|advanced cardiac life support",
    ),
    ("cert:pals", "PALS", r"\bpals\b"),
    ("cert:cpr", "CPR / First Aid", r"\bcpr\b|first aid certif"),
    (
        "cert:pharmacist",
        "Pharmacist license",
        r"\brph\b|licensed pharmacist|pharmacist licen",
    ),
    (
        "cert:cpht",
        "Pharmacy technician (CPhT)",
        r"\bcpht\b|pharmacy technician (?:certification|license|licence)|ptcb",
    ),
    ("cert:emt", "EMT / Paramedic", r"\bemt\b|paramedic"),
    (
        "cert:lcsw",
        "LCSW / LMSW",
        r"\blcsw\b|\blmsw\b|licensed (?:clinical|master) social worker",
    ),
    (
        "cert:lpc",
        "LPC / LMHC",
        r"\blpcc?\b|\blmhc\b|licensed professional counselor|licensed mental health counselor",
    ),
    ("cert:lmft", "LMFT", r"\blmft\b|licensed marriage"),
    ("cert:bcba", "BCBA", r"\bbcba\b"),
    ("cert:rbt", "RBT", r"\brbt\b|registered behavior technician"),
    (
        "cert:rrt",
        "Respiratory therapist (RRT)",
        r"\brrt\b|registered respiratory therapist|\bcrt\b",
    ),
    (
        "cert:rd",
        "Registered Dietitian",
        r"registered dietitian|\brdn\b",
    ),
    (
        "cert:pt",
        "Physical / occupational therapy license",
        r"licensed (?:physical|occupational) therapist|\bdpt\b|\botr/?l\b",
    ),
    (
        "cert:slp",
        "SLP (CCC-SLP)",
        r"ccc-slp|speech[- ]language pathologist licen",
    ),
    (
        "cert:phlebotomy",
        "Phlebotomy",
        r"phlebotomy (?:certification|certificate|license)|certified phlebotom",
    ),
    ("cert:cpa", "CPA", r"\bcpa\b|certified public accountant"),
    (
        "cert:cma",
        "CMA (management accountant)",
        r"certified management accountant",
    ),
    ("cert:cfa", "CFA", r"\bcfa\b|chartered financial analyst"),
    ("cert:cfp", "CFP", r"\bcfp\b|certified financial planner"),
    (
        "cert:cia",
        "CIA (internal auditor)",
        r"certified internal auditor",
    ),
    ("cert:cisa", "CISA", r"\bcisa\b"),
    (
        "cert:finra",
        "FINRA (Series 7, 63...)",
        r"series (?:6|7|24|63|65|66|79)\b|finra licen|\bsie\b exam",
    ),
    (
        "cert:insurance",
        "Insurance license",
        r"(?:life|health|property|casualty|p&c|insurance|adjuster)(?: and \w+)? licen[sc]e|producer licen[sc]e|licensed (?:insurance )?agent",
    ),
    (
        "cert:real_estate",
        "Real estate license",
        r"real estate licen[sc]e|licensed real estate",
    ),
    (
        "cert:bar",
        "Bar admission",
        r"bar admission|admitted to (?:practice|the bar)|licensed (?:to practice law|attorney)|member (?:in good standing )?of the [\w ]{0,30}bar|active bar",
    ),
    ("cert:notary", "Notary", r"\bnotary\b"),
    (
        "cert:pe",
        "PE (Professional Engineer)",
        r"\bp\.?e\.? licen|professional engineer\b|\bpe (?:license|licensure|certification|stamp)",
    ),
    (
        "cert:eit",
        "EIT / FE",
        r"\beit\b|\bfe exam|engineer[- ]in[- ]training",
    ),
    ("cert:pmp", "PMP", r"\bpmp\b"),
    ("cert:capm", "CAPM", r"\bcapm\b"),
    (
        "cert:scrum",
        "Scrum (CSM, PSM)",
        r"\bcsm\b|certified scrum ?master|\bpsm\b|\bsafe agilist|\bcspo\b",
    ),
    ("cert:six_sigma", "Six Sigma", r"six sigma|lean six"),
    ("cert:itil", "ITIL", r"\bitil\b"),
    ("cert:cissp", "CISSP", r"\bcissp\b"),
    ("cert:cism", "CISM", r"\bcism\b"),
    (
        "cert:security_plus",
        "CompTIA Security+",
        r"security ?\+|security plus|comptia sec",
    ),
    (
        "cert:comptia_a",
        "CompTIA A+",
        r"comptia a ?\+|\ba\+ certif",
    ),
    (
        "cert:network_plus",
        "CompTIA Network+",
        r"network ?\+|comptia network",
    ),
    ("cert:ccna", "Cisco (CCNA, CCNP)", r"\bccn[ap]\b|\bccie\b"),
    ("cert:oscp", "OSCP", r"\boscp\b"),
    ("cert:ceh", "CEH", r"\bceh\b|certified ethical hacker"),
    ("cert:giac", "GIAC", r"\bgiac\b|\bgsec\b|\bgcih\b"),
    (
        "cert:aws",
        "AWS certification",
        r"aws certifi|aws certified|aws solutions architect (?:certif|associate|professional)",
    ),
    (
        "cert:azure",
        "Azure certification",
        r"azure (?:certification|certified)|microsoft certified|\baz-\d{3}\b",
    ),
    (
        "cert:gcp",
        "Google Cloud certification",
        r"google cloud certified|gcp certifi|professional cloud architect",
    ),
    (
        "cert:kubernetes",
        "Kubernetes (CKA, CKAD)",
        r"\bcka\b|\bckad\b|certified kubernetes",
    ),
    (
        "cert:salesforce",
        "Salesforce certification",
        r"salesforce (?:certified|certification)",
    ),
    ("cert:cdl", "CDL", r"\bcdl\b|commercial driver'?s? licen"),
    (
        "cert:drivers_license",
        "Driver's license",
        r"(?:valid|current|clean)[^.\n]{0,25}driver'?s'? licen|driver'?s'? licen[sc]e (?:is )?required|driving licen[sc]e",
    ),
    (
        "cert:forklift",
        "Forklift",
        r"forklift (?:certif|licen|operator certif)",
    ),
    ("cert:osha", "OSHA", r"\bosha[- ]?(?:10|30)\b|osha certif"),
    (
        "cert:epa",
        "EPA 608",
        r"\bepa (?:section )?608|epa certif|universal certification",
    ),
    (
        "cert:electrician",
        "Electrician license",
        r"journeyman (?:electrician|license)|master electrician|electrical licen[sc]e|licensed electrician",
    ),
    (
        "cert:hvac",
        "HVAC license",
        r"hvac (?:license|certification)|nate certif",
    ),
    (
        "cert:teaching",
        "Teaching license",
        r"teaching (?:license|licence|certificate|certification|credential)|teacher (?:certification|license)|state certification in",
    ),
    (
        "cert:shrm",
        "SHRM / HRCI",
        r"\bshrm-(?:cp|scp)\b|\bphr\b|\bsphr\b",
    ),
    (
        "cert:food",
        "Food handler / ServSafe",
        r"servsafe|food handler",
    ),
    (
        "cert:security_guard",
        "Guard card",
        r"guard card|security guard licen|security officer licen",
    ),
    (
        "cert:cpc",
        "Medical coding (CPC)",
        r"\bcpc\b|\bccs\b|certified professional coder|\brhit\b|\brhia\b",
    ),
];

const LANGUAGES: &[Rule] = &[
    ("lang:spanish", "Spanish", r"spanish"),
    ("lang:french", "French", r"\bfrench\b"),
    ("lang:german", "German", r"\bgerman\b"),
    ("lang:mandarin", "Mandarin Chinese", r"mandarin|\bchinese\b"),
    ("lang:cantonese", "Cantonese", r"cantonese"),
    ("lang:japanese", "Japanese", r"japanese"),
    ("lang:korean", "Korean", r"\bkorean\b"),
    ("lang:portuguese", "Portuguese", r"portuguese"),
    ("lang:italian", "Italian", r"\bitalian\b"),
    ("lang:russian", "Russian", r"\brussian\b"),
    ("lang:arabic", "Arabic", r"\barabic\b"),
    ("lang:hindi", "Hindi", r"\bhindi\b"),
    ("lang:vietnamese", "Vietnamese", r"vietnamese"),
    ("lang:tagalog", "Tagalog / Filipino", r"tagalog|filipino"),
    ("lang:polish", "Polish", r"\bpolish\b"),
    ("lang:dutch", "Dutch", r"\bdutch\b"),
    ("lang:turkish", "Turkish", r"\bturkish\b"),
    ("lang:hebrew", "Hebrew", r"\bhebrew\b"),
    ("lang:ukrainian", "Ukrainian", r"ukrainian"),
    (
        "lang:haitian_creole",
        "Haitian Creole",
        r"haitian|\bcreole\b",
    ),
    ("lang:urdu", "Urdu", r"\burdu\b"),
    ("lang:bengali", "Bengali", r"bengali|\bbangla\b"),
    ("lang:punjabi", "Punjabi", r"punjabi"),
    ("lang:farsi", "Farsi / Persian", r"\bfarsi\b|\bpersian\b"),
    ("lang:somali", "Somali", r"\bsomali\b"),
    ("lang:swahili", "Swahili", r"swahili"),
    ("lang:greek", "Greek", r"\bgreek\b"),
    ("lang:thai", "Thai", r"\bthai\b"),
    ("lang:hmong", "Hmong", r"\bhmong\b"),
    ("lang:amharic", "Amharic", r"amharic"),
    (
        "lang:asl",
        "American Sign Language",
        r"american sign language|\basl\b|sign language",
    ),
];

/// A language only counts in a sentence about speaking it.
static LANGUAGE_CONTEXT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"fluen|bilingual|proficien|speak|spoken|verbal|written|native|language|multilingual|conversational|read and write|interpret|translat|\bin english and\b").unwrap()
});

const SKILLS: &[Rule] = &[
    // Languages.
    ("skill:python", "Python", r"\bpython\b"),
    ("skill:java", "Java", r"\bjava\b"),
    (
        "skill:javascript",
        "JavaScript",
        r"\bjavascript\b|\bes6\b|ecmascript",
    ),
    ("skill:typescript", "TypeScript", r"\btypescript\b"),
    ("skill:csharp", "C#", r"(?:^|[^a-z0-9])c#|\bc sharp\b"),
    ("skill:cpp", "C++", r"(?:^|[^a-z0-9])c\+\+"),
    (
        "skill:c",
        "C",
        r"\bc ?/ ?c\+\+|\bc, ?c\+\+|\bc and c\+\+|\bc programming|\bansi c\b|\bembedded c\b|\bc language",
    ),
    (
        "skill:go",
        "Go",
        r"\bgolang\b|\bgo ?\(golang\)|\bgo programming|(?:python|java|rust|ruby|scala|kotlin|c\+\+|typescript|node(?:\.js)?) ?(?:,|/|or|and) ?go\b|\bgo ?(?:,|/) ?(?:python|java|rust|ruby|scala|kotlin|c\+\+|typescript|node)",
    ),
    ("skill:rust", "Rust", r"\brust\b"),
    ("skill:ruby", "Ruby", r"\bruby\b"),
    ("skill:php", "PHP", r"\bphp\b"),
    ("skill:swift", "Swift", r"\bswift(?:ui)?\b"),
    ("skill:kotlin", "Kotlin", r"\bkotlin\b"),
    ("skill:scala", "Scala", r"\bscala\b"),
    (
        "skill:r",
        "R",
        r"\br (?:programming|language|studio)\b|\brstudio\b|(?:python|sas|sql|stata|matlab|spss) ?(?:,|/|or|and) ?r\b|\br ?(?:,|/) ?(?:python|sql|sas|stata)\b",
    ),
    ("skill:matlab", "MATLAB", r"\bmatlab\b"),
    ("skill:perl", "Perl", r"\bperl\b"),
    ("skill:bash", "Bash / shell", r"\bbash\b|shell script"),
    ("skill:powershell", "PowerShell", r"powershell"),
    ("skill:sql", "SQL", r"\bsql\b"),
    ("skill:nosql", "NoSQL", r"\bnosql\b"),
    ("skill:graphql", "GraphQL", r"graphql"),
    ("skill:html", "HTML", r"\bhtml5?\b"),
    ("skill:css", "CSS", r"\bcss3?\b|\bsass\b|\bscss\b"),
    ("skill:elixir", "Elixir", r"\belixir\b"),
    ("skill:haskell", "Haskell", r"haskell"),
    ("skill:clojure", "Clojure", r"clojure"),
    ("skill:dart", "Dart", r"\bdart\b"),
    ("skill:objective_c", "Objective-C", r"objective-c"),
    ("skill:lua", "Lua", r"\blua\b"),
    ("skill:solidity", "Solidity", r"solidity"),
    ("skill:vba", "VBA", r"\bvba\b"),
    ("skill:cobol", "COBOL", r"\bcobol\b"),
    ("skill:sas", "SAS", r"\bsas\b"),
    ("skill:abap", "ABAP", r"\babap\b"),
    ("skill:apex", "Apex (Salesforce)", r"\bapex\b"),
    // Frameworks and libraries.
    ("skill:react", "React", r"\breact(?:\.js|js)?\b"),
    ("skill:react_native", "React Native", r"react native"),
    ("skill:angular", "Angular", r"\bangular(?:js)?\b"),
    ("skill:vue", "Vue", r"\bvue(?:\.js|js)?\b"),
    ("skill:svelte", "Svelte", r"svelte"),
    ("skill:nextjs", "Next.js", r"next\.js|\bnextjs\b"),
    (
        "skill:node",
        "Node.js",
        r"node\.?js|\bnode ?(?:,|/)|(?:,|/) ?node\b",
    ),
    ("skill:express", "Express", r"express\.js|\bexpressjs\b"),
    ("skill:django", "Django", r"django"),
    ("skill:flask", "Flask", r"\bflask\b"),
    ("skill:fastapi", "FastAPI", r"fastapi"),
    ("skill:rails", "Ruby on Rails", r"ruby on rails|\brails\b"),
    (
        "skill:spring",
        "Spring",
        r"spring boot|spring framework|\bspring (?:mvc|cloud|data|security)\b",
    ),
    ("skill:dotnet", ".NET", r"\.net\b|asp\.net|dotnet"),
    ("skill:laravel", "Laravel", r"laravel"),
    ("skill:jquery", "jQuery", r"jquery"),
    ("skill:tailwind", "Tailwind CSS", r"tailwind"),
    ("skill:redux", "Redux", r"\bredux\b"),
    ("skill:grpc", "gRPC", r"\bgrpc\b"),
    (
        "skill:rest",
        "REST APIs",
        r"\brest(?:ful)? apis?\b|\brestful\b",
    ),
    ("skill:microservices", "Microservices", r"microservice"),
    ("skill:kafka", "Kafka", r"\bkafka\b"),
    ("skill:rabbitmq", "RabbitMQ", r"rabbitmq"),
    (
        "skill:spark",
        "Spark",
        r"apache spark|pyspark|spark sql|spark streaming|\bspark ?(?:,|/)|(?:,|/) ?spark\b",
    ),
    ("skill:hadoop", "Hadoop", r"hadoop"),
    ("skill:airflow", "Airflow", r"airflow"),
    ("skill:dbt", "dbt", r"\bdbt\b"),
    ("skill:pandas", "pandas", r"\bpandas\b"),
    ("skill:numpy", "NumPy", r"numpy"),
    (
        "skill:scikit_learn",
        "scikit-learn",
        r"scikit-learn|sklearn|scikit learn",
    ),
    ("skill:tensorflow", "TensorFlow", r"tensorflow"),
    ("skill:pytorch", "PyTorch", r"pytorch"),
    ("skill:keras", "Keras", r"\bkeras\b"),
    ("skill:hugging_face", "Hugging Face", r"hugging ?face"),
    ("skill:llm", "LLMs", r"\bllms?\b|large language model"),
    ("skill:langchain", "LangChain", r"langchain"),
    ("skill:nlp", "NLP", r"\bnlp\b|natural language processing"),
    (
        "skill:computer_vision",
        "Computer vision",
        r"computer vision|opencv",
    ),
    ("skill:cuda", "CUDA", r"\bcuda\b"),
    ("skill:selenium", "Selenium", r"selenium"),
    ("skill:cypress", "Cypress", r"\bcypress\b"),
    ("skill:playwright", "Playwright", r"playwright"),
    ("skill:jest", "Jest", r"\bjest\b"),
    ("skill:junit", "JUnit", r"\bjunit\b"),
    ("skill:postman", "Postman", r"\bpostman\b"),
    ("skill:webpack", "Webpack", r"webpack"),
    ("skill:d3", "D3.js", r"\bd3(?:\.js)?\b"),
    ("skill:threejs", "Three.js", r"three\.js|threejs"),
    (
        "skill:unity",
        "Unity",
        r"\bunity3d\b|\bunity (?:engine|game|developer|c#)|\bunity ?(?:,|/|or|and) ?unreal",
    ),
    (
        "skill:unreal",
        "Unreal Engine",
        r"unreal engine|\bunreal\b|\bue[45]\b",
    ),
    ("skill:ios", "iOS", r"\bios\b"),
    ("skill:android", "Android", r"\bandroid\b"),
    // Data stores.
    ("skill:postgres", "PostgreSQL", r"postgres(?:ql)?"),
    ("skill:mysql", "MySQL", r"mysql"),
    ("skill:mongodb", "MongoDB", r"\bmongo(?:db)?\b"),
    ("skill:redis", "Redis", r"\bredis\b"),
    (
        "skill:elasticsearch",
        "Elasticsearch",
        r"elastic ?search|opensearch|\belk\b",
    ),
    ("skill:cassandra", "Cassandra", r"cassandra"),
    ("skill:dynamodb", "DynamoDB", r"dynamodb"),
    (
        "skill:oracle_db",
        "Oracle Database",
        r"oracle (?:database|db|sql)|pl/sql",
    ),
    (
        "skill:sql_server",
        "SQL Server",
        r"sql server|\bmssql\b|t-sql",
    ),
    ("skill:snowflake", "Snowflake", r"snowflake"),
    ("skill:databricks", "Databricks", r"databricks"),
    ("skill:bigquery", "BigQuery", r"bigquery"),
    ("skill:redshift", "Redshift", r"redshift"),
    // Cloud and operations.
    ("skill:aws", "AWS", r"\baws\b|amazon web services"),
    ("skill:azure", "Azure", r"\bazure\b"),
    ("skill:gcp", "Google Cloud", r"\bgcp\b|google cloud"),
    ("skill:docker", "Docker", r"docker"),
    (
        "skill:kubernetes",
        "Kubernetes",
        r"kubernetes|\bk8s\b|\beks\b|\bgke\b|\baks\b",
    ),
    ("skill:terraform", "Terraform", r"terraform"),
    ("skill:ansible", "Ansible", r"ansible"),
    ("skill:jenkins", "Jenkins", r"jenkins"),
    ("skill:github_actions", "GitHub Actions", r"github actions"),
    ("skill:gitlab", "GitLab", r"gitlab"),
    (
        "skill:ci_cd",
        "CI/CD",
        r"\bci ?/ ?cd\b|continuous integration|continuous delivery|continuous deployment",
    ),
    ("skill:linux", "Linux", r"\blinux\b|\bunix\b"),
    ("skill:git", "Git", r"\bgit\b"),
    ("skill:helm", "Helm", r"\bhelm\b"),
    ("skill:prometheus", "Prometheus", r"prometheus"),
    ("skill:grafana", "Grafana", r"grafana"),
    ("skill:datadog", "Datadog", r"datadog"),
    ("skill:splunk", "Splunk", r"splunk"),
    (
        "skill:serverless",
        "Serverless / Lambda",
        r"serverless|aws lambda",
    ),
    ("skill:vmware", "VMware", r"vmware"),
    (
        "skill:active_directory",
        "Active Directory",
        r"active directory|entra id",
    ),
    ("skill:okta", "Okta", r"\bokta\b"),
    ("skill:cisco", "Cisco networking", r"\bcisco\b"),
    ("skill:siem", "SIEM", r"\bsiem\b"),
    // Business tools.
    (
        "skill:excel",
        "Excel",
        r"microsoft excel|ms excel|advanced excel|\bexcel ?(?:,|/|\(|spreadsheets?|skills|proficien|formulas|vlookup|pivot|macros)|(?:word|outlook|powerpoint|google sheets) ?(?:,|/|and) ?excel\b|microsoft office suite \(.{0,30}excel",
    ),
    ("skill:tableau", "Tableau", r"tableau"),
    ("skill:power_bi", "Power BI", r"power ?bi\b"),
    ("skill:looker", "Looker", r"\blooker\b"),
    ("skill:salesforce", "Salesforce", r"salesforce"),
    ("skill:hubspot", "HubSpot", r"hubspot"),
    ("skill:sap", "SAP", r"\bsap\b"),
    ("skill:netsuite", "NetSuite", r"netsuite"),
    ("skill:quickbooks", "QuickBooks", r"quickbooks"),
    (
        "skill:workday",
        "Workday (software)",
        r"workday (?:hcm|financials|integrations?|studio|reports?|recruiting|payroll)",
    ),
    ("skill:servicenow", "ServiceNow", r"servicenow"),
    ("skill:jira", "Jira", r"\bjira\b"),
    ("skill:confluence", "Confluence", r"confluence"),
    ("skill:sharepoint", "SharePoint", r"sharepoint"),
    (
        "skill:epic",
        "Epic (EHR)",
        r"\bepic (?:emr|ehr|systems|certif|certified|experience)|\bepic ?(?:,|/|or) ?cerner",
    ),
    ("skill:cerner", "Cerner", r"cerner"),
    ("skill:figma", "Figma", r"figma"),
    (
        "skill:adobe",
        "Adobe Creative Cloud",
        r"adobe creative|photoshop|indesign|after effects|premiere pro|adobe illustrator",
    ),
    ("skill:agile", "Agile / Scrum", r"\bagile\b|\bscrum\b"),
    // Engineering tools.
    ("skill:autocad", "AutoCAD", r"autocad"),
    ("skill:solidworks", "SolidWorks", r"solidworks"),
    ("skill:revit", "Revit", r"\brevit\b"),
    ("skill:catia", "CATIA", r"\bcatia\b"),
    ("skill:ansys", "ANSYS", r"\bansys\b"),
    ("skill:labview", "LabVIEW", r"labview"),
    (
        "skill:plc",
        "PLCs",
        r"\bplcs?\b|programmable logic controller",
    ),
    ("skill:verilog", "Verilog / SystemVerilog", r"verilog"),
    ("skill:vhdl", "VHDL", r"\bvhdl\b"),
];

// --- compiled rules -------------------------------------------------------------

fn set(rules: &[Rule]) -> RegexSet {
    RegexSet::new(rules.iter().map(|r| r.2)).expect("extraction rules compile")
}

static PERK_SET: LazyLock<RegexSet> = LazyLock::new(|| set(PERKS));
static CLEARANCE_SET: LazyLock<RegexSet> = LazyLock::new(|| set(CLEARANCES));
static SHIFT_SET: LazyLock<RegexSet> = LazyLock::new(|| set(SHIFTS));
static CERT_SET: LazyLock<RegexSet> = LazyLock::new(|| set(CERTS));
static LANGUAGE_SET: LazyLock<RegexSet> = LazyLock::new(|| set(LANGUAGES));
static SKILL_SET: LazyLock<RegexSet> = LazyLock::new(|| set(SKILLS));
static PERK_RES: LazyLock<Vec<Regex>> =
    LazyLock::new(|| PERKS.iter().map(|r| Regex::new(r.2).unwrap()).collect());
static SHIFT_RES: LazyLock<Vec<Regex>> =
    LazyLock::new(|| SHIFTS.iter().map(|r| Regex::new(r.2).unwrap()).collect());
static DEGREE_RES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    DEGREES
        .iter()
        .map(|(_, p)| Regex::new(p).unwrap())
        .collect()
});

fn re(p: &str) -> Regex {
    Regex::new(p).expect("extraction rule compiles")
}

/// Equal-opportunity and affirmative-action wording.
static BOILERPLATE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"equal (?:employment )?opportunit|without regard|regardless of|protected (?:veteran|class|status|characteristic)|affirmative action|\beeo\b|minorit|\bwomen\b|\bfemales?\b|disabilit|\bdisabled\b|sexual orientation|gender identity|national origin|vevraa|\bofccp\b",
    )
});

static NEGATIVE: LazyLock<Regex> =
    LazyLock::new(|| re(r"\bnot\b|\bno\b|n't\b|\bunable\b|\bcannot\b|\bwithout\b|\bineligible\b"));

static PREFERRED: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"prefer|a plus|nice to have|bonus|desired|desirable|\bideal|or equivalent|equivalent (?:work |practical |professional )?(?:experience|combination)|in lieu|or related experience|or comparable experience|advantage|helpful|beneficial|optional",
    )
});

/// A short line opening a list of nice-to-haves, or of requirements.
static PREFERRED_HEADER: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"^(?:preferred|nice[- ]to[- ]have|bonus|desired|desirable|pluses|ideal(?:ly)?|extra credit)|(?:preferred|desired|bonus|additional) (?:qualifications|skills|experience|requirements)|nice to haves?|bonus points|you might also have|it'?s a plus",
    )
});
static REQUIRED_HEADER: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?:minimum|basic|required|must[- ]have|key) (?:qualifications|requirements|skills|experience)|^(?:requirements|qualifications|what you(?:'ll| will)? (?:need|bring)|who you are|about you|responsibilities|what you'?ll do)",
    )
});
static DEGREE_NOISE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"programs like|alumni|founders?\b|our team|team (?:of|includes|members)|backgrounds? (?:from|include)|we hire",
    )
});

static VETERANS: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?:veterans?|military|service members?)[^.\n]{0,60}(?:encourag|welcome|invite)|(?:encourag|welcome|invite)[^.\n]{0,60}(?:veterans?|military)|veteran[- ]friendly|skillbridge|hire our heroes|transitioning (?:service members|military)",
    )
});

static ABOUT_APPLYING: LazyLock<Regex> = LazyLock::new(|| {
    re(r"apply|applicant|candidate|\bhire|hiring|\bjoin|skillbridge|hire our heroes")
});

static FAIR_CHANCE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"fair chance|second chance|arrest (?:and|or) conviction records|criminal (?:histor|record)[^.\n]{0,80}(?:consider|qualified applicants)|(?:consider|qualified applicants)[^.\n]{0,80}criminal (?:histor|record)|justice[- ]involved|returning citizens|formerly incarcerated",
    )
});

static TITLE_MANAGER: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"\b(?:director|head of|vp|svp|evp|vice president|chief|supervisor|superintendent|foreman|general manager|people manager|engineering manager|store manager|branch manager|office manager|nurse manager|operations manager|shift manager|restaurant manager|plant manager|district manager|regional manager|manager,? (?:software|engineering|data|people|machine learning|infrastructure))\b",
    )
});

static TEXT_MANAGER: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"direct reports|manag(?:e|ing) (?:a|the) team of|managing a team|lead(?:ing)? a team of|people management|people manager|supervis(?:e|ing) (?:a team|staff|employees|\d+)|hire,? (?:develop|coach|mentor)|hiring,? (?:developing|coaching|mentoring)|build and (?:lead|manage) (?:a|the|your) team|performance (?:reviews|evaluations) for",
    )
});

static TRAVEL_PERCENT: LazyLock<Regex> =
    LazyLock::new(|| re(r"(\d{1,3}) ?%[^.\n%]{0,40}?\btravel|\btravel[^.\n%]{0,50}?(\d{1,3}) ?%"));
static TRAVEL_MINIMAL: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?:minimal|occasional|limited|infrequent|light|some) (?:amount of )?(?:overnight )?travel|travel (?:is |will be )?(?:minimal|occasional|rare|infrequent)",
    )
});
static TRAVEL_MODERATE: LazyLock<Regex> =
    LazyLock::new(|| re(r"moderate travel|regular travel|travel regularly"));
static TRAVEL_EXTENSIVE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?:extensive|frequent|significant|heavy|substantial|considerable) (?:amount of )?travel|travel(?:ing)? (?:extensively|frequently)|road warrior",
    )
});

static STAFFING: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"on behalf of (?:our|a|one of our) (?:valued |esteemed )?client|\bour (?:valued |esteemed |prestigious )?client (?:is|has|in|located|seeks|is seeking|needs)|about our client|we are (?:a|an) (?:leading |national |premier |specialized )?(?:staffing|recruiting|recruitment|placement)|(?:is|as) a (?:leading |national |premier )?(?:staffing|recruiting|recruitment) (?:agency|firm|company)|this (?:position|role|job|opportunity) is (?:with|through) (?:a|our) (?:staffing|recruiting|client)",
    )
});
static NOT_STAFFING: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"not accept|unsolicited|third[- ]part|no agenc|agencies,? please|do not (?:contact|submit|send)|without a (?:signed|written)|recruiters,? please|not be responsible for|if contacted|scam|fraud|impersonat",
    )
});

static TEMP_TITLE: LazyLock<Regex> = LazyLock::new(|| re(r"\btemp(?:orary)?\b|fixed[- ]term"));
static TEMP_TEXT: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?:this is a|is a) (?:temporary|temp|fixed[- ]term)\b|temporary (?:position|role|assignment|employment|job|opportunity)|fixed[- ]term (?:contract|position|role)",
    )
});
static SEASONAL_TITLE: LazyLock<Regex> =
    LazyLock::new(|| re(r"\bseasonal\b|holiday (?:help|season)"));
static SEASONAL_TEXT: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"seasonal (?:position|role|employment|job|opportunity|associate|worker)|(?:this is a|is a) seasonal",
    )
});

const NUMBER: &str =
    r"\d{1,2}|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|fifteen";

static YEARS: LazyLock<Regex> = LazyLock::new(|| {
    re(&format!(
        r"\b({NUMBER}) ?(?:\(\d{{1,2}}\) ?)?(?:\+|plus)? ?(?:(?:-|–|to|or) ?(?:{NUMBER}) ?(?:\(\d{{1,2}}\) ?)?\+? ?)?(?:years?|yrs?)'? (?:of )?(?:[a-z/&,+.-]+ ){{0,5}}?(?:experience|exp\b|work(?:ing)? in|in (?:a|an) )"
    ))
});
static YEARS_AFTER: LazyLock<Regex> =
    LazyLock::new(|| re(r"experience ?(?:of|:|-|–)? ?(\d{1,2}) ?\+? ?(?:years?|yrs?)\b"));
/// "With over 20 years of experience, we..." is about the company.
static YEARS_COMPANY: LazyLock<Regex> = LazyLock::new(|| {
    re(r"\bwe\b|\bour\b|\bcompany\b|\bfirm\b|\bteam has\b|founded|\bhas been\b|\bhave been\b")
});
static NO_EXPERIENCE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"no (?:prior |previous |professional |work )?experience (?:is )?(?:required|necessary|needed)|no experience\b",
    )
});

static GRAD: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"graduat|class of|degree (?:completion|conferred)|completing (?:your|a|their) degree|grad date",
    )
});
static YEAR: LazyLock<Regex> = LazyLock::new(|| re(r"\b(20(?:2[4-9]|3[0-2]))\b"));
static SEASON_YEAR: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"\b(summer|fall|autumn|spring|winter) ?(?:of )?'?(20(?:2[4-9]|3[0-2]))\b|\b(20(?:2[4-9]|3[0-2])) (summer|fall|autumn|spring|winter)\b",
    )
});

// --- extraction -----------------------------------------------------------------

fn word_number(s: &str) -> Option<i16> {
    Some(match s {
        "one" => 1,
        "two" => 2,
        "three" => 3,
        "four" => 4,
        "five" => 5,
        "six" => 6,
        "seven" => 7,
        "eight" => 8,
        "nine" => 9,
        "ten" => 10,
        "eleven" => 11,
        "twelve" => 12,
        "fifteen" => 15,
        _ => return s.parse().ok(),
    })
}

/// Lowercased plain ASCII: curly quotes made straight, bullets as line
/// breaks, other non-ASCII characters as spaces, spaces collapsed. ASCII
/// keeps the regex engine on its fast path (its word boundaries are
/// Unicode-aware, and a non-ASCII byte would push it onto a much slower one).
fn prepare(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut space = false;
    for c in text.chars() {
        let c = match c {
            '\u{2018}' | '\u{2019}' | '\u{02bc}' => '\'',
            '\u{201c}' | '\u{201d}' => '"',
            '\u{2013}' | '\u{2014}' => '-',
            '\u{2022}' | '\u{00b7}' | '\n' => '\n',
            c if c.is_ascii_graphic() => c,
            _ => ' ',
        };
        if c == ' ' {
            if !space {
                out.push(' ');
            }
            space = true;
        } else {
            space = false;
            out.extend(c.to_lowercase());
        }
    }
    out
}

/// Rough sentences: lines and bullet points, split again at ". ".
fn sentences(text: &str) -> Vec<&str> {
    text.split(['\n', ';', '|'])
        .flat_map(|line| line.split(". "))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

struct Tags(Vec<String>);

impl Tags {
    fn add(&mut self, tag: impl Into<String>) {
        let tag = tag.into();
        if !self.0.contains(&tag) {
            self.0.push(tag);
        }
    }
}

/// Reads tags, years of experience and the job function from a posting.
/// `internship` adds the internship's start season (`start:2026-summer`).
pub fn extract(title: &str, department: Option<&str>, text: &str, internship: bool) -> Extracted {
    let title_l = prepare(title);
    let text_l = prepare(text);
    let all = format!("{title_l}\n{text_l}");
    let lines = sentences(&text_l);
    let mut tags = Tags(Vec::new());

    // Perks; a few need a sentence that isn't negative.
    for i in PERK_SET.matches(&text_l).iter() {
        let tag = PERKS[i].0;
        if NEGATABLE.contains(&tag) {
            let re = &PERK_RES[i];
            let positive = lines
                .iter()
                .any(|s| re.is_match(s) && !NEGATIVE.is_match(s));
            if !positive
                || (tag == "perk:visa_sponsorship" && normalize::restrictions(text).no_sponsorship)
            {
                continue;
            }
        }
        tags.add(tag);
    }

    // Clearance. "top secret" becomes one word so "secret" doesn't match it.
    let cleared = all.replace("top secret", "topsecret");
    let mut clearance = false;
    for i in CLEARANCE_SET.matches(&cleared).iter() {
        tags.add(CLEARANCES[i].0);
        clearance = true;
    }
    if clearance {
        tags.add("clearance:any");
    }

    // Licenses and certifications.
    let certs = CERT_SET.matches(&all);
    for i in certs.iter() {
        tags.add(CERTS[i].0);
    }
    if certs.matched_any() {
        tags.add("cert:any");
    }

    // Spoken languages, only in a sentence about speaking them.
    let mut language = false;
    for line in lines
        .iter()
        .filter(|s| LANGUAGE_CONTEXT.is_match(s) && !s.contains("poster"))
    {
        for i in LANGUAGE_SET.matches(line).iter() {
            tags.add(LANGUAGES[i].0);
            language = true;
        }
    }
    if language {
        tags.add("lang:any");
    }

    // Encouraged to apply, outside equal-opportunity boilerplate.
    if lines
        .iter()
        .any(|s| VETERANS.is_match(s) && ABOUT_APPLYING.is_match(s) && !BOILERPLATE.is_match(s))
    {
        tags.add("encourage:veterans");
    }
    if FAIR_CHANCE.is_match(&text_l) {
        tags.add("encourage:fair_chance");
    }

    // Shifts and schedules.
    for i in SHIFT_SET.matches(&all).iter() {
        let tag = SHIFTS[i].0;
        let re = &SHIFT_RES[i];
        let said = all
            .split('\n')
            .flat_map(|l| l.split(". "))
            .any(|s| re.is_match(s) && !NEGATIVE.is_match(s) && !s.contains("weekends off"));
        if said {
            tags.add(tag);
        }
    }

    // Travel: a stated percentage, else wording.
    let percent = TRAVEL_PERCENT
        .captures_iter(&text_l)
        .filter_map(|c| c.get(1).or_else(|| c.get(2)))
        .filter_map(|m| m.as_str().parse::<u32>().ok())
        .filter(|&n| n <= 100)
        .max();
    let travel = match percent {
        Some(0) => None,
        Some(n) if n <= 15 => Some("travel:minimal"),
        Some(n) if n <= 50 => Some("travel:moderate"),
        Some(_) => Some("travel:extensive"),
        None if TRAVEL_EXTENSIVE.is_match(&text_l) => Some("travel:extensive"),
        None if TRAVEL_MODERATE.is_match(&text_l) => Some("travel:moderate"),
        None if TRAVEL_MINIMAL.is_match(&text_l) => Some("travel:minimal"),
        None => None,
    };
    if let Some(t) = travel {
        tags.add(t);
    }

    // People manager.
    if TITLE_MANAGER.is_match(&title_l) || TEXT_MANAGER.is_match(&text_l) {
        tags.add("role:manager");
    }

    // Staffing agencies, when the posting says so.
    if lines
        .iter()
        .any(|s| STAFFING.is_match(s) && !NOT_STAFFING.is_match(s))
    {
        tags.add("staffing");
    }

    // Temporary and seasonal work.
    if TEMP_TITLE.is_match(&title_l) || TEMP_TEXT.is_match(&text_l) {
        tags.add("commit:temporary");
    }
    if SEASONAL_TITLE.is_match(&title_l) || SEASONAL_TEXT.is_match(&text_l) {
        tags.add("commit:seasonal");
    }

    // Education, per degree: required unless its sentence, or the header
    // above it ("Preferred qualifications"), softens it.
    let mut preferred_section = false;
    for s in &lines {
        if s.len() < 80 {
            if PREFERRED_HEADER.is_match(s) {
                preferred_section = true;
            } else if REQUIRED_HEADER.is_match(s) {
                preferred_section = false;
            }
        }
        if DEGREE_NOISE.is_match(s) {
            continue;
        }
        for (k, (level, _)) in DEGREES.iter().enumerate() {
            if DEGREE_RES[k].is_match(s) {
                let how = if preferred_section || PREFERRED.is_match(s) {
                    "preferred"
                } else {
                    "required"
                };
                tags.add(format!("edu:{level}:{how}"));
            }
        }
    }

    // Skills.
    for i in SKILL_SET.matches(&all).iter() {
        tags.add(SKILLS[i].0);
    }

    // Graduation years ("graduating between Dec 2025 and Jun 2026").
    for m in GRAD.find_iter(&text_l) {
        let end = (m.end() + 90).min(text_l.len());
        let end = (end..=text_l.len())
            .find(|&i| text_l.is_char_boundary(i))
            .unwrap_or(text_l.len());
        let window = &text_l[m.end()..end];
        let window = window.split(['\n', '.']).next().unwrap_or("");
        for y in YEAR.captures_iter(window) {
            tags.add(format!("grad:{}", &y[1]));
        }
    }

    // An internship's start season ("Summer 2026").
    if internship {
        let source = if SEASON_YEAR.is_match(&title_l) {
            &title_l
        } else {
            &text_l
        };
        if let Some(c) = SEASON_YEAR.captures(source) {
            let (season, year) = match (c.get(1), c.get(2), c.get(3), c.get(4)) {
                (Some(s), Some(y), _, _) => (s.as_str(), y.as_str()),
                (_, _, Some(y), Some(s)) => (s.as_str(), y.as_str()),
                _ => ("", ""),
            };
            if !season.is_empty() {
                let season = if season == "autumn" { "fall" } else { season };
                tags.add(format!("start:{year}-{season}"));
            }
        }
    }

    Extracted {
        min_years: min_years(&text_l),
        tags: tags.0,
        job_function: functions::classify(title, department),
    }
}

/// The smallest number of years of experience the posting asks for.
fn min_years(text: &str) -> Option<i16> {
    let mut best: Option<i16> = None;
    let mut keep = |n: i16| {
        if (0..=20).contains(&n) {
            best = Some(best.map_or(n, |b| b.min(n)));
        }
    };
    for c in YEARS.captures_iter(text) {
        let m = c.get(0).unwrap();
        let before = &text[..m.start()];
        let start = before.len().saturating_sub(40);
        let start = (start..=before.len())
            .find(|&i| before.is_char_boundary(i))
            .unwrap_or(0);
        if YEARS_COMPANY.is_match(&before[start..]) {
            continue;
        }
        if let Some(n) = word_number(&c[1]) {
            keep(n);
        }
    }
    for c in YEARS_AFTER.captures_iter(text) {
        if let Ok(n) = c[1].parse() {
            keep(n);
        }
    }
    if NO_EXPERIENCE.is_match(text) {
        keep(0);
    }
    best
}

// --- vocabulary for the filters --------------------------------------------------

fn infos(rules: &'static [Rule]) -> impl Iterator<Item = TagInfo> {
    rules
        .iter()
        .map(|&(value, label, _)| TagInfo { value, label })
}

/// Every tag the filters offer, with labels. Grouped by prefix
/// (`perk:`, `skill:`...); `x:any` markers are left out.
pub fn vocabulary() -> Vec<TagInfo> {
    infos(PERKS)
        .chain(infos(CLEARANCES).filter(|t| t.value != "clearance:any"))
        .chain(infos(SHIFTS))
        .chain(infos(CERTS))
        .chain(infos(LANGUAGES))
        .chain(infos(SKILLS))
        .chain(ENCOURAGE.iter().copied())
        .chain(TRAVEL.iter().copied())
        .chain(COMMIT.iter().copied())
        .chain(MISC.iter().copied())
        .chain(EDUCATION.iter().copied())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(text: &str) -> Vec<String> {
        extract("Engineer", None, text, false).tags
    }

    fn has(text: &str, tag: &str) -> bool {
        tags(text).iter().any(|t| t == tag)
    }

    #[test]
    fn rules_compile() {
        let _ = (
            &*PERK_SET,
            &*CLEARANCE_SET,
            &*SHIFT_SET,
            &*CERT_SET,
            &*LANGUAGE_SET,
            &*SKILL_SET,
            &*DEGREE_RES,
        );
        let _ = (&*YEARS, &*SEASON_YEAR, &*TITLE_MANAGER, &*TEXT_MANAGER);
        assert!(vocabulary().len() > 200);
    }

    #[test]
    fn education() {
        let t = tags(
            "Requirements:\nBachelor's degree in Computer Science.\nMaster's degree preferred.",
        );
        assert!(t.contains(&"edu:bachelors:required".into()), "{t:?}");
        assert!(t.contains(&"edu:masters:preferred".into()), "{t:?}");
        assert!(has(
            "BS in Computer Science or equivalent experience",
            "edu:bachelors:preferred"
        ));
        assert!(has("PhD in Machine Learning", "edu:doctorate:required"));
        assert!(!has("We value a growth mindset", "edu:bachelors:required"));
        // "Master data" isn't a degree.
        assert!(!has(
            "Experience with master data management",
            "edu:masters:required"
        ));
    }

    #[test]
    fn education_sections() {
        let t = tags(
            "Minimum qualifications\nBachelor's degree in Engineering\nPreferred qualifications\nMaster's degree or Ph.D. in Physics",
        );
        assert!(t.contains(&"edu:bachelors:required".into()), "{t:?}");
        assert!(t.contains(&"edu:masters:preferred".into()), "{t:?}");
        assert!(t.contains(&"edu:doctorate:preferred".into()), "{t:?}");
        assert!(!has(
            "Our team includes alumni of leading PhD programs like Stanford.",
            "edu:doctorate:required"
        ));
    }

    #[test]
    fn perks() {
        assert!(has(
            "We offer a 401(k) with company match.",
            "perk:401k_match"
        ));
        assert!(has(
            "We offer a 401(k) with company match.",
            "perk:retirement"
        ));
        assert!(has(
            "Unlimited PTO and 16 weeks of paid parental leave",
            "perk:pto"
        ));
        assert!(has(
            "Unlimited PTO and 16 weeks of paid parental leave",
            "perk:parental_leave"
        ));
        assert!(has(
            "Relocation assistance is available.",
            "perk:relocation"
        ));
        assert!(!has(
            "Relocation assistance is not available.",
            "perk:relocation"
        ));
        assert!(has(
            "We are happy to sponsor visas for this role.",
            "perk:visa_sponsorship"
        ));
        assert!(!has(
            "We are not able to sponsor visas.",
            "perk:visa_sponsorship"
        ));
        // The family-leave law isn't a perk.
        assert!(!has(
            "Family and Medical Leave Act rights",
            "perk:parental_leave"
        ));
    }

    #[test]
    fn clearance() {
        let t = tags("Must hold an active TS/SCI clearance with polygraph.");
        assert!(t.contains(&"clearance:ts_sci".into()));
        assert!(t.contains(&"clearance:polygraph".into()));
        assert!(t.contains(&"clearance:any".into()));
        let t = tags("Active Top Secret clearance required.");
        assert!(
            t.contains(&"clearance:top_secret".into()) && !t.contains(&"clearance:secret".into()),
            "{t:?}"
        );
        assert!(has(
            "Must be able to obtain a Secret clearance.",
            "clearance:secret"
        ));
        assert!(!has("Customs clearance experience", "clearance:any"));
    }

    #[test]
    fn languages_need_context() {
        assert!(has(
            "Bilingual in English and Spanish preferred.",
            "lang:spanish"
        ));
        assert!(has("Fluency in Mandarin required.", "lang:mandarin"));
        assert!(!has("Experience with Java and Go.", "lang:any"));
        assert!(!has("Our French office is growing.", "lang:french"));
        assert!(!has(
            "E-Verify Participation Poster (Bilingual English & Spanish)",
            "lang:spanish"
        ));
    }

    #[test]
    fn veterans_not_from_boilerplate() {
        assert!(!has(
            "We are an equal opportunity employer. All qualified applicants will receive consideration without regard to protected veteran status.",
            "encourage:veterans"
        ));
        assert!(!has(
            "Women, minorities, veterans and individuals with disabilities are encouraged to apply.",
            "encourage:veterans"
        ));
        assert!(has(
            "Military veterans are strongly encouraged to apply.",
            "encourage:veterans"
        ));
        assert!(has("Military encouraged to apply.", "encourage:veterans"));
        assert!(!has(
            "We encourage qualified minority, female, veteran, and disabled candidates to apply.",
            "encourage:veterans"
        ));
        assert!(!has(
            "We help communities welcome military families.",
            "encourage:veterans"
        ));
        assert!(has(
            "We will consider for employment qualified applicants with arrest and conviction records.",
            "encourage:fair_chance"
        ));
    }

    #[test]
    fn shifts_and_travel() {
        assert!(has(
            "This role works night shift, including weekends as needed.",
            "shift:night"
        ));
        assert!(has(
            "This role works night shift, including weekends as needed.",
            "shift:weekend"
        ));
        assert!(!has("No weekends required!", "shift:weekend"));
        assert!(has("Participate in an on-call rotation.", "shift:on_call"));
        assert!(has("Travel up to 25% of the time.", "travel:moderate"));
        assert!(has("Travel: 75%", "travel:extensive"));
        assert!(has(
            "Occasional travel to customer sites.",
            "travel:minimal"
        ));
        assert!(!has("Travel stipend for the offsite.", "travel:minimal"));
    }

    #[test]
    fn certs_and_skills() {
        assert!(!has(
            "Supervised by a Board Certified Behavior Analyst (BCBA).",
            "cert:md"
        ));
        assert!(has(
            "Board Certified in Internal or Family Medicine",
            "cert:md"
        ));
        assert!(!has(
            "5 years of professional engineering experience",
            "cert:pe"
        ));
        let t = tags("Active RN license and BLS certification required. PMP a plus.");
        for want in ["cert:rn", "cert:bls", "cert:pmp", "cert:any"] {
            assert!(t.contains(&want.into()), "{want} in {t:?}");
        }
        let t = tags(
            "Experience with Python, Go, React, Node.js, PostgreSQL, AWS and Kubernetes. C++ and C# a plus.",
        );
        for want in [
            "skill:python",
            "skill:go",
            "skill:react",
            "skill:node",
            "skill:postgres",
            "skill:aws",
            "skill:kubernetes",
            "skill:cpp",
            "skill:csharp",
        ] {
            assert!(t.contains(&want.into()), "{want} in {t:?}");
        }
        assert!(!has("Let's go build something great.", "skill:go"));
        assert!(!has("You will excel in a fast-paced team.", "skill:excel"));
        assert!(has("Advanced Excel skills (pivot tables)", "skill:excel"));
        assert!(!has("Javascript experience", "skill:java"));
    }

    #[test]
    fn years() {
        let y = |t: &str| min_years(&prepare(t));
        assert_eq!(
            y("5+ years of professional software engineering experience"),
            Some(5)
        );
        assert_eq!(
            y("3-5 years of experience in data analysis; 2 years experience with SQL"),
            Some(2)
        );
        assert_eq!(
            y("A minimum of three (3) years of related work experience"),
            Some(3)
        );
        assert_eq!(y("Experience: 4+ years"), Some(4));
        assert_eq!(y("No experience required! We train you."), Some(0));
        assert_eq!(
            y("With over 25 years of experience, our company leads the field."),
            None
        );
        assert_eq!(y("We have 10 years of experience in the market."), None);
        assert_eq!(y("Must be 18 years of age."), None);
    }

    #[test]
    fn manager_staffing_temporary() {
        let e = extract("Director of Engineering", None, "", false);
        assert!(e.tags.contains(&"role:manager".into()));
        assert!(has(
            "You will manage a team of 6 engineers.",
            "role:manager"
        ));
        assert!(
            !extract("Product Manager", None, "Own the roadmap.", false)
                .tags
                .contains(&"role:manager".into())
        );
        assert!(has(
            "We are recruiting on behalf of our client, a leading bank.",
            "staffing"
        ));
        assert!(!has(
            "We do not accept unsolicited resumes from staffing agencies.",
            "staffing"
        ));
        assert!(!has(
            "If contacted by a recruiting agency for a role here, check their email.",
            "staffing"
        ));
        assert!(has(
            "Our client is a fast-growing software company.",
            "staffing"
        ));
        assert!(
            extract("Seasonal Warehouse Associate", None, "", false)
                .tags
                .contains(&"commit:seasonal".into())
        );
        assert!(has(
            "This is a temporary position lasting 3 months.",
            "commit:temporary"
        ));
    }

    #[test]
    fn graduation_and_start() {
        let t = tags("Open to students graduating between December 2025 and June 2026.");
        assert!(
            t.contains(&"grad:2025".into()) && t.contains(&"grad:2026".into()),
            "{t:?}"
        );
        let e = extract("Software Engineering Intern (Summer 2026)", None, "", true);
        assert!(e.tags.contains(&"start:2026-summer".into()), "{:?}", e.tags);
    }
}
