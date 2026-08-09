//! Test-execution orchestration facade.
//!
//! Cargo and coverage command handling is owned by `vtest-adapter-rust`.
//! Unsupported runner capabilities are retained as diagnostics and never
//! produce fabricated Evidence.

use std::path::Path;

use vtest_adapter_api::{AdapterConfig, AdapterRegistry, Capability};
pub use vtest_adapter_rust::execution::{ExecutionError, ExecutionResult, RunnableTest};
use vtest_model::Diagnostic;
use vtest_store::VerifyLayout;

pub fn run_tests(
    root: &Path,
    layout: &VerifyLayout,
    tests: &[RunnableTest],
    fast: bool,
) -> Result<ExecutionResult, ExecutionError> {
    let registry = vtest_adapter_rust::registry().map_err(|error| ExecutionError::Io {
        path: root.to_owned(),
        source: std::io::Error::other(error.message),
    })?;
    run_tests_with_registry(root, layout, tests, fast, &registry)
}

/// Execute through an explicitly composed registry.  This is the extension
/// point used by synthetic and future language adapters; each adapter owns its
/// process and Evidence details.
pub fn run_tests_with_registry(
    root: &Path,
    _layout: &VerifyLayout,
    tests: &[RunnableTest],
    fast: bool,
    registry: &AdapterRegistry,
) -> Result<ExecutionResult, ExecutionError> {
    let mut result = ExecutionResult {
        evidence: Vec::new(),
        diagnostics: Vec::new(),
    };
    let config = AdapterConfig::default();
    for test in tests {
        let adapter_id = &test.entity.execution.adapter;
        let registration = match registry.require(adapter_id, Capability::Runner) {
            Ok(registration) => registration,
            Err(error) => {
                let diagnostic = if error.code.starts_with("E-") {
                    Diagnostic::error(error.code, error.message)
                } else {
                    Diagnostic::warning(error.code, error.message)
                };
                result
                    .diagnostics
                    .push(diagnostic.with_location(test.entity.location.clone()));
                continue;
            }
        };
        let Some(runner) = registration.runner.as_ref() else {
            continue;
        };
        let observation = runner
            .run(root, &test.entity, &config, fast)
            .map_err(|error| ExecutionError::Io {
                path: root.to_owned(),
                source: std::io::Error::other(error.message),
            })?;
        if let Some(evidence) = observation.evidence {
            result.evidence.push(evidence);
        }
        result.diagnostics.extend(observation.diagnostics);
    }
    Ok(result)
}
