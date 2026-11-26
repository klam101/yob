import type { JobsQuery, Preferences } from "@/api/client";

/**
 * Feed filters live in the URL query string, so a filtered view can be
 * bookmarked, shared, and later saved as a named search. List filters use
 * comma-separated values, matching the API (`workplace=remote,hybrid`);
 * `location` holds places separated by `;`, since a place can contain a
 * comma ("Austin, TX; Denver, CO").
 *
 * With no query at all, the feed applies the questionnaire answers instead
 * (`fromPreferences`). `all=1` marks "no filters, on purpose", so clearing
 * the filters doesn't bring the defaults back.
 */

export const FILTER_KEYS = [
  "q",
  "title",
  "location",
  "workplace",
  "employment_type",
  "seniority",
  "department",
  "company",
  "source",
  "posted_within",
  "posted_from",
  "posted_to",
  "apply",
  "salary_min",
  "include_unlisted_salary",
  "pay_period",
  "salary_listed",
  "hide_stale",
  "exclude_title",
  "exclude_company",
  "hide_no_sponsorship",
  "hide_citizens_only",
  "function",
  "years_min",
  "years_max",
  "years_unlisted",
  "role",
  "edu",
  "perk",
  "clearance",
  "hide_clearance",
  "cert",
  "x_cert",
  "no_cert",
  "lang",
  "x_lang",
  "encourage",
  "shift",
  "x_shift",
  "travel",
  "skill",
  "x_skill",
  "hide_staffing",
  "commit",
  "grad",
  "start",
  "industry",
  "x_industry",
  "org_type",
  "x_org_type",
  "size",
  "founded_min",
  "founded_max",
  "hq",
  "x_hq",
  "sort",
  "page",
  "all",
] as const;

export type FilterKey = (typeof FILTER_KEYS)[number];
export type Query = Partial<Record<FilterKey, string>>;

type Option = { value: string; label: string };

export const WORKPLACE: Option[] = [
  { value: "remote", label: "Remote" },
  { value: "hybrid", label: "Hybrid" },
  { value: "onsite", label: "On-site" },
];

export const EMPLOYMENT: Option[] = [
  { value: "full_time", label: "Full-time" },
  { value: "part_time", label: "Part-time" },
  { value: "contract", label: "Contract" },
  { value: "internship", label: "Internship" },
];

export const SENIORITY: Option[] = [
  { value: "intern", label: "Intern" },
  { value: "entry", label: "Entry level" },
  { value: "mid", label: "Mid level" },
  { value: "senior", label: "Senior" },
  { value: "staff_plus", label: "Staff / principal" },
  { value: "director_plus", label: "Director and above" },
];

export const ROLE: Option[] = [
  { value: "ic", label: "Individual contributor" },
  { value: "manager", label: "People manager" },
];

export const DEGREE_LEVELS: Option[] = [
  { value: "associate", label: "Associate's degree" },
  { value: "bachelors", label: "Bachelor's degree" },
  { value: "masters", label: "Master's degree" },
  { value: "doctorate", label: "Doctorate" },
];

export const DEGREE_STATES: Option[] = [
  { value: "required", label: "Required" },
  { value: "preferred", label: "Preferred" },
  { value: "not", label: "Not mentioned" },
];

/** Years of experience on the slider; the top means "or more". */
export const YEARS_TOP = 10;

/** Filter keys whose values are tags, and the tag prefix each uses. */
export const TAG_PREFIX: Partial<Record<FilterKey, string>> = {
  perk: "perk",
  clearance: "clearance",
  cert: "cert",
  x_cert: "cert",
  lang: "lang",
  x_lang: "lang",
  encourage: "encourage",
  shift: "shift",
  x_shift: "shift",
  travel: "travel",
  skill: "skill",
  x_skill: "skill",
  commit: "commit",
};

export const ORG_TYPES: Option[] = [
  { value: "public", label: "Public company" },
  { value: "private", label: "Private company" },
  { value: "nonprofit", label: "Nonprofit" },
  { value: "government", label: "Government" },
  { value: "cooperative", label: "Cooperative" },
];

