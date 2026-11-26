"use client";

import { Search, SlidersHorizontal } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { type Facets, type Vocabulary, api } from "@/api/client";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { DateInput } from "@/components/ui/date-picker";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input, inputClass } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import {
  APPLY,
  COMPANY_SIZES,
  EMPLOYMENT,
  type FilterKey,
  HOURS_PER_YEAR,
  NARROWING_KEYS,
  ORG_TYPES,
  PAY_PERIOD,
  POSTED_WITHIN,
  type Query,
  ROLE,
  SALARY_MIN,
  SENIORITY,
  SOURCE,
  WORKPLACE,
  activeCount,
  clearFilters,
  describe,
  listOf,
  removeFilter,
  toApiQuery,
  toggleInList,
  withValue,
  withoutKeys,
} from "@/lib/filters";
import { cn } from "cn";
import { CompanyPicker } from "./company-picker";
import {
  Education,
  FunctionPicker,
  NewGrad,
  Shifts,
  TagPicker,
  YearsRange,
  tagChoices,
  useVocabulary,
  vocabLabels,
} from "./detail-sections";
import { type Choice, CheckList, ChipGroup, Field, RadioList, RemovableChip, Segmented, facetChoices } from "./controls";
import { useFeed } from "./feed-context";
import { TagInput } from "./tag-input";

type Ctx = { facets: Facets; vocab: Vocabulary };

type Render = (draft: Query, set: (q: Query) => void, ctx: Ctx) => React.ReactNode;

type Section = {
  id: string;
  group: string;
  label: string;
  keys: FilterKey[];
  /** Extra words the filter search finds this section by. */
  terms?: string;
  render: Render;
};

const chips = (key: FilterKey, options: Choice[], label: string): Render =>
  function ChipSection(draft, set) {
    return (
      <ChipGroup
        label={label}
        options={options}
        selected={listOf(draft, key)}
        onToggle={(v) => set(toggleInList(draft, key, v))}
      />
    );
  };

const tags = (key: FilterKey, label: string, placeholder: string, hint: string) =>
  function TagSection(draft: Query, set: (q: Query) => void) {
    return (
      <div className="max-w-lg space-y-2">
        <TagInput
          label={label}
          value={listOf(draft, key)}
          onChange={(v) => set(withValue(draft, key, v))}
          separators={key === "location" ? [";"] : [","]}
          source={key === "location" ? "location" : undefined}
          placeholder={placeholder}
        />
        <p className="text-sm text-muted-foreground">{hint}</p>
      </div>
    );
  };

function ToggleRow({
  draft,
  set,
  filterKey,
  title,
  hint,
}: {
  draft: Query;
  set: (q: Query) => void;
  filterKey: FilterKey;
  title: string;
  hint: string;
}) {
  return (
    <label className="flex max-w-lg cursor-pointer items-start gap-3 rounded-xl bg-muted/60 p-4">
      <Switch
        checked={draft[filterKey] === "true"}
        onCheckedChange={(on) => set(withValue(draft, filterKey, on ? "true" : undefined))}
        className="mt-0.5"
      />
      <span className="space-y-1">
        <span className="block text-sm font-medium">{title}</span>
        <span className="block text-sm text-muted-foreground">{hint}</span>
      </span>
    </label>
  );
}

function DatePosted({ draft, set }: { draft: Query; set: (q: Query) => void }) {
  return (
    <div className="space-y-6">
      <RadioList
        options={POSTED_WITHIN}
        // With a custom range, no preset is picked (not even "Any time").
        value={draft.posted_from || draft.posted_to ? "range" : draft.posted_within}
        anyLabel="Any time"
        // A preset replaces a custom range.
        onChange={(v) => set(withValue(withoutKeys(draft, ["posted_from", "posted_to"]), "posted_within", v))}
        className="sm:grid sm:grid-flow-col sm:grid-rows-6 sm:gap-x-4"
      />
      <Field title="Custom range" hint="A start day, an end day, or both.">
        <div className="grid max-w-md grid-cols-2 gap-3">
          {(["posted_from", "posted_to"] as const).map((key) => (
            <label key={key} className="space-y-1.5">
              <span className="block text-xs font-medium text-muted-foreground">
                {key === "posted_from" ? "From" : "To"}
              </span>
              <DateInput
                aria-label={key === "posted_from" ? "Posted from" : "Posted to"}
                placeholder="MM/DD/YYYY"
                exactDay
                className={inputClass}
                value={draft[key] ?? null}
                // A range replaces a preset.
                onChange={(v) => set(withValue(withValue(draft, "posted_within", undefined), key, v ?? undefined))}
              />
            </label>
          ))}
        </div>
      </Field>
    </div>
  );
}

