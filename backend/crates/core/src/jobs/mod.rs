//! Companies, job boards (one adapter per hiring platform), normalized jobs,
//! and closed and stale detection.
//!
//! Data flow: the worker checks companies on a schedule stored in Postgres
//! (`ingest`), adapters read each platform's public job board (`sources`),
//! and the API only ever reads the stored jobs (`store`).

pub mod http;
pub mod ingest;
pub mod model;
pub mod normalize;
pub mod seed;
pub mod sources;
pub mod store;

pub use http::{FetchError, Fetcher};
pub use model::{EmploymentType, NormalizedJob, Platform, SalaryPeriod, Seniority, Workplace};
