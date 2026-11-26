"use client";

import { CircleAlert, CircleCheck, Loader2 } from "lucide-react";
import Link from "next/link";
import { useEffect, useId, useState } from "react";
import type { ApplyResult, Profile, ProfileSection, ResumeContent } from "@/api/client";
import { loadProfile, resumes as resumeApi } from "@/components/profile/save";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { cn } from "cn";
import { rowKind } from "@/lib/degrees";

const SECTIONS: { id: ProfileSection; label: string }[] = [
  { id: "personal", label: "Personal details" },
  { id: "education", label: "Education" },
  { id: "experience", label: "Work experience" },
  { id: "projects", label: "Projects" },
  { id: "skills", label: "Skills" },
];

const LABEL = Object.fromEntries(SECTIONS.map((s) => [s.id, s.label])) as Record<ProfileSection, string>;

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

/** How many entries the resume has for a section (empty rows don't count). */
function resumeCount(c: ResumeContent, s: ProfileSection) {
  const filled = (o: object) => Object.values(o).some((v) => (Array.isArray(v) ? v.length > 0 : !!v));
  switch (s) {
    case "personal":
      return c.header.name || c.header.email || c.header.phone || c.header.links.some((l) => l.url) ? 1 : 0;
    case "education":
      // The profile has one entry per degree row (at least one per school);
      // "Minor in …" and the like join the degree above them.
      return c.educations
        // Every entry has a row; an empty one doesn't make it filled.
        .filter((e) => filled({ ...e, degrees: e.degrees.filter((d) => filled(d)) }))
        .reduce(
          (n, e) =>
            n + Math.max(1, e.degrees.filter((d) => (d.degree || d.field) && rowKind(d.degree) === "degree").length),
          0,
        );
    case "experience":
      return c.experiences.filter(filled).length;
    case "projects":
      return c.projects.filter(filled).length;
    case "skills":
      return c.skills.length;
  }
}

function savedCount(p: Profile, s: ProfileSection) {
  switch (s) {
    case "personal":
      return 0;
    case "education":
      return p.educations.length;
    case "experience":
      return p.experiences.length;
    case "projects":
      return p.projects.length;
    case "skills":
      return p.skills.length;
  }
}

function effect(s: ProfileSection, fromResume: number, saved: number) {
  if (s === "personal") return "Fills your name, email, phone, location and links. Keeps everything else.";
  const unit = s === "skills" ? ["skill", "skills"] : ["entry", "entries"];
  const from = plural(fromResume, unit[0], unit[1]);
  return saved > 0 ? `Replaces your ${plural(saved, "saved " + unit[0], "saved " + unit[1])} with ${from} from this resume.` : `Adds ${from} from this resume.`;
}

/**
 * "Update profile": copies the chosen sections of a resume (as edited on
 * its page) into the profile. The only way a resume changes the profile,
 * besides the first resume filling an empty one.
 */
