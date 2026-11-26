-- Details read from each job's title and description by rules
-- (`core::jobs::extract`), for the feed's filters. The worker fills them for
-- stored jobs on its next start (NORMALIZE_VERSION 9).
--
-- tags: namespaced labels such as 'edu:bachelors:required', 'perk:401k_match',
--   'clearance:secret', 'skill:rust', 'shift:night', 'role:manager'.
-- min_years: the smallest "N+ years of experience" the posting asks for.
-- job_function: a slug from the fixed taxonomy in `core::jobs::functions`.

ALTER TABLE jobs
    ADD COLUMN tags text[] NOT NULL DEFAULT '{}',
    ADD COLUMN min_years smallint,
    ADD COLUMN job_function text;

CREATE INDEX jobs_feed_tags ON jobs USING gin (tags) WHERE closed_at IS NULL AND in_scope;
CREATE INDEX jobs_feed_function ON jobs (job_function) WHERE closed_at IS NULL AND in_scope;
CREATE INDEX jobs_feed_min_years ON jobs (min_years) WHERE closed_at IS NULL AND in_scope;
