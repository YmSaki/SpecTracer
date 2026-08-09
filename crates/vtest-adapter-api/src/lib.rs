//! Language- and runner-neutral adapter contracts.
//!
//! This crate intentionally contains no Cargo, Rust parser, coverage, or
//! process-specific types.  Concrete adapters implement the capability traits
//! and are composed by the orchestration crates.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Arc,
};

use serde::{Deserialize, Serialize};
pub use vtest_model::AdapterId;
use vtest_model::{
    CheckValue, Diagnostic, EvidenceRecord, RunnerInfo, ScanSummary, SourceFunction,
    TargetExecution, TestEntity, TestResult,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    Discovery,
    StaticAudit,
    StructuredTest,
    Runner,
    Coverage,
}

impl Capability {
    pub const ALL: [Self; 5] = [
        Self::Discovery,
        Self::StaticAudit,
        Self::StructuredTest,
        Self::Runner,
        Self::Coverage,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Discovery => "discovery",
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
pub struct AdapterError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiscoveryResult {
    pub adapter: AdapterId,
    pub summary: ScanSummary,
    pub tests: Vec<TestEntity>,
    pub sources: Vec<SourceFunction>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StaticAuditResult {
    pub verdict: CheckValue,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StructuredTestResult {
    pub test: Option<TestEntity>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunnerObservation {
    pub result: TestResult,
    pub runner: RunnerInfo,
    pub target_execution: TargetExecution,
    pub log: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunnerResult {
    pub observation: RunnerObservation,
    pub evidence: Option<EvidenceRecord>,
    pub diagnostics: Vec<Diagnostic>,
}

pub trait SourceDiscoveryAdapter: Send + Sync {
    fn descriptor(&self) -> AdapterDescriptor;
    fn discover(
        &self,
        root: &Path,
        config: &AdapterConfig,
    ) -> Result<DiscoveryResult, AdapterError>;
}

pub trait StaticAuditAdapter: Send + Sync {
    fn descriptor(&self) -> AdapterDescriptor;
    fn audit(
        &self,
        root: &Path,
        test: &TestEntity,
        config: &AdapterConfig,
    ) -> Result<StaticAuditResult, AdapterError>;
}

pub trait StructuredTestAdapter: Send + Sync {
    fn descriptor(&self) -> AdapterDescriptor;
}

pub trait TestRunnerAdapter: Send + Sync {
    fn descriptor(&self) -> AdapterDescriptor;
    fn run(
        &self,
        root: &Path,
        test: &TestEntity,
        config: &AdapterConfig,
        fast: bool,
    ) -> Result<RunnerResult, AdapterError>;
}

pub trait CoverageAdapter: Send + Sync {
    fn descriptor(&self) -> AdapterDescriptor;
}

pub struct AdapterRegistration {
    pub descriptor: AdapterDescriptor,
    pub discovery: Option<Arc<dyn SourceDiscoveryAdapter>>,
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

    pub fn has_capability(&self, capability: Capability) -> bool {
        self.descriptor.supports(capability)
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

    pub fn ids(&self) -> impl Iterator<Item = &AdapterId> {
        self.adapters.keys()
    }

    pub fn len(&self) -> usize {
        self.adapters.len()
    }

    pub fn is_empty(&self) -> bool {
        self.adapters.is_empty()
    }

    pub fn find_capability(&self, capability: Capability) -> Vec<&AdapterRegistration> {
        self.adapters
            .values()
            .filter(|adapter| adapter.has_capability(capability))
            .collect()
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
        if !adapter.has_capability(capability) {
            return Err(AdapterError::new(
                "W-ADAPTER-101",
                format!("adapter `{id}` does not provide `{}`", capability.as_str()),
            )
            .capability(capability));
        }
        Ok(adapter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_rejects_duplicate_ids_and_keeps_sorted_iteration() {
        let descriptor =
            AdapterDescriptor::new("synthetic", ["fixture"]).with_namespace("synthetic");
        let mut registry = AdapterRegistry::new();
        registry
            .register(AdapterRegistration::new(descriptor.clone()))
            .unwrap();
        let error = registry
            .register(AdapterRegistration::new(descriptor))
            .unwrap_err();
        assert_eq!(error.code, "E-ADAPTER-001");

        let mut registry = AdapterRegistry::new();
        let descriptor_a = AdapterDescriptor::new("z-adapter", ["z"]).with_namespace("z");
        let descriptor_b = AdapterDescriptor::new("a-adapter", ["a"]).with_namespace("a");
        registry
            .register(AdapterRegistration::new(descriptor_a))
            .unwrap();
        registry
            .register(AdapterRegistration::new(descriptor_b))
            .unwrap();
        let ids = registry.ids().map(AdapterId::as_str).collect::<Vec<_>>();
        assert_eq!(ids, ["a-adapter", "z-adapter"]);
    }
}
