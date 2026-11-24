import { ChevronLeft, ChevronRight, SearchX, ServerCrash } from "lucide-react";
import Link from "next/link";
import { Button } from "@/components/ui/button";
import { type Query, activeCount, clearFilters, toHref, withValue } from "@/lib/filters";
import { cn } from "cn";

export function Pagination({ query, page, pageSize, total }: { query: Query; page: number; pageSize: number; total: number }) {
  const pages = Math.max(1, Math.ceil(total / pageSize));
  if (pages === 1) return null;
  const href = (p: number) => toHref(withValue(query, "page", p > 1 ? String(p) : undefined));

  return (
    <nav aria-label="Pages" className="flex items-center justify-center gap-1 pt-2">
      <PageLink href={href(page - 1)} disabled={page <= 1} label="Previous page">
        <ChevronLeft />
      </PageLink>
      {pageWindow(page, pages).map((p, i) =>
        p === null ? (
          <span key={`gap-${i}`} className="px-1 text-muted-foreground" aria-hidden>
            …
          </span>
        ) : (
          <Button
            key={p}
            asChild
            size={p >= 1000 ? "sm" : "icon-sm"}
            variant={p === page ? "flat" : "ghost"}
            className={cn("tabular-nums", p === page && "font-semibold")}
          >
            <Link href={href(p)} aria-current={p === page ? "page" : undefined}>
              {p.toLocaleString()}
            </Link>
          </Button>
        ),
      )}
      <PageLink href={href(page + 1)} disabled={page >= pages} label="Next page">
        <ChevronRight />
      </PageLink>
    </nav>
  );
}

/**
 * First, last and the pages around the current one, with gaps as null:
 * 1 … 4 5 [6] 7 8 … 2336. Keeps the pager small for thousands of pages.
 */
export function pageWindow(page: number, pages: number, around = 2): (number | null)[] {
  const keep = new Set([1, pages]);
  for (let p = page - around; p <= page + around; p++) if (p >= 1 && p <= pages) keep.add(p);
  const sorted = [...keep].sort((a, b) => a - b);
  const out: (number | null)[] = [];
  sorted.forEach((p, i) => {
    if (i > 0 && p - sorted[i - 1] > 1) out.push(p - sorted[i - 1] === 2 ? p - 1 : null);
    out.push(p);
  });
  return out;
}

function PageLink({ href, disabled, label, children }: { href: string; disabled: boolean; label: string; children: React.ReactNode }) {
  if (disabled) {
    return (
      <Button size="icon-sm" variant="ghost" disabled aria-label={label}>
        {children}
      </Button>
    );
  }
  return (
    <Button asChild size="icon-sm" variant="ghost">
      <Link href={href} aria-label={label}>
        {children}
      </Link>
    </Button>
  );
}

export function EmptyResults({ query }: { query: Query }) {
  // No filters and no jobs: the first import hasn't finished yet.
  if (activeCount(query) === 0) {
    return (
      <div className="flex flex-col items-center gap-3 rounded-3xl border border-dashed border-border bg-card/50 px-6 py-16 text-center">
        <span className="flex size-12 items-center justify-center rounded-2xl bg-primary/12 text-primary">
          <SearchX className="size-5" />
        </span>
        <h2 className="text-base font-semibold">No jobs yet</h2>
        <p className="max-w-sm text-sm text-muted-foreground">
          The worker is fetching jobs from company job boards. The first import takes a few minutes;
          make sure it&apos;s running (<code className="rounded bg-muted px-1.5 py-0.5">./scripts/dev.sh dev</code>).
        </p>
        <Button asChild variant="flat" size="sm" shape="pill">
          <Link href="/settings#sources">See job sources</Link>
        </Button>
      </div>
    );
  }
  return (
    <div className="flex flex-col items-center gap-3 rounded-3xl border border-dashed border-border bg-card/50 px-6 py-16 text-center">
      <span className="flex size-12 items-center justify-center rounded-2xl bg-muted text-muted-foreground">
        <SearchX className="size-5" />
      </span>
      <h2 className="text-base font-semibold">No jobs match these filters</h2>
      <p className="max-w-sm text-sm text-muted-foreground">
        Try removing a filter or widening the date range.
      </p>
      <Button asChild variant="flat" size="sm" shape="pill">
        <Link href={toHref(clearFilters(query))}>Clear filters</Link>
      </Button>
    </div>
  );
}

export function ApiError({ message }: { message: string }) {
  return (
    <div className="flex flex-col items-center gap-3 rounded-3xl border border-destructive/30 bg-destructive/5 px-6 py-16 text-center">
      <span className="flex size-12 items-center justify-center rounded-2xl bg-destructive/12 text-destructive">
        <ServerCrash className="size-5" />
      </span>
      <h2 className="text-base font-semibold">Couldn&apos;t load jobs</h2>
      <p className="max-w-md text-sm text-muted-foreground">{message}</p>
      <p className="text-xs text-muted-foreground">
        Is the API running? Start it with <code className="rounded bg-muted px-1.5 py-0.5">./scripts/dev.sh dev</code>
      </p>
    </div>
  );
}
