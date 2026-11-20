-- Companies we track and the jobs read from their public job boards.
-- Jobs are never deleted: a job missing from its feed is closed (closed_at)
-- after two checks in a row, and old closed jobs lose their descriptions.

CREATE TABLE companies (
    id                   uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    name                 text NOT NULL,
    platform             text NOT NULL CHECK (platform IN ('greenhouse', 'lever', 'ashby')),
    board_token          text NOT NULL,
    careers_url          text NOT NULL,
    domain               text,
    active               boolean NOT NULL DEFAULT true,
    -- When the worker should check this company next. Stored here so a
    -- restart resumes the schedule instead of refetching everything.
    next_check_at        timestamptz NOT NULL DEFAULT now(),
    check_interval_min   integer NOT NULL DEFAULT 360,
    last_checked_at      timestamptz,
    last_status          text,   -- ok | error
    last_error           text,
    consecutive_failures integer NOT NULL DEFAULT 0,
    created_at           timestamptz NOT NULL DEFAULT now(),
    UNIQUE (platform, board_token)
);

CREATE INDEX companies_due ON companies (next_check_at) WHERE active;

CREATE TABLE jobs (
    id               uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    company_id       uuid NOT NULL REFERENCES companies (id) ON DELETE CASCADE,
    source_job_id    text NOT NULL,
    title            text NOT NULL,
    department       text,
    locations        text[] NOT NULL DEFAULT '{}',
    workplace        text NOT NULL,          -- onsite | hybrid | remote
    employment_type  text NOT NULL,          -- full_time | part_time | contract | internship
    seniority        text NOT NULL,          -- intern | entry | mid | senior | staff_plus
    salary_min       integer,
    salary_max       integer,
    salary_currency  text,
    salary_period    text,                   -- year | hour
    -- Yearly equivalent of salary_max, for filtering and sorting.
    salary_annual_max integer GENERATED ALWAYS AS (
        CASE WHEN salary_period = 'hour' THEN salary_max * 2080 ELSE salary_max END
    ) STORED,
    posted_at        timestamptz NOT NULL,   -- what the company claims
    first_seen_at    timestamptz NOT NULL DEFAULT now(),
    last_seen_at     timestamptz NOT NULL DEFAULT now(),
    closed_at        timestamptz,
    missed_checks    integer NOT NULL DEFAULT 0,
    repost_count     integer NOT NULL DEFAULT 0,
    content_hash     text NOT NULL,
    apply_url        text NOT NULL,
    description_html text,
    description_text text,
    -- Keyword search over title and department (company name is matched separately).
    search           tsvector GENERATED ALWAYS AS (
        to_tsvector('english', coalesce(title, '') || ' ' || coalesce(department, ''))
    ) STORED,
    UNIQUE (company_id, source_job_id)
);

CREATE INDEX jobs_open_recent ON jobs (first_seen_at DESC) WHERE closed_at IS NULL;
CREATE INDEX jobs_open_posted ON jobs (posted_at DESC) WHERE closed_at IS NULL;
CREATE INDEX jobs_company ON jobs (company_id);
CREATE INDEX jobs_hash ON jobs (company_id, content_hash);
CREATE INDEX jobs_search ON jobs USING gin (search);

-- History of each company check (the architecture's `tasks` table, scoped
-- to ingestion). Useful for the Job sources page and debugging.
CREATE TABLE company_checks (
    id          bigserial PRIMARY KEY,
    company_id  uuid NOT NULL REFERENCES companies (id) ON DELETE CASCADE,
    started_at  timestamptz NOT NULL,
    finished_at timestamptz NOT NULL DEFAULT now(),
    status      text NOT NULL,               -- ok | error
    jobs_seen   integer NOT NULL DEFAULT 0,
    jobs_new    integer NOT NULL DEFAULT 0,
    jobs_closed integer NOT NULL DEFAULT 0,
    error       text
);

CREATE INDEX company_checks_recent ON company_checks (company_id, started_at DESC);
