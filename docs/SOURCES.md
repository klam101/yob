# Job sources: coverage roadmap

Goal: virtually every valid job posting in the **US or remote-friendly for US workers**, from credible sources, including small employers and companies without a major hiring platform. For scale, hiringcafe lists about 2.8 million jobs worldwide; US and remote is roughly 1 million.

Two rules apply to every source:

- **Read what's meant to be read.** Official APIs, the public JSON a careers page itself loads, structured data (schema.org JobPosting), and RSS feeds. Respect robots.txt and each source's terms, including attribution where required. Identify as `yob/0.1 (personal job search)` with no personal details.
- **No LinkedIn or Indeed scraping.** Their terms forbid it, they actively block it, and LinkedIn has sued scrapers. Their job APIs are partner-only. Most of their listings also exist on the employer's own ATS or careers page, which we read directly. For a job you find there yourself, the extension will offer "Save this job" for the page you're viewing (user-initiated, one page, no crawling).

Status: **live** (in the app), **verified** (endpoint probed and working, not built yet), **planned**. Probes were done in November 2026.

## Tier 1: ATS platforms

Structured, direct employer listings. One adapter per platform covers every company on it; the work is finding the companies (see Discovery).

| Platform | How we read it | Status |
| --- | --- | --- |
| Greenhouse | `boards-api.greenhouse.io/v1/boards/{token}/jobs` (light list; `?content=true` adds descriptions) | live |
| Lever | `api.lever.co/v0/postings/{company}?mode=json` (rate-limits quickly) | live |
| Ashby | `api.ashbyhq.com/posting-api/job-board/{name}?includeCompensation=true` | live |
| Workday | POST `{tenant}.wd{n}.myworkdayjobs.com/wday/cxs/{tenant}/{site}/jobs` (the JSON the careers page loads), then one detail request per new US job; big enterprises. Some tenants (NVIDIA, Target) list at most 2,000 jobs | live |
| SmartRecruiters | `api.smartrecruiters.com/v1/companies/{id}/postings` (official public API), detail per new US job | live |
| Workable | `apply.workable.com/api/v1/widget/accounts/{account}?details=true` (public widget API); many small companies | live |
| Recruitee | `{company}.recruitee.com/api/offers/` (public careers API) | live |
| BambooHR | `{company}.bamboohr.com/careers/list`, then `/careers/{id}/detail` (careers page JSON); many small companies | live |
| Breezy HR, JazzHR, Personio, Teamtailor, Jobvite, Pinpoint, Rippling | Public careers feeds; endpoints confirmed one at a time | planned |
| iCIMS, SuccessFactors, Oracle (Taleo), Eightfold, Dayforce, UKG | The JSON or HTML their careers pages load; harder, confirmed one at a time | planned |

## Tier 2: company career pages

For employers without a major ATS. Most career sites publish **schema.org `JobPosting` structured data** (JSON-LD), because Google for Jobs requires it. A generic crawler reads a company's careers page and sitemap, extracts `JobPosting` objects, and follows robots.txt.

- Coverage varies: JavaScript-rendered pages may have no JSON-LD in the HTML (a Greenhouse-hosted job page tested in November 2026 had none), so this complements the ATS adapters rather than replacing them.
- Discovery: careers pages linked from company sites, and **Web Data Commons**, which publishes the schema.org `JobPosting` entities it extracts from Common Crawl.
- Status: planned.

## Tier 3: job boards

Broad, candidate-facing listings. Only boards with official APIs or feeds, used within their terms.

| Board | Access | Notes | Status |
| --- | --- | --- | --- |
| Himalayas | Free API (`himalayas.app/jobs/api`), 20 per page with a cursor | About 120,000 remote jobs, about 57% open to US workers; links back to Himalayas, not the employer | live |
| Remotive | Free API (`remotive.com/api/remote-jobs`) | Only about 15 jobs, delayed 24 hours; at most 4 calls a day; credit and link back required | live |
| Remote OK | Free API (`remoteok.com/api`) | Latest ~100 jobs; credit and a followed link back required; some text double-encoded (repaired) | live |
| USAJOBS | Free API with a key (`data.usajobs.gov`) | All US federal jobs open to the public; needs the key holder's email as User-Agent. Built against the documented response, not yet run with a real key | live (needs key) |
| Arbeitnow | Free API (`arbeitnow.com/api/job-board-api`) | German board: its "remote" jobs are mostly Germany-only, so almost nothing is in scope | skipped |
| Adzuna | Free API with a key | Short snippets instead of descriptions, links through Adzuna's redirect, and a small daily call allowance | skipped for now |
| LinkedIn, Indeed | Not used | See the rules above | excluded |

