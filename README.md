# yob

A personal, run-locally job search app: one feed of fresh jobs pulled straight from company hiring systems, ranked against your profile, with tailored resumes and cover letters and a Chrome extension that autofills applications.

Design: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). Progress: [docs/TODO.md](docs/TODO.md). Job sources roadmap: [docs/SOURCES.md](docs/SOURCES.md).

## Requirements

- Docker Desktop (WSL 2 backend)
- Rust (`winget install Rustlang.Rustup`) for local backend builds
- Node.js 24
- Ollama, the native Windows app (needed from the profile and matching features onward)

## Commands

Routine tasks run through one script in `scripts/`, from the repo root:

| Terminal | Run |
| --- | --- |
| Git Bash | `./scripts/dev.sh <command>` |
| PowerShell | `./scripts/dev.ps1 <command>` (forwards to `dev.sh` through Git Bash) |

Run it with no command to see the list. In PowerShell, don't use `bash ./scripts/dev.sh`: plain `bash` there starts WSL, which can't see the Windows Rust and Node installs.

First time:

```sh
./scripts/dev.sh setup   # checks tools, creates .env with a random DB password, installs frontend packages
./scripts/dev.sh up      # builds and starts everything in Docker
```

Then open http://127.0.0.1:3000 (`localhost` works too, but on Windows each new connection to it can stall about 2 seconds).

Optional: to add US federal jobs from USAJOBS, get a free key at https://developer.usajobs.gov/apirequest/ and set `USAJOBS_API_KEY` and `USAJOBS_EMAIL` in `.env` (see `.env.example`), then run `up` again. Everything else needs no keys.

| Command | What it does |
| --- | --- |
| `setup` | Checks Docker, Rust and Node; creates `.env` if missing; installs frontend packages |
| `ollama` | Checks the Ollama app is running and pulls the models yob uses (`qwen3-embedding:4b`, `qwen3:8b`). Needed from the profile features onward |
| `up` | Builds and starts the full stack in Docker, waits until healthy, prints the URLs |
| `down` | Stops everything. Database data is kept |
| `status` | Shows containers and the API health check |
| `logs [service]` | Follows logs for `api`, `worker`, `frontend`, `db`, `redis`, or all of them |
| `dev` | Day-to-day development: db and redis in Docker; the API, worker and web app run locally in one terminal with labeled output. The web app hot-reloads; after Rust changes, Ctrl+C and run it again. Ctrl+C stops all three |
| `stop` | Stops local API, worker and web processes left running, for example after closing the terminal during `dev` |
| `typegen` | Regenerates `frontend/src/api/schema.ts` from the running API. Run it after any API change |
| `parse-resume <file>` | Shows what the resume parser makes of a PDF, DOCX or text file: the cleaned text, the links found in it, the parsed profile and anything it couldn't place. `--raw` adds the text before cleanup |
| `check` | Rust format check, clippy, frontend lint and TypeScript check. Run before committing |
| `fmt` | Formats the Rust code |
| `psql` | Opens a Postgres shell |
| `redis` | Opens a Redis shell |
| `discover [file]` | Shows company discovery progress. With a file of job-board links (one per line), queues them to be checked and added. Needs the API running |
| `reset-password` | Sets a new password for the login account from a prompt, and logs out every session. For when you forget it |
| `reset-db` | Deletes all database data. Asks you to type `yes` first |

URLs: web app http://127.0.0.1:3000, API health http://127.0.0.1:8080/health, OpenAPI spec http://127.0.0.1:8080/openapi.json.

`dev` and `up` use the same ports, so `dev` stops the Docker api, worker and frontend containers first. Run `up` to switch back to the full Docker stack.

## Layout

| Path | What it is |
| --- | --- |
| `backend/crates/api` | Axum HTTP server (`api` binary) |
| `backend/crates/worker` | Background jobs (`worker` binary) |
| `backend/crates/core` | Modules: jobs, profile, matching, documents, applications, auth |
| `backend/crates/db` | Postgres pool and SQL migrations |
| `frontend/` | Next.js app |
| `scripts/` | `dev.sh` (all developer commands) and `dev.ps1` (PowerShell forwarder) |
| `docs/` | Architecture |

## How features get built

1. Define the API: Rust request and response types plus a stub endpoint, then `npm run typegen`.
2. Build the page against the stub.
3. Build the real backend: migration, core module logic, worker jobs.
4. Check it end to end with real data.
