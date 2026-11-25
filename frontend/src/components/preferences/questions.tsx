"use client";

import { Check } from "lucide-react";
import { useId } from "react";
import type { CompanySize, EmploymentType, Preferences, Seniority, Workplace } from "@/api/client";
import { CompanyPicker } from "@/components/filters/company-picker";
import { TagInput } from "@/components/filters/tag-input";
import { Switch } from "@/components/ui/switch";
import { EMPLOYMENT, SALARY_MIN, SENIORITY, WORKPLACE } from "@/lib/filters";
import { cn } from "cn";

/**
 * The questionnaire's questions, one block per topic. The wizard shows one
 * block per step; the preferences page shows them all at once.
 */
type Props = { value: Preferences; onChange: (next: Preferences) => void };

export const EMPTY_PREFERENCES: Preferences = {
  titles: [],
  seniority: [],
  employment_types: [],
  workplaces: [],
  locations: [],
  salary_min: null,
  include_unlisted_salary: true,
  company_sizes: [],
  industries: [],
  excluded_companies: [],
  excluded_title_words: [],
  work_authorization: { authorized_us: null, needs_sponsorship: null, us_citizen: null },
};

const TITLE_SUGGESTIONS = [
  "Software Engineer",
  "Frontend Engineer",
  "Backend Engineer",
  "Full Stack Engineer",
  "Data Engineer",
  "Data Scientist",
  "Machine Learning Engineer",
  "Product Manager",
  "Product Designer",
  "DevOps Engineer",
];

const INDUSTRY_SUGGESTIONS = [
  "Software",
  "AI",
  "Fintech",
  "Healthcare",
  "E-commerce",
  "Education",
  "Security",
  "Climate",
  "Gaming",
  "Government",
];

const SENIORITY_HINTS: Record<Seniority, string> = {
  intern: "Internships and co-ops",
  entry: "New grad, 0 to 2 years",
  mid: "About 2 to 5 years",
  senior: "About 5+ years",
  staff_plus: "Staff, principal, lead",
};

const WORKPLACE_HINTS: Record<Workplace, string> = {
  remote: "Work from anywhere in the US",
  hybrid: "Some days in an office",
  onsite: "In an office every day",
};

export const COMPANY_SIZES: { value: CompanySize; label: string; hint: string }[] = [
  { value: "startup", label: "Startup", hint: "1 to 50 people" },
  { value: "small", label: "Small", hint: "51 to 200" },
  { value: "medium", label: "Mid-size", hint: "201 to 1,000" },
  { value: "large", label: "Large", hint: "1,001 to 10,000" },
  { value: "enterprise", label: "Enterprise", hint: "10,000+" },
];

/** A titled block of one question. */
export function Question({
  title,
  hint,
  children,
}: {
  title: string;
  hint?: React.ReactNode;
  children: React.ReactNode;
}) {
  const id = useId();
  return (
    <div role="group" aria-labelledby={id} className="space-y-3">
      <div className="space-y-1">
        <h3 id={id} className="text-[0.9375rem] font-semibold">
          {title}
        </h3>
        {hint && <p className="text-sm text-muted-foreground">{hint}</p>}
      </div>
      {children}
    </div>
  );
}

/** Multi-select cards. */
function ChoiceGrid<T extends string>({
  options,
  selected,
  onChange,
  columns = 2,
}: {
  options: { value: T; label: string; hint?: string }[];
  selected: T[];
  onChange: (next: T[]) => void;
  columns?: 2 | 3;
}) {
  return (
    <div className={cn("grid gap-2", columns === 3 ? "sm:grid-cols-3" : "sm:grid-cols-2")}>
      {options.map((o) => {
        const on = selected.includes(o.value);
        return (
          <button
            key={o.value}
            type="button"
            aria-pressed={on}
            onClick={() => onChange(on ? selected.filter((v) => v !== o.value) : [...selected, o.value])}
            className={cn(
              "flex items-start gap-3 rounded-2xl border px-4 py-3 text-left transition-[background-color,border-color,transform] active:scale-[0.98]",
              on
                ? "border-primary/50 bg-primary/8 dark:bg-primary/15"
                : "border-border bg-card hover:border-primary/30 hover:bg-muted/60",
            )}
          >
            <span
              aria-hidden
              className={cn(
                "mt-0.5 flex size-5 shrink-0 items-center justify-center rounded-md border transition-colors",
                on ? "border-primary bg-primary text-primary-foreground" : "border-border bg-card",
              )}
            >
              {on && <Check className="size-3.5" />}
            </span>
            <span className="min-w-0">
              <span className="block text-sm font-medium">{o.label}</span>
              {o.hint && <span className="block text-xs text-muted-foreground">{o.hint}</span>}
            </span>
          </button>
        );
      })}
    </div>
  );
}

