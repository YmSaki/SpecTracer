//! Shared, domain-neutral model types for vtest.
//!
//! This crate owns values that cross crate or CLI boundaries.  Filesystem
//! access and derived indexes intentionally live in `vtest-store` and the
//! higher-level crates instead.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt, str::FromStr};

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
id_type!(ProjectPath);

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
        let digest = Sha256::digest(bytes);
        Self(format!("sha256:{digest:x}"))
    }

    /// Hash a domain-separated sequence of named, length-prefixed fields.
    pub fn from_subject<'a>(
        domain: &str,
        fields: impl IntoIterator<Item = (&'a str, Vec<u8>)>,
    ) -> Self {
        let mut encoder = SubjectEncoder::new(domain);
        for (name, value) in fields {
            encoder.field(name, &value);
        }
        encoder.finish()
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

struct SubjectEncoder {
    hasher: Sha256,
}

impl SubjectEncoder {
    fn new(domain: &str) -> Self {
        let mut value = Self {
            hasher: Sha256::new(),
        };
        value.field("domain", domain.as_bytes());
        value
    }

    fn field(&mut self, name: &str, value: &[u8]) {
        self.hasher.update((name.len() as u64).to_be_bytes());
        self.hasher.update(name.as_bytes());
        self.hasher.update((value.len() as u64).to_be_bytes());
        self.hasher.update(value);
    }

    fn finish(self) -> ContentHash {
        let digest = self.hasher.finalize();
        ContentHash(format!("sha256:{digest:x}"))
    }
}

/// A language-neutral, type-preserving value used in freshness projections.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum CanonicalProjection {
    Null,
    Bool(bool),
    Integer(i64),
    Unsigned(u64),
    Decimal(String),
    String(String),
    List(Vec<CanonicalProjection>),
    Map(BTreeMap<String, CanonicalProjection>),
}

impl CanonicalProjection {
    pub fn encode(&self) -> Vec<u8> {
        let mut output = Vec::new();
        self.encode_into(&mut output);
        output
    }

    fn encode_into(&self, output: &mut Vec<u8>) {
        match self {
            Self::Null => output.extend_from_slice(b"null"),
            Self::Bool(value) => {
                output.extend_from_slice(b"bool");
                output.push(u8::from(*value));
            }
            Self::Integer(value) => {
                output.extend_from_slice(b"integer");
                output.extend_from_slice(&value.to_be_bytes());
            }
            Self::Unsigned(value) => {
                output.extend_from_slice(b"unsigned");
                output.extend_from_slice(&value.to_be_bytes());
            }
            Self::Decimal(value) => encode_projection_value(output, b"decimal", value.as_bytes()),
            Self::String(value) => encode_projection_value(output, b"string", value.as_bytes()),
            Self::List(values) => {
                output.extend_from_slice(b"list");
                output.extend_from_slice(&(values.len() as u64).to_be_bytes());
                for value in values {
                    let encoded = value.encode();
                    output.extend_from_slice(&(encoded.len() as u64).to_be_bytes());
                    output.extend_from_slice(&encoded);
                }
            }
            Self::Map(values) => {
                output.extend_from_slice(b"map");
                output.extend_from_slice(&(values.len() as u64).to_be_bytes());
                for (key, value) in values {
                    encode_projection_value(output, b"key", key.as_bytes());
                    let encoded = value.encode();
                    output.extend_from_slice(&(encoded.len() as u64).to_be_bytes());
                    output.extend_from_slice(&encoded);
                }
            }
        }
    }
}

fn encode_projection_value(output: &mut Vec<u8>, tag: &[u8], value: &[u8]) {
    output.extend_from_slice(&(tag.len() as u64).to_be_bytes());
    output.extend_from_slice(tag);
    output.extend_from_slice(&(value.len() as u64).to_be_bytes());
    output.extend_from_slice(value);
}

