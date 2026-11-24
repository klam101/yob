import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import Link from "next/link";
import { Suspense } from "react";
import { ApiStatus } from "@/components/shell/api-status";
import { Nav, NavLinks } from "@/components/shell/nav";
import { ThemeProvider, ThemeToggle } from "@/components/shell/theme";
import { TooltipProvider } from "@/components/ui/tooltip";
import "./globals.css";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: { default: "yob", template: "%s · yob" },
  description: "Personal job search: one feed of fresh jobs, tailored documents, autofill.",
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html
      lang="en"
      // next-themes sets the theme class before React hydrates.
      suppressHydrationWarning
      className={`${geistSans.variable} ${geistMono.variable} h-full`}
    >
      <body className="min-h-full">
        <ThemeProvider>
          <TooltipProvider delayDuration={300}>
            <div className="flex min-h-dvh flex-col md:flex-row">
              <aside className="sticky top-0 z-30 flex items-center justify-between gap-2 border-b border-border/70 bg-background/85 px-3 py-2 backdrop-blur md:h-dvh md:w-20 md:flex-col md:justify-start md:border-r md:border-b-0 md:px-2 md:py-4">
                <Link
                  href="/jobs"
                  className="flex items-center gap-2 rounded-xl px-1 md:mb-4 md:flex-col md:gap-1"
                  aria-label="yob home"
                >
                  <span className="flex size-9 items-center justify-center rounded-xl bg-primary text-sm font-bold text-primary-foreground shadow-soft">
                    y
                  </span>
                  <span className="hidden text-sm font-semibold tracking-tight sm:inline md:hidden">yob</span>
                </Link>
                <Suspense fallback={<NavLinks pathname="" />}>
                  <Nav />
                </Suspense>
                <div className="flex items-center gap-0.5 sm:gap-1 md:mt-auto md:flex-col">
                  <ThemeToggle />
                  <ApiStatus />
                </div>
              </aside>
              <main className="min-w-0 flex-1">{children}</main>
            </div>
          </TooltipProvider>
        </ThemeProvider>
      </body>
    </html>
  );
}
