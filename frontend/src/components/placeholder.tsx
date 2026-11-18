/** Stand-in for a page whose feature hasn't been built yet. */
export function Placeholder({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="space-y-3">
      <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
      <p className="max-w-prose text-zinc-600 dark:text-zinc-400">{children}</p>
      <p className="text-sm text-zinc-500">Not built yet.</p>
    </section>
  );
}
