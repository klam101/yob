#!/usr/bin/env bash
# yob developer commands. Run from Git Bash (or any bash): ./scripts/dev.sh <command>
# Run ./scripts/dev.sh help for the list. Every command works from any directory.

set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

CONCURRENTLY="concurrently@10.0.7"
API_URL="http://localhost:8080"
WEB_URL="http://localhost:3000"

usage() {
  cat <<'EOF'
yob developer commands

Usage: ./scripts/dev.sh <command> [args]     (Git Bash)
       ./scripts/dev.ps1 <command> [args]    (PowerShell)

First time
  setup            Check tools, create .env (random DB password), install frontend packages

Full stack in Docker
  up               Build and start everything, wait until healthy, print URLs
  down             Stop everything (database data is kept)
  status           Show containers and API health
  logs [service]   Follow logs (services: api, worker, frontend, db, redis)

Local development (hot reload for the web app)
  dev              Run db and redis in Docker; run the API, worker and web app
                   locally in this terminal. Ctrl+C stops all three.
                   After Rust changes, Ctrl+C and run it again.
  stop             Stop local API, worker and web processes left running
                   (for example after closing the terminal during dev)

Code
  typegen          Regenerate frontend/src/api/schema.ts from the running API
  check            Format check, clippy, frontend lint, TypeScript check
  fmt              Format the Rust code

Data
  psql             Open a Postgres shell
  redis            Open a Redis shell
  discover [file]  Show company discovery progress; with a file of job-board
                   links (one per line), queue them to be checked and added
  reset-db         Delete ALL database and volume data (asks first)
EOF
}

# --- helpers ---------------------------------------------------------------

if [ -t 1 ]; then BOLD=$'\e[1m'; RED=$'\e[31m'; GREEN=$'\e[32m'; RESET=$'\e[0m'
else BOLD=""; RED=""; GREEN=""; RESET=""; fi

say()  { echo "${BOLD}==>${RESET} $*"; }
ok()   { echo "${GREEN}$*${RESET}"; }
fail() { echo "${RED}error:${RESET} $*" >&2; exit 1; }

need() {
  command -v "$1" >/dev/null 2>&1 || fail "$1 is not installed or not on PATH. $2"
}

need_env() {
  [ -f .env ] || fail ".env is missing. Run ./scripts/dev.sh setup first."
}

# Prints the API's /health body, or fails quietly if it isn't reachable.
api_health() {
  curl -fsS --max-time 3 "$API_URL/health" 2>/dev/null
}

# Interactive docker exec needs winpty in the standalone Git Bash window
# (mintty). VS Code's terminal doesn't.
docker_tty() {
  if [ "${TERM_PROGRAM:-}" = "mintty" ] && command -v winpty >/dev/null 2>&1; then
    winpty docker "$@"
  else
    docker "$@"
  fi
}

# --- commands --------------------------------------------------------------

cmd_setup() {
  need docker "Install Docker Desktop."
  need cargo  "Install Rust: winget install Rustlang.Rustup"
  need node   "Install Node.js 24."
  need npm    "Install Node.js 24."
  docker info >/dev/null 2>&1 || fail "Docker isn't running. Start Docker Desktop."

  if [ -f .env ]; then
    say ".env already exists, leaving it alone"
  else
    local pw
    pw=$(node -e 'console.log(require("crypto").randomBytes(18).toString("base64url"))')
    sed "s/change-me/$pw/g" .env.example > .env
    say "Created .env with a random database password"
  fi

  say "Installing frontend packages"
  (cd frontend && npm ci)
  ok "Setup done. Next: ./scripts/dev.sh up  (or ./scripts/dev.sh dev)"
}

cmd_up() {
  need_env
  say "Building and starting all services"
  docker compose up -d --build --wait --remove-orphans
  cmd_status
  echo
  ok "Web app:  $WEB_URL"
  ok "API:      $API_URL/health"
  ok "OpenAPI:  $API_URL/openapi.json"
}

cmd_down() {
  docker compose down
}

cmd_status() {
  docker compose ps --format 'table {{.Service}}\t{{.Status}}\t{{.Ports}}'
  echo
  if body=$(api_health); then
    ok "API health: $body"
  else
    echo "API health: not reachable at $API_URL"
  fi
}

cmd_logs() {
  docker compose logs -f --tail 100 "$@"
}

