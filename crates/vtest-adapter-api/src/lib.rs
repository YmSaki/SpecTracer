//! Language- and runner-neutral contracts implemented by vtest adapters.
//!
//! Adapters report observations and un-hashed DTOs.  Validation, canonical
//! subject hashing, record materialization, and aggregate verdicts remain
//! responsibilities of the core crates.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{collections::BTreeMap, path::Path, sync::Arc};
use thiserror::Error;
use vtest_model::{
    AdapterId, CanonicalProjection, CheckValue, Diagnostic, ExecutionDescriptor, RunnerInfo,
    SourceLocation, SrcId, TargetExecution, TargetRef, TestEntity, TestId, TestResult, VoId,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceFragment {
    pub location: SourceLocation,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ManagedTestDraft {
    pub id: TestId,
    pub covers: Vec<VoId>,
    pub targets: Vec<TargetRef>,
    pub intent: String,
    pub input: Option<String>,
    pub expect: Option<String>,
    pub kind: Option<String>,
    pub cases: Vec<String>,
    pub related: Vec<TestId>,
    pub execution: ExecutionDescriptor,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiscoveredTestDraft {
    pub adapter: AdapterId,
    pub location: SourceLocation,
    pub construct: SourceFragment,
    pub metadata_sources: Vec<SourceFragment>,
    pub managed: ManagedTestDraftLink,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)] // Normative API requires One(ManagedTestDraft) by value.
pub enum ManagedTestDraftLink {
    Missing,
    One(ManagedTestDraft),
    Multiple(Vec<ManagedTestDraft>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceTargetDraft {
    /// The canonical Target Reference, always a locator. `TargetRef::SrcId` is
    /// how something refers to a Source Target, never the Source Target's own
    /// identity.
    pub target: TargetRef,
    /// An optional permanent identity for the same Source Target. It is not a
    /// second entity and never enters the Source Target subject.
    pub src_id: Option<SrcId>,
    pub location: SourceLocation,
    pub construct: SourceFragment,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryCompleteness {
    Complete,
    Incomplete,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiscoveryBatch {
    pub adapter: AdapterId,
    pub completeness: DiscoveryCompleteness,
    pub discovered_tests: Vec<DiscoveredTestDraft>,
    pub source_targets: Vec<SourceTargetDraft>,
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
    pub effective_config: CanonicalProjection,
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
    pub invocation: CanonicalProjection,
    pub toolchain_identity: String,
    pub effective_config: CanonicalProjection,
    pub inputs: Vec<ExecutionInputDraft>,
}

/// One deterministic rule outcome inside a static audit observation.
///
/// The adapter owns the language-specific analysis, so per-rule verdicts,
/// human-readable reasons, and source locations can only originate here.  The
/// core reshapes these into the persisted `AuditRecord` reasons (rule / verdict
/// / claim / basis) and the CLI `data.audits[].rules[]` projection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuleObservationDraft {
    pub rule: String,
    pub verdict: CheckValue,
    pub reason: String,
    pub location: SourceLocation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StaticAuditObservation {
    pub verdict: CheckValue,
    pub reasons: Vec<String>,
    pub rules: Vec<RuleObservationDraft>,
    pub config: StaticAuditConfigDraft,
    pub analysis: StaticAnalysisClosureDraft,
}

/// A runner result before the core creates an Evidence record.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunnerObservation {
    pub result: TestResult,
    pub runner: RunnerInfo,
    pub target_execution: TargetExecution,
    pub execution_state: ExecutionStateDraft,
    pub log: Vec<u8>,
}

/// The outcome of running one Test. `TestResult` only expresses PASS/FAIL, so
/// the non-result cases the runner must still report (a skipped test, or a
/// requested filter that produced no result line) are separate variants. Every
/// variant carries the raw log because the core owns Evidence/log persistence.
/// The core turns `Ignored` into no Evidence and `MissingResult` into
/// E-EXEC-001/002 (discriminated by the exit code); a PASS/FAIL that disagrees
/// with the process exit code is E-EXEC-003, also decided core-side.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RunnerOutcome {
    Completed(Box<RunnerObservation>),
    Ignored { runner: RunnerInfo, log: Vec<u8> },
    MissingResult { runner: RunnerInfo, log: Vec<u8> },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterCapability {
    SourceDiscovery,
    TestWireCodec,
    StaticAudit,
    StructuredTest,
    TestRunner,
    Coverage,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AdapterDescriptor {
    pub id: AdapterId,
    pub languages: Vec<String>,
    pub capabilities: Vec<AdapterCapability>,
    pub config_namespace: String,
}

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("E-ADAPTER-001: duplicate or inconsistent adapter registration: {0}")]
    Registration(String),
    #[error("E-ADAPTER-002: malformed adapter output: {0}")]
    MalformedOutput(String),
    #[error("E-ADAPTER-003: adapter mismatch: {0}")]
    Mismatch(String),
    #[error("E-ADAPTER-004: adapter capability is unavailable: {0}")]
    MissingCapability(String),
    #[error("adapter operation failed: {0}")]
    Operation(String),
}

pub trait SourceDiscoveryAdapter: Send + Sync {
    fn discover(
        &self,
        root: &Path,
        config: &CanonicalProjection,
    ) -> Result<DiscoveryBatch, AdapterError>;
}

/// Converts adapter-owned compatibility properties to and from the neutral
/// execution coordinate.  Adapter properties never enter `TestEntity`.
pub trait TestWireCodec: Send + Sync {
    fn decode_execution(
        &self,
        execution: Option<&ExecutionDescriptor>,
        properties: &Map<String, Value>,
    ) -> Result<ExecutionDescriptor, AdapterError>;

    fn encode_properties(&self, test: &TestEntity) -> Result<Map<String, Value>, AdapterError>;
}

pub fn normalize_wire_targets(
    targets: Option<Vec<TargetRef>>,
    compatibility_target: Option<TargetRef>,
) -> Result<Vec<TargetRef>, AdapterError> {
    let targets = match (targets, compatibility_target) {
        (None, None) => {
            return Err(AdapterError::MalformedOutput(
                "Test wire input has no target".to_owned(),
            ))
        }
        (None, Some(target)) => vec![target],
        (Some(targets), None) => targets,
        (Some(targets), Some(target)) if targets.len() == 1 && targets[0] == target => targets,
        (Some(_), Some(_)) => {
            return Err(AdapterError::MalformedOutput(
                "targets and compatibility target are inconsistent".to_owned(),
            ))
        }
    };
    if targets.is_empty() {
        return Err(AdapterError::MalformedOutput(
            "Test wire targets must not be empty".to_owned(),
        ));
    }
    let unique = targets
        .iter()
        .map(TargetRef::normalized)
        .collect::<std::collections::BTreeSet<_>>();
    if unique.len() != targets.len() {
        return Err(AdapterError::MalformedOutput(
            "Test wire targets contain duplicates".to_owned(),
        ));
    }
    Ok(targets)
}

pub fn encode_wire_targets(targets: &[TargetRef]) -> Result<Map<String, Value>, AdapterError> {
    let normalized = normalize_wire_targets(Some(targets.to_vec()), None)?;
    let mut output = Map::new();
    output.insert(
        "targets".to_owned(),
        serde_json::to_value(&normalized)
            .map_err(|error| AdapterError::Operation(error.to_string()))?,
    );
    if let [target] = normalized.as_slice() {
        output.insert(
            "target".to_owned(),
            serde_json::to_value(target)
                .map_err(|error| AdapterError::Operation(error.to_string()))?,
        );
    }
    Ok(output)
}

/// Serialize a neutral Test with adapter-owned compatibility properties.
pub fn encode_test_wire(
    test: &TestEntity,
    codec: &dyn TestWireCodec,
) -> Result<Value, AdapterError> {
    test.validate().map_err(AdapterError::MalformedOutput)?;
    let mut object = serde_json::to_value(test)
        .map_err(|error| AdapterError::Operation(error.to_string()))?
        .as_object()
        .cloned()
        .ok_or_else(|| AdapterError::Operation("Test serialization is not an object".to_owned()))?;
    for (key, value) in encode_wire_targets(&test.targets)? {
        object.insert(key, value);
    }
    for (key, value) in codec.encode_properties(test)? {
        if is_core_test_field(&key) {
            return Err(AdapterError::Mismatch(format!(
                "adapter codec attempted to overwrite core Test field `{key}`"
            )));
        }
        object.insert(key, value);
    }
    Ok(Value::Object(object))
}

/// Normalize a current or compatibility Test JSON object into the neutral model.
pub fn decode_test_wire(
    value: Value,
    codec: &dyn TestWireCodec,
) -> Result<TestEntity, AdapterError> {
    let mut object = value.as_object().cloned().ok_or_else(|| {
        AdapterError::MalformedOutput("Test wire input must be an object".to_owned())
    })?;
    let targets = object
        .remove("targets")
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| AdapterError::MalformedOutput(error.to_string()))?;
    let target = object
        .remove("target")
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| AdapterError::MalformedOutput(error.to_string()))?;
    let targets = normalize_wire_targets(targets, target)?;
    let execution = object
        .remove("execution")
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| AdapterError::MalformedOutput(error.to_string()))?;
    let property_keys = object
        .keys()
        .filter(|key| !is_core_test_field(key))
        .cloned()
        .collect::<Vec<_>>();
    let properties = property_keys
        .into_iter()
        .map(|key| {
            let value = object
                .remove(&key)
                .expect("property key was collected from this object");
            (key, value)
        })
        .collect::<Map<_, _>>();
    let execution = codec.decode_execution(execution.as_ref(), &properties)?;
    object.insert(
        "targets".to_owned(),
        serde_json::to_value(targets)
            .map_err(|error| AdapterError::Operation(error.to_string()))?,
    );
    object.insert(
        "execution".to_owned(),
        serde_json::to_value(execution)
            .map_err(|error| AdapterError::Operation(error.to_string()))?,
    );
    let test: TestEntity = serde_json::from_value(Value::Object(object))
        .map_err(|error| AdapterError::MalformedOutput(error.to_string()))?;
    test.validate().map_err(AdapterError::MalformedOutput)?;
    Ok(test)
}

fn is_core_test_field(key: &str) -> bool {
    matches!(
        key,
        "id" | "covers"
            | "targets"
            | "target"
            | "intent"
            | "input"
            | "expect"
            | "kind"
            | "cases"
            | "related"
            | "location"
            | "content_hash"
            | "execution"
    )
}

pub trait StaticAuditAdapter: Send + Sync {
    /// Audit one Test's deterministic rules.
    ///
    /// `root` locates the project on disk so the adapter can read the Test and
    /// its resolved target sources; `config` is the core-loaded canonical
    /// projection so the adapter never re-parses `.verify/config.yaml` itself
    /// (adapters are store-free). This mirrors `SourceDiscoveryAdapter::discover`,
    /// the only capability with a real implementor today.
    fn audit(
        &self,
        root: &Path,
        config: &CanonicalProjection,
        test: &TestEntity,
    ) -> Result<StaticAuditObservation, AdapterError>;
}

pub trait StructuredTestAdapter: Send + Sync {
    fn built_in_form_kinds(&self) -> Vec<String>;
    fn accepts_compatibility_form(&self, schema: &Value) -> bool;
}

pub trait TestRunnerAdapter: Send + Sync {
    /// Run one Test and report a hash-free runner observation.
    ///
    /// `root` locates the project on disk; `config` is the core-loaded
    /// canonical projection of the execution-affecting configuration, so the
    /// adapter never re-parses `.verify/config.yaml` itself (adapters are
    /// store-free). This mirrors `SourceDiscoveryAdapter::discover` and
    /// `StaticAuditAdapter::audit`.
    fn run(
        &self,
        root: &Path,
        config: &CanonicalProjection,
        test: &TestEntity,
    ) -> Result<RunnerOutcome, AdapterError>;
}

pub trait CoverageAdapter: Send + Sync {
    fn capability_id(&self) -> &str;
}

#[derive(Default)]
pub struct AdapterRegistration {
    pub descriptor: Option<AdapterDescriptor>,
    pub source_discovery: Option<Arc<dyn SourceDiscoveryAdapter>>,
    pub test_wire_codec: Option<Arc<dyn TestWireCodec>>,
    pub static_audit: Option<Arc<dyn StaticAuditAdapter>>,
    pub structured_test: Option<Arc<dyn StructuredTestAdapter>>,
    pub test_runner: Option<Arc<dyn TestRunnerAdapter>>,
    pub coverage: Option<Arc<dyn CoverageAdapter>>,
}

impl AdapterRegistration {
    pub fn new(descriptor: AdapterDescriptor) -> Self {
        Self {
            descriptor: Some(descriptor),
            ..Self::default()
        }
    }

    fn implemented_capabilities(&self) -> Vec<AdapterCapability> {
        let mut values = Vec::new();
        if self.source_discovery.is_some() {
            values.push(AdapterCapability::SourceDiscovery);
        }
        if self.test_wire_codec.is_some() {
            values.push(AdapterCapability::TestWireCodec);
        }
        if self.static_audit.is_some() {
            values.push(AdapterCapability::StaticAudit);
        }
        if self.structured_test.is_some() {
            values.push(AdapterCapability::StructuredTest);
        }
        if self.test_runner.is_some() {
            values.push(AdapterCapability::TestRunner);
        }
        if self.coverage.is_some() {
            values.push(AdapterCapability::Coverage);
        }
        values
    }
}

#[derive(Default)]
pub struct AdapterRegistry {
    registrations: BTreeMap<AdapterId, AdapterRegistration>,
}

impl AdapterRegistry {
    pub fn from_registrations(
        registrations: impl IntoIterator<Item = AdapterRegistration>,
    ) -> Result<Self, AdapterError> {
        let mut registry = Self::default();
        for registration in registrations {
            registry.register(registration)?;
        }
        if registry.registrations.is_empty() {
            return Err(AdapterError::Registration(
                "at least one adapter is required".to_owned(),
            ));
        }
        Ok(registry)
    }

    pub fn register(&mut self, registration: AdapterRegistration) -> Result<(), AdapterError> {
        let descriptor = registration
            .descriptor
            .as_ref()
            .ok_or_else(|| AdapterError::Registration("descriptor is required".to_owned()))?;
        if descriptor.id.as_str().is_empty()
            || descriptor.config_namespace.is_empty()
            || descriptor.languages.is_empty()
        {
            return Err(AdapterError::Registration(format!(
                "adapter `{}` has an incomplete descriptor",
                descriptor.id
            )));
        }
        let mut declared = descriptor.capabilities.clone();
        declared.sort();
        declared.dedup();
        let implemented = registration.implemented_capabilities();
        if declared != implemented {
            return Err(AdapterError::Registration(format!(
                "adapter `{}` declares {declared:?} but implements {implemented:?}",
                descriptor.id
            )));
        }
        let id = descriptor.id.clone();
        if self.registrations.contains_key(&id) {
            return Err(AdapterError::Registration(format!(
                "duplicate adapter ID `{id}`"
            )));
        }
        self.registrations.insert(id, registration);
        Ok(())
    }

    pub fn ids(&self) -> impl Iterator<Item = &AdapterId> {
        self.registrations.keys()
    }

    pub fn descriptor(&self, id: &AdapterId) -> Result<&AdapterDescriptor, AdapterError> {
        self.registration(id)?
            .descriptor
            .as_ref()
            .ok_or_else(|| AdapterError::Registration("descriptor is required".to_owned()))
    }

    pub fn source_discovery(
        &self,
        id: &AdapterId,
    ) -> Result<&dyn SourceDiscoveryAdapter, AdapterError> {
        self.registration(id)?
            .source_discovery
            .as_deref()
            .ok_or_else(|| missing(id, AdapterCapability::SourceDiscovery))
    }

    pub fn test_wire_codec(&self, id: &AdapterId) -> Result<&dyn TestWireCodec, AdapterError> {
        self.registration(id)?
            .test_wire_codec
            .as_deref()
            .ok_or_else(|| missing(id, AdapterCapability::TestWireCodec))
    }

    pub fn static_audit(&self, id: &AdapterId) -> Result<&dyn StaticAuditAdapter, AdapterError> {
        self.registration(id)?
            .static_audit
            .as_deref()
            .ok_or_else(|| missing(id, AdapterCapability::StaticAudit))
    }

    pub fn structured_test(
        &self,
        id: &AdapterId,
    ) -> Result<&dyn StructuredTestAdapter, AdapterError> {
        self.registration(id)?
            .structured_test
            .as_deref()
            .ok_or_else(|| missing(id, AdapterCapability::StructuredTest))
    }

    pub fn test_runner(&self, id: &AdapterId) -> Result<&dyn TestRunnerAdapter, AdapterError> {
        self.registration(id)?
            .test_runner
            .as_deref()
            .ok_or_else(|| missing(id, AdapterCapability::TestRunner))
    }

    pub fn coverage(&self, id: &AdapterId) -> Result<&dyn CoverageAdapter, AdapterError> {
        self.registration(id)?
            .coverage
            .as_deref()
            .ok_or_else(|| missing(id, AdapterCapability::Coverage))
    }

    fn registration(&self, id: &AdapterId) -> Result<&AdapterRegistration, AdapterError> {
        self.registrations
            .get(id)
            .ok_or_else(|| AdapterError::Mismatch(format!("unknown adapter `{id}`")))
    }
}

fn missing(id: &AdapterId, capability: AdapterCapability) -> AdapterError {
    AdapterError::MissingCapability(format!("adapter `{id}` has no {capability:?}"))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MissingCapabilitySemantics {
    NotChecked,
    NotExecuted,
    OperationRejected,
}

pub fn missing_capability_semantics(capability: AdapterCapability) -> MissingCapabilitySemantics {
    match capability {
        AdapterCapability::TestRunner => MissingCapabilitySemantics::NotExecuted,
        AdapterCapability::StaticAudit | AdapterCapability::Coverage => {
            MissingCapabilitySemantics::NotChecked
        }
        AdapterCapability::SourceDiscovery
        | AdapterCapability::TestWireCodec
        | AdapterCapability::StructuredTest => MissingCapabilitySemantics::OperationRejected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor(id: &str) -> AdapterDescriptor {
        AdapterDescriptor {
            id: AdapterId::new(id),
            languages: vec!["fixture".to_owned()],
            capabilities: Vec::new(),
            config_namespace: id.to_owned(),
        }
    }

    #[test]
    fn registry_rejects_duplicate_adapter_ids() {
        let error = AdapterRegistry::from_registrations([
            AdapterRegistration::new(descriptor("synthetic")),
            AdapterRegistration::new(descriptor("synthetic")),
        ])
        .err()
        .expect("duplicate ID is rejected");
        assert!(error.to_string().contains("duplicate adapter ID"));
    }

    #[test]
    fn registry_iteration_and_lookup_are_id_sorted() {
        let registry = AdapterRegistry::from_registrations([
            AdapterRegistration::new(descriptor("zeta")),
            AdapterRegistration::new(descriptor("alpha")),
        ])
        .expect("valid registry");
        assert_eq!(
            registry.ids().map(AdapterId::as_str).collect::<Vec<_>>(),
            ["alpha", "zeta"]
        );
        assert_eq!(
            registry
                .descriptor(&AdapterId::new("alpha"))
                .expect("deterministic lookup")
                .id
                .as_str(),
            "alpha"
        );
    }

    #[test]
    fn registry_rejects_declared_capability_without_implementation() {
        let mut descriptor = descriptor("synthetic");
        descriptor.capabilities = vec![AdapterCapability::TestRunner];
        let error = AdapterRegistry::from_registrations([AdapterRegistration::new(descriptor)])
            .err()
            .expect("capability mismatch is rejected");
        assert!(error.to_string().contains("declares"));
    }
}
