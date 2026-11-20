import createClient from "openapi-fetch";
import type { components, paths } from "./schema";

/**
 * Typed client for the yob API. Paths, params and response bodies come from
 * `schema.ts`, generated from the API's OpenAPI spec with `npm run typegen`.
 *
 * Server components and the browser reach the API at different addresses:
 * inside Docker the server uses the `api` service name, while the browser
 * always goes through the port published on localhost.
 */
const baseUrl =
  typeof window === "undefined"
    ? (process.env.API_INTERNAL_URL ?? "http://127.0.0.1:8080")
    : (process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080");

export const api = createClient<paths>({ baseUrl });

type Schemas = components["schemas"];
export type JobSummary = Schemas["JobSummary"];
export type JobDetail = Schemas["JobDetail"];
export type JobsPage = Schemas["JobsPage"];
export type Facets = Schemas["Facets"];
export type Match = Schemas["Match"];
export type MatchLabel = Schemas["MatchLabel"];
export type Workplace = Schemas["Workplace"];
export type EmploymentType = Schemas["EmploymentType"];
export type Seniority = Schemas["Seniority"];
export type Source = Schemas["Source"];
export type Salary = Schemas["Salary"];
export type JobsQuery = NonNullable<paths["/jobs"]["get"]["parameters"]["query"]>;
