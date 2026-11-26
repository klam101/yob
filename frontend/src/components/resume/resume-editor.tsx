"use client";

import {
  ArrowDown,
  ArrowUp,
  CaseSensitive,
  Check,
  CloudAlert,
  Download,
  Globe,
  GripVertical,
  Link2,
  ListTree,
  Loader2,
  Mail,
  MapPin,
  PencilLine,
  Phone,
  Plus,
  Star,
  Trash2,
  UserRoundPen,
  Wand2,
  X,
} from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  ResumeChange,
  ResumeContent,
  ResumeDegree,
  ResumeDetail,
  ResumeEducation,
  ResumeExperience,
  ResumeInfo,
  SectionKind,
} from "@/api/client";
import { TagInput } from "@/components/filters/tag-input";
import { emptyProject } from "@/components/profile/editors";
import { BulletList, CheckField, FieldScope, IconButton } from "@/components/profile/fields";
import { resumes as resumeApi } from "@/components/profile/save";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Sheet, SheetBody, SheetClose, SheetContent, SheetDescription, SheetHeader, SheetTitle } from "@/components/ui/sheet";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { isAllCaps, tidyCaps } from "@/lib/caps";
import { DEGREE_ROWS, FIELDS_OF_STUDY } from "@/lib/degrees";
import { cn } from "cn";
import { ChangesPanel } from "./changes-panel";
import { ChangedTag, type Changes, ChangesContext, InlineMonth, InlineText } from "./inline";
import { DeleteDialog, EditDetailsDialog } from "./resume-manager";
import { UpdateProfileDialog } from "./update-profile-dialog";

/** Sets a value at a path like "experiences.1.title" without mutating. */
function setPath<T>(obj: T, path: string, value: unknown): T {
  const [head, ...rest] = path.split(".");
  const key = /^\d+$/.test(head) ? Number(head) : head;
  const copy = (Array.isArray(obj) ? [...obj] : { ...(obj as object) }) as Record<string | number, unknown>;
  if (rest.length) {
    if (copy[key] === undefined || copy[key] === null) return obj;
    copy[key] = setPath(copy[key], rest.join("."), value);
  } else {
    copy[key] = value;
  }
  return copy as T;
}

/** Whether `path` ("educations.0.degrees.1.field") names something in `c`. */
function resolves(c: unknown, path: string): boolean {
  let o: unknown = c;
  for (const k of path.split(".")) {
    if (o === null || typeof o !== "object" || !(k in o)) return false;
    o = (o as Record<string, unknown>)[k];
  }
  return true;
}

/**
 * Keeps the parser's change list pointing at the right fields when a list
 * changes: rows inserted (`by` > 0) or one removed (`by` = -1) at `at` in
 * the list at `list` ("experiences", "experiences.0.bullets"). A removed
 * row's changes go with it.
 */
export function shiftChanges(changes: ResumeChange[], list: string, at: number, by: number): ResumeChange[] {
  const prefix = `${list}.`;
  const move = (field: string): string | null => {
    if (!field.startsWith(prefix)) return field;
    const [n, ...rest] = field.slice(prefix.length).split(".");
    if (!/^\d+$/.test(n)) return field;
    const i = Number(n);
    if (by < 0 && i === at) return null;
    const j = by < 0 ? (i > at ? i - 1 : i) : i >= at ? i + by : i;
    return [list, j, ...rest].join(".");
  };
  return changes.flatMap((c) => {
    const field = move(c.field);
    if (field === null) return [];
    const undo = (c.undo ?? []).flatMap((u) => {
      const f = move(u.field);
      return f === null ? [] : [{ ...u, field: f }];
    });
    return [{ ...c, field, undo }];
  });
}

/** Every one-line text field that could be in capitals, by path. */
function capsFields(c: ResumeContent): [string, string][] {
  const out: [string, string][] = [
    ["header.name", c.header.name],
    ["header.subtitle", c.header.subtitle],
    ["header.location", c.header.location],
  ];
  c.educations.forEach((e, i) => {
    (["school", "location", "achievement"] as const).forEach((k) => out.push([`educations.${i}.${k}`, e[k]]));
    e.degrees.forEach((d, j) => {
      const at = `educations.${i}.degrees.${j}`;
      out.push([`${at}.degree`, d.degree], [`${at}.field`, d.field]);
    });
  });
  c.experiences.forEach((x, i) =>
    (["company", "title", "location"] as const).forEach((k) => out.push([`experiences.${i}.${k}`, x[k]])),
  );
  c.projects.forEach((p, i) =>
    (["name", "summary", "role"] as const).forEach((k) => out.push([`projects.${i}.${k}`, p[k]])),
  );
  c.skills.forEach((s, i) => out.push([`skills.${i}.category`, s.category]));
  return out.filter(([, v]) => isAllCaps(v));
}

const SECTION_TITLE: Record<SectionKind, string> = {
  education: "Education",
  experience: "Work experience",
  projects: "Projects",
  skills: "Skills",
};

const emptyDegree = (): ResumeDegree => ({ degree: "", field: "" });

