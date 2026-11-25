"use client";

import { CalendarDays, ChevronDown, ChevronLeft, ChevronRight } from "lucide-react";
import { Popover as PopoverPrimitive } from "radix-ui";
import { useId, useState } from "react";
import { DayPicker } from "react-day-picker";
import { cn } from "cn";

/*
 * Dates are whole days (`YYYY-MM-DD`) underneath and shown as month and year
 * ("Nov 2026"). Some application forms ask for the exact day; when the user
 * doesn't know it, the 1st is stored.
 */

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const MONTH_NAMES = [
  "january", "february", "march", "april", "may", "june",
  "july", "august", "september", "october", "november", "december",
];

const pad = (n: number) => String(n).padStart(2, "0");

/** A local date from `YYYY-MM-DD` (or `YYYY-MM`, the 1st). */
export function toDate(v: string | null | undefined): Date | undefined {
  const m = /^(\d{4})-(\d{1,2})(?:-(\d{1,2}))?$/.exec(v?.trim() ?? "");
  return m ? make(+m[1], +m[2], m[3] ? +m[3] : 1) : undefined;
}

export const toIso = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;

/** "2026-11-25" -> "Nov 2026". */
export function monthYear(v: string | null | undefined) {
  const d = toDate(v);
  return d ? `${MONTHS[d.getMonth()]} ${d.getFullYear()}` : "";
}

/** "2026-11-25" -> "11/25/2026", what the box shows while it's edited. */
const typed = (v: string | null | undefined) => {
  const d = toDate(v);
  return d ? `${pad(d.getMonth() + 1)}/${pad(d.getDate())}/${d.getFullYear()}` : "";
};

/** A real day in a sensible year, or undefined (no rolling 2/30 into March). */
function make(y: number, m: number, d: number): Date | undefined {
  if (y < 1900 || y > 2100) return undefined;
  const date = new Date(y, m - 1, d);
  return date.getFullYear() === y && date.getMonth() === m - 1 && date.getDate() === d ? date : undefined;
}

const fullYear = (y: string) => {
  if (y.length !== 2) return +y;
  const n = +y;
  return n <= (new Date().getFullYear() % 100) + 10 ? 2000 + n : 1900 + n;
};

/**
 * What people type for a date: "11/25/2026", "11/2026" (the 1st), "Nov 2026",
 * "November 25, 2026", "2026-11-25". Undefined when it isn't one.
 */
export function parseTyped(text: string): Date | undefined {
  const t = text.trim().toLowerCase().replace(/\s+/g, " ");
  let m = /^(\d{1,2})[/.-](\d{1,2})[/.-](\d{2}|\d{4})$/.exec(t);
  if (m) return make(fullYear(m[3]), +m[1], +m[2]);
  m = /^(\d{1,2})[/.-](\d{4})$/.exec(t);
  if (m) return make(+m[2], +m[1], 1);
  m = /^(\d{4})-(\d{1,2})(?:-(\d{1,2}))?$/.exec(t);
  if (m) return make(+m[1], +m[2], m[3] ? +m[3] : 1);
  m = /^([a-z]+)\.? (?:(\d{1,2})(?:st|nd|rd|th)?,? )?(\d{4})$/.exec(t);
  if (m) {
    const month = MONTH_NAMES.findIndex((n) => m![1].length >= 3 && n.startsWith(m![1]));
    if (month >= 0) return make(+m[3], month + 1, m[2] ? +m[2] : 1);
  }
  return undefined;
}

const navButton =
  "inline-flex size-8 items-center justify-center rounded-xl text-muted-foreground transition-colors outline-none hover:bg-muted hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/30";
const pill =
  "rounded-xl py-2 text-sm font-medium transition-colors outline-none hover:bg-muted focus-visible:ring-3 focus-visible:ring-ring/30";

/**
 * A month calendar in the app's style: "‹ Nov 2026 ›" over the days.
 * Clicking the title picks a month, and clicking the year there picks a
 * year, without native dropdowns.
 */
