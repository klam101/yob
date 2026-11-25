"use client";

import { ArrowDown, ArrowUp, Plus, Trash2, X } from "lucide-react";
import { createContext, useContext, useId, useLayoutEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input, inputClass } from "@/components/ui/input";
import { DateInput } from "@/components/ui/date-picker";
import { SuggestInput } from "@/components/ui/suggest";
import { Textarea } from "@/components/ui/textarea";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "cn";

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/** "2024-05-14" (or "2024-05") -> "May 2024". */
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

/** An error the API reported for one field, by path (`experiences.2.end`). */
export type FieldError = { field: string; message: string };

/**
 * Where fields are: the path of the section or entry they belong to
 * (`experiences.2`), and the error to show, if it's for one of them.
 */
const Scope = createContext<{ path: string; error: FieldError | null }>({ path: "", error: null });

/** Gives the fields inside it a path, and the error to match against it. */
export function FieldScope({
  path,
  error,
  children,
}: {
  path: string;
  error?: FieldError | null;
  children: React.ReactNode;
}) {
  const outer = useContext(Scope);
  const full = [outer.path, path].filter(Boolean).join(".");
  return <Scope.Provider value={{ path: full, error: error === undefined ? outer.error : error }}>{children}</Scope.Provider>;
}

/** The full path of a field named `name` in the current scope. */
export function useFieldPath(name?: string) {
  const { path } = useContext(Scope);
  return name ? [path, name].filter(Boolean).join(".") : path;
}

/** The message for this field (`name` inside the current scope), if any. */
export function useFieldError(name?: string) {
  const { path, error } = useContext(Scope);
  if (!name || !error) return undefined;
  const full = [path, name].filter(Boolean).join(".");
  return error.field === full ? error.message : undefined;
}

export function Field({
  label,
  hint,
  error,
  required,
  className,
  children,
}: {
  label: string;
  hint?: string;
  error?: string;
  required?: boolean;
  className?: string;
  children: (id: string, describedBy: string | undefined, invalid: boolean) => React.ReactNode;
}) {
  const id = useId();
  const described = [hint && `${id}-hint`, error && `${id}-error`].filter(Boolean).join(" ") || undefined;
  return (
    <div className={cn("space-y-1.5", className)}>
      <label htmlFor={id} className="block text-sm font-medium">
        {label}
        {required && (
          <span className="ml-0.5 text-destructive" aria-hidden>
            *
          </span>
        )}
      </label>
      {children(id, described, !!error)}
      {hint && (
        <p id={`${id}-hint`} className="text-xs text-muted-foreground">
          {hint}
        </p>
      )}
      {error && (
        <p id={`${id}-error`} role="alert" className="text-xs font-medium text-destructive">
          {error}
        </p>
      )}
    </div>
  );
}

export const invalidClass = "border-destructive/60 focus-visible:border-destructive focus-visible:ring-destructive/20";

export function TextField({
  label,
  name,
  value,
  onChange,
  hint,
  required,
  type = "text",
  placeholder,
  className,
  autoComplete,
  suggestions,
}: {
  label: string;
  /** Suggestions in a menu below, narrowed as you type (any text is allowed). */
  suggestions?: readonly string[];
  /** The field's name in the API, to show its error here. */
  name?: string;
  value: string;
  onChange: (v: string) => void;
  hint?: string;
  required?: boolean;
  type?: string;
  placeholder?: string;
  className?: string;
  autoComplete?: string;
}) {
  const error = useFieldError(name);
  return (
    <Field label={label} hint={hint} error={error} required={required} className={className}>
      {(id, describedBy, invalid) =>
        suggestions ? (
          <SuggestInput
            id={id}
            value={value}
            options={suggestions}
            onChange={onChange}
            placeholder={placeholder}
            aria-invalid={invalid || undefined}
            aria-describedby={describedBy}
            aria-required={required || undefined}
            className={cn(inputClass, invalid && invalidClass)}
          />
        ) : (
          <Input
            id={id}
            type={type}
            value={value}
            placeholder={placeholder}
            autoComplete={autoComplete ?? "off"}
            aria-invalid={invalid || undefined}
            aria-describedby={describedBy}
            aria-required={required || undefined}
            className={cn(invalid && invalidClass)}
            onChange={(e) => onChange(e.target.value)}
          />
        )
      }
    </Field>
  );
}

