"use client";

import { ChevronDown, Search } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { type Vocabulary, api } from "@/api/client";
import { Checkbox } from "@/components/ui/checkbox";
import { inputClass } from "@/components/ui/input";
import { Slider } from "@/components/ui/slider";
import { SuggestInput } from "@/components/ui/suggest";
import {
  DEGREE_LEVELS,
  DEGREE_STATES,
  type FilterKey,
  type Query,
  YEARS_TOP,
  listOf,
  toggleInList,
  withValue,
  withoutKeys,
  yearsLabel,
} from "@/lib/filters";
import { cn } from "cn";
import { type Choice, ChipGroup, Field, RemovableChip, Segmented } from "./controls";

export type Set = (q: Query) => void;

const EMPTY: Vocabulary = { functions: [], tags: [], industries: [], countries: [], companies_known: 0, companies_total: 0 };
let cached: Promise<Vocabulary> | null = null;

/** Job functions and detail tags with counts; fetched once per page load. */
export function useVocabulary(enabled: boolean): Vocabulary {
  const [vocab, setVocab] = useState<Vocabulary>(EMPTY);
  useEffect(() => {
    if (!enabled) return;
    cached ??= api
      .GET("/jobs/vocabulary")
      .then(({ data }) => data ?? EMPTY)
      .catch(() => {
        cached = null;
        return EMPTY;
      });
    let live = true;
    cached.then((v) => live && setVocab(v));
    return () => {
      live = false;
    };
  }, [enabled]);
  return vocab;
}

/** Labels for chips: `skill:rust` -> "Rust", `fn:software` -> "Software Development". */
export function vocabLabels(vocab: Vocabulary): Record<string, string> {
  const out: Record<string, string> = {};
  for (const t of vocab.tags) out[t.value] = t.label;
  for (const f of vocab.functions) out[`fn:${f.value}`] = f.label;
  return out;
}

/** The tags with this prefix as choices (slug values), biggest first. */
export function tagChoices(vocab: Vocabulary, prefix: string, sort = false): Choice[] {
  const list = vocab.tags
    .filter((t) => t.value.startsWith(`${prefix}:`))
    .map((t) => ({ value: t.value.slice(prefix.length + 1), label: t.label, count: t.count }));
  return sort ? list.sort((a, b) => (b.count ?? 0) - (a.count ?? 0)) : list;
}

