"use client";

import { ArrowUpRight, ChevronLeft, ChevronRight, RefreshCw, Search } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { type CompaniesPage, type CompanySource, api } from "@/api/client";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { sourceLabel, timeAgo, timeUntil } from "@/lib/format";
import { cn } from "cn";

type Status = "all" | "errors" | "pending";
const PAGE_SIZE = 50;

/**
 * Tracked companies, searched and paged on the server (there can be
 * thousands). Starts from the first page rendered by the server.
 */
export function SourcesTable({ initial }: { initial: CompaniesPage }) {
  const [data, setData] = useState(initial);
  const [search, setSearch] = useState("");
  const [status, setStatus] = useState<Status>("all");
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(false);
  const [queued, setQueued] = useState<Set<string>>(new Set());
  const first = useRef(true);

  async function load(q: string, s: Status, p: number) {
    setLoading(true);
    const { data } = await api
      .GET("/companies", { params: { query: { q: q || undefined, status: s, page: p, page_size: PAGE_SIZE } } })
      .catch(() => ({ data: undefined }));
    if (data) setData(data);
    setLoading(false);
  }

  useEffect(() => {
    // The first page came from the server.
    if (first.current) {
      first.current = false;
      return;
    }
    const timer = setTimeout(() => load(search.trim(), status, page), 200);
    return () => clearTimeout(timer);
  }, [search, status, page]);

  async function checkNow(id: string) {
    setQueued((q) => new Set(q).add(id));
    await api.POST("/companies/{id}/check", { params: { path: { id } } }).catch(() => {});
    // The worker picks it up within ~15s; reload to show the new status.
    setTimeout(() => {
      setQueued((q) => {
        const next = new Set(q);
        next.delete(id);
        return next;
      });
      load(search.trim(), status, page);
    }, 20_000);
  }

  const pages = Math.max(1, Math.ceil(data.total / PAGE_SIZE));
  const { summary } = data;

  return (
    <div className="space-y-3">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
        <label className="flex h-10 flex-1 items-center gap-2.5 rounded-xl bg-muted px-3">
          <Search className="size-4 text-muted-foreground" />
          <span className="sr-only">Search companies</span>
          <input
            value={search}
            onChange={(e) => {
              setSearch(e.target.value);
              setPage(1);
            }}
            placeholder="Search companies"
            className="w-full bg-transparent text-sm outline-none placeholder:text-muted-foreground"
          />
        </label>
        <div className="flex gap-1">
          {(
            [
              ["all", `All ${summary.companies.toLocaleString()}`],
              ["errors", `Errors ${summary.errors.toLocaleString()}`],
              ["pending", `Not checked ${summary.pending.toLocaleString()}`],
            ] as const
          ).map(([value, label]) => (
            <Button
              key={value}
              size="sm"
              shape="pill"
              variant={status === value ? "flat" : "ghost"}
              onClick={() => {
                setStatus(value);
                setPage(1);
              }}
            >
              {label}
            </Button>
          ))}
        </div>
      </div>

      <div
        className={cn(
          "overflow-x-auto rounded-2xl border border-border/70 transition-opacity",
          loading && "opacity-60",
        )}
      >
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
            {data.items.map((c) => (
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
                  <CheckStatus company={c} />
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
            {data.items.length === 0 && (
              <tr>
                <td colSpan={6} className="px-4 py-10 text-center text-muted-foreground">
                  No companies match.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      {pages > 1 && (
        <nav aria-label="Company pages" className="flex items-center justify-between gap-2 text-sm">
          <span className="text-muted-foreground tabular-nums">
            {((page - 1) * PAGE_SIZE + 1).toLocaleString()}–
            {Math.min(page * PAGE_SIZE, data.total).toLocaleString()} of {data.total.toLocaleString()}
          </span>
          <div className="flex gap-1">
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label="Previous page"
              disabled={page <= 1}
              onClick={() => setPage(page - 1)}
            >
              <ChevronLeft />
            </Button>
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label="Next page"
              disabled={page >= pages}
              onClick={() => setPage(page + 1)}
            >
              <ChevronRight />
            </Button>
          </div>
        </nav>
      )}
    </div>
  );
}

function CheckStatus({ company: c }: { company: CompanySource }) {
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