export function Calendar({
  selected,
  onSelect,
  onMonthOnly,
  onClear,
}: {
  selected: Date | undefined;
  onSelect: (d: Date) => void;
  /** "No exact day": the 1st of the month on show. */
  onMonthOnly: (d: Date) => void;
  onClear?: () => void;
}) {
  const [month, setMonth] = useState(() => {
    const d = selected ?? new Date();
    return new Date(d.getFullYear(), d.getMonth(), 1);
  });
  const [view, setView] = useState<"days" | "months" | "years">("days");
  const y = month.getFullYear();
  const shift = (months: number) => setMonth(new Date(y, month.getMonth() + months, 1));
  const decade = y - (y % 12);

  const header = (label: React.ReactNode, onLabel: (() => void) | undefined, prev: () => void, next: () => void, what: string) => (
    <div className="flex items-center justify-between gap-1 pb-2">
      <button type="button" className={navButton} onClick={prev} aria-label={`Previous ${what}`}>
        <ChevronLeft className="size-4" />
      </button>
      {onLabel ? (
        <button
          type="button"
          onClick={onLabel}
          className="inline-flex items-center gap-1 rounded-xl px-2.5 py-1 text-sm font-semibold transition-colors outline-none hover:bg-muted focus-visible:ring-3 focus-visible:ring-ring/30"
        >
          {label}
          <ChevronDown className="size-3.5 text-muted-foreground" />
        </button>
      ) : (
        <span className="px-2.5 py-1 text-sm font-semibold">{label}</span>
      )}
      <button type="button" className={navButton} onClick={next} aria-label={`Next ${what}`}>
        <ChevronRight className="size-4" />
      </button>
    </div>
  );

  return (
    <div className="w-[18rem]">
      {view === "days" && (
        <>
          {header(`${MONTHS[month.getMonth()]} ${y}`, () => setView("months"), () => shift(-1), () => shift(1), "month")}
          <DayPicker
            mode="single"
            required
            month={month}
            onMonthChange={setMonth}
            selected={selected}
            onSelect={(d) => d && onSelect(d)}
            hideNavigation
            showOutsideDays
            fixedWeeks
            classNames={{
              root: "w-full",
              months: "w-full",
              month: "w-full",
              month_caption: "sr-only",
              month_grid: "w-full border-collapse",
              weekdays: "flex",
              weekday: "flex-1 pb-1 text-center text-xs font-medium text-muted-foreground",
              week: "flex",
              day: "flex flex-1 justify-center p-0.5 text-center",
              day_button:
                "inline-flex size-9 items-center justify-center rounded-xl text-sm font-medium transition-colors outline-none hover:bg-muted focus-visible:ring-3 focus-visible:ring-ring/30",
              selected:
                "[&>button]:bg-primary [&>button]:text-primary-foreground [&>button]:shadow-soft [&>button]:hover:bg-primary",
              today: "[&>button]:ring-1 [&>button]:ring-primary/40 [&>button]:text-primary",
              outside: "[&>button]:text-muted-foreground/45",
            }}
          />
        </>
      )}
      {view === "months" && (
        <>
          {header(y, () => setView("years"), () => setMonth(new Date(y - 1, month.getMonth(), 1)), () => setMonth(new Date(y + 1, month.getMonth(), 1)), "year")}
          <div className="grid grid-cols-3 gap-1">
            {MONTHS.map((m, i) => (
              <button
                key={m}
                type="button"
                onClick={() => {
                  setMonth(new Date(y, i, 1));
                  setView("days");
                }}
                className={cn(pill, i === month.getMonth() && "bg-primary text-primary-foreground hover:bg-primary")}
              >
                {m}
              </button>
            ))}
          </div>
        </>
      )}
      {view === "years" && (
        <>
          {header(`${decade} – ${decade + 11}`, undefined, () => setMonth(new Date(y - 12, month.getMonth(), 1)), () => setMonth(new Date(y + 12, month.getMonth(), 1)), "years")}
          <div className="grid grid-cols-3 gap-1">
            {Array.from({ length: 12 }, (_, i) => decade + i).map((year) => (
              <button
                key={year}
                type="button"
                onClick={() => {
                  setMonth(new Date(year, month.getMonth(), 1));
                  setView("months");
                }}
                className={cn(pill, year === y && "bg-primary text-primary-foreground hover:bg-primary")}
              >
                {year}
              </button>
            ))}
          </div>
        </>
      )}
      <div className="mt-2 flex items-center justify-between border-t border-border/60 pt-2">
        {onClear ? (
          <button type="button" onClick={onClear} className="rounded-lg px-2 py-1 text-xs font-medium text-muted-foreground transition-colors hover:bg-muted hover:text-foreground">
            Clear
          </button>
        ) : (
          <span />
        )}
        <button
          type="button"
          onClick={() => onMonthOnly(month)}
          title="Don't know the day? This keeps the 1st."
          className="rounded-lg px-2 py-1 text-xs font-medium text-primary transition-colors hover:bg-primary/10"
        >
          No exact day: {MONTHS[month.getMonth()]} {y}
        </button>
      </div>
    </div>
  );
}