/** Job functions, grouped and collapsible, with a search. */
export function FunctionPicker({ vocab, draft, set }: { vocab: Vocabulary; draft: Query; set: Set }) {
  const selected = listOf(draft, "function");
  const [search, setSearch] = useState("");
  const [closed, setClosed] = useState<string[]>([]);
  const q = search.trim().toLowerCase();
  const groups = useMemo(() => {
    const out: { group: string; items: Vocabulary["functions"] }[] = [];
    for (const f of vocab.functions) {
      if (q && !`${f.label} ${f.group}`.toLowerCase().includes(q)) continue;
      const g = out.find((x) => x.group === f.group);
      if (g) g.items.push(f);
      else out.push({ group: f.group, items: [f] });
    }
    return out;
  }, [vocab, q]);

  if (!vocab.functions.length) return <p className="text-sm text-muted-foreground">Loading…</p>;
  return (
    <div className="space-y-3">
      <div className="flex items-center gap-3">
        <label className="flex h-10 flex-1 items-center gap-2.5 rounded-xl border border-input bg-card px-3 focus-within:ring-3 focus-within:ring-ring/40">
          <Search className="size-4 text-muted-foreground" />
          <input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search job functions"
            aria-label="Search job functions"
            className="w-full bg-transparent text-sm outline-none placeholder:text-muted-foreground"
          />
        </label>
        <button
          type="button"
          onClick={() => setClosed(closed.length ? [] : groups.map((g) => g.group))}
          className="shrink-0 text-xs font-medium text-muted-foreground hover:text-foreground"
        >
          {closed.length ? "Expand all" : "Collapse all"}
        </button>
      </div>
      {selected.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {selected.map((v) => (
            <RemovableChip
              key={v}
              label={vocab.functions.find((f) => f.value === v)?.label ?? v}
              onRemove={() => set(toggleInList(draft, "function", v))}
            />
          ))}
        </div>
      )}
      <div className="divide-y divide-border/60">
        {groups.map(({ group, items }) => {
          const open = q !== "" || !closed.includes(group);
          const n = items.filter((f) => selected.includes(f.value)).length;
          return (
            <div key={group} className="py-1.5">
              <button
                type="button"
                aria-expanded={open}
                onClick={() => setClosed(open ? [...closed, group] : closed.filter((g) => g !== group))}
                className="flex w-full items-center gap-2 rounded-lg px-2 py-2 text-left text-sm font-semibold hover:bg-muted/60"
              >
                <span className="flex-1">{group}</span>
                {n > 0 && (
                  <span className="rounded-full bg-primary/12 px-1.5 text-[0.6875rem] text-primary tabular-nums">{n}</span>
                )}
                <ChevronDown className={cn("size-4 text-muted-foreground transition-transform", open && "rotate-180")} />
              </button>
              {open && (
                <div className="sm:grid sm:grid-cols-2 sm:gap-x-4">
                  {items.map((f) => (
                    <label
                      key={f.value}
                      className="flex cursor-pointer items-center gap-3 rounded-lg px-2 py-2 text-sm transition-colors hover:bg-muted"
                    >
                      <Checkbox
                        checked={selected.includes(f.value)}
                        onCheckedChange={() => set(toggleInList(draft, "function", f.value))}
                      />
                      <span className="flex-1">{f.label}</span>
                      <span className="text-xs text-muted-foreground tabular-nums">{f.count.toLocaleString("en-US")}</span>
                    </label>
                  ))}
                </div>
              )}
            </div>
          );
        })}
        {groups.length === 0 && <p className="py-3 text-sm text-muted-foreground">No job function matches.</p>}
      </div>
    </div>
  );
}

/** Pick values from a known list by typing; picked ones show as chips. */
export function TagPicker({
  choices,
  draft,
  set,
  filterKey,
  placeholder,
  label,
}: {
  choices: Choice[];
  draft: Query;
  set: Set;
  filterKey: FilterKey;
  placeholder: string;
  label: string;
}) {
  const [text, setText] = useState("");
  const selected = listOf(draft, filterKey);
  const byLabel = new Map(choices.map((c) => [c.label, c.value]));
  const options = choices.filter((c) => !selected.includes(c.value)).map((c) => c.label);
  return (
    <div className="max-w-lg space-y-2">
      <SuggestInput
        aria-label={label}
        value={text}
        onChange={setText}
        options={options}
        limit={8}
        placeholder={placeholder}
        className={inputClass}
        onPick={(item) => {
          const value = byLabel.get(item.value);
          if (value) set(withValue(draft, filterKey, [...selected, value]));
          setText("");
        }}
      />
      {selected.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {selected.map((v) => (
            <RemovableChip
              key={v}
              label={choices.find((c) => c.value === v)?.label ?? v}
              onRemove={() => set(toggleInList(draft, filterKey, v))}
            />
          ))}
        </div>
      )}
    </div>
  );
}

/** Years of experience the job asks for, as a range; the top is "or more". */
export function YearsRange({ draft, set }: { draft: Query; set: Set }) {
  const lo = Number(draft.years_min) || 0;
  const hi = draft.years_max ? Math.min(Number(draft.years_max), YEARS_TOP) : YEARS_TOP;
  const active = !!(draft.years_min || draft.years_max);
  return (
    <div className="max-w-lg space-y-3">
      <div className="flex items-center justify-between text-sm">
        <span className="font-medium">{active ? yearsLabel(draft.years_min, draft.years_max) : "Any"}</span>
        <span className="text-xs text-muted-foreground">{YEARS_TOP} means {YEARS_TOP} or more</span>
      </div>
      <Slider
        min={0}
        max={YEARS_TOP}
        step={1}
        value={[lo, hi]}
        thumbLabels={["Fewest years", "Most years"]}
        onValueChange={([a, b]) => {
          const next =
            a === 0 && b === YEARS_TOP
              ? withoutKeys(draft, ["years_min", "years_max", "years_unlisted"])
              : withValue(withValue(draft, "years_min", String(a)), "years_max", b >= YEARS_TOP ? undefined : String(b));
          set(next);
        }}
        className="py-2"
      />
      {active && (
        <label className="flex cursor-pointer items-center gap-2.5 text-sm">
          <Checkbox
            checked={draft.years_unlisted === "true"}
            onCheckedChange={(on) => set(withValue(draft, "years_unlisted", on ? "true" : undefined))}
          />
          Keep jobs that don&apos;t say how many years
        </label>
      )}
    </div>
  );
}

