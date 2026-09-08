use crate::{AdapterId, ContentHash, DiagnosticLabel, TestId, VerificationState};
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

/// Records how a verification target was observed during test execution.
///
/// `result` is the canonical `VerificationState` this measurement produced;
/// `diagnostic` is a separate, optional label giving additional context
/// (e.g. why coverage is `NoEvidence`). The two are independent fields per
/// the canonical model: a diagnostic label is never a verification state.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TargetExecution {
    pub checked: bool,
    pub method: Option<String>,
    pub result: VerificationState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<DiagnosticLabel>,
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
    pub target_execution: TargetExecution,
    // CHECKME: Should log references have a dedicated type?
    pub log_ref: String,
}
