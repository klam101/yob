"use client";

import { MapPin, Search } from "lucide-react";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { SuggestInput } from "@/components/ui/suggest";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { WORKPLACE, listOf, withValue } from "@/lib/filters";
import { useFeed } from "./feed-context";

/**
 * Keyword and location apply on submit, or at once when a suggestion is
 * picked; workplace toggles apply at once. The keyword box suggests job
 * titles (and related ones: "coder" -> Software Developer) and companies as
 * you type; the location box suggests cities.
 * Remount with a new `key` when the URL's q/location change from elsewhere.
 */
export function SearchBar() {
  const { query, apply } = useFeed();
  const [q, setQ] = useState(query.q ?? "");
  const [location, setLocation] = useState(query.location ?? "");

  const search = (keyword: string, place: string) =>
    apply(withValue(withValue(query, "q", keyword.trim()), "location", place.trim()));

  function submit(e: React.FormEvent) {
    e.preventDefault();
    search(q, location);
  }

  // A title searches as a keyword; a company becomes a Company filter.
  function pickKeyword(item: { value: string; kind?: string }) {
    if (item.kind === "company") {
      const companies = listOf(query, "company");
      const has = companies.some((c) => c.toLowerCase() === item.value.toLowerCase());
      setQ("");
      apply(
        withValue(
          withValue(withValue(query, "company", has ? companies : [...companies, item.value]), "q", ""),
          "location",
          location.trim(),
        ),
      );
    } else {
      setQ(item.value);
      search(item.value, location);
    }
  }

  return (
    <form
      onSubmit={submit}
      role="search"
      className="flex flex-col gap-2 rounded-3xl border border-border/70 bg-card p-2 shadow-soft lg:flex-row lg:items-center"
    >
      <label className="flex h-11 flex-1 items-center gap-2.5 rounded-2xl px-3 transition-colors focus-within:bg-muted">
        <Search className="size-4.5 shrink-0 text-muted-foreground" />
        <SuggestInput
          source="search"
          showOnEmpty={false}
          value={q}
          onChange={setQ}
          onPick={pickKeyword}
          aria-label="Job title, company or skill"
          placeholder="Job title, company or skill"
          className="h-full w-full min-w-0 bg-transparent text-[0.9375rem] outline-none placeholder:text-muted-foreground"
        />
      </label>

      <span aria-hidden className="hidden h-6 w-px bg-border lg:block" />

      <label className="flex h-11 items-center gap-2.5 rounded-2xl px-3 transition-colors focus-within:bg-muted lg:w-60">
        <MapPin className="size-4.5 shrink-0 text-muted-foreground" />
        <SuggestInput
          source="location"
          showOnEmpty={false}
          value={location}
          onChange={setLocation}
          onPick={({ value }) => {
            setLocation(value);
            search(q, value);
          }}
          aria-label="Location"
          placeholder="City, state or Remote"
          className="h-full w-full min-w-0 bg-transparent text-[0.9375rem] outline-none placeholder:text-muted-foreground"
        />
      </label>

      <div className="flex items-center gap-2">
        <ToggleGroup
          type="multiple"
          aria-label="Workplace"
          value={listOf(query, "workplace")}
          onValueChange={(values) => apply(withValue(query, "workplace", values))}
          className="flex-1 rounded-2xl bg-muted p-1 lg:flex-none"
        >
          {WORKPLACE.map((w) => (
            <ToggleGroupItem
              key={w.value}
              value={w.value}
              className="h-9 flex-1 rounded-xl px-3 text-[0.8125rem] font-medium text-muted-foreground data-[state=on]:bg-card data-[state=on]:text-foreground data-[state=on]:shadow-soft lg:flex-none"
            >
              {w.label}
            </ToggleGroupItem>
          ))}
        </ToggleGroup>
        <Button type="submit" size="icon" shape="pill" className="size-11" aria-label="Search">
          <Search />
        </Button>
      </div>
    </form>
  );
}
