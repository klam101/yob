import { cn } from "cn";

/* eslint-disable @next/next/no-img-element -- small SVGs; next/image adds nothing here. */

/** The yob icon (red tile, gold yale), the same in light and dark mode. */
export function LogoMark({ size = 36, className }: { size?: number; className?: string }) {
  return (
    <img
      src="/brand/yob-icon.svg"
      alt="yob"
      width={size}
      height={size}
      className={cn("shrink-0 rounded-[28%] shadow-soft", className)}
    />
  );
}

/**
 * Icon plus wordmark: red wordmark on light, gold on dark. Both are in the
 * page and CSS shows one, so the right logo appears before any script runs.
 */
export function Logo({ height = 40, className }: { height?: number; className?: string }) {
  const width = Math.round((height * 326) / 120);
  return (
    <span className={cn("inline-flex", className)}>
      <img src="/brand/yob-logo-light.svg" alt="yob" width={width} height={height} className="dark:hidden" />
      <img src="/brand/yob-logo-dark.svg" alt="yob" width={width} height={height} className="hidden dark:block" />
    </span>
  );
}
