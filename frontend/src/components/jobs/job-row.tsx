import { Check } from "lucide-react";
import Link from "next/link";
import type { JobSummary, Match } from "@/api/client";
import { Skeleton } from "@/components/ui/skeleton";
import { MATCH_LABEL, MATCH_TONE } from "@/lib/format";
import { cn } from "cn";
import {
  ApplyButton,
  CompanyLogo,
  JobBadges,
  JobMeta,
  SaveHideActions,
  ScoreRing,
} from "./job-parts";

/**
 * One job in the feed. The title is a stretched link (its ::after covers the
 * card), so the whole row opens the job while action buttons, raised above
 * it with z-10, still work on their own.
 */
export function JobRow({ job }: { job: JobSummary }) {
  return (
    <article className="group relative flex overflow-hidden rounded-2xl border border-border/70 bg-card shadow-soft transition-[box-shadow,border-color] duration-200 hover:border-primary/30 hover:shadow-raised">
      <div className="flex min-w-0 flex-1 flex-col gap-4 p-5">
        <div className="flex gap-4">
          <CompanyLogo name={job.company.name} />
          <div className="min-w-0 flex-1 space-y-1.5">
            <JobBadges job={job} />
            <h2 className="text-lg leading-snug font-semibold tracking-tight">
              <Link
                href={`/jobs/${job.id}`}
                className="outline-none after:absolute after:inset-0 after:rounded-2xl after:content-[''] group-hover:text-primary focus-visible:after:ring-3 focus-visible:after:ring-ring/40"
              >
                {job.title}
              </Link>
            </h2>
            <p className="truncate text-sm">
              <span className="font-medium">{job.company.name}</span>
              {(job.company.industries.length > 0 || job.department) && (
                <span className="text-muted-foreground">
                  {" "}
                  · {job.company.industries.length > 0 ? job.company.industries.join(" · ") : job.department}
                </span>
              )}
            </p>
          </div>
          {job.match && <CompactMatch match={job.match} />}
        </div>

        <div className="flex flex-col gap-3 border-t border-border/60 pt-4 xl:flex-row xl:items-end">
          <JobMeta job={job} className="flex-1" />
          <div className="flex items-center justify-end gap-1">
            <SaveHideActions />
            <ApplyButton href={job.apply_url} />
          </div>
        </div>
      </div>

      {/* Unscored jobs (no resume yet) use the full width instead of an empty panel. */}
      {job.match && <MatchPanel match={job.match} />}
    </article>
  );
}

/** Right-hand score panel on wide screens. */
function MatchPanel({ match }: { match: Match }) {
  return (
    <aside className="hidden w-52 shrink-0 flex-col items-center gap-3 border-l border-border/60 bg-primary/4 px-4 py-5 text-center md:flex dark:bg-primary/6">
      <ScoreRing match={match} size={76} />
      <p className={cn("text-sm font-semibold", MATCH_TONE[match.label])}>{MATCH_LABEL[match.label]}</p>
      <ul className="w-full space-y-1.5 text-left">
        {match.reasons.slice(0, 3).map((reason) => (
          <li key={reason} className="flex gap-1.5 text-xs leading-snug text-muted-foreground">
            <Check className="mt-0.5 size-3.5 shrink-0 text-primary" />
            <span>{reason}</span>
          </li>
        ))}
      </ul>
    </aside>
  );
}

/** Small score shown next to the title on narrow screens. */
function CompactMatch({ match }: { match: Match }) {
  return (
    <div className="flex shrink-0 flex-col items-center gap-1 md:hidden">
      <ScoreRing match={match} size={48} />
      <span className={cn("text-[0.6875rem] font-medium", MATCH_TONE[match.label])}>
        {MATCH_LABEL[match.label].replace(" match", "")}
      </span>
    </div>
  );
}

export function JobRowSkeleton() {
  return (
    <div className="flex overflow-hidden rounded-2xl border border-border/70 bg-card shadow-soft">
      <div className="flex flex-1 flex-col gap-4 p-5">
        <div className="flex gap-4">
          <Skeleton className="size-12 rounded-2xl" />
          <div className="flex-1 space-y-2">
            <Skeleton className="h-4 w-24 rounded-full" />
            <Skeleton className="h-5 w-2/3" />
            <Skeleton className="h-4 w-1/3" />
          </div>
        </div>
        <div className="grid grid-cols-2 gap-3 border-t border-border/60 pt-4 lg:grid-cols-3">
          {Array.from({ length: 5 }, (_, i) => (
            <Skeleton key={i} className="h-4 w-3/4" />
          ))}
        </div>
      </div>
      <div className="hidden w-52 flex-col items-center gap-3 border-l border-border/60 p-5 md:flex">
        <Skeleton className="size-19 rounded-full" />
        <Skeleton className="h-4 w-24" />
        <Skeleton className="h-3 w-36" />
        <Skeleton className="h-3 w-32" />
      </div>
    </div>
  );
}