const emptyEducation = (): ResumeEducation => ({
  school: "",
  degrees: [emptyDegree()],
  gpa: "",
  location: "",
  start: null,
  end: null,
  achievement: "",
  coursework: [],
  bullets: [],
});

const emptyExperience = (): ResumeExperience => ({
  company: "",
  title: "",
  location: "",
  employment_type: "",
  start: null,
  end: null,
  summary: "",
  bullets: [],
});

type Status = "saved" | "dirty" | "saving" | "error";
type SaveError = { message: string; field?: string | null };

/** Brings a field into view and focuses its first input. */
function showField(path: string) {
  // A list path ("experiences") points at its first entry.
  const el =
    document.querySelector<HTMLElement>(`[data-field="${CSS.escape(path)}"]`) ??
    document.querySelector<HTMLElement>(`[data-field^="${CSS.escape(path)}."]`);
  el?.scrollIntoView({ block: "center", behavior: "smooth" });
  el?.querySelector<HTMLElement>("input, textarea")?.focus({ preventScroll: true });
}

/**
 * The resume edit page: a resume's own content, edited in place and saved
 * on its own. Editing it never changes the profile; "Update profile" copies
 * chosen sections over.
 */
export function ResumeEditor({ initial, openUpdate = false }: { initial: ResumeDetail; openUpdate?: boolean }) {
  const router = useRouter();
  const id = initial.info.id;
  const [info, setInfo] = useState<ResumeInfo>(initial.info);
  const [content, setContent] = useState<ResumeContent>(initial.content);
  const [status, setStatus] = useState<Status>("saved");
  const [error, setError] = useState<SaveError | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [dialog, setDialog] = useState<"edit" | "delete" | "update" | null>(openUpdate ? "update" : null);
  const [checking, setChecking] = useState(false);

  // After an upload lands here with ?update=1, drop it from the address so
  // a reload doesn't open the dialog again.
  useEffect(() => {
    if (openUpdate) router.replace(`/resume/edit/${id}`, { scroll: false });
  }, [openUpdate, id, router]);

  // Autosave: the latest content, how many edits happened, and how many
  // the last successful save included.
  const latest = useRef(content);
  const edits = useRef(0);
  const savedEdits = useRef(0);
  const inFlight = useRef(false);
  const again = useRef(false);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  // Lets a save schedule the next one (it can't name itself while declared).
  const flushLater = useRef<() => void>(() => {});

  const flush = useCallback(async () => {
    if (timer.current) clearTimeout(timer.current);
    if (inFlight.current) {
      again.current = true;
      return;
    }
    if (edits.current === savedEdits.current) return;
    inFlight.current = true;
    const upTo = edits.current;
    setStatus("saving");
    const r = await resumeApi.saveContent(id, latest.current);
    inFlight.current = false;
    if (r.ok) {
      savedEdits.current = upTo;
      setInfo(r.value.info);
      setError(null);
      setStatus(edits.current === upTo ? "saved" : "dirty");
    } else {
      setStatus("error");
      setError({ message: r.message, field: r.field });
    }
    if (again.current || (r.ok && edits.current !== upTo)) {
      again.current = false;
      timer.current = setTimeout(flushLater.current, 800);
    }
  }, [id]);
  useEffect(() => {
    flushLater.current = () => void flush();
  }, [flush]);

  const update = useCallback(
    (fn: (c: ResumeContent) => ResumeContent) => {
      setContent((c) => {
        const next = fn(c);
        latest.current = next;
        return next;
      });
      edits.current += 1;
      setStatus("dirty");
      if (timer.current) clearTimeout(timer.current);
      timer.current = setTimeout(flush, 800);
    },
    [flush],
  );
  const set = useCallback((path: string, value: unknown) => update((c) => setPath(c, path, value)), [update]);
  /** Rows inserted or removed in a list: move the change list along. */
  const shift = useCallback(
    (list: string, at: number, by: number) => update((c) => ({ ...c, changes: shiftChanges(c.changes, list, at, by) })),
    [update],
  );
  /** Removes row `i` of a top-level list and its changes. */
  const removeRow = useCallback(
    (list: "educations" | "experiences" | "projects", i: number) =>
      update((c) => ({
        ...c,
        [list]: (c[list] as unknown[]).filter((_, j) => j !== i),
        changes: shiftChanges(c.changes, list, i, -1),
      })),
    [update],
  );

  // Leaving with unsaved edits: send them with a request that outlives the page.
  useEffect(() => {
    const send = () => {
      if (edits.current === savedEdits.current) return;
      savedEdits.current = edits.current;
      void fetch(`/api/profile/resumes/${id}/content`, {
        method: "PUT",
        keepalive: true,
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(latest.current),
      });
    };
    window.addEventListener("pagehide", send);
    return () => {
      window.removeEventListener("pagehide", send);
      send();
    };
  }, [id]);

  const keep = useCallback(
    (i: number) => update((c) => ({ ...c, changes: c.changes.filter((_, j) => j !== i) })),
    [update],
  );
  const capsCount = capsFields(content).length;
  const fieldError = error?.field ? { field: error.field, message: error.message } : null;
  const toCheck = content.changes.length + content.unplaced.length;

  function fixAllCaps() {
    const fields = capsFields(content);
    update((c) => fields.reduce((acc, [path, v]) => setPath(acc, path, tidyCaps(v)), c));
    setNotice(`Fixed capitals in ${fields.length} ${fields.length === 1 ? "field" : "fields"}.`);
  }

  function undo(i: number) {
    update((c) => {
      const change = c.changes[i];
      const rest = { ...c, changes: c.changes.filter((_, j) => j !== i) };
      // A header link whose address came only from the file: undoing it
      // removes the link, not just its address.
      const link = /^header\.links\.(\d+)\.url$/.exec(change.field);
      if (change.kind === "link_from_file" && link) {
        const at = Number(link[1]);
        return {
          ...rest,
          header: { ...rest.header, links: rest.header.links.filter((_, j) => j !== at) },
          changes: shiftChanges(rest.changes, "header.links", at, -1),
        };
      }
      const next = (change.undo ?? []).reduce((acc, u) => setPath(acc, u.field, u.value), rest);
      // An undo can replace a whole list (the degree rows); changes on rows
      // that are gone go too.
      return { ...next, changes: next.changes.filter((ch) => resolves(next, ch.field)) };
    });
  }

  const byField = useMemo(() => {
    const m: Changes["byField"] = new Map();
    content.changes.forEach((change, index) => m.set(change.field, [...(m.get(change.field) ?? []), { change, index }]));
    return m;
  }, [content.changes]);
  const changesContext: Changes = { byField, keep, undo };

  const move = (i: number, by: number) =>
    update((c) => {
      const next = [...c.sections];
      const [s] = next.splice(i, 1);
      next.splice(i + by, 0, s);
      return { ...c, sections: next };
    });

  const edit: Edit = { content, set, update, shift, removeRow };

  return (
    <ChangesContext.Provider value={changesContext}>
      <div className="space-y-4 pt-6">
        {/* Top bar */}
        <div className="flex flex-wrap items-center gap-2">
          <Tooltip>
            <TooltipTrigger asChild>
              <Button asChild variant="bordered" size="icon-sm" aria-label="Back to resumes">
                <Link href="/resume" onClick={() => void flush()}>
                  <X />
                </Link>
              </Button>
            </TooltipTrigger>
            <TooltipContent>Back to resumes</TooltipContent>
          </Tooltip>
          <div className="flex min-w-0 items-center gap-2 rounded-xl bg-muted px-3 py-1.5">
            {info.is_primary && (
              <Tooltip>
                <TooltipTrigger asChild>
                  <Star className="size-4 shrink-0 fill-current text-success" aria-label="Primary resume" />
                </TooltipTrigger>
                <TooltipContent>Your primary resume</TooltipContent>
              </Tooltip>
            )}
            <h1 className="truncate text-sm font-semibold">{info.name}</h1>
          </div>
          <SaveStatus status={status} onRetry={() => void flush()} />
          <div className="ml-auto flex flex-wrap items-center gap-2">
            <Button
              size="sm"
              onClick={async () => {
                await flush();
                setDialog("update");
              }}
            >
              <UserRoundPen /> Update profile
            </Button>
            <TopButton label="Edit info" icon={PencilLine} onClick={() => setDialog("edit")} />
            <TopButton label="Download" icon={Download} href={`/api/profile/resumes/${id}/file`} />
            <TopButton label="Delete" icon={Trash2} onClick={() => setDialog("delete")} />
          </div>
        </div>

        {(notice || (error && !error.field)) && (
          <p
            role="status"
            className={cn(
              "rounded-xl px-3 py-2 text-sm",
              error && !error.field ? "bg-destructive/10 text-destructive" : "bg-primary/10 text-primary",
            )}
          >
            {error && !error.field ? error.message : notice}
          </p>
        )}

        <FieldScope path="" error={fieldError}>
          <article aria-label="Resume content" className="overflow-hidden rounded-3xl border border-border/70 bg-card shadow-soft">
            {/* What the parser changed, where Jobright has its "Analyze" bar. */}
            <div className="flex flex-wrap items-center gap-3 border-b border-border/60 px-5 py-4 sm:px-8">
              <span className="flex size-10 shrink-0 items-center justify-center rounded-2xl bg-primary/12 text-primary">
                {toCheck ? <Wand2 className="size-5" /> : <Check className="size-5" />}
              </span>
              <p className="min-w-0 flex-1 text-sm">
                {toCheck ? (
                  <>
                    <span className="font-semibold">
                      {content.changes.length} {content.changes.length === 1 ? "thing" : "things"} the parser changed
                    </span>
                    {content.unplaced.length > 0 && ` and ${content.unplaced.length} not placed`}. Check them against
                    your file.
                  </>
                ) : (
                  "Nothing left to check from the parser."
                )}
              </p>
              {capsCount > 0 && (
                <Button variant="bordered" size="sm" onClick={fixAllCaps}>
                  <CaseSensitive /> Fix capitals ({capsCount})
                </Button>
              )}
              {toCheck > 0 && (
                <Button variant="flat" size="sm" onClick={() => setChecking(true)}>
                  Check them
                </Button>
              )}
            </div>

            <div className="space-y-8 p-5 sm:p-8">
              <Header {...edit} />
              <InlineText
                name="summary"
                label="Summary"
                placeholder="Add a summary…"
                multiline
                value={content.summary}
                onChange={(v) => set("summary", v)}
                className="text-sm"
              />
              {content.sections.map((kind, i) => (
                <section
                  key={kind}
                  id={`sec-${kind}`}
                  aria-labelledby={`sec-${kind}-title`}
                  className="group/section scroll-mt-6 space-y-4 border-t border-border/60 pt-6"
                >
                  <div className="flex items-center gap-1">
                    <GripVertical className="-ml-1 size-4 text-muted-foreground/60" aria-hidden />
                    <h2 id={`sec-${kind}-title`} className="mr-auto text-xl font-bold tracking-tight">
                      {SECTION_TITLE[kind]}
                    </h2>
                    <div className="flex sm:opacity-0 sm:transition-opacity sm:group-focus-within/section:opacity-100 sm:group-hover/section:opacity-100">
                      <IconButton label={`Move ${SECTION_TITLE[kind]} up`} disabled={i === 0} onClick={() => move(i, -1)}>
                        <ArrowUp />
                      </IconButton>
                      <IconButton
                        label={`Move ${SECTION_TITLE[kind]} down`}
                        disabled={i === content.sections.length - 1}
                        onClick={() => move(i, 1)}
                      >
                        <ArrowDown />
                      </IconButton>
                    </div>
                  </div>
                  {kind === "education" && <Educations {...edit} />}
                  {kind === "experience" && <Experiences {...edit} />}
                  {kind === "projects" && <Projects {...edit} />}
                  {kind === "skills" && <Skills {...edit} />}
                </section>
              ))}
            </div>
          </article>
        </FieldScope>
      </div>

      {/* Jump to a section, like Jobright's floating outline button. */}
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button
            size="icon-lg"
            shape="pill"
            aria-label="Jump to a section"
            title="Jump to a section"
            className="fixed right-5 bottom-20 z-30 shadow-raised md:right-8 md:bottom-8"
          >
            <ListTree />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" side="top" className="w-48">
          {content.sections.map((kind) => (
            <DropdownMenuItem
              key={kind}
              onSelect={() => document.getElementById(`sec-${kind}`)?.scrollIntoView({ behavior: "smooth" })}
            >
              {SECTION_TITLE[kind]}
            </DropdownMenuItem>
          ))}
        </DropdownMenuContent>
      </DropdownMenu>

      <Sheet open={checking} onOpenChange={setChecking}>
        <SheetContent className="sm:max-w-md">
          <SheetHeader>
            <SheetTitle>Check what we read</SheetTitle>
            <SheetClose asChild>
              <Button variant="ghost" size="icon-sm" aria-label="Close">
                <X />
              </Button>
            </SheetClose>
          </SheetHeader>
          <SheetDescription className="sr-only">What the parser changed, and text it couldn&apos;t place.</SheetDescription>
          <SheetBody className="space-y-4">
            <ChangesPanel
              content={content}
              onJump={(field) => {
                setChecking(false);
                setTimeout(() => showField(field), 250);
              }}
              onKeep={keep}
              onUndo={undo}
              onKeepAll={() => update((c) => ({ ...c, changes: [] }))}
            />
            {content.unplaced.length > 0 && (
              <section aria-labelledby="unplaced-title" className="space-y-3 rounded-3xl border border-border/70 bg-card p-5">
                <h2 id="unplaced-title" className="text-sm font-semibold">
                  Not placed
                </h2>
                <p className="text-xs text-muted-foreground">
                  Text from your file that didn&apos;t fit a field. Copy it where it belongs, then dismiss it.
                </p>
                <ul className="space-y-2">
                  {content.unplaced.map((u, i) => (
                    <li key={i} className="flex items-start gap-1 rounded-xl bg-muted px-3 py-2 text-xs break-words">
                      <span className="flex-1 select-all">{u}</span>
                      <IconButton
                        label="Dismiss"
                        onClick={() => update((c) => ({ ...c, unplaced: c.unplaced.filter((_, j) => j !== i) }))}
                      >
                        <X />
                      </IconButton>
                    </li>
                  ))}
                </ul>
              </section>
            )}
          </SheetBody>
        </SheetContent>
      </Sheet>

      <UpdateProfileDialog
        resumeId={id}
        name={info.name}
        open={dialog === "update"}
        onOpenChange={(o) => setDialog(o ? "update" : null)}
        content={dialog === "update" ? content : undefined}
        onShowField={(field) => setTimeout(() => showField(field), 250)}
      />
      {dialog === "edit" && (
        <EditDetailsDialog
          resume={info}
          onClose={() => setDialog(null)}
          onSaved={(updated) => {
            setDialog(null);
            setInfo(updated);
          }}
          onReparsed={(fresh) => {
            setDialog(null);
            latest.current = fresh.content;
            edits.current = savedEdits.current;
            setContent(fresh.content);
            setInfo(fresh.info);
            setStatus("saved");
            setNotice("Read the file again. Your edits on this page were replaced.");
          }}
        />
      )}
      {dialog === "delete" && (
        <DeleteDialog resume={info} onClose={() => setDialog(null)} onDeleted={() => router.push("/resume")} />
      )}
    </ChangesContext.Provider>
  );
}

