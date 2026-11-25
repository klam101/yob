# Job search app architecture

The app is one codebase with clear internal modules (a modular monolith), Postgres, Redis, a background worker, local AI through Ollama, and a Chrome extension. That's enough to pull jobs from across the web, rank them against your profile, write tailored resumes and cover letters, and autofill applications, without running a fleet of services or paying for APIs.

## Goals and scope

The app replaces scouring dozens of sites with one feed of relevant jobs, then helps you apply to each one well.

- **Find:** one searchable feed of fresh listings from many company career pages, in the spirit of hiringcafe.com.
- **Match:** rank each job against your profile so the best fits rise to the top.
- **Tailor:** generate an ATS-friendly resume and cover letter per job, plus a keyword gap check.
- **Apply:** a browser extension that reads the application page and fills it from your profile, in the spirit of jobright.ai.
- **Track:** every job you saved, applied to or heard back from, in one place.

Out of scope for v1: fully automatic applying, multiple users, mobile apps, hosting online. The design leaves room for all of them.

## System overview

```text
 Web app (Next.js) ─────┐
                        ├──> API ──┐            ┌──> Ollama (local AI, native Windows app)
 Chrome extension ──────┘          │  Backend   │
        │                 Worker ──┘  (Rust)  ──┼──> Company job feeds (Greenhouse, Lever, Ashby, Workday...)
        v                                       ├──> Job board APIs (Himalayas, Remote OK, USAJOBS...)
 Application pages                              ├──> Postgres + pgvector (all saved data)
 (company career sites)                         ├──> File storage (resumes, cover letters)
                                                └──> Redis (queue, cache, rate limits, sessions)
```

The web app and extension only talk to the API. The backend is the only thing that touches the database, Redis, file storage, the job feeds and the AI models.

## Components

Six pieces, but only two processes to run on a server: the API and the worker. Both are built from the same backend codebase.

| Component | What it does | Runs where |
| --- | --- | --- |
| Web app | Job feed, search and filters, questionnaire, profile editor, document editor, application tracker | Browser (served by Next.js) |
| API | One backend with internal modules: `jobs`, `profile`, `matching`, `documents`, `applications`, `auth` | Rust binary in Docker |
| Worker | Feed checks, embedding new jobs, fit scoring, generating documents, anything slow | Rust binary in Docker, same codebase as the API |
| Database | Postgres with `pgvector`; the source of truth for everything saved | Docker |
| Redis | Work queue (Redis Streams), cache for searches and AI results, per-site rate limits, login sessions | Docker |
| Ollama | Runs the embedding and text models on the GPU | Native Windows app |
| File storage | Uploaded and generated resumes and cover letters (PDF, DOCX) | Local `./data/files` now, S3-compatible later |
| Extension | Reads application forms, fills them from your profile, attaches the right resume, logs the application | Chrome (Manifest V3) |

Redis only holds short-lived data. If it is wiped, nothing is lost: queued work is re-created by the schedule and caches rebuild.

Why a modular monolith: each module owns its tables and exposes a small interface to the others, so the code stays easy to follow, and any part can be lifted out later without a rewrite.

## Data model

Profile is stored as structured facts once, and every resume, cover letter and autofill answer is generated from those facts. That's what keeps the documents consistent and the extension accurate.