export const COMPANY_SIZES: (Option & { hint: string })[] = [
  { value: "startup", label: "1-50", hint: "Startup" },
  { value: "small", label: "51-200", hint: "Small" },
  { value: "medium", label: "201-1,000", hint: "Mid-size" },
  { value: "large", label: "1,001-10,000", hint: "Large" },
  { value: "enterprise", label: "10,001+", hint: "Enterprise" },
];

/** Keys whose values are separated by `;` (values may hold commas). */
const SEMICOLON_KEYS: ReadonlySet<FilterKey> = new Set(["location", "industry", "x_industry", "hq", "x_hq"]);

/** Seasons for internship starts, in calendar order. */
export const SEASONS = ["spring", "summer", "fall", "winter"] as const;

export const SOURCE: Option[] = [
  { value: "greenhouse", label: "Greenhouse" },
  { value: "lever", label: "Lever" },
  { value: "ashby", label: "Ashby" },
  { value: "workday", label: "Workday" },
  { value: "smartrecruiters", label: "SmartRecruiters" },
  { value: "workable", label: "Workable" },
  { value: "recruitee", label: "Recruitee" },
  { value: "bamboohr", label: "BambooHR" },
  // Job boards: only jobs no tracked employer board has.
  { value: "himalayas", label: "Himalayas" },
  { value: "remotive", label: "Remotive" },
  { value: "remoteok", label: "Remote OK" },
  { value: "usajobs", label: "USAJOBS" },
];

/** Sources that are job boards rather than an employer's own hiring system. */
export const JOB_BOARDS: ReadonlySet<string> = new Set(["himalayas", "remotive", "remoteok", "usajobs"]);

export const POSTED_WITHIN: Option[] = [
  { value: "24h", label: "Past 24 hours" },
  { value: "3d", label: "Past 3 days" },
  { value: "1w", label: "Past week" },
  { value: "2w", label: "Past 2 weeks" },
  { value: "3w", label: "Past 3 weeks" },
  { value: "1m", label: "Past month" },
  { value: "2m", label: "Past 2 months" },
  { value: "3m", label: "Past 3 months" },
  { value: "6m", label: "Past 6 months" },
  { value: "1y", label: "Past year" },
];

/** The short list for the feed's Date posted chip. */
export const POSTED_WITHIN_SHORT = POSTED_WITHIN.filter((o) => ["24h", "3d", "1w", "2w", "1m", "3m"].includes(o.value));

export const APPLY: (Option & { hint: string })[] = [
  {
    value: "simple",
    label: "Quick apply",
    hint: "A short form on the employer's own page, no account needed (Greenhouse, Lever, Ashby and similar).",
  },
  {
    value: "account",
    label: "Account required",
    hint: "You create an account before applying, and often re-enter your resume (Workday).",
  },
];

export const PAY_PERIOD: Option[] = [
  { value: "year", label: "Yearly" },
  { value: "hour", label: "Hourly" },
];

/** Hours in a working year, for comparing hourly and yearly pay. */
export const HOURS_PER_YEAR = 2080;

export const SALARY_MIN: Option[] = [
  { value: "50000", label: "$50k+" },
  { value: "75000", label: "$75k+" },
  { value: "100000", label: "$100k+" },
  { value: "125000", label: "$125k+" },
  { value: "150000", label: "$150k+" },
  { value: "200000", label: "$200k+" },
];

export const SORT: Option[] = [
  { value: "recommended", label: "Recommended" },
  { value: "newest", label: "Newest" },
  { value: "salary", label: "Highest salary" },
];

export function labelOf(options: Option[], value: string | undefined) {
  return options.find((o) => o.value === value)?.label;
}

/** "90000" -> "$90k+". */
export function salaryLabel(value: string | undefined) {
  const n = Number(value);
  if (!value || !Number.isFinite(n) || n <= 0) return undefined;
  return labelOf(SALARY_MIN, value) ?? `$${Math.round(n / 1000)}k+`;
}

