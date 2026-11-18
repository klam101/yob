# yob: personal job search app

A single-user, run-locally job search app: one feed of fresh jobs pulled straight from company hiring systems, ranked against the user's profile, with tailored resumes and cover letters and a Chrome extension that autofills applications. Inspired by hiringcafe.com (search and filters) and jobright.ai (autofill extension). Do not scrape or reverse-engineer hiringcafe itself; copy the approach, not the data.

Full design: `docs/ARCHITECTURE.md`. Read it before large changes.

## Hard rules

- **No paid APIs.** All AI runs locally through Ollama. Never add OpenAI, Anthropic or other paid AI SDKs.
- **Single user now, multi-user ready.** Every personal table has `user_id`, and there is one login account from day one. Sign-ups stay off.
- **Local only.** Everything runs with `docker compose up` on Windows (Docker Desktop + WSL 2) and binds to localhost. No cloud hosting yet.
- **The user does git.** Don't create branches, commits, pushes or PRs unless asked.
- **Postgres is the source of truth.** Redis only holds data that can be rebuilt (queue, cache, rate limits, sessions).
- **Never invent resume content.** Tailoring may only rephrase facts from `profile_items`.
- **Don't scrape LinkedIn or Indeed.**

## Stack

| Part | Choice |
| --- | --- |
| Backend | Rust, one Cargo workspace in `backend/`: `crates/api` (Axum), `crates/worker`, `crates/core` (modules: jobs, profile, matching, documents, applications, auth), `crates/db` (SQLx + migrations) |
| Web app | TypeScript, Next.js + React, in `web/` |
| Extension | TypeScript, Chrome Manifest V3 built with Vite, in `extension/` |
| Database | Postgres with pgvector (`pgvector/pgvector` image) |
| Queue, cache, rate limits, sessions | Redis (Redis Streams for the job queue) |
| AI | Ollama, running as the native Windows app (not in Docker); containers reach it at `host.docker.internal:11434` |
| Embedding model | `qwen3-embedding:4b` |
| Text model | `qwen3:8b` (resume parsing, fit scoring, tailoring, form answers); use Ollama's JSON-schema `format` for structured output |
| Files | `object_store` crate, local `./data/files` folder now |
| Shared types | `utoipa` OpenAPI spec, TypeScript types generated from it for `web/` and `extension/` |

Main Rust crates: axum, tower-http, tokio, sqlx, pgvector, redis, deadpool-redis, tower-sessions(-redis-store), tokio-cron-scheduler, serde, reqwest, scraper, ollama-rs, pdf-extract, docx-rs, typst, object_store, argon2, jsonwebtoken, aes-gcm, utoipa, dotenvy, thiserror, anyhow, tracing. Crate names were chosen from memory; confirm and pin versions when adding them.

User hardware: Windows, RTX 4060 Ti 8 GB, 16 GB RAM, i7-8700. Keep model choices within 8 GB of VRAM.

## Product decisions

- **Preferences come from a questionnaire first** (input fields: titles, seniority, locations, remote, salary floor, work authorization, company size/industry, deal-breakers). They act as the feed's default filters.
- **hiringcafe-style advanced filters** are always visible and can be saved as named searches.
- **The resume is separate.** It only drives fit scores, "Recommended for you" ranking, tailoring and autofill. It never changes the questionnaire answers.
- **Ingestion pulls every open job** from tracked companies; preferences filter at search time.
- **Adapters are per platform, not per company.** Order: Greenhouse, Lever, Ashby (official public feeds), then Workday, Eightfold, iCIMS, SuccessFactors, Oracle (the JSON their careers pages load).
- **Adaptive polling.** Active or saved companies every 15 minutes, quiet ones every 6 to 24 hours. Diff job ids and fetch full descriptions only for new ids. Rate limit per host via Redis.
- **Stale jobs.** Missing from the feed means closed and hidden. Record `first_seen_at`, detect reposts, and give jobs open more than 60 days or reposted more than twice a "possibly stale" flag with a filter.
- **Autofill fills, the user submits.** Full auto-apply is out of scope for v1.

## Build order

1. Architecture (done; see `docs/ARCHITECTURE.md`)
2. Job feed: workspace and compose setup, Postgres + Redis, Greenhouse/Lever/Ashby adapters, search page with filters and ranking
3. Questionnaire, profile from resume, tailored resume and cover letter, keyword gap report
4. Autofill extension and application tracker
