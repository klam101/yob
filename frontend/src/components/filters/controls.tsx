"use client";

import { Check, X } from "lucide-react";
import { useId } from "react";
import { Checkbox } from "@/components/ui/checkbox";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { cn } from "cn";

export type Choice = { value: string; label: string; count?: number };

const row =
  "flex cursor-pointer items-center gap-3 rounded-lg px-2 py-2 text-sm transition-colors hover:bg-muted";

/** Multi-select list of checkboxes. */
export function CheckList({
  options,
  selected,
  onToggle,
  className,
}: {
  options: Choice[];
  selected: string[];
  onToggle: (value: string) => void;
  className?: string;
}) {
  const id = useId();
  return (
    <div className={cn("flex flex-col", className)}>
      {options.map((o) => {
        const inputId = `${id}-${o.value}`;
        return (
          <label key={o.value} htmlFor={inputId} className={row}>
            <Checkbox
              id={inputId}
              checked={selected.includes(o.value)}
              onCheckedChange={() => onToggle(o.value)}
            />
            <span className="flex-1">{o.label}</span>
            {o.count !== undefined && (
              <span className="text-xs text-muted-foreground tabular-nums">{o.count}</span>
            )}
          </label>
        );
      })}
    </div>
  );
}

/** Single-select list with an "Any" option that clears the filter. */
export function RadioList({
  options,
  value,
  onChange,
  anyLabel = "Any",
  className,
}: {
  options: Choice[];
  value: string | undefined;
  onChange: (value: string | undefined) => void;
  anyLabel?: string;
  className?: string;
}) {
  const id = useId();
  const all = [{ value: "", label: anyLabel }, ...options];
  return (
    <RadioGroup
      value={value ?? ""}
      onValueChange={(v) => onChange(v || undefined)}
      className={cn("gap-0", className)}
    >
      {all.map((o) => {
        const inputId = `${id}-${o.value || "any"}`;
        return (
          <label key={o.value || "any"} htmlFor={inputId} className={row}>
            <RadioGroupItem id={inputId} value={o.value} />
            <span className="flex-1">{o.label}</span>
          </label>
        );
      })}
    </RadioGroup>
  );
}

/** Turns API facet counts (departments, companies) into list choices. */
export function facetChoices(facets: { value: string; count: number }[]): Choice[] {
  return facets.map((f) => ({ value: f.value, label: f.value, count: f.count }));
}

/** A titled block inside a filter section, with an optional hint. */
export function Field({
  title,
  hint,
  children,
  className,
}: {
  title: string;
  hint?: string;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("space-y-2.5", className)}>
      <div>
        <h4 className="text-[0.6875rem] font-semibold tracking-wider text-muted-foreground uppercase">{title}</h4>
        {hint && <p className="mt-0.5 text-sm text-muted-foreground">{hint}</p>}
      </div>
      {children}
    </div>
  );
}

/** Pills that toggle on and off; any number can be on. */
export function ChipGroup({
  options,
  selected,
  onToggle,
  className,
  label,
}: {
  options: Choice[];
  selected: string[];
  onToggle: (value: string) => void;
  className?: string;
  label?: string;
}) {
  return (
    <div role="group" aria-label={label} className={cn("flex flex-wrap gap-2", className)}>
      {options.map((o) => {
        const on = selected.includes(o.value);
        return (
          <button
            key={o.value}
            type="button"
            aria-pressed={on}
            onClick={() => onToggle(o.value)}
            className={cn(
              "inline-flex h-8 items-center gap-1.5 rounded-full border px-3 text-[0.8125rem] font-medium transition-[color,background-color,border-color,transform] duration-150 outline-none select-none focus-visible:ring-3 focus-visible:ring-ring/40 active:scale-[0.97]",
              on
                ? "border-primary/30 bg-primary/12 text-primary dark:bg-primary/20"
                : "border-border bg-card text-foreground/80 hover:bg-muted",
            )}
          >
            {on && <Check className="-ml-0.5 size-3.5" aria-hidden />}
            {o.label}
            {o.count !== undefined && (
              <span className="text-xs text-muted-foreground tabular-nums">{o.count.toLocaleString("en-US")}</span>
            )}
          </button>
        );
      })}
    </div>
  );
}

/** One choice out of a few, as a segmented switch. `undefined` is "Any". */
export function Segmented({
  options,
  value,
  onChange,
  anyLabel,
  label,
}: {
  options: Choice[];
  value: string | undefined;
  onChange: (value: string | undefined) => void;
  /** Adds a first option that clears the value. */
  anyLabel?: string;
  label?: string;
}) {
  const all = anyLabel ? [{ value: "", label: anyLabel }, ...options] : options;
  const current = value ?? "";
  return (
    <div role="radiogroup" aria-label={label} className="inline-flex flex-wrap gap-0.5 rounded-xl bg-muted p-1">
      {all.map((o) => {
        const on = o.value === current;
        return (
          <button
            key={o.value || "any"}
            type="button"
            role="radio"
            aria-checked={on}
            onClick={() => onChange(o.value || undefined)}
            className={cn(
              "h-7 rounded-lg px-3 text-[0.8125rem] font-medium transition-colors outline-none focus-visible:ring-3 focus-visible:ring-ring/40",
              on ? "bg-card text-foreground shadow-sm" : "text-muted-foreground hover:text-foreground",
            )}
          >
            {o.label}
          </button>
        );
      })}
    </div>
  );
}

/** An applied filter with a remove button. */
export function RemovableChip({ label, onRemove }: { label: string; onRemove: () => void }) {
  return (
    <span className="inline-flex h-7 max-w-64 shrink-0 items-center gap-1 rounded-full border border-primary/25 bg-primary/8 pr-1 pl-2.5 text-xs font-medium text-primary dark:bg-primary/15">
      <span className="truncate">{label}</span>
      <button
        type="button"
        onClick={onRemove}
        aria-label={`Remove ${label}`}
        className="grid size-5 place-items-center rounded-full transition-colors hover:bg-primary/15"
      >
        <X className="size-3" aria-hidden />
      </button>
    </span>
  );
}
