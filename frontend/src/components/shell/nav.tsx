"use client";

import { Briefcase, FileText, KanbanSquare, Settings, UserRound } from "lucide-react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { cn } from "cn";

export const NAV = [
  { href: "/jobs", label: "Jobs", icon: Briefcase },
  { href: "/tracker", label: "Tracker", icon: KanbanSquare },
  { href: "/documents", label: "Documents", icon: FileText },
  { href: "/profile", label: "Profile", icon: UserRound },
  { href: "/settings", label: "Settings", icon: Settings },
] as const;

/** Vertical rail on desktop, horizontal row on phones. */
export function Nav() {
  return <NavLinks pathname={usePathname()} />;
}

/**
 * The links themselves. Rendered without a pathname as the Suspense fallback,
 * because the current URL is only known at request time on dynamic routes.
 */
export function NavLinks({ pathname }: { pathname: string }) {
  return (
    <nav aria-label="Main" className="flex gap-0.5 sm:gap-1 md:flex-col md:gap-1.5">
      {NAV.map(({ href, label, icon: Icon }) => {
        // Job preferences are part of the profile.
        const active =
          pathname === href || pathname.startsWith(`${href}/`) || (href === "/profile" && pathname === "/preferences");
        return (
          <Link
            key={href}
            href={href}
            aria-current={active ? "page" : undefined}
            className={cn(
              "group flex flex-col items-center gap-1 rounded-xl px-0.5 py-1.5 text-[0.6875rem] sm:px-2 font-medium transition-colors md:w-16 md:py-2",
              active ? "text-primary" : "text-muted-foreground hover:text-foreground",
            )}
          >
            <span
              className={cn(
                "flex size-9 items-center justify-center rounded-xl transition-[background-color,transform] duration-150 group-active:scale-95",
                active ? "bg-primary/12 dark:bg-primary/20" : "group-hover:bg-muted",
              )}
            >
              <Icon className="size-[1.15rem]" strokeWidth={active ? 2.25 : 2} />
            </span>
            <span className="sr-only sm:not-sr-only">{label}</span>
          </Link>
        );
      })}
    </nav>
  );
}
