# yob to-do list

Progress by feature, in build order. Tick items as they're done and add new ones as they come up.
Design: [ARCHITECTURE.md](ARCHITECTURE.md).

## Done

### Planning
- [x] Architecture and product decisions (`docs/ARCHITECTURE.md`, `CLAUDE.md`)

### Project setup
- [x] Docker Compose: Postgres with pgvector, Redis, api, worker, frontend (localhost only)
- [x] Rust workspace: `api` (Axum, `/health`, OpenAPI spec), `worker`, `core` modules, `db` (migrations)
- [x] First migration: pgvector extension and `users` table
- [x] Next.js app with typed API client generated from the OpenAPI spec (`typegen`)
- [x] `scripts/dev.sh` and `dev.ps1` for routine commands; README command table
- [x] Starter code pushed to `main`; branch `feature/frontend-foundation`

### Frontend foundation and job feed UI (against stub data)
- [x] Stub `GET /jobs` (filters, sort, paging, facets) and `GET /jobs/{id}` with 24 fictional jobs
- [x] shadcn/ui components restyled HeroUI-like; indigo theme with light, dark and system modes
- [x] App shell: nav rail (top bar on phones), theme toggle, API status
- [x] Jobs feed: search bar, filter chips, All filters panel with live count, sort, pagination, empty and error states
- [x] Job detail page: match card, requirements, keyword gap, description, company tab, people-search links

### 1. Job feed: real data (`feature/job-ingestion`)
- [x] Migrations: `companies`, `jobs` (full-text search, USD-comparable salary), `company_checks` history
- [x] Normalized job shape and content hash in `core::jobs`
- [x] Greenhouse, Lever and Ashby adapters on their official public job-board APIs
- [x] Job descriptions cleaned to safe HTML (ammonia) before storing
- [x] Workplace, job type, seniority and salary from feed fields, titles and description text
- [x] 170 verified starter companies (about 23,000 open jobs); add a company from its board URL
- [x] Worker: incremental checks, only new jobs downloaded, jobs closed after two missed checks, reposts detected, stale flag
- [x] Adaptive schedule stored in Postgres (15 minutes, 6 hours or daily) with a per-host rate limit in Redis
- [x] `/jobs` endpoints read the database; fixtures deleted
- [x] Settings → Job sources page: companies, status, add by URL, check now
- [x] Daily pruning of descriptions on jobs closed more than 90 days

## To do

### 1b. Coverage (roadmap and details: [SOURCES.md](SOURCES.md))

Goal: virtually every US or remote job from credible sources. No LinkedIn or Indeed scraping.

**Company discovery and scale** (on `feature/job-ingestion`)
- [x] Keep only US and remote jobs at import (out-of-scope jobs stored as small rows without descriptions)
- [x] Re-normalize stored jobs when parsing rules change (seniority, scope, text-parsed salary), without refetching
- [x] Faster imports: batch inserts instead of one statement per job; unchanged jobs aren't rewritten
- [x] Discover Greenhouse, Lever and Ashby boards from Common Crawl (slow, resumable, retrying) and verify them before adding
- [x] Discover boards from links inside job descriptions
- [x] Queue many board links at once (Settings, or `./scripts/dev.sh discover <file>`)
- [x] Feed at scale: trigram index for title search, cached facets, searchable Company filter
- [x] Job sources page: discovery stats and a paginated company table
- [x] Company names for Lever and Ashby boards, read from the board page title
- [ ] Confirm a full Common Crawl pass once the index is answering. On November 20, 2026 it answered 3 pages (1,291 Greenhouse candidates, 929 added as companies), then returned 504s through November 22
- [ ] Measure `/jobs` at 200k+ jobs (release build) and add indexes where `EXPLAIN` shows scans
- [ ] Tune the US scope lists from what lands out of scope (`SELECT locations … WHERE NOT in_scope`)

