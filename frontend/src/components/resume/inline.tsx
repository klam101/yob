"use client";

import { ArrowRight, CaseSensitive, Check, Undo2, Wand2 } from "lucide-react";
import { createContext, useContext, useId, useState } from "react";
import type { ResumeChange } from "@/api/client";
import { GrowingTextarea, IconButton, invalidClass, useFieldError, useFieldPath } from "@/components/profile/fields";
import { Button } from "@/components/ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { DateInput } from "@/components/ui/date-picker";
import { SuggestInput } from "@/components/ui/suggest";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { isAllCaps, tidyCaps } from "@/lib/caps";
import { cn } from "cn";
import { REASON, fieldLabel } from "./changes-panel";

/**
 * The parser's changes still waiting to be checked, by field path, and the
 * Keep and Undo actions, for the "Changed" tags on the fields.
 */
export type Changes = {
  byField: Map<string, { change: ResumeChange; index: number }[]>;
  keep: (index: number) => void;
  undo: (index: number) => void;
};

export const ChangesContext = createContext<Changes>({ byField: new Map(), keep: () => {}, undo: () => {} });

/**
 * A small amber "Changed" tag after a field the parser changed. Clicking it
 * shows what the file said, what it became and why, with Keep and Undo.
 * It goes away once every change on the field is kept or undone.
 */
export function ChangedTag({ path, className }: { path: string; className?: string }) {
  const { byField, keep, undo } = useContext(ChangesContext);
  const items = byField.get(path);
  const [open, setOpen] = useState(false);
  if (!items?.length) return null;
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <Tooltip>
        <TooltipTrigger asChild>
          <PopoverTrigger asChild>
            <button
              type="button"
              aria-label="Changed by the parser: show what changed"
              className={cn(
                "inline-flex h-6 shrink-0 items-center gap-1 self-center rounded-full bg-warning/18 px-2 text-xs font-medium whitespace-nowrap text-[color-mix(in_oklch,var(--warning),black_35%)] transition-colors outline-none hover:bg-warning/28 focus-visible:ring-3 focus-visible:ring-warning/40 dark:bg-warning/20 dark:text-warning",
                className,
              )}
            >
              <Wand2 className="size-3" aria-hidden /> Changed
            </button>
          </PopoverTrigger>
        </TooltipTrigger>
        <TooltipContent>Changed by the parser. Click to check it.</TooltipContent>
      </Tooltip>
      <PopoverContent align="start" className="w-80">
        <p className="text-xs font-semibold text-muted-foreground">{fieldLabel(path)}</p>
        {items.map(({ change: c, index }) => (
          <div key={index} className="space-y-2">
            <div className="flex flex-wrap items-center gap-1.5 text-xs break-all">
              {c.before ? (
                <span className="rounded-md bg-muted px-1.5 py-0.5 text-muted-foreground line-through decoration-muted-foreground/50">
                  {c.before}
                </span>
              ) : (
                <span className="text-muted-foreground">(not in the text)</span>
              )}
              <ArrowRight className="size-3 shrink-0 text-muted-foreground" aria-label="became" />
              <span className="rounded-md bg-muted px-1.5 py-0.5 font-medium">{c.after}</span>
            </div>
            <p className="text-xs text-muted-foreground">{REASON[c.kind]}</p>
            <div className="flex gap-1.5">
              <Button
                size="xs"
                variant="flat"
                onClick={() => {
                  keep(index);
                  if (items.length === 1) setOpen(false);
                }}
              >
                <Check /> Keep
              </Button>
              {(c.undo ?? []).length > 0 && (
                <Button
                  size="xs"
                  variant="ghost"
                  onClick={() => {
                    undo(index);
                    if (items.length === 1) setOpen(false);
                  }}
                >
                  <Undo2 /> Undo
                </Button>
              )}
            </div>
          </div>
        ))}
      </PopoverContent>
    </Popover>
  );
}

const inlineClass =
  "w-full min-w-0 rounded-lg border border-transparent bg-transparent px-2 py-1 outline-none transition-[background-color,border-color,box-shadow] placeholder:text-muted-foreground/70 hover:bg-muted/70 focus-visible:border-ring focus-visible:bg-card focus-visible:ring-3 focus-visible:ring-ring/25";

/** Empty fields look like soft grey pills, so it's clear they can be filled. */
export const emptyClass = "bg-muted/60 dark:bg-muted/60";

