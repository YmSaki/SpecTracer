//! The `go` adapter (BD-350, BD-357).
//!
//! Responsibility: own everything Go-specific that is not the lexical reading
//! of Go source. BD-357 gives the `go` adapter its discovery, static audit,
//! runner and coverage; AGENTS.md 9.2 places the toolchain parts in this crate,
//! as `vtest-adapter-rust` does for `rust-cargo`. This crate declares
//! `source_discovery`, `test_runner` and `coverage`: `go test` as the runner
//! (DES-621, DES-647 to DES-652, DES-677, DES-678), its Execution State (DES-623
//! to DES-632, DES-680) and its coverage, `go-cover` (BD-354, DES-663 to
//! DES-679).
//!
//! Boundary: discovery is delegated to `vtest_adapter_lexical::go`, which keeps
//! the `@vtest.` grammar and the Go syntax in one place (AGENTS.md 4.2, 11); this
//! is the one adapter-to-adapter dependency and it is for discovery and for
//! placing a target in its file only. What this crate adds to a discovered Test
//! is its execution coordinates, which need the Go module the Test is in
//! (DES-643 to DES-645, DES-684). The static audit (DES-619) is not provided, so
//! verification reports `oracle_presence` as `NO_EVIDENCE` (`NOT_CHECKED`) for a
//! Go Test, whatever its Evidence says; `target_binding` can reach `PASS` only
//! through the coverage of the Test's own run (DES-676, DS-683).

mod coverage;
mod execution_state;
mod module;
mod runner;
mod syntax;

use std::path::Path;

use vtest_adapter_api::{
    Adapter, AdapterDescriptor, AdapterScanConfig, Capability, CoverageAdapter, DiscoveryError,
    DiscoveryOutcome, SourceDiscoveryAdapter, TestRunnerAdapter, COVERAGE_OFF,
};

pub use coverage::{GoCoverAdapter, COVERAGE_METHOD};
pub use runner::{GoTestRunner, COVERAGE_RUNNER_KIND, RUNNER_KIND};
pub use vtest_adapter_lexical::go::ADAPTER_ID;

/// DS-1879: the values of `run.coverage` for `go`: the measurement `go-cover`
/// (BD-354) or none.
const RUN_COVERAGE_VALUES: &[&str] = &["go-cover", COVERAGE_OFF];

/// DES-644: the `suite.kind` of a Go Test, the package it is in.
pub(crate) const PACKAGE_SUITE: &str = "package";

/// The `go` adapter.
#[derive(Clone, Debug, Default)]
pub struct GoAdapter {
    discovery: vtest_adapter_lexical::go::GoAdapter,
    runner: GoTestRunner,
    coverage: GoCoverAdapter,
}

/// The `go` adapter the composition root registers.
#[must_use]
pub fn adapter() -> GoAdapter {
    GoAdapter::default()
}

impl GoAdapter {
    /// The adapter with the given runner, for tests that start `go` under an
    /// environment of their own (see [`GoTestRunner::with_environment`]).
    #[must_use]
    pub fn with_runner(runner: GoTestRunner) -> Self {
        Self {
            discovery: vtest_adapter_lexical::go::GoAdapter,
            // The module path is read by the `go` the runs were started with.
            coverage: GoCoverAdapter::with_environment(runner.environment.clone()),
            runner,
        }
    }
}

impl Adapter for GoAdapter {
    fn descriptor(&self) -> AdapterDescriptor {
        AdapterDescriptor {
            id: ADAPTER_ID.to_owned(),
            languages: vec!["go".to_owned()],
            capabilities: vec![
                Capability::SourceDiscovery,
                Capability::TestRunner,
                Capability::Coverage,
            ],
            config_namespace: ADAPTER_ID.to_owned(),
        }
    }

    fn as_source_discovery(&self) -> Option<&dyn SourceDiscoveryAdapter> {
        Some(self)
    }

    fn as_test_runner(&self) -> Option<&dyn TestRunnerAdapter> {
        Some(&self.runner)
    }

    fn as_coverage(&self) -> Option<&dyn CoverageAdapter> {
        Some(&self.coverage)
    }

    fn run_coverage_values(&self) -> Option<&'static [&'static str]> {
        Some(RUN_COVERAGE_VALUES)
    }
}

impl SourceDiscoveryAdapter for GoAdapter {
    fn id(&self) -> &'static str {
        ADAPTER_ID
    }

    /// The lexical `go` discovery, with each Test's execution coordinates set
    /// from the Go module its file is in (DES-643 to DES-645, DES-684, DES-687).
    fn discover(
        &self,
        root: &Path,
        fallback_package: &str,
        config: &AdapterScanConfig,
    ) -> Result<DiscoveryOutcome, DiscoveryError> {
        let mut outcome = self.discovery.discover(root, fallback_package, config)?;
        for test in &mut outcome.tests {
            test.execution =
                module::execution_for(root, test.location.path.as_str(), &test.execution.selector);
        }
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests;