/** Yes / No, or neither. Choosing the selected answer again clears it. */
function YesNo({
  label,
  hint,
  value,
  onChange,
}: {
  label: string;
  hint?: string;
  value: boolean | null | undefined;
  onChange: (v: boolean | null) => void;
}) {
  const id = useId();
  return (
    <div
      role="radiogroup"
      aria-labelledby={id}
      className="flex flex-col gap-3 rounded-2xl border border-border bg-card px-4 py-3 sm:flex-row sm:items-center sm:justify-between"
    >
      <div className="min-w-0">
        <p id={id} className="text-sm font-medium">
          {label}
        </p>
        {hint && <p className="text-xs text-muted-foreground">{hint}</p>}
      </div>
      <div className="flex shrink-0 gap-1 rounded-xl bg-muted p-1">
        {[
          { v: true, text: "Yes" },
          { v: false, text: "No" },
        ].map(({ v, text }) => (
          <button
            key={text}
            type="button"
            role="radio"
            aria-checked={value === v}
            onClick={() => onChange(value === v ? null : v)}
            className={cn(
              "h-8 min-w-16 rounded-lg px-3 text-sm font-medium transition-colors",
              value === v ? "bg-card text-foreground shadow-soft" : "text-muted-foreground hover:text-foreground",
            )}
          >
            {text}
          </button>
        ))}
      </div>
    </div>
  );
}

export function RolesQuestions({ value, onChange }: Props) {
  return (
    <div className="space-y-7">
      <Question
        title="What job titles are you looking for?"
        hint="Add a few. A job matches when its title has every word of one of them, in any order."
      >
        <TagInput
          label="Job titles"
          value={value.titles}
          onChange={(titles) => onChange({ ...value, titles })}
          placeholder="e.g. Software Engineer, then Enter"
          suggestions={TITLE_SUGGESTIONS}
        />
      </Question>
      <Question title="What level?" hint="Pick every level you'd consider. None picked means any level.">
        <ChoiceGrid
          options={SENIORITY.map((o) => ({
            value: o.value as Seniority,
            label: o.label,
            hint: SENIORITY_HINTS[o.value as Seniority],
          }))}
          selected={value.seniority}
          onChange={(seniority) => onChange({ ...value, seniority })}
        />
      </Question>
    </div>
  );
}

export function JobTypeQuestions({ value, onChange }: Props) {
  return (
    <Question title="What kind of work?" hint="None picked means any kind.">
      <ChoiceGrid
        options={EMPLOYMENT.map((o) => ({ value: o.value as EmploymentType, label: o.label }))}
        selected={value.employment_types}
        onChange={(employment_types) => onChange({ ...value, employment_types })}
      />
    </Question>
  );
}

export function LocationQuestions({ value, onChange }: Props) {
  const remote = value.workplaces.includes("remote");
  return (
    <div className="space-y-7">
      <Question title="Where do you want to work?" hint="None picked means any arrangement.">
        <ChoiceGrid
          columns={3}
          options={WORKPLACE.map((o) => ({
            value: o.value as Workplace,
            label: o.label,
            hint: WORKPLACE_HINTS[o.value as Workplace],
          }))}
          selected={value.workplaces}
          onChange={(workplaces) => onChange({ ...value, workplaces })}
        />
      </Question>
      <Question
        title="Which places?"
        hint={
          remote
            ? "For hybrid and on-site jobs. Remote jobs show wherever they're based."
            : "A city with its state (Austin, TX), or a whole state (Texas). Leave empty for anywhere in the US."
        }
      >
        <TagInput
          label="Locations"
          value={value.locations}
          onChange={(locations) => onChange({ ...value, locations })}
          separators={[";"]}
          placeholder="e.g. Austin, TX, then Enter"
        />
      </Question>
    </div>
  );
}

export function AuthorizationQuestions({ value, onChange }: Props) {
  const a = value.work_authorization;
  const set = (patch: Partial<typeof a>) => onChange({ ...value, work_authorization: { ...a, ...patch } });
  return (
    <Question
      title="Three quick questions"
      hint="Used to hide jobs you can't take, and saved with your application answers for autofill."
    >
      <div className="space-y-2">
        <YesNo
          label="Are you authorized to work in the US?"
          value={a.authorized_us}
          onChange={(authorized_us) => set({ authorized_us })}
        />
        <YesNo
          label="Will you need visa sponsorship, now or later?"
          hint="Yes hides jobs that say they won't sponsor."
          value={a.needs_sponsorship}
          onChange={(needs_sponsorship) => set({ needs_sponsorship })}
        />
        <YesNo
          label="Are you a U.S. citizen?"
          hint="No hides jobs open only to citizens or security-clearance holders."
          value={a.us_citizen}
          onChange={(us_citizen) => set({ us_citizen })}
        />
      </div>
    </Question>
  );
}

