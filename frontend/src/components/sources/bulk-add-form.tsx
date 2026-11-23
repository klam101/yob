"use client";

import { ListPlus } from "lucide-react";
import { useState } from "react";
import { api } from "@/api/client";
import { Button } from "@/components/ui/button";

/**
 * Queues many job-board links at once (one per line). The worker verifies
 * each one in the background and adds those with open jobs.
 */
export function BulkAddForm() {
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<{ kind: "ok" | "error"; text: string } | null>(null);
  const urls = text
    .split(/[\s,]+/)
    .map((u) => u.trim())
    .filter(Boolean);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (urls.length === 0) return;
    setBusy(true);
    setMessage(null);
    try {
      const { data, error } = await api.POST("/discovery/candidates", { body: { urls } });
      if (data) {
        const skipped = data.recognized - data.queued;
        const parts = [`Queued ${data.queued.toLocaleString()} new ${data.queued === 1 ? "board" : "boards"} for checking.`];
        if (skipped > 0) parts.push(`${skipped.toLocaleString()} already known.`);
        if (data.unrecognized.length > 0) {
          parts.push(`${data.unrecognized.length.toLocaleString()} not recognized as job boards.`);
        }
        setMessage({ kind: "ok", text: parts.join(" ") });
        setText(data.unrecognized.join("\n"));
      } else {
        setMessage({ kind: "error", text: error?.message ?? "Couldn't queue those links." });
      }
    } catch {
      setMessage({ kind: "error", text: "The API didn't respond. Is it running?" });
    } finally {
      setBusy(false);
    }
  }

  return (
    <form onSubmit={submit} className="space-y-2">
      <textarea
        value={text}
        onChange={(e) => setText(e.target.value)}
        rows={4}
        placeholder={"One job-board link per line, e.g.\nhttps://jobs.ashbyhq.com/company\nhttps://boards.greenhouse.io/company"}
        aria-label="Job board links"
        className="w-full rounded-xl border border-input bg-transparent px-3 py-2 text-sm shadow-xs outline-none placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50"
      />
      <div className="flex flex-wrap items-center gap-3">
        <Button type="submit" variant="flat" disabled={busy || urls.length === 0}>
          <ListPlus /> {busy ? "Queueing…" : `Queue ${urls.length || ""} ${urls.length === 1 ? "link" : "links"}`}
        </Button>
        {message && (
          <p role="status" className={message.kind === "ok" ? "text-sm text-success" : "text-sm text-destructive"}>
            {message.text}
          </p>
        )}
      </div>
    </form>
  );
}
