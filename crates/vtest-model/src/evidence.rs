//! Execution Evidence: the `EvidenceRecord` persisted as
//! `.verify/evidence/<ULID>.yaml`, with its revision, Execution State
//! subject, content hashes, runner info and target coverage.
//!
//! Boundary: this module stores what the runner reported and never
//! re-judges a result (DS-470, DS-471). Writing and reading the YAML belong
//! to `vtest-exec` / `vtest-store`; deriving `target_binding` states and
//! diagnostic labels from a record belongs to `vtest-verify` (DS-472,
//! DS-473, DS-832).
//!
//! Clauses: DS-S077 (record), DES-097 / DES-101 / DES-184 (Execution
//! State), DES-187 / DES-188 / DS-1410 (target coverage domain), DS-265 /
//! DS-1629 (adapter match), DS-475 (missing Execution State → STALE),
//! DS-476 (E-SCAN-010).

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

impl TargetCoverageResult {
    /// DES-187/DS-1410: the fail-closed aggregate of per-target results.
    /// `PASS` only when every entry is `PASS`; otherwise `FAIL` when any
    /// entry is `FAIL`, else `UNKNOWN`. An empty list is `UNKNOWN` (nothing
    /// was shown to be reached). The Evidence writer records this value and
    /// the reader recomputes it to detect an aggregate that contradicts its
    /// entries (DS-476).
    pub fn aggregate<'a>(results: impl IntoIterator<Item = &'a TargetCoverageResult>) -> Self {
        let mut any = false;
        let mut all_pass = true;
        let mut any_fail = false;
        for result in results {
            any = true;
            match result {
                Self::Pass => {}
                Self::Fail => {
                    all_pass = false;
                    any_fail = true;
                }
                Self::Unknown => all_pass = false,
            }
        }
        if any && all_pass {
            Self::Pass
        } else if any_fail {
            Self::Fail
        } else {
            Self::Unknown
        }
    }
}

/// Per-target coverage observation stored inside `TargetCoverage::targets`.
///
/// The target value is the canonical adapter-owned locator string from the
/// Evidence wire shape. Its result remains the restricted DES-187/DES-188
/// three-value domain; `UNKNOWN` is represented by a null count.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TargetCoverageTarget {
    pub target: String,
    pub result: TargetCoverageResult,
    #[serde(default)]
    pub count: Option<u64>,
}

/// Records how a verification target was observed during test execution.
///
/// When `checked` is false, `method` and `result` are null and `targets` is
/// empty. A legacy `checked: false` record carrying `result: UNKNOWN` is
/// normalized to that shape by the Evidence reader. When `checked` is true,
/// `result` is the restricted `TargetCoverageResult` domain. Diagnostic
/// labels are not stored here; they are derived downstream from
/// `checked`/`count`/`result` (DS-832).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TargetCoverage {
    pub checked: bool,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub result: Option<TargetCoverageResult>,
    #[serde(default)]
    pub targets: Vec<TargetCoverageTarget>,
    #[serde(default)]
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
    /// The record's ULID, also its file name under `.verify/evidence/`
    /// (DS-S077). A plain `String`: readers compare ids only for equality
    /// and lexical order (`vtest-verify` keeps the greater id as a Test's
    /// latest record), so no dedicated type is needed.
    pub id: String,
    pub test_id: TestId,
    /// DS-265 validity input: the adapter that produced this record must
    /// match the Test's current adapter for the record to be reusable.
    pub adapter: AdapterId,
    pub result: TestResult,
    /// RFC 3339 timestamp written by `vtest-exec` at execution time. Stored
    /// and re-emitted verbatim; `vtest-scan` orders history by its string
    /// order and nothing parses it, so it stays a `String`.
    pub executed_at: String,
    pub revision: Revision,
    pub execution_state: ExecutionState,
    pub hashes: EvidenceHashes,
    pub runner: RunnerInfo,
    pub target_coverage: TargetCoverage,
    /// `.verify/`-relative path of the runner log (`cache/logs/<id>.log`),
    /// written by `vtest-exec`. Core stores it and does not read the log
    /// back, so it stays a `String`.
    pub log_ref: String,
}
