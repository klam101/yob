import { notFound } from "next/navigation";
import { Suspense } from "react";
import { api } from "@/api/client";
import { ApiError } from "@/components/jobs/feed-states";
import { JobDetailView } from "@/components/jobs/job-detail";
import { Skeleton } from "@/components/ui/skeleton";

export default function JobPage(props: PageProps<"/jobs/[id]">) {
  return (
    <div className="mx-auto w-full max-w-6xl px-4 py-6 sm:px-6 md:px-10 md:py-8">
      <Suspense fallback={<DetailSkeleton />}>
        <JobLoader params={props.params} />
      </Suspense>
    </div>
  );
}

async function JobLoader({ params }: Pick<PageProps<"/jobs/[id]">, "params">) {
  const { id } = await params;

  let result;
  try {
    result = await api.GET("/jobs/{id}", { params: { path: { id } } });
  } catch (e) {
    return <ApiError message={e instanceof Error ? e.message : "The API didn't respond."} />;
  }
  if (result.response.status === 404) notFound();
  const detail = result.data;
  if (!detail) return <ApiError message={result.error?.message ?? "Unexpected response from the API."} />;

  // Other open roles at the same company, for the Company tab.
  const { data: related } = await api
    .GET("/jobs", { params: { query: { company: detail.job.company.name, page_size: 8 } } })
    .catch(() => ({ data: undefined }));

  return (
    <>
      <title>{`${detail.job.title} at ${detail.job.company.name} · yob`}</title>
      <JobDetailView detail={detail} related={(related?.items ?? []).filter((j) => j.id !== id)} />
    </>
  );
}

function DetailSkeleton() {
  return (
    <div className="space-y-5" aria-busy>
      <Skeleton className="h-8 w-36 rounded-full" />
      <div className="grid gap-5 lg:grid-cols-[minmax(0,1fr)_20rem]">
        <div className="space-y-5">
          <Skeleton className="h-80 rounded-3xl" />
          <Skeleton className="h-11 w-48 rounded-2xl" />
          <Skeleton className="h-56 rounded-3xl" />
        </div>
        <div className="space-y-5">
          <Skeleton className="h-64 rounded-3xl" />
          <Skeleton className="h-52 rounded-3xl" />
        </div>
      </div>
    </div>
  );
}