function ApplyProcess({ draft, set }: { draft: Query; set: (q: Query) => void }) {
  const selected = listOf(draft, "apply");
  return (
    <div className="grid max-w-xl gap-2">
      {APPLY.map((o) => {
        const on = selected.includes(o.value);
        return (
          <label
            key={o.value}
            className={cn(
              "flex cursor-pointer items-start gap-3 rounded-xl border p-4 transition-colors",
              on ? "border-primary/40 bg-primary/6 dark:bg-primary/12" : "border-border hover:bg-muted/60",
            )}
          >
            <Checkbox checked={on} onCheckedChange={() => set(toggleInList(draft, "apply", o.value))} className="mt-0.5" />
            <span className="space-y-1">
              <span className="block text-sm font-medium">{o.label}</span>
              <span className="block text-sm text-muted-foreground">{o.hint}</span>
            </span>
          </label>
        );
      })}
      <p className="text-sm text-muted-foreground">
        Jobs found only on job boards are in neither group: their apply link can lead anywhere.
      </p>
    </div>
  );
}

/** Minimum pay, typed per year or per hour; stored per year. */
function SalarySection({ draft, set }: { draft: Query; set: (q: Query) => void }) {
  const [unit, setUnit] = useState<"year" | "hour">("year");
  const yearly = Number(draft.salary_min) || 0;
  const shown = yearly ? String(unit === "hour" ? Math.round(yearly / HOURS_PER_YEAR) : yearly) : "";
  const [text, setText] = useState<string | null>(null); // non-null while typing

  function commit(raw: string) {
    const n = Number(raw.replace(/[$,\s]/g, ""));
    const value = Number.isFinite(n) && n > 0 ? Math.round(unit === "hour" ? n * HOURS_PER_YEAR : n) : undefined;
    let next = withValue(draft, "salary_min", value ? String(value) : undefined);
    if (!value) next = withValue(next, "include_unlisted_salary", undefined);
    set(next);
  }

  return (
    <div className="space-y-6">
      <Field title="Minimum pay" hint="Jobs whose top of range reaches at least this much.">
        <div className="flex max-w-md flex-wrap items-center gap-3">
          <label className="flex h-10 min-w-0 flex-1 items-center gap-1.5 rounded-xl border border-input bg-card px-3 focus-within:ring-3 focus-within:ring-ring/40">
            <span className="text-sm text-muted-foreground">$</span>
            <input
              inputMode="numeric"
              aria-label={unit === "hour" ? "Minimum hourly pay" : "Minimum yearly pay"}
              placeholder={unit === "hour" ? "e.g. 40" : "e.g. 90,000"}
              value={text ?? (shown ? Number(shown).toLocaleString("en-US") : "")}
              onChange={(e) => setText(e.target.value)}
              onBlur={(e) => {
                commit(e.target.value);
                setText(null);
              }}
              onKeyDown={(e) => {
                if (e.key === "Enter") e.currentTarget.blur();
              }}
              className="w-full min-w-0 bg-transparent text-sm outline-none placeholder:text-muted-foreground"
            />
          </label>
          <Segmented
            label="Pay per"
            options={[
              { value: "year", label: "Per year" },
              { value: "hour", label: "Per hour" },
            ]}
            value={unit}
            onChange={(v) => setUnit(v === "hour" ? "hour" : "year")}
          />
        </div>
        {unit === "year" ? (
          <ChipGroup
            label="Common minimums"
            options={SALARY_MIN}
            selected={draft.salary_min ? [draft.salary_min] : []}
            onToggle={(v) => set(withValue(draft, "salary_min", draft.salary_min === v ? undefined : v))}
          />
        ) : (
          <p className="text-sm text-muted-foreground">
            Hourly pay is compared as {HOURS_PER_YEAR.toLocaleString("en-US")} hours a year.
          </p>
        )}
      </Field>
      <Field title="Listed pay period" hint="How the posting states its pay.">
        <Segmented
          label="Listed pay period"
          options={PAY_PERIOD}
          anyLabel="Any"
          value={draft.pay_period}
          onChange={(v) => set(withValue(draft, "pay_period", v))}
        />
      </Field>
      <Field title="Unlisted salaries">
        <div className="space-y-2">
          <ToggleRow
            draft={draft}
            set={set}
            filterKey="salary_listed"
            title="Only jobs that list a salary"
            hint="Hides postings that leave pay out."
          />
          {draft.salary_min && draft.salary_listed !== "true" && (
            <ToggleRow
              draft={draft}
              set={set}
              filterKey="include_unlisted_salary"
              title="Keep jobs that don't list a salary"
              hint="With a minimum set, they're hidden unless this is on. Many postings leave pay out."
            />
          )}
        </div>
      </Field>
    </div>
  );
}