/** A top-bar button whose label folds into a tooltip on phones. */
function TopButton({
  label,
  icon: Icon,
  onClick,
  href,
}: {
  label: string;
  icon: React.ComponentType;
  onClick?: () => void;
  href?: string;
}) {
  const inner = (
    <>
      <Icon />
      <span className="sr-only sm:not-sr-only">{label}</span>
    </>
  );
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        {href ? (
          <Button asChild variant="bordered" size="sm" className="max-sm:size-8 max-sm:px-0">
            <a href={href} download>
              {inner}
            </a>
          </Button>
        ) : (
          <Button variant="bordered" size="sm" onClick={onClick} className="max-sm:size-8 max-sm:px-0">
            {inner}
          </Button>
        )}
      </TooltipTrigger>
      <TooltipContent className="sm:hidden">{label}</TooltipContent>
    </Tooltip>
  );
}

function SaveStatus({ status, onRetry }: { status: Status; onRetry: () => void }) {
  return (
    <span className="flex items-center gap-1 text-xs text-muted-foreground" role="status" aria-live="polite">
      {status === "saved" && (
        <>
          <Check className="size-3.5 text-success" /> Saved
        </>
      )}
      {status === "dirty" && "Unsaved changes"}
      {status === "saving" && (
        <>
          <Loader2 className="size-3.5 animate-spin" /> Saving…
        </>
      )}
      {status === "error" && (
        <button type="button" onClick={onRetry} className="flex items-center gap-1 text-destructive hover:underline">
          <CloudAlert className="size-3.5" /> Not saved. Fix the marked field, or retry
        </button>
      )}
    </span>
  );
}

