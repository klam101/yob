import * as React from "react"
import { cn } from "cn"

// Matches Input: filled, rounded, card background on focus.
function Textarea({ className, ...props }: React.ComponentProps<"textarea">) {
  return (
    <textarea
      data-slot="textarea"
      className={cn(
        "flex field-sizing-content min-h-20 w-full rounded-xl border border-transparent bg-muted px-3 py-2 text-base transition-[color,background-color,border-color,box-shadow] outline-none placeholder:text-muted-foreground hover:bg-[color-mix(in_oklch,var(--muted),var(--foreground)_4%)] focus-visible:border-ring focus-visible:bg-card focus-visible:ring-3 focus-visible:ring-ring/25 disabled:cursor-not-allowed disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-3 aria-invalid:ring-destructive/20 md:text-sm",
        className
      )}
      {...props}
    />
  )
}

export { Textarea }
