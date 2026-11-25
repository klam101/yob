import { redirect } from "next/navigation";
import { Suspense } from "react";
import { serverApi } from "@/api/server";
import { ApiError } from "@/components/jobs/feed-states";
import { Wizard } from "@/components/preferences/wizard";
import { Skeleton } from "@/components/ui/skeleton";

export const metadata = { title: "Set up your search" };

/** The first-run questionnaire. Outside the app shell, like the login page. */
export default function OnboardingPage() {
  return (
    <main className="min-h-dvh bg-muted/40 px-4 py-8 sm:py-14">
      <div className="mx-auto w-full max-w-2xl space-y-8">
        <div className="space-y-4">
          <span className="flex size-10 items-center justify-center rounded-xl bg-primary text-base font-bold text-primary-foreground shadow-soft">
            y
          </span>
          <div className="space-y-1">
            <h1 className="text-2xl font-semibold tracking-tight sm:text-3xl">What are you looking for?</h1>
            <p className="text-muted-foreground">
              Your answers become the job feed&apos;s default filters. Skip anything; you can change it all later
              under Profile → Job preferences.
            </p>
          </div>
        </div>
        <Suspense fallback={<Skeleton className="h-[28rem] rounded-3xl" />}>
          <LoadedWizard />
        </Suspense>
      </div>
    </main>
  );
}

async function LoadedWizard() {
  const api = await serverApi();
  const res = await api.GET("/preferences").catch(() => null);
  if (res?.response.status === 401) redirect("/login?next=/onboarding");
  if (!res?.data) return <ApiError message="Couldn't load your preferences." />;
  return <Wizard initial={res.data.preferences} />;
}
