//! Built-in Rust/Cargo adapter.
//!
//! All Rust parser, Cargo target resolution, static audit, test runner, and
//! coverage implementation lives in this crate.  The orchestration crates
//! depend on its neutral results and do not own Rust-specific dependencies.

pub mod audit;
pub mod discovery;
pub mod execution;
pub mod forms;
pub mod operations;

pub use audit::*;
pub use discovery::*;
pub use execution::*;
pub use forms::{
    ensure_builtin_forms, load_form_schema, RUST_INTEGRATION_FORM, RUST_UNIT_FUNCTION_FORM,
};
pub use operations::*;

use std::sync::Arc;
use vtest_adapter_api::{
    AdapterDescriptor, AdapterRegistration, AdapterRegistry, Capability, CoverageAdapter,
    RunnerObservation, RunnerResult, SourceDiscoveryAdapter, StaticAuditAdapter, StaticAuditResult,
    StructuredTestAdapter, TestRunnerAdapter,
};

/// Descriptor for the built-in Rust adapter.
pub fn descriptor() -> AdapterDescriptor {
    AdapterDescriptor::new("rust-cargo", ["rust"])
        .with_namespace("rust")
        .with_capabilities([
            Capability::Discovery,
            Capability::StaticAudit,
            Capability::StructuredTest,
            Capability::Runner,
            Capability::Coverage,
        ])
}

/// Compose the built-in adapter in the same registry shape used by CLI and
/// MCP.  The individual capability implementations are intentionally kept in
/// this crate; callers only select them through the neutral registry.
pub fn registration() -> AdapterRegistration {
    let adapter = Arc::new(RustCargoAdapter);
    AdapterRegistration::new(descriptor())
        .with_discovery(adapter.clone())
        .with_static_audit(adapter.clone())
        .with_structured_test(adapter.clone())
        .with_runner(adapter.clone())
        .with_coverage(adapter)
}

pub fn registry() -> Result<AdapterRegistry, vtest_adapter_api::AdapterError> {
    let mut registry = AdapterRegistry::new();
    registry.register(registration())?;
    Ok(registry)
}

/// Marker implementation used to expose the built-in capabilities through the
/// registry.  Rich Rust results remain available through the public functions
/// in the sibling modules for the legacy CLI API.
#[derive(Clone, Copy, Debug, Default)]
pub struct RustCargoAdapter;

impl SourceDiscoveryAdapter for RustCargoAdapter {
    fn descriptor(&self) -> AdapterDescriptor {
        descriptor()
    }

    fn discover(
        &self,
        root: &std::path::Path,
        _config: &vtest_adapter_api::AdapterConfig,
    ) -> Result<vtest_adapter_api::DiscoveryResult, vtest_adapter_api::AdapterError> {
        let result = discovery::scan_project(root).map_err(|error| {
            vtest_adapter_api::AdapterError::new("E-ADAPTER-002", error.to_string())
                .capability(Capability::Discovery)
        })?;
        Ok(vtest_adapter_api::DiscoveryResult {
            adapter: vtest_model::AdapterId::from("rust-cargo"),
            summary: result.summary,
            tests: result.tests,
            sources: result.sources,
            diagnostics: result.diagnostics,
        })
    }
}

impl StaticAuditAdapter for RustCargoAdapter {
    fn descriptor(&self) -> AdapterDescriptor {
        descriptor()
    }

