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
    hash_execution_state_subject, AdapterId, CanonicalProjection, CheckValue, ContentHash,
    Diagnostic, ExecutionDescriptor, ExecutionInputSubject, ExecutionStateSubjectInput, FormSchema,
    RunnerInfo, SourceLocation, SrcId, TargetExecution, TargetRef, TestEntity, TestId, TestResult,
    VoId,
};

/// Compute the Execution State subject hash of a draft, or `None` when the draft
/// is incomplete (an incomplete closure never yields a fresh subject). This is
/// the single draft -> hash mapping shared by the runner path (which hashes the
/// pre-run snapshot) and the freshness re-derivation path (which hashes the
/// current reconstruction), so the two can never disagree on field encoding.
pub fn hash_execution_state_draft(
    adapter: &AdapterId,
    draft: &ExecutionStateDraft,
) -> Option<ContentHash> {
    if !draft.complete {
        return None;
    }
    let inputs = draft
        .inputs
        .iter()
        .map(|input| ExecutionInputSubject {
            root_identity: &input.root_identity,
            root_relative_path: &input.root_relative_path,
            kind: &input.kind,
            bytes: &input.bytes,
        })
        .collect::<Vec<_>>();
    Some(hash_execution_state_subject(&ExecutionStateSubjectInput {
        adapter,
        schema_id: &draft.schema_id,
        schema_version: &draft.schema_version,
        head_revision: draft.head_revision.as_deref(),
        runner_kind: &draft.runner_kind,
        invocation: &draft.invocation,
        toolchain_identity: &draft.toolchain_identity,
        effective_config: &draft.effective_config,
        inputs: &inputs,
    }))
}

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
/// A per-target verdict for a target-scoped rule (DA-002 / DA-003).  The adapter
/// returns one entry per declared target *before* the core folds them into the
/// rule-level `verdict`; the core maps `target` to its canonical Locator for the
/// persisted record (詳細設計 §3.6, §7.1, §7.2).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuleTargetVerdictDraft {
    pub target: TargetRef,
    pub verdict: CheckValue,
    pub reason: String,
    pub location: SourceLocation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuleObservationDraft {
    pub rule: String,
    pub verdict: CheckValue,
    pub reason: String,
    pub location: SourceLocation,
    /// Per-target verdicts for the target-scoped rules DA-002 / DA-003; empty
    /// for non-target-scoped rules.  The rule-level `verdict` above is the pure
    /// static fold of these (§7.2).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<RuleTargetVerdictDraft>,
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

/// Adapter-neutral desired state for a Structured Edit, produced by core from
/// validated form answers / `--set` values / the current Test (詳細設計 §8.1).
/// Every field is a LOGICAL value; only the owning `StructuredTestAdapter`
/// renders it into its own declaration syntax (基本仕様 §6.1, 別紙A §15).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StructuredEditFields {
    pub id: String,
    pub covers: Vec<String>,
    pub targets: Vec<String>,
    pub intent: String,
    pub input: Option<String>,
    pub expect: Option<String>,
    pub kind: Option<String>,
    pub cases: Vec<String>,
    pub related: Vec<String>,
    pub fn_name: String,
    pub file: String,
}

pub trait StructuredTestAdapter: Send + Sync {
    /// The Form `kind`s this adapter owns unconditionally, independent of any
    /// declared `adapter` field (別紙A §14.3's built-in forms).
    fn built_in_form_kinds(&self) -> Vec<String>;

    /// Deterministic acceptance of a Form Schema that declares no `adapter`
    /// (a read-compatible legacy Form). Must decide from schema CONTENT --
    /// never from `kind`'s string alone (別紙A §14.2) -- so an unrelated
    /// adapter's Form is not silently claimed.
    fn accepts_compatibility_form(&self, schema: &Value) -> bool;

    /// Render the full replacement text for a Test's extended source range
    /// (declaration + construct) from LOGICAL desired fields only (別紙A
    /// §15.1-§15.4). `current_source` is the Test's current extended range,
    /// already de-indented; `current_id` / `current_selector` are read-only
    /// context for diagnostics and signature relocation. Core never
    /// interprets or emits this adapter's declaration syntax itself -- this
    /// method is the sole authority for the edit path (mirrors the create
    /// path's `render_form_template`, which is already adapter-owned).
    fn render_edit(
        &self,
        current_source: &str,
        current_id: &str,
        current_selector: &str,
        desired: &StructuredEditFields,
        body: Option<&str>,
    ) -> Result<String, Diagnostic>;
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

