"use client";

import { ChevronDown, X } from "lucide-react";
import type { Facets } from "@/api/client";
import { Button } from "@/components/ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Switch } from "@/components/ui/switch";
import {
  EMPLOYMENT,
  type FilterKey,
  POSTED_WITHIN,
  SALARY_MIN,
  SENIORITY,
  SOURCE,
  activeCount,
  clearFilters,
  labelOf,
  listOf,
  toggleInList,
  withValue,
} from "@/lib/filters";
import { cn } from "cn";
import { AllFiltersDialog } from "./all-filters-dialog";
import { type Choice, CheckList, RadioList, facetChoices } from "./controls";
import { useFeed } from "./feed-context";

/** hiringcafe-style row of filter chips; each opens a small popover. */
export function FilterChips({ facets }: { facets: Facets }) {
  const { query, apply } = useFeed();
  const narrowing = activeCount(query);

  return (
    <div className="flex flex-wrap items-center gap-2">
      <AllFiltersDialog facets={facets} />

      <span aria-hidden className="mx-0.5 h-5 w-px bg-border" />

      <RadioChip
        label="Date posted"
        filterKey="posted_within"
        options={POSTED_WITHIN}
        anyLabel="Any time"
      />
      <CheckChip label="Experience" filterKey="seniority" options={SENIORITY} />
      <CheckChip label="Job type" filterKey="employment_type" options={EMPLOYMENT} />
      <RadioChip label="Salary" filterKey="salary_min" options={SALARY_MIN} anyLabel="Any salary" />
      <CheckChip label="Department" filterKey="department" options={facetChoices(facets.departments)} scroll />
      <CheckChip label="Company" filterKey="company" options={facetChoices(facets.companies)} scroll />
      <CheckChip label="Source" filterKey="source" options={SOURCE} />

      <label className="flex h-8 cursor-pointer items-center gap-2 rounded-full border border-border bg-card px-3 text-[0.8125rem] font-medium transition-colors hover:bg-muted">
        <Switch
          checked={query.hide_stale === "true"}
          onCheckedChange={(on) => apply(withValue(query, "hide_stale", on ? "true" : undefined))}
          className="scale-90"
        />
        Hide stale
      </label>

      {narrowing > 0 && (
        <Button variant="light" size="sm" shape="pill" onClick={() => apply(clearFilters(query))}>
          <X /> Clear filters
        </Button>
      )}
    </div>
  );
}

function ChipTrigger({ label, summary, active }: { label: string; summary?: string; active: boolean }) {
  return (
    <PopoverTrigger asChild>
      <Button
        variant={active ? "flat" : "bordered"}
        size="sm"
        shape="pill"
        className={cn("max-w-56", active && "border-primary/25")}
      >
        <span className="truncate">{summary ? `${label}: ${summary}` : label}</span>
        <ChevronDown className="opacity-60" />
      </Button>
    </PopoverTrigger>
  );
}

function CheckChip({
  label,
  filterKey,
  options,
  scroll,
}: {
  label: string;
  filterKey: FilterKey;
  options: Choice[];
  scroll?: boolean;
}) {
  const { query, apply } = useFeed();
  const selected = listOf(query, filterKey);
  const summary =
    selected.length === 1
      ? (options.find((o) => o.value === selected[0])?.label ?? selected[0])
      : selected.length > 1
        ? `${selected.length} selected`
        : undefined;

  const list = (
    <CheckList
      options={options}
      selected={selected}
      onToggle={(value) => apply(toggleInList(query, filterKey, value))}
    />
  );

  return (
    <Popover>
      <ChipTrigger label={label} summary={summary} active={selected.length > 0} />
      <PopoverContent align="start" className="w-64 p-1.5">
        <ChipHeader label={label} onClear={selected.length ? () => apply(withValue(query, filterKey, undefined)) : undefined} />
        {scroll ? <ScrollArea className="max-h-72">{list}</ScrollArea> : list}
      </PopoverContent>
    </Popover>
  );
}

function RadioChip({
  label,
  filterKey,
  options,
  anyLabel,
}: {
  label: string;
  filterKey: FilterKey;
  options: Choice[];
  anyLabel: string;
}) {
  const { query, apply } = useFeed();
  const value = query[filterKey];
  return (
    <Popover>
      <ChipTrigger label={label} summary={labelOf(options, value)} active={!!value} />
      <PopoverContent align="start" className="w-60 p-1.5">
        <ChipHeader label={label} />
        <RadioList
          options={options}
          value={value}
          anyLabel={anyLabel}
          onChange={(v) => apply(withValue(query, filterKey, v))}
        />
      </PopoverContent>
    </Popover>
  );
}

function ChipHeader({ label, onClear }: { label: string; onClear?: () => void }) {
  return (
    <div className="flex items-center justify-between px-2 pt-1 pb-1.5">
      <span className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">{label}</span>
      {onClear && (
        <button type="button" onClick={onClear} className="text-xs font-medium text-primary hover:underline">
          Clear
        </button>
      )}
    </div>
  );
}