| Table | Holds | Key links |
| --- | --- | --- |
| `users` | Login, settings | |
| `search_preferences` | Questionnaire answers: titles, seniority, job types, workplaces, locations, salary floor (and whether to keep jobs without a salary), company sizes, industries, deal-breakers (title words, companies); `completed_at` is NULL while skipped. Work authorization is kept in `application_answers` | `user_id` |
| `profiles`, `profile_links` | Name, contact details, address, headline, summary; LinkedIn, GitHub and portfolio links | `user_id` |
| `experiences`, `educations`, `projects`, `skills` | The profile's lists, one row per entry with a `position` for order, month dates (an experience with no end is current) and bullets as an ordered `text[]` | `user_id` |
| `application_answers` | Work authorization and sponsorship in plain columns; equal-employment answers as one AES-256-GCM blob (key `PROFILE_ENCRYPTION_KEY`) | `user_id` |
| `resumes` | Uploaded resumes: file key in file storage, file name and display name, type, extracted text and link targets, target job title, `is_primary` (one per user), and `content`: the resume's own edited content as JSON (`ResumeContent`) | `user_id` |
| `companies` | Name, careers site, platform (ATS, or `external` for employers known only from a job board), board id, polling interval, last checked | |
| `jobs` | Title, location, salary, description, posted date, first seen, closed at, repost count, apply URL, source, content hash, embedding; `feed` when only a job board lists it | `company_id` |
| `feeds`, `job_sources` | Job boards with public APIs and their schedule; every board listing, pointing at the job it was matched to | `job_id` |
| `job_matches` | Fit score and reasons for one user and one job | `user_id`, `job_id` |
| `documents` | A generated resume or cover letter: content, file path, version | `user_id`, `job_id` |
| `applications` | Status (saved, applied, interviewing, offer, rejected), dates, notes, documents sent | `user_id`, `job_id` |
| `tasks` | History of background jobs and results (the live queue is in Redis) | |

Every personal table carries `user_id` even though v1 has one user, so adding users later is a login change, not a schema rewrite.

**Login.** One account. Sign-up works only while `users` is empty, so the first visit creates the account and after that there is only login (checked in the API, under a table lock). Passwords are argon2id hashes. A session is a random 256-bit token in an `HttpOnly`, `SameSite=Lax` cookie (`yob_session`); Redis keeps only its SHA-256 hash with a 30-day expiry that each request refreshes, so a Redis dump can't be replayed. Failed logins are limited to 5 per email per 15 minutes. The browser calls the API through the web app's `/api` path (`frontend/src/proxy.ts` forwards it), so the cookie is first-party; the same proxy redirects pages to `/login` without a session. Personal API routes take the `CurrentUser` extractor and answer 401 without a session. `./scripts/dev.sh reset-password` sets a new password and ends every session.

**Profile.** Lists are stored the relational way: five jobs are five rows in `experiences`, not five columns. Each section saves whole (`PUT /profile/{section}`), replacing its rows in one transaction. Experiences and educations are saved newest first (current entries, then by end date), whatever order they were sent in; projects and links keep the order given. Validation lives in core, and an error about one field names it (`{"message", "field": "experiences.2.end"}`) so the editor shows it under that field. Overlapping jobs are allowed (the editor only notes them).

The Profile page is one scrolling page with tabs that jump to its sections (Personal, Education, Work experience, Skills, Application answers); each section edits in a side panel, with one box per bullet. Projects aren't on it: they're edited on each resume's edit page and copied to the profile with Update profile. The profile is filled by hand or from a resume:

- `POST /profile/resume` takes a PDF or DOCX (checked by its bytes, 5 MB at most) and keeps the file in file storage. `resume::extract` returns the text (`pdf-extract`; DOCX from `word/document.xml`, with Word's list paragraphs as bullets) and the **link targets** (PDF `/Link` annotations in reading order, DOCX hyperlink relationships), because a resume often shows only "GitHub" or a user name and the address is only in the link.
- `core::profile::clean` first undoes text-extraction quirks, one function each: compatibility characters (NFKC), hyphen look-alikes (LaTeX and Word emit U+2011, which breaks date ranges), icon-font characters (Font Awesome; a GitHub icon before a user name becomes that address), LaTeX small caps (whose "i" comes out lowercase: "QUALiTY"), and words hyphenated across lines.
- `core::profile::parse` then works **with rules, no AI**: contact details by pattern, sections by known headings, entries split where a new dated header block starts, dates in a dozen formats, title vs. company by job-title words, degree lines split into degree, major and minor, skill groups with or without a colon, and wrapped lines joined back. Links are placed by kind and name: LinkedIn and GitHub accounts, a project's repository by matching its name, the first other header link as the portfolio, and the rest in order to projects whose header showed a link label.
- The parser also records **what it changed** from the file's literal text (`Change`: field path, kind, before, after, and values to undo): capitals, a degree line split into degree/major/minor, a link label taken off a project name, "Class of 2026" read as May, addresses found only in the file's links, words joined across lines, small caps fixed. `./scripts/dev.sh parse-resume <file>` prints each step, including these changes, for a file that misreads.
- **Each resume keeps its own content** (`resumes.content`, `core::profile::resume_content`): the parse, cleaned up, then edited in place on the resume edit page (`/resume/edit/{id}`, laid out like Jobright's; `PUT /profile/resumes/{id}/content`, saved as you type). The page lists the parser's changes with Keep and Undo (in a side panel opened from the bar at the top of the resume), fixes capitals per field or all at once, shows coursework as chips, and orders sections per resume. Entries and links keep their places when saved, empty ones included, so the changes' field paths stay valid; deleting a row moves the changes along. Editing a resume never changes the profile. Resumes uploaded before contents were kept are read from their file the first time they're opened; "Read the file again" (in Edit info) replaces the content with a fresh parse.
- **Update profile is a dialog, not a page** (`core::profile::apply`, `POST /profile/resumes/{id}/apply`): the user picks sections (personal details, education, work experience, projects, skills). Personal details merge: what the resume has replaces the saved value, the rest (street, ZIP, phone type) stays, and links are added by address. Other sections are replaced, with the usual validation and newest-first order. A section that fails validation is left as it was and reported with the resume field to fix ("Show me"); the others still save.
- **The first resume fills the profile automatically, once** (`apply::fill_once`, the same path with every section): while the profile is empty and was never filled (`profiles.filled_at`), on upload or on the next `GET /profile`. The Profile page then shows "Filled from {resume}" with a link to the parser's changes. Later uploads open their edit page with the Update profile dialog already open.

## Job ingestion

Jobs come mainly from the hiring systems companies use for their career pages. hiringcafe works the same way at a much larger scale (about 2.8 million listings from 46 platforms): nothing pushes new jobs to it; it re-checks career pages often. Job boards with free public APIs fill gaps, matched against those first (see "Job boards" below).

**Adapters per platform, not per company.** One adapter covers every company on that platform.

| Order | Platform | How we read it |
| --- | --- | --- |
| 1 | Greenhouse | `boards-api.greenhouse.io/v1/boards/{token}/jobs` (light list); `?content=true` adds descriptions (HTML-escaped) for all jobs in one request. No workplace or employment type, so those are inferred. |
| 1 | Lever | `api.lever.co/v0/postings/{company}?mode=json`, everything in one request. Rate-limits quickly. |
| 1 | Ashby | `api.ashbyhq.com/posting-api/job-board/{name}?includeCompensation=true`, everything in one request, with structured pay |
| 2 | Workday | The JSON the careers page loads. Token `tenant/wdN/site`. List: POST `/wday/cxs/{tenant}/{site}/jobs`, 20 per page; detail: GET per job. Some tenants stop listing at 2,000 jobs. |
| 2 | SmartRecruiters | Official Posting API, `api.smartrecruiters.com/v1/companies/{id}/postings`, 100 per page; detail per job. Unknown ids return an empty list. |
| 2 | Workable | `apply.workable.com/api/v1/widget/accounts/{account}?details=true`, everything in one request. Only says "telecommuting" or not. |
| 2 | Recruitee | `{company}.recruitee.com/api/offers/`, everything in one request, with remote and hybrid flags and sometimes pay |
| 2 | BambooHR | The JSON the careers page loads: `{company}.bamboohr.com/careers/list`, then `/careers/{id}/detail` per job. A missing company redirects to bamboohr.com. |
| 3 | Eightfold | The JSON the careers page loads (Microsoft, Starbucks, Qualcomm, PayPal are listed as customers) |
| 3 | iCIMS, SuccessFactors, Oracle | Confirmed one at a time |
| never | LinkedIn, Indeed, hiringcafe | Terms forbid scraping |

The endpoints for tiers 1 and 2 were confirmed in November 2026; the rest get confirmed when each adapter is built. Code: `backend/crates/core/src/jobs/` (`sources/` per platform, `normalize.rs`, `ingest.rs`, `store.rs`).

**Postgres is the job database; pages never call job boards.** The worker keeps a stored copy fresh in the background, and the feed and detail pages only query Postgres. They stay fast and keep working with the worker stopped or offline. Data survives restarts in the `pgdata` volume. Around 23,000 open jobs take about 230 MB.

**Finding companies** (`discover.rs`; roadmap in [SOURCES.md](SOURCES.md)). A verified starter list ships in `seed_companies.json` (170 companies) and is inserted on worker start. Discovery adds more as `company_candidates`:

- **Common Crawl:** the worker reads the URL index of the three newest crawls for board links on every supported platform in the background (Workday's huge pattern last). It pauses 3 seconds between pages, retries the often-overloaded index with long backoff, and saves progress per page in `discovery_progress`, so it resumes after restarts. Then it checks daily for a new crawl.
- **Links:** board links inside new job postings.
- **Manual:** one URL on Settings → Job sources (checked on the spot), or many at once there or with `./scripts/dev.sh discover <file>`.

When no company is due, the worker verifies candidates through the same rate-limited fetcher. Verifying reads a single page on paged platforms (Workday, SmartRecruiters). A board with open jobs becomes a company, named from Greenhouse's board API, the Lever or Ashby page title, or the company name in SmartRecruiters, Workable or Recruitee data; Workday and BambooHR use the token. A missing or empty board is retried after 30 days. Preferences filter at search time, so the list can be broad.

**Scope: US and remote.** `normalize::us_scope` keeps a job when any location is in the US or isn't recognized (plain "Remote" counts as in). It drops a job only when every location is clearly elsewhere. Out-of-scope jobs are stored as rows without descriptions (`in_scope = false`), so checks still recognize them; the feed, facets and counts skip them.

**How a check runs.**

1. The schedule lives in Postgres (`companies.next_check_at`), so a restart resumes it instead of refetching everything. The worker runs 6 check slots and claims one due company whenever a slot frees up, with `FOR UPDATE SKIP LOCKED` and a 60-minute lease, so several workers never check the same company twice and one slow board doesn't hold up the rest. Spare slots verify discovered boards. (This replaces a Redis Streams queue for ingestion; Redis Streams stays planned for on-demand AI work.)
2. The interval adapts to how often the company posts: 15 minutes if it posted in the last week, 6 hours if in the last month, otherwise daily, with ±10% jitter. Boards with more than 2,000 jobs wait at least 6 hours, since listing them alone takes hundreds of requests. Failures back off from 15 minutes up to a day. Redis holds a per-host request budget per second (Greenhouse 4, SmartRecruiters 4, Ashby 2, each Workday tenant and Recruitee company 2, Lever, Workable, BambooHR and others 1), shared by the worker and the API.
3. Each adapter returns either full jobs (Lever, Ashby, Workable, Recruitee) or a partial list of stubs with id, title and locations (Greenhouse, Workday, SmartRecruiters, BambooHR). For a partial list, new out-of-scope stubs are stored as they are, and details are fetched only for new jobs in the US or with unknown locations: the whole board in one request for Greenhouse, one request per job elsewhere, at most 1,500 per check. The rest wait for the next check, 15 minutes later.
4. Each job is normalized to one shape (workplace, job type, seniority from the title, salary from structured fields or the description), its HTML sanitized, and its content hashed. Jobs are written 500 per statement (`jsonb_to_recordset`); unchanged open jobs aren't rewritten. Every check is logged in `company_checks`.
5. **Rule changes reach stored jobs.** Each row records the `normalize_version` it was derived with, and whether its salary came from text or a structured field. When the rules in `normalize.rs` change, bump `NORMALIZE_VERSION`. On start, the worker re-derives seniority, scope and text-parsed salaries from the stored title, locations and description, without refetching. Salaries are read from ranges like "$120,000 - $150,000", "$120K to $150K" and "187,500 USD – 364,000 USD" (common on Workday).
6. Later: new jobs are embedded and queued for scoring.

**Job boards** (`feeds/`; status on Settings → Job boards). Himalayas (about 120,000 remote jobs), Remotive and Remote OK need no key; USAJOBS runs when `USAJOBS_API_KEY` and `USAJOBS_EMAIL` are set in `.env`. Boards are scheduled like companies (`feeds` table, same slots and lease): Himalayas every 30 minutes, Remote OK every 2 hours, Remotive and USAJOBS every 6.

- **Reading:** each check reads the newest pages until one has nothing new, then the next stretch of a full pass that resumes from a saved cursor. A 429 partway through ends that check early without counting as a failure, because progress is saved every page; the next check then waits 3 hours. Himalayas allows roughly 100 pages before refusing for hours, so its full pass takes days, while the newest pages, read first, keep new jobs arriving. Small boards fit in one page, so every check is a full pass.
- **Quality:** a listing is kept only if it's open to US workers (the board's country list, or the location rules), has a company, a title and https links, and doesn't come from a known reposting service.
- **Matching** (`company_key` and `title_key`, SQL functions with indexes): same company and title as an open job, with the same workplace or an overlapping location. A match on an employer's job (or another board's) adds only a `job_sources` row. If the employer is tracked but the job isn't there, the employer's board is trusted and nothing is added. Otherwise the listing becomes the board's own job (`jobs.feed`) under an inactive `external` company; when the employer's board is tracked later, the next check matches it and closes the board's copy. In November 2026, 49 of 79 Remote OK listings matched employer jobs we already had.
- **Closing:** when a full pass ends, listings it didn't see are removed, and the board's own jobs with no listing left close.
- **Credit:** the job page links each listing ("via Remote OK", "Also listed on Himalayas"), as Remotive and Remote OK require. Board links found in listings feed discovery.

**Feed at scale.** Feed queries use partial indexes over open, in-scope jobs and a `pg_trgm` index for substring title search. Filter facets (top 60 departments, top 200 companies) are cached in Redis for 5 minutes; other companies are searched through `GET /jobs/companies`.

**Stale and ghost jobs.**

- A job missing from its company's feed on two checks in a row is marked closed and hidden (one bad response can't close jobs). The row is kept, with `last_seen_at`, so saved jobs can show that they expired.
- Descriptions of jobs closed more than 90 days ago are cleared daily; title, dates and hash stay for repost detection.
- Salaries in currencies other than USD are shown but excluded from the USD salary filter and sort.
- Every job records `first_seen_at`, not just the posted date the company claims.
- A job matching one that closed earlier is flagged as a repost, with a count.
- Jobs open more than 60 days, or reposted more than twice, get a "possibly stale" badge, and a filter hides them.