type Edit = {
  content: ResumeContent;
  set: (path: string, value: unknown) => void;
  update: (fn: (c: ResumeContent) => ResumeContent) => void;
  shift: (list: string, at: number, by: number) => void;
  removeRow: (list: "educations" | "experiences" | "projects", i: number) => void;
};

function linkIcon(url: string) {
  const u = url.toLowerCase();
  if (u.includes("linkedin.com") || u.includes("github.com")) return Link2;
  return Globe;
}

function Header({ content, set, update }: Edit) {
  const h = content.header;
  const [showTags, setShowTags] = useState(h.tags.length > 0);
  return (
    <FieldScope path="header">
      <header className="space-y-3">
        <InlineText
          name="name"
          label="Name"
          placeholder="Your name"
          value={h.name}
          onChange={(v) => set("header.name", v)}
          className="text-3xl font-bold tracking-tight"
        />
        <InlineText
          name="subtitle"
          label="Subtitle"
          placeholder="Add a subtitle, like QA engineer…"
          value={h.subtitle}
          onChange={(v) => set("header.subtitle", v)}
          className="text-sm"
        />
        <div className="grid gap-2 text-sm sm:grid-cols-3">
          <InlineText name="email" label="Email" type="email" icon={Mail} value={h.email} onChange={(v) => set("header.email", v)} />
          <InlineText name="phone" label="Phone" type="tel" icon={Phone} value={h.phone} onChange={(v) => set("header.phone", v)} />
          <InlineText
            name="location"
            label="Location"
            placeholder="Location…"
            icon={MapPin}
            source="location"
            value={h.location}
            onChange={(v) => set("header.location", v)}
          />
        </div>
        {showTags ? (
          <TagInput
            label="Tags"
            placeholder="Add tags, like Remote or Open to relocate…"
            value={h.tags}
            onChange={(v) => set("header.tags", v)}
          />
        ) : (
          <Button variant="bordered" size="sm" onClick={() => setShowTags(true)}>
            <Plus /> Tags
          </Button>
        )}
        <div className="grid gap-2 sm:grid-cols-3">
          {h.links.map((l, i) => {
            const Icon = linkIcon(l.url);
            return (
              <FieldScope key={i} path={`links.${i}`}>
                <div className="group relative space-y-1 rounded-2xl bg-muted/60 p-2 text-sm">
                  <div className="flex items-center gap-1 pr-8">
                    <Icon className="ml-1 size-4 shrink-0 text-muted-foreground" />
                    <InlineText
                      name="label"
                      label={`Link ${i + 1} label`}
                      placeholder="Label"
                      value={l.label}
                      onChange={(v) => set(`header.links.${i}.label`, v)}
                      className="font-medium"
                    />
                    <ChangedTag path={`header.links.${i}.url`} />
                  </div>
                  <InlineText
                    tag={false}
                    name="url"
                    label={`Link ${i + 1} address`}
                    placeholder="https://…"
                    value={l.url}
                    onChange={(v) => set(`header.links.${i}.url`, v)}
                    className="text-xs"
                  />
                  <div className="absolute top-1.5 right-1.5 sm:opacity-0 sm:transition-opacity sm:group-focus-within:opacity-100 sm:group-hover:opacity-100">
                    <IconButton
                      label={`Remove link ${i + 1}`}
                      onClick={() =>
                        update((c) => ({
                          ...c,
                          header: { ...c.header, links: c.header.links.filter((_, j) => j !== i) },
                          changes: shiftChanges(c.changes, "header.links", i, -1),
                        }))
                      }
                    >
                      <X />
                    </IconButton>
                  </div>
                </div>
              </FieldScope>
            );
          })}
        </div>
        <Button variant="bordered" size="sm" onClick={() => set("header.links", [...h.links, { label: "", url: "" }])}>
          <Plus /> Link
        </Button>
      </header>
    </FieldScope>
  );
}

