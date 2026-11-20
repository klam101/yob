import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"
import { cn } from "cn"
import { Slot } from "radix-ui"

// Pill-shaped chips. Tinted variants keep text readable in both themes.
const badgeVariants = cva(
  "group/badge inline-flex w-fit shrink-0 items-center justify-center gap-1 overflow-hidden rounded-full border border-transparent font-medium whitespace-nowrap transition-colors focus-visible:ring-3 focus-visible:ring-ring/40 [&>svg]:pointer-events-none [&>svg]:size-3.5!",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground",
        flat: "bg-primary/12 text-primary dark:bg-primary/20",
        secondary: "bg-secondary text-secondary-foreground",
        outline: "border-border bg-card text-foreground",
        success: "bg-success/14 text-success dark:bg-success/20",
        warning:
          "bg-warning/18 text-[color-mix(in_oklch,var(--warning),black_35%)] dark:bg-warning/20 dark:text-warning",
        destructive: "bg-destructive/12 text-destructive dark:bg-destructive/20",
        ghost: "text-muted-foreground",
      },
      size: {
        default: "h-6 px-2.5 text-xs",
        sm: "h-5 px-2 text-[0.6875rem]",
        lg: "h-7 px-3 text-[0.8125rem]",
      },
    },
    defaultVariants: {
      variant: "secondary",
      size: "default",
    },
  }
)

function Badge({
  className,
  variant = "secondary",
  size = "default",
  asChild = false,
  ...props
}: React.ComponentProps<"span"> &
  VariantProps<typeof badgeVariants> & { asChild?: boolean }) {
  const Comp = asChild ? Slot.Root : "span"

  return (
    <Comp
      data-slot="badge"
      data-variant={variant}
      className={cn(badgeVariants({ variant, size }), className)}
      {...props}
    />
  )
}

export { Badge, badgeVariants }
