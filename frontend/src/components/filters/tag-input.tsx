"use client";

import { Plus, X } from "lucide-react";
import { useId, useState } from "react";
import { cn } from "cn";

/**
 * A list of short entries typed one at a time: Enter (or a separator key)
 * adds the text as a chip, Backspace in an empty box removes the last one.
 * Suggestions not yet chosen show below as one-click chips.
 */
export function TagInput({
  value,
  onChange,
  placeholder,
  separators = [","],
  suggestions = [],
  label,
  className,
  max = 30,
}: {
  value: string[];
  onChange: (next: string[]) => void;
  placeholder?: string;
  /** Keys that end an entry, besides Enter. A separator can't be inside an entry. */
  separators?: string[];
  suggestions?: readonly string[];
  /** Accessible name for the text box. */
  label: string;
  className?: string;
  max?: number;
}) {
  const id = useId();
  const [text, setText] = useState("");
  const has = (v: string) => value.some((x) => x.toLowerCase() === v.toLowerCase());

  function add(raw: string) {
    let next = [...value];
    for (const piece of separators.reduce((parts, sep) => parts.flatMap((p) => p.split(sep)), [raw])) {
      const v = piece.split(/\s+/).join(" ").trim();
      if (v && !next.some((x) => x.toLowerCase() === v.toLowerCase()) && next.length < max) next = [...next, v];
    }
    if (next.length !== value.length) onChange(next);
    setText("");
  }

  const remove = (v: string) => onChange(value.filter((x) => x !== v));
  const open = suggestions.filter((s) => !has(s));

  return (
    <div className={cn("space-y-2.5", className)}>
      <div className="flex min-h-11 flex-wrap items-center gap-1.5 rounded-xl border border-transparent bg-muted px-2 py-1.5 transition-[background-color,border-color,box-shadow] focus-within:border-ring focus-within:bg-card focus-within:ring-3 focus-within:ring-ring/25">
        {value.map((v) => (
          <span
            key={v}
            className="flex items-center gap-1 rounded-lg bg-card py-1 pr-1 pl-2.5 text-sm shadow-soft dark:bg-background"
          >
            {v}
            <button
              type="button"
              onClick={() => remove(v)}
              aria-label={`Remove ${v}`}
              className="rounded-md p-0.5 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
            >
              <X className="size-3.5" />
            </button>
          </span>
        ))}
        <input
          id={id}
          aria-label={label}
          value={text}
          placeholder={value.length ? "" : placeholder}
          onChange={(e) => {
            const v = e.target.value;
            if (separators.some((s) => v.includes(s))) add(v);
            else setText(v);
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              if (text.trim()) add(text);
            } else if (e.key === "Backspace" && !text && value.length) {
              onChange(value.slice(0, -1));
            }
          }}
          onBlur={() => text.trim() && add(text)}
          className="h-8 min-w-32 flex-1 bg-transparent px-1.5 text-base outline-none placeholder:text-muted-foreground md:text-sm"
        />
      </div>
      {open.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {open.map((s) => (
            <button
              key={s}
              type="button"
              onClick={() => add(s)}
              className="flex items-center gap-1 rounded-full border border-border bg-card px-2.5 py-1 text-[0.8125rem] text-foreground/80 transition-colors hover:border-primary/40 hover:bg-primary/5 hover:text-foreground"
            >
              <Plus className="size-3.5 text-muted-foreground" />
              {s}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
