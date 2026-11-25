"use client";

import {
  Download,
  Ellipsis,
  FilePlus2,
  FileText,
  Loader2,
  PencilLine,
  RefreshCcw,
  Sparkles,
  Star,
  Trash2,
  UserRoundPen,
} from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState } from "react";
import type { Profile, ResumeDetail, ResumeInfo, ResumeUploaded } from "@/api/client";
import { ProjectEditor } from "@/components/profile/editors";
import { FieldScope, TextField } from "@/components/profile/fields";
import { ResumeUpload } from "@/components/profile/resume-upload";
import { resumes as resumeApi, save } from "@/components/profile/save";
import { Section } from "@/components/profile/section";
import { ProjectView } from "@/components/profile/views";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { timeAgo } from "@/lib/format";
import { UpdateProfileDialog } from "./update-profile-dialog";

type Dialogs =
  | { kind: "add" }
  | { kind: "edit"; resume: ResumeInfo }
  | { kind: "delete"; resume: ResumeInfo }
  | { kind: "update"; resume: ResumeInfo }
  | null;

/**
 * The Resume page: uploaded resumes (with a primary one) and the projects
 * tailored resumes draw from. A resume opens on its edit page; "Update
 * profile" copies chosen sections of it into the profile.
 */
export function ResumeManager({ initial, profile }: { initial: ResumeInfo[]; profile: Profile }) {
  const router = useRouter();
  const [list, setList] = useState(initial);
  const [projects, setProjects] = useState(profile.projects);
  const [dialog, setDialog] = useState<Dialogs>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [notice, setNotice] = useState<{ tone: "error" | "ok"; text: string } | null>(null);
  const [editingProjects, setEditingProjects] = useState(false);

  async function refresh() {
    const r = await resumeApi.list();
    if (r.ok) setList(r.value);
  }

  async function makePrimary(r: ResumeInfo) {
    setBusy(r.id);
    const res = await resumeApi.update(r.id, { name: null, target_title: null, primary: true });
    setBusy(null);
    if (!res.ok) return setNotice({ tone: "error", text: res.message });
    await refresh();
    setNotice({ tone: "ok", text: `${res.value.name} is now your primary resume.` });
  }

  return (
    <div className="space-y-8">
      <section aria-labelledby="resumes-title" className="space-y-3">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h2 id="resumes-title" className="text-xl font-semibold tracking-tight">
              Your resumes
            </h2>
            <p className="text-sm text-muted-foreground">
              {list.length === 0
                ? "No resumes yet."
                : `${list.length} saved. The primary one is used for autofill and as the base for tailoring.`}
            </p>
          </div>
          <Button onClick={() => setDialog({ kind: "add" })}>
            <FilePlus2 /> Add resume
          </Button>
        </div>

        {notice && (
          <p
            role={notice.tone === "error" ? "alert" : "status"}
            className={
              notice.tone === "error"
                ? "rounded-xl bg-destructive/10 px-3 py-2 text-sm text-destructive"
                : "rounded-xl bg-success/12 px-3 py-2 text-sm text-success"
            }
          >
            {notice.text}
          </p>
        )}

        <div className="overflow-hidden rounded-3xl border border-border/70 bg-card shadow-soft">
          {list.length === 0 ? (
            <div className="flex flex-col items-center gap-3 px-6 py-12 text-center">
              <span className="flex size-11 items-center justify-center rounded-2xl bg-primary/12 text-primary">
                <FileText className="size-5" />
              </span>
              <p className="font-medium">Upload a resume to fill your profile from it</p>
              <Button variant="bordered" onClick={() => setDialog({ kind: "add" })}>
                <FilePlus2 /> Add resume
              </Button>
            </div>
          ) : (
            <table className="w-full text-sm">
              <caption className="sr-only">Your resumes</caption>
              <thead className="hidden border-b border-border/70 text-left text-xs text-muted-foreground sm:table-header-group">
                <tr>
                  <th scope="col" className="px-5 py-3 font-medium">
                    Resume
                  </th>
                  <th scope="col" className="px-3 py-3 font-medium">
                    Target job title
                  </th>
                  <th scope="col" className="px-3 py-3 font-medium">
                    Last modified
                  </th>
                  <th scope="col" className="px-3 py-3 font-medium">
                    Uploaded
                  </th>
                  <th scope="col" className="w-14 px-3 py-3">
                    <span className="sr-only">Actions</span>
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y divide-border/60">
                {list.map((r) => (
                  <tr
                    key={r.id}
                    className="relative flex flex-wrap items-center gap-x-3 px-5 py-4 transition-colors hover:bg-muted/40 sm:table-row sm:px-0 sm:py-0"
                  >
                    <td className="min-w-0 flex-1 sm:px-5 sm:py-4">
                      <div className="flex min-w-0 items-center gap-2">
                        <FileText className="size-4 shrink-0 text-muted-foreground" />
                        {/* The whole row opens the resume. */}
                        <Link
                          href={`/resume/edit/${r.id}`}
                          className="truncate font-medium outline-none after:absolute after:inset-0 after:rounded-none hover:underline focus-visible:underline"
                        >
                          {r.name}
                        </Link>
                        {r.is_primary && (
                          <Badge variant="success" size="sm">
                            <Star className="fill-current" /> Primary
                          </Badge>
                        )}
                      </div>
                      <p className="mt-0.5 truncate pl-6 text-xs text-muted-foreground">{r.filename}</p>
                    </td>
                    <td className="order-last w-full pl-6 text-muted-foreground sm:order-none sm:w-auto sm:px-3 sm:py-4">
                      {r.target_title || <span className="sm:text-muted-foreground/60">No target title</span>}
                    </td>
                    <td className="hidden px-3 py-4 text-muted-foreground sm:table-cell" suppressHydrationWarning>
                      {timeAgo(r.updated_at)}
                    </td>
                    <td className="hidden px-3 py-4 text-muted-foreground sm:table-cell" suppressHydrationWarning>
                      {timeAgo(r.uploaded_at)}
                    </td>
                    <td className="relative z-10 sm:px-3 sm:py-4">
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button variant="ghost" size="icon-sm" aria-label={`Actions for ${r.name}`} disabled={busy === r.id}>
                            {busy === r.id ? <Loader2 className="animate-spin" /> : <Ellipsis />}
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end" className="w-56">
                          <DropdownMenuItem disabled={r.is_primary} onSelect={() => makePrimary(r)}>
                            <Star /> Use as primary
                          </DropdownMenuItem>
                          <DropdownMenuItem onSelect={() => setDialog({ kind: "edit", resume: r })}>
                            <PencilLine /> Edit info
                          </DropdownMenuItem>
                          <DropdownMenuItem onSelect={() => setDialog({ kind: "update", resume: r })}>
                            <UserRoundPen /> Update profile
                          </DropdownMenuItem>
                          <DropdownMenuItem asChild>
                            <a href={`/api/profile/resumes/${r.id}/file`} download>
                              <Download /> Download original
                            </a>
                          </DropdownMenuItem>
                          <DropdownMenuSeparator />
                          <DropdownMenuItem variant="destructive" onSelect={() => setDialog({ kind: "delete", resume: r })}>
                            <Trash2 /> Delete
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      </section>

      <div className="rounded-3xl border border-border/70 bg-card p-5 shadow-soft sm:p-7">
        <Section
          id="projects"
          title="Projects"
          scope="projects"
          open={editingProjects}
          onOpenChange={setEditingProjects}
          note="Tailored resumes pick from these projects. They aren't part of your profile."
          view={<ProjectView items={projects} />}
          initial={() => projects}
          edit={(d, set) => <ProjectEditor value={d} onChange={set} />}
          onSave={async (d) => {
            const r = await save.projects(d);
            if (r.ok) setProjects(r.value);
            return r;
          }}
        />
      </div>

      <section className="flex items-start gap-4 rounded-3xl border border-dashed border-border p-5 sm:p-7">
        <span className="flex size-10 shrink-0 items-center justify-center rounded-2xl bg-primary/12 text-primary">
          <Sparkles className="size-5" />
        </span>
        <div className="space-y-1">
          <h2 className="font-semibold">Tailored versions</h2>
          <p className="text-sm text-muted-foreground">
            Coming next: a version of your resume for each job, rephrased from your profile (never made up), with
            a keyword gap report.
          </p>
        </div>
      </section>

      <AddResumeDialog
        open={dialog?.kind === "add"}
        onClose={() => setDialog(null)}
        onParsed={(d) => {
          setDialog(null);
          // The first resume fills the profile by itself. Later ones open
          // with "Update profile", which asks which sections to use.
          router.push(d.profile_filled ? `/resume/edit/${d.resume_id}` : `/resume/edit/${d.resume_id}?update=1`);
        }}
      />
      {dialog?.kind === "update" && (
        <UpdateProfileDialog
          resumeId={dialog.resume.id}
          name={dialog.resume.name}
          open
          onOpenChange={(o) => !o && setDialog(null)}
        />
      )}
      {dialog?.kind === "edit" && (
        <EditDetailsDialog
          resume={dialog.resume}
          onClose={() => setDialog(null)}
          onSaved={async (updated) => {
            setDialog(null);
            await refresh();
            setNotice({ tone: "ok", text: `Saved ${updated.name}.` });
          }}
          onReparsed={async () => {
            setDialog(null);
            await refresh();
            setNotice({ tone: "ok", text: "Read the file again." });
          }}
        />
      )}
      {dialog?.kind === "delete" && (
        <DeleteDialog
          resume={dialog.resume}
          onClose={() => setDialog(null)}
          onDeleted={async () => {
            const name = dialog.resume.name;
            setDialog(null);
            await refresh();
            setNotice({ tone: "ok", text: `Deleted ${name}. Your profile wasn't changed.` });
          }}
        />
      )}
    </div>
  );
}

function AddResumeDialog({
  open,
  onClose,
  onParsed,
}: {
  open: boolean;
  onClose: () => void;
  onParsed: (d: ResumeUploaded) => void;
}) {
  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>Add a resume</DialogTitle>
          <DialogDescription>
            We&apos;ll read it on this computer and open it, so you can check what we read. Your profile changes
            only when you choose &ldquo;Update profile&rdquo;, except for your first resume, which fills an empty
            profile.
          </DialogDescription>
        </DialogHeader>
        <ResumeUpload onParsed={onParsed} />
      </DialogContent>
    </Dialog>
  );
}

export function EditDetailsDialog({
  resume,
  onClose,
  onSaved,
  onReparsed,
}: {
  resume: ResumeInfo;
  onClose: () => void;
  onSaved: (updated: ResumeInfo) => void;
  /** After "Read the file again" replaced the resume's content. */
  onReparsed: (fresh: ResumeDetail) => void;
}) {
  const [name, setName] = useState(resume.name);
  const [title, setTitle] = useState(resume.target_title);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<{ message: string; field?: string | null } | null>(null);
  const [confirmReparse, setConfirmReparse] = useState(false);

  async function reparse() {
    setSaving(true);
    setError(null);
    const r = await resumeApi.reparse(resume.id);
    setSaving(false);
    if (r.ok) onReparsed(r.value);
    else setError(r);
  }

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setSaving(true);
    setError(null);
    const r = await resumeApi.update(resume.id, { name, target_title: title, primary: null });
    setSaving(false);
    if (r.ok) onSaved(r.value);
    else setError(r);
  }

  return (
    <Dialog open onOpenChange={(o) => !o && !saving && onClose()}>
      <DialogContent className="sm:max-w-md">
        <form onSubmit={submit} className="space-y-4">
          <DialogHeader>
            <DialogTitle>Edit resume info</DialogTitle>
            <DialogDescription>How it&apos;s listed. The file itself doesn&apos;t change.</DialogDescription>
          </DialogHeader>
          <FieldScope path="" error={error?.field ? { field: error.field, message: error.message } : null}>
            <TextField label="Name" name="name" required value={name} onChange={setName} />
            <TextField
              label="Target job title"
              name="target_title"
              value={title}
              onChange={setTitle}
              placeholder="e.g. QA engineer"
              hint="The kind of job this version is for."
            />
          </FieldScope>
          {error && !error.field && (
            <p role="alert" className="rounded-xl bg-destructive/10 px-3 py-2 text-sm text-destructive">
              {error.message}
            </p>
          )}
          <div className="space-y-2 rounded-2xl bg-muted/60 p-3 text-sm">
            <p className="font-medium">Read the file again</p>
            <p className="text-xs text-muted-foreground">
              Replaces this resume&apos;s content with a fresh read of {resume.filename}, including any edits you made
              to it. Your profile isn&apos;t changed.
            </p>
            {confirmReparse ? (
              <div className="flex flex-wrap gap-2">
                <Button type="button" size="sm" variant="destructive" onClick={reparse} disabled={saving}>
                  <RefreshCcw /> Replace with a fresh read
                </Button>
                <Button type="button" size="sm" variant="ghost" onClick={() => setConfirmReparse(false)} disabled={saving}>
                  Keep my edits
                </Button>
              </div>
            ) : (
              <Button type="button" size="sm" variant="bordered" onClick={() => setConfirmReparse(true)} disabled={saving}>
                <RefreshCcw /> Read the file again
              </Button>
            )}
          </div>
          <DialogFooter>
            <Button type="button" variant="ghost" onClick={onClose} disabled={saving}>
              Cancel
            </Button>
            <Button type="submit" disabled={saving}>
              {saving ? "Saving…" : "Save"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

export function DeleteDialog({
  resume,
  onClose,
  onDeleted,
}: {
  resume: ResumeInfo;
  onClose: () => void;
  onDeleted: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  async function remove() {
    setBusy(true);
    const r = await resumeApi.remove(resume.id);
    setBusy(false);
    if (r.ok) onDeleted();
    else setError(r.message);
  }
  return (
    <Dialog open onOpenChange={(o) => !o && !busy && onClose()}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Delete {resume.name}?</DialogTitle>
          <DialogDescription>
            The file is removed from this computer. Your profile stays as it is.
            {resume.is_primary && " Your newest other resume becomes the primary one."}
          </DialogDescription>
        </DialogHeader>
        {error && (
          <p role="alert" className="rounded-xl bg-destructive/10 px-3 py-2 text-sm text-destructive">
            {error}
          </p>
        )}
        <DialogFooter>
          <Button variant="ghost" onClick={onClose} disabled={busy}>
            Cancel
          </Button>
          <Button variant="destructive" onClick={remove} disabled={busy}>
            {busy ? "Deleting…" : "Delete"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
