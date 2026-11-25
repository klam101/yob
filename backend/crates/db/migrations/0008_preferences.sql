-- Search preferences from the onboarding questionnaire. They are the job
-- feed's default filters; the resume never changes them. Work authorization
-- answers live in `application_answers` (shared with autofill), not here.

CREATE TABLE search_preferences (
    user_id                 uuid PRIMARY KEY REFERENCES users ON DELETE CASCADE,
    titles                  text[] NOT NULL DEFAULT '{}',
    seniority               text[] NOT NULL DEFAULT '{}',
    employment_types        text[] NOT NULL DEFAULT '{}',
    workplaces              text[] NOT NULL DEFAULT '{}',
    locations               text[] NOT NULL DEFAULT '{}',
    -- Yearly, in USD.
    salary_min              integer,
    -- Keep jobs that don't list a salary when salary_min is set.
    include_unlisted_salary boolean NOT NULL DEFAULT true,
    company_sizes           text[] NOT NULL DEFAULT '{}',
    industries              text[] NOT NULL DEFAULT '{}',
    -- Deal-breakers.
    excluded_companies      text[] NOT NULL DEFAULT '{}',
    excluded_title_words    text[] NOT NULL DEFAULT '{}',
    -- NULL while the questionnaire was skipped; set when it's finished.
    completed_at            timestamptz,
    created_at              timestamptz NOT NULL DEFAULT now(),
    updated_at              timestamptz NOT NULL DEFAULT now()
);

-- Read from the description by `normalize::restrictions` (re-derived on the
-- next worker start, NORMALIZE_VERSION 7).
ALTER TABLE jobs
    ADD COLUMN no_sponsorship boolean NOT NULL DEFAULT false,
    ADD COLUMN citizens_only  boolean NOT NULL DEFAULT false;