# Stops processes left over from `dev`: anything run from backend\target, the
# Next dev server for frontend\, and the npm and cargo processes that started them.
# Ctrl+C normally stops them, but a closed terminal or crash can leave them
# holding ports 8080 and 3000.
stop_local() {
  command -v cygpath >/dev/null 2>&1 || return 0   # Windows (Git Bash) only
  local root
  root=$(cygpath -w "$PWD")
  YOB_ROOT="$root" powershell.exe -NoProfile -Command '
    $root = $env:YOB_ROOT
    Get-CimInstance Win32_Process | Where-Object {
      ($_.ExecutablePath -and $_.ExecutablePath.StartsWith("$root\backend\target\", "OrdinalIgnoreCase")) -or
      ($_.Name -eq "node.exe" -and ($_.CommandLine -like "*$root\frontend\*" -or $_.CommandLine -like "*--prefix frontend run dev*")) -or
      ($_.Name -eq "cargo.exe" -and $_.CommandLine -like "*--manifest-path backend/Cargo.toml*")
    } | ForEach-Object {
      Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue
      "stopped leftover $($_.Name) (pid $($_.ProcessId))"
    }'
}

cmd_dev() {
  need_env
  need cargo "Install Rust: winget install Rustlang.Rustup"
  need npm   "Install Node.js 24."

  stop_local
  # Clean up on any exit: Ctrl+C, closing the terminal, or a crash.
  trap stop_local EXIT
  trap 'exit 130' INT TERM HUP

  say "Stopping the Docker api, worker and frontend containers (they hold ports 8080 and 3000)"
  docker compose stop api worker frontend

  say "Starting db and redis"
  docker compose up -d --wait db redis

  # Build once up front so the two cargo runs below don't wait on each other.
  say "Building the API and worker"
  cargo build --manifest-path backend/Cargo.toml --bin api --bin worker

  say "Starting api, worker and frontend (Ctrl+C stops all three)"
  # Run from the repo root so the binaries find .env. concurrently stops the
  # whole process tree on Ctrl+C, which plain background jobs don't on Windows.
  npx --yes "$CONCURRENTLY" \
    --names api,worker,frontend \
    --prefix-colors blue,magenta,green \
    --kill-others-on-fail \
    "cargo run --manifest-path backend/Cargo.toml --bin api" \
    "cargo run --manifest-path backend/Cargo.toml --bin worker" \
    "npm --prefix frontend run dev" &
  # Wait in the background so bash can react to signals and run the cleanup;
  # a foreground command would delay the traps until it exited by itself.
  wait $!
}

cmd_typegen() {
  api_health >/dev/null || fail "The API isn't running at $API_URL. Start it with ./scripts/dev.sh up or ./scripts/dev.sh dev, then try again."
  (cd frontend && npm run typegen)
}

cmd_check() {
  say "cargo fmt --check"
  (cd backend && cargo fmt --check)
  say "cargo clippy"
  (cd backend && cargo clippy --all-targets -- -D warnings)
  say "frontend lint"
  (cd frontend && npm run lint)
  say "frontend TypeScript check"
  (cd frontend && npx tsc --noEmit)
  ok "All checks passed"
}

cmd_discover() {
  api_health >/dev/null || fail "The API isn't running at $API_URL. Start it with ./scripts/dev.sh up or ./scripts/dev.sh dev, then try again."
  if [ -n "${1:-}" ]; then
    [ -f "$1" ] || fail "No such file: $1"
    say "Queueing job-board links from $1"
  fi
  # Node is already required for the frontend; it reads the file and calls the API.
  API_URL="$API_URL" node - "${1:-}" <<'JS'
const fs = require("node:fs");
const [file] = process.argv.slice(2);
const api = process.env.API_URL;
(async () => {
  if (file) {
    const urls = fs.readFileSync(file, "utf8").split(/[\s,]+/).filter(Boolean);
    const res = await fetch(`${api}/discovery/candidates`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ urls }),
    });
    const body = await res.json();
    if (!res.ok) throw new Error(body.message ?? `HTTP ${res.status}`);
    console.log(`Queued ${body.queued} new boards (${body.recognized} recognized, ${body.unrecognized.length} not job boards).`);
    for (const u of body.unrecognized.slice(0, 10)) console.log(`  not a board: ${u}`);
  }
  const s = await (await fetch(`${api}/discovery`)).json();
  const index = s.index_pages_total == null ? `${s.index_pages_read} pages read` : `${s.index_pages_read} of ${s.index_pages_total} pages read`;
  console.log(`Discovery: ${s.pending} waiting to check, ${s.added} added (${s.added_last_week} this week), ${s.rejected} not added.`);
  console.log(`Common Crawl: ${index}.`);
})().catch((e) => { console.error(`error: ${e.message}`); process.exit(1); });
JS
}

cmd_fmt() {
  (cd backend && cargo fmt)
  ok "Rust code formatted"
}

cmd_psql() {
  # Uses the container's own POSTGRES_USER and POSTGRES_DB.
  docker_tty compose exec db sh -c 'psql -U "$POSTGRES_USER" -d "$POSTGRES_DB"'
}

cmd_redis() {
  docker_tty compose exec redis redis-cli
}

cmd_reset_db() {
  echo "${RED}This deletes ALL database data (jobs, profile, documents) and stops every service.${RESET}"
  read -r -p "Type yes to continue: " answer
  [ "$answer" = "yes" ] || { echo "Cancelled."; exit 0; }
  docker compose down -v
  ok "Data deleted. Run ./scripts/dev.sh up to start fresh."
}

# --- dispatch --------------------------------------------------------------

command="${1:-help}"
shift || true

case "$command" in
  help|-h|--help) usage ;;
  setup)    cmd_setup ;;
  up)       cmd_up ;;
  down)     cmd_down ;;
  status)   cmd_status ;;
  logs)     cmd_logs "$@" ;;
  dev)      cmd_dev ;;
  stop)     stop_local; ok "No local dev processes left running" ;;
  typegen)  cmd_typegen ;;
  check)    cmd_check ;;
  fmt)      cmd_fmt ;;
  discover) cmd_discover "$@" ;;
  psql)     cmd_psql ;;
  redis)    cmd_redis ;;
  reset-db) cmd_reset_db ;;
  *)
    echo "Unknown command: $command" >&2
    echo >&2
    usage >&2
    exit 1
    ;;
esac
