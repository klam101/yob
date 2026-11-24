"use client";

import { FileUp, Loader2 } from "lucide-react";
import { useRef, useState } from "react";
import type { ResumeDraft } from "@/api/client";
import { Button } from "@/components/ui/button";
import { cn } from "cn";
import { uploadResume } from "./save";

/** Drop zone (or button) for a PDF or DOCX; hands the parsed draft back. */
export function ResumeUpload({
  onParsed,
  compact = false,
}: {
  onParsed: (draft: ResumeDraft) => void;
  compact?: boolean;
}) {
  const input = useRef<HTMLInputElement>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [over, setOver] = useState(false);

  async function handle(file: File | undefined) {
    if (!file) return;
    setError(null);
    setBusy(true);
    const result = await uploadResume(file);
    setBusy(false);
    if (input.current) input.current.value = "";
    if (result.ok) onParsed(result.value);
    else setError(result.message);
  }

  const picker = (
    <input
      ref={input}
      type="file"
      accept=".pdf,.docx,application/pdf,application/vnd.openxmlformats-officedocument.wordprocessingml.document"
      className="sr-only"
      aria-label="Resume file"
      onChange={(e) => handle(e.target.files?.[0])}
    />
  );

  if (compact) {
    return (
      <div className="space-y-2">
        {picker}
        <Button variant="bordered" className="w-full" disabled={busy} onClick={() => input.current?.click()}>
          {busy ? <Loader2 className="animate-spin" /> : <FileUp />} {busy ? "Reading…" : "Fill from a resume"}
        </Button>
        {error && (
          <p role="alert" className="text-xs text-destructive">
            {error}
          </p>
        )}
      </div>
    );
  }

  return (
    <div className="space-y-2">
      {picker}
      <button
        type="button"
        disabled={busy}
        onClick={() => input.current?.click()}
        onDragOver={(e) => {
          e.preventDefault();
          setOver(true);
        }}
        onDragLeave={() => setOver(false)}
        onDrop={(e) => {
          e.preventDefault();
          setOver(false);
          handle(e.dataTransfer.files?.[0]);
        }}
        className={cn(
          "flex w-full flex-col items-center gap-2 rounded-2xl border-2 border-dashed border-border px-6 py-10 text-center transition-colors hover:border-primary/60 hover:bg-primary/5",
          over && "border-primary bg-primary/5",
        )}
      >
        <span className="flex size-11 items-center justify-center rounded-2xl bg-primary/12 text-primary dark:bg-primary/20">
          {busy ? <Loader2 className="size-5 animate-spin" /> : <FileUp className="size-5" />}
        </span>
        <span className="font-medium">{busy ? "Reading your resume…" : "Drop your resume here, or click to choose"}</span>
        <span className="text-xs text-muted-foreground">PDF or Word (.docx), up to 5 MB</span>
      </button>
      {error && (
        <p role="alert" className="rounded-xl bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </p>
      )}
    </div>
  );
}
