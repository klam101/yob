import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import { ApiStatus } from "@/components/api-status";
import { Nav } from "@/components/nav";
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
  title: "yob",
  description: "Personal job search: one feed of fresh jobs, tailored documents, autofill.",
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html
      lang="en"
      className={`${geistSans.variable} ${geistMono.variable} h-full antialiased`}
    >
      <body className="flex min-h-full flex-col font-sans md:flex-row">
        <aside className="flex flex-col gap-4 border-b border-zinc-200 p-4 md:w-56 md:shrink-0 md:border-r md:border-b-0 dark:border-zinc-800">
          <span className="px-3 text-lg font-semibold tracking-tight">yob</span>
          <Nav />
          <div className="px-3 md:mt-auto">
            <ApiStatus />
          </div>
        </aside>
        <main className="flex-1 p-6 md:p-10">{children}</main>
      </body>
    </html>
  );
}
