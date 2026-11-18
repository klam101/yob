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
        v                                       │
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
| `search_preferences` | Questionnaire answers: target titles, seniority, locations, remote, salary floor, industries, deal-breakers, plus saved named searches | `user_id` |
| `profile_items` | Experience, education, projects, skills, each its own row with dates and bullets | `user_id` |
| `answers` | Reusable form answers: work authorization, sponsorship, start date, demographics (encrypted), links | `user_id` |
| `companies` | Name, careers site, platform (ATS), board id, polling interval, last checked | |
| `jobs` | Title, location, salary, description, posted date, first seen, closed at, repost count, apply URL, source, content hash, embedding | `company_id` |
| `job_matches` | Fit score and reasons for one user and one job | `user_id`, `job_id` |
| `documents` | A generated resume or cover letter: content, file path, version | `user_id`, `job_id` |
| `applications` | Status (saved, applied, interviewing, offer, rejected), dates, notes, documents sent | `user_id`, `job_id` |
| `tasks` | History of background jobs and results (the live queue is in Redis) | |

Every personal table carries `user_id` even though v1 has one user, so adding users later is a login change, not a schema rewrite.

## Job ingestion

Jobs come straight from the hiring systems companies use for their career pages, not from job boards. hiringcafe works the same way at a much larger scale (about 2.8 million listings from 46 platforms): nothing pushes new jobs to it; it re-checks career pages often.

**Adapters per platform, not per company.** One adapter covers every company on that platform.

| Order | Platform | How we read it |
| --- | --- | --- |
| 1 | Greenhouse | Official public job board API, one feed per company |
| 1 | Lever | Official public postings API |
| 1 | Ashby | Official public job board API |
| 2 | Workday | The JSON the company's careers page loads (unofficial) |
| 2 | Eightfold | The JSON the careers page loads (Microsoft, Starbucks, Qualcomm, PayPal are listed as customers) |
| 3 | iCIMS, SuccessFactors, Oracle, SmartRecruiters | Confirmed one at a time |
| never | LinkedIn, Indeed, hiringcafe | Terms forbid scraping |

Endpoints are from memory and get confirmed when each adapter is built.

**Finding companies.** Start from public lists of companies on Greenhouse, Lever and Ashby plus search results for their board URLs; grow whenever you save or paste a job from an unknown company. Preferences filter at search time, so the list can be broad.

**How a check runs.**

1. Each company has an adaptive interval: 15 minutes for companies that post often or that you've saved jobs from, 6 to 24 hours for quiet ones. Redis rate limits each host.
2. The worker fetches the company's job id list and compares it with the last one. Only new ids get a full description download.
3. Each job is normalized to one shape and hashed.
4. New jobs are embedded and queued for scoring.

**Stale and ghost jobs.**

- A job missing from its company's feed is marked closed and hidden right away.
- Every job records `first_seen_at`, not just the posted date the company claims.
- A job matching one that closed earlier is flagged as a repost, with a count.
- Jobs open more than 60 days, or reposted more than twice, get a "possibly stale" badge, and a filter hides them.

## Onboarding and search preferences

What you want comes from your own answers; your resume only drives recommendations and tailoring.

1. **Questionnaire first.** Input fields on first launch: titles, seniority (intern, entry, mid, senior, staff+), locations, remote or on-site, salary floor, work authorization and sponsorship, company size or industry, deal-breakers. Editable any time.
2. **The feed uses those answers as default filters.**
3. **Advanced filters stay visible** (title, location, remote, salary, seniority, company, platform, posted date, stale), and any filter set can be saved as a named search.
4. **Resume upload, separately, powers recommendations.** A local model turns the resume into `profile_items`. Within the jobs your answers allow, they drive the fit score and a "Recommended for you" sort, and supply the facts for tailoring and autofill. The resume never changes your answers.

## Matching and tailored documents

Matching runs in two passes so the AI only reads jobs worth reading.

1. **Filter and rank (every job).** Hard filters from preferences, then vector similarity between the job and your profile in pgvector, down to the top few hundred.
2. **Score (top jobs only).** `qwen3:8b` reads each shortlisted job against your profile and returns a 0 to 100 score with two or three reasons, as schema-checked JSON.

Tailoring, when you click "Tailor":

1. Pull the job's required and preferred skills and keywords.
2. Pick and reorder the most relevant `profile_items`, rewording bullets to the job's language only where truthful. Never invent experience.
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
- Confirm exact model tags against the Ollama library when setting up.

## Tech stack

Rust for the API and worker; TypeScript for the web app (Next.js, React) and extension (Manifest V3, Vite).

```text
yob/
  backend/
    crates/api/      Axum HTTP server (binary)
    crates/worker/   scheduled and queued jobs (binary)
    crates/core/     modules: jobs, profile, matching, documents, applications, auth
    crates/db/       SQLx queries and migrations
  web/               Next.js app
  extension/         Chrome extension
  docs/              this file
  docker-compose.yml
```

| Need | Rust crates | Notes |
| --- | --- | --- |
| Web server | `axum`, `tower-http` | Routing, CORS, logging, compression |
| Async runtime | `tokio` | |
| Database | `sqlx`, `pgvector` | Compile-time checked queries, migrations |
| Queue, cache, rate limits, sessions | `redis`, `deadpool-redis`, `tower-sessions-redis-store` | Redis Streams for the queue |
| Schedule | `tokio-cron-scheduler` | Queues feed checks on each company's interval |
| JSON | `serde`, `serde_json` | |
| HTTP client | `reqwest` | Job feeds |
| HTML parsing | `scraper` | Cleaning descriptions, careers pages |
| Local AI | `ollama-rs` | Chat, structured output, embeddings |
| Reading resumes | `pdf-extract`, `docx-rs` | |
| Writing resumes | `typst`, `docx-rs` | ATS-safe PDFs |
| File storage | `object_store` | Local folder now, S3 later |
| Login | `argon2`, `tower-sessions`, `jsonwebtoken` | Ready for multiple users later |
| Encryption | `aes-gcm` | Sensitive answers |
| Shared types | `utoipa` | OpenAPI spec, TS types generated from it |
| Config, errors, logs | `dotenvy`, `thiserror`, `anyhow`, `tracing` | |

Crate names are from memory; confirm and pin versions when adding them.

## Running it locally

Deployment is how the app is packaged and started; hosting is where it runs. For now both are your PC: `docker compose up`, then open `http://localhost:3000`.

| Service | Image or build | Port |
| --- | --- | --- |
| `web` | Next.js app | 3000 |
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
