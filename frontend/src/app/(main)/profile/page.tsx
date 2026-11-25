import { redirect } from "next/navigation";
import { Suspense } from "react";
import { serverApi } from "@/api/server";
import { ApiError } from "@/components/jobs/feed-states";
import { ProfileEditor } from "@/components/profile/profile-editor";
import { Skeleton } from "@/components/ui/skeleton";

export const metadata = { title: "Profile" };

export default function ProfilePage() {
  return (
    <div className="mx-auto w-full max-w-6xl space-y-6 px-4 py-6 sm:px-6 md:px-10 md:py-10">
      <header className="space-y-1">
        <h1 className="text-2xl font-semibold tracking-tight sm:text-3xl">Profile</h1>
        <p className="text-muted-foreground">
          The facts your tailored resumes, cover letters and autofill are built from. Only what you enter here is
          used, never anything made up.
        </p>
      </header>
      <Suspense fallback={<Skeleton className="h-[32rem] rounded-3xl" />}>
        <LoadedProfile />
      </Suspense>
    </div>
  );
}

async function LoadedProfile() {
  const api = await serverApi();
  const profile = await api.GET("/profile").catch(() => null);
  if (profile?.response.status === 401) redirect("/login?next=/profile");
  if (!profile?.data) return <ApiError message="Couldn't load your profile." />;
  return <ProfileEditor initial={profile.data} />;
}