const SECTIONS: Section[] = [
  {
    id: "posted",
    group: "Results",
    label: "Date posted",
    keys: ["posted_within", "posted_from", "posted_to"],
    terms: "new recent age days weeks range",
    render: (draft, set) => <DatePosted draft={draft} set={set} />,
  },
  {
    id: "apply",
    group: "Results",
    label: "Apply process",
    keys: ["apply"],
    terms: "quick easy simple account workday form",
    render: (draft, set) => <ApplyProcess draft={draft} set={set} />,
  },
  {
    id: "exclude",
    group: "Results",
    label: "Exclude",
    keys: ["hide_stale", "exclude_title"],
    terms: "hide stale repost old titles words deal-breakers",
    render: (draft, set) => (
      <div className="space-y-6">
        <ToggleRow
          draft={draft}
          set={set}
          filterKey="hide_stale"
          title="Hide possibly stale jobs"
          hint="Jobs open more than 60 days, or reposted more than twice, are often no longer hiring."
        />
        {tags(
          "exclude_title",
          "Hide titles with these words",
          "e.g. manager, then Enter",
          "Hides jobs whose title has any of these words or phrases.",
        )(draft, set)}
      </div>
    ),
  },
  {
    id: "function",
    group: "Role",
    label: "Job function",
    keys: ["function"],
    terms: "department category field area role type software nursing sales engineering",
    render: (draft, set, { vocab }) => <FunctionPicker vocab={vocab} draft={draft} set={set} />,
  },
  {
    id: "department",
    group: "Role",
    label: "Department (as posted)",
    keys: ["department"],
    terms: "team",
    render: (draft, set, { facets }) => (
      <CheckList
        options={facetChoices(facets.departments)}
        selected={listOf(draft, "department")}
        onToggle={(v) => set(toggleInList(draft, "department", v))}
        className="sm:grid sm:grid-cols-2 sm:gap-x-4"
      />
    ),
  },
  {
    id: "titles",
    group: "Role",
    label: "Titles & keywords",
    keys: ["title", "q", "skill", "x_skill"],
    terms: "job title keyword search skills technology tech stack tools boolean",
    render: (draft, set, { vocab }) => (
      <div className="space-y-6">
        <Field title="Job titles">
          {tags(
            "title",
            "Job titles",
            "e.g. Software Engineer, then Enter",
            "Any of these. A job matches when its title has every word of one of them, in any order.",
          )(draft, set)}
        </Field>
        <Field
          title="Keywords"
          hint='Searched in the title, company and description. Use "quotes" for a phrase, OR between options, and -word to leave a word out.'
        >
          <Input
            value={draft.q ?? ""}
            onChange={(e) => set(withValue(draft, "q", e.target.value))}
            placeholder='e.g. rust OR go -"on-call"'
            className="max-w-lg"
          />
        </Field>
        <Field title="Technology & skills" hint="Jobs whose posting names any of these.">
          <TagPicker
            label="Skills"
            choices={tagChoices(vocab, "skill", true)}
            draft={draft}
            set={set}
            filterKey="skill"
            placeholder="e.g. React, then pick from the list"
          />
        </Field>
        <Field title="Excluded skills" hint="Hide jobs whose posting names any of these.">
          <TagPicker
            label="Excluded skills"
            choices={tagChoices(vocab, "skill", true)}
            draft={draft}
            set={set}
            filterKey="x_skill"
            placeholder="e.g. PHP"
          />
        </Field>
      </div>
    ),
  },
  {
    id: "seniority",
    group: "Role",
    label: "Experience",
    keys: ["seniority", "role", "years_min", "years_max", "years_unlisted", "grad", "start"],
    terms: "seniority level entry junior senior staff director intern years manager individual contributor new grad graduation",
    render: (draft, set) => (
      <div className="space-y-6">
        <Field title="Seniority">
          <ChipGroup
            label="Seniority"
            options={SENIORITY}
            selected={listOf(draft, "seniority")}
            onToggle={(v) => set(toggleInList(draft, "seniority", v))}
          />
        </Field>
        <Field title="Role type" hint="Whether the job manages people.">
          <Segmented
            label="Role type"
            anyLabel="Any"
            options={ROLE}
            value={draft.role}
            onChange={(v) => set(withValue(draft, "role", v))}
          />
        </Field>
        <Field title="Years of experience" hint="The fewest years the posting asks for.">
          <YearsRange draft={draft} set={set} />
        </Field>
        <NewGrad draft={draft} set={set} />
      </div>
    ),
  },
  {
    id: "type",
    group: "Role",
    label: "Job type",
    keys: ["employment_type", "commit"],
    terms: "commitment full-time part-time contract internship temporary seasonal temp",
    render: (draft, set, { vocab }) => (
      <div className="space-y-6">
        <Field title="Job type">
          <ChipGroup
            label="Job type"
            options={EMPLOYMENT}
            selected={listOf(draft, "employment_type")}
            onToggle={(v) => set(toggleInList(draft, "employment_type", v))}
          />
        </Field>
        <Field title="Temporary work" hint="Postings that say the job is temporary or seasonal.">
          <ChipGroup
            label="Temporary work"
            options={tagChoices(vocab, "commit")}
            selected={listOf(draft, "commit")}
            onToggle={(v) => set(toggleInList(draft, "commit", v))}
          />
        </Field>
      </div>
    ),
  },
  {
    id: "location",
    group: "Location",
    label: "Location",
    keys: ["location"],
    terms: "city state place",
    render: tags(
      "location",
      "Locations",
      "e.g. Austin, TX, then Enter",
      "Any of these: a city with its state, or a whole state. When Remote is picked under Workplace, remote jobs show too.",
    ),
  },
  {
    id: "workplace",
    group: "Location",
    label: "Workplace",
    keys: ["workplace"],
    terms: "remote hybrid onsite office",
    render: chips("workplace", WORKPLACE, "Workplace"),
  },
  {
    id: "salary",
    group: "Pay & benefits",
    label: "Salary",
    keys: ["salary_min", "include_unlisted_salary", "pay_period", "salary_listed"],
    terms: "pay compensation hourly yearly wage",
    render: (draft, set) => <SalarySection draft={draft} set={set} />,
  },
  {
    id: "perks",
    group: "Pay & benefits",
    label: "Benefits & perks",
    keys: ["perk"],
    terms: "401k retirement pto vacation time off parental leave tuition relocation visa four-day",
    render: (draft, set, { vocab }) => (
      <Field title="Perks mentioned" hint="Jobs whose posting mentions every perk picked. Not every employer lists every perk.">
        <ChipGroup
          label="Perks"
          options={tagChoices(vocab, "perk")}
          selected={listOf(draft, "perk")}
          onToggle={(v) => set(toggleInList(draft, "perk", v))}
        />
      </Field>
    ),
  },
  {
    id: "authorization",
    group: "Requirements",
    label: "Work authorization",
    keys: ["hide_no_sponsorship", "hide_citizens_only", "hide_clearance"],
    terms: "visa sponsorship h1b citizen citizenship clearance",
    render: (draft, set) => (
      <div className="space-y-2">
        <ToggleRow
          draft={draft}
          set={set}
          filterKey="hide_no_sponsorship"
          title="Hide jobs that won't sponsor a visa"
          hint={'Read from the posting: "unable to sponsor", "no visa sponsorship", "without sponsorship" and similar.'}
        />
        <ToggleRow
          draft={draft}
          set={set}
          filterKey="hide_citizens_only"
          title="Hide jobs for U.S. citizens only"
          hint={'Read from the posting: "must be a U.S. citizen", or a security clearance required.'}
        />
        <ToggleRow
          draft={draft}
          set={set}
          filterKey="hide_clearance"
          title="Hide jobs that mention a security clearance"
          hint="Secret, Top Secret, Public Trust and the like."
        />
        <p className="pt-2 text-sm text-muted-foreground">
          Looking for employers that do sponsor? Pick Visa sponsorship under Benefits &amp; perks.
        </p>
      </div>
    ),
  },
  {
    id: "education",
    group: "Requirements",
    label: "Education",
    keys: ["edu"],
    terms: "degree bachelor master phd doctorate associate college",
    render: (draft, set) => <Education draft={draft} set={set} />,
  },
  {
    id: "certs",
    group: "Requirements",
    label: "Licenses & certifications",
    keys: ["cert", "x_cert", "no_cert"],
    terms: "license certification credential rn cpa pmp cdl aws",
    render: (draft, set, { vocab }) => (
      <div className="space-y-6">
        <Field title="Include" hint="Jobs that mention any of these.">
          <TagPicker
            label="Licenses to include"
            choices={tagChoices(vocab, "cert", true)}
            draft={draft}
            set={set}
            filterKey="cert"
            placeholder="e.g. RN, PMP, CDL"
          />
        </Field>
        <Field title="Exclude" hint="Hide jobs that ask for any of these.">
          <TagPicker
            label="Licenses to exclude"
            choices={tagChoices(vocab, "cert", true)}
            draft={draft}
            set={set}
            filterKey="x_cert"
            placeholder="e.g. Driver's license"
          />
        </Field>
        <ToggleRow
          draft={draft}
          set={set}
          filterKey="no_cert"
          title="No license or certification needed"
          hint="Hides every job that mentions one."
        />
      </div>
    ),
  },
  {
    id: "clearance",
    group: "Requirements",
    label: "Security clearance",
    keys: ["clearance"],
    terms: "secret top secret ts/sci public trust polygraph government",
    render: (draft, set, { vocab }) => (
      <Field title="Clearance level" hint="Any of these.">
        <ChipGroup
          label="Clearance level"
          options={[{ value: "none", label: "No clearance mentioned" }, ...tagChoices(vocab, "clearance")]}
          selected={listOf(draft, "clearance")}
          onToggle={(v) => set(toggleInList(draft, "clearance", v))}
        />
      </Field>
    ),
  },
  {
    id: "languages",
    group: "Requirements",
    label: "Languages",
    keys: ["lang", "x_lang"],
    terms: "spanish bilingual french mandarin language fluent",
    render: (draft, set, { vocab }) => (
      <div className="space-y-6">
        <Field title="Languages asked for" hint="Jobs that ask for any of these.">
          <TagPicker
            label="Languages"
            choices={tagChoices(vocab, "lang", true)}
            draft={draft}
            set={set}
            filterKey="lang"
            placeholder="e.g. Spanish"
          />
        </Field>
        <Field title="Excluded languages" hint="Hide jobs that ask for any of these.">
          <TagPicker
            label="Excluded languages"
            choices={tagChoices(vocab, "lang", true)}
            draft={draft}
            set={set}
            filterKey="x_lang"
            placeholder="e.g. German"
          />
        </Field>
      </div>
    ),
  },
  {
    id: "encourage",
    group: "Requirements",
    label: "Encouraged to apply",
    keys: ["encourage"],
    terms: "veterans military fair chance second chance records",
    render: (draft, set, { vocab }) => (
      <Field title="Applicant groups" hint="Jobs that welcome these applicants in so many words (beyond equal-opportunity boilerplate).">
        <ChipGroup
          label="Applicant groups"
          options={tagChoices(vocab, "encourage")}
          selected={listOf(draft, "encourage")}
          onToggle={(v) => set(toggleInList(draft, "encourage", v))}
        />
      </Field>
    ),
  },
  {
    id: "shifts",
    group: "Schedule & travel",
    label: "Shifts & schedules",
    keys: ["shift", "x_shift"],
    terms: "night weekend overtime on-call evening day holiday",
    render: (draft, set, { vocab }) => <Shifts vocab={vocab} draft={draft} set={set} />,
  },
  {
    id: "travel",
    group: "Schedule & travel",
    label: "Travel",
    keys: ["travel"],
    terms: "travel percent",
    render: (draft, set, { vocab }) => (
      <Field title="Travel" hint="From the share of time a posting says ('up to 25%') or its wording. Any of these.">
        <ChipGroup
          label="Travel"
          options={[{ value: "none", label: "No travel mentioned" }, ...tagChoices(vocab, "travel")]}
          selected={listOf(draft, "travel")}
          onToggle={(v) => set(toggleInList(draft, "travel", v))}
        />
      </Field>
    ),
  },
  {
    id: "company",
    group: "Company",
    label: "Company",
    keys: ["company", "exclude_company", "hide_staffing"],
    terms: "employer exclude staffing agency recruiter",
    render: (draft, set, { facets }) => (
      <div className="space-y-6">
        <Field title="Companies" hint="Only jobs from these employers.">
          <CompanyPicker
            top={facetChoices(facets.companies)}
            selected={listOf(draft, "company")}
            onToggle={(v) => set(toggleInList(draft, "company", v))}
            className="max-w-md"
          />
        </Field>
        <Field title="Excluded companies" hint="Never show jobs from these employers.">
          <CompanyPicker
            top={[]}
            selected={listOf(draft, "exclude_company")}
            onToggle={(v) => set(toggleInList(draft, "exclude_company", v))}
            className="max-w-md"
          />
        </Field>
        <ToggleRow
          draft={draft}
          set={set}
          filterKey="hide_staffing"
          title="Hide staffing agencies"
          hint="Agencies by name, and postings that recruit for an unnamed client."
        />
      </div>
    ),
  },
  {
    id: "industry",
    group: "Company",
    label: "Industry & type",
    keys: ["industry", "x_industry", "org_type", "x_org_type"],
    terms: "industry sector public private nonprofit government startup",
    render: (draft, set, { vocab }) => (
      <div className="space-y-6">
        <FactsCoverage vocab={vocab} />
        <Field title="Industries" hint="Companies in any of these.">
          <TagPicker
            label="Industries"
            choices={vocab.industries.map((i) => ({ value: i.value, label: i.value, count: i.count }))}
            draft={draft}
            set={set}
            filterKey="industry"
            placeholder="e.g. software, banking, health care"
          />
        </Field>
        <Field title="Excluded industries" hint="Hide companies in any of these.">
          <TagPicker
            label="Excluded industries"
            choices={vocab.industries.map((i) => ({ value: i.value, label: i.value, count: i.count }))}
            draft={draft}
            set={set}
            filterKey="x_industry"
            placeholder="e.g. defense industry"
          />
        </Field>
        <Field title="Organization type" hint="Any of these.">
          <ChipGroup
            label="Organization type"
            options={ORG_TYPES}
            selected={listOf(draft, "org_type")}
            onToggle={(v) => set(toggleInList(draft, "org_type", v))}
          />
        </Field>
        <Field title="Excluded organization types">
          <ChipGroup
            label="Excluded organization types"
            options={ORG_TYPES}
            selected={listOf(draft, "x_org_type")}
            onToggle={(v) => set(toggleInList(draft, "x_org_type", v))}
          />
        </Field>
      </div>
    ),
  },
  {
    id: "size",
    group: "Company",
    label: "Size",
    keys: ["size"],
    terms: "headcount employees startup enterprise",
    render: (draft, set, { vocab }) => (
      <div className="space-y-6">
        <FactsCoverage vocab={vocab} />
        <Field title="Employees" hint="Any of these.">
          <CheckList
            options={COMPANY_SIZES.map((o) => ({ value: o.value, label: `${o.label} (${o.hint.toLowerCase()})` }))}
            selected={listOf(draft, "size")}
            onToggle={(v) => set(toggleInList(draft, "size", v))}
            className="max-w-sm"
          />
        </Field>
      </div>
    ),
  },
  {
    id: "founded",
    group: "Company",
    label: "Founded",
    keys: ["founded_min", "founded_max"],
    terms: "founding year age established",
    render: (draft, set, { vocab }) => (
      <div className="space-y-6">
        <FactsCoverage vocab={vocab} />
        <Field title="Founded between">
          <div className="flex max-w-xs items-center gap-3">
            {(["founded_min", "founded_max"] as const).map((key, i) => (
              <div key={key} className="contents">
                {i === 1 && <span className="text-muted-foreground">-</span>}
                <Input
                  inputMode="numeric"
                  aria-label={key === "founded_min" ? "Founded in or after" : "Founded in or before"}
                  placeholder={key === "founded_min" ? "Any year" : "Present"}
                  value={draft[key] ?? ""}
                  onChange={(e) => {
                    const v = e.target.value.replace(/\D/g, "").slice(0, 4);
                    set(withValue(draft, key, v || undefined));
                  }}
                />
              </div>
            ))}
          </div>
        </Field>
      </div>
    ),
  },
  {
    id: "hq",
    group: "Company",
    label: "HQ country",
    keys: ["hq", "x_hq"],
    terms: "headquarters country foreign american",
    render: (draft, set, { vocab }) => (
      <div className="space-y-6">
        <FactsCoverage vocab={vocab} />
        <Field title="Headquartered in" hint="Any of these countries.">
          <TagPicker
            label="Headquarters countries"
            choices={vocab.countries.map((c) => ({ value: c.value, label: c.value, count: c.count }))}
            draft={draft}
            set={set}
            filterKey="hq"
            placeholder="e.g. United States"
          />
        </Field>
        <Field title="Not headquartered in">
          <TagPicker
            label="Excluded headquarters countries"
            choices={vocab.countries.map((c) => ({ value: c.value, label: c.value, count: c.count }))}
            draft={draft}
            set={set}
            filterKey="x_hq"
            placeholder="e.g. India"
          />
        </Field>
      </div>
    ),
  },
  {
    id: "source",
    group: "Company",
    label: "Job source",
    keys: ["source"],
    terms: "platform board greenhouse lever ashby workday",
    render: (draft, set) => (
      <CheckList
        options={SOURCE}
        selected={listOf(draft, "source")}
        onToggle={(v) => set(toggleInList(draft, "source", v))}
        className="sm:grid sm:grid-cols-2 sm:gap-x-4"
      />
    ),
  },
];

