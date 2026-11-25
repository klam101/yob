"use client";

import { ArrowRight, CheckCircle2 } from "lucide-react";
import Link from "next/link";
import { useState } from "react";
import type { Preferences } from "@/api/client";
import { Button } from "@/components/ui/button";
import { savePreferences } from "./api";
import { STEPS } from "./questions";

/** Every question on one page, saved together. */
export function PreferencesForm({ initial }: { initial: Preferences }) {
  const [saved, setSaved] = useState(initial);
  const [draft, setDraft] = useState(initial);
  const [saving, setSaving] = useState(false);
  const [status, setStatus] = useState<{ ok: boolean; message: string } | null>(null);
  const dirty = JSON.stringify(draft) !== JSON.stringify(saved);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setSaving(true);
    setStatus(null);
    const r = await savePreferences(draft);
    setSaving(false);
    if (r.ok) {
      setSaved(r.value.preferences);
      setDraft(r.value.preferences);
      setStatus({ ok: true, message: "Saved. The feed uses these as its default filters." });
    } else {
      setStatus({ ok: false, message: r.message });
    }
  }

  return (
    <form onSubmit={submit} className="grid gap-6 lg:grid-cols-[1fr_17rem]">
      <div className="min-w-0 space-y-6">
        {STEPS.map(({ id, title, Body }) => (
          <section
            key={id}
            aria-labelledby={`pref-${id}`}
            className="space-y-5 rounded-3xl border border-border/70 bg-card p-5 shadow-soft sm:p-7"
          >
            <h2 id={`pref-${id}`} className="text-lg font-semibold">
              {title}
            </h2>
            <Body value={draft} onChange={setDraft} />
          </section>
        ))}
      </div>

      <aside className="lg:sticky lg:top-6 lg:self-start">
        <div className="space-y-3 rounded-3xl border border-border/70 bg-card p-5 shadow-soft">
          {status && (
            <p
              role={status.ok ? "status" : "alert"}
              className={
                status.ok
                  ? "flex items-start gap-2 rounded-xl bg-success/10 px-3 py-2 text-sm text-[color-mix(in_oklch,var(--success),black_30%)] dark:text-success"
                  : "rounded-xl bg-destructive/10 px-3 py-2 text-sm text-destructive"
              }
            >
              {status.ok && <CheckCircle2 className="mt-0.5 size-4 shrink-0" />}
              {status.message}
            </p>
          )}
          <Button type="submit" size="lg" className="w-full" disabled={saving || !dirty}>
            {saving ? "Saving…" : dirty ? "Save preferences" : "No changes"}
          </Button>
          <Button type="button" variant="ghost" className="w-full" disabled={saving || !dirty} onClick={() => setDraft(saved)}>
            Undo changes
          </Button>
          <Button asChild variant="light" className="w-full">
            <Link href="/jobs">
              See matching jobs <ArrowRight />
            </Link>
          </Button>
        </div>
      </aside>
    </form>
  );
}
