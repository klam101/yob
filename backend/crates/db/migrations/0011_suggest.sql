-- Type-ahead suggestions for fields that grow with the job data: job titles
-- and companies. The worker rebuilds them from `jobs` and `companies`
-- (`core::suggest::rebuild`); everything here can be thrown away and rebuilt.
-- Cities and schools come from fixed lists shipped with the app and are
-- searched in memory, so they aren't stored here.
CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE TABLE suggest_terms (
    kind   text    NOT NULL,           -- 'title' | 'company'
    value  text    NOT NULL,           -- as shown: "Software Engineer"
    -- `value` folded for search, with a leading space so "word starts"
    -- are ' ' || word: " software engineer".
    search text    NOT NULL,
    weight integer NOT NULL DEFAULT 0, -- how common: open jobs with it
    PRIMARY KEY (kind, search)
);

CREATE INDEX suggest_terms_search_trgm ON suggest_terms USING gin (search gin_trgm_ops);
