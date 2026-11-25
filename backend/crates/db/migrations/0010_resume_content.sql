-- A resume's own content (`core::profile::resume_content`), edited on its
-- page, and the marker for filling the profile from the first resume.

-- JSON of ResumeContent. NULL for resumes uploaded before this; they're read
-- again from their file the first time they're opened.
ALTER TABLE resumes
    ADD COLUMN content         jsonb,
    ADD COLUMN content_version integer NOT NULL DEFAULT 0;

-- The profile is filled automatically from a resume once: while it's empty
-- and this was never set. Later resumes ask first.
ALTER TABLE profiles
    ADD COLUMN filled_from_resume uuid REFERENCES resumes ON DELETE SET NULL,
    ADD COLUMN filled_at          timestamptz,
    -- Sections left out of that fill because what was read didn't pass
    -- validation (for example an email that isn't one).
    ADD COLUMN fill_skipped       text[] NOT NULL DEFAULT '{}',
    -- The user dismissed the "check what we read" banner.
    ADD COLUMN fill_reviewed      boolean NOT NULL DEFAULT false;
