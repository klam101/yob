import Link from "next/link";
import { Suspense } from "react";
import { AccountMenu } from "@/components/shell/account-menu";
import { ApiStatus } from "@/components/shell/api-status";
import { LogoMark } from "@/components/shell/logo";
import { Nav, NavLinks } from "@/components/shell/nav";
import { ThemeToggle } from "@/components/shell/theme-toggle";

/** The app shell: nav rail on desktop, top bar on phones. */
export default function MainLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex min-h-dvh flex-col md:flex-row">
      <aside className="sticky top-0 z-30 flex items-center justify-between gap-2 border-b border-border/70 bg-background/85 px-3 py-2 backdrop-blur md:h-dvh md:w-20 md:flex-col md:justify-start md:border-r md:border-b-0 md:px-2 md:py-4">
        <Link
          href="/jobs"
          className="flex items-center gap-2 rounded-xl px-1 md:mb-4 md:flex-col md:gap-1"
          aria-label="yob home"
        >
          <LogoMark size={36} />
          {/* The wordmark beside the icon on the phone top bar. */}
          <span aria-hidden className="hidden sm:inline md:hidden">
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img src="/brand/yob-wordmark-light.svg" alt="" className="h-5 w-auto dark:hidden" />
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img src="/brand/yob-wordmark-dark.svg" alt="" className="hidden h-5 w-auto dark:block" />
          </span>
        </Link>
        <Suspense fallback={<NavLinks pathname="" />}>
          <Nav />
        </Suspense>
        <div className="flex items-center gap-0.5 sm:gap-1 md:mt-auto md:flex-col">
          {/* No room on small phones; the account menu has the theme too. */}
          <ThemeToggle className="hidden sm:inline-flex" />
          <AccountMenu />
          <ApiStatus />
        </div>
      </aside>
      <main className="min-w-0 flex-1">{children}</main>
    </div>
  );
}
