use crate::{ContentHash, SourceLocation, TargetRef, TestId, VoId};
use serde::{Deserialize, Serialize};

// TODO: Review fail-closed handling of `Unknown`.
// Execution must not silently fall back to an unscoped Cargo target.

/// Identifies the Rust/Cargo target that contains a test.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "name", rename_all = "snake_case")]
pub enum TestTarget {
    Lib,
    Bin(String),
    IntegrationTest(String),
    Unknown,
}

// TODO: Split canonical test data from discovery and Rust/Cargo execution metadata.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TestEntity {
    pub id: TestId,
    pub covers: Vec<VoId>,
    pub target: TargetRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub additional_targets: Vec<TargetRef>,
    pub intent: String,
    pub input: Option<String>,
    pub expect: Option<String>,
    pub kind: Option<String>,
    pub cases: Vec<String>,
    pub related: Vec<TestId>,
    pub location: SourceLocation,
    pub content_hash: ContentHash,
    pub filter: String,
    pub package: String,
    pub test_target: TestTarget,
}
