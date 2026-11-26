"use client";

import { ChevronDown, PencilLine, Sparkles, X } from "lucide-react";
import Link from "next/link";
import { useState } from "react";
import type { Facets } from "@/api/client";
import { Button } from "@/components/ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Switch } from "@/components/ui/switch";
import {
  EMPLOYMENT,
  type FilterKey,
  POSTED_WITHIN,
  POSTED_WITHIN_SHORT,
  SENIORITY,
  SOURCE,
  activeCount,
  clearFilters,
  labelOf,
  listOf,
  salaryLabel,
  salaryOptions,
  sameFilters,
  toggleInList,
  withValue,
} from "@/lib/filters";
import { cn } from "cn";
import { AllFiltersDialog } from "./all-filters-dialog";
import { CompanyPicker } from "./company-picker";
import { FunctionPicker, useVocabulary } from "./detail-sections";
import { type Choice, CheckList, RadioList, facetChoices } from "./controls";
import { useFeed } from "./feed-context";

/** hiringcafe-style row of filter chips; each opens a small popover. */
export function FilterChips({ facets }: { facets: Facets }) {
  const { query, apply } = useFeed();
  const narrowing = activeCount(query);

  return (
    <div className="flex flex-wrap items-center gap-2">
      <PreferencesChip />
      <AllFiltersDialog facets={facets} />

      <span aria-hidden className="mx-0.5 h-5 w-px bg-border" />

      <RadioChip
        label="Date posted"
        filterKey="posted_within"
        options={POSTED_WITHIN_SHORT}
        anyLabel="Any time"
      />
      <CheckChip label="Experience" filterKey="seniority" options={SENIORITY} />
      <CheckChip label="Job type" filterKey="employment_type" options={EMPLOYMENT} />
      <RadioChip
        label="Salary"
        filterKey="salary_min"
        options={salaryOptions(query.salary_min)}
        anyLabel="Any salary"
      />
      <FunctionChip />
      <CompanyChip top={facetChoices(facets.companies)} />
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

/**
 * Your job preferences as one chip: highlighted while they're applied,
 * one click to go back to them after changing filters.
 */
function PreferencesChip() {
  const { query, defaults, apply } = useFeed();
  if (activeCount(defaults) === 0) {
    return (
      <Button asChild variant="bordered" size="sm" shape="pill">
        <Link href="/preferences">
          <Sparkles /> Set job preferences
        </Link>
      </Button>
    );
  }
  const applied = sameFilters(query, defaults);
  return (
    <div className="flex items-center">
      <Button
        variant={applied ? "flat" : "bordered"}
        size="sm"
        shape="pill"
        aria-pressed={applied}
        onClick={() => apply(query.sort ? { ...defaults, sort: query.sort } : defaults)}
        className={cn("rounded-r-none pr-2.5", applied && "border-primary/25")}
      >
        <Sparkles /> {applied ? "My preferences" : "Back to my preferences"}
      </Button>
      <Button
        asChild
        variant={applied ? "flat" : "bordered"}
        size="sm"
        shape="pill"
        className={cn("-ml-px rounded-l-none px-2.5", applied && "border-primary/25")}
      >
        <Link href="/preferences" aria-label="Edit job preferences">
          <PencilLine />
        </Link>
      </Button>
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

/** Job functions, grouped, in a wide popover. */
function FunctionChip() {
  const { query, apply } = useFeed();
  const [open, setOpen] = useState(false);
  const vocab = useVocabulary(open || listOf(query, "function").length > 0);
  const selected = listOf(query, "function");
  const first = vocab.functions.find((f) => f.value === selected[0])?.label ?? selected[0];
  const summary = selected.length === 1 ? first : selected.length > 1 ? `${selected.length} selected` : undefined;
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <ChipTrigger label="Job function" summary={summary} active={selected.length > 0} />
      <PopoverContent align="start" className="w-[min(36rem,calc(100vw-2rem))] p-3">
        <ChipHeader
          label="Job function"
          onClear={selected.length ? () => apply(withValue(query, "function", undefined)) : undefined}
        />
        <ScrollArea className="max-h-[min(28rem,60dvh)]">
          <div className="p-1">
            <FunctionPicker vocab={vocab} draft={query} set={apply} />
          </div>
        </ScrollArea>
      </PopoverContent>
    </Popover>
  );
}

function CompanyChip({ top }: { top: Choice[] }) {
  const { query, apply } = useFeed();
  const selected = listOf(query, "company");
  const summary =
    selected.length === 1 ? selected[0] : selected.length > 1 ? `${selected.length} selected` : undefined;
  return (
    <Popover>
      <ChipTrigger label="Company" summary={summary} active={selected.length > 0} />
      <PopoverContent align="start" className="w-72 p-1.5">
        <ChipHeader
          label="Company"
          onClear={selected.length ? () => apply(withValue(query, "company", undefined)) : undefined}
        />
        <CompanyPicker
          top={top}
          selected={selected}
          onToggle={(value) => apply(toggleInList(query, "company", value))}
          autoFocus
        />
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
      <ChipTrigger
        label={label}
        summary={
          labelOf(options, value) ??
          (filterKey === "salary_min" ? salaryLabel(value) : (labelOf(POSTED_WITHIN, value) ?? value))
        }
        active={!!value}
      />
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

