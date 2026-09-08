//! Canonical operation functions shared between the CLI's argument-parsing
//! wrappers and (in a future PR) an MCP server. Each function here returns
//! typed data and performs no printing / exit-code decision — that stays in
//! `lib.rs`'s `run_*` wrappers, mirroring the existing (pre-`ops`)
//! `run_verify`/`run_scan`/`run_doctor` shape as closely as this slice's
//! budget allowed. See the closure-slice task note for the acknowledged
//! asymmetry: `verify`/`scan`/`doctor`/`init` were not refactored to match.

pub mod doctor;
pub mod init;
pub mod run;
pub mod scan;
pub mod verify;