export function TextAreaField({
  label,
  name,
  value,
  onChange,
  hint,
  placeholder,
  className,
}: {
  label: string;
  name?: string;
  value: string;
  onChange: (v: string) => void;
  hint?: string;
  placeholder?: string;
  className?: string;
}) {
  const error = useFieldError(name);
  return (
    <Field label={label} hint={hint} error={error} className={className}>
      {(id, describedBy, invalid) => (
        <Textarea
          id={id}
          value={value}
          placeholder={placeholder}
          aria-invalid={invalid || undefined}
          aria-describedby={describedBy}
          className={cn(invalid && invalidClass)}
          onChange={(e) => onChange(e.target.value)}
        />
      )}
    </Field>
  );
}

/**
 * A date field: shows "Nov 2026", stores the whole day (`YYYY-MM-DD`; the
 * 1st when only a month is given), with a calendar below. Empty means no date.
 */
export function DateField({
  label,
  name,
  value,
  onChange,
  disabled,
}: {
  label: string;
  name?: string;
  value: string | null;
  onChange: (v: string | null) => void;
  disabled?: boolean;
}) {
  const error = useFieldError(name);
  return (
    <Field label={label} error={error}>
      {(id, describedBy, invalid) => (
        <DateInput
          id={id}
          aria-label={label}
          placeholder="MM/DD/YYYY"
          value={value}
          onChange={onChange}
          disabled={disabled}
          aria-invalid={invalid || undefined}
          aria-describedby={describedBy}
          className={cn(inputClass, invalid && invalidClass)}
        />
      )}
    </Field>
  );
}

