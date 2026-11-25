-- Fields the resume parser and the autofill extension need, plus resume
-- details for the Resume page.

ALTER TABLE profiles
    ADD COLUMN middle_name text NOT NULL DEFAULT '',
    -- "mobile", "home" or "work"; empty when not given.
    ADD COLUMN phone_type  text NOT NULL DEFAULT '';

ALTER TABLE educations ADD COLUMN minor text NOT NULL DEFAULT '';

-- A one-line description under the project name ("Online banking platform").
ALTER TABLE projects ADD COLUMN summary text NOT NULL DEFAULT '';

ALTER TABLE resumes
    -- Shown in the list; starts as the file name without its extension.
    ADD COLUMN name         text NOT NULL DEFAULT '',
    ADD COLUMN target_title text NOT NULL DEFAULT '',
    ADD COLUMN is_primary   boolean NOT NULL DEFAULT false,
    ADD COLUMN updated_at   timestamptz NOT NULL DEFAULT now(),
    -- Link targets found in the file (PDF link annotations, DOCX hyperlinks),
    -- so the profile can be filled from it again later.
    ADD COLUMN links        text[] NOT NULL DEFAULT '{}';

UPDATE resumes SET name = regexp_replace(filename, '\.[A-Za-z0-9]+$', ''), updated_at = uploaded_at;

-- Each user's newest resume starts as their primary one.
UPDATE resumes r SET is_primary = true
WHERE id = (SELECT id FROM resumes n WHERE n.user_id = r.user_id ORDER BY uploaded_at DESC LIMIT 1);

CREATE UNIQUE INDEX resumes_one_primary ON resumes (user_id) WHERE is_primary;