/** How many companies the Company facts filters know about. */
function FactsCoverage({ vocab }: { vocab: Vocabulary }) {
  if (!vocab.companies_total) return null;
  const pct = Math.round((vocab.companies_known / vocab.companies_total) * 100);
  return (
    <p className="max-w-lg rounded-xl bg-muted/60 px-4 py-3 text-sm text-muted-foreground">
      From Wikidata, known for {vocab.companies_known.toLocaleString("en-US")} of{" "}
      {vocab.companies_total.toLocaleString("en-US")} companies ({pct}%). Using these filters leaves out companies we
      know nothing about.
    </p>
  );
}

const SELECTED = "selected";

/** Every applied filter, grouped by section. */
function SelectedFilters({
  draft,
  set,
  onOpen,
  labels,
}: {
  draft: Query;
  set: (q: Query) => void;
  onOpen: (id: string) => void;
  labels: Record<string, string>;
}) {
  const groups = SECTIONS.map((s) => ({ section: s, active: describe(draft, s.keys, labels) })).filter(
    (g) => g.active.length,
  );
  if (!groups.length) {
    return <p className="text-sm text-muted-foreground">No filters yet. Pick a category on the left to add some.</p>;
  }
  return (
    <div className="divide-y divide-border/60">
      {groups.map(({ section, active }) => (
        <div key={section.id} className="space-y-2.5 py-4 first:pt-0">
          <div className="flex items-center justify-between gap-3">
            <button
              type="button"
              onClick={() => onOpen(section.id)}
              className="text-[0.6875rem] font-semibold tracking-wider text-muted-foreground uppercase hover:text-foreground"
            >
              {section.label}
            </button>
            <ClearButton onClick={() => set(withoutKeys(draft, section.keys))} />
          </div>
          <div className="flex flex-wrap gap-1.5">
            {active.map((f) => (
              <RemovableChip key={`${f.key}|${f.value ?? ""}`} label={f.label} onRemove={() => set(removeFilter(draft, f))} />
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}

function ClearButton({ onClick }: { onClick: () => void }) {
  return (
    <button type="button" onClick={onClick} className="text-xs font-medium text-muted-foreground hover:text-foreground">
      Clear
    </button>
  );
}

export function AllFiltersDialog({ facets }: { facets: Facets }) {
  const { query, apply } = useFeed();
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState<Query>(query);
  const [sectionId, setSectionId] = useState(SECTIONS[0].id);
  const [search, setSearch] = useState("");
  const count = useMatchCount(draft, open);
  const vocab = useVocabulary(open);
  const labels = useMemo(() => vocabLabels(vocab), [vocab]);

  const visible = useMemo(() => {
    const s = search.trim().toLowerCase();
    return s ? SECTIONS.filter((x) => `${x.group} ${x.label} ${x.terms ?? ""}`.toLowerCase().includes(s)) : SECTIONS;
  }, [search]);
  const section = sectionId === SELECTED && !search.trim() ? null : (visible.find((x) => x.id === sectionId) ?? visible[0]);
  const total = activeCount(query);
  const draftCount = activeCount(draft);
  const active = describe(draft, NARROWING_KEYS, labels);

  function onOpenChange(next: boolean) {
    if (next) {
      setDraft(query); // start from what's applied
      setSectionId(activeCount(query) ? SELECTED : SECTIONS[0].id);
      setSearch("");
    }
    setOpen(next);
  }

  const navButton = (id: string, label: string, n: number, current: boolean) => (
    <button
      type="button"
      onClick={() => setSectionId(id)}
      aria-current={current ? "page" : undefined}
      className={cn(
        "flex shrink-0 items-center justify-between gap-2 rounded-xl px-3 py-2 text-left text-sm transition-colors",
        current ? "bg-primary/12 font-medium text-primary dark:bg-primary/20" : "text-foreground/80 hover:bg-muted",
      )}
    >
      {label}
      {n > 0 && (
        <span className="rounded-full bg-primary px-1.5 text-[0.6875rem] text-primary-foreground tabular-nums">{n}</span>
      )}
    </button>
  );

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogTrigger asChild>
        <Button variant={total ? "default" : "bordered"} size="sm" shape="pill">
          <SlidersHorizontal /> All filters
          {total > 0 && (
            <span className="ml-0.5 rounded-full bg-primary-foreground/20 px-1.5 text-xs tabular-nums">{total}</span>
          )}
        </Button>
      </DialogTrigger>
      <DialogContent className="flex h-[min(46rem,calc(100dvh-2rem))] flex-col gap-0 overflow-hidden p-0 sm:max-w-4xl">
        <div className="flex items-center gap-3 border-b border-border/70 p-4 pr-14">
          <DialogTitle className="sr-only">All filters</DialogTitle>
          <DialogDescription className="sr-only">Choose filters, then show the matching jobs.</DialogDescription>
          <label className="flex h-10 flex-1 items-center gap-2.5 rounded-xl bg-muted px-3">
            <Search className="size-4 text-muted-foreground" />
            <input
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search filters"
              aria-label="Search filters"
              className="w-full bg-transparent text-sm outline-none placeholder:text-muted-foreground"
            />
          </label>
        </div>

        <div className="flex min-h-0 flex-1 flex-col sm:flex-row">
          <nav
            aria-label="Filter categories"
            className="flex shrink-0 gap-1 overflow-x-auto border-b border-border/70 p-2 [scrollbar-width:none] sm:w-56 sm:flex-col sm:overflow-y-auto sm:border-r sm:border-b-0 sm:p-3 sm:[scrollbar-width:thin] [&::-webkit-scrollbar]:hidden sm:[&::-webkit-scrollbar]:block"
          >
            {!search.trim() && navButton(SELECTED, "Selected filters", draftCount, section === null)}
            {visible.map((s, i) => {
              const newGroup = i === 0 || visible[i - 1].group !== s.group;
              return (
                <div key={s.id} className="contents">
                  {newGroup && (
                    <p
                      className={cn(
                        "hidden px-3 pb-1 text-[0.6875rem] font-semibold tracking-wider text-muted-foreground uppercase sm:block",
                        i === 0 && search.trim() ? "pt-1" : "pt-5",
                      )}
                    >
                      {s.group}
                    </p>
                  )}
                  {navButton(s.id, s.label, activeCount(draft, s.keys), section?.id === s.id)}
                </div>
              );
            })}
            {visible.length === 0 && <p className="px-3 py-2 text-sm text-muted-foreground">No filters match.</p>}
          </nav>

          <section className="min-h-0 flex-1 overflow-y-auto">
            <div className="sticky top-0 z-10 flex items-center justify-between gap-3 border-b border-border/60 bg-card/95 px-5 py-3.5 backdrop-blur sm:px-6">
              <h3 className="text-base font-semibold">{section ? section.label : "Selected filters"}</h3>
              {section
                ? activeCount(draft, section.keys) > 0 && (
                    <ClearButton onClick={() => setDraft(withoutKeys(draft, section.keys))} />
                  )
                : draftCount > 0 && <ClearButton onClick={() => setDraft(clearFilters(draft))} />}
            </div>
            <div className="p-5 sm:p-6">
              {section ? (
                section.render(draft, setDraft, { facets, vocab })
              ) : (
                <SelectedFilters draft={draft} set={setDraft} onOpen={setSectionId} labels={labels} />
              )}
            </div>
          </section>
        </div>

        <div className="border-t border-border/70 bg-card">
          {active.length > 0 && (
            <div className="flex gap-1.5 overflow-x-auto px-5 pt-3 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
              {active.map((f) => (
                <RemovableChip
                  key={`${f.key}|${f.value ?? ""}`}
                  label={f.label}
                  onRemove={() => setDraft(removeFilter(draft, f))}
                />
              ))}
            </div>
          )}
          <DialogFooter className="m-0 flex-row items-center justify-between gap-3 border-0 bg-transparent px-5 py-3.5 sm:justify-between">
            <Button variant="light" onClick={() => setDraft(clearFilters(draft))} disabled={draftCount === 0}>
              Clear all
            </Button>
            <Button
              onClick={() => {
                apply(draft);
                setOpen(false);
              }}
              className="min-w-40"
            >
              {count === null ? "Show results" : `Show ${count.toLocaleString("en-US")} ${count === 1 ? "job" : "jobs"}`}
            </Button>
          </DialogFooter>
        </div>
      </DialogContent>
    </Dialog>
  );
}

/** Asks the API how many jobs the draft filters match (debounced). */
function useMatchCount(draft: Query, enabled: boolean): number | null {
  const [count, setCount] = useState<number | null>(null);
  useEffect(() => {
    if (!enabled) return;
    const controller = new AbortController();
    const timer = setTimeout(() => {
      api
        .GET("/jobs", {
          params: { query: { ...toApiQuery(draft, 1), page: undefined } },
          signal: controller.signal,
        })
        .then(({ data }) => setCount(data?.total ?? null))
        .catch(() => {});
    }, 250);
    return () => {
      clearTimeout(timer);
      controller.abort();
    };
  }, [draft, enabled]);
  return enabled ? count : null;
}

// Every narrowing key belongs to a section (checked in development).
if (process.env.NODE_ENV !== "production") {
  const covered = new Set(SECTIONS.flatMap((s) => s.keys));
  const missing = NARROWING_KEYS.filter((k) => !covered.has(k));
  if (missing.length) console.warn("Filters without a section:", missing);
}
