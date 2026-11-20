"use client";

import { ArrowUpRight, RefreshCw, Search } from "lucide-react";
import { useRouter } from "next/navigation";
import { useMemo, useState, useTransition } from "react";
import { type CompanySource, api } from "@/api/client";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { sourceLabel, timeAgo, timeUntil } from "@/lib/format";

type Filter = "all" | "errors" | "pending";

export function SourcesTable({ companies }: { companies: CompanySource[] }) {
  const router = useRouter();
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState<Filter>("all");
  const [queued, setQueued] = useState<Set<string>>(new Set());
  const [, startTransition] = useTransition();

  const errors = companies.filter((c) => c.last_status === "error").length;
  const pending = companies.filter((c) => !c.last_status).length;

  const rows = useMemo(() => {
    const s = search.trim().toLowerCase();
    return companies.filter((c) => {
      if (filter === "errors" && c.last_status !== "error") return false;
      if (filter === "pending" && c.last_status) return false;
      return !s || c.name.toLowerCase().includes(s) || c.board_token.includes(s);
    });
  }, [companies, search, filter]);

  async function checkNow(id: string) {
    setQueued((q) => new Set(q).add(id));
    await api.POST("/companies/{id}/check", { params: { path: { id } } }).catch(() => {});
    // The worker picks it up within ~15s; refresh to show the new status.
    setTimeout(() => startTransition(() => router.refresh()), 20_000);
  }

  return (
    <div className="space-y-3">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
        <label className="flex h-10 flex-1 items-center gap-2.5 rounded-xl bg-muted px-3">
          <Search className="size-4 text-muted-foreground" />
          <span className="sr-only">Search companies</span>
          <input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search companies"
            className="w-full bg-transparent text-sm outline-none placeholder:text-muted-foreground"
          />
        </label>
        <div className="flex gap-1">
          {(
            [
              ["all", `All ${companies.length}`],
              ["errors", `Errors ${errors}`],
              ["pending", `Not checked ${pending}`],
            ] as const
          ).map(([value, label]) => (
            <Button
              key={value}
              size="sm"
              shape="pill"
              variant={filter === value ? "flat" : "ghost"}
              onClick={() => setFilter(value)}
            >
              {label}
            </Button>
          ))}
        </div>
      </div>

      <div className="overflow-x-auto rounded-2xl border border-border/70">
        <table className="w-full min-w-[42rem] text-sm">
          <thead className="bg-muted/60 text-left text-xs text-muted-foreground">
            <tr>
              <th className="px-4 py-2.5 font-medium">Company</th>
              <th className="px-4 py-2.5 font-medium">Platform</th>
              <th className="px-4 py-2.5 text-right font-medium">Open jobs</th>
              <th className="px-4 py-2.5 font-medium">Last checked</th>
              <th className="px-4 py-2.5 font-medium">Next check</th>
              <th className="px-4 py-2.5">
                <span className="sr-only">Actions</span>
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border/60">
            {rows.map((c) => (
              <tr key={c.id} className="transition-colors hover:bg-muted/40">
                <td className="px-4 py-2.5">
                  <a
                    href={c.careers_url}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="inline-flex items-center gap-1 font-medium hover:text-primary"
                  >
                    {c.name} <ArrowUpRight className="size-3.5 text-muted-foreground" />
                  </a>
                </td>
                <td className="px-4 py-2.5">
                  <Badge variant="secondary" size="sm">
                    {sourceLabel(c.platform)}
                  </Badge>
                </td>
                <td className="px-4 py-2.5 text-right tabular-nums">{c.open_jobs.toLocaleString()}</td>
                <td className="px-4 py-2.5">
                  <Status company={c} />
                </td>
                {/* Relative times differ by a few seconds between server and browser. */}
                <td className="px-4 py-2.5 text-muted-foreground" suppressHydrationWarning>
                  {queued.has(c.id) ? "Queued" : timeUntil(c.next_check_at)}
                </td>
                <td className="px-4 py-2.5 text-right">
                  <Button
                    variant="ghost"
                    size="xs"
                    onClick={() => checkNow(c.id)}
                    disabled={queued.has(c.id)}
                  >
                    <RefreshCw /> Check now
                  </Button>
                </td>
              </tr>
            ))}
            {rows.length === 0 && (
              <tr>
                <td colSpan={6} className="px-4 py-10 text-center text-muted-foreground">
                  No companies match.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function Status({ company: c }: { company: CompanySource }) {
  if (!c.last_status || !c.last_checked_at) {
    return <span className="text-muted-foreground">Not yet</span>;
  }
  if (c.last_status === "error") {
    return (
      <Tooltip>
        <TooltipTrigger asChild>
          <span tabIndex={0}>
            <Badge variant="destructive" size="sm">
              <span suppressHydrationWarning>Failed {timeAgo(c.last_checked_at)}</span>
            </Badge>
          </span>
        </TooltipTrigger>
        <TooltipContent className="max-w-xs">{c.last_error ?? "Unknown error"}</TooltipContent>
      </Tooltip>
    );
  }
  return <span suppressHydrationWarning>{timeAgo(c.last_checked_at)}</span>;
}