    /// Reconstruct the CURRENT Execution State draft for `test` under this
    /// adapter's own schema, without running the Test. Verify-time freshness
    /// re-derivation (`evidence_validity`) calls this -- through the adapter
    /// resolved from the record's own `AdapterId` via the registry, never a
    /// hardcoded adapter -- and compares its hash to the recorded one, so a
    /// re-derivation is only ever compared against a subject produced by the
    /// SAME schema (詳細設計 line 810 / 1390).
    ///
    /// `runner_kind` is the persisted `runner.kind` from the Evidence being
    /// re-checked, letting the adapter recover any runner-kind-dependent
    /// projection (e.g. coverage on/off) the same way the original run did.
    ///
    /// The default returns an incomplete draft: an adapter that has not
    /// implemented this cannot prove its current input closure is complete,
    /// so the caller's freshness verdict is UNKNOWN, never a guessed match or
    /// a guessed drift (基本仕様 §7.8).
    fn current_execution_state(
        &self,
        _root: &Path,
        _test: &TestEntity,
        _runner_kind: &str,
    ) -> ExecutionStateDraft {
        ExecutionStateDraft {
            schema_id: String::new(),
            schema_version: String::new(),
            complete: false,
            head_revision: None,
            runner_kind: String::new(),
            invocation: CanonicalProjection::Null,
            toolchain_identity: String::new(),
            effective_config: CanonicalProjection::Null,
            inputs: Vec::new(),
        }
    }
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

/// Resolves the Form `schema` to exactly one registered StructuredTest-capable
/// adapter, per the registry contract (別紙A §14.2-§14.4, 別紙C §18.3.7): a
/// Form declaring `adapter` must name a registered adapter that both carries
/// the StructuredTest capability and owns `kind` (no OTHER adapter's built-in
/// kind set already claims it); a Form with no `adapter` (a read-compatible
/// legacy Form) resolves only if exactly one capable adapter's built-in kind
/// declaration or `accepts_compatibility_form` matcher accepts it. Every
/// rejection path (unknown adapter, missing capability, owner mismatch, 0 or
/// 2+ matches) is returned as `Err` before any caller may mutate a file --
/// core must never fall back to any adapter for an unresolved kind.
pub fn resolve_structured_test_adapter<'a>(
    registry: &'a AdapterRegistry,
    schema: &FormSchema,
) -> Result<(AdapterId, &'a dyn StructuredTestAdapter), AdapterError> {
    let schema_value = serde_json::to_value(schema)
        .map_err(|error| AdapterError::MalformedOutput(error.to_string()))?;
    if let Some(declared) = &schema.adapter {
        let adapter_id = AdapterId::new(declared.as_str());
        let adapter = registry.structured_test(&adapter_id)?;
        let owners = kind_owners(registry, &schema.kind);
        if owners.len() > 1 {
            return Err(AdapterError::Registration(format!(
                "kind `{}` is declared as built-in by multiple adapters: {owners:?}",
                schema.kind
            )));
        }
        if let Some(owner) = owners.first() {
            if owner != &adapter_id {
                return Err(AdapterError::Registration(format!(
                    "form `{}` declares adapter `{declared}` but kind `{}` is owned by `{owner}`",
                    schema.kind, schema.kind
                )));
            }
        }
        return Ok((adapter_id, adapter));
    }
    let mut matches = Vec::new();
    for id in registry.ids() {
        let Ok(adapter) = registry.structured_test(id) else {
            continue;
        };
        if adapter
            .built_in_form_kinds()
            .iter()
            .any(|kind| kind == &schema.kind)
            || adapter.accepts_compatibility_form(&schema_value)
        {
            matches.push((id.clone(), adapter));
        }
    }
    match matches.len() {
        1 => Ok(matches.into_iter().next().expect("length checked above")),
        0 => Err(AdapterError::Registration(format!(
            "form `{}` declares no `adapter` and no registered Structured Test adapter accepts it",
            schema.kind
        ))),
        _ => Err(AdapterError::Registration(format!(
            "form `{}` declares no `adapter` and matches multiple registered Structured Test adapters",
            schema.kind
        ))),
    }
}

/// Every registered StructuredTest-capable adapter that declares `kind` as
/// its own built-in (別紙A §14.3). More than one entry means the registry
/// itself has a duplicate kind declaration (詳細設計 §17.1 E-ADAPTER-001,
/// 別紙C §18.3.7's 「同じkindを複数adapterが宣言する」) -- independent of
/// which adapter any particular Form happens to declare.
fn kind_owners(registry: &AdapterRegistry, kind: &str) -> Vec<AdapterId> {
    registry
        .ids()
        .filter_map(|id| {
            let adapter = registry.structured_test(id).ok()?;
            adapter
                .built_in_form_kinds()
                .iter()
                .any(|candidate| candidate == kind)
                .then(|| id.clone())
        })
        .collect()
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

    /// @vtest.id TEST-AAPI-001
    /// @vtest.covers VO-AAPI-001
    /// @vtest.target crates/vtest-adapter-api/src/lib.rs::AdapterRegistry::register
    /// @vtest.intent Registering two adapters with the same ID is rejected as a duplicate
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

    /// @vtest.id TEST-AAPI-002
    /// @vtest.covers VO-AAPI-002
    /// @vtest.target crates/vtest-adapter-api/src/lib.rs::AdapterRegistry::ids
    /// @vtest.intent Registry iteration and lookup are deterministically ordered by adapter ID
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

    /// @vtest.id TEST-AAPI-003
    /// @vtest.covers VO-AAPI-001
    /// @vtest.target crates/vtest-adapter-api/src/lib.rs::AdapterRegistry::register
    /// @vtest.intent A declared capability with no implementation is rejected at registration
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
