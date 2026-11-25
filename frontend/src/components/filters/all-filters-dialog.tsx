"use client";

import { Search, SlidersHorizontal } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { type Facets, api } from "@/api/client";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import {
  EMPLOYMENT,
  type FilterKey,
  POSTED_WITHIN,
  type Query,
  SENIORITY,
  SOURCE,
  WORKPLACE,
  activeCount,
  clearFilters,
  listOf,
  salaryOptions,
  toApiQuery,
  toggleInList,
  withValue,
} from "@/lib/filters";
import { cn } from "cn";
import { CompanyPicker } from "./company-picker";
import { type Choice, CheckList, RadioList, facetChoices } from "./controls";
import { useFeed } from "./feed-context";
import { TagInput } from "./tag-input";

type Section = {
  id: string;
  group: string;
  label: string;
  keys: FilterKey[];
  render: (draft: Query, set: (q: Query) => void, facets: Facets) => React.ReactNode;
};

const checks = (key: FilterKey, options: (facets: Facets) => Choice[]): Section["render"] =>
  function CheckSection(draft, set, facets) {
    return (
      <CheckList
        options={options(facets)}
        selected={listOf(draft, key)}
        onToggle={(v) => set(toggleInList(draft, key, v))}
        className="sm:grid sm:grid-cols-2 sm:gap-x-4"
      />
    );
  };

const radios = (key: FilterKey, options: Choice[], anyLabel: string): Section["render"] =>
  function RadioSection(draft, set) {
    return (
      <RadioList
        options={options}
        value={draft[key]}
        anyLabel={anyLabel}
        onChange={(v) => set(withValue(draft, key, v))}
        className="sm:grid sm:grid-cols-2 sm:gap-x-4"
      />
    );
  };

const tags = (key: FilterKey, label: string, placeholder: string, hint: string): Section["render"] =>
  function TagSection(draft, set) {
    return (
      <div className="max-w-lg space-y-2">
        <TagInput
          label={label}
          value={listOf(draft, key)}
          onChange={(v) => set(withValue(draft, key, v))}
          separators={key === "location" ? [";"] : [","]}
          placeholder={placeholder}
        />
        <p className="text-sm text-muted-foreground">{hint}</p>
      </div>
    );
  };