**More ATS platforms**
- [x] Workday (POST to the careers page's JSON endpoint, detail per new US job)
- [x] SmartRecruiters, Workable, Recruitee, BambooHR
- [x] Discovery for each new platform (Common Crawl patterns, links in postings, pasted links)
- [x] Worker check slots: a slow board (large Workday tenant) no longer holds up other checks
- [x] Salaries written as "187,500 USD – 364,000 USD" (Workday)
- [ ] Workday tenants that stop listing at 2,000 jobs (NVIDIA, Target): split the listing by location facets
- [ ] Find more Workday tenants (no public directory; Common Crawl `*.myworkdayjobs.com` is the main hope)
- [ ] Later: Breezy, JazzHR, Personio, Teamtailor, Jobvite, iCIMS, SuccessFactors, Oracle, Eightfold

**Job board APIs**
- [ ] De-duplication: employer ATS wins; match by apply URL, then company + title + location + hash; keep every source as attribution
- [ ] Himalayas, Remotive, RemoteOK (with required attribution)
- [ ] USAJOBS and Adzuna (free API keys)

**Career pages, staffing and long tail**
- [ ] Career-page crawler reading schema.org JobPosting (sitemaps, robots.txt), with Web Data Commons discovery
- [ ] "Staffing agency" tag and filter
- [ ] Niche boards (government, higher ed, nonprofit) via RSS and APIs
- [ ] Hacker News "Who is hiring" via the Algolia API, parsed by the local model
- [ ] Extension: "Save this job" for a LinkedIn or Indeed page you're viewing (no crawling)

### 2. Login and saved searches
- [ ] Single login account (argon2 password, sessions in Redis); sign-ups stay off
- [ ] Save, rename and delete named searches (stored filter query)

### 3. Onboarding questionnaire
- [ ] `search_preferences` table and API
- [ ] Step-by-step wizard: roles, job type, location and remote, work authorization, seniority, salary and deal-breakers
- [ ] Preferences become the feed's default filters; editable later in Profile

### 4. Resume and profile
- [ ] Resume upload (PDF, DOCX) to local file storage
- [ ] Parse the resume into `profile_items` with `qwen3:8b` (JSON schema output); review and edit before saving
- [ ] Profile page tabs: Personal, Education, Experience, Skills, Application answers
- [ ] Encrypt sensitive answers (demographics, visa status) with aes-gcm

### 5. Matching
- [ ] Embed jobs and profile with `qwen3-embedding:4b` into pgvector
- [ ] Shortlist by vector similarity, then score the top jobs with `qwen3:8b` (score, breakdown, reasons)
- [ ] "Recommended" sort uses real scores; real keyword gap
- [ ] Extract experience years, education, skills, clearance and sponsorship from descriptions
- [ ] More filters from the extracted data: Education, Skills, Clearance, Sponsorship

### 6. Tailored documents
- [ ] Tailor a resume per job from `profile_items` (rephrase only, never invent)
- [ ] Render ATS-safe PDF (Typst) and DOCX
- [ ] Cover letter from the same facts plus the company description
- [ ] Documents page: base resume, tailored versions, editor
- [ ] Turn on the AI tools on the job detail page

### 7. Application tracker
- [ ] `applications` table and API
- [ ] Save and Hide work in the feed and on the detail page
- [ ] Tracker page: status tabs with counts (saved, applied, interviewing, offer, rejected), notes, expired-job banner

### 8. Autofill extension
- [ ] Chrome extension (Manifest V3, Vite) with a short-lived token from the API
- [ ] Field maps for Greenhouse, Lever and Ashby forms, then Workday
- [ ] Fill unknown forms by matching labels to the profile; local model for free-text questions
- [ ] Attach the tailored resume; highlight unfilled fields
- [ ] Log the application in the tracker on submit

### Later
- [ ] Adapters for Eightfold, iCIMS, SuccessFactors and Oracle
- [ ] Company data: industry, size, funding stage, founding year
- [ ] Sponsorship likelihood from public H-1B filing data

### Housekeeping
- [x] Rust tests for normalization, board URLs and adapter parsing (`cargo test`)
- [ ] Database tests for the ingest rules (closing, reposts); browser tests for the feed
- [ ] Review the 14 high-severity `npm audit` warnings from the Next.js starter
- [ ] Try Ctrl+C in `./scripts/dev.sh dev` to confirm it stops everything
