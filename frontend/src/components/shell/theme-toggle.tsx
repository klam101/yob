"use client";

import { Moon, Sun } from "lucide-react";
import { useTheme } from "next-themes";
import { useSyncExternalStore } from "react";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "cn";
import { centerOf, useThemeSwitch } from "./use-theme-switch";

// True only after hydration, so nothing mismatches the server render.
const subscribe = () => () => {};
const useMounted = () => useSyncExternalStore(subscribe, () => true, () => false);

/** Switches between light and dark mode (System stays in the account menu). */
export function ThemeToggle({ className }: { className?: string }) {
  const { resolvedTheme } = useTheme();
  const switchTheme = useThemeSwitch();
  const mounted = useMounted();
  const dark = mounted && resolvedTheme === "dark";
  const label = dark ? "Switch to light mode" : "Switch to dark mode";
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          aria-label={label}
          onClick={(e) => switchTheme(dark ? "light" : "dark", centerOf(e.currentTarget))}
          className={cn("text-muted-foreground hover:text-foreground", className)}
        >
          {dark ? <Sun /> : <Moon />}
        </Button>
      </TooltipTrigger>
      <TooltipContent side="right">{label}</TooltipContent>
    </Tooltip>
  );
}
