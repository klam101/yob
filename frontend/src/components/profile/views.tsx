import { ExternalLink, Globe, Link2, Mail, MapPin, Phone } from "lucide-react";
import type { Answers, Education, Experience, Personal, ProfileLink, Project, Skill } from "@/api/client";
import { Badge } from "@/components/ui/badge";
import { formatRange } from "./fields";

function Empty({ children }: { children: React.ReactNode }) {
  return <p className="text-sm text-muted-foreground">{children}</p>;
}

function Bullets({ items }: { items: string[] }) {
  if (!items.length) return null;
  return (
    <ul className="list-disc space-y-1 pl-5 text-sm text-foreground/90">
      {items.map((b, i) => (
        <li key={i}>{b}</li>
      ))}
    </ul>
  );
}

function Chip({ icon: Icon, children }: { icon: React.ComponentType<{ className?: string }>; children: React.ReactNode }) {
  return (
    <span className="inline-flex max-w-full items-center gap-1.5 rounded-full bg-muted px-3 py-1 text-xs">
      <Icon className="size-3.5 shrink-0" />
      <span className="truncate">{children}</span>
    </span>
  );
}

const LINK_LABEL: Record<ProfileLink["kind"], string> = {
  linkedin: "LinkedIn",
  github: "GitHub",
  portfolio: "Portfolio",
  other: "Link",
};

export function PersonalView({ p, links }: { p: Personal; links: ProfileLink[] }) {
  const name = [p.first_name, p.middle_name, p.last_name].filter(Boolean).join(" ");
  const address = [p.street, p.city, [p.region, p.postal_code].filter(Boolean).join(" "), p.country]
    .filter(Boolean)
    .join(", ");
  if (!name && !p.email && !links.length) return <Empty>No personal details yet.</Empty>;
  return (
    <div className="space-y-3">
      <div>
        <p className="text-xl font-semibold tracking-tight">{name || "No name yet"}</p>
        {p.headline && <p className="text-sm text-muted-foreground">{p.headline}</p>}
      </div>
      <div className="flex flex-wrap gap-2">
        {address && <Chip icon={MapPin}>{address}</Chip>}
        {p.email && <Chip icon={Mail}>{p.email}</Chip>}
        {p.phone && (
          <Chip icon={Phone}>
            {p.phone}
            {p.phone_type && <span className="text-muted-foreground"> · {p.phone_type}</span>}
          </Chip>
        )}
        {links.map((l) => (
          <a
            key={l.url}
            href={l.url}
            target="_blank"
            rel="noopener noreferrer"
            className="max-w-full rounded-full focus-visible:ring-3 focus-visible:ring-ring/40 focus-visible:outline-none"
            aria-label={`${LINK_LABEL[l.kind]}: ${l.url}`}
          >
            <Chip icon={l.kind === "portfolio" ? Globe : l.kind === "other" ? ExternalLink : Link2}>
              <span className="font-medium">{LINK_LABEL[l.kind]}</span>{" "}
              <span className="text-muted-foreground">{l.url.replace(/^https?:\/\/(www\.)?/, "").replace(/\/$/, "")}</span>
            </Chip>
          </a>
        ))}
      </div>
      {p.summary && <p className="text-sm whitespace-pre-line text-foreground/90">{p.summary}</p>}
    </div>
  );
}

function Timeline({ children }: { children: React.ReactNode }) {
  return <ol>{children}</ol>;
}

/**
 * One entry on the timeline: a ring at the date, and a line from it down to
 * the next entry's ring. The line runs through the rings' centers (both are
 * centered 6px from the left) and stops 2px short of each ring, so it never
 * pokes through; the last entry's fades out below it.
 */
function TimelineItem({ when, children }: { when: string; children: React.ReactNode }) {
  return (
    // The gap to the next entry is this item's own bottom padding, so the
    // line can end just above the next ring: that ring's top is 4px into the
    // next item, and the line stops 2px short of it.
    <li className="group/item relative space-y-1 pb-6 pl-7 last:pb-0">
      {/* m-0: `space-y-1` would give these a bottom margin and cut the line short. */}
      <span aria-hidden className="absolute top-1 left-0 m-0 size-3 rounded-full border-2 border-primary bg-background" />
      <span
        aria-hidden
        // The last entry's line runs down beside it and fades out.
        className="absolute top-4.5 -bottom-0.5 left-1.25 m-0 w-0.5 rounded-full bg-primary/20 group-last/item:bottom-0 group-last/item:bg-transparent group-last/item:bg-linear-to-b group-last/item:from-primary/20 group-last/item:to-transparent"
      />
      {when && <p className="text-xs text-muted-foreground">{when}</p>}
      {children}
    </li>
  );
}

