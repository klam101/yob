import { notFound, redirect } from "next/navigation";
import { Suspense } from "react";
import { serverApi } from "@/api/server";
import { ApiError } from "@/components/jobs/feed-states";
import { ResumeEditor } from "@/components/resume/resume-editor";
import { Skeleton } from "@/components/ui/skeleton";

export const metadata = { title: "Edit resume" };

type Props = {
  params: Promise<{ id: string }>;
  searchParams: Promise<{ update?: string }>;
};

/** The resume edit page. `?update=1` opens "Update profile" (after an upload). */
export default function ResumeEditPage({ params, searchParams }: Props) {
  return (
    <div className="mx-auto w-full max-w-6xl px-4 pb-24 sm:px-6 md:px-10 md:pb-10">
      <Suspense fallback={<Skeleton className="mt-6 h-[40rem] rounded-3xl" />}>
        <LoadedResume params={params} searchParams={searchParams} />
      </Suspense>
    </div>
  );
}

async function LoadedResume({ params, searchParams }: Props) {
  const [{ id }, { update }] = await Promise.all([params, searchParams]);
  const api = await serverApi();
  const r = await api.GET("/profile/resumes/{id}", { params: { path: { id } } }).catch(() => null);
  if (r?.response.status === 401) redirect(`/login?next=/resume/edit/${id}`);
  if (r?.response.status === 404) notFound();
  if (!r?.data) return <ApiError message="Couldn't load this resume." />;
  return <ResumeEditor key={id} initial={r.data} openUpdate={update === "1"} />;
}
