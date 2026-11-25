"use client";

import { CheckCircle2, ChevronRight, CircleAlert, FileText, PencilLine, ShieldCheck, SlidersHorizontal } from "lucide-react";
import Link from "next/link";
import { useEffect, useState } from "react";
import type { Profile, ResumeDraft } from "@/api/client";
import { Button } from "@/components/ui/button";
import { cn } from "cn";
import {
  AnswersEditor,
  EducationEditor,
  ExperienceEditor,
  LinksEditor,
  PersonalEditor,
  SkillsEditor,
  skillsToText,
  textToSkills,
} from "./editors";
import { ResumeReview } from "./resume-review";
import { ResumeUpload } from "./resume-upload";
import { isEmpty, loadProfile, save } from "./save";
import { Section } from "./section";
import { AnswersView, EducationView, ExperienceView, PersonalView, SkillsView } from "./views";

const SECTIONS = [
  { id: "personal", label: "Personal" },
  { id: "education", label: "Education" },
  { id: "experience", label: "Work experience" },
  { id: "skills", label: "Skills" },
  { id: "answers", label: "Application answers" },
] as const;
type SectionId = (typeof SECTIONS)[number]["id"];

/** What's missing, each pointing at the section that fixes it. */
function missing(p: Profile): { label: string; section: SectionId }[] {
  const out: { label: string; section: SectionId }[] = [];
  if (!p.personal.first_name || !p.personal.last_name) out.push({ label: "Your name", section: "personal" });
  if (!p.personal.email) out.push({ label: "Email", section: "personal" });
  if (!p.personal.phone) out.push({ label: "Phone", section: "personal" });
  if (!p.personal.city && !p.personal.region) out.push({ label: "Where you live", section: "personal" });
  if (!p.educations.length) out.push({ label: "Education", section: "education" });
  if (!p.experiences.length) out.push({ label: "Work experience", section: "experience" });
  if (!p.skills.length) out.push({ label: "Skills", section: "skills" });
  if (p.answers.authorized_us === null || p.answers.needs_sponsorship === null)
    out.push({ label: "Work authorization", section: "answers" });
  return out;
}

/** The section in view, for highlighting its tab. */
function useActiveSection() {
  const [active, setActive] = useState<SectionId>("personal");
  useEffect(() => {
    const els = SECTIONS.map((s) => document.getElementById(s.id)).filter((e): e is HTMLElement => !!e);
    const seen = new Map<string, boolean>();
    const io = new IntersectionObserver(
      (entries) => {
        for (const e of entries) seen.set(e.target.id, e.isIntersecting);
        const first = SECTIONS.find((s) => seen.get(s.id));
        if (first) setActive(first.id);
      },
      // The band just under the sticky tabs.
      { rootMargin: "-120px 0px -55% 0px" },
    );
    els.forEach((el) => io.observe(el));
    return () => io.disconnect();
  }, []);
  return [active, setActive] as const;
}