export function UpdateProfileDialog({
  resumeId,
  name,
  open,
  onOpenChange,
  content: given,
  onShowField,
}: {
  resumeId: string;
  name: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** The resume's content when it's already loaded (the edit page). */
  content?: ResumeContent;
  /** On the edit page: close and bring this field into view. */
  onShowField?: (field: string) => void;
}) {
  const statusId = useId();
  const [content, setContent] = useState<ResumeContent | null>(given ?? null);
  const [profile, setProfile] = useState<Profile | null>(null);
  const [chosen, setChosen] = useState<Set<ProfileSection>>(new Set());
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<ApplyResult | null>(null);

  // Fresh counts each time it opens; sections with content start checked.
  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    (async () => {
      setResult(null);
      setError(null);
      const [c, p] = await Promise.all([
        given ? Promise.resolve(given) : resumeApi.detail(resumeId).then((r) => (r.ok ? r.value.content : null)),
        loadProfile(),
      ]);
      if (cancelled) return;
      if (!c || !p) {
        setError("Couldn't load the resume or your profile. Is the API running?");
        return;
      }
      setContent(c);
      setProfile(p);
      setChosen(new Set(SECTIONS.map((s) => s.id).filter((s) => resumeCount(c, s) > 0)));
    })();
    return () => {
      cancelled = true;
    };
  }, [open, resumeId, given]);

  async function apply() {
    setBusy(true);
    setError(null);
    const r = await resumeApi.apply(resumeId, [...chosen]);
    setBusy(false);
    if (r.ok) setResult(r.value);
    else setError(r.message);
  }

  const ready = content && profile;

  return (
    <Dialog open={open} onOpenChange={(o) => !busy && onOpenChange(o)}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>Update profile from {name}</DialogTitle>
          <DialogDescription>
            {result
              ? result.failed.length
                ? "Some sections were updated. Fix the rest on this resume, then try them again."
                : "Done. Autofill and tailoring use your updated profile."
              : "Pick the sections to copy from this resume, as you've edited it. Only those change. Your profile is what autofill and tailoring use."}
          </DialogDescription>
        </DialogHeader>

        {!ready && !error && (
          <p className="flex items-center gap-2 py-6 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" /> Loading…
          </p>
        )}

        {ready && !result && (
          <fieldset className="space-y-2">
            <legend className="sr-only">Sections to update</legend>
            {SECTIONS.map((s, i) => {
              const n = resumeCount(content, s.id);
              const id = `${statusId}-${s.id}`;
              return (
                <label
                  key={s.id}
                  htmlFor={id}
                  className={cn(
                    "flex cursor-pointer items-start gap-3 rounded-2xl bg-muted/60 px-3 py-2.5",
                    n === 0 && "cursor-not-allowed opacity-60",
                  )}
                >
                  <Checkbox
                    id={id}
                    autoFocus={i === 0}
                    disabled={n === 0 || busy}
                    checked={chosen.has(s.id)}
                    onCheckedChange={(v) => {
                      const next = new Set(chosen);
                      if (v === true) next.add(s.id);
                      else next.delete(s.id);
                      setChosen(next);
                    }}
                    className="mt-0.5"
                  />
                  <span className="space-y-0.5">
                    <span className="block text-sm font-medium">{s.label}</span>
                    <span className="block text-xs text-muted-foreground">
                      {n === 0 ? "Nothing on this resume." : effect(s.id, n, savedCount(profile, s.id))}
                    </span>
                  </span>
                </label>
              );
            })}
          </fieldset>
        )}

        {result && (
          <div id={statusId} role="status" className="space-y-3 text-sm">
            {result.updated.length > 0 && (
              <p className="flex items-start gap-2 rounded-xl bg-success/12 px-3 py-2 text-success">
                <CircleCheck className="mt-0.5 size-4 shrink-0" />
                Updated: {result.updated.map((s) => LABEL[s]).join(", ")}.
              </p>
            )}
            {result.failed.map((f) => (
              <div key={f.section} className="space-y-1 rounded-xl bg-destructive/10 px-3 py-2 text-destructive">
                <p className="flex items-start gap-2">
                  <CircleAlert className="mt-0.5 size-4 shrink-0" />
                  <span>
                    <span className="font-medium">{LABEL[f.section]}: not updated.</span> {f.message} Fix it on this
                    resume and try again.
                  </span>
                </p>
                {f.field &&
                  (onShowField ? (
                    <Button
                      variant="light"
                      size="xs"
                      className="ml-6 text-destructive"
                      onClick={() => {
                        onOpenChange(false);
                        onShowField(f.field!);
                      }}
                    >
                      Show me
                    </Button>
                  ) : (
                    <Button asChild variant="light" size="xs" className="ml-6 text-destructive">
                      <Link href={`/resume/edit/${resumeId}`}>Open the resume</Link>
                    </Button>
                  ))}
              </div>
            ))}
          </div>
        )}

        {error && (
          <p role="alert" className="rounded-xl bg-destructive/10 px-3 py-2 text-sm text-destructive">
            {error}
          </p>
        )}

        <DialogFooter>
          {result ? (
            <>
              <Button variant="ghost" onClick={() => onOpenChange(false)}>
                Done
              </Button>
              <Button asChild>
                <Link href="/profile">Go to profile</Link>
              </Button>
            </>
          ) : (
            <>
              <Button variant="ghost" onClick={() => onOpenChange(false)} disabled={busy}>
                Not now
              </Button>
              <Button onClick={apply} disabled={!ready || busy || chosen.size === 0}>
                {busy && <Loader2 className="animate-spin" />}
                {busy ? "Updating…" : "Update profile"}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
