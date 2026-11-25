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
  "salary_min",
  "include_unlisted_salary",
  "hide_stale",
  "exclude_title",
  "exclude_company",
  "hide_no_sponsorship",
  "hide_citizens_only",
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
  { value: "staff_plus", label: "Staff and above" },
];

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
  { value: "1m", label: "Past month" },
  { value: "3m", label: "Past 3 months" },
];

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
  const sep = key === "location" ? ";" : ",";
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
  const joined = Array.isArray(value) ? value.join(key === "location" ? "; " : ",") : value;
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
  "salary_min",
  "include_unlisted_salary",
  "hide_stale",
  "exclude_title",
  "exclude_company",
  "hide_no_sponsorship",
  "hide_citizens_only",
];

export function activeCount(query: Query, keys: FilterKey[] = NARROWING_KEYS) {
  return keys.reduce((n, key) => {
    // A modifier of the salary filter, not a filter of its own.
    if (!query[key] || key === "include_unlisted_salary") return n;
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
    salary_min: num(query.salary_min),
    include_unlisted_salary: query.salary_min ? yes(query.include_unlisted_salary) : undefined,
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