## Onboarding and search preferences

What you want comes from your own answers; your resume only drives recommendations and tailoring.

1. **Questionnaire first.** Until it's finished or skipped, the feed sends you to `/onboarding`: six steps (roles and seniority, job type, workplace and places, work authorization, salary, companies and deal-breakers). Every question can be left blank, meaning "any". Editable any time at `/preferences` (linked from Profile).
2. **The feed uses those answers as default filters.** With no query in the URL, `/jobs` applies them (`fromPreferences` in `src/lib/filters.ts`); a "My preferences" chip shows when they're applied and brings them back after you change filters. "Clear filters" sets `all=1`, so an empty feed stays empty on purpose. How each answer filters:
   - **Titles:** any title matches when the job title has every one of its words, as whole words in any order ("Software Engineer" finds "Engineer, Software").
   - **Places:** `location` holds places separated by `;`. A state matches by name or code ("Austin, TX" finds "Austin, Texas" and "US-TX-Austin" but not "Portland, ME" for "Portland, OR"). With Remote picked, remote jobs match wherever they're based.
   - **Salary floor:** by the yearly maximum, optionally keeping jobs that list no salary.
   - **Deal-breakers:** title words (whole words, so "intern" doesn't hide "Internal") and exact company names.
   - **Work authorization:** needing sponsorship hides jobs flagged `no_sponsorship`; not being a citizen hides jobs flagged `citizens_only`. `normalize::restrictions` sets both flags from explicit wording in the description only ("unable to sponsor", "must be a U.S. citizen", "active Secret clearance"), so a job that says nothing is never hidden. "U.S. citizen or permanent resident" and export-license boilerplate don't count.
   - **Company size and industry** are saved but don't filter until company data is collected.
3. **Advanced filters stay visible** (titles, keywords, location, remote, salary, seniority, company, platform, posted date, stale, deal-breakers, work authorization), and any filter set can be saved as a named search.
4. **Resume upload, separately, powers recommendations.** The resume is read into a draft profile you review before saving (see Profile below). Within the jobs your answers allow, they drive the fit score and a "Recommended for you" sort, and supply the facts for tailoring and autofill. The resume never changes your answers.

## Matching and tailored documents

Matching runs in two passes so the AI only reads jobs worth reading.

1. **Filter and rank (every job).** Hard filters from preferences, then vector similarity between the job and your profile in pgvector, down to the top few hundred.
2. **Score (top jobs only).** `qwen3:8b` reads each shortlisted job against your profile and returns a 0 to 100 score with two or three reasons, as schema-checked JSON.

Tailoring, when you click "Tailor":

1. Pull the job's required and preferred skills and keywords.
2. Pick and reorder the most relevant profile entries, rewording bullets to the job's language only where truthful. Never invent experience.
3. Render a plain single-column, ATS-safe resume (Typst for PDF, plus DOCX).
4. Write a cover letter from the same facts plus the company description.
5. Show a keyword gap report: which keywords from the job are covered, and which are missing so you can decide whether to add them.

You review and edit every document before it's used.

## Autofill extension

The extension fills the form; you check it and click submit.

1. Detect the platform from the URL and page structure.
2. Read the job title and company and ask the API for the job and tailored resume (creating the job if new).
3. Fill fields: known platforms use a field map; unknown forms match labels to your profile, with the local model answering free-text questions.
4. Upload the tailored resume and highlight anything it couldn't fill.
5. On submit, log the application in the tracker.

The extension stores no personal data; it signs in to the API with a short-lived token. Full auto-apply could reuse the same pieces later.

## Local AI models

All AI runs free through Ollama, installed as the native Windows app so it uses the RTX 4060 Ti (8 GB) directly.

| Job | Model | Approx. size |
| --- | --- | --- |
| Matching (embeddings) | `qwen3-embedding:4b` | 2.5 GB |
| Parsing, scoring, tailoring, form answers | `qwen3:8b` | 5 to 6 GB at 4-bit |

- Lighter fallbacks: `qwen3-embedding:0.6b`, `qwen3:4b`. Stronger options need a 16 GB card: `qwen3-embedding:8b`, `qwen3:14b`.
- Ollama loads and unloads models as needed, so both don't need to fit at once. With 16 GB of system RAM, avoid heavy apps during big batches.
- Changing the embedding model means re-embedding all jobs; vectors from different models don't mix.
- Use Ollama's JSON-schema `format` option for anything parsed by code.
- Confirm exact model tags against the Ollama library when setting up. `./scripts/dev.sh ollama` checks the app is running and pulls both models; the model names live in `core::ollama`, and `/health` reports `ollama` and `missing_models` (Ollama being down doesn't mark the API degraded, since the job feed doesn't need it).

## Tech stack

Rust for the API and worker; TypeScript for the web app (Next.js, React) and extension (Manifest V3, Vite).

```text
yob/
  backend/
    crates/api/      Axum HTTP server (binary)
    crates/worker/   scheduled and queued jobs (binary)
    crates/core/     modules: jobs, profile, matching, documents, applications, auth
    crates/db/       SQLx queries and migrations
  frontend/          Next.js app
  extension/         Chrome extension
  docs/              this file
  docker-compose.yml
```

| Need | Rust crates | Notes |
| --- | --- | --- |
| Web server | `axum`, `tower-http` | Routing, CORS, logging, compression |
| Async runtime | `tokio` | |
| Database | `sqlx`, `pgvector` | Compile-time checked queries, migrations |
| Queue, cache, rate limits, sessions | `redis`, `deadpool-redis` | Redis Streams for the queue; sessions are a few Redis keys in `core::auth` rather than `tower-sessions`, whose Redis store brings a second Redis client |
| Schedule | `tokio-cron-scheduler` | Queues feed checks on each company's interval |
| JSON | `serde`, `serde_json` | |
| HTTP client | `reqwest` | Job feeds |
| HTML parsing | `scraper` | Cleaning descriptions, careers pages |
| Local AI | `ollama-rs` | Chat, structured output, embeddings |
| Reading resumes | `pdf-extract`, `docx-rs` | |
| Writing resumes | `typst`, `docx-rs` | ATS-safe PDFs |
| File storage | `object_store` | Local folder now, S3 later |
| Login | `argon2`, `axum-extra` cookies; `jsonwebtoken` later for extension tokens | Ready for multiple users later |
| Encryption | `aes-gcm` | Sensitive answers |
| Shared types | `utoipa` | OpenAPI spec, TS types generated from it |
| Config, errors, logs | `dotenvy`, `thiserror`, `anyhow`, `tracing` | |

Crate names are from memory; confirm and pin versions when adding them.

## Running it locally

Deployment is how the app is packaged and started; hosting is where it runs. For now both are your PC: `docker compose up`, then open `http://127.0.0.1:3000`.

| Service | Image or build | Port |
| --- | --- | --- |
| `frontend` | Next.js app | 3000 |
| `api` | Rust API binary | 8080 |
| `worker` | Rust worker binary | none |
| `db` | `pgvector/pgvector` | 5432 |
| `redis` | `redis` | 6379 |
| Ollama | Native Windows app, outside Docker; containers use `host.docker.internal:11434` | 11434 |

Ports bind to localhost only. Hosting later doesn't mean public sign-ups: the app controls who logs in, and sign-ups stay off unless turned on.

## Security and privacy

- Secrets live in a local `.env` that git ignores.
- Sensitive answers (demographics, visa status) are encrypted at the column level.
- The extension uses a short-lived token, talks only to your API, and requests page access only on application sites.
- Resume files are private and served through short-lived links.
- Nothing is sent to a job site until you submit, and no data leaves your machine for AI.

## Build order

1. **Architecture.** Done.
2. **Job feed.** Workspace and compose setup, Postgres and Redis, Greenhouse/Lever/Ashby adapters, adaptive polling, stale-job flags, search page with filters. Done when the feed shows fresh jobs from a few hundred companies.
3. **Profile and tailored documents.** Questionnaire, resume parsing, fit scoring, tailored resume and cover letter, keyword gap report.
4. **Autofill extension and tracker.** Greenhouse, Lever and Ashby forms first, then Workday; log every submission.