/** An entry with its delete button, shown on hover (always on phones). */
function EntryShell({
  label,
  onDelete,
  children,
}: {
  label: string;
  onDelete: () => void;
  children: React.ReactNode;
}) {
  return (
    <div className="group relative space-y-1.5 pr-10">
      <div className="absolute top-0 right-0 sm:opacity-0 sm:transition-opacity sm:group-focus-within:opacity-100 sm:group-hover:opacity-100">
        <IconButton label={`Delete ${label}`} onClick={onDelete}>
          <Trash2 />
        </IconButton>
      </div>
      {children}
    </div>
  );
}

function Dates({
  start,
  end,
  onStart,
  onEnd,
  current,
}: {
  start: string | null;
  end: string | null;
  onStart: (v: string | null) => void;
  onEnd: (v: string | null) => void;
  /** Shows "Current" (experiences). */
  current?: { checked: boolean; onChange: (v: boolean) => void };
}) {
  return (
    <div className="flex flex-wrap items-center gap-x-1 gap-y-1">
      <InlineMonth name="start" label="Start date" value={start} onChange={onStart} />
      <span className="text-muted-foreground" aria-hidden>
        →
      </span>
      {current?.checked ? (
        <span className="px-2 text-sm text-muted-foreground">Present</span>
      ) : (
        <InlineMonth name="end" label="End date" value={end} onChange={onEnd} />
      )}
      {current && (
        <div className="ml-2">
          <CheckField label="Current" checked={current.checked} onChange={current.onChange} />
        </div>
      )}
    </div>
  );
}

