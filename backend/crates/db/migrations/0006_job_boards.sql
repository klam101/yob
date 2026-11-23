-- Job boards with public APIs (Himalayas, Remote OK, Remotive, USAJOBS).
-- Their listings are matched against jobs we already have from employers'
-- own boards; the employer's copy wins and the board is kept as an
-- attribution. Listings with no match become jobs of their own.

-- Employers known only from a job board. Never checked by the worker
-- (inactive); `board_token` holds the matching key of the name.
ALTER TABLE companies DROP CONSTRAINT companies_platform_check;
ALTER TABLE companies ADD CONSTRAINT companies_platform_check CHECK (
    platform IN ('greenhouse', 'lever', 'ashby', 'workday', 'smartrecruiters', 'workable',
                 'recruitee', 'bamboohr', 'external'));

-- One row per job board, scheduled like companies. Rows are added by the
-- worker on startup; `active` is false for boards that need an API key
-- that isn't set.
CREATE TABLE feeds (
    id                   text PRIMARY KEY,   -- himalayas | remotive | remoteok | usajobs
    active               boolean NOT NULL DEFAULT true,
    next_check_at        timestamptz NOT NULL DEFAULT now(),
    last_checked_at      timestamptz,
    last_status          text,               -- ok | error
    last_error           text,
    consecutive_failures integer NOT NULL DEFAULT 0,
    -- A full pass over a big board takes several checks; it resumes here.
    sweep_cursor         text,
    sweep_started_at     timestamptz,
    last_pass_at         timestamptz
);

-- Every place a job is listed on a job board, pointing at the job it was
-- matched to: an employer's job, the board's own job, or none (the
-- employer's board is tracked but doesn't have the job). Rows not seen
-- during a full pass over the board are deleted.
CREATE TABLE job_sources (
    feed          text NOT NULL REFERENCES feeds (id),
    external_id   text NOT NULL,
    job_id        uuid REFERENCES jobs (id) ON DELETE SET NULL,
    url           text NOT NULL,
    first_seen_at timestamptz NOT NULL DEFAULT now(),
    last_seen_at  timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (feed, external_id)
);
CREATE INDEX job_sources_job ON job_sources (job_id);

-- Set on jobs that only a job board lists (no employer copy).
ALTER TABLE jobs ADD COLUMN feed text REFERENCES feeds (id);
CREATE INDEX jobs_feed_open ON jobs (feed) WHERE feed IS NOT NULL AND closed_at IS NULL;

-- Matching keys. "Acme, Inc." and "ACME" → "acme"; "Senior Engineer
-- (Remote - US)" and "Senior Engineer" → "senior engineer".
CREATE FUNCTION company_key(name text) RETURNS text
LANGUAGE sql IMMUTABLE PARALLEL SAFE RETURNS NULL ON NULL INPUT
RETURN replace(
    regexp_replace(
        regexp_replace(lower(replace(name, '&', ' and ')), '[^a-z0-9]+', ' ', 'g'),
        '\m(the|inc|incorporated|llc|ltd|limited|corp|corporation|co|gmbh|plc|ag|sa|bv|pbc)\M',
        ' ', 'g'),
    ' ', '');

CREATE FUNCTION title_key(title text) RETURNS text
LANGUAGE sql IMMUTABLE PARALLEL SAFE RETURNS NULL ON NULL INPUT
RETURN btrim(regexp_replace(
    regexp_replace(lower(title),
        '\m(remote|hybrid|us|usa|united states|north america|americas|anywhere|worldwide)\M',
        ' ', 'g'),
    '[^a-z0-9]+', ' ', 'g'));

CREATE INDEX companies_key ON companies (company_key(name));
CREATE INDEX jobs_open_title_key ON jobs (company_id, title_key(title)) WHERE closed_at IS NULL;
