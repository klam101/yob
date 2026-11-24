-- The profile: one table per section. Lists (experiences, educations, ...)
-- are one row per entry, ordered by `position`, so a profile with five jobs
-- is five rows in `experiences`. Bullets are an ordered text[] inside a row.
-- Dates are stored as the first day of their month.

CREATE TABLE profiles (
    user_id     uuid PRIMARY KEY REFERENCES users ON DELETE CASCADE,
    first_name  text NOT NULL DEFAULT '',
    last_name   text NOT NULL DEFAULT '',
    email       text NOT NULL DEFAULT '',
    phone       text NOT NULL DEFAULT '',
    street      text NOT NULL DEFAULT '',
    city        text NOT NULL DEFAULT '',
    region      text NOT NULL DEFAULT '',
    postal_code text NOT NULL DEFAULT '',
    country     text NOT NULL DEFAULT '',
    headline    text NOT NULL DEFAULT '',
    summary     text NOT NULL DEFAULT '',
    updated_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE profile_links (
    id       uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id  uuid NOT NULL REFERENCES users ON DELETE CASCADE,
    position int NOT NULL,
    kind     text NOT NULL CHECK (kind IN ('linkedin', 'github', 'portfolio', 'other')),
    url      text NOT NULL
);
CREATE INDEX profile_links_user ON profile_links (user_id, position);

CREATE TABLE educations (
    id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id        uuid NOT NULL REFERENCES users ON DELETE CASCADE,
    position       int NOT NULL,
    school         text NOT NULL,
    degree         text NOT NULL DEFAULT '',
    field_of_study text NOT NULL DEFAULT '',
    gpa            text NOT NULL DEFAULT '',
    location       text NOT NULL DEFAULT '',
    start_date     date,
    end_date       date,
    details        text NOT NULL DEFAULT ''
);
CREATE INDEX educations_user ON educations (user_id, position);

CREATE TABLE experiences (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         uuid NOT NULL REFERENCES users ON DELETE CASCADE,
    position        int NOT NULL,
    company         text NOT NULL,
    title           text NOT NULL,
    location        text NOT NULL DEFAULT '',
    employment_type text NOT NULL DEFAULT '',
    start_date      date,
    -- NULL with a start date means "Present".
    end_date        date,
    bullets         text[] NOT NULL DEFAULT '{}'
);
CREATE INDEX experiences_user ON experiences (user_id, position);

CREATE TABLE projects (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    uuid NOT NULL REFERENCES users ON DELETE CASCADE,
    position   int NOT NULL,
    name       text NOT NULL,
    role       text NOT NULL DEFAULT '',
    url        text NOT NULL DEFAULT '',
    start_date date,
    end_date   date,
    bullets    text[] NOT NULL DEFAULT '{}',
    skills     text[] NOT NULL DEFAULT '{}'
);
CREATE INDEX projects_user ON projects (user_id, position);

CREATE TABLE skills (
    id       uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id  uuid NOT NULL REFERENCES users ON DELETE CASCADE,
    position int NOT NULL,
    name     text NOT NULL,
    category text NOT NULL DEFAULT ''
);
CREATE UNIQUE INDEX skills_user_name ON skills (user_id, lower(name));

-- Answers application forms ask for. Work authorization is plain; the
-- equal-employment answers are one AES-GCM encrypted JSON blob (key from
-- PROFILE_ENCRYPTION_KEY), only decrypted for the profile page and autofill.
CREATE TABLE application_answers (
    user_id             uuid PRIMARY KEY REFERENCES users ON DELETE CASCADE,
    work_countries      text[] NOT NULL DEFAULT '{}',
    us_citizen          boolean,
    authorized_us       boolean,
    needs_sponsorship   boolean,
    authorization_basis text NOT NULL DEFAULT '',
    eeo_nonce           bytea,
    eeo_ciphertext      bytea,
    updated_at          timestamptz NOT NULL DEFAULT now()
);

-- Uploaded resumes. The file lives in local file storage under `file_key`.
CREATE TABLE resumes (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      uuid NOT NULL REFERENCES users ON DELETE CASCADE,
    file_key     text NOT NULL,
    filename     text NOT NULL,
    content_type text NOT NULL,
    size_bytes   bigint NOT NULL,
    text         text NOT NULL DEFAULT '',
    uploaded_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX resumes_user ON resumes (user_id, uploaded_at DESC);
