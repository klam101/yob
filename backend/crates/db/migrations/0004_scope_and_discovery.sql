-- US + remote scope, re-normalizing stored jobs, company discovery, and
-- indexes for a feed with hundreds of thousands of jobs.

-- Jobs outside the US (and not remote-friendly for US workers) are kept as
-- small rows without descriptions, so incremental checks still recognize
-- them, but they never show in the feed.
ALTER TABLE jobs ADD COLUMN in_scope boolean NOT NULL DEFAULT true;

-- Which version of the rules in normalize.rs produced the derived fields.
-- The worker re-derives rows with an older version on startup.
ALTER TABLE jobs ADD COLUMN normalize_version integer NOT NULL DEFAULT 1;

-- Where the salary came from: 'structured' (a platform field, with a
-- currency) or 'text' (parsed from the description, re-derived when the
-- rules change). Existing rows: Greenhouse has no salary field, so its
-- salaries were parsed from text. Lever and Ashby rows are marked structured;
-- the few parsed from their text keep their value until they change.
ALTER TABLE jobs ADD COLUMN salary_source text;
UPDATE jobs j SET salary_source = CASE WHEN c.platform = 'greenhouse' THEN 'text' ELSE 'structured' END
FROM companies c
WHERE c.id = j.company_id AND j.salary_max IS NOT NULL;

-- The feed: open, in-scope jobs, newest first.
DROP INDEX jobs_open_posted;
CREATE INDEX jobs_feed_posted ON jobs (posted_at DESC) WHERE closed_at IS NULL AND in_scope;
CREATE INDEX jobs_feed_salary ON jobs (salary_annual_max DESC NULLS LAST)
    WHERE closed_at IS NULL AND in_scope;
CREATE INDEX jobs_renormalize ON jobs (normalize_version);

-- Substring search on titles and company names (ILIKE '%…%').
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE INDEX jobs_title_trgm ON jobs USING gin (title gin_trgm_ops);
CREATE INDEX companies_name_trgm ON companies USING gin (name gin_trgm_ops);
CREATE INDEX companies_name_lower ON companies (lower(name));

-- Job boards found by discovery, waiting to be verified. A board with open
-- jobs becomes a company; an empty or missing one is retried after 30 days.
CREATE TABLE company_candidates (
    id            bigserial PRIMARY KEY,
    platform      text NOT NULL,
    token         text NOT NULL,
    source        text NOT NULL,                   -- commoncrawl | link | manual
    status        text NOT NULL DEFAULT 'pending', -- pending | added | rejected
    found_at      timestamptz NOT NULL DEFAULT now(),
    checked_at    timestamptz,
    next_check_at timestamptz NOT NULL DEFAULT now(),
    jobs_found    integer,
    error         text,
    company_id    uuid REFERENCES companies (id) ON DELETE SET NULL
);

CREATE UNIQUE INDEX company_candidates_board ON company_candidates (platform, lower(token));
CREATE INDEX company_candidates_due ON company_candidates (next_check_at)
    WHERE status IN ('pending', 'rejected');

-- How far discovery got through each Common Crawl index, so it resumes
-- where it stopped.
CREATE TABLE discovery_progress (
    crawl      text NOT NULL,
    pattern    text NOT NULL,
    pages      integer,           -- null until the page count is known
    next_page  integer NOT NULL DEFAULT 0,
    found      integer NOT NULL DEFAULT 0,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (crawl, pattern)
);
