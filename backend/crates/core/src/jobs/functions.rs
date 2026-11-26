//! Job functions: a fixed taxonomy every job is sorted into from its title
//! (then its department), for the feed's Job function filter. Employers'
//! own department names are free text ("R&D", "Eng - Platform", blank);
//! this gives one clean list across all of them.
//!
//! Rules are tried in order and the first match wins, so specific functions
//! come before broad ones ("Nurse Practitioner" before "Nurse", "Security
//! Engineer" before "Engineer").

use std::sync::LazyLock;

use regex::{Regex, RegexSet};
use serde::Serialize;
use utoipa::ToSchema;

/// One function: its slug (stored in `jobs.job_function`), group and label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
pub struct JobFunction {
    pub value: &'static str,
    pub group: &'static str,
    pub label: &'static str,
}

const fn f(value: &'static str, group: &'static str, label: &'static str) -> JobFunction {
    JobFunction {
        value,
        group,
        label,
    }
}

pub const OTHER: &str = "other";

/// Display order: groups, then functions within each group.
pub const FUNCTIONS: &[JobFunction] = &[
    f("software", "Technology", "Software Development"),
    f("qa", "Technology", "QA & Testing"),
    f("devops", "Technology", "DevOps & Infrastructure"),
    f("security", "Technology", "Cybersecurity"),
    f("it", "Technology", "IT & Tech Support"),
    f("data_ml", "Data & AI", "Machine Learning & AI"),
    f("data", "Data & AI", "Data & Analytics"),
    f("product", "Product & Design", "Product Management"),
    f("design", "Product & Design", "Design & UX"),
    f("hardware", "Engineering", "Hardware & Electrical"),
    f(
        "mechanical",
        "Engineering",
        "Mechanical, Industrial & Manufacturing",
    ),
    f("civil", "Engineering", "Civil, Construction & Architecture"),
    f("engineering", "Engineering", "Other Engineering"),
    f(
        "project",
        "Business & Operations",
        "Project & Program Management",
    ),
    f(
        "operations",
        "Business & Operations",
        "Operations & Strategy",
    ),
    f("consulting", "Business & Operations", "Consulting"),
    f("legal", "Business & Operations", "Legal & Compliance"),
    f("finance", "Finance", "Accounting & Finance"),
    f("banking", "Finance", "Banking, Investing & Insurance"),
    f("hr", "People & Admin", "HR & Recruiting"),
    f("admin", "People & Admin", "Administrative & Office"),
    f("sales", "Sales & Marketing", "Sales & Business Development"),
    f(
        "marketing",
        "Sales & Marketing",
        "Marketing & Communications",
    ),
    f("content", "Sales & Marketing", "Writing, Media & Content"),
    f("customer", "Customer Service", "Customer Success & Support"),
    f("physician", "Healthcare", "Physicians & Advanced Practice"),
    f("nursing", "Healthcare", "Nursing"),
    f("pharmacy", "Healthcare", "Pharmacy"),
    f("therapy", "Healthcare", "Therapy & Rehabilitation"),
    f("mental_health", "Healthcare", "Mental Health & Social Work"),
    f(
        "allied_health",
        "Healthcare",
        "Allied Health & Medical Support",
    ),
    f("science", "Science & Education", "Science & Research"),
    f("education", "Science & Education", "Education & Training"),
    f(
        "supply_chain",
        "Trades & Logistics",
        "Supply Chain & Warehouse",
    ),
    f("driving", "Trades & Logistics", "Driving & Delivery"),
    f(
        "production",
        "Trades & Logistics",
        "Manufacturing & Production",
    ),
    f(
        "trades",
        "Trades & Logistics",
        "Skilled Trades & Maintenance",
    ),
    f("retail", "Retail & Hospitality", "Retail"),
    f("food", "Retail & Hospitality", "Food & Hospitality"),
    f(
        "public_safety",
        "Public Service",
        "Security & Public Safety",
    ),
    f("real_estate", "Public Service", "Real Estate & Property"),
    f(
        "government",
        "Public Service",
        "Government & Nonprofit Programs",
    ),
    f(OTHER, "Other", "Other"),
];