/** The salary choices, plus the current value when it isn't one of them. */
export function salaryOptions(current: string | undefined): Option[] {
  const label = salaryLabel(current);
  if (!current || !label || SALARY_MIN.some((o) => o.value === current)) return SALARY_MIN;
  return [...SALARY_MIN, { value: current, label }].sort((a, b) => Number(a.value) - Number(b.value));
}

/** Reads the filter keys from Next's `searchParams`, ignoring anything else. */
export function readQuery(
  params: Record<string, string | string[] | undefined>,
): Query {
  const query: Query = {};
  for (const key of FILTER_KEYS) {
    const raw = params[key];
    const value = (Array.isArray(raw) ? raw.join(",") : raw)?.trim();
    if (value) query[key] = value;
  }
  return query;
}

export function listOf(query: Query, key: FilterKey): string[] {
  const sep = SEMICOLON_KEYS.has(key) ? ";" : ",";
  return (query[key] ?? "")
    .split(sep)
    .map((v) => v.trim())
    .filter(Boolean);
}

/**
 * Returns a new query with one key changed. Any filter change resets to
 * page 1; an empty value removes the key.
 */
export function withValue(
  query: Query,
  key: FilterKey,
  value: string | string[] | undefined,
): Query {
  const next: Query = { ...query };
  const joined = Array.isArray(value) ? value.join(SEMICOLON_KEYS.has(key) ? "; " : ",") : value;
  if (joined) next[key] = joined;
  else delete next[key];
  if (key !== "page") delete next.page;
  return next;
}

export function toggleInList(query: Query, key: FilterKey, value: string): Query {
  const current = listOf(query, key);
  const next = current.includes(value)
    ? current.filter((v) => v !== value)
    : [...current, value];
  return withValue(query, key, next);
}

export function toHref(query: Query, path = "/jobs"): string {
  const params = new URLSearchParams();
  for (const key of FILTER_KEYS) {
    const value = query[key];
    if (value) params.set(key, value);
  }
  const qs = params.toString().replaceAll("%2C", ",").replaceAll("%3B", ";");
  return qs ? `${path}?${qs}` : path;
}

/** Filters that narrow results (not sort or paging). */
export const NARROWING_KEYS: FilterKey[] = [
  "q",
  "title",
  "location",
  "workplace",
  "employment_type",
  "seniority",
  "department",
  "company",
  "source",
  "posted_within",
  "posted_from",
  "posted_to",
  "apply",
  "salary_min",
  "include_unlisted_salary",
  "pay_period",
  "salary_listed",
  "hide_stale",
  "exclude_title",
  "exclude_company",
  "hide_no_sponsorship",
  "hide_citizens_only",
  "function",
  "years_min",
  "years_max",
  "years_unlisted",
  "role",
  "edu",
  "perk",
  "clearance",
  "hide_clearance",
  "cert",
  "x_cert",
  "no_cert",
  "lang",
  "x_lang",
  "encourage",
  "shift",
  "x_shift",
  "travel",
  "skill",
  "x_skill",
  "hide_staffing",
  "commit",
  "grad",
  "start",
  "industry",
  "x_industry",
  "org_type",
  "x_org_type",
  "size",
  "founded_min",
  "founded_max",
  "hq",
  "x_hq",
];

export function activeCount(query: Query, keys: FilterKey[] = NARROWING_KEYS) {
  return keys.reduce((n, key) => {
    // Modifiers of another filter, not filters of their own; a years range
    // counts once.
    if (!query[key] || key === "include_unlisted_salary" || key === "years_unlisted") return n;
    if (key === "years_max" && query.years_min) return n;
    if (key === "founded_max" && query.founded_min) return n;
    const list = listOf(query, key);
    return n + (list.length > 1 ? list.length : 1);
  }, 0);
}

/** No filters at all, on purpose (so the preference defaults stay off). */
export function clearFilters(query: Query): Query {
  return query.sort ? { sort: query.sort, all: "1" } : { all: "1" };
}

