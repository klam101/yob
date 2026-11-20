import {
  ArrowUpRight,
  Check,
  FileText,
  GraduationCap,
  Info,
  Mail,
  Plus,
  Sparkles,
  Target,
  Timer,
  Users,
  type LucideIcon,
} from "lucide-react";
import Link from "next/link";
import type { JobDetail, JobSummary } from "@/api/client";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { MATCH_LABEL, MATCH_TONE, peopleLinks, sourceLabel, timeAgo } from "@/lib/format";
import { cn } from "cn";
import { BackButton } from "./back-button";
import {
  ApplyButton,
  CompanyLogo,
  JobBadges,
  JobMeta,
  SaveHideActions,
  ScoreRing,
} from "./job-parts";

export function JobDetailView({ detail, related }: { detail: JobDetail; related: JobSummary[] }) {
  const { job } = detail;

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-center gap-2">
        <BackButton />
        <div className="ml-auto flex items-center gap-1">
          <SaveHideActions />
          <Button asChild variant="light" size="sm" shape="pill" className="hidden sm:inline-flex">
            <a href={job.apply_url} target="_blank" rel="noopener noreferrer">
              Original posting <ArrowUpRight data-icon="inline-end" />
            </a>
          </Button>
          <ApplyButton href={job.apply_url} size="default" />
        </div>
      </div>

      <div className="grid gap-5 lg:grid-cols-[minmax(0,1fr)_20rem]">
        <div className="min-w-0 space-y-5">
          <section className="rounded-3xl border border-border/70 bg-card p-5 shadow-soft sm:p-7">
            <div className="flex flex-col gap-6 xl:flex-row xl:items-start">
              <div className="min-w-0 flex-1 space-y-4">
                <div className="flex items-center gap-3">
                  <CompanyLogo name={job.company.name} className="size-11 rounded-xl text-base" />
                  <p className="text-sm">
                    <span className="font-semibold">{job.company.name}</span>
                    <span className="text-muted-foreground">
                      {" "}
                      · posted {timeAgo(job.posted_at)} · via {sourceLabel(job.source)}
                    </span>
                  </p>
                </div>
                <h1 className="text-2xl leading-tight font-semibold tracking-tight text-balance sm:text-3xl">
                  {job.title}
                </h1>
                <JobBadges job={job} />
                <JobMeta job={job} className="pt-1 lg:grid-cols-2" />
                {job.closed_at && (
                  <p className="flex gap-2 rounded-xl bg-destructive/10 px-3 py-2.5 text-sm text-destructive">
                    <Info className="mt-0.5 size-4 shrink-0" />
                    This job was removed from {job.company.name}&apos;s job board {timeAgo(job.closed_at)} and
                    is probably no longer accepting applications.
                  </p>
                )}
                {detail.workplace_note && (
                  <p className="flex gap-2 rounded-xl bg-muted/70 px-3 py-2.5 text-sm text-foreground/80">
                    <Info className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
                    {detail.workplace_note}
                  </p>
                )}
              </div>
              <MatchCard detail={detail} />
            </div>

            {(detail.summary || job.company.industries.length > 0) && (
              <div className="mt-6 space-y-3 border-t border-border/60 pt-6">
                {detail.summary && <p className="leading-relaxed text-pretty">{detail.summary}</p>}
                {job.company.industries.length > 0 && (
                  <div className="flex flex-wrap gap-1.5">
                    {job.company.industries.map((i) => (
                      <Badge key={i} variant="flat">
                        {i}
                      </Badge>
                    ))}
                  </div>
                )}
              </div>
            )}
          </section>

          <Tabs defaultValue="overview" className="gap-4">
            <TabsList className="h-11 rounded-2xl p-1">
              <TabsTrigger value="overview" className="rounded-xl px-4">
                Overview
              </TabsTrigger>
              <TabsTrigger value="company" className="rounded-xl px-4">
                Company
              </TabsTrigger>
            </TabsList>

            <TabsContent value="overview" className="space-y-5">
              <Requirements detail={detail} />
              {detail.keywords && <KeywordGap matched={detail.keywords.matched} missing={detail.keywords.missing} />}
              <section className="rounded-3xl border border-border/70 bg-card p-5 shadow-soft sm:p-7">
                <h2 className="mb-4 text-lg font-semibold">Full description</h2>
                {detail.description_html ? (
                  // Sanitized by the worker (ammonia) before it's stored.
                  <div className="prose-job" dangerouslySetInnerHTML={{ __html: detail.description_html }} />
                ) : (
                  <p className="text-sm text-muted-foreground">
                    The description isn&apos;t kept for jobs that closed a long time ago.
                  </p>
                )}
              </section>
            </TabsContent>

            <TabsContent value="company">
              <CompanyPanel job={job} related={related} />
            </TabsContent>
          </Tabs>
        </div>

        <aside className="space-y-5 lg:sticky lg:top-6 lg:self-start">
          <ToolsCard />
          <PeopleCard job={job} />
        </aside>
      </div>
    </div>
  );
}

