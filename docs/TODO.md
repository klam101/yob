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
- [x] De-duplication: the employer's job wins (company + title + workplace or location); every listing kept as attribution in `job_sources`
- [x] Himalayas, Remotive, Remote OK, with "via" and "Also listed on" links on the job page
- [x] USAJOBS when `USAJOBS_API_KEY` and `USAJOBS_EMAIL` are in `.env`
- [x] Settings → Job boards: status, jobs only on a board, listings matched
- [ ] Run USAJOBS with a real key and compare its answer with the documented shape the adapter was built from
- [ ] Better company matching across boards: "Oura Health Ltd" (Himalayas) and "ŌURA" (Remote OK) are still two companies
- [ ] Adzuna, if the snippet-only descriptions and daily call limit turn out to be worth it (skipped for now; Arbeitnow skipped: German jobs)

**Career pages, staffing and long tail**
- [ ] Career-page crawler reading schema.org JobPosting (sitemaps, robots.txt), with Web Data Commons discovery
- [ ] "Staffing agency" tag and filter (Himalayas adds many: staffing firms and AI-training gig platforms like micro1 and Mercor)
- [ ] Niche boards (government, higher ed, nonprofit) via RSS and APIs
- [ ] Hacker News "Who is hiring" via the Algolia API, parsed by the local model
- [ ] Extension: "Save this job" for a LinkedIn or Indeed page you're viewing (no crawling)

### 2. Login and saved searches
- [x] Single login account: first-visit sign-up only while no account exists, argon2 passwords, sessions in Redis, 5 failed logins per 15 minutes
- [x] Browser calls the API through the web app's `/api` proxy (first-party cookie); pages redirect to `/login` without a session
- [x] Account menu (theme, log out); `./scripts/dev.sh reset-password`
- [ ] Save, rename and delete named searches (stored filter query)

### 3. Onboarding questionnaire
- [x] `search_preferences` table and API (`GET`/`PUT /preferences`, `POST /preferences/skip`); work authorization shared with `application_answers`
- [x] Step-by-step wizard at `/onboarding`: roles and seniority, job type, workplace and places, work authorization, salary, companies and deal-breakers; the feed sends you there until it's finished or skipped
- [x] Preferences become the feed's default filters ("My preferences" chip); editable at `/preferences`, linked from Profile
- [x] New feed filters: several titles, several places (`;`), keep jobs without a salary, excluded title words and companies, hide no-sponsorship and citizens-only jobs
- [x] `no_sponsorship` and `citizens_only` read from descriptions by explicit wording (NORMALIZE_VERSION 8)
- [ ] Company size and industry filters, once company data is collected (answers are already saved)
- [ ] Title synonyms ("SWE", "Sr.", "Front-end" vs "Frontend") so fewer matching jobs are missed
- [ ] Suggest job titles from the profile's experience on the first step

### 4. Resume and profile
- [x] Profile tables, one per section (`experiences`, `educations`, `projects`, `skills`, ...); each section saves as a whole, in order
- [x] Resume upload (PDF, DOCX) to local file storage (`./data/files`), checked by content
- [x] Rule-based resume parser (no AI) into a draft, with unplaced text listed
- [x] Profile page: start card (upload or manual), tabs Personal, Experience, Education, Projects, Skills, Application answers; checklist; download uploaded resumes
- [x] Equal-employment answers encrypted with AES-256-GCM (`PROFILE_ENCRYPTION_KEY`, added to `.env` automatically)
- [x] Try the parser on your real resume and add a fixture (fictional, same layout) for anything it misreads (`smallcaps.txt`, `links.pdf`)
- [x] Text cleanup before parsing (`profile::clean`): ligatures, hyphen look-alikes (U+2011), icon-font characters, LaTeX small caps, words hyphenated across lines
- [x] Links read from the file itself (PDF link annotations, DOCX hyperlinks) and placed: LinkedIn, GitHub, portfolio, project repos
- [x] Education minor, project one-line summary, middle name and phone type; skill groups without a colon; "Class of 2026" means May
- [x] Profile page like Jobright's: one scrolling page with section tabs, editing in a side panel, one box per bullet, errors shown under the field, completeness card
- [x] Experience and education sorted newest first on save; overlapping jobs get a note, not an error
- [x] Resume page (`/resume`, was Documents): resumes list with a primary one, rename and target title, update profile from a resume, download, delete
- [x] `./scripts/dev.sh parse-resume <file>` to see what the parser makes of a file
- [x] Resume page per resume (now `/resume/edit/{id}`): its own content edited in place with autosave; what the parser changed with Keep/Undo; fix capitals; coursework chips; section order per resume
- [x] The first resume fills an empty profile automatically, once; later resumes ask before updating it
- [x] One resume page: `/resume/edit/{id}` laid out like Jobright's; "Update profile" is a dialog with section checkboxes (the separate review screen is gone)
- [x] Degrees on a resume as "X in Y" rows ("Bachelor of Science in Computer Science", "Minor in Business", "Major in Mathematics"), one row per "+"; both boxes suggest and take any text; older saved resumes upgraded in place
- [x] App-styled suggestion menu (`SuggestInput`) below the box, a few options narrowed as you type; replaces every native `<datalist>`
- [ ] Two-column PDF layouts (sidebar resumes): text extraction interleaves the columns
- [ ] Export a resume (PDF/DOCX from its content), with Stage 6 rendering
- [ ] "Analyze" a resume against a job: keyword gap (Stages 5–6)
- [ ] Optional "Improve with AI" pass over the draft with `qwen3:8b` once Ollama is set up
- [ ] Back up `PROFILE_ENCRYPTION_KEY`: without it, saved equal-employment answers can't be read

### 5. Matching
- [ ] Embed jobs and profile with `qwen3-embedding:4b` into pgvector
- [ ] Shortlist by vector similarity, then score the top jobs with `qwen3:8b` (score, breakdown, reasons)
- [ ] "Recommended" sort uses real scores; real keyword gap
- [ ] Extract experience years, education, skills, clearance and sponsorship from descriptions
- [ ] More filters from the extracted data: Education, Skills, Clearance, Sponsorship

### 6. Tailored documents
- [ ] Tailor a resume per job from the profile tables (rephrase only, never invent)
- [ ] Render ATS-safe PDF (Typst) and DOCX
- [ ] Cover letter from the same facts plus the company description
- [ ] Resume page: tailored versions and an editor (the page and the resumes list exist)
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
- [x] Phone width (390px): the top bar and the Settings → Job sources table no longer scroll the page sideways
- [x] The app works when opened at `127.0.0.1:3000` (CORS allowed only `localhost`); local URLs default to `127.0.0.1`
- [x] Himalayas: a 429 partway through ends the check early instead of failing it, and the next check waits 3 hours (it allows about 100 pages, then refuses for hours)
- [x] `./scripts/dev.sh ollama` and Ollama status in `/health`
- [ ] Start Ollama and pull the models (`./scripts/dev.sh ollama`) before the profile work
