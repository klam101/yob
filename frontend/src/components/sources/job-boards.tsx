import { ArrowUpRight } from "lucide-react";
import type { Feed, FeedStatus } from "@/api/client";
import { Badge } from "@/components/ui/badge";
import { sourceLabel, timeAgo, timeUntil } from "@/lib/format";

const HOME: Record<Feed, string> = {
  himalayas: "https://himalayas.app/jobs",
  remotive: "https://remotive.com",
  remoteok: "https://remoteok.com",
  usajobs: "https://www.usajobs.gov",
};

/** Status of each job board: jobs it added, listings matched to jobs we already had, and check times. */
export function JobBoardsTable({ feeds }: { feeds: FeedStatus[] }) {
  return (
    <div className="relative overflow-x-auto rounded-2xl border border-border/70">
      <table className="w-full min-w-[42rem] text-sm">
        <thead className="bg-muted/60 text-left text-xs text-muted-foreground">
          <tr>
            <th className="px-4 py-2.5 font-medium">Board</th>
            <th className="px-4 py-2.5 text-right font-medium">Jobs only here</th>
            <th className="px-4 py-2.5 text-right font-medium">Already had</th>
            <th className="px-4 py-2.5 font-medium">Last checked</th>
            <th className="px-4 py-2.5 font-medium">Next check</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-border/60">
          {feeds.map((f) => (
            <tr key={f.id} className="align-top">
              <td className="px-4 py-2.5">
                <a
                  href={HOME[f.id]}
                  target="_blank"
                  rel="noopener"
                  className="inline-flex items-center gap-1 font-medium hover:text-primary"
                >
                  {sourceLabel(f.id)} <ArrowUpRight className="size-3.5 text-muted-foreground" />
                </a>
              </td>
              {f.enabled ? (
                <>
                  <td className="px-4 py-2.5 text-right tabular-nums">{f.own_jobs.toLocaleString()}</td>
                  <td className="px-4 py-2.5 text-right tabular-nums">{f.matched.toLocaleString()}</td>
                  <td className="px-4 py-2.5">
                    <LastCheck feed={f} />
                  </td>
                  <td className="px-4 py-2.5 text-muted-foreground" suppressHydrationWarning>
                    {timeUntil(f.next_check_at)}
                    {f.passing && <span className="block text-xs">Full pass in progress</span>}
                  </td>
                </>
              ) : (
                <td colSpan={4} className="px-4 py-2.5 text-muted-foreground">
                  Off. Add a free API key to <code className="text-xs">.env</code> to turn it on (see{" "}
                  <code className="text-xs">.env.example</code>).
                </td>
              )}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function LastCheck({ feed: f }: { feed: FeedStatus }) {
  if (!f.last_status || !f.last_checked_at) return <span className="text-muted-foreground">Not yet</span>;
  if (f.last_status === "error") {
    return (
      <div className="space-y-1">
        <Badge variant="destructive" size="sm">
          <span suppressHydrationWarning>Failed {timeAgo(f.last_checked_at)}</span>
        </Badge>
        {f.last_error && <p className="max-w-xs text-xs text-muted-foreground">{f.last_error}</p>}
      </div>
    );
  }
  return <span suppressHydrationWarning>{timeAgo(f.last_checked_at)}</span>;
}
