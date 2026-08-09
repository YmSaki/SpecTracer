//! Language- and runner-neutral adapter contracts.
//!
//! Adapters return observations and hash-free drafts.  Core crates own
//! canonicalization, subject hashing, record persistence, and aggregation.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Arc,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use vtest_model::{
    AdapterId, CheckValue, Diagnostic, ExecutionDescriptor, NeutralTargetRef, ScanSummary,
    SourceFunction, SourceLocation, TestEntity, TestId, TestResult,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    Discovery,
    TestWireCodec,
    StaticAudit,
    StructuredTest,
    Runner,
    Coverage,
}

impl Capability {
    pub const ALL: [Self; 6] = [
        Self::Discovery,
        Self::TestWireCodec,
        Self::StaticAudit,
        Self::StructuredTest,
        Self::Runner,
        Self::Coverage,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Discovery => "discovery",
            Self::TestWireCodec => "test-wire-codec",
            Self::StaticAudit => "static-audit",
            Self::StructuredTest => "structured-test",
            Self::Runner => "runner",
            Self::Coverage => "coverage",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AdapterDescriptor {
    pub id: AdapterId,
    pub languages: Vec<String>,
    pub capabilities: BTreeSet<Capability>,
    pub config_namespace: String,
}

impl AdapterDescriptor {
    pub fn new(
        id: impl Into<AdapterId>,
        languages: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        let mut languages = languages.into_iter().map(Into::into).collect::<Vec<_>>();
        languages.sort();
        languages.dedup();
        Self {
            id: id.into(),
            languages,
            capabilities: BTreeSet::new(),
            config_namespace: String::new(),
        }
    }

    pub fn with_capabilities(mut self, capabilities: impl IntoIterator<Item = Capability>) -> Self {
        self.capabilities.extend(capabilities);
        self
    }

    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.config_namespace = namespace.into();
        self
    }

    pub fn supports(&self, capability: Capability) -> bool {
        self.capabilities.contains(&capability)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AdapterConfig {
    pub values: BTreeMap<String, String>,
}

impl AdapterConfig {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceFragment {
    pub adapter: AdapterId,
    pub project_relative_path: String,
    pub opaque_locator: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ManagedTestDraft {
    pub adapter: AdapterId,
    pub id: TestId,
    pub covers: Vec<String>,
    pub targets: Vec<NeutralTargetRef>,
    pub intent: String,
    pub input: Option<String>,
    pub expect: Option<String>,
    pub kind: Option<String>,
    pub cases: Vec<String>,
    pub related: Vec<TestId>,
    pub location: SourceLocation,
    pub execution: ExecutionDescriptor,
    pub metadata_sources: Vec<SourceFragment>,
    pub construct: Vec<u8>,
    /// Adapter-owned wire payload. The core passes it to the adapter's wire
    /// codec and never interprets its fields.
    #[serde(default)]
    pub wire_payload: serde_json::Value,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiscoveredTestDraft {
    pub adapter: AdapterId,
    pub location: SourceLocation,
    pub construct: Vec<u8>,
    pub metadata_sources: Vec<SourceFragment>,
    pub managed: ManagedTestDraftLink,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ManagedTestDraftLink {
    Missing,
    One(Box<ManagedTestDraft>),
    Multiple(Vec<ManagedTestDraft>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceTargetDraft {
    pub adapter: AdapterId,
    pub target: NeutralTargetRef,
    pub location: SourceLocation,
    pub construct: Vec<u8>,
    pub src_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DiscoveryCompleteness {
    Complete,
    Incomplete { reason: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiscoveryBatch {
    pub adapter: AdapterId,
    pub summary: ScanSummary,
    pub completeness: DiscoveryCompleteness,
    pub discovered_tests: Vec<DiscoveredTestDraft>,
    pub managed_tests: Vec<ManagedTestDraft>,
    pub targets: Vec<SourceTargetDraft>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Canonical discovery output consumed by the language-neutral orchestration
/// layer.  The adapter owns parsing and resolution; the core only merges this
/// already-typed result and rebuilds diagnostics/aggregates.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiscoveryResult {
    pub adapter: AdapterId,
    pub summary: ScanSummary,
    pub tests: Vec<TestEntity>,
    pub sources: Vec<SourceFunction>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StaticAnalysisClosureDraft {
    pub complete: bool,
    pub sources: Vec<SourceFragment>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StaticAuditConfigDraft {
    pub rule_set_id: String,
    pub rule_set_version: String,
    pub effective_config: serde_json::Value,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StaticAuditObservation {
    pub verdict: CheckValue,
    pub config: StaticAuditConfigDraft,
    pub closure: StaticAnalysisClosureDraft,
    pub rules: Vec<AuditRuleObservation>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AuditRuleObservation {
    pub rule: String,
    pub verdict: CheckValue,
    pub reason: String,
    pub location: SourceLocation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExecutionInputDraft {
    pub root_identity: String,
    pub root_relative_path: String,
    pub kind: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExecutionStateDraft {
    pub schema_id: String,
    pub schema_version: String,
    pub complete: bool,
    pub head_revision: Option<String>,
    pub runner_kind: String,
    pub invocation: serde_json::Value,
    pub toolchain_identity: String,
    pub effective_config: serde_json::Value,
    pub inputs: Vec<ExecutionInputDraft>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunnerObservation {
    pub adapter: AdapterId,
    pub result: TestResult,
    pub runner_kind: String,
    pub command: String,
    pub exit_code: i32,
    pub log: String,
    pub target_execution: TargetExecutionObservation,
    pub execution_state: ExecutionStateDraft,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TargetExecutionObservation {
    pub checked: bool,
    pub method: Option<String>,
    pub result: CheckValue,
    pub count: Option<u64>,
    pub targets: Vec<TargetExecutionEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TargetExecutionEntry {
    pub target: NeutralTargetRef,
    pub result: CheckValue,
    pub count: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StructuredTestResult {
    pub test: Option<ManagedTestDraft>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
#[error("adapter {code}: {message}")]
pub struct AdapterError {
    pub code: String,
    pub message: String,
    pub capability: Option<Capability>,
}

impl AdapterError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            capability: None,
        }
    }

    pub fn capability(mut self, capability: Capability) -> Self {
        self.capability = Some(capability);
        self
    }

    pub fn diagnostic(&self) -> Diagnostic {
        Diagnostic::error(self.code.clone(), self.message.clone())
    }
}

pub trait SourceDiscoveryAdapter: Send + Sync {
    fn descriptor(&self) -> AdapterDescriptor;
    fn discover(&self, root: &Path, config: &AdapterConfig)
        -> Result<DiscoveryBatch, AdapterError>;
}

pub trait TestWireCodec: Send + Sync {
    fn descriptor(&self) -> AdapterDescriptor;
    fn decode_test(&self, _bytes: &[u8]) -> Result<Vec<ManagedTestDraft>, AdapterError>;
    fn materialize_test(&self, draft: &ManagedTestDraft) -> Result<TestEntity, AdapterError>;
    fn encode_test(&self, test: &TestEntity) -> Result<serde_json::Value, AdapterError>;
}

pub trait StaticAuditAdapter: Send + Sync {
    fn descriptor(&self) -> AdapterDescriptor;
    fn audit(
        &self,
        root: &Path,
        test: &TestEntity,
        config: &AdapterConfig,
    ) -> Result<StaticAuditObservation, AdapterError>;
}

pub trait StructuredTestAdapter: Send + Sync {
    fn descriptor(&self) -> AdapterDescriptor;
    fn form_kinds(&self) -> BTreeSet<String> {
        BTreeSet::new()
    }
}

pub trait TestRunnerAdapter: Send + Sync {
    fn descriptor(&self) -> AdapterDescriptor;
    fn run(
        &self,
        root: &Path,
        test: &TestEntity,
        config: &AdapterConfig,
        fast: bool,
    ) -> Result<RunnerObservation, AdapterError>;
}

pub trait CoverageAdapter: Send + Sync {
    fn descriptor(&self) -> AdapterDescriptor;
}

pub struct AdapterRegistration {
    pub descriptor: AdapterDescriptor,
    pub discovery: Option<Arc<dyn SourceDiscoveryAdapter>>,
    pub wire_codec: Option<Arc<dyn TestWireCodec>>,
    pub static_audit: Option<Arc<dyn StaticAuditAdapter>>,
    pub structured_test: Option<Arc<dyn StructuredTestAdapter>>,
    pub runner: Option<Arc<dyn TestRunnerAdapter>>,
    pub coverage: Option<Arc<dyn CoverageAdapter>>,
}

impl AdapterRegistration {
    pub fn new(descriptor: AdapterDescriptor) -> Self {
        Self {
            descriptor,
            discovery: None,
            wire_codec: None,
            static_audit: None,
            structured_test: None,
            runner: None,
            coverage: None,
        }
    }

    pub fn with_discovery(mut self, adapter: Arc<dyn SourceDiscoveryAdapter>) -> Self {
        self.discovery = Some(adapter);
        self
    }

    pub fn with_wire_codec(mut self, adapter: Arc<dyn TestWireCodec>) -> Self {
        self.wire_codec = Some(adapter);
        self
    }

    pub fn with_static_audit(mut self, adapter: Arc<dyn StaticAuditAdapter>) -> Self {
        self.static_audit = Some(adapter);
        self
    }

    pub fn with_structured_test(mut self, adapter: Arc<dyn StructuredTestAdapter>) -> Self {
        self.structured_test = Some(adapter);
        self
    }

    pub fn with_runner(mut self, adapter: Arc<dyn TestRunnerAdapter>) -> Self {
        self.runner = Some(adapter);
        self
    }

    pub fn with_coverage(mut self, adapter: Arc<dyn CoverageAdapter>) -> Self {
        self.coverage = Some(adapter);
        self
    }
}

#[derive(Default)]
pub struct AdapterRegistry {
    adapters: BTreeMap<AdapterId, AdapterRegistration>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, registration: AdapterRegistration) -> Result<(), AdapterError> {
        let id = registration.descriptor.id.clone();
        if id.as_str().trim().is_empty()
            || registration.descriptor.config_namespace.trim().is_empty()
        {
            return Err(AdapterError::new(
                "E-ADAPTER-001",
                "adapter id and config namespace must be non-empty",
            ));
        }
        if self.adapters.contains_key(&id) {
            return Err(AdapterError::new(
                "E-ADAPTER-001",
                format!("adapter `{id}` is already registered"),
            ));
        }
        for capability in Capability::ALL {
            let declared = registration.descriptor.supports(capability);
            let implemented = match capability {
                Capability::Discovery => registration.discovery.is_some(),
                Capability::TestWireCodec => registration.wire_codec.is_some(),
                Capability::StaticAudit => registration.static_audit.is_some(),
                Capability::StructuredTest => registration.structured_test.is_some(),
                Capability::Runner => registration.runner.is_some(),
                Capability::Coverage => registration.coverage.is_some(),
            };
            if declared != implemented {
                return Err(AdapterError::new(
                    "E-ADAPTER-001",
                    format!(
                        "adapter `{id}` capability `{}` declaration does not match implementation",
                        capability.as_str()
                    ),
                ));
            }
        }
        self.adapters.insert(id, registration);
        Ok(())
    }

    pub fn get(&self, id: &AdapterId) -> Option<&AdapterRegistration> {
        self.adapters.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AdapterId, &AdapterRegistration)> {
        self.adapters.iter()
    }

    pub fn require(
        &self,
        id: &AdapterId,
        capability: Capability,
    ) -> Result<&AdapterRegistration, AdapterError> {
        let Some(adapter) = self.adapters.get(id) else {
            return Err(AdapterError::new(
                "E-ADAPTER-001",
                format!("adapter `{id}` is not registered"),
            ));
        };
        if !adapter.descriptor.supports(capability) {
            return Err(AdapterError::new(
                "E-ADAPTER-004",
                format!("adapter `{id}` does not provide `{}`", capability.as_str()),
            )
            .capability(capability));
        }
        Ok(adapter)
    }

    pub fn ids(&self) -> impl Iterator<Item = &AdapterId> {
        self.adapters.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_rejects_duplicate_ids_and_missing_capabilities() {
        let descriptor = AdapterDescriptor::new("synthetic", ["fixture"])
            .with_namespace("synthetic")
            .with_capabilities([Capability::Discovery]);
        let mut registry = AdapterRegistry::new();
        let error = registry.register(AdapterRegistration::new(descriptor.clone()));
        assert!(
            error.is_err(),
            "declaration without implementation must fail closed"
        );
        let error = error.unwrap_err();
        assert_eq!(error.code, "E-ADAPTER-001");

        let descriptor =
            AdapterDescriptor::new("synthetic", ["fixture"]).with_namespace("synthetic");
        registry
            .register(AdapterRegistration::new(descriptor.clone()))
            .unwrap();
        let duplicate = registry.register(AdapterRegistration::new(descriptor));
        assert!(duplicate.is_err(), "duplicate adapter must be rejected");
        let duplicate = duplicate.unwrap_err();
        assert_eq!(duplicate.code, "E-ADAPTER-001");
        let missing = match registry.require(&AdapterId::from("synthetic"), Capability::Runner) {
            Ok(_) => panic!("explicit operation capability is required"),
            Err(error) => error,
        };
        assert_eq!(missing.code, "E-ADAPTER-004");
    }
}