export function ProfileEditor({ initial }: { initial: Profile }) {
  const [profile, setProfile] = useState(initial);
  const [draft, setDraft] = useState<ResumeDraft | null>(null);
  const [manual, setManual] = useState(false);
  const [editing, setEditing] = useState<SectionId | null>(null);
  const [active, setActive] = useActiveSection();

  const patch = (p: Partial<Profile>) => setProfile((cur) => ({ ...cur, ...p }));
  const sheet = (id: SectionId) => ({
    id,
    open: editing === id,
    onOpenChange: (o: boolean) => setEditing(o ? id : null),
  });

  async function afterReview() {
    setDraft(null);
    const fresh = await loadProfile();
    if (fresh) setProfile(fresh);
  }

  if (draft) {
    return <ResumeReview draft={draft} current={profile} onSaved={afterReview} onCancel={() => setDraft(null)} />;
  }

  if (isEmpty(profile) && !manual) {
    return (
      <div className="mx-auto max-w-2xl space-y-6 rounded-3xl border border-border/70 bg-card p-6 shadow-soft sm:p-8">
        <div className="space-y-1">
          <h2 className="text-xl font-semibold tracking-tight">Set up your profile</h2>
          <p className="text-sm text-muted-foreground">
            Your profile is the source for tailored resumes, cover letters and autofill. Start from your resume
            and check what we read, or fill it in yourself.
          </p>
        </div>
        <ResumeUpload
          onParsed={(d) => {
            setDraft(d);
            window.scrollTo({ top: 0 });
          }}
        />
        <div className="flex items-center gap-3 text-xs text-muted-foreground">
          <span className="h-px flex-1 bg-border" /> or <span className="h-px flex-1 bg-border" />
        </div>
        <Button variant="bordered" className="w-full" onClick={() => setManual(true)}>
          <PencilLine /> Fill it in manually
        </Button>
        <p className="flex items-start gap-2 text-xs text-muted-foreground">
          <ShieldCheck className="size-4 shrink-0" />
          Reading the resume happens on this computer; nothing is sent anywhere.
        </p>
      </div>
    );
  }

  const todo = missing(profile);

  return (
    <div className="grid gap-6 lg:grid-cols-[1fr_18rem]">
      <div className="min-w-0 space-y-4">
        <p className="flex items-center gap-2 text-sm text-muted-foreground">
          <ShieldCheck className="size-4 shrink-0 text-success" />
          Your profile stays on this computer.
        </p>
        <div className="rounded-3xl border border-border/70 bg-card shadow-soft">
          <nav
            aria-label="Profile sections"
            className="sticky top-0 z-10 overflow-x-auto rounded-t-3xl border-b border-border/70 bg-card/95 px-3 backdrop-blur sm:px-5"
          >
            <ul className="flex gap-1">
              {SECTIONS.map((s) => (
                <li key={s.id} className="shrink-0">
                  <a
                    href={`#${s.id}`}
                    aria-current={active === s.id ? "location" : undefined}
                    onClick={() => setActive(s.id)}
                    className={cn(
                      "-mb-px block border-b-2 px-3 py-3.5 text-sm font-medium whitespace-nowrap transition-colors",
                      active === s.id
                        ? "border-primary text-foreground"
                        : "border-transparent text-muted-foreground hover:text-foreground",
                    )}
                  >
                    {s.label}
                  </a>
                </li>
              ))}
            </ul>
          </nav>
          <div className="divide-y divide-border/60 px-5 sm:px-7">
            <div className="py-6 sm:py-7">
              <Section
                {...sheet("personal")}
                title="Personal"
                scope="personal"
                view={<PersonalView p={profile.personal} links={profile.links} />}
                initial={() => ({ personal: profile.personal, links: profile.links })}
                edit={(d, set) => (
                  <div className="space-y-8">
                    <PersonalEditor value={d.personal} onChange={(personal) => set({ ...d, personal })} />
                    <div className="space-y-3">
                      <h3 className="text-base font-semibold">Links</h3>
                      <LinksEditor value={d.links} onChange={(links) => set({ ...d, links })} />
                    </div>
                  </div>
                )}
                onSave={async (d) => {
                  const p = await save.personal(d.personal);
                  if (!p.ok) return p;
                  const l = await save.links(d.links);
                  if (!l.ok) return l;
                  patch({ personal: p.value, links: l.value });
                  return { ok: true };
                }}
              />
            </div>
            <div className="py-6 sm:py-7">
              <Section
                {...sheet("education")}
                title="Education"
                scope="educations"
                view={<EducationView items={profile.educations} />}
                initial={() => profile.educations}
                edit={(d, set) => <EducationEditor value={d} onChange={set} />}
                onSave={async (d) => {
                  const r = await save.educations(d);
                  if (r.ok) patch({ educations: r.value });
                  return r;
                }}
              />
            </div>
            <div className="py-6 sm:py-7">
              <Section
                {...sheet("experience")}
                title="Work experience"
                scope="experiences"
                view={<ExperienceView items={profile.experiences} />}
                initial={() => profile.experiences}
                edit={(d, set) => <ExperienceEditor value={d} onChange={set} />}
                onSave={async (d) => {
                  const r = await save.experiences(d);
                  if (r.ok) patch({ experiences: r.value });
                  return r;
                }}
              />
            </div>
            <div className="py-6 sm:py-7">
              <Section
                {...sheet("skills")}
                title="Skills"
                view={<SkillsView items={profile.skills} />}
                initial={() => skillsToText(profile.skills)}
                edit={(d, set) => <SkillsEditor value={d} onChange={set} />}
                onSave={async (d) => {
                  const r = await save.skills(textToSkills(d));
                  if (r.ok) patch({ skills: r.value });
                  return r;
                }}
              />
            </div>
            <div className="py-6 sm:py-7">
              <Section
                {...sheet("answers")}
                title="Application answers"
                view={<AnswersView a={profile.answers} />}
                initial={() => profile.answers}
                edit={(d, set) => <AnswersEditor value={d} onChange={set} />}
                onSave={async (d) => {
                  const r = await save.answers(d);
                  if (r.ok) patch({ answers: r.value });
                  return r;
                }}
              />
            </div>
          </div>
        </div>
      </div>

      <aside className="space-y-4 lg:sticky lg:top-6 lg:self-start">
        <div
          className={cn(
            "space-y-3 rounded-3xl border p-5 shadow-soft",
            todo.length ? "border-border/70 bg-card" : "border-success/30 bg-success/10",
          )}
        >
          {todo.length === 0 ? (
            <>
              <CheckCircle2 className="size-6 text-success" />
              <p className="text-sm font-medium">Your profile is complete. Autofill and tailoring have what they need.</p>
              <Button asChild className="w-full">
                <Link href="/jobs">Find jobs</Link>
              </Button>
            </>
          ) : (
            <>
              <div className="flex items-center gap-2">
                <CircleAlert className="size-4 text-warning" />
                <h2 className="text-sm font-semibold">
                  {todo.length} {todo.length === 1 ? "thing" : "things"} left to add
                </h2>
              </div>
              <p className="text-xs text-muted-foreground">Applications ask for these. Add them once here.</p>
              <ul className="space-y-0.5">
                {todo.map((t) => (
                  <li key={t.label}>
                    <button
                      type="button"
                      onClick={() => setEditing(t.section)}
                      className="flex w-full items-center gap-2 rounded-lg px-1.5 py-1.5 text-left text-sm hover:bg-muted"
                    >
                      <span className="size-1.5 shrink-0 rounded-full bg-warning" aria-hidden />
                      <span className="flex-1">{t.label}</span>
                      <ChevronRight className="size-4 text-muted-foreground" />
                    </button>
                  </li>
                ))}
              </ul>
            </>
          )}
        </div>
        <nav aria-label="Related" className="divide-y divide-border/60 rounded-3xl border border-border/70 bg-card shadow-soft">
          <Link href="/resume" className="flex items-center gap-3 px-5 py-4 text-sm font-medium hover:bg-muted/60 first:rounded-t-3xl">
            <FileText className="size-4 text-muted-foreground" />
            <span className="flex-1">Manage my resume</span>
            <ChevronRight className="size-4 text-muted-foreground" />
          </Link>
          <Link
            href="/preferences"
            className="flex items-center gap-3 px-5 py-4 text-sm font-medium hover:bg-muted/60 last:rounded-b-3xl"
          >
            <SlidersHorizontal className="size-4 text-muted-foreground" />
            <span className="flex-1">Job preferences</span>
            <ChevronRight className="size-4 text-muted-foreground" />
          </Link>
        </nav>
      </aside>
    </div>
  );
}
