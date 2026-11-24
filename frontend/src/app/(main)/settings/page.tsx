import { Suspense } from "react";
import { type DiscoveryStats, type FeedStatus, api } from "@/api/client";
import { ApiError } from "@/components/jobs/feed-states";
import { AddCompanyForm } from "@/components/sources/add-company-form";
import { BulkAddForm } from "@/components/sources/bulk-add-form";
import { JobBoardsTable } from "@/components/sources/job-boards";
import { SourcesTable } from "@/components/sources/sources-table";
import { Skeleton } from "@/components/ui/skeleton";
import { timeAgo } from "@/lib/format";

export const metadata = { title: "Settings" };

export default function SettingsPage() {
  return (
    <div className="mx-auto w-full max-w-6xl space-y-6 px-4 py-6 sm:px-6 md:px-10 md:py-10">
      <header className="space-y-1">
        <h1 className="text-2xl font-semibold tracking-tight sm:text-3xl">Settings</h1>
        <p className="text-muted-foreground">Job sources now; account and AI models later.</p>
      </header>

      <section id="sources" className="space-y-5 rounded-3xl border border-border/70 bg-card p-5 shadow-soft sm:p-7">
        <div className="space-y-1">
          <h2 className="text-lg font-semibold">Job sources</h2>
          <p className="text-sm text-muted-foreground">
            Companies whose job boards are checked in the background. Active companies are checked every
            15 minutes, quiet ones every 6 to 24 hours. Jobs are stored locally, so the feed never waits
            on these sites. Only jobs in the US or remote are shown.
          </p>
        </div>
        <AddCompanyForm />
        <Suspense fallback={<Skeleton className="h-96 rounded-2xl" />}>
          <Sources />
        </Suspense>
      </section>

      <section id="boards" className="space-y-5 rounded-3xl border border-border/70 bg-card p-5 shadow-soft sm:p-7">
        <div className="space-y-1">
          <h2 className="text-lg font-semibold">Job boards</h2>
          <p className="text-sm text-muted-foreground">
            Public job board APIs, read a few times a day. Most of their listings are jobs we already have from
            the employer&apos;s own board, so those are matched up and the employer&apos;s copy is shown. Listings
            only a board has are added, credited to the board with a link back. Only jobs open to US workers are
            kept.
          </p>
        </div>
        <Suspense fallback={<Skeleton className="h-48 rounded-2xl" />}>
          <JobBoards />
        </Suspense>
      </section>

      <section id="discovery" className="space-y-5 rounded-3xl border border-border/70 bg-card p-5 shadow-soft sm:p-7">
        <div className="space-y-1">
          <h2 className="text-lg font-semibold">Company discovery</h2>
          <p className="text-sm text-muted-foreground">
            New job boards are found automatically: from the public Common Crawl web index (read slowly
            in the background) and from board links inside job postings. Each one is checked once and
            added if it has open jobs; empty boards get another look after 30 days.
          </p>
        </div>
        <Suspense fallback={<Skeleton className="h-24 rounded-2xl" />}>
          <Discovery />
        </Suspense>
        <div className="space-y-2">
          <h3 className="text-sm font-medium">Add many boards at once</h3>
          <BulkAddForm />
        </div>
      </section>
    </div>
  );
}

async function Sources() {
  let result;
  try {
    result = await api.GET("/companies", { params: { query: { page_size: 50 } } });
  } catch (e) {
    return <ApiError message={e instanceof Error ? e.message : "The API didn't respond."} />;
  }
  if (!result.data) return <ApiError message="Unexpected response from the API." />;

  const { summary } = result.data;
  return (
    <div className="space-y-4">
      <Stats
        items={[
          { label: "Companies", value: summary.companies.toLocaleString() },
          { label: "Open jobs (US + remote)", value: summary.open_jobs.toLocaleString() },
          { label: "With errors", value: summary.errors.toLocaleString() },
        ]}
      />
      {/* Remounts after "Add company" refreshes the page with a new count. */}
      <SourcesTable key={summary.companies} initial={result.data} />
    </div>
  );
}

async function JobBoards() {
  let feeds: FeedStatus[] | undefined;
  try {
    feeds = (await api.GET("/feeds")).data;
  } catch {
    // Shown as unavailable below.
  }
  if (!feeds) return <ApiError message="Couldn't load job boards." />;
  return <JobBoardsTable feeds={feeds} />;
}

async function Discovery() {
  let stats: DiscoveryStats | undefined;
  try {
    stats = (await api.GET("/discovery")).data;
  } catch {
    // Shown as unavailable below.
  }
  if (!stats) return <ApiError message="Couldn't load discovery progress." />;

  const index =
    stats.index_pages_total != null
      ? `${stats.index_pages_read.toLocaleString()} of ${stats.index_pages_total.toLocaleString()} pages`
      : stats.index_pages_read > 0
        ? `${stats.index_pages_read.toLocaleString()} pages`
        : "Waiting for the index";

  return (
    <div className="space-y-2">
      <Stats
        items={[
          { label: "Waiting to check", value: stats.pending.toLocaleString() },
          { label: "Added", value: stats.added.toLocaleString(), note: `${stats.added_last_week.toLocaleString()} this week` },
          { label: "Not added", value: stats.rejected.toLocaleString(), note: "Missing or no open jobs" },
          { label: "Common Crawl", value: index },
        ]}
      />
      {stats.last_index_read_at && (
        <p className="text-xs text-muted-foreground" suppressHydrationWarning>
          Index last read {timeAgo(stats.last_index_read_at)}. It&apos;s often overloaded; discovery retries
          and resumes where it stopped.
        </p>
      )}
    </div>
  );
}

function Stats({ items }: { items: { label: string; value: string; note?: string }[] }) {
  return (
    <dl className="grid grid-cols-2 gap-3 sm:grid-cols-[repeat(auto-fit,minmax(10rem,1fr))]">
      {items.map((s) => (
        <div key={s.label} className="rounded-2xl bg-muted/60 px-4 py-3">
          <dt className="text-xs text-muted-foreground">{s.label}</dt>
          <dd className="text-xl font-semibold tabular-nums">{s.value}</dd>
          {s.note && <dd className="text-xs text-muted-foreground">{s.note}</dd>}
        </div>
      ))}
    </dl>
  );
}