Board listings are often duplicates of ATS jobs we already have, so they go through de-duplication (below). In November 2026, 49 of 79 in-scope Remote OK listings were jobs we already had from employers' boards.

## Tier 4: staffing agencies

Extra employer and contract listings. Large agencies (Robert Half, Randstad, Adecco, TEKsystems and others) mostly run on Workday, iCIMS or Bullhorn, or publish JSON-LD, so Tiers 1 and 2 cover them. Their jobs get a **"staffing agency"** tag so the feed can show or hide them. Postings often hide the real employer and duplicate each other, so de-duplication matters more here. Status: planned.

## Tier 5: niche job boards

Specialized and underserved markets: government (USAJOBS, state boards), higher education, nonprofits, healthcare, trades, remote-only boards. Use RSS or APIs where offered, otherwise JSON-LD (Tier 2). Added one at a time, by demand. Status: planned.

## Tier 6: other sources

Long-tail coverage.

- **Hacker News "Who is hiring"**: monthly thread via the Algolia HN API (verified; around 780 comments a month). Free-form text, so it needs AI parsing with the local model.
- **Board links inside other jobs**: Greenhouse, Lever, Ashby and other ATS links found in job descriptions or aggregator apply links become discovery candidates for free.

## Discovery: finding companies

Adapters are per platform, so coverage grows by finding more company boards.

- **Common Crawl index** (`index.commoncrawl.org`): search crawled URLs on ATS hosts (`jobs.lever.co/*`, `jobs.ashbyhq.com/*`, `boards.greenhouse.io/*`, `*.myworkdayjobs.com` and so on) and extract board names. One crawl's first 5,000 Lever URLs yielded 267 distinct boards. The service is often overloaded (504s), so discovery runs slowly in the background, resumes where it stopped, and retries.
- **Verification:** each candidate board is probed once through the rate-limited fetcher and added as a company only if it has open jobs. Rejected candidates are retried after 30 days.
- **Manual:** paste a board link on Settings → Job sources, paste many under Company discovery, or run `./scripts/dev.sh discover <file>`.
- **Status:** live for all eight live Tier 1 platforms (Common Crawl, links in postings, manual). Workday tenants have no public directory, so the first ones were found by probing well-known employers' careers URLs (November 2026: 22 of 30 guesses were real, from Capital One to CVS Health with 18,000 jobs). No ATS host publishes a sitemap of boards (checked November 2026).

## Cross-cutting

- **De-duplication and canonical source:** the employer's own ATS or careers page wins over aggregators. Boards rarely link straight to the employer (Himalayas, Remotive and Remote OK link to their own pages), so matching is by company + normalized title + workplace or location. Every board listing is kept as an attribution on the job it matched (`job_sources`). Live for job boards.
- **Scope at import:** keep jobs in the US or remote for US workers. Out-of-scope jobs are stored as small rows without descriptions, so incremental checks still recognize them.
- **Quality:** board jobs need a company, a title and https links, must be open to US workers, and are dropped when they come from a known reposting service (Lifelancer, Jobgether). The "possibly stale" and repost signals apply to every source.
- **Scale:** about 1 million US and remote jobs is roughly 8 to 10 GB in Postgres, plus about 10 GB once every job has an embedding. That's fine locally on an SSD; embeddings can be limited to jobs that pass the user's preference filters. Search uses Postgres full-text search and trigram indexes; a dedicated search engine is only needed if those get slow.

## Order

1. **Company discovery** for Greenhouse, Lever and Ashby, plus the scale work it needs: US scope, batch inserts, re-normalizing stored jobs, a faster feed at 200k+ jobs. (Done.)
2. **More ATS platforms:** Workday, SmartRecruiters, Workable, Recruitee, BambooHR, with discovery for each. (Done.)
3. **Job board APIs** with de-duplication: Himalayas, Remotive, Remote OK, USAJOBS. (Done; Arbeitnow and Adzuna skipped, see Tier 3.)
4. **Career-page crawler** (JSON-LD), with Web Data Commons discovery.
5. **Staffing tag, niche boards, HN "Who is hiring".**
