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
        #[derive(
            Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize,
        )]
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

    /// Hash a typed, domain-separated collection of fields.  Field names and
    /// byte lengths are included so concatenation cannot create an ambiguous
    /// representation.  This is the only shared hash primitive used by the
    /// core subject builders; adapters return bytes and never final hashes.
    pub fn from_domain_fields(domain: &str, fields: &[(&str, &[u8])]) -> Self {
        let mut hasher = Sha256::new();
        write_len_prefixed(&mut hasher, domain.as_bytes());
        for (name, value) in fields {
            write_len_prefixed(&mut hasher, name.as_bytes());
            write_len_prefixed(&mut hasher, value);
        }
        Self(format!("sha256:{:x}", hasher.finalize()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn write_len_prefixed(hasher: &mut Sha256, value: &[u8]) {
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value);
}

pub fn test_subject_hash(
    adapter: &AdapterId,
    test_id: &TestId,
    metadata: &str,
    location: &SourceLocation,
    execution: &ExecutionDescriptor,
    construct: &[u8],
) -> ContentHash {
    // Bind stable construct identity, not absolute byte offsets.  A change to
    // an earlier sibling may move a later item in the file without changing
    // that Test's construct or canonical locator.
    let location_identity = serde_json::json!({
        "adapter": location.adapter.as_ref(),
        "path": location
            .project_relative_path
            .as_deref()
            .unwrap_or(&location.file),
        "locator": location
            .opaque_locator
            .as_deref()
            .unwrap_or(&location.function),
    });
    let location = serde_json::to_vec(&location_identity).unwrap_or_default();
    let execution = serde_json::to_vec(execution).unwrap_or_default();
    ContentHash::from_domain_fields(
        "vtest:test-subject:v1",
        &[
            ("adapter", adapter.as_str().as_bytes()),
            ("test_id", test_id.as_str().as_bytes()),
            ("metadata", metadata.as_bytes()),
            ("location", &location),
            ("execution", &execution),
            ("construct", construct),
        ],
    )
}

pub fn target_subject_hash(target: &NeutralTargetRef, construct: &[u8]) -> ContentHash {
    let target = serde_json::to_vec(target).unwrap_or_default();
    ContentHash::from_domain_fields(
        "vtest:target-subject:v1",
        &[("target", &target), ("construct", construct)],
    )
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

/// A language-neutral opaque target reference.  The adapter owns the meaning
/// of `value`; the core never parses it as a path, module, symbol, or function.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NeutralTargetRef {
    pub adapter: AdapterId,
    pub value: String,
}

impl NeutralTargetRef {
    pub fn new(adapter: impl Into<AdapterId>, value: impl Into<String>) -> Self {
        Self {
            adapter: adapter.into(),
            value: value.into(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: String,
    pub function: String,
    pub start_line: usize,
    pub end_line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
    /// Neutral provenance fields.  The legacy fields above remain readable by
    /// the v1 wire codec; new core consumers use these fields when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter: Option<AdapterId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_relative_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opaque_locator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byte_range: Option<ByteRange>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ByteRange {
    pub start: usize,
    pub end: usize,
}

impl SourceLocation {
    pub fn legacy(
        file: impl Into<String>,
        function: impl Into<String>,
        start_line: usize,
        end_line: usize,
        start_byte: usize,
        end_byte: usize,
    ) -> Self {
        let file = file.into();
        let function = function.into();
        Self {
            project_relative_path: Some(file.clone()),
            opaque_locator: Some(function.clone()),
            adapter: Some(AdapterId::from("rust-cargo")),
            byte_range: Some(ByteRange {
                start: start_byte,
                end: end_byte,
            }),
            file,
            function,
            start_line,
            end_line,
            start_byte,
            end_byte,
        }
    }

    pub fn neutral(
        adapter: impl Into<AdapterId>,
        path: impl Into<String>,
        locator: impl Into<String>,
        range: ByteRange,
    ) -> Self {
        let adapter = adapter.into();
        let path = path.into();
        let locator = locator.into();
        Self {
            adapter: Some(adapter),
            project_relative_path: Some(path.clone()),
            opaque_locator: Some(locator.clone()),
            byte_range: Some(range),
            file: path,
            function: locator,
            start_line: 0,
            end_line: 0,
            start_byte: range.start,
            end_byte: range.end,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct TestSuite {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExecutionDescriptor {
    pub adapter: AdapterId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suite: Option<TestSuite>,
    #[serde(default)]
    pub selector: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_root: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TestEntity {
    pub id: TestId,
    pub covers: Vec<VoId>,
    pub targets: Vec<NeutralTargetRef>,
    pub intent: String,
    pub input: Option<String>,
    pub expect: Option<String>,
    pub kind: Option<String>,
    pub cases: Vec<String>,
    pub related: Vec<TestId>,
    pub location: SourceLocation,
    pub content_hash: ContentHash,
    /// Neutral execution coordinates. Adapter-specific wire compatibility is
    /// owned by the adapter codec and is not part of this domain entity.
    #[serde(default)]
    pub execution: ExecutionDescriptor,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceFunction {
    /// The adapter-owned identity of the implementation construct.  Core
    /// consumers compare this value opaquely and never parse its syntax.
    pub target: NeutralTargetRef,
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
    #[serde(rename = "test_traceability")]
    TestTraceability,
}

impl CheckItem {
    pub const FULL_SCOPE: [Self; 12] = [
        Self::SpecCoverage,
        Self::VoDecomposition,
        Self::VoCoverage,
        Self::TestExistence,
        Self::StaticAudit,
        Self::SemanticAudit,
        Self::ImplConsistency,
        Self::TestExecution,
        Self::RuntimeResult,
        Self::TargetExecution,
        Self::EvidenceValidity,
        Self::TestTraceability,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::SpecCoverage => "spec_coverage",
            Self::VoDecomposition => "vo_decomposition",
            Self::VoCoverage => "vo_coverage",
            Self::TestExistence => "test_existence",
            Self::StaticAudit => "static_audit",
            Self::SemanticAudit => "semantic_audit",
            Self::ImplConsistency => "impl_consistency",
            Self::TestExecution => "test_execution",
            Self::RuntimeResult => "runtime_result",
            Self::TargetExecution => "target_execution",
            Self::EvidenceValidity => "evidence_validity",
            Self::TestTraceability => "test_traceability",
        }
    }
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_subject: Option<ContentHash>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<EvidenceTargetHash>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceTargetHash {
    pub target: NeutralTargetRef,
    pub target_construct: ContentHash,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExecutionStateRecord {
    pub schema: String,
    pub complete: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<ContentHash>,
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
    /// Per-declared-target measurements.  The aggregate fields above retain
    /// the v0.1 wire shape, while this list is the canonical representation
    /// for multi-target execution.  A checked record must contain exactly one
    /// entry for every declared target.
    #[serde(default)]
    pub targets: Vec<TargetExecutionEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TargetExecutionEntry {
    pub target: NeutralTargetRef,
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
    pub result: TestResult,
    pub executed_at: String,
    pub revision: Revision,
    pub hashes: EvidenceHashes,
    pub runner: RunnerInfo,
    pub target_execution: TargetExecution,
    pub log_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter: Option<AdapterId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_state: Option<ExecutionStateRecord>,
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
    fn domain_hash_is_length_delimited_and_non_rust_safe() {
        let first =
            ContentHash::from_domain_fields("vtest:test-subject:v1", &[("a", b"ab"), ("b", b"c")]);
        let second =
            ContentHash::from_domain_fields("vtest:test-subject:v1", &[("a", b"a"), ("b", b"bc")]);
        assert_ne!(first, second);
        assert_ne!(
            ContentHash::from_domain_fields("domain-a", &[("field", b"x")]),
            ContentHash::from_domain_fields("domain-b", &[("field", b"x")])
        );
    }

    #[test]
    fn test_subject_hash_binds_non_adjacent_metadata() {
        let location = SourceLocation::neutral(
            "synthetic",
            "fixture.spec",
            "case",
            ByteRange { start: 0, end: 12 },
        );
        let execution = ExecutionDescriptor {
            adapter: AdapterId::from("synthetic"),
            selector: "case".to_owned(),
            ..ExecutionDescriptor::default()
        };
        let first = test_subject_hash(
            &AdapterId::from("synthetic"),
            &TestId::new("TEST-SYNTHETIC"),
            "intent: first",
            &location,
            &execution,
            b"case construct",
        );
        let second = test_subject_hash(
            &AdapterId::from("synthetic"),
            &TestId::new("TEST-SYNTHETIC"),
            "intent: second",
            &location,
            &execution,
            b"case construct",
        );
        assert_ne!(first, second);
    }
}