function MatchCard({ detail }: { detail: JobDetail }) {
  const match = detail.job.match;
  if (!match || !detail.breakdown) {
    return (
      <div className="flex w-full shrink-0 flex-col items-center gap-2 rounded-2xl border border-dashed border-border p-5 text-center xl:w-64">
        <Sparkles className="size-5 text-muted-foreground" />
        <p className="text-sm font-medium">Not scored yet</p>
        <p className="text-xs text-muted-foreground">Add your resume to see how well you fit.</p>
      </div>
    );
  }
  const rows = [
    { label: "Experience level", value: detail.breakdown.experience },
    { label: "Skills", value: detail.breakdown.skill },
    { label: "Education", value: detail.breakdown.education },
  ];
  return (
    <div className="w-full shrink-0 rounded-2xl bg-primary/5 p-5 ring-1 ring-primary/15 xl:w-64 dark:bg-primary/8">
      <div className="flex items-center gap-4">
        <ScoreRing match={match} size={68} />
        <div>
          <p className={cn("font-semibold", MATCH_TONE[match.label])}>{MATCH_LABEL[match.label]}</p>
          <p className="text-xs text-muted-foreground">Compared with your profile</p>
        </div>
      </div>
      <dl className="mt-4 space-y-3">
        {rows.map((r) => (
          <div key={r.label} className="space-y-1.5">
            <div className="flex justify-between text-sm">
              <dt className="text-foreground/80">{r.label}</dt>
              <dd className="font-semibold tabular-nums">{r.value}%</dd>
            </div>
            <div className="h-1.5 overflow-hidden rounded-full bg-primary/12">
              <div className="h-full rounded-full bg-primary" style={{ width: `${r.value}%` }} />
            </div>
          </div>
        ))}
      </dl>
      <ul className="mt-4 space-y-1.5 border-t border-primary/15 pt-3">
        {match.reasons.map((reason) => (
          <li key={reason} className="flex gap-1.5 text-xs leading-snug text-muted-foreground">
            <Check className="mt-0.5 size-3.5 shrink-0 text-primary" />
            {reason}
          </li>
        ))}
      </ul>
    </div>
  );
}

function Requirements({ detail }: { detail: JobDetail }) {
  const items: { icon: LucideIcon; label: string; value: string }[] = [
  ];
  // Only facts we actually extracted: unknown is not "no experience required".
  if (detail.yoe_min !== null && detail.yoe_min !== undefined) {
    items.push({
      icon: Timer,
      label: "Experience",
      value: detail.yoe_min === 0 ? "No experience required" : `${detail.yoe_min}+ years`,
    });
  }
  if (detail.education) items.push({ icon: GraduationCap, label: "Education", value: detail.education });
  const showSkills = !detail.keywords && detail.skills.length > 0;
  if (items.length === 0 && !showSkills) return null;

  return (
    <section className="rounded-3xl border border-border/70 bg-card p-5 shadow-soft sm:p-7">
      <h2 className="mb-4 text-lg font-semibold">Requirements</h2>
      <div className="grid gap-4 sm:grid-cols-2">
        {items.map(({ icon: Icon, label, value }) => (
          <div key={label} className="flex gap-3">
            <span className="flex size-9 shrink-0 items-center justify-center rounded-xl bg-muted text-muted-foreground">
              <Icon className="size-4" />
            </span>
            <div className="min-w-0">
              <p className="text-xs font-medium text-muted-foreground">{label}</p>
              <p className="text-sm">{value}</p>
            </div>
          </div>
        ))}
      </div>
      {/* With a keyword gap below, the skills already appear there. */}
      {showSkills && (
        <>
          <p className="mt-5 mb-2 text-xs font-medium text-muted-foreground">Skills mentioned</p>
          <div className="flex flex-wrap gap-1.5">
            {detail.skills.map((s) => (
              <Badge key={s} variant="outline" size="lg">
                {s}
              </Badge>
            ))}
          </div>
        </>
      )}
    </section>
  );
}

function KeywordGap({ matched, missing }: { matched: string[]; missing: string[] }) {
  const total = matched.length + missing.length;
  return (
    <section className="rounded-3xl border border-border/70 bg-card p-5 shadow-soft sm:p-7">
      <div className="mb-4 flex items-baseline justify-between gap-3">
        <h2 className="text-lg font-semibold">Keyword gap</h2>
        <p className="text-sm text-muted-foreground">
          <span className="font-semibold text-foreground tabular-nums">{matched.length}</span> of {total} covered by your profile
        </p>
      </div>
      <div className="flex flex-wrap gap-1.5">
        {matched.map((k) => (
          <Badge key={k} variant="success" size="lg">
            <Check /> {k}
          </Badge>
        ))}
        {missing.map((k) => (
          <Badge key={k} variant="outline" size="lg" className="border-dashed text-muted-foreground">
            <Plus /> {k}
          </Badge>
        ))}
      </div>
      {missing.length > 0 && (
        <p className="mt-4 text-xs text-muted-foreground">
          Dashed keywords aren&apos;t in your profile. Add them to your resume only if they&apos;re true for you.
        </p>
      )}
    </section>
  );
}

