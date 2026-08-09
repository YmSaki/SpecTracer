//! W7 adapter boundary acceptance: a non-Rust synthetic adapter can register
//! and merge without changing the neutral model or Rust parser.

use std::{path::Path, sync::Arc};

use vtest_adapter_api::{
    AdapterConfig, AdapterDescriptor, AdapterRegistration, AdapterRegistry, Capability,
    DiscoveryResult, RunnerObservation, RunnerResult, SourceDiscoveryAdapter, TestRunnerAdapter,
};
use vtest_exec::{run_tests_with_registry, RunnableTest};
use vtest_model::{
    AdapterId, CheckValue, ContentHash, EvidenceHashes, EvidenceRecord, ExecutionDescriptor,
    Locator, Revision, RunnerInfo, ScanSummary, SourceFunction, SourceLocation, TargetExecution,
    TargetRef, TestEntity, TestId, TestResult, TestSuite, TestTarget,
};
use vtest_scan::scan_project_with_registry;
use vtest_store::VerifyLayout;

#[derive(Clone)]
struct SyntheticAdapter {
    id: &'static str,
    test_id: &'static str,
}

impl SourceDiscoveryAdapter for SyntheticAdapter {
    fn descriptor(&self) -> AdapterDescriptor {
        AdapterDescriptor::new(self.id, ["synthetic"])
            .with_namespace("synthetic")
            .with_capabilities([Capability::Discovery])
    }

    fn discover(
        &self,
        _root: &Path,
        _config: &AdapterConfig,
    ) -> Result<DiscoveryResult, vtest_adapter_api::AdapterError> {
        let location = SourceLocation {
            file: format!("tests/{}.decl", self.id),
            function: "test".to_owned(),
            start_line: 1,
            end_line: 1,
            start_byte: 0,
            end_byte: 1,
        };
        let target = Locator {
            path: format!("{}.decl", self.id),
            item_path: "target".to_owned(),
        };
        let test = TestEntity {
            id: TestId::from(self.test_id),
            covers: Vec::new(),
            target: TargetRef::Locator(target.clone()),
            additional_targets: Vec::new(),
            intent: "synthetic adapter boundary".to_owned(),
            input: None,
            expect: None,
            kind: Some("synthetic".to_owned()),
            cases: Vec::new(),
            related: Vec::new(),
            location: location.clone(),
            content_hash: ContentHash::from_text(self.test_id),
            execution: ExecutionDescriptor {
                adapter: AdapterId::from(self.id),
                project: Some("synthetic".to_owned()),
                suite: Some(TestSuite {
                    kind: "fixture".to_owned(),
                    name: None,
                }),
                selector: self.test_id.to_owned(),
            },
            filter: self.test_id.to_owned(),
            package: "synthetic".to_owned(),
            test_target: TestTarget::Unknown,
        };
        Ok(DiscoveryResult {
            adapter: AdapterId::from(self.id),
            summary: ScanSummary {
                files: 1,
                tests: 1,
                sources: 1,
            },
            tests: vec![test],
            sources: vec![SourceFunction {
                locator: target,
                src_id: None,
                location,
                content_hash: ContentHash::from_text("synthetic target"),
            }],
            diagnostics: Vec::new(),
        })
    }
}

fn registry(adapter: SyntheticAdapter) -> AdapterRegistry {
    let descriptor = adapter.descriptor();
    let mut registry = AdapterRegistry::new();
    registry
        .register(AdapterRegistration::new(descriptor).with_discovery(Arc::new(adapter)))
        .unwrap();
    registry
}

struct SyntheticRunner {
    id: &'static str,
}

impl TestRunnerAdapter for SyntheticRunner {
    fn descriptor(&self) -> AdapterDescriptor {
        AdapterDescriptor::new(self.id, ["synthetic"])
            .with_namespace("synthetic")
            .with_capabilities([Capability::Runner])
    }

    fn run(
        &self,
        _root: &Path,
        test: &TestEntity,
        _config: &AdapterConfig,
        _fast: bool,
    ) -> Result<RunnerResult, vtest_adapter_api::AdapterError> {
        let target_hash = ContentHash::from_text("synthetic target");
        let runner = RunnerInfo {
            kind: "synthetic".to_owned(),
            command: "synthetic run".to_owned(),
            exit_code: 0,
        };
        let target_execution = TargetExecution {
            checked: false,
            method: None,
            result: CheckValue::NotChecked,
            count: None,
        };
        let evidence = EvidenceRecord {
            id: "synthetic-evidence".to_owned(),
            test_id: test.id.clone(),
            adapter: Some(AdapterId::from(self.id)),
            result: TestResult::Pass,
            executed_at: "2026-08-09T00:00:00Z".to_owned(),
            revision: Revision {
                commit: Some("synthetic".to_owned()),
                dirty: false,
            },
            hashes: EvidenceHashes {
                test_fn: test.content_hash.clone(),
                target_fn: target_hash.clone(),
                target_fns: vec![target_hash],
            },
            runner: runner.clone(),
            target_execution: target_execution.clone(),
            log_ref: "cache/logs/synthetic-evidence.log".to_owned(),
        };
        Ok(RunnerResult {
            observation: RunnerObservation {
                result: TestResult::Pass,
                runner,
                target_execution,
                log: "synthetic pass".to_owned(),
            },
            evidence: Some(evidence),
            diagnostics: Vec::new(),
        })
    }
}

