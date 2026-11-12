//! Application layer: the canonical operations behind `vtest-cli` and
//! `vtest-mcp` (#175 D2).
//!
//! One operation is one function. Each returns typed data (or a pre-built
//! JSON envelope) and decides neither printing nor process exit; the
//! entrypoints only convert arguments in and results out. The CLI's `run_*`
//! wrappers and the MCP tool handlers call the same function here, so
//! DS-1563 (every MCP tool returns the same `data` / `diagnostics` as the
//! CLI's `--format json` for the same input) holds by construction rather
//! than by two independently written code paths.
//!
//! This crate also owns the adapter composition root
//! ([`registry::builtin_registry`]) and the shared envelope and exit-code
//! mapping ([`envelope`]).

pub mod approval;
pub mod audit;
pub mod doc;
pub mod doctor;
pub mod envelope;
pub mod form;
pub mod init;
pub mod judgment;
pub mod pending;
pub mod project;
pub mod registry;
pub mod report;
pub mod run;
pub mod scan;
pub mod test_op;
pub mod verify;
pub mod vo;
