import { SearchX } from "lucide-react";
import Link from "next/link";
import { Button } from "@/components/ui/button";

export default function JobNotFound() {
  return (
    <div className="mx-auto flex max-w-md flex-col items-center gap-3 px-4 py-24 text-center">
      <span className="flex size-12 items-center justify-center rounded-2xl bg-muted text-muted-foreground">
        <SearchX className="size-5" />
      </span>
      <h1 className="text-xl font-semibold">Job not found</h1>
      <p className="text-sm text-muted-foreground">
        It may have closed and been removed from the company&apos;s careers page.
      </p>
      <Button asChild variant="flat" shape="pill">
        <Link href="/jobs">Back to jobs</Link>
      </Button>
    </div>
  );
}