/** True when the URL has no query at all: the feed shows the defaults. */
export function isBare(query: Query) {
  return FILTER_KEYS.every((k) => !query[k]);
}

/** Whether two queries narrow the results the same way. */
export function sameFilters(a: Query, b: Query) {
  const norm = (q: Query, k: FilterKey) =>
    listOf(q, k)
      .map((v) => v.toLowerCase())
      .sort()
      .join("|");
  return NARROWING_KEYS.every((k) => norm(a, k) === norm(b, k));
}

/**
 * The questionnaire answers as feed filters. Company size and industry
 * aren't filters yet (there's no company data to match them against).
 */
export function fromPreferences(p: Preferences): Query {
  let q: Query = {};
  const set = (key: FilterKey, value: string | string[] | undefined) => {
    q = withValue(q, key, value);
  };
  set("title", p.titles);
  set("seniority", p.seniority);
  set("employment_type", p.employment_types);
  set("workplace", p.workplaces);
  set("location", p.locations);
  if (p.salary_min) {
    set("salary_min", String(p.salary_min));
    if (p.include_unlisted_salary) set("include_unlisted_salary", "true");
  }
  set("exclude_title", p.excluded_title_words);
  set("exclude_company", p.excluded_companies);
  if (p.work_authorization.needs_sponsorship === true) set("hide_no_sponsorship", "true");
  if (p.work_authorization.us_citizen === false) set("hide_citizens_only", "true");
  return q;
}

/** Converts the URL query into the API's query parameters. */
export function toApiQuery(query: Query, pageSize = 20): JobsQuery {
  const num = (v?: string) => (v && Number.isFinite(Number(v)) ? Number(v) : undefined);
  const yes = (v?: string) => (v === "true" ? true : undefined);
  return {
    q: query.q,
    title: query.title,
    location: query.location,
    workplace: query.workplace,
    employment_type: query.employment_type,
    seniority: query.seniority,
    department: query.department,
    company: query.company,
    source: query.source,
    posted_within: POSTED_WITHIN.some((o) => o.value === query.posted_within)
      ? (query.posted_within as JobsQuery["posted_within"])
      : undefined,
    posted_from: isoDate(query.posted_from),
    posted_to: isoDate(query.posted_to),
    apply: query.apply,
    salary_min: num(query.salary_min),
    include_unlisted_salary:
      query.salary_min && query.salary_listed !== "true" ? yes(query.include_unlisted_salary) : undefined,
    pay_period: PAY_PERIOD.some((o) => o.value === query.pay_period)
      ? (query.pay_period as JobsQuery["pay_period"])
      : undefined,
    salary_listed: yes(query.salary_listed),
    function: query.function,
    years_min: num(query.years_min),
    years_max: num(query.years_max),
    years_unlisted: query.years_min || query.years_max ? yes(query.years_unlisted) : undefined,
    role: query.role === "ic" || query.role === "manager" ? query.role : undefined,
    edu: query.edu,
    perk: query.perk,
    clearance: query.clearance,
    hide_clearance: yes(query.hide_clearance),
    cert: query.cert,
    x_cert: query.x_cert,
    no_cert: yes(query.no_cert),
    lang: query.lang,
    x_lang: query.x_lang,
    encourage: query.encourage,
    shift: query.shift,
    x_shift: query.x_shift,
    travel: query.travel,
    skill: query.skill,
    x_skill: query.x_skill,
    hide_staffing: yes(query.hide_staffing),
    commit: query.commit,
    grad: query.grad,
    start: query.start,
    industry: query.industry,
    x_industry: query.x_industry,
    org_type: query.org_type,
    x_org_type: query.x_org_type,
    size: query.size,
    founded_min: num(query.founded_min),
    founded_max: num(query.founded_max),
    hq: query.hq,
    x_hq: query.x_hq,
    hide_stale: yes(query.hide_stale),
    exclude_title: query.exclude_title,
    exclude_company: query.exclude_company,
    hide_no_sponsorship: yes(query.hide_no_sponsorship),
    hide_citizens_only: yes(query.hide_citizens_only),
    sort: SORT.some((o) => o.value === query.sort)
      ? (query.sort as JobsQuery["sort"])
      : undefined,
    page: num(query.page),
    page_size: pageSize,
  };
}

