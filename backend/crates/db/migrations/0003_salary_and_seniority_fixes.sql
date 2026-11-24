-- Salary filters and sorting compare yearly USD amounts. Jobs paid in other
-- currencies (₩, ¥, ₹…) would otherwise rank as huge USD salaries. They keep
-- their salary for display; they just don't take part in USD comparisons.
ALTER TABLE jobs DROP COLUMN salary_annual_max;
ALTER TABLE jobs ADD COLUMN salary_annual_max integer GENERATED ALWAYS AS (
    CASE
        WHEN salary_currency IS DISTINCT FROM 'USD' THEN NULL
        WHEN salary_period = 'hour' THEN salary_max * 2080
        ELSE salary_max
    END
) STORED;

-- One-time fix: engineering managers were classified as mid-level before the
-- seniority rule learned about them (see normalize::infer_seniority).
UPDATE jobs SET seniority = 'senior'
WHERE seniority IN ('entry', 'mid')
  AND title ~* '\mmanager\M'
  AND title ~* '\m(engineering|software|development|infrastructure)\M';