function AddButton({ label, onClick }: { label: string; onClick: () => void }) {
  return (
    <Button variant="bordered" size="sm" onClick={onClick}>
      <Plus /> {label}
    </Button>
  );
}

/**
 * An education entry's "X in Y" rows: "Bachelor of Science" in "Computer
 * Science", "Minor" in "Business". Both boxes suggest as you type and take
 * any text. Each "+" adds one row.
 */
function Degrees({
  degrees,
  at,
  set,
  update,
}: {
  degrees: ResumeDegree[];
  at: string;
  set: Edit["set"];
  update: Edit["update"];
}) {
  // The row just added, so its first box gets the cursor.
  const [added, setAdded] = useState<number | null>(null);
  return (
    <FieldScope path="degrees">
      <div className="space-y-1.5 text-sm">
        {degrees.map((d, j) => {
          const path = `${at}.degrees.${j}`;
          return (
            <FieldScope key={j} path={String(j)}>
              <div className="group/degree flex flex-wrap items-start gap-1 sm:flex-nowrap">
                <div className="w-full sm:w-60 sm:shrink-0">
                  <InlineText
                    tag={false}
                    name="degree"
                    label={`Row ${j + 1}: degree, major or minor`}
                    placeholder="Degree, major or minor"
                    suggestions={DEGREE_ROWS}
                    autoFocus={added === j}
                    value={d.degree}
                    onChange={(v) => set(`${path}.degree`, v)}
                    className="font-medium"
                  />
                </div>
                <span className="flex h-7.5 items-center px-1 text-muted-foreground">in</span>
                <div className="min-w-0 flex-1">
                  <InlineText
                    tag={false}
                    name="field"
                    label={`Row ${j + 1}: field of study`}
                    placeholder="Field of study"
                    suggestions={FIELDS_OF_STUDY}
                    value={d.field}
                    onChange={(v) => set(`${path}.field`, v)}
                  />
                </div>
                {/* The row's tags at its end, so they never cover a box. */}
                <ChangedTag path={`${path}.degree`} className="mt-0.75" />
                <ChangedTag path={`${path}.field`} className="mt-0.75" />
                <div className="sm:opacity-0 sm:transition-opacity sm:group-focus-within/degree:opacity-100 sm:group-hover/degree:opacity-100">
                  <IconButton
                    label={`Remove row ${j + 1}`}
                    onClick={() =>
                      update((c) => {
                        const list = `${at}.degrees`;
                        const next = setPath(
                          c,
                          list,
                          degrees.filter((_, k) => k !== j),
                        );
                        return { ...next, changes: shiftChanges(next.changes, list, j, -1) };
                      })
                    }
                  >
                    <Trash2 />
                  </IconButton>
                </div>
              </div>
            </FieldScope>
          );
        })}
        <Button
          variant="light"
          size="xs"
          onClick={() => {
            setAdded(degrees.length);
            set(`${at}.degrees`, [...degrees, emptyDegree()]);
          }}
        >
          <Plus /> Degree, major or minor
        </Button>
      </div>
    </FieldScope>
  );
}

