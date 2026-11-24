"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { api } from "@/api/client";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

const MIN_PASSWORD = 10;

/**
 * Log in, or create the account on the very first visit. Sign-up only works
 * while no account exists; the API enforces that too.
 */
export function LoginForm({ mode, next, apiDown }: { mode: "login" | "signup"; next: string; apiDown: boolean }) {
  const router = useRouter();
  const signup = mode === "signup";
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [repeat, setRepeat] = useState("");
  const [error, setError] = useState<string | null>(apiDown ? "Can't reach the API. Is the stack running?" : null);
  const [busy, setBusy] = useState(false);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    if (signup && password !== repeat) {
      setError("The passwords don't match.");
      return;
    }
    setBusy(true);
    try {
      const body = { email, password };
      const { error: failed } = signup
        ? await api.POST("/auth/signup", { body })
        : await api.POST("/auth/login", { body });
      if (failed) {
        setError(failed.message);
        return;
      }
      setPassword("");
      setRepeat("");
      router.replace(next);
      router.refresh();
    } catch {
      setError("Can't reach the API. Is the stack running?");
    } finally {
      setBusy(false);
    }
  }

  return (
    <form onSubmit={submit} className="space-y-5 rounded-3xl border border-border/70 bg-card p-6 shadow-soft sm:p-8">
      <div className="space-y-1">
        <h1 className="text-xl font-semibold tracking-tight">{signup ? "Create your account" : "Log in"}</h1>
        <p className="text-sm text-muted-foreground">
          {signup
            ? "yob has one account, and this is it. Everything stays on this computer."
            : "Welcome back."}
        </p>
      </div>

      <label className="block space-y-1.5">
        <span className="text-sm font-medium">Email</span>
        <Input
          type="email"
          autoComplete="email"
          required
          value={email}
          onChange={(e) => setEmail(e.target.value)}
        />
      </label>
      <div className="space-y-1.5">
        <label className="block space-y-1.5">
          <span className="text-sm font-medium">Password</span>
          <Input
            type="password"
            autoComplete={signup ? "new-password" : "current-password"}
            required
            minLength={signup ? MIN_PASSWORD : undefined}
            aria-describedby={signup ? "password-hint" : undefined}
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
        </label>
        {signup && (
          <p id="password-hint" className="text-xs text-muted-foreground">
            At least {MIN_PASSWORD} characters.
          </p>
        )}
      </div>
      {signup && (
        <label className="block space-y-1.5">
          <span className="text-sm font-medium">Repeat password</span>
          <Input
            type="password"
            autoComplete="new-password"
            required
            value={repeat}
            onChange={(e) => setRepeat(e.target.value)}
          />
        </label>
      )}

      {error && (
        <p role="alert" className="rounded-xl bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </p>
      )}

      <Button type="submit" className="w-full" size="lg" disabled={busy}>
        {busy ? "One moment…" : signup ? "Create account" : "Log in"}
      </Button>
      {!signup && (
        <p className="text-center text-xs text-muted-foreground">
          Forgot it? Run <code className="rounded bg-muted px-1 py-0.5">./scripts/dev.sh reset-password</code>
        </p>
      )}
    </form>
  );
}
