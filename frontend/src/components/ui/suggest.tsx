"use client";

import { Popover as PopoverPrimitive } from "radix-ui";
import { useEffect, useId, useRef, useState } from "react";
import { api, type SuggestKind } from "@/api/client";
import { cn } from "cn";

/** Lowercase, without apostrophes, so "bachelors" finds "Bachelor's". */
const fold = (s: string) => s.toLowerCase().replace(/['’]/g, "");

/**
 * The options that match what's typed, best first, at most `limit`:
 * options that start with the text, then ones where every typed word starts
 * a word ("bach sci" -> "Bachelor of Science (BS)"), then ones that contain
 * it. Ties keep the list's order, so put common choices first. Empty text
 * gives the first `limit` options.
 */
export function rankSuggestions(
  options: readonly string[],
  query: string,
  limit = 8,
  /** Extra words an option is found by ("uk" for United Kingdom). */
  aliases?: Record<string, string>,
): string[] {
  const q = fold(query.trim());
  if (!q) return options.slice(0, limit);
  const tokens = q.split(/\s+/);
  const scored: [number, number, string][] = [];
  options.forEach((o, i) => {
    const f = fold(o);
    const extra = aliases?.[o] ? fold(aliases[o]) : "";
    const words = `${f} ${extra}`.split(/[^a-z0-9]+/).filter(Boolean);
    let score: number | null = null;
    if (f.startsWith(q) || extra.split(" ").includes(q)) score = 0;
    else if (tokens.every((t) => words.some((w) => w.startsWith(t)))) score = 1;
    else if (f.includes(q)) score = 2;
    if (score !== null) scored.push([score, i, o]);
  });
  scored.sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  return scored.slice(0, limit).map(([, , o]) => o);
}

/** The option with the typed words in bold, where they start a word. */
function Highlight({ text, query }: { text: string; query: string }) {
  const lower = text.toLowerCase();
  const bold = new Array<boolean>(text.length).fill(false);
  for (const t of query.toLowerCase().split(/\s+/).filter(Boolean)) {
    let at = -1;
    for (let i = lower.indexOf(t); i !== -1; i = lower.indexOf(t, i + 1)) {
      if (i === 0 || !/[a-z0-9]/.test(lower[i - 1])) {
        at = i;
        break;
      }
    }
    if (at === -1) at = lower.indexOf(t);
    if (at !== -1) bold.fill(true, at, at + t.length);
  }
  const parts: { text: string; bold: boolean }[] = [];
  for (let i = 0; i < text.length; i++) {
    const last = parts[parts.length - 1];
    if (last && last.bold === bold[i]) last.text += text[i];
    else parts.push({ text: text[i], bold: bold[i] });
  }
  return (
    <>
      {parts.map((p, i) =>
        p.bold ? (
          <span key={i} className="font-semibold text-foreground">
            {p.text}
          </span>
        ) : (
          <span key={i}>{p.text}</span>
        ),
      )}
    </>
  );
}

type InputProps = Omit<React.ComponentProps<"input">, "value" | "onChange" | "list" | "ref">;

type Item = { value: string; detail?: string };

/**
 * Suggestions from the API (`/suggest/{kind}`) for what's typed, fetched a
 * moment after typing stops; older requests are cancelled.
 */
function useRemote(kind: SuggestKind | undefined, query: string, enabled: boolean, limit: number) {
  const [state, setState] = useState<{ key: string; items: Item[] } | null>(null);
  const q = query.trim();
  const key = `${kind}:${q}`;
  useEffect(() => {
    if (!kind || !enabled) return;
    const ctl = new AbortController();
    const timer = setTimeout(async () => {
      const { data } = await api
        .GET("/suggest/{kind}", { params: { path: { kind }, query: { q, limit } }, signal: ctl.signal })
        .catch(() => ({ data: undefined }));
      if (!ctl.signal.aborted && data) {
        setState({ key, items: data.map((d) => ({ value: d.value, detail: d.detail ?? undefined })) });
      }
    }, 150);
    return () => {
      clearTimeout(timer);
      ctl.abort();
    };
  }, [kind, key, q, enabled, limit]);
  // While the answer for this text is on its way, keep showing the last one.
  return { items: state?.items ?? [], loading: !!kind && enabled && state?.key !== key };
}

/**
 * A text box with a suggestion menu that opens below it, in the app's own
 * style (light or dark with the theme). It shows a few options and narrows
 * them as you type, like a search box; any text is still allowed. Use this
 * for every suggestion list instead of a native `<datalist>`.
 *
 * Options come from a fixed list (`options`) or from the API (`source`:
 * cities, schools, companies, job titles), for lists too big to send.
 */
export function SuggestInput({
  value,
  onChange,
  options,
  source,
  onPick,
  aliases,
  limit = 8,
  onKeyDown,
  onFocus,
  onBlur,
  ...props
}: InputProps & {
  value: string;
  onChange: (v: string) => void;
  options?: readonly string[];
  source?: SuggestKind;
  /** Extra words each option is found by ("uk" for United Kingdom). */
  aliases?: Record<string, string>;
  /**
   * Handles a picked option instead of `onChange` (to fill other fields from
   * it, or to add it to a list as is).
   */
  onPick?: (item: Item) => void;
  /** How many options show at once. */
  limit?: number;
}) {
  const id = useId();
  const input = useRef<HTMLInputElement>(null);
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(-1);
  const remote = useRemote(source, value, open, limit);
  const items: Item[] = source
    ? remote.items
    : rankSuggestions(options ?? [], value, limit, aliases).map((v) => ({ value: v }));
  // Nothing to suggest, or the text already is the only match.
  const useful = items.length > 0 && !(items.length === 1 && items[0].value === value.trim());
  const shown = open && useful;

  function pick(item: Item) {
    if (onPick) onPick(item);
    else onChange(item.value);
    setOpen(false);
    setActive(-1);
  }

  return (
    <PopoverPrimitive.Root open={shown} onOpenChange={(o) => !o && setOpen(false)}>
      <PopoverPrimitive.Anchor asChild>
        <input
          {...props}
          ref={input}
          role="combobox"
          aria-autocomplete="list"
          aria-expanded={shown}
          aria-controls={shown ? `${id}-list` : undefined}
          aria-activedescendant={shown && active >= 0 ? `${id}-opt-${active}` : undefined}
          aria-busy={source ? remote.loading : undefined}
          autoComplete="off"
          value={value}
          onChange={(e) => {
            onChange(e.target.value);
            setOpen(true);
            setActive(-1);
          }}
          onFocus={(e) => {
            setOpen(true);
            onFocus?.(e);
          }}
          onBlur={(e) => {
            setOpen(false);
            setActive(-1);
            onBlur?.(e);
          }}
          onKeyDown={(e) => {
            if (e.key === "ArrowDown" || e.key === "ArrowUp") {
              e.preventDefault();
              if (!shown) {
                setOpen(true);
                setActive(e.key === "ArrowDown" ? 0 : items.length - 1);
              } else {
                const n = items.length;
                const down = e.key === "ArrowDown";
                setActive((a) => (down ? (a >= n - 1 ? 0 : a + 1) : a <= 0 ? n - 1 : a - 1));
              }
              return;
            }
            if (e.key === "Enter" && shown && active >= 0 && active < items.length) {
              e.preventDefault();
              pick(items[active]);
              return;
            }
            if (e.key === "Escape" && shown) {
              e.preventDefault();
              setOpen(false);
              setActive(-1);
              return;
            }
            onKeyDown?.(e);
          }}
        />
      </PopoverPrimitive.Anchor>
      <PopoverPrimitive.Portal>
        <PopoverPrimitive.Content
          side="bottom"
          align="start"
          sideOffset={6}
          collisionPadding={8}
          // Typing stays in the box.
          onOpenAutoFocus={(e) => e.preventDefault()}
          onCloseAutoFocus={(e) => e.preventDefault()}
          onInteractOutside={(e) => {
            if (e.target === input.current) e.preventDefault();
          }}
          className="z-50 max-h-(--radix-popover-content-available-height) w-max max-w-[min(26rem,calc(100vw-1rem))] min-w-(--radix-popover-trigger-width) origin-(--radix-popover-content-transform-origin) overflow-y-auto rounded-2xl border border-border/60 bg-popover p-1.5 text-popover-foreground shadow-raised outline-hidden duration-100 data-[side=bottom]:slide-in-from-top-1 data-[side=top]:slide-in-from-bottom-1 data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95"
        >
          <div
            id={`${id}-list`}
            role="listbox"
            aria-label={props["aria-label"] ? `${props["aria-label"]} suggestions` : "Suggestions"}
            className={cn("transition-opacity", source && remote.loading && "opacity-70")}
          >
            {items.map((o, i) => (
              <div
                key={`${o.value}|${o.detail ?? ""}`}
                id={`${id}-opt-${i}`}
                role="option"
                aria-selected={i === active}
                // Keep focus in the box; pick on click.
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => pick(o)}
                onMouseMove={() => active !== i && setActive(i)}
                className={cn(
                  "cursor-pointer rounded-xl px-3 py-2 text-sm text-foreground/85 select-none",
                  i === active && "bg-muted text-foreground",
                )}
              >
                <Highlight text={o.value} query={value} />
                {o.detail && <div className="text-xs text-muted-foreground">{o.detail}</div>}
              </div>
            ))}
          </div>
        </PopoverPrimitive.Content>
      </PopoverPrimitive.Portal>
    </PopoverPrimitive.Root>
  );
}
