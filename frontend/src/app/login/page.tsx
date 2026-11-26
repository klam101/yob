import { redirect } from "next/navigation";
import { Suspense } from "react";
import { serverApi } from "@/api/server";
import { LoginForm } from "@/components/auth/login-form";
import { Logo } from "@/components/shell/logo";
import { Skeleton } from "@/components/ui/skeleton";

export const metadata = { title: "Log in" };

/** Only same-site paths, so `?next=` can't send you to another website. */
function safeNext(next: string | string[] | undefined) {
  return typeof next === "string" && next.startsWith("/") && !next.startsWith("//") ? next : "/jobs";
}

export default function LoginPage({ searchParams }: PageProps<"/login">) {
  return (
    <main className="flex min-h-dvh items-center justify-center bg-muted/40 px-4 py-10">
      <div className="w-full max-w-sm space-y-6">
        <div className="flex justify-center">
          <Logo height={48} />
        </div>
        <Suspense fallback={<Skeleton className="h-80 rounded-3xl" />}>
          <Login searchParams={searchParams} />
        </Suspense>
      </div>
    </main>
  );
}

async function Login({ searchParams }: Pick<PageProps<"/login">, "searchParams">) {
  const next = safeNext((await searchParams).next);
  const api = await serverApi();
  const { data } = await api.GET("/auth/status").catch(() => ({ data: undefined }));
  if (data?.user) redirect(next);
  return <LoginForm mode={data?.account_exists === false ? "signup" : "login"} next={next} apiDown={!data} />;
}
