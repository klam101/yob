//! Companies, job boards (one adapter per hiring platform), normalized jobs,
//! and closed and stale detection.
//!
//! Data flow: the worker checks companies on a schedule stored in Postgres
//! (`ingest`), adapters read each platform's public job board (`sources`),
//! job boards with public APIs are read and de-duplicated against them
//! (`feeds`), and the API only ever reads the stored jobs (`store`).

pub mod discover;
pub mod feeds;
pub mod http;
pub mod ingest;
pub mod model;
pub mod normalize;
pub mod search;
pub mod seed;
pub mod sources;
pub mod store;

pub use http::{FetchError, Fetcher};
pub use model::{
    EmploymentType, Feed, NormalizedJob, Platform, SalaryPeriod, Seniority, Source, Workplace,
};
