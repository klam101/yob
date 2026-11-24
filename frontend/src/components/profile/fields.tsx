"use client";

import { ArrowDown, ArrowUp, Plus, Trash2 } from "lucide-react";
import { useId, useState } from "react";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "cn";

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/** "2024-05" -> "May 2024". */
export function formatMonth(m: string | null | undefined) {
  if (!m) return "";
  const [y, mo] = m.split("-");
  return `${MONTHS[Number(mo) - 1] ?? ""} ${y}`.trim();
}

/** "May 2022 – Present", "2019 – Jun 2020", or "" when there are no dates. */
export function formatRange(start: string | null | undefined, end: string | null | undefined, current = true) {
  if (!start && !end) return "";
  if (start && !end) return current ? `${formatMonth(start)} – Present` : formatMonth(start);
  if (!start) return formatMonth(end);
  if (start === end) return formatMonth(start);
  return `${formatMonth(start)} – ${formatMonth(end)}`;
}

export function Field({
  label,
  hint,
  className,
  children,
}: {
  label: string;
  hint?: string;
  className?: string;
  children: (id: string) => React.ReactNode;
}) {
  const id = useId();
  return (
    <div className={cn("space-y-1.5", className)}>
      <label htmlFor={id} className="block text-sm font-medium">
        {label}
      </label>
      {children(id)}
      {hint && <p className="text-xs text-muted-foreground">{hint}</p>}
    </div>
  );
}

export function TextField({
  label,
  value,
  onChange,
  hint,
  type = "text",
  placeholder,
  className,
  autoComplete,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  hint?: string;
  type?: string;
  placeholder?: string;
  className?: string;
  autoComplete?: string;
}) {
  return (
    <Field label={label} hint={hint} className={className}>
      {(id) => (
        <Input
          id={id}
          type={type}
          value={value}
          placeholder={placeholder}
          autoComplete={autoComplete ?? "off"}
          onChange={(e) => onChange(e.target.value)}
        />
      )}
    </Field>
  );
}

export function TextAreaField({
  label,
  value,
  onChange,
  hint,
  placeholder,
  className,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  hint?: string;
  placeholder?: string;
  className?: string;
}) {
  return (
    <Field label={label} hint={hint} className={className}>
      {(id) => <Textarea id={id} value={value} placeholder={placeholder} onChange={(e) => onChange(e.target.value)} />}
    </Field>
  );
}

/** A month picker (`YYYY-MM`); empty means no date. */
export function MonthField({
  label,
  value,
  onChange,
  disabled,
}: {
  label: string;
  value: string | null;
  onChange: (v: string | null) => void;
  disabled?: boolean;
}) {
  return (
    <Field label={label}>
      {(id) => (
        <Input
          id={id}
          type="month"
          value={value ?? ""}
          disabled={disabled}
          onChange={(e) => onChange(e.target.value || null)}
        />
      )}
    </Field>
  );
}

/** One bullet per line. */
export function BulletsField({
  label = "Bullet points",
  value,
  onChange,
}: {
  label?: string;
  value: string[];
  onChange: (v: string[]) => void;
}) {
  return (
    <Field label={label} hint="One bullet per line.">
      {(id) => (
        <Textarea
          id={id}
          className="min-h-28"
          value={value.join("\n")}
          onChange={(e) => onChange(e.target.value.split("\n"))}
        />
      )}
    </Field>
  );
}

const selectClass =
  "h-10 w-full rounded-xl border border-transparent bg-muted px-3 text-base outline-none transition-[color,background-color,border-color,box-shadow] hover:bg-[color-mix(in_oklch,var(--muted),var(--foreground)_4%)] focus-visible:border-ring focus-visible:bg-card focus-visible:ring-3 focus-visible:ring-ring/25 md:text-sm";

