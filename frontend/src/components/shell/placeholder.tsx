import type { LucideIcon } from "lucide-react";
import { Badge } from "@/components/ui/badge";

/** Stand-in for a page whose feature hasn't been built yet. */
export function Placeholder({
  title,
  icon: Icon,
  children,
}: {
  title: string;
  icon: LucideIcon;
  children: React.ReactNode;
}) {
  return (
    <section className="mx-auto flex max-w-xl flex-col items-center gap-4 py-24 text-center">
      <span className="flex size-14 items-center justify-center rounded-2xl bg-primary/12 text-primary dark:bg-primary/20">
        <Icon className="size-6" />
      </span>
      <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
      <p className="text-muted-foreground">{children}</p>
      <Badge variant="flat">Coming soon</Badge>
    </section>
  );
}
