"use client";

import { ArrowDownUp, Check } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { SORT, labelOf, withValue } from "@/lib/filters";
import { useFeed } from "./feed-context";

export function ResultsHeader({ total, companies }: { total: number; companies: number }) {
  const { query, apply, isPending } = useFeed();
  const sort = query.sort ?? "recommended";

  return (
    <div className="flex items-center justify-between gap-3">
      <p className="text-sm text-muted-foreground" aria-live="polite">
        <span className="font-semibold text-foreground tabular-nums">{total}</span> {total === 1 ? "job" : "jobs"}
        {" · "}
        <span className="font-semibold text-foreground tabular-nums">{companies}</span>{" "}
        {companies === 1 ? "company" : "companies"}
        {isPending && <span className="ml-2 animate-pulse">Updating…</span>}
      </p>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="ghost" size="sm" shape="pill">
            <ArrowDownUp /> {labelOf(SORT, sort)}
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" className="w-48">
          {SORT.map((s) => (
            <DropdownMenuItem
              key={s.value}
              onSelect={() => apply(withValue(query, "sort", s.value === "recommended" ? undefined : s.value))}
            >
              <Check className={s.value === sort ? "opacity-100" : "opacity-0"} />
              {s.label}
            </DropdownMenuItem>
          ))}
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}
