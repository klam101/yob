"use client";

import { useEffect, useState } from "react";
import { api } from "@/api/client";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "cn";

type Status = "checking" | "ok" | "degraded" | "offline";

const styles: Record<Status, { dot: string; label: string }> = {
  checking: { dot: "bg-muted-foreground/50", label: "Checking API…" },
  ok: { dot: "bg-success", label: "API connected" },
  degraded: { dot: "bg-warning", label: "API degraded: database or Redis unreachable" },
  offline: { dot: "bg-destructive", label: "API offline" },
};

/** Calls GET /health from the browser, so it checks CORS and the whole chain. */
export function ApiStatus() {
  const [status, setStatus] = useState<Status>("checking");

  useEffect(() => {
    let cancelled = false;
    api
      .GET("/health")
      .then(({ data, error }) => {
        if (cancelled) return;
        const body = data ?? error;
        setStatus(body?.status === "ok" ? "ok" : body ? "degraded" : "offline");
      })
      .catch(() => !cancelled && setStatus("offline"));
    return () => {
      cancelled = true;
    };
  }, []);

  const { dot, label } = styles[status];
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span role="status" aria-label={label} className="flex size-9 items-center justify-center">
          <span className={cn("size-2.5 rounded-full ring-4 ring-background", dot)} />
        </span>
      </TooltipTrigger>
      <TooltipContent side="right">{label}</TooltipContent>
    </Tooltip>
  );
}