#[test]
fn adapter_acceptance_merges_synthetic_results_deterministically() {
    let mut first = AdapterRegistry::new();
    let a = SyntheticAdapter {
        id: "z-synthetic",
        test_id: "TEST-Z",
    };
    let b = SyntheticAdapter {
        id: "a-synthetic",
        test_id: "TEST-A",
    };
    first
        .register(AdapterRegistration::new(a.descriptor()).with_discovery(Arc::new(a)))
        .unwrap();
    first
        .register(AdapterRegistration::new(b.descriptor()).with_discovery(Arc::new(b)))
        .unwrap();
    let result = scan_project_with_registry(Path::new("."), &first).unwrap();
    assert_eq!(
        result
            .tests
            .iter()
            .map(|test| test.id.as_str())
            .collect::<Vec<_>>(),
        ["TEST-A", "TEST-Z"]
    );
    assert_eq!(
        result
            .adapters
            .iter()
            .map(|adapter| adapter.id.as_str())
            .collect::<Vec<_>>(),
        ["a-synthetic", "z-synthetic"]
    );
}

#[test]
fn adapter_acceptance_rejects_duplicate_test_ids_and_missing_capability() {
    let mut duplicate = AdapterRegistry::new();
    for id in ["a-synthetic", "b-synthetic"] {
        let adapter = SyntheticAdapter {
            id,
            test_id: "TEST-DUPLICATE",
        };
        duplicate
            .register(
                AdapterRegistration::new(adapter.descriptor()).with_discovery(Arc::new(adapter)),
            )
            .unwrap();
    }
    let error = scan_project_with_registry(Path::new("."), &duplicate).unwrap_err();
    assert!(error.to_string().contains("E-ADAPTER-003"));

    let registry = registry(SyntheticAdapter {
        id: "synthetic",
        test_id: "TEST-SYNTHETIC",
    });
    let error = match registry.require(&AdapterId::from("synthetic"), Capability::Coverage) {
        Ok(_) => panic!("missing coverage capability must not be accepted"),
        Err(error) => error,
    };
    assert_eq!(error.code, "W-ADAPTER-101");

    let discovery = SyntheticAdapter {
        id: "unknown",
        test_id: "TEST-UNKNOWN-ADAPTER",
    };
    let mut test = discovery
        .discover(Path::new("."), &AdapterConfig::default())
        .unwrap()
        .tests
        .into_iter()
        .next()
        .unwrap();
    test.execution.adapter = AdapterId::from("unregistered");
    let result = run_tests_with_registry(
        Path::new("."),
        &VerifyLayout::new(Path::new(".")),
        &[RunnableTest {
            entity: test,
            target_hashes: vec![ContentHash::from_text("target")],
            target_locator: None,
        }],
        true,
        &AdapterRegistry::new(),
    )
    .unwrap();
    assert!(result.has_errors());
    assert!(result.evidence.is_empty());
}

#[test]
fn adapter_acceptance_runs_synthetic_runner_and_binds_evidence_to_adapter() {
    let discovery = SyntheticAdapter {
        id: "synthetic",
        test_id: "TEST-SYNTHETIC-RUNNER",
    };
    let scan = discovery
        .discover(Path::new("."), &AdapterConfig::default())
        .unwrap();
    let test = scan.tests.into_iter().next().unwrap();
    let target_hash = scan.sources[0].content_hash.clone();
    let runnable = RunnableTest {
        entity: test,
        target_hashes: vec![target_hash],
        target_locator: Some(scan.sources[0].locator.clone()),
    };
    let runner = SyntheticRunner { id: "synthetic" };
    let mut registry = AdapterRegistry::new();
    registry
        .register(AdapterRegistration::new(runner.descriptor()).with_runner(Arc::new(runner)))
        .unwrap();
    let result = run_tests_with_registry(
        Path::new("."),
        &VerifyLayout::new(Path::new(".")),
        &[runnable],
        true,
        &registry,
    )
    .unwrap();
    assert!(result.diagnostics.is_empty());
    assert_eq!(result.evidence.len(), 1);
    assert_eq!(
        result.evidence[0].adapter.as_ref().map(AdapterId::as_str),
        Some("synthetic")
    );
    assert_eq!(
        result.evidence[0].target_execution.result,
        CheckValue::NotChecked
    );
}