fn canonical_json(value: &Value) -> Vec<u8> {
    match value {
        Value::Null => CanonicalProjection::Null,
        Value::Bool(value) => CanonicalProjection::Bool(*value),
        Value::Number(value) if value.is_i64() => {
            CanonicalProjection::Integer(value.as_i64().expect("checked i64"))
        }
        Value::Number(value) if value.is_u64() => {
            CanonicalProjection::Unsigned(value.as_u64().expect("checked u64"))
        }
        Value::Number(value) => CanonicalProjection::Decimal(value.to_string()),
        Value::String(value) => CanonicalProjection::String(value.clone()),
        Value::Array(values) => {
            let values = values
                .iter()
                .map(|value| projection_from_json(value.clone()))
                .collect();
            CanonicalProjection::List(values)
        }
        Value::Object(values) => CanonicalProjection::Map(
            values
                .iter()
                .map(|(key, value)| (key.clone(), projection_from_json(value.clone())))
                .collect(),
        ),
    }
    .encode()
}

fn projection_from_json(value: Value) -> CanonicalProjection {
    match value {
        Value::Null => CanonicalProjection::Null,
        Value::Bool(value) => CanonicalProjection::Bool(value),
        Value::Number(value) if value.is_i64() => {
            CanonicalProjection::Integer(value.as_i64().expect("checked i64"))
        }
        Value::Number(value) if value.is_u64() => {
            CanonicalProjection::Unsigned(value.as_u64().expect("checked u64"))
        }
        Value::Number(value) => CanonicalProjection::Decimal(value.to_string()),
        Value::String(value) => CanonicalProjection::String(value),
        Value::Array(values) => {
            CanonicalProjection::List(values.into_iter().map(projection_from_json).collect())
        }
        Value::Object(values) => CanonicalProjection::Map(
            values
                .into_iter()
                .map(|(key, value)| (key, projection_from_json(value)))
                .collect(),
        ),
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum TargetRef {
    Locator { adapter: AdapterId, value: String },
    SrcId(SrcId),
}

impl TargetRef {
    pub fn normalized(&self) -> String {
        match self {
            Self::Locator { adapter, value } => format!("{adapter}::{value}"),
            Self::SrcId(id) => id.to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceRange {
    pub start: usize,
    pub end: usize,
    #[serde(default)]
    pub start_line: usize,
    #[serde(default)]
    pub end_line: usize,
}

impl SourceRange {
    pub fn is_valid_for(self, bytes: &[u8]) -> bool {
        self.start <= self.end && self.end <= bytes.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub adapter: AdapterId,
    pub path: ProjectPath,
    pub locator: String,
    pub byte_range: SourceRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TestSuite {
    pub kind: String,
    pub name: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExecutionDescriptor {
    pub adapter: AdapterId,
    pub project: Option<String>,
    pub suite: Option<TestSuite>,
    pub selector: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TestEntity {
    pub id: TestId,
    pub covers: Vec<VoId>,
    pub targets: Vec<TargetRef>,
    pub intent: String,
    pub input: Option<String>,
    pub expect: Option<String>,
    pub kind: Option<String>,
    pub cases: Vec<String>,
    pub related: Vec<TestId>,
    pub location: SourceLocation,
    pub content_hash: ContentHash,
    pub execution: ExecutionDescriptor,
}

impl TestEntity {
    pub fn validate(&self) -> Result<(), String> {
        if self.targets.is_empty() {
            return Err("TestEntity.targets must contain at least one TargetRef".to_owned());
        }
        if self.execution.adapter != self.location.adapter {
            return Err("Test execution and SourceLocation adapter IDs must match".to_owned());
        }
        if self.execution.selector.is_empty() {
            return Err("ExecutionDescriptor.selector must not be empty".to_owned());
        }
        let unique = self
            .targets
            .iter()
            .map(TargetRef::normalized)
            .collect::<std::collections::BTreeSet<_>>();
        if unique.len() != self.targets.len() {
            return Err("TestEntity.targets must not contain duplicates".to_owned());
        }
        for target in &self.targets {
            if let TargetRef::Locator { adapter, value } = target {
                if adapter.as_str().is_empty() || value.is_empty() {
                    return Err("locator TargetRef requires adapter and opaque value".to_owned());
                }
            }
        }
        Ok(())
    }
}

pub struct TestSubjectInput<'a> {
    pub adapter: &'a AdapterId,
    pub id: &'a TestId,
    pub covers: &'a [VoId],
    pub targets: &'a [TargetRef],
    pub intent: &'a str,
    pub input: Option<&'a str>,
    pub expect: Option<&'a str>,
    pub kind: Option<&'a str>,
    pub cases: &'a [String],
    pub related: &'a [TestId],
    pub location: &'a SourceLocation,
    pub execution: &'a ExecutionDescriptor,
    pub construct: &'a [u8],
}

pub fn hash_test_subject(input: &TestSubjectInput<'_>) -> ContentHash {
    let mut covers = input
        .covers
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    covers.sort();
    let mut targets = input
        .targets
        .iter()
        .map(TargetRef::normalized)
        .collect::<Vec<_>>();
    targets.sort();
    let mut related = input
        .related
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    related.sort();
    let normalized_construct = String::from_utf8(input.construct.to_vec())
        .map(|value| normalize_hashed_text(&value).into_bytes())
        .unwrap_or_else(|_| input.construct.to_vec());
    let suite = input
        .execution
        .suite
        .as_ref()
        .map(|suite| serde_json::json!({"kind": suite.kind, "name": suite.name}));
    ContentHash::from_subject(
        "vtest:test-subject:v1",
        [
            ("adapter", input.adapter.as_str().as_bytes().to_vec()),
            ("id", input.id.as_str().as_bytes().to_vec()),
            ("covers", canonical_json(&serde_json::json!(covers))),
            ("targets", canonical_json(&serde_json::json!(targets))),
            ("intent", input.intent.as_bytes().to_vec()),
            ("input", canonical_json(&serde_json::json!(input.input))),
            ("expect", canonical_json(&serde_json::json!(input.expect))),
            ("kind", canonical_json(&serde_json::json!(input.kind))),
            ("cases", canonical_json(&serde_json::json!(input.cases))),
            ("related", canonical_json(&serde_json::json!(related))),
            (
                "location.adapter",
                input.location.adapter.as_str().as_bytes().to_vec(),
            ),
            (
                "location.path",
                input.location.path.as_str().as_bytes().to_vec(),
            ),
            (
                "location.locator",
                input.location.locator.as_bytes().to_vec(),
            ),
            (
                "execution.adapter",
                input.execution.adapter.as_str().as_bytes().to_vec(),
            ),
            (
                "execution.project",
                canonical_json(&serde_json::json!(input.execution.project)),
            ),
            ("execution.suite", canonical_json(&serde_json::json!(suite))),
            (
                "execution.selector",
                input.execution.selector.as_bytes().to_vec(),
            ),
            ("construct", normalized_construct),
        ],
    )
}

pub fn hash_target_subject(target: &TargetRef, construct: &[u8]) -> ContentHash {
    let normalized_construct = String::from_utf8(construct.to_vec())
        .map(|value| normalize_hashed_text(&value).into_bytes())
        .unwrap_or_else(|_| construct.to_vec());
    ContentHash::from_subject(
        "vtest:target-subject:v1",
        [
            ("target", target.normalized().into_bytes()),
            ("construct", normalized_construct),
        ],
    )
}

pub fn hash_static_audit_config_subject(
    adapter: &AdapterId,
    rule_set_id: &str,
    rule_set_version: &str,
    effective_config: &CanonicalProjection,
) -> ContentHash {
    ContentHash::from_subject(
        "vtest:static-audit-config:v1",
        [
            ("adapter", adapter.as_str().as_bytes().to_vec()),
            ("rule_set_id", rule_set_id.as_bytes().to_vec()),
            ("rule_set_version", rule_set_version.as_bytes().to_vec()),
            ("effective_config", effective_config.encode()),
        ],
    )
}

pub fn hash_static_analysis_source_subject(
    adapter: &AdapterId,
    location: &SourceLocation,
    bytes: &[u8],
) -> ContentHash {
    ContentHash::from_subject(
        "vtest:static-analysis-source:v1",
        [
            ("adapter", adapter.as_str().as_bytes().to_vec()),
            ("path", location.path.as_str().as_bytes().to_vec()),
            ("locator", location.locator.as_bytes().to_vec()),
            ("bytes", bytes.to_vec()),
        ],
    )
}

pub fn hash_record_subject(record: &CanonicalProjection) -> ContentHash {
    ContentHash::from_subject("vtest:record-subject:v1", [("record", record.encode())])
}

/// The hash of a referenced Specification source. This is the only definition
/// of a Specification source hash: `SpecRecord.sha256` is the value this
/// returned at registration time, never a substitute for the current one.
pub fn hash_specification_source(source: &str) -> SpecSourceHash {
    SpecSourceHash(ContentHash::from_text(source))
}

/// A Specification source hash. Distinct from the hash of the SPEC record that
/// references it, so the two cannot be compared or substituted by accident.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SpecSourceHash(ContentHash);

impl SpecSourceHash {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// The snapshot value as a plain content hash. A caller that binds a
    /// judgment to this is binding to registration time, not to the current
    /// Specification source, so it must recompute the source itself.
    pub fn registered_snapshot(&self) -> &ContentHash {
        &self.0
    }
}

impl fmt::Display for SpecSourceHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for SpecSourceHash {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse().map(Self)
    }
}

/// Bind an Approval to everything that can change its validity: the subject
/// VO's own record subject plus every upstream dependency subject, in the
/// canonical order the closure was resolved in.
pub fn hash_approval_subject<'a>(
    subject: &ContentHash,
    dependencies: impl IntoIterator<Item = (&'a str, &'a str, &'a ContentHash)>,
) -> ContentHash {
    let closure = CanonicalProjection::List(
        dependencies
            .into_iter()
            .map(|(kind, id, hash)| {
                CanonicalProjection::Map(BTreeMap::from([
                    ("kind".to_owned(), CanonicalProjection::String(kind.into())),
                    ("id".to_owned(), CanonicalProjection::String(id.into())),
                    (
                        "hash".to_owned(),
                        CanonicalProjection::String(hash.as_str().to_owned()),
                    ),
                ]))
            })
            .collect(),
    );
    ContentHash::from_subject(
        "vtest:approval-subject:v1",
        [
            ("subject", subject.as_str().as_bytes().to_vec()),
            ("dependencies", closure.encode()),
        ],
    )
}

pub fn hash_spec_subject(record: &CanonicalProjection, source: &str) -> ContentHash {
    ContentHash::from_subject(
        "vtest:spec-subject:v1",
        [
            ("record", record.encode()),
            ("source", normalize_hashed_text(source).as_bytes().to_vec()),
        ],
    )
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceTarget {
    pub target: TargetRef,
    pub src_id: Option<SrcId>,
    pub location: SourceLocation,
    pub content_hash: ContentHash,
}

pub type SourceFunction = SourceTarget;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiscoveredTest {
    pub adapter: AdapterId,
    pub location: SourceLocation,
    pub content_hash: ContentHash,
    pub managed: ManagedTestLink,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedTestLink {
    Missing,
    One(TestId),
    Multiple(Vec<TestId>),
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
pub struct EvidenceTargetHash {
    pub target: String,
    pub target_construct: ContentHash,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceHashes {
    pub test_subject: Option<ContentHash>,
    pub targets: Vec<EvidenceTargetHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility: Option<CompatibilityEvidenceHashes>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompatibilityEvidenceHashes {
    pub test_construct: ContentHash,
    pub target_constructs: Vec<ContentHash>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExecutionStateSubject {
    pub schema: String,
    pub complete: bool,
    pub hash: Option<ContentHash>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunnerInfo {
    pub kind: String,
    pub command: String,
    pub exit_code: i32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TargetExecutionObservation {
    pub target: String,
    pub result: CheckValue,
    pub count: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TargetExecution {
    pub checked: bool,
    pub method: Option<String>,
    pub result: Option<CheckValue>,
    pub targets: Vec<TargetExecutionObservation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility_count: Option<u64>,
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
    pub adapter: Option<AdapterId>,
    pub result: TestResult,
    pub executed_at: String,
    pub revision: Revision,
    pub execution_state: Option<ExecutionStateSubject>,
    pub hashes: EvidenceHashes,
    pub runner: RunnerInfo,
    pub target_execution: TargetExecution,
    pub log_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditSubject {
    Record {
        id: String,
        hash: ContentHash,
    },
    Target {
        target: String,
        hash: ContentHash,
    },
    StaticConfig {
        adapter: AdapterId,
        capability: String,
        hash: ContentHash,
    },
    AnalysisSource {
        adapter: AdapterId,
        path: ProjectPath,
        locator: String,
        hash: ContentHash,
    },
}

pub struct ExecutionInputSubject<'a> {
    pub root_identity: &'a str,
    pub root_relative_path: &'a str,
    pub kind: &'a str,
    pub bytes: &'a [u8],
}

pub struct ExecutionStateSubjectInput<'a> {
    pub adapter: &'a AdapterId,
    pub schema_id: &'a str,
    pub schema_version: &'a str,
    pub head_revision: Option<&'a str>,
    pub runner_kind: &'a str,
    pub invocation: &'a CanonicalProjection,
    pub toolchain_identity: &'a str,
    pub effective_config: &'a CanonicalProjection,
    pub inputs: &'a [ExecutionInputSubject<'a>],
}

pub fn hash_execution_state_subject(input: &ExecutionStateSubjectInput<'_>) -> ContentHash {
    let mut entries = input.inputs.iter().collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        (left.root_identity, left.root_relative_path, left.kind).cmp(&(
            right.root_identity,
            right.root_relative_path,
            right.kind,
        ))
    });
    let encoded_entries = CanonicalProjection::List(
        entries
            .into_iter()
            .map(|entry| {
                CanonicalProjection::Map(BTreeMap::from([
                    (
                        "root_identity".to_owned(),
                        CanonicalProjection::String(entry.root_identity.to_owned()),
                    ),
                    (
                        "root_relative_path".to_owned(),
                        CanonicalProjection::String(entry.root_relative_path.to_owned()),
                    ),
                    (
                        "kind".to_owned(),
                        CanonicalProjection::String(entry.kind.to_owned()),
                    ),
                    (
                        "bytes".to_owned(),
                        CanonicalProjection::String(hex_bytes(entry.bytes)),
                    ),
                ]))
            })
            .collect(),
    )
    .encode();
    ContentHash::from_subject(
        "vtest:execution-state:v1",
        [
            ("adapter", input.adapter.as_str().as_bytes().to_vec()),
            ("schema_id", input.schema_id.as_bytes().to_vec()),
            ("schema_version", input.schema_version.as_bytes().to_vec()),
            (
                "head_revision",
                canonical_json(&serde_json::json!(input.head_revision)),
            ),
            ("runner_kind", input.runner_kind.as_bytes().to_vec()),
            ("invocation", input.invocation.encode()),
            (
                "toolchain_identity",
                input.toolchain_identity.as_bytes().to_vec(),
            ),
            ("effective_config", input.effective_config.encode()),
            ("inputs", encoded_entries),
        ],
    )
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

/// Agent Form Engineering schema: the questions, options, and Rust template a
/// Structured Test Operation is driven by. Domain data shared across the store
/// (which loads it) and the adapters (which render from it).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FormSchema {
    pub kind: String,
    pub adapter: Option<String>,
    pub title: String,
    pub fields: Vec<FormField>,
    pub template: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FormField {
    pub name: String,
    pub question: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub required: bool,
    pub options: Vec<String>,
    pub validate: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FormValue {
    Scalar(String),
    List(Vec<String>),
}

impl FormValue {
    pub fn values(&self) -> Vec<&str> {
        match self {
            Self::Scalar(value) => vec![value.as_str()],
            Self::List(values) => values.iter().map(String::as_str).collect(),
        }
    }

    pub fn render(&self) -> String {
        match self {
            Self::Scalar(value) => value.clone(),
            Self::List(values) => values.join(","),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Scalar(value) => value.trim().is_empty(),
            Self::List(values) => {
                values.is_empty() || values.iter().any(|value| value.trim().is_empty())
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FormAnswers {
    pub form: String,
    pub answers: BTreeMap<String, FormValue>,
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
    fn locator_is_adapter_scoped_and_opaque() {
        let locator = TargetRef::Locator {
            adapter: AdapterId::new("synthetic"),
            value: "component(add)/scenario[happy]".to_owned(),
        };
        assert_eq!(
            locator.normalized(),
            "synthetic::component(add)/scenario[happy]"
        );
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

    fn fixture_location() -> SourceLocation {
        SourceLocation {
            adapter: AdapterId::new("synthetic"),
            path: ProjectPath::new("source/cases.synth"),
            locator: "scenario[adding two values]".to_owned(),
            byte_range: SourceRange {
                start: 65,
                end: 176,
                start_line: 1,
                end_line: 1,
            },
        }
    }

    fn fixture_execution() -> ExecutionDescriptor {
        ExecutionDescriptor {
            adapter: AdapterId::new("synthetic"),
            project: None,
            suite: Some(TestSuite {
                kind: "fixed-observation".to_owned(),
                name: None,
            }),
            selector: "scenario[adding two values]".to_owned(),
        }
    }

    fn synthetic_test_hash(intent: &str) -> ContentHash {
        let id = TestId::new("TEST-SYNTH-ADD");
        let covers = vec![VoId::new("VO-SYNTH-ADD")];
        let targets = vec![TargetRef::Locator {
            adapter: AdapterId::new("synthetic"),
            value: "component(add)/scenario[happy]".to_owned(),
        }];
        let location = fixture_location();
        let execution = fixture_execution();
        hash_test_subject(&TestSubjectInput {
            adapter: &execution.adapter,
            id: &id,
            covers: &covers,
            targets: &targets,
            intent,
            input: None,
            expect: None,
            kind: Some("scenario"),
            cases: &[],
            related: &[],
            location: &location,
            execution: &execution,
            construct: b"scenario[adding two values]",
        })
    }

    #[test]
    fn non_adjacent_logical_metadata_changes_the_test_subject() {
        assert_ne!(
            synthetic_test_hash("adding two values returns their sum"),
            synthetic_test_hash("adding two values returns a changed sum")
        );
    }

    #[test]
    fn byte_range_movement_does_not_change_the_test_subject() {
        let original = synthetic_test_hash("adding two values returns their sum");
        let mut moved = fixture_location();
        moved.byte_range.start += 100;
        moved.byte_range.end += 100;
        let id = TestId::new("TEST-SYNTH-ADD");
        let covers = vec![VoId::new("VO-SYNTH-ADD")];
        let targets = vec![TargetRef::Locator {
            adapter: AdapterId::new("synthetic"),
            value: "component(add)/scenario[happy]".to_owned(),
        }];
        let execution = fixture_execution();
        let actual = hash_test_subject(&TestSubjectInput {
            adapter: &execution.adapter,
            id: &id,
            covers: &covers,
            targets: &targets,
            intent: "adding two values returns their sum",
            input: None,
            expect: None,
            kind: Some("scenario"),
            cases: &[],
            related: &[],
            location: &moved,
            execution: &execution,
            construct: b"scenario[adding two values]",
        });
        assert_eq!(original, actual);
    }

    #[test]
    fn static_config_subject_binds_only_the_typed_projection() {
        let adapter = AdapterId::new("synthetic");
        let projection = CanonicalProjection::Map(BTreeMap::from([(
            "assertion_macros".to_owned(),
            CanonicalProjection::List(vec![CanonicalProjection::String("check".to_owned())]),
        )]));
        let same = hash_static_audit_config_subject(&adapter, "rules", "1", &projection);
        let changed = hash_static_audit_config_subject(
            &adapter,
            "rules",
            "1",
            &CanonicalProjection::Map(BTreeMap::from([(
                "assertion_macros".to_owned(),
                CanonicalProjection::List(vec![CanonicalProjection::String("verify".to_owned())]),
            )])),
        );
        assert_ne!(same, changed);
        assert_eq!(
            same,
            hash_static_audit_config_subject(&adapter, "rules", "1", &projection)
        );
    }

    #[test]
    fn subject_encoding_distinguishes_null_empty_and_empty_list() {
        let null = CanonicalProjection::Null.encode();
        let empty = CanonicalProjection::String(String::new()).encode();
        let list = CanonicalProjection::List(Vec::new()).encode();
        assert_ne!(null, empty);
        assert_ne!(empty, list);
        assert_ne!(null, list);
    }

    #[test]
    fn subject_domains_prevent_cross_type_hash_reuse() {
        let value = CanonicalProjection::String("same bytes".to_owned());
        assert_ne!(
            hash_record_subject(&value),
            hash_spec_subject(&value, "same bytes")
        );
        assert_ne!(
            hash_record_subject(&value),
            ContentHash::from_subject("vtest:target-subject:v1", [("record", value.encode())])
        );
    }
}
