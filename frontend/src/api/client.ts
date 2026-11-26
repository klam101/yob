import createClient from "openapi-fetch";
import type { components, paths } from "./schema";

/**
 * Typed client for the yob API. Paths, params and response bodies come from
 * `schema.ts`, generated from the API's OpenAPI spec with `npm run typegen`.
 *
 * The browser calls `/api` on this same origin; `src/proxy.ts` forwards it to
 * the API, so the session cookie stays first-party. Server components call
 * the API directly (the `api` service inside Docker); personal endpoints need
 * the visitor's cookie, so use `serverApi()` for those.
 */
const baseUrl =
  typeof window === "undefined"
    ? (process.env.API_INTERNAL_URL ?? "http://127.0.0.1:8080")
    : "/api";

export const api = createClient<paths>({ baseUrl });

type Schemas = components["schemas"];
export type JobSummary = Schemas["JobSummary"];
export type JobDetail = Schemas["JobDetail"];
export type JobsPage = Schemas["JobsPage"];
export type Facets = Schemas["Facets"];
export type Vocabulary = Schemas["Vocabulary"];
export type Match = Schemas["Match"];
export type MatchLabel = Schemas["MatchLabel"];
export type Workplace = Schemas["Workplace"];
export type EmploymentType = Schemas["EmploymentType"];
export type Seniority = Schemas["Seniority"];
export type Platform = Schemas["Platform"];
export type Source = Schemas["Source"];
export type Feed = Schemas["Feed"];
export type FeedStatus = Schemas["FeedStatus"];
export type Listing = Schemas["Listing"];
export type CompanySource = Schemas["CompanySource"];
export type AddCompany = Schemas["AddCompany"];
export type CompaniesPage = Schemas["CompaniesPage"];
export type DiscoveryStats = Schemas["DiscoveryStats"];
export type Salary = Schemas["Salary"];
export type SuggestKind = Schemas["SuggestKind"];
export type Suggestion = Schemas["Suggestion"];
export type JobsQuery = NonNullable<paths["/jobs"]["get"]["parameters"]["query"]>;
export type Profile = Schemas["Profile"];
export type Personal = Schemas["Personal"];
export type ProfileLink = Schemas["Link"];
export type LinkKind = Schemas["LinkKind"];
export type Education = Schemas["Education"];
export type Experience = Schemas["Experience"];
export type Project = Schemas["Project"];
export type Skill = Schemas["Skill"];
export type Answers = Schemas["Answers"];
export type Eeo = Schemas["Eeo"];
export type ResumeUploaded = Schemas["ResumeUploaded"];
export type ApplyResult = Schemas["ApplyResult"];
export type ProfileSection = Schemas["ProfileSection"];
export type ResumeInfo = Schemas["ResumeInfo"];
export type ResumeUpdate = Schemas["ResumeUpdate"];
export type ResumeDetail = Schemas["ResumeDetail"];
export type ResumeContent = Schemas["ResumeContent"];
export type ResumeChange = Schemas["Change"];
export type ResumeEducation = Schemas["ResumeEducation"];
export type ResumeDegree = Schemas["ResumeDegree"];
export type ResumeExperience = Schemas["ResumeExperience"];
export type SectionKind = Schemas["SectionKind"];
export type FilledFrom = Schemas["FilledFrom"];
export type Preferences = Schemas["Preferences"];
export type SearchPreferences = Schemas["SearchPreferences"];
export type CompanySize = Schemas["CompanySize"];
export type WorkAuthorization = Schemas["WorkAuthorization"];
export type Onboarding = Schemas["Onboarding"];
