//! Language-neutral scan orchestration.
//!
//! The Rust implementation lives in `vtest-adapter-rust`; this crate selects
//! registered discovery capabilities, merges their results deterministically,
//! and retains the v0.1 public scan API for callers.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use vtest_adapter_api::{AdapterConfig, AdapterRegistry, Capability};
pub use vtest_adapter_rust::operations::*;
pub use vtest_adapter_rust::{
    AuditState, EvidenceState, TestMutationResult, TestSelection, TestView,
};
pub use vtest_adapter_rust::{ScanError, ScanResult};
pub use vtest_model::{AdapterId, SourceFunction, TestEntity};
use vtest_store::load_config;
pub use vtest_store::ProjectConfig;

/// Scan using the built-in Rust/Cargo adapter, preserving the v0.1 API.
pub fn scan_project(root: &Path) -> Result<ScanResult, ScanError> {
    let config = load_config(root)?;
    let registry = vtest_adapter_rust::registry().map_err(|error| ScanError::Adapter {
        code: error.code,
        message: error.message,
    })?;
    validate_config_adapters(&config, &registry)?;
    scan_project_with_registry(root, &registry)
}

pub fn scan_project_with_config(
    root: &Path,
    config: &ProjectConfig,
) -> Result<ScanResult, ScanError> {
    let registry = vtest_adapter_rust::registry().map_err(|error| ScanError::Adapter {
        code: error.code,
        message: error.message,
    })?;
    validate_config_adapters(config, &registry)?;
    vtest_adapter_rust::discovery::scan_project_with_config(root, config)
}

fn validate_config_adapters(
    config: &ProjectConfig,
    registry: &AdapterRegistry,
) -> Result<(), ScanError> {
    for configured in &config.adapters {
        let id = AdapterId::from(configured.id.clone());
        if registry.get(&id).is_none() {
            return Err(ScanError::Adapter {
                code: "E-ADAPTER-001".to_owned(),
                message: format!("configured adapter `{}` is not registered", configured.id),
            });
        }
    }
    Ok(())
}

/// Run all registered discovery adapters and merge their results by adapter ID,
/// path, and Test ID.  Duplicate Test IDs are an error and an empty registry is
/// never treated as a successful empty scan.
pub fn scan_project_with_registry(
    root: &Path,
    registry: &AdapterRegistry,
) -> Result<ScanResult, ScanError> {
    if registry.is_empty() {
        return Err(ScanError::Adapter {
            code: "E-ADAPTER-001".to_owned(),
            message: "no adapters are registered for discovery".to_owned(),
        });
    }
    let config = AdapterConfig::default();
    let mut merged_tests = Vec::new();
    let mut merged_sources = Vec::new();
    let mut diagnostics = Vec::new();
    let mut adapters = Vec::new();
    let mut by_test_id = BTreeSet::new();
    let mut summary = vtest_model::ScanSummary {
        files: 0,
        tests: 0,
        sources: 0,
    };
    let mut discovered = false;
    for (id, registration) in registry.iter() {
        let Some(adapter) = registration.discovery.as_ref() else {
            continue;
        };
        discovered = true;
        let result = adapter
            .discover(root, &config)
            .map_err(|error| ScanError::Adapter {
                code: error.code,
                message: error.message,
            })?;
        summary.files += result.summary.files;
        summary.tests += result.summary.tests;
        summary.sources += result.summary.sources;
        adapters.push(registration.descriptor.clone());
        for test in result.tests {
            if !by_test_id.insert(test.id.clone()) {
                return Err(ScanError::Adapter {
                    code: "E-ADAPTER-003".to_owned(),
                    message: format!("duplicate Test ID `{}` returned by adapter `{id}`", test.id),
                });
            }
            merged_tests.push(test);
        }
        merged_sources.extend(result.sources);
        diagnostics.extend(result.diagnostics);
    }
    if !discovered {
        return Err(ScanError::Adapter {
            code: "E-ADAPTER-001".to_owned(),
            message: "no registered adapter provides discovery".to_owned(),
        });
    }
    merged_tests.sort_by(|left, right| {
        left.execution
            .adapter
            .cmp(&right.execution.adapter)
            .then_with(|| left.location.file.cmp(&right.location.file))
            .then_with(|| left.id.cmp(&right.id))
    });
    merged_sources.sort_by(|left, right| left.locator.as_string().cmp(&right.locator.as_string()));
    let mut seen_sources = BTreeSet::new();
    merged_sources.retain(|source| seen_sources.insert(source.locator.as_string()));
    summary.tests = merged_tests.len();
    summary.sources = merged_sources.len();
    adapters.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(ScanResult {
        summary,
        tests: merged_tests,
        sources: merged_sources,
        diagnostics,
        adapters,
    })
}

/// Return a deterministic map of the capabilities available to each adapter.
pub fn capability_map(registry: &AdapterRegistry) -> BTreeMap<AdapterId, Vec<String>> {
    registry
        .iter()
        .map(|(id, registration)| {
            let capabilities = Capability::ALL
                .into_iter()
                .filter(|capability| registration.has_capability(*capability))
                .map(|capability| capability.as_str().to_owned())
                .collect();
            (id.clone(), capabilities)
        })
        .collect()
}