function CompanyPanel({ job, related }: { job: JobSummary; related: JobSummary[] }) {
  return (
    <section className="space-y-5 rounded-3xl border border-border/70 bg-card p-5 shadow-soft sm:p-7">
      <div className="flex items-center gap-4">
        <CompanyLogo name={job.company.name} />
        <div>
          <h2 className="text-lg font-semibold">{job.company.name}</h2>
          <a
            href={job.company.careers_url}
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-1 text-sm text-primary hover:underline"
          >
            Careers page <ArrowUpRight className="size-3.5" />
          </a>
        </div>
      </div>
      {job.company.description && <p className="leading-relaxed">{job.company.description}</p>}
      {job.company.industries.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {job.company.industries.map((i) => (
            <Badge key={i} variant="flat">
              {i}
            </Badge>
          ))}
        </div>
      )}
      <div>
        <h3 className="mb-2 text-sm font-semibold">
          Other open roles <span className="font-normal text-muted-foreground">({related.length})</span>
        </h3>
        {related.length === 0 ? (
          <p className="text-sm text-muted-foreground">No other open roles right now.</p>
        ) : (
          <ul className="divide-y divide-border/60 rounded-2xl border border-border/60">
            {related.map((r) => (
              <li key={r.id}>
                <Link
                  href={`/jobs/${r.id}`}
                  className="flex items-center justify-between gap-3 px-4 py-3 text-sm transition-colors first:rounded-t-2xl last:rounded-b-2xl hover:bg-muted"
                >
                  <span className="truncate font-medium">{r.title}</span>
                  <span className="shrink-0 text-xs text-muted-foreground">{timeAgo(r.posted_at)}</span>
                </Link>
              </li>
            ))}
          </ul>
        )}
      </div>
    </section>
  );
}

function ToolsCard() {
  const tools = [
    { icon: FileText, title: "Tailor your resume", body: "Reorder and reword your experience for this job" },
    { icon: Mail, title: "Write a cover letter", body: "Drafted from your profile and this posting" },
    { icon: Target, title: "Analyze your fit", body: "Strengths, gaps and what to emphasize" },
  ];
  return (
    <section className="rounded-3xl border border-border/70 bg-card p-5 shadow-soft">
      <div className="mb-3 flex items-center justify-between">
        <h2 className="font-semibold">AI tools</h2>
        <Badge variant="flat" size="sm">
          Coming soon
        </Badge>
      </div>
      <ul className="space-y-2">
        {tools.map(({ icon: Icon, title, body }) => (
          <li key={title} className="flex gap-3 rounded-2xl border border-border/60 p-3 opacity-70">
            <span className="flex size-9 shrink-0 items-center justify-center rounded-xl bg-primary/12 text-primary dark:bg-primary/20">
              <Icon className="size-4" />
            </span>
            <span>
              <span className="block text-sm font-medium">{title}</span>
              <span className="block text-xs text-muted-foreground">{body}</span>
            </span>
          </li>
        ))}
      </ul>
      <p className="mt-3 text-xs text-muted-foreground">Available once your profile and documents are set up.</p>
    </section>
  );
}

function PeopleCard({ job }: { job: JobSummary }) {
  return (
    <section className="rounded-3xl border border-border/70 bg-card p-5 shadow-soft">
      <div className="mb-1 flex items-center gap-2">
        <Users className="size-4 text-primary" />
        <h2 className="font-semibold">People at {job.company.name}</h2>
      </div>
      <p className="mb-3 text-xs text-muted-foreground">
        Find someone to ask about the role. Opens a search in a new tab.
      </p>
      <ul className="space-y-1.5">
        {peopleLinks(job.company.name, job.title, job.department).map((l) => (
          <li key={l.label}>
            <a
              href={l.href}
              target="_blank"
              rel="noopener noreferrer"
              className="group flex items-center justify-between gap-3 rounded-xl px-3 py-2.5 transition-colors hover:bg-muted"
            >
              <span>
                <span className="block text-sm font-medium">{l.label}</span>
                <span className="block text-xs text-muted-foreground">{l.hint}</span>
              </span>
              <ArrowUpRight className="size-4 text-muted-foreground transition-transform group-hover:translate-x-0.5 group-hover:-translate-y-0.5" />
            </a>
          </li>
        ))}
      </ul>
    </section>
  );
}
