"use client";

import { ArrowRight, Check, Info, Undo2, Wand2 } from "lucide-react";
import type { ResumeChange, ResumeContent } from "@/api/client";
import { Button } from "@/components/ui/button";

const REASON: Record<ResumeChange["kind"], string> = {
  capitals: "Written in capitals in the file; changed to title case.",
  small_caps: "The font's small capitals came out with a lowercase \"i\"; fixed.",
  joined_word: "A word was split across two lines; joined back.",
  icon_link: "An icon stood for this account; read as its address.",
  link_label: "\"GitHub\" or a similar link label was part of the name; taken off.",
  degree_split: "One line held the degree, major and minor; split into fields.",
  class_of_may: "Only a year was given; read as May, when most degrees finish.",
  link_from_file: "The address was only in the file's link, not in its text.",
};

const SECTION: Record<string, string> = {
  header: "Header",
  educations: "Education",
  experiences: "Work experience",
  projects: "Project",
};

const FIELD: Record<string, string> = {
  title: "title",
  company: "company",
  school: "school",
  degree: "degree",
  field_of_study: "major",
  minor: "minor",
  end: "end date",
  start: "start date",
  name: "name",
  summary: "summary",
  url: "link",
  bullets: "bullet",
  links: "link",
};

/** "experiences.1.title" -> "Work experience 2 · title". */
export function fieldLabel(path: string) {
  const parts = path.split(".");
  const [section, index, field, sub] = parts;
  if (section === "header") {
    if (index === "links") return `Header · link ${Number(field) + 1}`;
    return `Header · ${FIELD[index] ?? index}`;
  }
  const where = `${SECTION[section] ?? section} ${Number(index) + 1}`;
  const what = field === "bullets" ? `bullet ${Number(sub) + 1}` : (FIELD[field] ?? field);
  return `${where} · ${what}`;
}

/**
 * What the parser changed from the file's text, to check one by one. Keep
 * drops the note; Undo puts the file's text back first.
 */
export function ChangesPanel({
  content,
  onKeep,
  onUndo,
  onKeepAll,
  onJump,
}: {
  content: ResumeContent;
  /** Brings a field into view. */
  onJump: (field: string) => void;
  onKeep: (i: number) => void;
  onUndo: (i: number) => void;
  onKeepAll: () => void;
}) {
  const changes = content.changes;

  if (changes.length === 0) {
    return (
      <div className="flex items-start gap-3 rounded-3xl border border-success/30 bg-success/10 p-5 text-sm">
        <Check className="mt-0.5 size-4 shrink-0 text-success" />
        Nothing left to check from the parser.
      </div>
    );
  }
  return (
    <section aria-labelledby="changes-title" className="space-y-3 rounded-3xl border border-border/70 bg-card p-5 shadow-soft">
      <div className="flex items-start gap-2">
        <Wand2 className="mt-0.5 size-4 shrink-0 text-primary" />
        <div className="min-w-0 flex-1">
          <h2 id="changes-title" className="text-sm font-semibold">
            {changes.length} {changes.length === 1 ? "thing" : "things"} the parser changed
          </h2>
          <p className="text-xs text-muted-foreground">
            Check each against your file. Fields with a <span className="inline-block size-1.5 rounded-full bg-warning align-middle" /> dot
            are on this list.
          </p>
        </div>
      </div>
      <ol className="space-y-2">
        {changes.map((c, i) => (
          <li key={`${c.field}-${c.kind}-${i}`} className="space-y-2 rounded-2xl bg-muted/60 p-3">
            <button
              type="button"
              onClick={() => onJump(c.field)}
              className="text-left text-xs font-semibold text-primary hover:underline"
            >
              {fieldLabel(c.field)}
            </button>
            <div className="flex flex-wrap items-center gap-1.5 text-xs break-all">
              {c.before ? (
                <span className="rounded-md bg-background px-1.5 py-0.5 text-muted-foreground line-through decoration-muted-foreground/50">
                  {c.before}
                </span>
              ) : (
                <span className="text-muted-foreground">(not in the text)</span>
              )}
              <ArrowRight className="size-3 shrink-0 text-muted-foreground" aria-label="became" />
              <span className="rounded-md bg-background px-1.5 py-0.5 font-medium">{c.after}</span>
            </div>
            <p className="flex items-start gap-1.5 text-xs text-muted-foreground">
              <Info className="mt-0.5 size-3 shrink-0" />
              {REASON[c.kind]}
            </p>
            <div className="flex gap-1.5">
              <Button size="xs" variant="flat" onClick={() => onKeep(i)}>
                <Check /> Keep
              </Button>
              {(c.undo ?? []).length > 0 && (
                <Button size="xs" variant="ghost" onClick={() => onUndo(i)}>
                  <Undo2 /> Undo
                </Button>
              )}
            </div>
          </li>
        ))}
      </ol>
      <Button variant="bordered" size="sm" className="w-full" onClick={onKeepAll}>
        Keep all
      </Button>
    </section>
  );
}
