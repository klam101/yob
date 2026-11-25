"use client";

import { CaseSensitive } from "lucide-react";
import { createContext, useContext, useId, useState } from "react";
import { GrowingTextarea, IconButton, invalidClass, useFieldError, useFieldPath } from "@/components/profile/fields";
import { isAllCaps, tidyCaps } from "@/lib/caps";
import { cn } from "cn";

/** Field paths with a parser change still waiting to be checked. */
export const OpenChanges = createContext<Set<string>>(new Set());

const inlineClass =
  "w-full min-w-0 rounded-lg border border-transparent bg-transparent px-2 py-1 outline-none transition-[background-color,border-color,box-shadow] placeholder:text-muted-foreground/70 hover:bg-muted/70 focus-visible:border-ring focus-visible:bg-card focus-visible:ring-3 focus-visible:ring-ring/25";

/** Empty fields look like soft grey pills, so it's clear they can be filled. */
export const emptyClass = "bg-muted/60 dark:bg-muted/60";

/** A dot marking a field the parser changed, until it's checked. */
function ChangedDot({ path }: { path: string }) {
  const open = useContext(OpenChanges);
  if (!open.has(path)) return null;
  return (
    <span
      className="absolute top-1/2 -left-3 size-1.5 -translate-y-1/2 rounded-full bg-warning"
      title="The parser changed this; see the list on the right"
      aria-hidden
    />
  );
}

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
      <div className="relative flex items-start gap-1">
        <ChangedDot path={path} />
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
        ) : (
          <input
            {...common}
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
      </div>
      {error && (
        <p id={`${id}-error`} role="alert" className="px-2 pt-0.5 text-xs font-medium text-destructive">
          {error}
        </p>
      )}
    </div>
  );
}

/** A month (`YYYY-MM`) edited in place; empty means no date. */
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
  // An empty month picker shows "--------- ----"; show the label instead
  // until it's focused.
  const [focused, setFocused] = useState(false);
  const picker = !!value || focused;
  return (
    <div className="min-w-0" data-field={path}>
      <div className="relative">
        <ChangedDot path={path} />
        <input
          id={id}
          type={picker ? "month" : "text"}
          placeholder={label}
          onFocus={() => setFocused(true)}
          onBlur={() => setFocused(false)}
          aria-label={label}
          value={value ?? ""}
          disabled={disabled}
          aria-invalid={error ? true : undefined}
          aria-describedby={error ? `${id}-error` : undefined}
          onChange={(e) => onChange(e.target.value || null)}
          className={cn(
            inlineClass,
            "w-40 text-sm text-muted-foreground disabled:opacity-60",
            !value && emptyClass,
            error && invalidClass,
          )}
        />
      </div>
      {error && (
        <p id={`${id}-error`} role="alert" className="px-2 pt-0.5 text-xs font-medium text-destructive">
          {error}
        </p>
      )}
    </div>
  );
}
