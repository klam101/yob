import {
  ArrowUpRight,
  Ban,
  Bookmark,
  Briefcase,
  Building2,
  Clock,
  DollarSign,
  MapPin,
  type LucideIcon,
} from "lucide-react";
import type { JobSummary, Match } from "@/api/client";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import {
  MATCH_LABEL,
  MATCH_TONE,
  employmentLabel,
  formatSalary,
  isNew,
  seniorityLabel,
  timeAgo,
  workplaceLabel,
} from "@/lib/format";
import { cn } from "cn";

/** Colored initial until we have real logos. Hue is derived from the name. */
export function CompanyLogo({ name, className }: { name: string; className?: string }) {
  const hue = [...name].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 360, 7);
  return (
    <span
      aria-hidden
      style={{ "--logo-hue": hue } as React.CSSProperties}
      className={cn(
        "flex size-12 shrink-0 items-center justify-center rounded-2xl text-lg font-semibold",
        "bg-[oklch(0.94_0.04_var(--logo-hue))] text-[oklch(0.42_0.12_var(--logo-hue))]",
        "dark:bg-[oklch(0.3_0.06_var(--logo-hue))] dark:text-[oklch(0.86_0.08_var(--logo-hue))]",
        className,
      )}
    >
      {name.charAt(0)}
    </span>
  );
}

export function JobBadges({ job }: { job: JobSummary }) {
  return (
    <div className="flex flex-wrap items-center gap-1.5">
      <Badge variant="secondary" size="sm">
        {timeAgo(job.posted_at)}
      </Badge>
      {isNew(job) && (
        <Badge variant="success" size="sm">
          New
        </Badge>
      )}
      {job.repost_count > 0 && (
        <Badge variant="warning" size="sm">
          Reposted ×{job.repost_count}
        </Badge>
      )}
      {job.possibly_stale && (
        <Tooltip>
          <TooltipTrigger asChild>
            <Badge variant="destructive" size="sm" tabIndex={0}>
              Possibly stale
            </Badge>
          </TooltipTrigger>
          <TooltipContent>Open more than 60 days or reposted more than twice</TooltipContent>
        </Tooltip>
      )}
    </div>
  );
}

function MetaItem({ icon: Icon, children }: { icon: LucideIcon; children: React.ReactNode }) {
  return (
    <span className="flex min-w-0 items-center gap-2 text-sm text-foreground/80">
      <Icon className="size-4 shrink-0 text-muted-foreground" />
      <span className="truncate">{children}</span>
    </span>
  );
}

export function JobMeta({ job, className }: { job: JobSummary; className?: string }) {
  const [first, ...rest] = job.locations;
  return (
    <div className={cn("grid grid-cols-2 gap-x-4 gap-y-2 sm:gap-x-6 lg:grid-cols-3", className)}>
      <MetaItem icon={MapPin}>
        {first}
        {rest.length > 0 && (
          <Tooltip>
            <TooltipTrigger asChild>
              <Badge variant="flat" size="sm" className="relative z-10 ml-1.5 align-middle" tabIndex={0}>
                +{rest.length} more
              </Badge>
            </TooltipTrigger>
            <TooltipContent>{rest.join(" · ")}</TooltipContent>
          </Tooltip>
        )}
      </MetaItem>
      <MetaItem icon={Building2}>{workplaceLabel(job.workplace)}</MetaItem>
      <MetaItem icon={Clock}>{employmentLabel(job.employment_type)}</MetaItem>
      <MetaItem icon={Briefcase}>{seniorityLabel(job.seniority)}</MetaItem>
      <MetaItem icon={DollarSign}>
        {job.salary ? (
          formatSalary(job.salary)
        ) : (
          <span className="text-muted-foreground">Salary not listed</span>
        )}
      </MetaItem>
    </div>
  );
}

/** Circular score with the label color; `size` in px. */
export function ScoreRing({ match, size = 64 }: { match: Match; size?: number }) {
  const stroke = Math.max(4, Math.round(size / 12));
  const r = (size - stroke) / 2;
  const c = 2 * Math.PI * r;
  return (
    <span
      className={cn("relative inline-flex shrink-0 items-center justify-center", MATCH_TONE[match.label])}
      style={{ width: size, height: size }}
      role="img"
      aria-label={`${match.score}% ${MATCH_LABEL[match.label]}`}
    >
      <svg width={size} height={size} className="-rotate-90">
        <circle cx={size / 2} cy={size / 2} r={r} fill="none" strokeWidth={stroke} className="stroke-current opacity-15" />
        <circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          fill="none"
          strokeWidth={stroke}
          strokeLinecap="round"
          strokeDasharray={c}
          strokeDashoffset={c * (1 - match.score / 100)}
          className="stroke-current"
        />
      </svg>
      <span className="absolute font-semibold tracking-tight text-foreground" style={{ fontSize: size * 0.28 }}>
        {match.score}
        <span className="text-[0.6em] text-muted-foreground">%</span>
      </span>
    </span>
  );
}

/** A visible action that isn't available yet; explains why on hover. */
export function UnavailableAction({
  icon: Icon,
  label,
  reason,
}: {
  icon: LucideIcon;
  label: string;
  reason: string;
}) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span tabIndex={0} className="relative z-10 rounded-xl" aria-label={`${label} (${reason})`}>
          <Button variant="ghost" size="icon-sm" disabled aria-hidden tabIndex={-1}>
            <Icon />
          </Button>
        </span>
      </TooltipTrigger>
      <TooltipContent>
        {label}: {reason}
      </TooltipContent>
    </Tooltip>
  );
}

export function SaveHideActions() {
  const reason = "available once the tracker is built";
  return (
    <>
      <UnavailableAction icon={Ban} label="Hide" reason={reason} />
      <UnavailableAction icon={Bookmark} label="Save" reason={reason} />
    </>
  );
}

export function ApplyButton({ href, size = "sm" }: { href: string; size?: "sm" | "default" }) {
  return (
    <Button asChild size={size} className="relative z-10">
      <a href={href} target="_blank" rel="noopener noreferrer">
        Apply <ArrowUpRight data-icon="inline-end" />
      </a>
    </Button>
  );
}
