"use client";

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
