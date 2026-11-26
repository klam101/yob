"use client";

import { useTheme } from "next-themes";
import { useCallback } from "react";
import { flushSync } from "react-dom";

export type ThemeChoice = "light" | "dark" | "system";

/**
 * Switches the theme with a circle reveal: the new theme spreads out from
 * `origin` (the control that was used), using the browser's View
 * Transitions API. With reduced motion it's a short crossfade instead; where
 * the API is missing, instant.
 */
export function useThemeSwitch() {
  const { setTheme } = useTheme();
  return useCallback(
    (next: ThemeChoice, origin?: { x: number; y: number }) => {
      if (!document.startViewTransition) {
        setTheme(next);
        return;
      }
      const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
      const x = origin?.x ?? window.innerWidth / 2;
      const y = origin?.y ?? window.innerHeight / 2;
      const r = Math.hypot(Math.max(x, window.innerWidth - x), Math.max(y, window.innerHeight - y));
      const root = document.documentElement;
      root.style.setProperty("--theme-x", `${x}px`);
      root.style.setProperty("--theme-y", `${y}px`);
      root.style.setProperty("--theme-r", `${r}px`);
      // The CSS picks the fade over the circle by this class.
      root.classList.toggle("theme-fade", reduce);
      const transition = document.startViewTransition(() => {
        // Change <html> right away so the "after" snapshot has the new
        // theme; next-themes then stores the choice and agrees.
        const resolved =
          next === "system" ? (window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light") : next;
        root.classList.remove("light", "dark");
        root.classList.add(resolved);
        root.dataset.theme = resolved;
        root.style.colorScheme = resolved;
        flushSync(() => setTheme(next));
      });
      transition.finished.finally(() => root.classList.remove("theme-fade"));
    },
    [setTheme],
  );
}

/** The center of the element an event came from. */
export function centerOf(el: Element | null) {
  if (!el) return undefined;
  const r = el.getBoundingClientRect();
  return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
}