type InputProps = Omit<React.ComponentProps<"input">, "value" | "onChange" | "type" | "ref">;

/**
 * A date box: shows "Nov 2026", takes a typed date ("11/25/2026",
 * "11/2026") or one picked from the calendar below it, and stores the whole
 * day (`YYYY-MM-DD`; the 1st when only a month is given).
 */
export function DateInput({
  value,
  onChange,
  className,
  wrapperClassName,
  disabled,
  ...props
}: InputProps & {
  value: string | null;
  onChange: (v: string | null) => void;
  wrapperClassName?: string;
}) {
  const id = useId();
  const [text, setText] = useState<string | null>(null); // non-null while editing
  const [bad, setBad] = useState(false);
  const [open, setOpen] = useState(false);
  const label = props["aria-label"] ?? "date";

  function commit() {
    if (text === null) return;
    const t = text.trim();
    if (!t) {
      setBad(false);
      if (value !== null) onChange(null);
      return;
    }
    const d = parseTyped(t);
    if (!d) {
      setBad(true);
      return;
    }
    setBad(false);
    setText(typed(toIso(d)));
    if (toIso(d) !== value) onChange(toIso(d));
  }

  const pick = (d: Date | null) => {
    onChange(d ? toIso(d) : null);
    setText(null);
    setBad(false);
    setOpen(false);
  };

  const describedBy = [props["aria-describedby"], bad ? `${id}-bad` : undefined].filter(Boolean).join(" ") || undefined;

  return (
    <PopoverPrimitive.Root open={open} onOpenChange={setOpen}>
      <div className={cn("min-w-0", wrapperClassName)}>
        <PopoverPrimitive.Anchor asChild>
          <div className="relative">
            <input
              {...props}
              type="text"
              inputMode="text"
              autoComplete="off"
              disabled={disabled}
              value={text ?? monthYear(value)}
              title={value ? `Saved as ${typed(value)}` : undefined}
              aria-describedby={describedBy}
              aria-invalid={props["aria-invalid"] || bad || undefined}
              onFocus={(e) => {
                setText(typed(value));
                props.onFocus?.(e);
              }}
              onChange={(e) => {
                setText(e.target.value);
                if (bad) setBad(false);
              }}
              onBlur={(e) => {
                commit();
                setText(null);
                props.onBlur?.(e);
              }}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  commit();
                } else if (e.key === "ArrowDown" && e.altKey) {
                  e.preventDefault();
                  setOpen(true);
                }
                props.onKeyDown?.(e);
              }}
              className={cn("pr-9", className)}
            />
            <PopoverPrimitive.Trigger asChild>
              <button
                type="button"
                disabled={disabled}
                aria-label={`Pick ${label.toLowerCase()} from a calendar`}
                className="absolute top-1/2 right-1 inline-flex size-7 -translate-y-1/2 items-center justify-center rounded-lg text-muted-foreground transition-colors outline-none hover:bg-muted hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/30 disabled:opacity-50"
              >
                <CalendarDays className="size-4" />
              </button>
            </PopoverPrimitive.Trigger>
          </div>
        </PopoverPrimitive.Anchor>
        {bad && (
          <p id={`${id}-bad`} role="alert" className="px-2 pt-0.5 text-xs font-medium text-destructive">
            Type a date like 11/25/2026 or 11/2026.
          </p>
        )}
      </div>
      <PopoverPrimitive.Portal>
        <PopoverPrimitive.Content
          side="bottom"
          align="start"
          sideOffset={6}
          collisionPadding={8}
          aria-label={`Pick ${label.toLowerCase()}`}
          className="z-50 origin-(--radix-popover-content-transform-origin) rounded-2xl border border-border/60 bg-popover p-3 text-popover-foreground shadow-raised outline-hidden duration-100 data-[side=bottom]:slide-in-from-top-1 data-[side=top]:slide-in-from-bottom-1 data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95"
        >
          <Calendar
            selected={toDate(value)}
            onSelect={pick}
            onMonthOnly={(d) => pick(new Date(d.getFullYear(), d.getMonth(), 1))}
            onClear={value ? () => pick(null) : undefined}
          />
        </PopoverPrimitive.Content>
      </PopoverPrimitive.Portal>
    </PopoverPrimitive.Root>
  );
}