/// Rules in match order: (function, pattern over the lowercased title or
/// department).
const RULES: &[(&str, &str)] = &[
    // Role nouns that decide the function whatever comes before them
    // ("AI and Robotics Recruiter", "Assistant Manager (06589)").
    (
        "hr",
        r"recruiter|recruiting coordinator|talent acquisition|\bsourcer\b",
    ),
    ("legal", r"\battorney|paralegal|\bcounsel\b"),
    ("retail", r"assistant (?:store |general )?manager\b"),
    // Healthcare titles that contain broad words ("technician", "assistant").
    (
        "physician",
        r"\bphysician\b|advanced practice|\bsurgeon|psychiatrist|hospitalist|anesthesiolog|nurse practitioner|\baprn\b|\bcrna\b|\bfnp\b|physician assistant|\bpa-c\b|\bdentist|optometrist|radiologist|\bmd\b|pediatrician|\bobgyn\b|ob/gyn",
    ),
    ("pharmacy", r"pharmac"),
    ("nursing", r"\bnurse|nursing|\brn\b|\blpn\b|\blvn\b|\bcna\b"),
    (
        "therapy",
        r"therapist|\btherapy|rehab|speech[- ]language|\bslp\b|\bcota\b|\bpta\b|audiolog|chiropract",
    ),
    (
        "mental_health",
        r"direct support professional|\bdsp\b|counsel(or|ing)|psycholog|social work|\blcsw\b|\blmsw\b|behavior(al)? (health|technician|analyst)|\bbcba\b|\brbt\b|case manag|mental health|substance|peer (support|specialist)|clinician",
    ),
    (
        "allied_health",
        r"medical assistant|specimen|phlebotom|radiolog|sonograph|ultrasound|\bmri\b|\bct tech|x-ray|surgical tech|lab(oratory)? (tech|assistant)|medical (lab|technologist|coder|coding|billing|scribe|receptionist)|respiratory|\bemt\b|paramedic|dental (assistant|hygienist)|hygienist|patient care|sterile processing|dietitian|nutrition|health information|caregiver|home health|patient (access|services|coordinator)|optician|veterinar|vet tech|\bcma\b|histotech|cytotech|perfusionist|dosimetr|technologist",
    ),
    // Technology, specific before broad.
    (
        "security",
        r"secur(ity|e) (engineer|analyst|architect|specialist|operations|researcher|consultant)|cyber|\bsoc\b|penetration|pen ?test|infosec|information security|identity (and|&) access|\biam\b|threat|vulnerab|appsec|detection (and|&) response|grc analyst",
    ),
    (
        "data_ml",
        r"machine learning|\bml\b|\bai\b|artificial intelligence|data scien|deep learning|applied scien|computer vision|\bnlp\b|\bllm|generative|\bmlops\b|research engineer",
    ),
    (
        "data",
        r"\bgis\b|data (analy|engineer|architect|warehouse|platform|quality|governance|steward|management|ops|modeler|specialist|integration|infrastructure)|business intelligence|\bbi\b|analytics|\betl\b|database|\bdba\b|reporting analyst|tableau|power bi|statistic",
    ),
    (
        "qa",
        r"\bqa\b|quality assurance (engineer|analyst|tester|automation)|test(ing)? (engineer|automation|analyst|lead)|\bsdet\b|software tester|\btester\b",
    ),
    (
        "devops",
        r"devops|site reliability|\bsre\b|infrastructure|platform engineer|cloud (engineer|architect|operations)|systems (engineer|administrator|architect)|sysadmin|network (engineer|administrator|architect)|\bnoc\b|kubernetes|release engineer|build engineer|production engineer",
    ),
    (
        "it",
        r"\bit\b|integration analyst|systems? integration|\boracle\b|functional (?:lead|consultant|analyst)|help ?desk|desktop support|technical support|tech support|service desk|support (technician|engineer|analyst)|it specialist|information technology|systems analyst|\bsap\b|\berp\b|salesforce (admin|administrator)|servicenow|workday (analyst|consultant)|\bcrm\b",
    ),
    (
        "software",
        r"software|developer|programmer|full[- ]?stack|front[- ]?end|back[- ]?end|\bios\b|android|mobile (engineer|app)|web (engineer|developer)|firmware|embedded|game (developer|programmer|engineer)|\bswe\b|\bsde\b|engineering manager|solutions architect|technical architect|application (engineer|architect)|integration engineer|forward deployed|member of technical staff|blockchain|\.net\b|java\b|python\b|golang|ruby\b|react\b|node\b",
    ),
    // Product and design.
    (
        "product",
        r"product development|product (manager|owner|lead|director|management|operations|marketing manager|analyst)|head of product|\bvp,? (of )?product|director of product|\bapm\b|\bgpm\b",
    ),
    (
        "design",
        r"design(er)?\b|\bux\b|\bui\b|user experience|user research|creative director|art director|illustrat|animator|\b3d artist|motion graphics|visual|graphic",
    ),
    // Other engineering.
    (
        "hardware",
        r"electrical|hardware|electronic|\basic\b|\bfpga\b|\brf\b|\bpcb\b|semiconductor|silicon|analog|digital design|verification engineer|optical|photonic|power (systems|electronics)|controls engineer|test engineer",
    ),
    (
        "mechanical",
        r"mechanical|manufacturing engineer|industrial engineer|process engineer|quality engineer|reliability engineer|aerospace|propulsion|materials engineer|chemical engineer|mechatronic|robotics|automation engineer|packaging engineer|validation engineer|systems engineer|avionics|thermal",
    ),
    (
        "civil",
        r"civil|construction|structural|superintendent|estimator|survey|architectural|licensed architect|project architect|landscape architect|\bmep\b|building|geotechnical|environmental engineer|transportation engineer|water resources|land development|project engineer",
    ),
    ("engineering", r"engineer"),
    // Business.
    (
        "project",
        r"project (manager|coordinator|lead|director|management|controller|analyst|scheduler)|program (manager|coordinator|director|management|lead)|\bpmo\b|scrum master|delivery (manager|lead)|agile coach",
    ),
    (
        "legal",
        r"attorney|counsel\b|lawyer|paralegal|legal|compliance|regulatory|contracts? (manager|specialist|administrator|negotiator)|privacy|\bjd\b|litigation|patent",
    ),
    (
        "hr",
        r"recruit|talent (acquisition|partner|sourcer)|sourcer|human resources|\bhr\b|\bhris\b|people (ops|operations|partner|team)|benefits (specialist|manager|analyst|administrator)|compensation|\bhrbp\b|learning (and|&) development|\bl&d\b|training (specialist|manager|coordinator)|payroll",
    ),
    (
        "finance",
        r"account(ant|ing)|finance|financial|controller|\bcfo\b|\btax\b|audit|bookkeep|treasury|\bfp&a\b|billing|accounts (payable|receivable)|\bcpa\b|revenue|budget|cost analyst|collections",
    ),
    (
        "banking",
        r"adjuster|\bbank|investment|portfolio|trader|trading|equity|wealth|loan|mortgage|credit|underwrit|actuar|insurance|claims|broker|asset management|private equity|venture|capital markets|fund|teller|financial advisor|relationship banker",
    ),
    (
        "sales",
        r"account manag|sales|account executive|business development|\bbdr\b|\bsdr\b|account manager|partnerships?|territory|pre-?sales|channel|\bae\b|key account|revenue operations|\brevops\b|inside sales|solutions consultant|sales engineer",
    ),
    (
        "marketing",
        r"event planner|marketing|growth|\bseo\b|\bsem\b|brand|demand gen|campaign|social media|communications|public relations|\bpr\b|events? (manager|coordinator|specialist)|community manager|influencer|lifecycle|crm manager|copywrit",
    ),
    (
        "content",
        r"publication|writer|editor|journalist|content|reporter|producer|video|photograph|podcast|editorial|translator|interpreter|localization",
    ),
    (
        "customer",
        r"reservation|guest services|customer|client (success|services|support|experience)|support (specialist|representative|agent|associate)|call center|contact center|implementation (specialist|consultant|manager)|onboarding|technical account manager|account coordinator",
    ),
    (
        "operations",
        r"change manag|operations|strategy|strategic|business analyst|chief of staff|process improvement|bizops|\bops\b|program analyst|planning analyst|business systems|management analyst|general manager|chief operating|\bcoo\b|\bceo\b",
    ),
    ("consulting", r"consult(ant|ing)|advisory|advisor"),
    // Science and education.
    (
        "science",
        r"scientist|research (associate|assistant|technician|coordinator|fellow)|researcher|laborator|chemist|biolog|clinical (research|trial)|\bcra\b|postdoc|pharmacolog|toxicolog|epidemiolog|biostatistic|geolog|physicist|ecolog|environmental",
    ),
    (
        "education",
        r"teacher|tutor|instructor|professor|faculty|lecturer|educator|teaching|curriculum|(school|assistant) principal|\bschool\b|academic|\bdean\b|paraprofessional|childcare|child care|preschool|early childhood|admissions|student",
    ),
    // Trades, logistics, retail, food.
    (
        "driving",
        r"driver|delivery|courier|\bcdl\b|truck|chauffeur",
    ),
    (
        "supply_chain",
        r"customs|import|export|inbound|outbound|stocking|supply chain|logistic|procurement|purchasing|buyer|sourcing|inventory|warehouse|shipping|receiving|fulfillment|distribution|dispatcher|material handler|forklift|picker|packer|transportation|fleet|demand planner|supply planner",
    ),
    (
        "production",
        r"production|assembler|assembly|manufacturing (associate|technician|operator|specialist)|machine operator|\boperator\b|fabricat|packag|quality (inspector|control|technician)|\bcnc\b",
    ),
    (
        "trades",
        r"technician|mechanic|electrician|plumb|hvac|welder|carpenter|maintenance|machinist|installer|lineman|millwright|painter|janitor|custodian|groundskeep|facilities|landscap|roofer|apprentice|journeyman|pipefitter|locksmith|housekeep",
    ),
    (
        "retail",
        r"retail|store (manager|associate|lead)|cashier|sales (associate|floor)|merchandis|stocker|shift (lead|supervisor)|key holder|keyholder|boutique",
    ),
    (
        "food",
        r"\bcook\b|chef|server\b|bartender|barista|dishwasher|\bhost(ess)?\b|restaurant|food|kitchen|hospitality|hotel|front desk agent|concierge|catering|banquet|culinary|baker|crew member",
    ),
    (
        "public_safety",
        r"security (officer|guard|supervisor)|police|firefight|corrections|loss prevention|public safety|emergency management|lifeguard|patrol",
    ),
    (
        "real_estate",
        r"real estate|property (manager|management)|leasing|realtor|appraiser|\btitle\b|escrow|community association",
    ),
    (
        "government",
        r"advocate|program (specialist|assistant|officer)|policy|grant|nonprofit|fundraising|development officer|donor|volunteer|outreach|advocacy|caseworker|eligibility",
    ),
    (
        "admin",
        r"mailroom|administrative|admin(istrator)? assistant|executive assistant|office (manager|coordinator|administrator|assistant)|receptionist|front desk|secretary|clerk|data entry|scheduler|coordinator|assistant\b",
    ),
];

