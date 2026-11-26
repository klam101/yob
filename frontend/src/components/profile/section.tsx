"use client";

import { ChevronRight, Info, Loader2, PencilLine } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Sheet,
  SheetBody,
  SheetClose,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { type FieldError, FieldScope } from "./fields";

export type SaveResult = { ok: true } | { ok: false; message: string; field?: string | null };

/**
 * One profile section: a read-only view with a pencil button, and an editor
 * in a side panel. Saving replaces the whole section; closing the panel
 * without saving drops the changes.
 *
 * `open` and `onOpenChange` let the page open a section's editor from
 * elsewhere (the completeness card).
 */
export function Section<D>({
  id,
  title,
  view,
  initial,
  edit,
  onSave,
  scope,
  open,
  onOpenChange,
  note = "Changes here are also used to autofill job applications.",
}: {
  id?: string;
  title: string;
  view: React.ReactNode;
  initial: () => D;
  edit: (draft: D, set: (d: D) => void) => React.ReactNode;
  onSave: (draft: D) => Promise<SaveResult>;
  /** The API path of the section's fields (`experiences`), for errors. */
  scope?: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  note?: string | null;
}) {
  const [draft, setDraft] = useState<D | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<{ message: string; field?: string | null } | null>(null);
  const body = useRef<HTMLDivElement>(null);

  // A fresh draft each time the panel opens.
  const [wasOpen, setWasOpen] = useState(open);
  if (open !== wasOpen) {
    setWasOpen(open);
    setDraft(open ? initial() : null);
    setError(null);
  }

  // After a failed save, bring the field with the error into view.
  useEffect(() => {
    if (!error?.field) return;
    const el = body.current?.querySelector<HTMLElement>("[aria-invalid=true]");
    el?.scrollIntoView({ block: "center", behavior: "smooth" });
    el?.focus({ preventScroll: true });
  }, [error]);

  async function submit() {
    if (draft === null) return;
    setSaving(true);
    setError(null);
    const r = await onSave(draft);
    setSaving(false);
    if (r.ok) onOpenChange(false);
    else setError({ message: r.message, field: r.field });
  }

  const fieldError: FieldError | null = error?.field ? { field: error.field, message: error.message } : null;

  return (
    <section id={id} className="scroll-mt-28 space-y-4" aria-labelledby={id && `${id}-title`}>
      <div className="flex items-center justify-between gap-3">
        <h2 id={id && `${id}-title`} className="text-xl font-semibold tracking-tight">
          {title}
        </h2>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon" aria-label={`Edit ${title.toLowerCase()}`} onClick={() => onOpenChange(true)}>
              <PencilLine />
            </Button>
          </TooltipTrigger>
          <TooltipContent>Edit {title.toLowerCase()}</TooltipContent>
        </Tooltip>
      </div>
      {view}

      <Sheet open={open} onOpenChange={(o) => !saving && onOpenChange(o)}>
        <SheetContent>
          <SheetHeader>
            <SheetClose asChild>
              <Button variant="bordered" size="icon-sm" aria-label="Close without saving">
                <ChevronRight />
              </Button>
            </SheetClose>
            <SheetTitle>{title}</SheetTitle>
            <Button onClick={submit} disabled={saving || draft === null}>
              {saving && <Loader2 className="animate-spin" />}
              {saving ? "Saving…" : "Save"}
            </Button>
          </SheetHeader>
          {note ? (
            <SheetDescription className="flex shrink-0 items-center gap-2 bg-muted/60 px-4 py-2 text-xs sm:px-6">
              <Info className="size-3.5 shrink-0" />
              {note}
            </SheetDescription>
          ) : (
            <SheetDescription className="sr-only">Edit {title.toLowerCase()}</SheetDescription>
          )}
          <SheetBody ref={body}>
            {error && (
              <p role="alert" className="mb-5 rounded-xl bg-destructive/10 px-3 py-2 text-sm text-destructive">
                {error.field ? `${error.message} It's marked below.` : error.message}
              </p>
            )}
            {draft !== null && (
              <FieldScope path={scope ?? ""} error={fieldError}>
                {edit(draft, setDraft)}
              </FieldScope>
            )}
          </SheetBody>
          <SheetFooter>
            <SheetClose asChild>
              <Button variant="ghost" disabled={saving}>
                Cancel
              </Button>
            </SheetClose>
          </SheetFooter>
        </SheetContent>
      </Sheet>
    </section>
  );
}