function Educations({ content, set, update, shift, removeRow }: Edit) {
  return (
    <FieldScope path="educations">
      <div className="space-y-8">
        {content.educations.map((e, i) => {
          const at = `educations.${i}`;
          return (
            <FieldScope key={i} path={String(i)}>
              <EntryShell label={`education ${i + 1}`} onDelete={() => removeRow("educations", i)}>
                <InlineText
                  name="school"
                  label="School"
                  source="school"
                  value={e.school}
                  onChange={(v) => set(`${at}.school`, v)}
                  // The school's city fills an empty location.
                  onPick={({ value, detail }) =>
                    update((c) => {
                      const next = setPath(c, `${at}.school`, value);
                      return detail && !e.location ? setPath(next, `${at}.location`, detail) : next;
                    })
                  }
                  className="font-semibold"
                />
                <Dates start={e.start} end={e.end} onStart={(v) => set(`${at}.start`, v)} onEnd={(v) => set(`${at}.end`, v)} />
                <div className="max-w-xs">
                  <InlineText name="gpa" label="GPA" placeholder="GPA…" value={e.gpa} onChange={(v) => set(`${at}.gpa`, v)} className="text-sm" />
                </div>
                <div className="max-w-md">
                  <InlineText name="location" label="Location" placeholder="Location…" source="location" value={e.location} onChange={(v) => set(`${at}.location`, v)} className="text-sm" />
                </div>
                <Degrees degrees={e.degrees} at={at} set={set} update={update} />
                <div className="max-w-md">
                  <InlineText
                    name="achievement"
                    label="Achievement"
                    placeholder="Add achievement…"
                    value={e.achievement}
                    onChange={(v) => set(`${at}.achievement`, v)}
                    className="text-sm"
                  />
                </div>
                <div className="px-2" data-field={`${at}.coursework`}>
                  <TagInput
                    label={`Coursework for education ${i + 1}`}
                    placeholder="Add coursework…"
                    value={e.coursework}
                    onChange={(v) => set(`${at}.coursework`, v)}
                    separators={[",", ";"]}
                  />
                </div>
                <div className="px-2">
                  <BulletList
                    value={e.bullets}
                    onChange={(v) => set(`${at}.bullets`, v)}
                    onShift={(pos, by) => shift(`${at}.bullets`, pos, by)}
                    marker={(n) => <ChangedTag path={`${at}.bullets.${n}`} className="mt-2" />}
                  />
                </div>
              </EntryShell>
            </FieldScope>
          );
        })}
        <AddButton label="Education" onClick={() => update((c) => ({ ...c, educations: [...c.educations, emptyEducation()] }))} />
      </div>
    </FieldScope>
  );
}

function Experiences({ content, set, update, shift, removeRow }: Edit) {
  return (
    <FieldScope path="experiences">
      <div className="space-y-8">
        {content.experiences.map((x, i) => {
          const at = `experiences.${i}`;
          return (
            <FieldScope key={i} path={String(i)}>
              <EntryShell label={`work experience ${i + 1}`} onDelete={() => removeRow("experiences", i)}>
                <InlineText name="company" label="Company" source="company" value={x.company} onChange={(v) => set(`${at}.company`, v)} className="font-semibold" />
                <Dates
                  start={x.start}
                  end={x.end}
                  onStart={(v) => set(`${at}.start`, v)}
                  onEnd={(v) => set(`${at}.end`, v)}
                  current={{
                    checked: !!x.start && !x.end,
                    onChange: (on) => set(`${at}.end`, on ? null : x.start),
                  }}
                />
                <div className="max-w-md">
                  <InlineText name="location" label="Location" placeholder="Location…" source="location" value={x.location} onChange={(v) => set(`${at}.location`, v)} className="text-sm" />
                </div>
                <InlineText name="title" label="Job title" placeholder="Job title" source="title" value={x.title} onChange={(v) => set(`${at}.title`, v)} className="text-sm font-medium" />
                <InlineText
                  name="summary"
                  label="Role summary"
                  placeholder="Summary…"
                  multiline
                  value={x.summary}
                  onChange={(v) => set(`${at}.summary`, v)}
                  className="text-sm"
                />
                <div className="px-2">
                  <BulletList
                    value={x.bullets}
                    onChange={(v) => set(`${at}.bullets`, v)}
                    onShift={(pos, by) => shift(`${at}.bullets`, pos, by)}
                    marker={(n) => <ChangedTag path={`${at}.bullets.${n}`} className="mt-2" />}
                  />
                </div>
              </EntryShell>
            </FieldScope>
          );
        })}
        <AddButton label="Work experience" onClick={() => update((c) => ({ ...c, experiences: [...c.experiences, emptyExperience()] }))} />
      </div>
    </FieldScope>
  );
}

