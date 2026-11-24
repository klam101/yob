import createClient from "openapi-fetch";
import { cookies } from "next/headers";
import type { paths } from "./schema";

/**
 * API client for server components that call personal endpoints: forwards
 * the visitor's cookies so the API sees their session. Reading cookies makes
 * the caller dynamic, so it must sit inside `<Suspense>`.
 */
export async function serverApi() {
  const cookie = (await cookies()).toString();
  return createClient<paths>({
    baseUrl: process.env.API_INTERNAL_URL ?? "http://127.0.0.1:8080",
    headers: cookie ? { cookie } : {},
  });
}