const toggle = (key: FilterKey, title: string, hint: string): Section["render"] =>
  function ToggleSection(draft, set) {
    return <ToggleRow draft={draft} set={set} filterKey={key} title={title} hint={hint} />;
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
    <label className="flex max-w-md cursor-pointer items-start gap-3 rounded-xl bg-muted/60 p-4">
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

const text = (key: FilterKey, placeholder: string): Section["render"] =>
  function TextSection(draft, set) {
    return (
      <Input
        value={draft[key] ?? ""}
        onChange={(e) => set(withValue(draft, key, e.target.value))}
        placeholder={placeholder}
        className="max-w-md"
      />
    );
  };

// Grouped like hiringcafe's panel. Filters that need AI extraction
// (education, skills, clearance, sponsorship) are added once jobs are enriched.
const SECTIONS: Section[] = [
  { id: "posted", group: "Results", label: "Date posted", keys: ["posted_within"], render: radios("posted_within", POSTED_WITHIN, "Any time") },
  {
    id: "stale",
    group: "Results",
    label: "Stale and reposts",
    keys: ["hide_stale"],
    render: (draft, set) => (
      <label className="flex max-w-md cursor-pointer items-start gap-3 rounded-xl bg-muted/60 p-4">
        <Switch
          checked={draft.hide_stale === "true"}
          onCheckedChange={(on) => set(withValue(draft, "hide_stale", on ? "true" : undefined))}
          className="mt-0.5"
        />
        <span className="space-y-1">
          <span className="block text-sm font-medium">Hide possibly stale jobs</span>
          <span className="block text-sm text-muted-foreground">
            Jobs open more than 60 days, or reposted more than twice, are often no longer hiring.
          </span>
        </span>
      </label>
    ),
  },
  {
    id: "titles",
    group: "Role",
    label: "Job titles",
    keys: ["title"],
    render: tags(
      "title",
      "Job titles",
      "e.g. Software Engineer, then Enter",
      "Any of these. A job matches when its title has every word of one of them, in any order.",
    ),
  },
  { id: "keywords", group: "Role", label: "Keywords", keys: ["q"], render: text("q", "Title, company or skill, e.g. Rust") },
  { id: "department", group: "Role", label: "Department", keys: ["department"], render: checks("department", (f) => facetChoices(f.departments)) },
  { id: "seniority", group: "Role", label: "Experience level", keys: ["seniority"], render: checks("seniority", () => SENIORITY) },
  { id: "type", group: "Role", label: "Job type", keys: ["employment_type"], render: checks("employment_type", () => EMPLOYMENT) },
  {
    id: "salary",
    group: "Pay",
    label: "Salary",
    keys: ["salary_min"],
    render: (draft, set) => (
      <div className="space-y-4">
        <RadioList
          options={salaryOptions(draft.salary_min)}
          value={draft.salary_min}
          anyLabel="Any salary"
          onChange={(v) => set(withValue(draft, "salary_min", v))}
          className="sm:grid sm:grid-cols-2 sm:gap-x-4"
        />
        {draft.salary_min && (
          <ToggleRow
            draft={draft}
            set={set}
            filterKey="include_unlisted_salary"
            title="Also show jobs that don't list a salary"
            hint="Many postings leave pay out."
          />
        )}
      </div>
    ),
  },
  {
    id: "location",
    group: "Location",
    label: "Location",
    keys: ["location"],
    render: tags(
      "location",
      "Locations",
      "e.g. Austin, TX, then Enter",
      "Any of these: a city with its state, or a whole state. When Remote is picked under Workplace, remote jobs show too.",
    ),
  },
  { id: "workplace", group: "Location", label: "Workplace", keys: ["workplace"], render: checks("workplace", () => WORKPLACE) },
  {
    id: "company",
    group: "Company",
    label: "Company",
    keys: ["company"],
    render: (draft, set, facets) => (
      <CompanyPicker
        top={facetChoices(facets.companies)}
        selected={listOf(draft, "company")}
        onToggle={(v) => set(toggleInList(draft, "company", v))}
        className="max-w-md"
      />
    ),
  },
  { id: "source", group: "Company", label: "Source", keys: ["source"], render: checks("source", () => SOURCE) },
  {
    id: "exclude-title",
    group: "Deal-breakers",
    label: "Excluded titles",
    keys: ["exclude_title"],
    render: tags(
      "exclude_title",
      "Hide titles with these words",
      "e.g. manager, then Enter",
      "Hides jobs whose title has any of these words or phrases.",
    ),
  },
  {
    id: "exclude-company",
    group: "Deal-breakers",
    label: "Excluded companies",
    keys: ["exclude_company"],
    render: (draft, set) => (
      <CompanyPicker
        top={[]}
        selected={listOf(draft, "exclude_company")}
        onToggle={(v) => set(toggleInList(draft, "exclude_company", v))}
        className="max-w-md"
      />
    ),
  },
  {
    id: "sponsorship",
    group: "Work authorization",
    label: "Visa sponsorship",
    keys: ["hide_no_sponsorship"],
    render: toggle(
      "hide_no_sponsorship",
      "Hide jobs that won't sponsor a visa",
      "Read from the posting: \"unable to sponsor\", \"no visa sponsorship\", \"without sponsorship\" and similar.",
    ),
  },
  {
    id: "citizenship",
    group: "Work authorization",
    label: "Citizenship",
    keys: ["hide_citizens_only"],
    render: toggle(
      "hide_citizens_only",
      "Hide jobs for U.S. citizens only",
      "Read from the posting: \"must be a U.S. citizen\", or a security clearance required.",
    ),
  },
];

export function AllFiltersDialog({ facets }: { facets: Facets }) {
  const { query, apply } = useFeed();
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState<Query>(query);
  const [sectionId, setSectionId] = useState(SECTIONS[0].id);
  const [search, setSearch] = useState("");
  const count = useMatchCount(draft, open);

  const visible = useMemo(() => {
    const s = search.trim().toLowerCase();
    return s ? SECTIONS.filter((x) => `${x.group} ${x.label}`.toLowerCase().includes(s)) : SECTIONS;
  }, [search]);
  const section = visible.find((x) => x.id === sectionId) ?? visible[0];
  const total = activeCount(query);

  function onOpenChange(next: boolean) {
    if (next) setDraft(query); // start from what's applied
    setOpen(next);
  }

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
      <DialogContent className="flex max-h-[min(44rem,calc(100dvh-2rem))] flex-col gap-0 overflow-hidden p-0 sm:max-w-3xl">
        <div className="flex items-center gap-3 border-b border-border/70 p-4 pr-14">
          <DialogTitle className="sr-only">All filters</DialogTitle>
          <DialogDescription className="sr-only">
            Choose filters, then show the matching jobs.
          </DialogDescription>
          <label className="flex h-10 flex-1 items-center gap-2.5 rounded-xl bg-muted px-3">
            <Search className="size-4 text-muted-foreground" />
            <input
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search filters"
              className="w-full bg-transparent text-sm outline-none placeholder:text-muted-foreground"
            />
          </label>
        </div>

        <div className="flex min-h-0 flex-1 flex-col sm:flex-row">
          <nav aria-label="Filter categories" className="flex shrink-0 gap-1 overflow-x-auto border-b border-border/70 p-2 sm:w-56 sm:flex-col sm:overflow-y-auto sm:border-r sm:border-b-0 sm:p-3">
            {visible.map((s, i) => {
              const newGroup = i === 0 || visible[i - 1].group !== s.group;
              const n = activeCount(draft, s.keys);
              return (
                <div key={s.id} className="contents">
                  {newGroup && (
                    <p className={cn("hidden px-3 pb-1 text-[0.6875rem] font-semibold tracking-wider text-muted-foreground uppercase sm:block", i === 0 ? "pt-1" : "pt-5")}>
                      {s.group}
                    </p>
                  )}
                  <button
                    type="button"
                    onClick={() => setSectionId(s.id)}
                    className={cn(
                      "flex shrink-0 items-center justify-between gap-2 rounded-xl px-3 py-2 text-left text-sm transition-colors",
                      section?.id === s.id
                        ? "bg-primary/12 font-medium text-primary dark:bg-primary/20"
                        : "text-foreground/80 hover:bg-muted",
                    )}
                  >
                    {s.label}
                    {n > 0 && (
                      <span className="rounded-full bg-primary px-1.5 text-[0.6875rem] text-primary-foreground tabular-nums">
                        {n}
                      </span>
                    )}
                  </button>
                </div>
              );
            })}
            {visible.length === 0 && <p className="px-3 py-2 text-sm text-muted-foreground">No filters match.</p>}
          </nav>

          <section className="min-h-72 flex-1 overflow-y-auto p-5 sm:p-6">
            {section && (
              <>
                <h3 className="mb-4 text-base font-semibold">{section.label}</h3>
                {section.render(draft, setDraft, facets)}
              </>
            )}
          </section>
        </div>

        <DialogFooter className="m-0 flex-row items-center justify-between gap-3 border-border/70 bg-card px-5 py-4 sm:justify-between">
          <Button variant="light" onClick={() => setDraft(clearFilters(draft))} disabled={activeCount(draft) === 0}>
            Clear all
          </Button>
          <Button
            onClick={() => {
              apply(draft);
              setOpen(false);
            }}
            className="min-w-40"
          >
            {count === null ? "Show results" : `Show ${count} ${count === 1 ? "job" : "jobs"}`}
          </Button>
        </DialogFooter>
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
