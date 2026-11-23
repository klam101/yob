import type { JobsQuery } from "@/api/client";

/**
 * Feed filters live in the URL query string, so a filtered view can be
 * bookmarked, shared, and later saved as a named search. List filters use
 * comma-separated values, matching the API (`workplace=remote,hybrid`).
 */

export const FILTER_KEYS = [
  "q",
  "location",
  "workplace",
  "employment_type",
  "seniority",
  "department",
  "company",
  "source",
  "posted_within",
  "salary_min",
  "hide_stale",
  "sort",
  "page",
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
];

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
  return (query[key] ?? "").split(",").filter(Boolean);
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
  const joined = Array.isArray(value) ? value.join(",") : value;
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
  const qs = params.toString().replaceAll("%2C", ",");
  return qs ? `${path}?${qs}` : path;
}

/** Filters that narrow results (not sort or paging). */
export const NARROWING_KEYS: FilterKey[] = [
  "q",
  "location",
  "workplace",
  "employment_type",
  "seniority",
  "department",
  "company",
  "source",
  "posted_within",
  "salary_min",
  "hide_stale",
];

export function activeCount(query: Query, keys: FilterKey[] = NARROWING_KEYS) {
  return keys.reduce((n, key) => {
    if (!query[key]) return n;
    const list = listOf(query, key);
    return n + (list.length > 1 ? list.length : 1);
  }, 0);
}

export function clearFilters(query: Query): Query {
  return query.sort ? { sort: query.sort } : {};
}

/** Converts the URL query into the API's query parameters. */
export function toApiQuery(query: Query, pageSize = 20): JobsQuery {
  const num = (v?: string) => (v && Number.isFinite(Number(v)) ? Number(v) : undefined);
  return {
    q: query.q,
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
    hide_stale: query.hide_stale === "true" ? true : undefined,
    sort: SORT.some((o) => o.value === query.sort)
      ? (query.sort as JobsQuery["sort"])
      : undefined,
    page: num(query.page),
    page_size: pageSize,
  };
}
