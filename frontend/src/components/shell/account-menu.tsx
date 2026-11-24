"use client";

import { LogOut, Monitor, Moon, Sun } from "lucide-react";
import { useTheme } from "next-themes";
import { useRouter } from "next/navigation";
import { useEffect, useState, useSyncExternalStore } from "react";
import { api } from "@/api/client";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

// True only after hydration, so nothing mismatches the server render.
const subscribe = () => () => {};
function useMounted() {
  return useSyncExternalStore(subscribe, () => true, () => false);
}

/** The account button: who's logged in, the theme, and Log out. */
export function AccountMenu() {
  const router = useRouter();
  const { theme, setTheme } = useTheme();
  const mounted = useMounted();
  const [email, setEmail] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    api
      .GET("/auth/me")
      .then(({ data, response }) => {
        if (cancelled) return;
        if (data) setEmail(data.email);
        // The cookie is there but the session ended (expired, or the password
        // was reset): back to the login page.
        else if (response.status === 401) router.replace("/login");
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [router]);

  async function logOut() {
    await api.POST("/auth/logout").catch(() => {});
    // A full page load, so no page or form state from the session survives.
    window.location.assign("/login");
  }

  const initial = email?.[0]?.toUpperCase();
  return (
    <DropdownMenu>
      <Tooltip>
        <TooltipTrigger asChild>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="icon" aria-label="Account and theme">
              {initial ? (
                <span className="flex size-7 items-center justify-center rounded-full bg-primary/12 text-xs font-semibold text-primary dark:bg-primary/20">
                  {initial}
                </span>
              ) : (
                <Monitor />
              )}
            </Button>
          </DropdownMenuTrigger>
        </TooltipTrigger>
        <TooltipContent side="right">Account</TooltipContent>
      </Tooltip>
      <DropdownMenuContent side="right" align="end" className="w-56">
        {email && (
          <>
            <DropdownMenuLabel className="truncate font-normal text-muted-foreground">{email}</DropdownMenuLabel>
            <DropdownMenuSeparator />
          </>
        )}
        <DropdownMenuLabel className="text-xs text-muted-foreground">Theme</DropdownMenuLabel>
        <DropdownMenuRadioGroup value={mounted ? theme : undefined} onValueChange={setTheme}>
          <DropdownMenuRadioItem value="light">
            <Sun /> Light
          </DropdownMenuRadioItem>
          <DropdownMenuRadioItem value="dark">
            <Moon /> Dark
          </DropdownMenuRadioItem>
          <DropdownMenuRadioItem value="system">
            <Monitor /> System
          </DropdownMenuRadioItem>
        </DropdownMenuRadioGroup>
        <DropdownMenuSeparator />
        <DropdownMenuItem onSelect={logOut}>
          <LogOut /> Log out
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
