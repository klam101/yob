"use client";

import { CheckCircle2, Circle, Download, PencilLine, ShieldCheck } from "lucide-react";
import { useState } from "react";
import type { Profile, ResumeDraft, ResumeInfo } from "@/api/client";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { timeAgo } from "@/lib/format";
import {
  AnswersEditor,
  EducationEditor,
  ExperienceEditor,
  LinksEditor,
  PersonalEditor,
  ProjectEditor,
  SkillsEditor,
  skillsToText,
  textToSkills,
} from "./editors";
import { ResumeReview } from "./resume-review";
import { ResumeUpload } from "./resume-upload";
import { isEmpty, loadProfile, save } from "./save";
import { AnswersView, EducationView, ExperienceView, PersonalView, ProjectView, SkillsView } from "./views";

type Result = { ok: true } | { ok: false; message: string };

/**
 * One section: read-only until the pencil is pressed, then an editor with
 * Save and Cancel. Saving replaces the whole section.
 */
function Section<D>({
  title,
  view,
  initial,
  edit,
  onSave,
}: {
  title: string;
  view: React.ReactNode;
  initial: () => D;
  edit: (draft: D, set: (d: D) => void) => React.ReactNode;
  onSave: (draft: D) => Promise<Result>;
}) {
  const [draft, setDraft] = useState<D | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function submit() {
    if (draft === null) return;
    setSaving(true);
    setError(null);
    const r = await onSave(draft);
    setSaving(false);
    if (r.ok) setDraft(null);
    else setError(r.message);
  }

  return (
    <section className="space-y-4">
      <div className="flex items-center justify-between gap-3">
        <h2 className="text-lg font-semibold">{title}</h2>
        {draft === null && (
          <Button variant="ghost" size="icon" aria-label={`Edit ${title.toLowerCase()}`} onClick={() => setDraft(initial())}>
            <PencilLine />
          </Button>
        )}
      </div>
      {draft === null ? (
        view
      ) : (
        <div className="space-y-4">
          {edit(draft, setDraft)}
          {error && (
            <p role="alert" className="rounded-xl bg-destructive/10 px-3 py-2 text-sm text-destructive">
              {error}
            </p>
          )}
          <div className="flex gap-2">
            <Button onClick={submit} disabled={saving}>
              {saving ? "Saving…" : "Save"}
            </Button>
            <Button
              variant="ghost"
              disabled={saving}
              onClick={() => {
                setDraft(null);
                setError(null);
              }}
            >
              Cancel
            </Button>
          </div>
        </div>
      )}
    </section>
  );
}

const TABS = [
  { id: "personal", label: "Personal" },
  { id: "experience", label: "Experience" },
  { id: "education", label: "Education" },
  { id: "projects", label: "Projects" },
  { id: "skills", label: "Skills" },
  { id: "answers", label: "Application answers" },
] as const;

