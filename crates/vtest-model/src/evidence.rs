use crate::{ContentHash, TestId};
use serde::{Deserialize, Serialize};

/// Predecessor-model verification result.
///
/// This type mixes verification states and diagnostic conditions.
#[deprecated(
    note = "Predecessor model: replace with VerificationState and DiagnosticLabel during the canonical v0.1 migration"
)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CheckValue {
    Pass,
    Fail,
    Mismatch,
    Missing,
    NotChecked,
    NotExecuted,
    Stale,
    Unknown,
}

/// Identifies the Git revision and whether the working tree had uncommitted changes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Revision {
    pub commit: Option<String>,
    pub dirty: bool,
}

// TODO: Remove the predecessor single-target wire shape.
// Evidence hashes should have one canonical representation.
/// Stores content hashes bound to test and target functions in execution evidence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceHashes {
    pub test_fn: ContentHash,
    /// The first target hash retained for the v0.1 wire shape.
    pub target_fn: ContentHash,
    /// All declared target hashes in annotation order.  An empty value means
    /// that the record uses the v0.1 single-target shape and `target_fn` is
    /// the complete set.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub target_fns: Vec<ContentHash>,
}

/// Describes the test runner invocation that produced execution evidence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunnerInfo {
    pub kind: String,
    pub command: String,
    pub exit_code: i32,
}

/// Records how a verification target was observed during test execution.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TargetExecution {
    pub checked: bool,
    pub method: Option<String>,
    pub result: CheckValue,
    pub count: Option<u64>,
}

/// Result reported by the test runner for an executed test.
///
/// This is a test execution result, not a verification state.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TestResult {
    #[serde(rename = "PASS")]
    Pass,
    #[serde(rename = "FAIL")]
    Fail,
}

/// Records execution evidence for a single managed test.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceRecord {
    // CHECKME: Should evidence IDs use a dedicated EvidenceId type?
    pub id: String,
    pub test_id: TestId,
    pub result: TestResult,
    // CHECKME: Should execution timestamps use a validated timestamp type?
    pub executed_at: String,
    pub revision: Revision,
    pub hashes: EvidenceHashes,
    pub runner: RunnerInfo,
    pub target_execution: TargetExecution,
    // CHECKME: Should log references have a dedicated type?
    pub log_ref: String,
}