function Projects({ content, set, update, shift, removeRow }: Edit) {
  return (
    <FieldScope path="projects">
      <div className="space-y-8">
        {content.projects.map((p, i) => {
          const at = `projects.${i}`;
          return (
            <FieldScope key={i} path={String(i)}>
              <EntryShell label={`project ${i + 1}`} onDelete={() => removeRow("projects", i)}>
                <InlineText name="name" label="Project name" value={p.name} onChange={(v) => set(`${at}.name`, v)} className="font-semibold" />
                <Dates start={p.start} end={p.end} onStart={(v) => set(`${at}.start`, v)} onEnd={(v) => set(`${at}.end`, v)} />
                <InlineText name="summary" label="One-line summary" placeholder="One-line summary…" value={p.summary} onChange={(v) => set(`${at}.summary`, v)} className="text-sm" />
                <div className="grid gap-1 text-sm sm:grid-cols-2">
                  <InlineText name="role" label="Your role" placeholder="Your role…" value={p.role} onChange={(v) => set(`${at}.role`, v)} />
                  <InlineText name="url" label="Link" placeholder="https://…" icon={Link2} value={p.url} onChange={(v) => set(`${at}.url`, v)} />
                </div>
                <div className="px-2" data-field={`${at}.skills`}>
                  <TagInput label={`Technologies for project ${i + 1}`} placeholder="Add technologies…" value={p.skills} onChange={(v) => set(`${at}.skills`, v)} />
                </div>
                <div className="px-2">
                  <BulletList
                    value={p.bullets}
                    onChange={(v) => set(`${at}.bullets`, v)}
                    onShift={(pos, by) => shift(`${at}.bullets`, pos, by)}
                    marker={(n) => <ChangedTag path={`${at}.bullets.${n}`} className="mt-2" />}
                  />
                </div>
              </EntryShell>
            </FieldScope>
          );
        })}
        <AddButton label="Project" onClick={() => update((c) => ({ ...c, projects: [...c.projects, emptyProject()] }))} />
      </div>
    </FieldScope>
  );
}

/** Skills as groups: a category name and its skills as chips. */
function Skills({ content, update }: Edit) {
  // Groups without skills yet only exist here until their first skill.
  const [empty, setEmpty] = useState<string[]>([]);
  const saved: { category: string; names: string[] }[] = [];
  for (const s of content.skills) {
    const g = saved.find((x) => x.category === s.category);
    if (g) g.names.push(s.name);
    else saved.push({ category: s.category, names: [s.name] });
  }
  const groups = [...saved, ...empty.map((category) => ({ category, names: [] as string[] }))];
  const write = (next: { category: string; names: string[] }[]) => {
    setEmpty(next.filter((g) => g.names.length === 0).map((g) => g.category));
    update((c) => ({
      ...c,
      skills: next.flatMap((g) => g.names.map((name) => ({ name, category: g.category }))),
    }));
  };
  return (
    <div className="space-y-5">
      {groups.map((g, i) => (
        <div key={i} className="group space-y-1">
          <div className="flex items-center gap-1">
            <input
              aria-label={`Skill group ${i + 1} name`}
              placeholder="Group, like Languages"
              value={g.category}
              onChange={(e) => write(groups.map((x, j) => (j === i ? { ...x, category: e.target.value } : x)))}
              className={cn(
                "min-w-0 flex-1 rounded-lg border border-transparent bg-transparent px-2 py-1 text-sm font-semibold outline-none hover:bg-muted/70 focus-visible:border-ring focus-visible:bg-card focus-visible:ring-3 focus-visible:ring-ring/25",
                !g.category && "bg-muted/60",
              )}
            />
            {isAllCaps(g.category) && (
              <IconButton
                label={`Fix capitals: "${tidyCaps(g.category)}"`}
                onClick={() => write(groups.map((x, j) => (j === i ? { ...x, category: tidyCaps(x.category) } : x)))}
              >
                <CaseSensitive />
              </IconButton>
            )}
            <div className="sm:opacity-0 sm:transition-opacity sm:group-focus-within:opacity-100 sm:group-hover:opacity-100">
              <IconButton label={`Delete skill group ${i + 1}`} onClick={() => write(groups.filter((_, j) => j !== i))}>
                <Trash2 />
              </IconButton>
            </div>
          </div>
          <div className="px-2">
            <TagInput
              label={`Skills in ${g.category || `group ${i + 1}`}`}
              placeholder="Add a skill…"
              value={g.names}
              onChange={(names) => write(groups.map((x, j) => (j === i ? { ...x, names } : x)))}
              max={100}
            />
          </div>
        </div>
      ))}
      <AddButton label="Skill group" onClick={() => setEmpty([...empty, ""])} />
    </div>
  );
}
