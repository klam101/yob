import { Suspense } from "react";
import { api } from "@/api/client";
import { FeedProvider, PendingArea } from "@/components/filters/feed-context";
import { FilterChips } from "@/components/filters/filter-chips";
import { ResultsHeader } from "@/components/filters/results-header";
import { SearchBar } from "@/components/filters/search-bar";
import { ApiError, EmptyResults, Pagination } from "@/components/jobs/feed-states";
import { JobRow, JobRowSkeleton } from "@/components/jobs/job-row";
import { Skeleton } from "@/components/ui/skeleton";
import { readQuery, toApiQuery } from "@/lib/filters";

export const metadata = { title: "Jobs" };

const PAGE_SIZE = 10;

export default function JobsPage(props: PageProps<"/jobs">) {
  return (
    <div className="mx-auto w-full max-w-6xl space-y-6 px-4 py-6 sm:px-6 md:px-10 md:py-10">
      <header className="space-y-1">
        <h1 className="text-2xl font-semibold tracking-tight sm:text-3xl">Jobs</h1>
        <p className="text-muted-foreground">Fresh roles from company career pages, ranked for you.</p>
      </header>
      {/* The query string is only known at request time, so the feed streams in. */}
      <Suspense fallback={<FeedSkeleton />}>
        <Feed searchParams={props.searchParams} />
      </Suspense>
    </div>
  );
}

async function Feed({ searchParams }: Pick<PageProps<"/jobs">, "searchParams">) {
  const query = readQuery(await searchParams);

  let result;
  try {
    result = await api.GET("/jobs", { params: { query: toApiQuery(query, PAGE_SIZE) } });
  } catch (e) {
    return <ApiError message={e instanceof Error ? e.message : "The API didn't respond."} />;
  }
  const { data, error } = result;
  if (!data) return <ApiError message={error?.message ?? "Unexpected response from the API."} />;

  return (
    <FeedProvider query={query}>
      <div className="space-y-4">
        <SearchBar key={`${query.q ?? ""}|${query.location ?? ""}`} />
        <FilterChips facets={data.facets} />
      </div>
      <PendingArea className="space-y-4">
        <ResultsHeader total={data.total} companies={data.company_count} />
        {data.items.length === 0 ? (
          <EmptyResults query={query} />
        ) : (
          <ul className="space-y-3">
            {data.items.map((job) => (
              <li key={job.id}>
                <JobRow job={job} />
              </li>
            ))}
          </ul>
        )}
        <Pagination query={query} page={data.page} pageSize={data.page_size} total={data.total} />
      </PendingArea>
    </FeedProvider>
  );
}

function FeedSkeleton() {
  return (
    <div className="space-y-4" aria-busy>
      <Skeleton className="h-15 rounded-3xl" />
      <div className="flex flex-wrap gap-2">
        {Array.from({ length: 8 }, (_, i) => (
          <Skeleton key={i} className="h-8 w-28 rounded-full" />
        ))}
      </div>
      <Skeleton className="h-5 w-40" />
      <div className="space-y-3">
        {Array.from({ length: 4 }, (_, i) => (
          <JobRowSkeleton key={i} />
        ))}
      </div>
    </div>
  );
}
