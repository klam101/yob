import { Suspense } from "react";
import { api } from "@/api/client";
import { ApiError } from "@/components/jobs/feed-states";
import { AddCompanyForm } from "@/components/sources/add-company-form";
import { SourcesTable } from "@/components/sources/sources-table";
import { Skeleton } from "@/components/ui/skeleton";

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
            on these sites.
          </p>
        </div>
        <AddCompanyForm />
        <Suspense fallback={<Skeleton className="h-96 rounded-2xl" />}>
          <Sources />
        </Suspense>
      </section>
    </div>
  );
}

async function Sources() {
  let result;
  try {
    result = await api.GET("/companies");
  } catch (e) {
    return <ApiError message={e instanceof Error ? e.message : "The API didn't respond."} />;
  }
  if (!result.data) return <ApiError message="Unexpected response from the API." />;

  const companies = result.data;
  const openJobs = companies.reduce((n, c) => n + c.open_jobs, 0);
  const stats = [
    { label: "Companies", value: companies.length.toLocaleString() },
    { label: "Open jobs", value: openJobs.toLocaleString() },
    { label: "With errors", value: companies.filter((c) => c.last_status === "error").length.toString() },
  ];

  return (
    <div className="space-y-4">
      <dl className="grid grid-cols-3 gap-3">
        {stats.map((s) => (
          <div key={s.label} className="rounded-2xl bg-muted/60 px-4 py-3">
            <dt className="text-xs text-muted-foreground">{s.label}</dt>
            <dd className="text-xl font-semibold tabular-nums">{s.value}</dd>
          </div>
        ))}
      </dl>
      <SourcesTable companies={companies} />
    </div>
  );
}
