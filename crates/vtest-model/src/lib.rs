//! Shared, domain-neutral model types for vtest.
//!
//! This crate owns values that cross crate or CLI boundaries.  Filesystem
//! access and derived indexes intentionally live in `vtest-store` and the
//! higher-level crates instead.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fmt, str::FromStr};

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

id_type!(SpecId);
id_type!(ReqId);
id_type!(VoId);
id_type!(TestId);
id_type!(SrcId);
id_type!(AdapterId);

/// A SHA-256 hash bound to a canonical source or record representation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContentHash(String);

impl ContentHash {
    pub fn from_text(text: &str) -> Self {
        let normalized = normalize_hashed_text(text);
        let digest = Sha256::digest(normalized.as_bytes());
        Self(format!("sha256:{digest:x}"))
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let text = String::from_utf8_lossy(bytes);
        Self::from_text(&text)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for ContentHash {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value
            .strip_prefix("sha256:")
            .is_some_and(|hex| hex.len() == 64)
            && value[7..].chars().all(|ch| ch.is_ascii_hexdigit())
        {
            Ok(Self(value.to_owned()))
        } else {
            Err("content hash must be sha256:<64 hexadecimal characters>".to_owned())
        }
    }
}

/// Normalize exactly the whitespace specified by detailed design §1.3.
pub fn normalize_hashed_text(text: &str) -> String {
    let normalized_endings = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut normalized = String::with_capacity(normalized_endings.len());
    for line in normalized_endings.split_inclusive('\n') {
        let has_newline = line.ends_with('\n');
        let body = if has_newline {
            &line[..line.len() - 1]
        } else {
            line
        };
        normalized.push_str(body.trim_end_matches([' ', '\t']));
        if has_newline {
            normalized.push('\n');
        }
    }
    normalized
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Locator {
    pub path: String,
    pub item_path: String,
}

impl Locator {
    pub fn parse(value: &str) -> Option<Self> {
        let separator = value.find("::")?;
        let (path, item_path) = value.split_at(separator);
        let item_path = item_path.strip_prefix("::")?;
        if path.is_empty() || item_path.is_empty() || !path.ends_with(".rs") {
            return None;
        }
        Some(Self {
            path: path.replace('\\', "/"),
            item_path: item_path.to_owned(),
        })
    }

    pub fn as_string(&self) -> String {
        format!("{}::{}", self.path, self.item_path)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum TargetRef {
    Locator(Locator),
    SrcId(SrcId),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: String,
    pub function: String,
    pub start_line: usize,
    pub end_line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "name", rename_all = "snake_case")]
pub enum TestTarget {
    Lib,
    Bin(String),
    IntegrationTest(String),
    Unknown,
}

/// A language-neutral test suite or execution unit.  The adapter owns the
/// interpretation of `kind` and `name`; the verifier treats both as opaque
/// values so the domain model is not tied to Cargo or any other ecosystem.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Default)]
pub struct TestSuite {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Adapter-owned coordinates for executing one Test.  `selector` is opaque to
/// the core and must uniquely identify the Test within the selected adapter.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExecutionDescriptor {
    pub adapter: AdapterId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suite: Option<TestSuite>,
    pub selector: String,
}

impl Default for ExecutionDescriptor {
    fn default() -> Self {
        Self {
            adapter: AdapterId::from("rust-cargo"),
            project: None,
            suite: None,
            selector: String::new(),
        }
    }
}

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
    /// Neutral execution coordinates introduced additively in the adapter
    /// migration.  Missing values in older JSON are mapped to the legacy
    /// Rust-compatible default and never make an old record PASS by
    /// themselves.
    #[serde(default)]
    pub execution: ExecutionDescriptor,
    /// Legacy Rust/Cargo fields retained for v0.1 wire compatibility.  New
    /// core consumers should use `execution` instead.
    pub filter: String,
    pub package: String,
    pub test_target: TestTarget,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceFunction {
    pub locator: Locator,
    pub src_id: Option<SrcId>,
    pub location: SourceLocation,
    pub content_hash: ContentHash,
}

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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum CheckItem {
    #[serde(rename = "spec_coverage")]
    SpecCoverage,
    #[serde(rename = "vo_decomposition")]
    VoDecomposition,
    #[serde(rename = "vo_coverage")]
    VoCoverage,
    #[serde(rename = "test_existence")]
    TestExistence,
    #[serde(rename = "static_audit")]
    StaticAudit,
    #[serde(rename = "semantic_audit")]
    SemanticAudit,
    #[serde(rename = "impl_consistency")]
    ImplConsistency,
    #[serde(rename = "test_execution")]
    TestExecution,
    #[serde(rename = "runtime_result")]
    RuntimeResult,
    #[serde(rename = "target_execution")]
    TargetExecution,
    #[serde(rename = "evidence_validity")]
    EvidenceValidity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "warning")]
    Warning,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: DiagnosticSeverity,
    pub message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Box<SourceLocation>>,
}

impl Diagnostic {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: DiagnosticSeverity::Error,
            message: message.into(),
            candidates: Vec::new(),
            location: None,
        }
    }

    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: DiagnosticSeverity::Warning,
            message: message.into(),
            candidates: Vec::new(),
            location: None,
        }
    }

    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = Some(Box::new(location));
        self
    }

    pub fn with_candidates(mut self, candidates: impl IntoIterator<Item = String>) -> Self {
        self.candidates = candidates.into_iter().collect();
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == DiagnosticSeverity::Error
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct JsonEnvelope<T> {
    pub ok: bool,
    pub data: T,
    pub diagnostics: Vec<Diagnostic>,
}

impl<T> JsonEnvelope<T> {
    pub fn new(ok: bool, data: T, diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            ok,
            data,
            diagnostics,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ExitCode {
    Ok = 0,
    VerificationFailed = 1,
    Usage = 2,
    Internal = 3,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScanSummary {
    pub files: usize,
    pub tests: usize,
    pub sources: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Revision {
    pub commit: Option<String>,
    pub dirty: bool,
}

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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunnerInfo {
    pub kind: String,
    pub command: String,
    pub exit_code: i32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TargetExecution {
    pub checked: bool,
    pub method: Option<String>,
    pub result: CheckValue,
    pub count: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TestResult {
    #[serde(rename = "PASS")]
    Pass,
    #[serde(rename = "FAIL")]
    Fail,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub id: String,
    pub test_id: TestId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter: Option<AdapterId>,
    pub result: TestResult,
    pub executed_at: String,
    pub revision: Revision,
    pub hashes: EvidenceHashes,
    pub runner: RunnerInfo,
    pub target_execution: TargetExecution,
    pub log_ref: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_normalization_only_changes_line_endings_and_trailing_space() {
        assert_eq!(
            ContentHash::from_text("a  \r\nb  \r\n"),
            ContentHash::from_text("a\nb\n")
        );
        assert_ne!(
            ContentHash::from_text("a\n b\n"),
            ContentHash::from_text("a\nb\n")
        );
        assert_ne!(ContentHash::from_text("a"), ContentHash::from_text("a\n"));
    }

    #[test]
    fn locator_splits_at_first_separator() {
        let locator = Locator::parse("src/lib.rs::module::function").expect("valid locator");
        assert_eq!(locator.path, "src/lib.rs");
        assert_eq!(locator.item_path, "module::function");
    }

    #[test]
    fn envelope_has_required_top_level_fields() {
        let value = serde_json::to_value(JsonEnvelope::new(
            true,
            ScanSummary {
                files: 1,
                tests: 2,
                sources: 3,
            },
            vec![],
        ))
        .unwrap();
        assert_eq!(value["ok"], true);
        assert!(value.get("data").is_some());
        assert!(value.get("diagnostics").is_some());
    }

    #[test]
    fn execution_descriptor_is_additive_and_legacy_json_gets_a_safe_default() {
        let value = serde_json::json!({
            "id": "TEST-1",
            "covers": [],
            "target": {"kind": "locator", "value": {"path": "src/lib.rs", "item_path": "target"}},
            "intent": "intent",
            "input": null,
            "expect": null,
            "kind": null,
            "cases": [],
            "related": [],
            "location": {"file": "tests/test.rs", "function": "test", "start_line": 1, "end_line": 1, "start_byte": 0, "end_byte": 1},
            "content_hash": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            "filter": "test",
            "package": "fixture",
            "test_target": {"kind": "lib"}
        });
        let entity: TestEntity = serde_json::from_value(value).unwrap();
        assert_eq!(entity.execution.adapter.as_str(), "rust-cargo");
        assert!(entity.execution.selector.is_empty());

        let encoded = serde_json::to_value(&entity).unwrap();
        assert_eq!(encoded["execution"]["adapter"], "rust-cargo");
    }
}
