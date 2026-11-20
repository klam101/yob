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

## To do

### 1. Job feed: real data
- [ ] Migration: `companies` and `jobs` tables (fields from ARCHITECTURE.md) with indexes and full-text search
- [ ] Normalized job type and content hash in `core::jobs`
- [ ] Greenhouse adapter (confirm the public board API endpoint first)
- [ ] Lever adapter
- [ ] Ashby adapter
- [ ] Clean job descriptions to safe HTML before storing
- [ ] Derive workplace, job type and seniority from feed fields and titles; parse salary from text when the feed has none
- [ ] Starter list of companies on each platform, plus adding a company from its board URL
- [ ] Worker: fetch each company's job ids, download only new jobs, close missing ones, detect reposts, flag stale jobs
- [ ] Adaptive polling (15 minutes for active companies, 6 to 24 hours for quiet ones) with a per-host rate limit in Redis
- [ ] Job queue on Redis Streams and a `tasks` history table
- [ ] Replace the stub `/jobs` endpoints with database queries; delete the fixtures
- [ ] Check end to end: the feed shows fresh jobs from a few hundred companies

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
- [ ] Adapters for Workday, Eightfold, iCIMS, SuccessFactors, Oracle and SmartRecruiters
- [ ] Company data: industry, size, funding stage, founding year
- [ ] Sponsorship likelihood from public H-1B filing data

### Housekeeping
- [ ] Tests: Rust unit tests for filters and adapters; browser tests for the feed
- [ ] Review the 14 high-severity `npm audit` warnings from the Next.js starter
- [ ] Try Ctrl+C in `./scripts/dev.sh dev` to confirm it stops everything
