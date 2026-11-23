import type { JobSummary, MatchLabel, Salary } from "@/api/client";
import { EMPLOYMENT, SENIORITY, SOURCE, WORKPLACE, labelOf } from "./filters";

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/** "just now", "5h ago", "3d ago", "2mo ago". */
export function timeAgo(iso: string, now = Date.now()): string {
  const diff = Math.max(0, now - new Date(iso).getTime());
  if (diff < HOUR) return diff < 5 * MINUTE ? "just now" : `${Math.floor(diff / MINUTE)}m ago`;
  if (diff < DAY) return `${Math.floor(diff / HOUR)}h ago`;
  if (diff < 30 * DAY) return `${Math.floor(diff / DAY)}d ago`;
  if (diff < 365 * DAY) return `${Math.floor(diff / (30 * DAY))}mo ago`;
  return `${Math.floor(diff / (365 * DAY))}y ago`;
}

/** "in 5m", "in 3h", "due now" for scheduled times. */
export function timeUntil(iso: string, now = Date.now()): string {
  const diff = new Date(iso).getTime() - now;
  if (diff <= MINUTE) return "due now";
  if (diff < HOUR) return `in ${Math.round(diff / MINUTE)}m`;
  if (diff < DAY) return `in ${Math.round(diff / HOUR)}h`;
  return `in ${Math.round(diff / DAY)}d`;
}

export function isNew(job: JobSummary, now = Date.now()): boolean {
  return now - new Date(job.posted_at).getTime() < DAY;
}

/** "$120K", "£85K", "₩315M", "$42" in the salary's own currency. */
function money(n: number, currency: string, period: Salary["period"]): string {
  try {
    return new Intl.NumberFormat("en-US", {
      style: "currency",
      currency,
      notation: period === "hour" ? "standard" : "compact",
      // $48.50/hr, $120K, $1.3M
      minimumFractionDigits: period === "hour" && !Number.isInteger(n) ? 2 : 0,
      maximumFractionDigits: period === "hour" ? 2 : n >= 1_000_000 ? 1 : 0,
    }).format(n);
  } catch {
    return `${n.toLocaleString("en-US")} ${currency}`; // unknown currency code
  }
}

/** "$120K–$150K/yr", "$42–$48/hr", "£85K–£110K/yr". */
export function formatSalary(s: Salary): string {
  const fmt = (n: number) => money(n, s.currency, s.period);
  const unit = s.period === "hour" ? "/hr" : "/yr";
  const range = s.min === s.max ? fmt(s.max) : `${fmt(s.min)}–${fmt(s.max)}`;
  return `${range}${unit}`;
}

export const workplaceLabel = (v: string) => labelOf(WORKPLACE, v) ?? v;
export const employmentLabel = (v: string) => labelOf(EMPLOYMENT, v) ?? v;
export const seniorityLabel = (v: string) => labelOf(SENIORITY, v) ?? v;
export const sourceLabel = (v: string) =>
  labelOf(SOURCE, v) ?? v.charAt(0).toUpperCase() + v.slice(1);

export const MATCH_LABEL: Record<MatchLabel, string> = {
  strong: "Strong match",
  good: "Good match",
  fair: "Fair match",
  low: "Weak match",
};

/** Tailwind text color class for a match label. */
export const MATCH_TONE: Record<MatchLabel, string> = {
  strong: "text-success",
  good: "text-primary",
  fair: "text-warning",
  low: "text-muted-foreground",
};

/** People-search links the user opens in their own browser. No scraping. */
export function peopleLinks(company: string, title: string, department?: string | null) {
  const enc = encodeURIComponent;
  const team = department ?? title;
  return [
    {
      label: `People at ${company}`,
      hint: department ? `${department} team on LinkedIn` : "On LinkedIn",
      href: `https://www.linkedin.com/search/results/people/?keywords=${enc(`${company} ${team}`)}`,
    },
    {
      label: "Recruiters",
      hint: `Recruiters at ${company} on LinkedIn`,
      href: `https://www.linkedin.com/search/results/people/?keywords=${enc(`${company} recruiter`)}`,
    },
    {
      label: "Search the web",
      hint: `People with this role at ${company}`,
      href: `https://www.google.com/search?q=${enc(`site:linkedin.com/in "${company}" ${title}`)}`,
    },
  ];
}
