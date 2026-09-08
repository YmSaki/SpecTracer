use crate::{AdapterId, ContentHash, TestId};
use serde::{Deserialize, Serialize};

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

/// Result reported for target-binding coverage, per DES-187/DES-188.
///
/// This is a restricted 3-value domain (PASS/FAIL/UNKNOWN), distinct from
/// the canonical 5-value `VerificationState`. Diagnostic labels (e.g.
/// `NOT_CHECKED`, `NOT_EXECUTED`) are derived from `checked`/`count`/`result`
/// by the vtest-verify target_binding check-building logic (DS-832); they
/// are not stored on this record.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TargetCoverageResult {
    #[serde(rename = "PASS")]
    Pass,
    #[serde(rename = "FAIL")]
    Fail,
    #[serde(rename = "UNKNOWN")]
    Unknown,
}

/// Records how a verification target was observed during test execution.
///
/// `result` is the restricted `TargetCoverageResult` this measurement
/// produced. Diagnostic labels are not stored here; they are derived
/// downstream from `checked`/`count`/`result` (DS-832).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TargetCoverage {
    pub checked: bool,
    pub method: Option<String>,
    pub result: TargetCoverageResult,
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

/// Execution State subject, per DES-097 domain `vtest:execution-state:v1`.
///
/// DES-097 binds this hash to the full manifest of repository / toolchain /
/// local-dependency inputs that can change execution results (adapter ID,
/// snapshot schema ID/version, HEAD revision, runner kind and canonical
/// invocation projection, toolchain identity, the canonical projection of
/// adapter config that affects results, and a complete manifest of
/// repository/local-dependency inputs). DES-101 assigns manifest
/// enumeration to the adapter and hashing to core; DES-184 permits `hash`
/// to be `None` exactly when `complete` is `false`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExecutionState {
    pub schema: String,
    pub complete: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<ContentHash>,
}

/// Records execution evidence for a single managed test.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceRecord {
    // CHECKME: Should evidence IDs use a dedicated EvidenceId type?
    pub id: String,
    pub test_id: TestId,
    /// DS-265 validity input: the adapter that produced this record must
    /// match the Test's current adapter for the record to be reusable.
    pub adapter: AdapterId,
    pub result: TestResult,
    // CHECKME: Should execution timestamps use a validated timestamp type?
    pub executed_at: String,
    pub revision: Revision,
    pub execution_state: ExecutionState,
    pub hashes: EvidenceHashes,
    pub runner: RunnerInfo,
    pub target_coverage: TargetCoverage,
    // CHECKME: Should log references have a dedicated type?
    pub log_ref: String,
}
