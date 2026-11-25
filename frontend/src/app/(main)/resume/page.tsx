import { redirect } from "next/navigation";
import { Suspense } from "react";
import { serverApi } from "@/api/server";
import { ApiError } from "@/components/jobs/feed-states";
import { ResumeManager } from "@/components/resume/resume-manager";
import { Skeleton } from "@/components/ui/skeleton";

export const metadata = { title: "Resume" };

export default function ResumePage() {
  return (
    <div className="mx-auto w-full max-w-6xl space-y-6 px-4 py-6 sm:px-6 md:px-10 md:py-10">
      <header className="space-y-1">
        <h1 className="text-2xl font-semibold tracking-tight sm:text-3xl">Resume</h1>
        <p className="text-muted-foreground">
          Your uploaded resumes and the projects tailored versions are built from.
        </p>
      </header>
      <Suspense fallback={<Skeleton className="h-[28rem] rounded-3xl" />}>
        <LoadedResumes />
      </Suspense>
    </div>
  );
}

async function LoadedResumes() {
  const api = await serverApi();
  const [resumes, profile] = await Promise.all([
    api.GET("/profile/resumes").catch(() => null),
    api.GET("/profile").catch(() => null),
  ]);
  if (resumes?.response.status === 401 || profile?.response.status === 401) redirect("/login?next=/resume");
  if (!resumes?.data || !profile?.data) return <ApiError message="Couldn't load your resumes." />;
  return <ResumeManager initial={resumes.data} profile={profile.data} />;
}