export function EducationView({ items }: { items: Education[] }) {
  if (!items.length) return <Empty>No education yet.</Empty>;
  return (
    <Timeline>
      {items.map((e, i) => (
        <TimelineItem key={i} when={formatRange(e.start, e.end, false)}>
          <p className="font-semibold">{e.school}</p>
          <p className="text-sm">
            {[[e.degree, e.field_of_study].filter(Boolean).join(" in "), e.minor && `Minor in ${e.minor}`]
              .filter(Boolean)
              .join(" · ")}
          </p>
          <p className="text-sm text-muted-foreground">
            {[e.gpa && `GPA ${e.gpa}`, e.location].filter(Boolean).join(" · ")}
          </p>
          {e.details && <p className="text-sm whitespace-pre-line text-foreground/90">{e.details}</p>}
        </TimelineItem>
      ))}
    </Timeline>
  );
}

export function ExperienceView({ items }: { items: Experience[] }) {
  if (!items.length) return <Empty>No work experience yet.</Empty>;
  return (
    <Timeline>
      {items.map((e, i) => (
        <TimelineItem key={i} when={formatRange(e.start, e.end)}>
          <p className="font-semibold">{e.company || e.title}</p>
          {e.company && e.title && <p className="text-sm">{e.title}</p>}
          {(e.employment_type || e.location) && (
            <p className="text-sm text-muted-foreground">
              {[e.employment_type, e.location].filter(Boolean).join(" · ")}
            </p>
          )}
          <Bullets items={e.bullets} />
        </TimelineItem>
      ))}
    </Timeline>
  );
}

export function ProjectView({ items }: { items: Project[] }) {
  if (!items.length) return <Empty>No projects yet.</Empty>;
  return (
    <Timeline>
      {items.map((p, i) => (
        <TimelineItem key={i} when={formatRange(p.start, p.end, false)}>
          <p className="font-semibold">
            {p.url ? (
              <a href={p.url} target="_blank" rel="noopener noreferrer" className="hover:underline">
                {p.name}
              </a>
            ) : (
              p.name
            )}
          </p>
          {p.summary && <p className="text-sm">{p.summary}</p>}
          {p.role && <p className="text-sm text-muted-foreground">{p.role}</p>}
          {p.skills.length > 0 && (
            <div className="flex flex-wrap gap-1.5">
              {p.skills.map((s) => (
                <Badge key={s} variant="flat">
                  {s}
                </Badge>
              ))}
            </div>
          )}
          <Bullets items={p.bullets} />
        </TimelineItem>
      ))}
    </Timeline>
  );
}

export function SkillsView({ items }: { items: Skill[] }) {
  if (!items.length) return <Empty>No skills yet.</Empty>;
  const groups = new Map<string, string[]>();
  for (const s of items) groups.set(s.category, [...(groups.get(s.category) ?? []), s.name]);
  return (
    <div className="space-y-4">
      {[...groups].map(([cat, names]) => (
        <div key={cat} className="space-y-2">
          {cat && <p className="text-xs font-medium text-muted-foreground">{cat}</p>}
          <div className="flex flex-wrap gap-1.5">
            {names.map((n) => (
              <Badge key={n} variant="flat">
                {n}
              </Badge>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}

const yesNo = (v: boolean | null) => (v === null ? "Not provided" : v ? "Yes" : "No");

function Row({ q, a }: { q: string; a: string }) {
  return (
    <div className="grid gap-1 py-2.5 sm:grid-cols-[1fr_16rem] sm:gap-6">
      <dt className="text-sm">{q}</dt>
      <dd>
        <span className="inline-flex rounded-lg bg-muted px-2.5 py-1 text-sm">{a || "Not provided"}</span>
      </dd>
    </div>
  );
}

export function AnswersView({ a }: { a: Answers }) {
  return (
    <dl className="divide-y divide-border/60">
      <Row q="Countries where you want to work" a={a.work_countries.join(", ")} />
      <Row q="Are you a U.S. citizen?" a={yesNo(a.us_citizen)} />
      <Row q="Are you legally authorized to work in the U.S.?" a={yesNo(a.authorized_us)} />
      <Row q="Will you now or in the future need sponsorship?" a={yesNo(a.needs_sponsorship)} />
      <Row q="Authorization basis" a={a.authorization_basis} />
      <Row q="Gender" a={a.eeo.gender} />
      <Row q="Race" a={a.eeo.race} />
      <Row q="Hispanic or Latino?" a={a.eeo.hispanic_latino} />
      <Row q="Veteran status" a={a.eeo.veteran} />
      <Row q="Disability" a={a.eeo.disability} />
      <Row q="LGBTQ+?" a={a.eeo.lgbtq} />
      <Row q="Sexual orientation" a={a.eeo.sexual_orientation} />
      <Row q="Pronouns" a={a.eeo.pronouns} />
    </dl>
  );
}
