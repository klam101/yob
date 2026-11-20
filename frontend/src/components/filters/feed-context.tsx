"use client";

import { useRouter } from "next/navigation";
import { createContext, useCallback, useContext, useMemo, useTransition } from "react";
import { type Query, toHref } from "@/lib/filters";
import { cn } from "cn";

type Feed = {
  /** The filters currently applied (from the URL, read on the server). */
  query: Query;
  /** True while the server renders results for a new query. */
  isPending: boolean;
  apply: (next: Query) => void;
};

const FeedContext = createContext<Feed | null>(null);

export function FeedProvider({ query, children }: { query: Query; children: React.ReactNode }) {
  const router = useRouter();
  const [isPending, startTransition] = useTransition();

  const apply = useCallback(
    (next: Query) => {
      // Inside a transition, the current results stay visible until the new
      // ones are ready instead of flashing a loading state.
      startTransition(() => router.replace(toHref(next), { scroll: false }));
    },
    [router],
  );

  const value = useMemo(() => ({ query, isPending, apply }), [query, isPending, apply]);
  return <FeedContext value={value}>{children}</FeedContext>;
}

export function useFeed(): Feed {
  const feed = useContext(FeedContext);
  if (!feed) throw new Error("useFeed must be used inside <FeedProvider>");
  return feed;
}

/** Dims its children while new results load. */
export function PendingArea({ children, className }: { children: React.ReactNode; className?: string }) {
  const { isPending } = useFeed();
  return (
    <div
      aria-busy={isPending}
      className={cn("transition-opacity duration-200", isPending && "pointer-events-none opacity-55", className)}
    >
      {children}
    </div>
  );
}