static RULE_SET: LazyLock<RegexSet> = LazyLock::new(|| {
    RegexSet::new(RULES.iter().map(|(_, p)| *p)).expect("job function rules compile")
});

/// Lookarounds aren't supported by the `regex` crate, so a few rules use a
/// second check instead.
static NOT_DESIGN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"design (engineer|verification)").unwrap());

fn first_match(text: &str) -> Option<&'static str> {
    let text = text.to_lowercase();
    RULE_SET
        .matches(&text)
        .iter()
        .map(|i| RULES[i].0)
        .find(|f| !(*f == "design" && NOT_DESIGN.is_match(&text)))
}

/// The function a job belongs to: from its title, else its department,
/// else [`OTHER`].
pub fn classify(title: &str, department: Option<&str>) -> &'static str {
    first_match(title)
        .or_else(|| department.and_then(first_match))
        .unwrap_or(OTHER)
}

pub fn get(value: &str) -> Option<&'static JobFunction> {
    FUNCTIONS.iter().find(|f| f.value == value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_name_known_functions() {
        for (slug, _) in RULES {
            assert!(get(slug).is_some(), "{slug} missing from FUNCTIONS");
        }
        let _ = &*RULE_SET;
    }

    #[test]
    fn titles() {
        let cases = [
            ("Senior Software Engineer, Backend", "software"),
            ("Staff Machine Learning Engineer", "data_ml"),
            ("Data Analyst II", "data"),
            ("Site Reliability Engineer", "devops"),
            ("Security Engineer", "security"),
            ("IT Support Specialist", "it"),
            ("Help Desk Technician", "it"),
            ("QA Automation Engineer", "qa"),
            ("Product Manager, Payments", "product"),
            ("Senior Product Designer", "design"),
            ("Mechanical Engineer", "mechanical"),
            ("Electrical Engineer", "hardware"),
            ("Civil Engineer", "civil"),
            ("Nurse Practitioner", "physician"),
            ("Registered Nurse - ICU", "nursing"),
            ("Pharmacy Technician", "pharmacy"),
            ("Physical Therapist", "therapy"),
            ("Licensed Clinical Social Worker", "mental_health"),
            ("Medical Assistant", "allied_health"),
            ("Account Executive, Mid-Market", "sales"),
            ("Marketing Manager", "marketing"),
            ("Customer Success Manager", "customer"),
            ("Staff Accountant", "finance"),
            ("Mortgage Loan Officer", "banking"),
            ("Technical Recruiter", "hr"),
            ("Executive Assistant", "admin"),
            ("Paralegal", "legal"),
            ("Project Manager", "project"),
            ("CDL Truck Driver", "driving"),
            ("Warehouse Associate", "supply_chain"),
            ("HVAC Technician", "trades"),
            ("Line Cook", "food"),
            ("Store Manager", "retail"),
            ("Research Scientist, Oncology", "science"),
            ("High School Math Teacher", "education"),
            ("Security Officer", "public_safety"),
            ("Solutions Architect", "software"),
            ("ASIC Design Engineer", "hardware"),
            ("AI and Robotics Recruiter", "hr"),
            ("Assistant Manager (06589)", "retail"),
            ("Auto Damage Adjuster (Field)", "banking"),
            ("Reservation Agent", "customer"),
        ];
        for (title, want) in cases {
            assert_eq!(classify(title, None), want, "{title}");
        }
    }

    #[test]
    fn department_then_other() {
        assert_eq!(classify("Associate", Some("Sales")), "sales");
        assert_eq!(classify("Associate", None), OTHER);
    }
}
