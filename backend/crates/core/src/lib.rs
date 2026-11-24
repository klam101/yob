//! Business logic for yob, split into modules that each own their tables.
//! The API and worker binaries call into these modules; modules talk to each
//! other only through their public functions.

pub mod config;
pub mod ollama;
pub mod shutdown;
pub mod telemetry;

pub mod applications;
pub mod auth;
pub mod documents;
pub mod jobs;
pub mod matching;
pub mod profile;
