"use client";

import { ArrowLeft, ArrowRight, Loader2 } from "lucide-react";
import { useRef, useState } from "react";
import type { Preferences } from "@/api/client";
import { Button } from "@/components/ui/button";
import { cn } from "cn";
import { savePreferences, skipPreferences } from "./api";
import { STEPS } from "./questions";
import { scrollToTop } from "@/lib/scroll";

/**
 * The onboarding questionnaire, one topic per step. Nothing is saved until
 * the last step, and every question can be left blank (blank means "any").
 */
export function Wizard({ initial }: { initial: Preferences }) {
  const [draft, setDraft] = useState(initial);
  const [step, setStep] = useState(0);
  const [busy, setBusy] = useState<"save" | "skip" | null>(null);
  const [error, setError] = useState<string | null>(null);
  const heading = useRef<HTMLHeadingElement>(null);

  const last = step === STEPS.length - 1;
  const { title, Body } = STEPS[step];

  // A full page load: the router remembers that /jobs redirected here before
  // the answers were saved, and would follow that redirect again.
  function toFeed() {
    // eslint-disable-next-line @next/next/no-location-assign-relative-destination
    window.location.assign("/jobs");
  }

  function go(to: number) {
    setStep(to);
    setError(null);
    scrollToTop();
    // Move focus to the new step's heading for keyboard and screen-reader users.
    requestAnimationFrame(() => heading.current?.focus());
  }

  async function finish() {
    setBusy("save");
    setError(null);
    const r = await savePreferences(draft);
    if (!r.ok) {
      setBusy(null);
      setError(r.message);
      return;
    }
    toFeed();
  }

  async function skip() {
    setBusy("skip");
    const r = await skipPreferences();
    if (!r.ok) {
      setBusy(null);
      setError(r.message);
      return;
    }
    toFeed();
  }

  return (
    <div className="space-y-6">
      <div className="space-y-3">
        <div className="flex items-center justify-between gap-3 text-sm">
          <span className="font-medium text-muted-foreground">
            Step {step + 1} of {STEPS.length}
          </span>
          <Button variant="light" size="sm" onClick={skip} disabled={busy !== null}>
            {busy === "skip" ? "Skipping…" : "Skip for now"}
          </Button>
        </div>
        <ol className="flex gap-1.5" aria-label="Steps">
          {STEPS.map((s, i) => (
            <li key={s.id} className="flex-1">
              <button
                type="button"
                onClick={() => go(i)}
                aria-current={i === step ? "step" : undefined}
                aria-label={`Step ${i + 1}: ${s.title}`}
                className={cn(
                  "block h-1.5 w-full rounded-full transition-colors",
                  i <= step ? "bg-primary" : "bg-muted hover:bg-primary/30",
                )}
              />
            </li>
          ))}
        </ol>
      </div>

      <div className="space-y-6 rounded-3xl border border-border/70 bg-card p-5 shadow-soft sm:p-8">
        <h2 ref={heading} tabIndex={-1} className="text-xl font-semibold tracking-tight outline-none">
          {title}
        </h2>
        <Body value={draft} onChange={setDraft} />
      </div>

      {error && (
        <p role="alert" className="rounded-xl bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </p>
      )}

      <div className="flex items-center justify-between gap-3">
        <Button variant="ghost" onClick={() => go(step - 1)} disabled={step === 0 || busy !== null}>
          <ArrowLeft /> Back
        </Button>
        {last ? (
          <Button size="lg" onClick={finish} disabled={busy !== null}>
            {busy === "save" ? <Loader2 className="animate-spin" /> : null}
            {busy === "save" ? "Saving…" : "Show my jobs"}
          </Button>
        ) : (
          <Button size="lg" onClick={() => go(step + 1)}>
            Next <ArrowRight />
          </Button>
        )}
      </div>
    </div>
  );
}