export function ProfileEditor({ initial, resumes: initialResumes }: { initial: Profile; resumes: ResumeInfo[] }) {
  const [profile, setProfile] = useState(initial);
  const [resumes, setResumes] = useState(initialResumes);
  const [draft, setDraft] = useState<ResumeDraft | null>(null);
  const [manual, setManual] = useState(false);
  const [tab, setTab] = useState<string>("personal");

  const patch = (p: Partial<Profile>) => setProfile((cur) => ({ ...cur, ...p }));

  async function afterReview() {
    setDraft(null);
    const fresh = await loadProfile();
    if (fresh) setProfile(fresh);
  }

  function parsed(d: ResumeDraft) {
    setDraft(d);
    setResumes((r) => [
      {
        id: d.resume_id,
        filename: d.filename,
        content_type: "",
        size_bytes: 0,
        uploaded_at: new Date().toISOString(),
      },
      ...r,
    ]);
    window.scrollTo({ top: 0 });
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
        <ResumeUpload onParsed={parsed} />
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

  const done = {
    personal: !!(profile.personal.first_name && profile.personal.email),
    experience: profile.experiences.length > 0,
    education: profile.educations.length > 0,
    projects: profile.projects.length > 0,
    skills: profile.skills.length > 0,
    answers: profile.answers.authorized_us !== null,
  };

  return (
    <div className="grid gap-6 lg:grid-cols-[1fr_18rem]">
      <Tabs value={tab} onValueChange={setTab} className="min-w-0 gap-4">
        <TabsList variant="line" className="w-full justify-start overflow-x-auto border-b border-border/70 pb-1">
          {TABS.map((t) => (
            <TabsTrigger key={t.id} value={t.id} className="flex-none px-3">
              {t.label}
            </TabsTrigger>
          ))}
        </TabsList>
        <div className="rounded-3xl border border-border/70 bg-card p-5 shadow-soft sm:p-7">
          <TabsContent value="personal">
            <Section
              title="Personal details"
              view={<PersonalView p={profile.personal} links={profile.links} />}
              initial={() => ({ personal: profile.personal, links: profile.links })}
              edit={(d, set) => (
                <div className="space-y-6">
                  <PersonalEditor value={d.personal} onChange={(personal) => set({ ...d, personal })} />
                  <div className="space-y-3">
                    <h3 className="text-sm font-semibold">Links</h3>
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
          </TabsContent>
          <TabsContent value="experience">
            <Section
              title="Work experience"
              view={<ExperienceView items={profile.experiences} />}
              initial={() => profile.experiences}
              edit={(d, set) => <ExperienceEditor value={d} onChange={set} />}
              onSave={async (d) => {
                const r = await save.experiences(d);
                if (r.ok) patch({ experiences: r.value });
                return r;
              }}
            />
          </TabsContent>
          <TabsContent value="education">
            <Section
              title="Education"
              view={<EducationView items={profile.educations} />}
              initial={() => profile.educations}
              edit={(d, set) => <EducationEditor value={d} onChange={set} />}
              onSave={async (d) => {
                const r = await save.educations(d);
                if (r.ok) patch({ educations: r.value });
                return r;
              }}
            />
          </TabsContent>
          <TabsContent value="projects">
            <Section
              title="Projects"
              view={<ProjectView items={profile.projects} />}
              initial={() => profile.projects}
              edit={(d, set) => <ProjectEditor value={d} onChange={set} />}
              onSave={async (d) => {
                const r = await save.projects(d);
                if (r.ok) patch({ projects: r.value });
                return r;
              }}
            />
          </TabsContent>
          <TabsContent value="skills">
            <Section
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
          </TabsContent>
          <TabsContent value="answers">
            <Section
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
          </TabsContent>
        </div>
      </Tabs>

      <aside className="space-y-4">
        <div className="space-y-3 rounded-3xl border border-border/70 bg-card p-5 shadow-soft">
          <h2 className="text-sm font-semibold">Profile checklist</h2>
          <ul className="space-y-1">
            {TABS.map((t) => (
              <li key={t.id}>
                <button
                  type="button"
                  onClick={() => setTab(t.id)}
                  className="flex w-full items-center gap-2 rounded-lg px-1.5 py-1 text-left text-sm hover:bg-muted"
                >
                  {done[t.id] ? (
                    <CheckCircle2 className="size-4 text-success" />
                  ) : (
                    <Circle className="size-4 text-muted-foreground" />
                  )}
                  {t.label}
                </button>
              </li>
            ))}
          </ul>
        </div>
        <div className="space-y-3 rounded-3xl border border-border/70 bg-card p-5 shadow-soft">
          <h2 className="text-sm font-semibold">Resumes</h2>
          <ResumeUpload compact onParsed={parsed} />
          {resumes.length > 0 && (
            <ul className="space-y-1">
              {resumes.slice(0, 5).map((r) => (
                <li key={r.id}>
                  <a
                    href={`/api/profile/resumes/${r.id}/file`}
                    className="flex items-center gap-2 rounded-lg px-1.5 py-1 text-sm hover:bg-muted"
                  >
                    <Download className="size-4 shrink-0 text-muted-foreground" />
                    <span className="min-w-0 flex-1 truncate">{r.filename}</span>
                    <span className="shrink-0 text-xs text-muted-foreground" suppressHydrationWarning>
                      {timeAgo(r.uploaded_at)}
                    </span>
                  </a>
                </li>
              ))}
            </ul>
          )}
        </div>
      </aside>
    </div>
  );
}
