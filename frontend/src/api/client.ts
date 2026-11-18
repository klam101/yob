import createClient from "openapi-fetch";
import type { paths } from "./schema";

/**
 * Typed client for the yob API. Paths, params and response bodies come from
 * `schema.ts`, generated from the API's OpenAPI spec with `npm run typegen`.
 */
export const api = createClient<paths>({
  baseUrl: process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080",
});
