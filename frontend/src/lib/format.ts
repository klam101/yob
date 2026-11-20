import type { JobSummary, MatchLabel, Salary } from "@/api/client";
import { EMPLOYMENT, SENIORITY, WORKPLACE, labelOf } from "./filters";

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

export function isNew(job: JobSummary, now = Date.now()): boolean {
  return now - new Date(job.posted_at).getTime() < DAY;
}

function money(n: number, period: Salary["period"]): string {
  if (period === "hour") return `$${n}`;
  return n >= 1000 ? `$${Math.round(n / 1000)}k` : `$${n}`;
}

/** "$120k–$150k/yr" or "$42–$48/hr". */
export function formatSalary(s: Salary): string {
  const unit = s.period === "hour" ? "/hr" : "/yr";
  const range =
    s.min === s.max ? money(s.max, s.period) : `${money(s.min, s.period)}–${money(s.max, s.period)}`;
  return `${range}${unit}`;
}

export const workplaceLabel = (v: string) => labelOf(WORKPLACE, v) ?? v;
export const employmentLabel = (v: string) => labelOf(EMPLOYMENT, v) ?? v;
export const seniorityLabel = (v: string) => labelOf(SENIORITY, v) ?? v;
export const sourceLabel = (v: string) => v.charAt(0).toUpperCase() + v.slice(1);

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
export function peopleLinks(company: string, title: string, department: string) {
  const enc = encodeURIComponent;
  return [
    {
      label: `People at ${company}`,
      hint: `${department} team on LinkedIn`,
      href: `https://www.linkedin.com/search/results/people/?keywords=${enc(`${company} ${department}`)}`,
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
