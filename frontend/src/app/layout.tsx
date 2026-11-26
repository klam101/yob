import type { Metadata, Viewport } from "next";
import { Geist_Mono, Space_Grotesk } from "next/font/google";
import { ThemeProvider } from "@/components/shell/theme";
import { TooltipProvider } from "@/components/ui/tooltip";
import "./globals.css";

// The brand typeface, self-hosted by Next at build time.
const spaceGrotesk = Space_Grotesk({
  variable: "--font-space-grotesk",
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

export const viewport: Viewport = {
  themeColor: [
    { media: "(prefers-color-scheme: light)", color: "#f6f0e4" },
    { media: "(prefers-color-scheme: dark)", color: "#15120f" },
  ],
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html
      lang="en"
      // next-themes sets the theme class before React hydrates.
      suppressHydrationWarning
      className={`${spaceGrotesk.variable} ${geistMono.variable} h-full`}
    >
      <body>
        <ThemeProvider>
          <TooltipProvider delayDuration={300}>
            {children}
          </TooltipProvider>
        </ThemeProvider>
      </body>
    </html>
  );
}