/** Per degree: required, preferred and/or not mentioned. */
export function Education({ draft, set }: { draft: Query; set: Set }) {
  const selected = listOf(draft, "edu");
  return (
    <div className="space-y-5">
      {DEGREE_LEVELS.map((level) => (
        <Field key={level.value} title={level.label}>
          <ChipGroup
            label={level.label}
            options={DEGREE_STATES}
            selected={selected.filter((v) => v.startsWith(`${level.value}:`)).map((v) => v.split(":")[1])}
            onToggle={(state) => set(toggleInList(draft, "edu", `${level.value}:${state}`))}
          />
        </Field>
      ))}
      <p className="text-sm text-muted-foreground">
        Pick more than one to allow several. Wording like &quot;or equivalent experience&quot; counts as preferred.
      </p>
    </div>
  );
}

/** Per shift: doesn't matter, must be mentioned, or hidden. */
export function Shifts({ vocab, draft, set }: { vocab: Vocabulary; draft: Query; set: Set }) {
  const shifts = tagChoices(vocab, "shift");
  const want = listOf(draft, "shift");
  const avoid = listOf(draft, "x_shift");
  if (!shifts.length) return <p className="text-sm text-muted-foreground">Loading…</p>;
  return (
    <div className="space-y-5">
      {shifts.map((s) => {
        const value = want.includes(s.value) ? "want" : avoid.includes(s.value) ? "avoid" : undefined;
        return (
          <Field key={s.value} title={s.label}>
            <Segmented
              label={s.label}
              anyLabel="Doesn't matter"
              options={[
                { value: "want", label: "Mentioned" },
                { value: "avoid", label: "Hide these" },
              ]}
              value={value}
              onChange={(v) => {
                let next = withValue(draft, "shift", want.filter((x) => x !== s.value));
                next = withValue(next, "x_shift", avoid.filter((x) => x !== s.value));
                if (v === "want") next = toggleInList(next, "shift", s.value);
                if (v === "avoid") next = toggleInList(next, "x_shift", s.value);
                set(next);
              }}
            />
          </Field>
        );
      })}
    </div>
  );
}

/** Graduation years and internship starts: this year and the next few. */
export function NewGrad({ draft, set }: { draft: Query; set: Set }) {
  const year = new Date().getFullYear();
  const years = [year - 1, year, year + 1, year + 2, year + 3].map(String);
  const starts = [year, year + 1].flatMap((y) =>
    (["spring", "summer", "fall"] as const).map((s) => ({
      value: `${y}-${s}`,
      label: `${s.charAt(0).toUpperCase()}${s.slice(1)} ${y}`,
    })),
  );
  return (
    <>
      <Field title="Graduation year" hint="Jobs that name your graduation year (new-grad and internship postings).">
        <ChipGroup
          label="Graduation year"
          options={years.map((y) => ({ value: y, label: y }))}
          selected={listOf(draft, "grad")}
          onToggle={(v) => set(toggleInList(draft, "grad", v))}
        />
      </Field>
      <Field title="Internship start" hint="Internships that say when they start.">
        <ChipGroup
          label="Internship start"
          options={starts}
          selected={listOf(draft, "start")}
          onToggle={(v) => set(toggleInList(draft, "start", v))}
        />
      </Field>
    </>
  );
}