    fn audit(
        &self,
        root: &std::path::Path,
        test: &vtest_model::TestEntity,
        _config: &vtest_adapter_api::AdapterConfig,
    ) -> Result<vtest_adapter_api::StaticAuditResult, vtest_adapter_api::AdapterError> {
        let scan = discovery::scan_project(root).map_err(|error| {
            vtest_adapter_api::AdapterError::new("E-ADAPTER-002", error.to_string())
                .capability(Capability::StaticAudit)
        })?;
        let summary = audit_static(
            root,
            &scan,
            &AuditOptions {
                test_id: Some(test.id.to_string()),
            },
        )
        .map_err(|error| {
            vtest_adapter_api::AdapterError::new("E-ADAPTER-002", error.to_string())
                .capability(Capability::StaticAudit)
        })?;
        let Some(audit) = summary.audits.first() else {
            return Ok(StaticAuditResult {
                verdict: vtest_model::CheckValue::NotChecked,
                diagnostics: vec![vtest_model::Diagnostic::warning(
                    "W-ADAPTER-101",
                    "Rust audit produced no result",
                )],
            });
        };
        let verdict = match audit.verdict {
            AuditVerdict::Pass => vtest_model::CheckValue::Pass,
            AuditVerdict::Fail => vtest_model::CheckValue::Fail,
            AuditVerdict::Unknown => vtest_model::CheckValue::Unknown,
        };
        Ok(StaticAuditResult {
            verdict,
            diagnostics: audit.diagnostics.clone(),
        })
    }
}

impl StructuredTestAdapter for RustCargoAdapter {
    fn descriptor(&self) -> AdapterDescriptor {
        descriptor()
    }
}

impl TestRunnerAdapter for RustCargoAdapter {
    fn descriptor(&self) -> AdapterDescriptor {
        descriptor()
    }

    fn run(
        &self,
        root: &std::path::Path,
        test: &vtest_model::TestEntity,
        _config: &vtest_adapter_api::AdapterConfig,
        fast: bool,
    ) -> Result<vtest_adapter_api::RunnerResult, vtest_adapter_api::AdapterError> {
        let layout = vtest_store::VerifyLayout::new(root);
        let scan = discovery::scan_project(root).map_err(|error| {
            vtest_adapter_api::AdapterError::new("E-ADAPTER-002", error.to_string())
                .capability(Capability::Runner)
        })?;
        let target_hashes = std::iter::once(&test.target)
            .chain(test.additional_targets.iter())
            .filter_map(|target| match target {
                vtest_model::TargetRef::Locator(locator) => scan
                    .sources
                    .iter()
                    .find(|source| source.locator == *locator)
                    .map(|source| source.content_hash.clone()),
                vtest_model::TargetRef::SrcId(src_id) => scan
                    .sources
                    .iter()
                    .find(|source| source.src_id.as_ref() == Some(src_id))
                    .map(|source| source.content_hash.clone()),
            })
            .collect();
        let runnable = RunnableTest {
            entity: test.clone(),
            target_hashes,
            target_locator: match &test.target {
                vtest_model::TargetRef::Locator(locator) => Some(locator.clone()),
                vtest_model::TargetRef::SrcId(_) => None,
            },
        };
        let result = run_tests(root, &layout, &[runnable], fast).map_err(|error| {
            vtest_adapter_api::AdapterError::new("E-ADAPTER-002", error.to_string())
                .capability(Capability::Runner)
        })?;
        let evidence = result.evidence.into_iter().next();
        let Some(evidence_record) = evidence.clone() else {
            return Ok(RunnerResult {
                observation: RunnerObservation {
                    result: vtest_model::TestResult::Fail,
                    runner: vtest_model::RunnerInfo {
                        kind: "cargo-test".to_owned(),
                        command: String::new(),
                        exit_code: -1,
                    },
                    target_execution: vtest_model::TargetExecution {
                        checked: false,
                        method: None,
                        result: vtest_model::CheckValue::NotExecuted,
                        count: None,
                    },
                    log: String::new(),
                },
                evidence: None,
                diagnostics: result.diagnostics,
            });
        };
        Ok(RunnerResult {
            observation: RunnerObservation {
                result: evidence_record.result,
                runner: evidence_record.runner.clone(),
                target_execution: evidence_record.target_execution.clone(),
                log: evidence_record.log_ref.clone(),
            },
            evidence,
            diagnostics: result.diagnostics,
        })
    }
}

impl CoverageAdapter for RustCargoAdapter {
    fn descriptor(&self) -> AdapterDescriptor {
        descriptor()
    }
}
