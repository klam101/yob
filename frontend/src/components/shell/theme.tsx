"use client";

import { ThemeProvider as NextThemes } from "next-themes";

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  return (
    // Light by default (the brand's main look). Both the `dark` class (for
    // Tailwind's dark: variant) and `data-theme` (the brand's switch) are set.
    <NextThemes
      attribute={["class", "data-theme"]}
      defaultTheme="light"
      enableSystem
      disableTransitionOnChange
    >
      {children}
    </NextThemes>
  );
}