/**
 * Text that looks like part of the resume until it's hovered or focused,
 * then edits in place. Shows its API error from the surrounding
 * `FieldScope`, and a "fix capitals" button when the text is all caps.
 */
export function InlineText({
  name,
  label,
  value,
  onChange,
  placeholder,
  className,
  multiline = false,
  type = "text",
  icon: Icon,
  fit = false,
  tag = true,
  suggestions,
  autoFocus,
}: {
  /** The field's name in the content, for errors and change marks. */
  name: string;
  /** Accessible name; the placeholder shows it visually. */
  label: string;
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  className?: string;
  multiline?: boolean;
  type?: string;
  /** An icon in front, like the envelope before an email. */
  icon?: React.ComponentType<{ className?: string }>;
  /** As wide as its text, to sit inside a line of other words. */
  fit?: boolean;
  /** Show the "Changed" tag here; off when the caller places it. */
  tag?: boolean;
  /** Suggestions in a menu below, narrowed as you type (any text is allowed). */
  suggestions?: readonly string[];
  autoFocus?: boolean;
}) {
  const id = useId();
  const path = useFieldPath(name);
  const error = useFieldError(name);
  const caps = isAllCaps(value);
  const common = {
    id,
    "aria-label": label,
    "aria-invalid": error ? true : undefined,
    "aria-describedby": error ? `${id}-error` : undefined,
    placeholder: placeholder ?? label,
  };
  return (
    <div className={cn("min-w-0", fit && "inline-block max-w-full")} data-field={path}>
      <div className="flex flex-wrap items-start gap-1 sm:flex-nowrap">
        {Icon && <Icon className="mt-2 size-4 shrink-0 text-muted-foreground" />}
        {multiline ? (
          <GrowingTextarea
            {...common}
            value={value}
            onChange={(e) => onChange(e.target.value)}
            className={cn(
              inlineClass,
              "min-h-8 bg-transparent shadow-none dark:bg-transparent",
              !value && emptyClass,
              error && invalidClass,
              className,
            )}
          />
        ) : suggestions ? (
          <SuggestInput
            {...common}
            autoFocus={autoFocus}
            value={value}
            options={suggestions}
            onChange={onChange}
            className={cn(
              inlineClass,
              fit && "field-sizing-content w-auto min-w-20 max-w-full",
              !value && emptyClass,
              error && invalidClass,
              className,
            )}
          />
        ) : (
          <input
            {...common}
            autoFocus={autoFocus}
            type={type}
            value={value}
            autoComplete="off"
            onChange={(e) => onChange(e.target.value)}
            className={cn(
              inlineClass,
              fit && "field-sizing-content w-auto min-w-20 max-w-full",
              !value && emptyClass,
              error && invalidClass,
              className,
            )}
          />
        )}
        {caps && (
          <IconButton label={`Fix capitals: "${tidyCaps(value)}"`} onClick={() => onChange(tidyCaps(value))} className="mt-0.5">
            <CaseSensitive />
          </IconButton>
        )}
        {tag && <ChangedTag path={path} className="mt-1" />}
      </div>
      {error && (
        <p id={`${id}-error`} role="alert" className="px-2 pt-0.5 text-xs font-medium text-destructive">
          {error}
        </p>
      )}
    </div>
  );
}

/**
 * A date edited in place: shows "Nov 2026", stores the whole day
 * (`YYYY-MM-DD`), with a calendar below. Empty means no date.
 */
export function InlineMonth({
  name,
  label,
  value,
  onChange,
  disabled,
}: {
  name: string;
  label: string;
  value: string | null;
  onChange: (v: string | null) => void;
  disabled?: boolean;
}) {
  const id = useId();
  const path = useFieldPath(name);
  const error = useFieldError(name);
  return (
    <div className="min-w-0" data-field={path}>
      <div className="flex items-center gap-1">
        <DateInput
          id={id}
          aria-label={label}
          placeholder={label}
          value={value}
          onChange={onChange}
          disabled={disabled}
          aria-invalid={error ? true : undefined}
          aria-describedby={error ? `${id}-error` : undefined}
          wrapperClassName="w-40"
          className={cn(
            inlineClass,
            "text-sm text-muted-foreground disabled:opacity-60",
            !value && emptyClass,
            error && invalidClass,
          )}
        />
        <ChangedTag path={path} />
      </div>
      {error && (
        <p id={`${id}-error`} role="alert" className="px-2 pt-0.5 text-xs font-medium text-destructive">
          {error}
        </p>
      )}
    </div>
  );
}
