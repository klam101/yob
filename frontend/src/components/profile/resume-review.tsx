"use client";

import { Info } from "lucide-react";
import { useState } from "react";
import type { Personal, Profile, ResumeDraft } from "@/api/client";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  EducationEditor,
  ExperienceEditor,
  LinksEditor,
  PersonalEditor,
  ProjectEditor,
  SkillsEditor,
  skillsToText,
  textToSkills,
} from "./editors";
import { save } from "./save";

type Key = "personal" | "educations" | "experiences" | "projects" | "skills";

/** Parsed values fill in; fields the resume didn't have keep what's saved. */
function mergePersonal(saved: Personal, parsed: Personal): Personal {
  const out = { ...saved };
  for (const k of Object.keys(parsed) as (keyof Personal)[]) if (parsed[k]) out[k] = parsed[k];
  return out;
}

/**
 * Everything read from the resume, editable, before any of it is saved.
 * Each section can be left out, so a re-upload doesn't overwrite a section
 * you've already polished.
 */
export function ResumeReview({
  draft,
  current,
  onSaved,
  onCancel,
}: {
  draft: ResumeDraft;
  current: Profile;
  onSaved: () => void;
  onCancel: () => void;
}) {
  const parsed = draft.profile;
  const [personal, setPersonal] = useState(() => mergePersonal(current.personal, parsed.personal));
  const [links, setLinks] = useState(() => [
    ...current.links,
    ...parsed.links.filter((l) => !current.links.some((c) => c.url.toLowerCase() === l.url.toLowerCase())),
  ]);
  const [educations, setEducations] = useState(parsed.educations);
  const [experiences, setExperiences] = useState(parsed.experiences);
  const [projects, setProjects] = useState(parsed.projects);
  const [skills, setSkills] = useState(skillsToText(parsed.skills));

  // A section is used when the resume had something for it.
  const [use, setUse] = useState<Record<Key, boolean>>({
    personal: true,
    educations: parsed.educations.length > 0,
    experiences: parsed.experiences.length > 0,
    projects: parsed.projects.length > 0,
    skills: parsed.skills.length > 0,
  });
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function saveAll() {
    setSaving(true);
    setError(null);
    const steps: [Key, () => ReturnType<(typeof save)[keyof typeof save]>][] = [
      ["personal", () => save.personal(personal)],
      ["personal", () => save.links(links)],
      ["educations", () => save.educations(educations)],
      ["experiences", () => save.experiences(experiences)],
      ["projects", () => save.projects(projects)],
      ["skills", () => save.skills(textToSkills(skills))],
    ];
    for (const [key, run] of steps) {
      if (!use[key]) continue;
      const result = await run();
      if (!result.ok) {
        setError(`${label[key]}: ${result.message}`);
        setSaving(false);
        return;
      }
    }
    setSaving(false);
    onSaved();
  }

  const label: Record<Key, string> = {
    personal: "Personal details and links",
    educations: "Education",
    experiences: "Work experience",
    projects: "Projects",
    skills: "Skills",
  };
  const replaces = (k: Key, count: number) =>
    k !== "personal" && count > 0 ? `Replaces the ${count} saved ${count === 1 ? "entry" : "entries"}.` : null;
  const savedCount: Record<Key, number> = {
    personal: 0,
    educations: current.educations.length,
    experiences: current.experiences.length,
    projects: current.projects.length,
    skills: current.skills.length,
  };

  const section = (k: Key, body: React.ReactNode) => (
    <section className="space-y-4 rounded-3xl border border-border/70 bg-card p-5 shadow-soft sm:p-6">
      <div className="flex items-start justify-between gap-3">
        <div>
          <h2 className="text-lg font-semibold">{label[k]}</h2>
          {use[k] && replaces(k, savedCount[k]) && (
            <p className="text-xs text-[color-mix(in_oklch,var(--warning),black_35%)] dark:text-warning">{replaces(k, savedCount[k])}</p>
          )}
        </div>
        <label className="flex shrink-0 items-center gap-2 text-sm">
          <Checkbox checked={use[k]} onCheckedChange={(v) => setUse({ ...use, [k]: v === true })} />
          Save this
        </label>
      </div>
      {use[k] && body}
    </section>
  );

  return (
    <div className="grid gap-6 lg:grid-cols-[1fr_20rem]">
      <div className="space-y-6">
        <div className="space-y-1">
          <h2 className="text-xl font-semibold tracking-tight">Check what we read from {draft.filename}</h2>
          <p className="text-sm text-muted-foreground">
            Nothing is saved yet. Fix anything that landed in the wrong field, then save.
          </p>
        </div>
        {section(
          "personal",
          <div className="space-y-6">
            <PersonalEditor value={personal} onChange={setPersonal} />
            <LinksEditor value={links} onChange={setLinks} />
          </div>,
        )}
        {section("experiences", <ExperienceEditor value={experiences} onChange={setExperiences} />)}
        {section("educations", <EducationEditor value={educations} onChange={setEducations} />)}
        {section("projects", <ProjectEditor value={projects} onChange={setProjects} />)}
        {section("skills", <SkillsEditor value={skills} onChange={setSkills} />)}
      </div>

      <aside className="space-y-4 lg:sticky lg:top-6 lg:self-start">
        <div className="space-y-3 rounded-3xl border border-border/70 bg-card p-5 shadow-soft">
          {error && (
            <p role="alert" className="rounded-xl bg-destructive/10 px-3 py-2 text-sm text-destructive">
              {error}
            </p>
          )}
          <Button className="w-full" size="lg" disabled={saving} onClick={saveAll}>
            {saving ? "Saving…" : "Save to profile"}
          </Button>
          <Button variant="ghost" className="w-full" disabled={saving} onClick={onCancel}>
            Discard
          </Button>
        </div>
        {draft.unsorted.length > 0 && (
          <div className="space-y-3 rounded-3xl border border-border/70 bg-card p-5 shadow-soft">
            <div className="flex items-center gap-2">
              <Info className="size-4 text-muted-foreground" />
              <h3 className="text-sm font-semibold">Not placed</h3>
            </div>
            <p className="text-xs text-muted-foreground">
              Text from your resume that didn&apos;t fit a field. Copy anything you want into the right section.
            </p>
            <ul className="space-y-2">
              {draft.unsorted.map((u, i) => (
                <li key={i} className="rounded-xl bg-muted px-3 py-2 text-xs break-words select-all">
                  {u}
                </li>
              ))}
            </ul>
          </div>
        )}
      </aside>
    </div>
  );
}
