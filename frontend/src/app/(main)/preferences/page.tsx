import { redirect } from "next/navigation";
import { Suspense } from "react";
import { serverApi } from "@/api/server";
import { ApiError } from "@/components/jobs/feed-states";
import { PreferencesForm } from "@/components/preferences/preferences-form";
import { Skeleton } from "@/components/ui/skeleton";

export const metadata = { title: "Job preferences" };

export default function PreferencesPage() {
  return (
    <div className="mx-auto w-full max-w-6xl space-y-6 px-4 py-6 sm:px-6 md:px-10 md:py-10">
      <header className="space-y-1">
        <h1 className="text-2xl font-semibold tracking-tight sm:text-3xl">Job preferences</h1>
        <p className="text-muted-foreground">
          What you&apos;re looking for. These are the job feed&apos;s default filters; your resume never changes
          them.
        </p>
      </header>
      <Suspense fallback={<Skeleton className="h-[40rem] rounded-3xl" />}>
        <LoadedPreferences />
      </Suspense>
    </div>
  );
}

async function LoadedPreferences() {
  const api = await serverApi();
  const res = await api.GET("/preferences").catch(() => null);
  if (res?.response.status === 401) redirect("/login?next=/preferences");
  if (!res?.data) return <ApiError message="Couldn't load your preferences." />;
  return <PreferencesForm initial={res.data.preferences} />;
}
