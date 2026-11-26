-- Company facts from Wikidata (CC0), matched by company name
-- (`core::jobs::company_facts`), for the feed's Company filters: industry,
-- organization type, size, founding year and headquarters country. The
-- worker fills them for companies with open jobs and refreshes them monthly;
-- `facts_checked_at` is set even when nothing matched, so misses aren't
-- retried every week.

ALTER TABLE companies
    ADD COLUMN wikidata_id text,
    ADD COLUMN industries text[] NOT NULL DEFAULT '{}',
    ADD COLUMN org_type text,          -- public | private | nonprofit | government | cooperative
    ADD COLUMN employees integer,
    ADD COLUMN founded smallint,
    ADD COLUMN hq_country text,
    ADD COLUMN facts_checked_at timestamptz;

CREATE INDEX companies_industries ON companies USING gin (industries);