const isoDate = (v?: string) => (v && /^\d{4}-\d{2}-\d{2}$/.test(v) ? v : undefined);

/** "2026-11-03" -> "Nov 3, 2026". */
export function shortDate(iso: string) {
  const [y, m, d] = iso.split("-").map(Number);
  if (!y || !m || !d) return iso;
  return new Date(y, m - 1, d).toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" });
}

/** One applied filter value, as a removable chip. */
export type ActiveFilter = { key: FilterKey; value?: string; label: string };

const yesLabels: Partial<Record<FilterKey, string>> = {
  hide_stale: "Hide possibly stale",
  hide_no_sponsorship: "Hide no visa sponsorship",
  hide_citizens_only: "Hide U.S. citizens only",
  salary_listed: "Salary listed",
  hide_clearance: "Hide clearance required",
  no_cert: "No license needed",
  hide_staffing: "Hide staffing agencies",
};

/** "four_day_week" -> "Four day week", when no label is known. */
const prettySlug = (v: string) => (v.charAt(0).toUpperCase() + v.slice(1)).replaceAll("_", " ");

const capital = (v: string) => v.charAt(0).toUpperCase() + v.slice(1);

/** "2" / "5" -> "2-5 years"; a missing or top end is open. */
export function yearsLabel(min?: string, max?: string) {
  const lo = Number(min) || 0;
  const hi = max === undefined || max === "" || Number(max) >= YEARS_TOP ? undefined : Number(max);
  if (hi === undefined) return `${lo}+ years`;
  if (lo === hi) return `${lo} ${lo === 1 ? "year" : "years"}`;
  return `${lo}-${hi} years`;
}

/**
 * Every applied filter as a labeled chip, in `keys` order. List filters
 * give one chip per value.
 */
