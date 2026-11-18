"use client";

import { useEffect, useState } from "react";
import { api } from "@/api/client";

type Status = "checking" | "ok" | "degraded" | "offline";

const styles: Record<Status, { dot: string; label: string }> = {
  checking: { dot: "bg-zinc-400", label: "Checking API" },
  ok: { dot: "bg-emerald-500", label: "API connected" },
  degraded: { dot: "bg-amber-500", label: "API degraded" },
  offline: { dot: "bg-red-500", label: "API offline" },
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
    <p className="flex items-center gap-2 text-xs text-zinc-500" role="status">
      <span className={`size-2 rounded-full ${dot}`} aria-hidden />
      {label}
    </p>
  );
}