export function SalaryQuestions({ value, onChange }: Props) {
  const id = useId();
  const amount = value.salary_min ?? null;
  return (
    <Question title="What's your minimum salary?" hint="Yearly, in US dollars. Hourly pay is converted at 2,080 hours a year.">
      <div className="space-y-4">
        <div className="flex flex-wrap gap-1.5">
          {SALARY_MIN.map((o) => {
            const on = amount === Number(o.value);
            return (
              <button
                key={o.value}
                type="button"
                aria-pressed={on}
                onClick={() => onChange({ ...value, salary_min: on ? null : Number(o.value) })}
                className={cn(
                  "h-9 rounded-full border px-3.5 text-sm font-medium transition-colors",
                  on
                    ? "border-primary/50 bg-primary/10 text-primary dark:bg-primary/20"
                    : "border-border bg-card hover:bg-muted",
                )}
              >
                {o.label}
              </button>
            );
          })}
        </div>
        <label htmlFor={id} className="flex h-11 max-w-60 items-center gap-2 rounded-xl bg-muted px-3 focus-within:bg-card focus-within:ring-3 focus-within:ring-ring/25">
          <span className="text-muted-foreground">$</span>
          <input
            id={id}
            inputMode="numeric"
            aria-label="Minimum yearly salary"
            placeholder="Or type an amount"
            value={amount ? amount.toLocaleString("en-US") : ""}
            onChange={(e) => {
              const digits = e.target.value.replace(/\D/g, "").slice(0, 7);
              onChange({ ...value, salary_min: digits ? Number(digits) : null });
            }}
            className="h-full w-full bg-transparent text-base outline-none placeholder:text-muted-foreground md:text-sm"
          />
          <span className="text-sm text-muted-foreground">/year</span>
        </label>
        <label className="flex max-w-lg cursor-pointer items-start gap-3 rounded-2xl bg-muted/60 p-4">
          <Switch
            checked={value.include_unlisted_salary}
            onCheckedChange={(on) => onChange({ ...value, include_unlisted_salary: on })}
            className="mt-0.5"
          />
          <span className="space-y-1">
            <span className="block text-sm font-medium">Also show jobs that don&apos;t list a salary</span>
            <span className="block text-sm text-muted-foreground">
              Many postings leave pay out, especially outside states that require it.
            </span>
          </span>
        </label>
      </div>
    </Question>
  );
}

export function CompanyQuestions({ value, onChange }: Props) {
  return (
    <div className="space-y-7">
      <Question
        title="Company size and industry"
        hint="Saved for later: they'll filter the feed once yob collects company details."
      >
        <div className="space-y-4">
          <ChoiceGrid
            columns={3}
            options={COMPANY_SIZES}
            selected={value.company_sizes}
            onChange={(company_sizes) => onChange({ ...value, company_sizes })}
          />
          <TagInput
            label="Industries"
            value={value.industries}
            onChange={(industries) => onChange({ ...value, industries })}
            placeholder="e.g. Fintech, then Enter"
            suggestions={INDUSTRY_SUGGESTIONS}
          />
        </div>
      </Question>
      <Question title="Deal-breakers" hint="Jobs matching these are always hidden from the feed.">
        <div className="space-y-4">
          <div className="space-y-1.5">
            <p className="text-sm font-medium">Hide titles with these words</p>
            <TagInput
              label="Hide titles with these words"
              value={value.excluded_title_words}
              onChange={(excluded_title_words) => onChange({ ...value, excluded_title_words })}
              placeholder="e.g. manager, sales"
            />
          </div>
          <div className="space-y-1.5">
            <p className="text-sm font-medium">Never show these companies</p>
            <CompanyPicker
              top={[]}
              selected={value.excluded_companies}
              onToggle={(name) =>
                onChange({
                  ...value,
                  excluded_companies: value.excluded_companies.includes(name)
                    ? value.excluded_companies.filter((c) => c !== name)
                    : [...value.excluded_companies, name],
                })
              }
              className="max-w-md"
            />
          </div>
        </div>
      </Question>
    </div>
  );
}

/** The wizard's steps, in order. */
export const STEPS = [
  { id: "roles", title: "Roles", Body: RolesQuestions },
  { id: "type", title: "Job type", Body: JobTypeQuestions },
  { id: "location", title: "Location", Body: LocationQuestions },
  { id: "authorization", title: "Work authorization", Body: AuthorizationQuestions },
  { id: "salary", title: "Salary", Body: SalaryQuestions },
  { id: "company", title: "Companies", Body: CompanyQuestions },
] as const;