export function SelectField({
  label,
  value,
  onChange,
  options,
  hint,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  options: readonly string[];
  hint?: string;
}) {
  // Keep a saved value even if it's no longer one of the options.
  const all = value && !options.includes(value) ? [value, ...options] : options;
  return (
    <Field label={label} hint={hint}>
      {(id) => (
        <select id={id} className={selectClass} value={value} onChange={(e) => onChange(e.target.value)}>
          <option value="">Not provided</option>
          {all.map((o) => (
            <option key={o} value={o}>
              {o}
            </option>
          ))}
        </select>
      )}
    </Field>
  );
}

/** Yes / No / not answered. */
export function YesNoField({
  label,
  value,
  onChange,
}: {
  label: string;
  value: boolean | null;
  onChange: (v: boolean | null) => void;
}) {
  return (
    <Field label={label}>
      {(id) => (
        <select
          id={id}
          className={selectClass}
          value={value === null ? "" : value ? "yes" : "no"}
          onChange={(e) => onChange(e.target.value === "" ? null : e.target.value === "yes")}
        >
          <option value="">Not provided</option>
          <option value="yes">Yes</option>
          <option value="no">No</option>
        </select>
      )}
    </Field>
  );
}

export function CheckField({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  const id = useId();
  return (
    <div className="flex items-center gap-2">
      <Checkbox id={id} checked={checked} onCheckedChange={(v) => onChange(v === true)} />
      <label htmlFor={id} className="text-sm">
        {label}
      </label>
    </div>
  );
}

/**
 * Add, remove and reorder entries of a list section. Each entry renders its
 * own fields; the list owns order.
 */
export function ListEditor<T>({
  items,
  onChange,
  newItem,
  addLabel,
  itemLabel,
  render,
}: {
  items: T[];
  onChange: (items: T[]) => void;
  newItem: () => T;
  addLabel: string;
  itemLabel: (item: T, index: number) => string;
  render: (item: T, update: (patch: Partial<T>) => void) => React.ReactNode;
}) {
  // Stable keys, so an entry's own state (like "current") moves with it.
  // Every change goes through this list, so the keys follow the items.
  const [keys, setKeys] = useState(() => items.map((_, i) => i));
  const [nextKey, setNextKey] = useState(items.length);
  if (keys.length !== items.length) {
    // Items replaced from outside: start over.
    setKeys(items.map((_, i) => nextKey + i));
    setNextKey(nextKey + items.length);
  }

  const move = (i: number, by: number) => {
    const reorder = <X,>(list: X[]) => {
      const next = [...list];
      const [it] = next.splice(i, 1);
      next.splice(i + by, 0, it);
      return next;
    };
    setKeys(reorder(keys));
    onChange(reorder(items));
  };
  const remove = (i: number) => {
    setKeys(keys.filter((_, j) => j !== i));
    onChange(items.filter((_, j) => j !== i));
  };
  const add = () => {
    setKeys([...keys, nextKey]);
    setNextKey(nextKey + 1);
    onChange([...items, newItem()]);
  };
  return (
    <div className="space-y-4">
      {items.map((item, i) => (
        <div key={keys[i]} className="space-y-4 rounded-2xl border border-border/70 p-4">
          <div className="flex items-center justify-between gap-2">
            <p className="truncate text-sm font-semibold">{itemLabel(item, i)}</p>
            <div className="flex shrink-0 gap-1">
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                aria-label="Move up"
                disabled={i === 0}
                onClick={() => move(i, -1)}
              >
                <ArrowUp />
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                aria-label="Move down"
                disabled={i === items.length - 1}
                onClick={() => move(i, 1)}
              >
                <ArrowDown />
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                aria-label="Remove"
                onClick={() => remove(i)}
              >
                <Trash2 />
              </Button>
            </div>
          </div>
          {render(item, (patch) => onChange(items.map((it, j) => (j === i ? { ...it, ...patch } : it))))}
        </div>
      ))}
      <Button type="button" variant="bordered" onClick={add}>
        <Plus /> {addLabel}
      </Button>
    </div>
  );
}
