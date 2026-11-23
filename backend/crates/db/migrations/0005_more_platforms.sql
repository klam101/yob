-- Five more hiring platforms. Workday board tokens are "tenant/wdN/site".
ALTER TABLE companies DROP CONSTRAINT companies_platform_check;
ALTER TABLE companies ADD CONSTRAINT companies_platform_check CHECK (
    platform IN ('greenhouse', 'lever', 'ashby', 'workday', 'smartrecruiters', 'workable',
                 'recruitee', 'bamboohr')
);