export function describe(
  query: Query,
  keys: FilterKey[] = NARROWING_KEYS,
  /** Labels of tags (`skill:rust`) and job functions (`fn:software`). */
  labels: Record<string, string> = {},
): ActiveFilter[] {
  const out: ActiveFilter[] = [];
  for (const key of keys) {
    const raw = query[key];
    if (!raw) continue;
    switch (key) {
      case "include_unlisted_salary":
      case "years_unlisted":
        // Shown as part of the salary or years chip.
        break;
      case "years_min":
      case "years_max": {
        if (key === "years_max" && query.years_min) break;
        const unlisted = query.years_unlisted === "true" ? " or unlisted" : "";
        out.push({ key: "years_min", label: `${yearsLabel(query.years_min, query.years_max)} experience${unlisted}` });
        break;
      }
      case "role":
        out.push({ key, label: labelOf(ROLE, raw) ?? raw });
        break;
      case "function":
        for (const value of listOf(query, key)) {
          out.push({ key, value, label: labels[`fn:${value}`] ?? prettySlug(value) });
        }
        break;
      case "edu":
        for (const value of listOf(query, key)) {
          const [level, state] = value.split(":");
          const degree = labelOf(DEGREE_LEVELS, level)?.replace(" degree", "") ?? level;
          out.push({ key, value, label: `${degree}: ${(labelOf(DEGREE_STATES, state) ?? state).toLowerCase()}` });
        }
        break;
      case "grad":
        for (const value of listOf(query, key)) out.push({ key, value, label: `Graduating ${value}` });
        break;
      case "founded_min":
      case "founded_max": {
        if (key === "founded_max" && query.founded_min) break;
        const lo = query.founded_min;
        const hi = query.founded_max;
        const label = lo && hi ? `Founded ${lo}-${hi}` : lo ? `Founded ${lo} or later` : `Founded by ${hi}`;
        out.push({ key: "founded_min", label });
        break;
      }
      case "industry":
      case "x_industry":
      case "hq":
      case "x_hq": {
        const lead = { industry: "", x_industry: "Not ", hq: "HQ in ", x_hq: "HQ not in " }[key];
        for (const value of listOf(query, key)) out.push({ key, value, label: `${lead}${capital(value)}` });
        break;
      }
      case "size":
        for (const value of listOf(query, key)) {
          out.push({ key, value, label: `${labelOf(COMPANY_SIZES, value) ?? value} people` });
        }
        break;
      case "start":
        for (const value of listOf(query, key)) {
          const [year, season] = value.split("-");
          out.push({ key, value, label: `Starts ${capital(season ?? "")} ${year}` });
        }
        break;
      case "q":
        out.push({ key, label: `"${raw}"` });
        break;
      case "posted_within":
        out.push({ key, label: labelOf(POSTED_WITHIN, raw) ?? raw });
        break;
      case "posted_from":
        out.push({ key, label: `Posted from ${shortDate(raw)}` });
        break;
      case "posted_to":
        out.push({ key, label: `Posted until ${shortDate(raw)}` });
        break;
      case "salary_min": {
        const unlisted = query.include_unlisted_salary === "true" && query.salary_listed !== "true";
        out.push({ key, label: `${salaryLabel(raw) ?? raw}${unlisted ? " or unlisted" : ""}` });
        break;
      }
      case "pay_period":
        out.push({ key, label: `Paid ${labelOf(PAY_PERIOD, raw)?.toLowerCase() ?? raw}` });
        break;
      case "hide_stale":
      case "hide_no_sponsorship":
      case "hide_citizens_only":
      case "salary_listed":
      case "hide_clearance":
      case "no_cert":
      case "hide_staffing":
        if (raw === "true") out.push({ key, label: yesLabels[key] ?? key });
        break;
      default: {
        const prefix = TAG_PREFIX[key];
        if (prefix) {
          const none: Record<string, string> = { clearance: "No clearance mentioned", travel: "No travel mentioned" };
          const verb: Partial<Record<FilterKey, string>> = {
            x_cert: "Not ",
            x_lang: "Not ",
            x_skill: "Not ",
            x_shift: "No ",
          };
          for (const value of listOf(query, key)) {
            const label = value === "none" ? none[prefix] : labels[`${prefix}:${value}`];
            const text = label ?? prettySlug(value);
            out.push({ key, value, label: `${verb[key] ?? ""}${verb[key] === "No " ? text.toLowerCase() : text}` });
          }
          break;
        }
        const options: Partial<Record<FilterKey, Option[]>> = {
          workplace: WORKPLACE,
          employment_type: EMPLOYMENT,
          seniority: SENIORITY,
          source: SOURCE,
          apply: APPLY,
          org_type: ORG_TYPES,
          x_org_type: ORG_TYPES,
        };
        const not: Partial<Record<FilterKey, string>> = {
          exclude_title: "Not ",
          exclude_company: "Not ",
          x_org_type: "Not ",
        };
        for (const value of listOf(query, key)) {
          const label = labelOf(options[key] ?? [], value) ?? value;
          out.push({ key, value, label: `${not[key] ?? ""}${label}` });
        }
      }
    }
  }
  return out;
}

/** The query without one chip's filter. */
export function removeFilter(query: Query, f: ActiveFilter): Query {
  if (f.value !== undefined) {
    return withValue(query, f.key, listOf(query, f.key).filter((v) => v !== f.value));
  }
  let next = withValue(query, f.key, undefined);
  if (f.key === "salary_min") next = withValue(next, "include_unlisted_salary", undefined);
  if (f.key === "years_min") next = withoutKeys(next, ["years_max", "years_unlisted"]);
  if (f.key === "founded_min") next = withoutKeys(next, ["founded_max"]);
  return next;
}

/** The query with all of these keys cleared. */
export function withoutKeys(query: Query, keys: FilterKey[]): Query {
  return keys.reduce((q, k) => withValue(q, k, undefined), query);
}
