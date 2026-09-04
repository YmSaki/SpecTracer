//! Shared, domain-neutral model types for vtest.
//!
//! This crate owns values that cross crate or CLI boundaries.  Filesystem
//! access and derived indexes intentionally live in `vtest-store` and the
//! higher-level crates instead.

mod diagnostic;
mod document;
mod evidence;
mod hash;
mod id;
mod protocol;
mod source;
mod subject_hash;
mod test;
mod verification;
mod vo;

pub use diagnostic::*;
pub use document::*;
pub use evidence::*;
pub use hash::*;
pub use id::*;
pub use protocol::*;
pub use source::*;
pub use subject_hash::*;
pub use test::*;
pub use verification::*;
pub use vo::*;