/** A textarea that grows with its text. */
export function GrowingTextarea({
  className,
  value,
  boxRef,
  ...props
}: Omit<React.ComponentProps<"textarea">, "ref"> & {
  value: string;
  boxRef?: (el: HTMLTextAreaElement | null) => void;
}) {
  const ref = useRef<HTMLTextAreaElement | null>(null);
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight + 2}px`;
  }, [value]);
  return (
    <Textarea
      ref={(el) => {
        ref.current = el;
        boxRef?.(el);
      }}
      rows={1}
      value={value}
      className={cn("min-h-10 resize-none overflow-hidden py-2", className)}
      {...props}
    />
  );
}

/**
 * One box per bullet. Enter adds a bullet below, Backspace in an empty
 * bullet removes it, and pasting several lines makes several bullets.
 */
export function BulletList({
  label = "Bullet points",
  name = "bullets",
  value,
  onChange,
  onShift,
  marker,
}: {
  label?: string;
  name?: string;
  value: string[];
  onChange: (v: string[]) => void;
  /**
   * Bullets were inserted (`by` > 0) or one removed (`by` = -1) at `at`,
   * so anything that points at bullets by index can follow.
   */
  onShift?: (at: number, by: number) => void;
  /** Something to show after bullet `index` (the resume page's "Changed" tag). */
  marker?: (index: number) => React.ReactNode;
}) {
  const error = useFieldError(name);
  const base = useFieldPath(name);
  const labelId = useId();
  const boxes = useRef<(HTMLTextAreaElement | null)[]>([]);
  // Which box to focus after the list changes, and where to put the cursor.
  const focusNext = useRef<{ index: number; at: "start" | "end" } | null>(null);
  // Stable keys, so typing in one box never remounts another.
  const [keys, setKeys] = useState(() => value.map((_, i) => i));
  const [nextKey, setNextKey] = useState(value.length);
  if (keys.length !== value.length) {
    setKeys(value.map((_, i) => nextKey + i));
    setNextKey(nextKey + value.length);
  }

  useLayoutEffect(() => {
    const f = focusNext.current;
    if (!f) return;
    focusNext.current = null;
    const el = boxes.current[f.index];
    if (!el) return;
    el.focus();
    const pos = f.at === "start" ? 0 : el.value.length;
    el.setSelectionRange(pos, pos);
  });

  /** Inserts `items` at `at` into `base` (the list after any edit). */
  const insert = (at: number, items: string[], focus: number, base: string[] = value) => {
    const next = [...base];
    next.splice(at, 0, ...items);
    const k = [...keys];
    k.splice(at, 0, ...items.map((_, i) => nextKey + i));
    setKeys(k);
    setNextKey(nextKey + items.length);
    focusNext.current = { index: focus, at: "end" };
    onShift?.(at, items.length);
    onChange(next);
  };
  const remove = (i: number, focus: number | null) => {
    setKeys(keys.filter((_, j) => j !== i));
    if (focus !== null) focusNext.current = { index: focus, at: "end" };
    onShift?.(i, -1);
    onChange(value.filter((_, j) => j !== i));
  };
  const set = (i: number, text: string) => onChange(value.map((b, j) => (j === i ? text : b)));

  return (
    <div className="space-y-2" role="group" aria-labelledby={labelId}>
      <p id={labelId} className="text-sm font-medium">
        {label}
      </p>
      {value.length > 0 && (
        <ol className="space-y-2">
          {value.map((b, i) => (
            <li key={keys[i]} className="flex items-start gap-2" data-field={`${base}.${i}`}>
              <span aria-hidden className="mt-4 size-1.5 shrink-0 rounded-full bg-foreground/50" />
              <GrowingTextarea
                boxRef={(el) => {
                  boxes.current[i] = el;
                }}
                value={b}
                aria-label={`Bullet ${i + 1}`}
                onChange={(e) => set(i, e.target.value.replace(/\n/g, " "))}
                onKeyDown={(e) => {
                  const el = e.currentTarget;
                  if (e.key === "Enter" && !e.shiftKey && !e.nativeEvent.isComposing) {
                    e.preventDefault();
                    // Split at the cursor: the rest moves to a new bullet.
                    const head = el.value.slice(0, el.selectionStart).trimEnd();
                    const tail = el.value.slice(el.selectionEnd).trimStart();
                    const next = [...value];
                    next[i] = head;
                    insert(i + 1, [tail], i + 1, next);
                    if (tail) focusNext.current = { index: i + 1, at: "start" };
                  } else if (e.key === "Backspace" && el.value === "" && value.length > 0) {
                    e.preventDefault();
                    remove(i, i > 0 ? i - 1 : value.length > 1 ? 0 : null);
                  }
                }}
                onPaste={(e) => {
                  const text = e.clipboardData.getData("text");
                  const lines = text
                    .split(/\r?\n/)
                    .map((l) => l.replace(/^\s*(?:[•·●▪◦‣\-*–]|\d+[.)])\s+/, "").trim())
                    .filter(Boolean);
                  if (lines.length < 2) return;
                  e.preventDefault();
                  const el = e.currentTarget;
                  const before = el.value.slice(0, el.selectionStart);
                  const after = el.value.slice(el.selectionEnd);
                  const first = `${before}${lines[0]}`;
                  const last = `${lines[lines.length - 1]}${after}`;
                  const middle = lines.slice(1, -1);
                  const next = [...value];
                  next[i] = first;
                  insert(i + 1, [...middle, last], i + lines.length - 1, next);
                }}
                className={cn(error && invalidClass)}
              />
              {marker?.(i)}
              <IconButton label="Remove bullet" onClick={() => remove(i, null)} className="mt-1">
                <X />
              </IconButton>
            </li>
          ))}
        </ol>
      )}
      <Button
        type="button"
        variant="light"
        size="sm"
        onClick={() => insert(value.length, [""], value.length)}
      >
        <Plus /> Bullet point
      </Button>
      {error && (
        <p role="alert" className="text-xs font-medium text-destructive">
          {error}
        </p>
      )}
    </div>
  );
}

/** An icon-only button with a tooltip and a matching accessible name. */
export function IconButton({
  label,
  onClick,
  disabled,
  className,
  children,
}: {
  label: string;
  onClick: () => void;
  disabled?: boolean;
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          aria-label={label}
          disabled={disabled}
          onClick={onClick}
          className={cn("shrink-0", className)}
        >
          {children}
        </Button>
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}

const selectClass =
  "h-10 w-full rounded-xl border border-transparent bg-muted px-3 text-base outline-none transition-[color,background-color,border-color,box-shadow] hover:bg-[color-mix(in_oklch,var(--muted),var(--foreground)_4%)] focus-visible:border-ring focus-visible:bg-card focus-visible:ring-3 focus-visible:ring-ring/25 md:text-sm";

export function SelectField({
  label,
  name,
  value,
  onChange,
  options,
  hint,
}: {
  label: string;
  name?: string;
  value: string;
  onChange: (v: string) => void;
  options: readonly (string | { value: string; label: string })[];
  hint?: string;
}) {
  const error = useFieldError(name);
  const opts = options.map((o) => (typeof o === "string" ? { value: o, label: o } : o));
  // Keep a saved value even if it's no longer one of the options.
  const all = value && !opts.some((o) => o.value === value) ? [{ value, label: value }, ...opts] : opts;
  return (
    <Field label={label} hint={hint} error={error}>
      {(id, describedBy, invalid) => (
        <select
          id={id}
          className={cn(selectClass, invalid && invalidClass)}
          value={value}
          aria-invalid={invalid || undefined}
          aria-describedby={describedBy}
          onChange={(e) => onChange(e.target.value)}
        >
          <option value="">Not provided</option>
          {all.map((o) => (
            <option key={o.value} value={o.value}>
              {o.label}
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
 * Add and remove entries of a list section, headed "Work experience 1, 2…".
 * Each entry renders its own fields inside a `FieldScope` for its index, so
 * an API error for `experiences.1.end` shows under that entry's end date.
 * Lists sorted by date on save have no move buttons; hand-ordered ones
 * (`reorder`) do.
 */
export function ListEditor<T>({
  items,
  onChange,
  newItem,
  addLabel,
  itemLabel,
  render,
  reorder = false,
}: {
  items: T[];
  onChange: (items: T[]) => void;
  newItem: () => T;
  addLabel: string;
  /** "Work experience" -> headings "Work experience 1", "Work experience 2"… */
  itemLabel: string;
  render: (item: T, update: (patch: Partial<T>) => void, index: number) => React.ReactNode;
  reorder?: boolean;
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
    const shift = <X,>(list: X[]) => {
      const next = [...list];
      const [it] = next.splice(i, 1);
      next.splice(i + by, 0, it);
      return next;
    };
    setKeys(shift(keys));
    onChange(shift(items));
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
    <div className="space-y-6">
      {items.map((item, i) => (
        <FieldScope key={keys[i]} path={String(i)}>
          <section className="space-y-4 border-b border-border/60 pb-6" aria-label={`${itemLabel} ${i + 1}`}>
            <div className="flex items-center gap-1">
              <h3 className="mr-1 text-base font-semibold">
                {itemLabel} {i + 1}
              </h3>
              {reorder && (
                <>
                  <IconButton label="Move up" disabled={i === 0} onClick={() => move(i, -1)}>
                    <ArrowUp />
                  </IconButton>
                  <IconButton label="Move down" disabled={i === items.length - 1} onClick={() => move(i, 1)}>
                    <ArrowDown />
                  </IconButton>
                </>
              )}
              <IconButton label={`Delete ${itemLabel.toLowerCase()} ${i + 1}`} onClick={() => remove(i)}>
                <Trash2 />
              </IconButton>
            </div>
            {render(item, (patch) => onChange(items.map((it, j) => (j === i ? { ...it, ...patch } : it))), i)}
          </section>
        </FieldScope>
      ))}
      <Button type="button" variant="bordered" onClick={add}>
        <Plus /> {addLabel}
      </Button>
    </div>
  );
}
