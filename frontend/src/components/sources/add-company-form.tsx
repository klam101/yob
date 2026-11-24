"use client";

import { Plus } from "lucide-react";
import { useRouter } from "next/navigation";
import { useState, useTransition } from "react";
import { api } from "@/api/client";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

/** Adds a company from its job-board link on any supported platform. */
export function AddCompanyForm() {
  const router = useRouter();
  const [url, setUrl] = useState("");
  const [name, setName] = useState("");
  const [message, setMessage] = useState<{ kind: "ok" | "error"; text: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const [, startTransition] = useTransition();

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (!url.trim()) return;
    setBusy(true);
    setMessage(null);
    try {
      const { data, error } = await api.POST("/companies", {
        body: { url: url.trim(), name: name.trim() || undefined },
      });
      if (data) {
        setMessage({ kind: "ok", text: `Added ${data.name}. Its jobs will appear within a minute.` });
        setUrl("");
        setName("");
        startTransition(() => router.refresh());
      } else {
        setMessage({ kind: "error", text: error?.message ?? "Couldn't add that company." });
      }
    } catch {
      setMessage({ kind: "error", text: "The API didn't respond. Is it running?" });
    } finally {
      setBusy(false);
    }
  }

  return (
    <form onSubmit={submit} className="space-y-3">
      <div className="flex flex-col gap-2 sm:flex-row">
        <Input
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          placeholder="Job board link, e.g. https://jobs.lever.co/company"
          aria-label="Job board link"
          className="sm:flex-[2]"
        />
        <Input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Name (optional)"
          aria-label="Company name"
          className="sm:flex-1"
        />
        <Button type="submit" disabled={busy || !url.trim()}>
          <Plus /> {busy ? "Checking…" : "Add company"}
        </Button>
      </div>
      <p className="text-xs text-muted-foreground">
        Works with Greenhouse, Lever, Ashby, Workday (…myworkdayjobs.com/…), SmartRecruiters,
        Workable, Recruitee and BambooHR job boards. Find the link on a company&apos;s careers
        page.
      </p>
      {message && (
        <p
          role="status"
          className={message.kind === "ok" ? "text-sm text-success" : "text-sm text-destructive"}
        >
          {message.text}
        </p>
      )}
    </form>
  );
}
