"use client";

import { Search } from "lucide-react";
import { useEffect, useState } from "react";
import { api } from "@/api/client";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "cn";
import { type Choice, CheckList } from "./controls";

/**
 * Company filter. Shows the companies with the most jobs, and searches all
 * of them as you type (there can be thousands). Selected companies stay at
 * the top of the list.
 */
export function CompanyPicker({
  top,
  selected,
  onToggle,
  className,
  autoFocus,
}: {
  top: Choice[];
  selected: string[];
  onToggle: (value: string) => void;
  className?: string;
  autoFocus?: boolean;
}) {
  const [search, setSearch] = useState("");
  const [results, setResults] = useState<Choice[] | null>(null);
  const [loading, setLoading] = useState(false);
  const q = search.trim();

  useEffect(() => {
    if (!q) return;
    let cancelled = false;
    const timer = setTimeout(async () => {
      setLoading(true);
      const { data } = await api.GET("/jobs/companies", { params: { query: { q } } }).catch(() => ({ data: undefined }));
      if (cancelled) return;
      setResults((data ?? []).map((c) => ({ value: c.value, label: c.value, count: c.count })));
      setLoading(false);
    }, 200);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [q]);

  const shown = q ? (results ?? []) : top;
  // Selected companies first, then the rest without repeating them.
  const isSelected = (v: string) => selected.some((s) => s.toLowerCase() === v.toLowerCase());
  const pinned: Choice[] = selected.map(
    (s) => shown.find((c) => c.value.toLowerCase() === s.toLowerCase()) ??
      top.find((c) => c.value.toLowerCase() === s.toLowerCase()) ?? { value: s, label: s },
  );
  const options = [...pinned, ...shown.filter((c) => !isSelected(c.value))];

  return (
    <div className={cn("space-y-1.5", className)}>
      <label className="flex h-9 items-center gap-2 rounded-lg bg-muted px-2.5">
        <Search className="size-4 text-muted-foreground" />
        <span className="sr-only">Search companies</span>
        <input
          value={search}
          onChange={(e) => {
            setSearch(e.target.value);
            if (!e.target.value.trim()) setResults(null);
          }}
          placeholder="Search all companies"
          autoFocus={autoFocus}
          className="w-full bg-transparent text-sm outline-none placeholder:text-muted-foreground"
        />
      </label>
      <ScrollArea className="max-h-72">
        <CheckList
          options={options}
          selected={selected}
          onToggle={onToggle}
        />
        {q && !loading && results?.length === 0 && (
          <p className="px-2 py-3 text-sm text-muted-foreground">No companies match “{q}”.</p>
        )}
        {!q && top.length > 0 && (
          <p className="px-2 pt-1 pb-2 text-xs text-muted-foreground">
            Showing the {top.length} companies with the most jobs. Search to find others.
          </p>
        )}
      </ScrollArea>
    </div>
  );
}
