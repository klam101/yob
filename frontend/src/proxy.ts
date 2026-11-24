import { type NextRequest, NextResponse } from "next/server";

/** Same name as the API's session cookie (`routes/auth.rs`). */
const SESSION_COOKIE = "yob_session";

/**
 * Runs before every page and `/api` request.
 *
 * - `/api/*` is forwarded to the API, so the browser only ever talks to this
 *   origin and the session cookie is first-party.
 * - Pages need a session cookie; without one they redirect to `/login`. The
 *   API still checks the session itself on every personal request.
 */
export function proxy(request: NextRequest) {
  const { pathname, search } = request.nextUrl;

  if (pathname === "/api" || pathname.startsWith("/api/")) {
    const target = new URL(
      pathname.slice("/api".length) + search,
      process.env.API_INTERNAL_URL ?? "http://127.0.0.1:8080",
    );
    return NextResponse.rewrite(target);
  }

  if (pathname === "/login" || request.cookies.has(SESSION_COOKIE)) {
    return NextResponse.next();
  }
  const login = new URL("/login", request.url);
  if (pathname !== "/") login.searchParams.set("next", pathname + search);
  return NextResponse.redirect(login);
}

export const config = {
  // Everything except Next's own assets and files with an extension.
  matcher: ["/((?!_next/static|_next/image|.*\\.[a-zA-Z0-9]+$).*)"],
};
