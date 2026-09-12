//! Test execution, target coverage attribution, and append-only Evidence recording.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::Serialize;
use thiserror::Error;
use vtest_adapter_api::{
    CoverageAdapter, CoverageTargetMeasurement, RunnerOutput, RunnerTestResult, TestRunnerAdapter,
};
use vtest_adapter_rust::{RustCargoCoverageAdapter, RustCargoTestRunner};
use vtest_model::{
    ContentHash, Diagnostic, EvidenceHashes, EvidenceRecord, Locator, Revision, RunnerInfo,
    TargetCoverage, TargetCoverageResult, TargetCoverageTarget, TestEntity, TestResult,
};
use vtest_store::{
    execution_state::{reconstruct_execution_state, ExecutionStateInputs},
    new_record_id, now_rfc3339, write_new_record, VerifyLayout,
};

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("runner adapter error: {0}")]
    Runner(#[from] vtest_adapter_api::TestRunnerError),
}

#[derive(Clone, Debug)]
pub struct RunnableTest {
    pub entity: TestEntity,
    pub target_hashes: Vec<ContentHash>,
    pub target_locator: Option<Locator>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ExecutionResult {
    pub evidence: Vec<EvidenceRecord>,
    pub diagnostics: Vec<Diagnostic>,
}

impl ExecutionResult {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }
}

pub fn run_tests(
    root: &Path,
    layout: &VerifyLayout,
    tests: &[RunnableTest],
    fast: bool,
) -> Result<ExecutionResult, ExecutionError> {
    run_tests_with_runner(
        root,
        layout,
        tests,
        fast,
        &RustCargoTestRunner::new(),
        &RustCargoCoverageAdapter::new(),
    )
}

pub fn run_tests_with_runner(
    root: &Path,
    layout: &VerifyLayout,
    tests: &[RunnableTest],
    fast: bool,
    runner: &dyn TestRunnerAdapter,
    coverage: &dyn CoverageAdapter,
) -> Result<ExecutionResult, ExecutionError> {
    let log_dir = layout.cache_dir().join("logs");
    fs::create_dir_all(&log_dir).map_err(|source| ExecutionError::Io {
        path: log_dir.clone(),
        source,
    })?;
    fs::create_dir_all(layout.evidence_dir()).map_err(|source| ExecutionError::Io {
        path: layout.evidence_dir(),
        source,
    })?;
    let revision = git_revision(root);
    // `coverage.availability` is only asked when `!fast`, matching the
    // pre-split short-circuit: in fast mode target_coverage is always
    // `checked: false` and the tool is never even probed.
    let coverage_availability = if fast {
        None
    } else {
        Some(coverage.availability(root))
    };
    let coverage_is_available = matches!(coverage_availability, Some(Ok(())));
    let cov_dir = layout.cache_dir().join("cov");
    if coverage_is_available {
        fs::create_dir_all(&cov_dir).map_err(|source| ExecutionError::Io {
            path: cov_dir.clone(),
            source,
        })?;
    }
    let mut evidence = Vec::new();
    let mut diagnostics = Vec::new();
    for test in tests {
        let record_id = new_record_id();
        let coverage_path =
            coverage_is_available.then(|| cov_dir.join(format!("{record_id}.json")));
        let command_spec = runner.command(
            root,
            &test.entity.execution,
            coverage_path.is_some(),
            coverage_path.as_deref(),
        )?;
        let mut command = Command::new(&command_spec.program);
        command
            .current_dir(&command_spec.current_dir)
            .args(&command_spec.args);
        for (key, value) in &command_spec.env {
            command.env(key, value);
        }
        let output = command.output().map_err(|source| ExecutionError::Io {
            path: root.to_owned(),
            source,
        })?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let raw_log = format!("$ {}\n{}{}", command_spec.command_line, stdout, stderr);
        let log_path = log_dir.join(format!("{record_id}.log"));
        fs::write(&log_path, raw_log).map_err(|source| ExecutionError::Io {
            path: log_path.clone(),
            source,
        })?;
        let observation = runner.parse(
            &test.entity.execution,
            RunnerOutput {
                stdout: &stdout,
                stderr: &stderr,
                exit_code: output.status.code(),
            },
        );
        match observation {
            RunnerTestResult::Ignored => {}
            RunnerTestResult::Pass | RunnerTestResult::Fail => {
                let observed_pass = matches!(observation, RunnerTestResult::Pass);
                let process_pass = output.status.success();
                if observed_pass != process_pass {
                    diagnostics.push(
                        Diagnostic::error(
                            "E-EXEC-003",
                            format!(
                                "runner exit status contradicts result for Test {}",
                                test.entity.id
                            ),
                        )
                        .with_location(test.entity.location.clone()),
                    );
                    continue;
                }
                let target_coverage = if fast {
                    not_checked_target_coverage()
                } else if let Some(coverage_path) = &coverage_path {
                    let measurement = test.target_locator.as_ref().and_then(|target| {
                        coverage
                            .measure(coverage_path, std::slice::from_ref(target))
                            .into_iter()
                            .next()
                    });
                    target_coverage_from_measurement(coverage.method(), measurement)
                } else {
                    let reason = coverage_availability
                        .as_ref()
                        .and_then(|result| result.as_ref().err())
                        .cloned()
                        .unwrap_or_else(|| "coverage capability unavailable".to_owned());
                    diagnostics.push(
                        Diagnostic::warning("W-EXEC-101", reason)
                            .with_location(test.entity.location.clone()),
                    );
                    not_checked_target_coverage()
                };
                // DES-213「Evidence writerは`adapter`を必須で記録し、保存前に
                // Testの`ExecutionDescriptor.adapter`およびrunner kindとの
                // 整合を検証する」: the Test's own declared execution
                // adapter is authoritative, not a value re-derived from a
                // resolved target's locator or a hardcoded literal. If a
                // resolved target's locator names a *different* adapter,
                // that is a real inconsistency DES-213 asks this writer to
                // check for — reported as a diagnostic rather than
                // silently preferring one value over the other.
                let adapter_id = test.entity.execution.adapter.clone();
                if let Some(locator) = &test.target_locator {
                    if locator.adapter != adapter_id {
                        diagnostics.push(
                            Diagnostic::warning(
                                "W-EXEC-102",
                                format!(
                                    "Test {} declares execution.adapter {:?} but its resolved \
                                     target locator names adapter {:?} (DES-213)",
                                    test.entity.id, adapter_id, locator.adapter
                                ),
                            )
                            .with_location(test.entity.location.clone()),
                        );
                    }
                }
                let record = EvidenceRecord {
                    id: record_id.clone(),
                    test_id: test.entity.id.clone(),
                    adapter: adapter_id.clone(),
                    result: if observed_pass {
                        TestResult::Pass
                    } else {
                        TestResult::Fail
                    },
                    executed_at: now_rfc3339(),
                    revision: revision.clone(),
                    // DES-097/098/099/100/101/210/211/212: reconstructed by
                    // the shared `vtest-store::execution_state` module (see
                    // its module doc for the disclosed scope limits —
                    // single-workspace-root topology, no adapter-config
                    // projection input exists in this repository yet).
                    execution_state: reconstruct_execution_state(
                        root,
                        ExecutionStateInputs {
                            adapter: &adapter_id,
                            schema: "rust-cargo-execution-state-v1",
                            head_commit: revision.commit.as_deref(),
                            runner_kind: &command_spec.runner_kind,
                            invocation: &command_spec.command_line,
                        },
                    ),
                    hashes: EvidenceHashes {
                        test_fn: test.entity.content_hash.clone(),
                        target_fn: test
                            .target_hashes
                            .first()
                            .cloned()
                            .unwrap_or_else(|| ContentHash::from_text("")),
                        target_fns: test.target_hashes.clone(),
                    },
                    runner: RunnerInfo {
                        kind: command_spec.runner_kind.clone(),
                        command: command_spec.command_line.clone(),
                        exit_code: output.status.code().unwrap_or(-1),
                    },
                    target_coverage,
                    log_ref: format!("cache/logs/{record_id}.log"),
                };
                let path = layout.evidence_dir().join(format!("{record_id}.yaml"));
                write_new_record(&path, &evidence_yaml(&record)).map_err(|error| {
                    ExecutionError::Io {
                        path,
                        source: std::io::Error::other(error.to_string()),
                    }
                })?;
                evidence.push(record);
            }
            RunnerTestResult::Unknown => {
                let code = if !output.status.success() {
                    "E-EXEC-001"
                } else {
                    "E-EXEC-002"
                };
                diagnostics.push(
                    Diagnostic::error(
                        code,
                        format!("requested Test {} has no result line", test.entity.id),
                    )
                    .with_location(test.entity.location.clone()),
                );
            }
        }
    }
    Ok(ExecutionResult {
        evidence,
        diagnostics,
    })
}

fn git_revision(root: &Path) -> Revision {
    let commit = Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|value| !value.is_empty());
    let dirty = Command::new("git")
        .current_dir(root)
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .is_some_and(|output| !output.stdout.is_empty());
    Revision { commit, dirty }
}

fn not_checked_target_coverage() -> TargetCoverage {
    TargetCoverage {
        checked: false,
        method: None,
        result: None,
        targets: Vec::new(),
        count: None,
    }
}

/// Maps a [`vtest_adapter_api::CoverageTargetMeasurement`] (already judged
/// PASS/FAIL/UNKNOWN by the `CoverageAdapter`, DS-832) onto the Evidence
/// wire's `target_coverage` shape (DES-187/DES-188). `None` means the Test
/// declared no target locator at all — `checked: true` with an `UNKNOWN`
/// result and no `targets` entries, the same shape the pre-split code used
/// for "no target to measure" (distinct from `not_checked_target_coverage`,
/// which is `checked: false` for "could not measure at all").
fn target_coverage_from_measurement(
    method: &str,
    measurement: Option<CoverageTargetMeasurement>,
) -> TargetCoverage {
    let Some(measurement) = measurement else {
        return TargetCoverage {
            checked: true,
            method: Some(method.to_owned()),
            result: Some(TargetCoverageResult::Unknown),
            targets: Vec::new(),
            count: None,
        };
    };
    TargetCoverage {
        checked: true,
        method: Some(method.to_owned()),
        result: Some(measurement.result),
        targets: vec![TargetCoverageTarget {
            target: canonical_locator(&measurement.target),
            result: measurement.result,
            count: measurement.count,
        }],
        count: measurement.count,
    }
}

fn canonical_locator(locator: &Locator) -> String {
    format!("{}::{}", locator.adapter, locator.value)
}

fn evidence_yaml(record: &EvidenceRecord) -> String {
    let target = &record.target_coverage;
    format!(
        "id: {id}\ntest_id: {test_id}\nadapter: {adapter}\nresult: {result}\nexecuted_at: {executed_at}\nrevision:\n  commit: {commit}\n  dirty: {dirty}\nexecution_state:\n  schema: {es_schema}\n  complete: {es_complete}\n  hash: {es_hash}\nhashes:\n  test_fn: {test_fn}\n  target_fn: {target_fn}\n  target_fns:\n{target_fns}runner:\n  kind: {kind}\n  command: {command}\n  exit_code: {exit_code}\ntarget_coverage:\n  checked: {checked}\n  method: {method}\n  result: {target_result}\n{targets}  count: {count}\nlog_ref: {log_ref}\n",
        id = yaml_scalar(&record.id),
        test_id = yaml_scalar(record.test_id.as_str()),
        adapter = yaml_scalar(record.adapter.as_str()),
        result = yaml_scalar(match record.result { TestResult::Pass => "PASS", TestResult::Fail => "FAIL" }),
        executed_at = yaml_scalar(&record.executed_at),
        commit = record.revision.commit.as_deref().map(yaml_scalar).unwrap_or_else(|| "null".to_owned()),
        dirty = record.revision.dirty,
        es_schema = yaml_scalar(&record.execution_state.schema),
        es_complete = record.execution_state.complete,
        es_hash = record.execution_state.hash.as_ref().map(|hash| yaml_scalar(hash.as_str())).unwrap_or_else(|| "null".to_owned()),
        test_fn = yaml_scalar(record.hashes.test_fn.as_str()),
        target_fn = yaml_scalar(record.hashes.target_fn.as_str()),
        target_fns = if record.hashes.target_fns.is_empty() {
            format!("    - {}\n", yaml_scalar(record.hashes.target_fn.as_str()))
        } else {
            record
                .hashes
                .target_fns
                .iter()
                .map(|hash| format!("    - {}\n", yaml_scalar(hash.as_str())))
                .collect::<String>()
        },
        kind = yaml_scalar(&record.runner.kind),
        command = yaml_scalar(&record.runner.command),
        exit_code = record.runner.exit_code,
        checked = target.checked,
        method = target.method.as_deref().map(yaml_scalar).unwrap_or_else(|| "null".to_owned()),
        target_result = target
            .result
            .map(target_coverage_result_name)
            .map(yaml_scalar)
            .unwrap_or_else(|| "null".to_owned()),
        targets = target_coverage_targets_yaml(&target.targets),
        count = target.count.map(|value| value.to_string()).unwrap_or_else(|| "null".to_owned()),
        log_ref = yaml_scalar(&record.log_ref),
    )
}

fn target_coverage_result_name(result: TargetCoverageResult) -> &'static str {
    match result {
        TargetCoverageResult::Pass => "PASS",
        TargetCoverageResult::Fail => "FAIL",
        TargetCoverageResult::Unknown => "UNKNOWN",
    }
}

fn target_coverage_targets_yaml(targets: &[TargetCoverageTarget]) -> String {
    if targets.is_empty() {
        return "  targets: []\n".to_owned();
    }
    let mut yaml = String::from("  targets:\n");
    for target in targets {
        yaml.push_str(&format!(
            "    - target: {}\n      result: {}\n      count: {}\n",
            yaml_scalar(&target.target),
            yaml_scalar(target_coverage_result_name(target.result)),
            target
                .count
                .map(|value| value.to_string())
                .unwrap_or_else(|| "null".to_owned()),
        ));
    }
    yaml
}

fn yaml_scalar(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use vtest_adapter_api::{
        RunnerCommand, RunnerOutput, RunnerTestResult, TestRunnerAdapter, TestRunnerError,
    };
    use vtest_model::{AdapterId, ExecutionDescriptor, ProjectPath, SourceLocation, SourceRange};

    fn rust_locator(path: &str, item_path: &str) -> Locator {
        Locator {
            adapter: AdapterId::new("rust-cargo"),
            value: format!("{path}::{item_path}"),
        }
    }

    #[derive(Clone, Copy, Debug)]
    struct FixedResultRunner;

    impl TestRunnerAdapter for FixedResultRunner {
        fn id(&self) -> &'static str {
            "fake-runner"
        }

        fn command(
            &self,
            root: &Path,
            _execution: &ExecutionDescriptor,
            _coverage: bool,
            _coverage_output_path: Option<&Path>,
        ) -> Result<RunnerCommand, TestRunnerError> {
            let program = std::env::current_exe()
                .expect("test executable")
                .to_string_lossy()
                .into_owned();
            Ok(RunnerCommand {
                command_line: format!("{program} --list"),
                program,
                args: vec!["--list".to_owned()],
                current_dir: root.to_owned(),
                env: BTreeMap::new(),
                runner_kind: "fake-runner".to_owned(),
            })
        }

        fn parse(
            &self,
            _execution: &ExecutionDescriptor,
            _output: RunnerOutput<'_>,
        ) -> RunnerTestResult {
            RunnerTestResult::Pass
        }
    }

    #[derive(Clone, Debug)]
    struct FakeCoverageAdapter {
        method: &'static str,
        availability: Result<(), String>,
        result: TargetCoverageResult,
        count: Option<u64>,
    }

    impl CoverageAdapter for FakeCoverageAdapter {
        fn id(&self) -> &'static str {
            "fake-coverage"
        }

        fn method(&self) -> &'static str {
            self.method
        }

        fn availability(&self, _root: &Path) -> Result<(), String> {
            self.availability.clone()
        }

        fn measure(
            &self,
            _coverage_output_path: &Path,
            targets: &[Locator],
        ) -> Vec<CoverageTargetMeasurement> {
            targets
                .iter()
                .map(|target| CoverageTargetMeasurement {
                    target: target.clone(),
                    result: self.result,
                    count: self.count,
                })
                .collect()
        }
    }

    fn never_available_coverage(reason: &str) -> FakeCoverageAdapter {
        FakeCoverageAdapter {
            method: "fake-coverage",
            availability: Err(reason.to_owned()),
            result: TargetCoverageResult::Unknown,
            count: None,
        }
    }

    #[test]
    fn evidence_is_built_from_runner_observation() {
        let root = std::env::temp_dir().join(format!("vtest-exec-runner-{}", new_record_id()));
        fs::create_dir_all(&root).expect("create runner fixture root");
        let entity = TestEntity {
            id: vtest_model::TestId::new("TEST-EXEC-FAKE-RUNNER"),
            covers: Vec::new(),
            targets: Vec::new(),
            intent: "fixed runner result".to_owned(),
            input: None,
            expect: None,
            kind: None,
            cases: Vec::new(),
            related: Vec::new(),
            location: SourceLocation {
                adapter: AdapterId::new("fake-runner"),
                path: ProjectPath::new("fixture.test"),
                locator: "fixed".to_owned(),
                byte_range: SourceRange { start: 0, end: 1 },
            },
            content_hash: ContentHash::from_text("fixed runner result"),
            execution: ExecutionDescriptor {
                adapter: AdapterId::new("fake-runner"),
                project: None,
                suite: None,
                selector: "fixed".to_owned(),
            },
        };
        let result = run_tests_with_runner(
            &root,
            &vtest_store::VerifyLayout::new(&root),
            &[RunnableTest {
                entity,
                target_hashes: Vec::new(),
                target_locator: None,
            }],
            true,
            &FixedResultRunner,
            &never_available_coverage("unused in fast mode"),
        )
        .expect("fixed runner should produce evidence");

        assert_eq!(result.evidence.len(), 1);
        assert_eq!(result.evidence[0].adapter.as_str(), "fake-runner");
        assert_eq!(result.evidence[0].runner.kind, "fake-runner");
        assert_eq!(result.evidence[0].result, TestResult::Pass);
        assert!(!result.evidence[0].target_coverage.checked);
        assert!(result.diagnostics.is_empty());
        fs::remove_dir_all(root).expect("remove runner fixture root");
    }

    fn runnable_test_with_target(id: &str, target_locator: Option<Locator>) -> RunnableTest {
        RunnableTest {
            entity: TestEntity {
                id: vtest_model::TestId::new(id),
                covers: Vec::new(),
                targets: Vec::new(),
                intent: "coverage capability fixture".to_owned(),
                input: None,
                expect: None,
                kind: None,
                cases: Vec::new(),
                related: Vec::new(),
                location: SourceLocation {
                    adapter: AdapterId::new("fake-runner"),
                    path: ProjectPath::new("fixture.test"),
                    locator: "fixed".to_owned(),
                    byte_range: SourceRange { start: 0, end: 1 },
                },
                content_hash: ContentHash::from_text(id),
                execution: ExecutionDescriptor {
                    adapter: AdapterId::new("fake-runner"),
                    project: None,
                    suite: None,
                    selector: "fixed".to_owned(),
                },
            },
            target_hashes: Vec::new(),
            target_locator,
        }
    }

    // このテストの claim（CoverageAdapter が返す method 名と Target 別到達
    // 計測が target_coverage（method/result/targets/count）へそのまま写る
    // こと）を正本で検索したが該当する VO が見当たらない。候補: DES-352
    // （adapter は DTO を返す責務を負う、という一般則の具体化）だが、
    // 「exec が adapter の出力をそのまま転記する」という writer 側の配線
    // 自体を claim する条文は見つけられていない — 未確定。covers を持たな
    // い無印の #[test]（W-SCAN-101）のまま残す。
    #[test]
    fn target_coverage_is_built_from_a_fake_coverage_adapter_measurement() {
        let root = std::env::temp_dir().join(format!("vtest-exec-coverage-{}", new_record_id()));
        fs::create_dir_all(&root).expect("create coverage fixture root");
        let target = rust_locator("src/lib.rs", "add");
        let test = runnable_test_with_target("TEST-EXEC-FAKE-COVERAGE", Some(target.clone()));
        let coverage = FakeCoverageAdapter {
            method: "fake-cov",
            availability: Ok(()),
            result: TargetCoverageResult::Pass,
            count: Some(3),
        };
        let result = run_tests_with_runner(
            &root,
            &vtest_store::VerifyLayout::new(&root),
            &[test],
            false,
            &FixedResultRunner,
            &coverage,
        )
        .expect("fake coverage adapter should produce evidence");

        assert_eq!(result.evidence.len(), 1);
        let target_coverage = &result.evidence[0].target_coverage;
        assert!(target_coverage.checked);
        assert_eq!(target_coverage.method.as_deref(), Some("fake-cov"));
        assert_eq!(target_coverage.result, Some(TargetCoverageResult::Pass));
        assert_eq!(target_coverage.count, Some(3));
        assert_eq!(target_coverage.targets.len(), 1);
        assert_eq!(
            target_coverage.targets[0].target,
            canonical_locator(&target)
        );
        assert_eq!(
            target_coverage.targets[0].result,
            TargetCoverageResult::Pass
        );
        assert_eq!(target_coverage.targets[0].count, Some(3));
        fs::remove_dir_all(root).expect("remove coverage fixture root");
    }

    /// @vtest.id TEST-EXEC-COVERAGE-UNAVAILABLE-NOT-CHECKED
    /// @vtest.covers VO-EXEC-COVERAGE-UNAVAILABLE-NOT-CHECKED
    /// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
    /// @vtest.intent CoverageAdapter.availabilityがErrを返すとき、target_coverageがchecked:false・method:null・result:null・targets:[]となり、adapterが返した理由文言のままW-EXEC-101診断が出ることを検証する（DS-473）
    #[test]
    fn coverage_unavailable_produces_not_checked_target_coverage_and_w_exec_101() {
        let root = std::env::temp_dir().join(format!("vtest-exec-nocoverage-{}", new_record_id()));
        fs::create_dir_all(&root).expect("create coverage fixture root");
        let test = runnable_test_with_target("TEST-EXEC-NO-COVERAGE", None);
        let coverage = never_available_coverage("fake coverage tool is unavailable");
        let result = run_tests_with_runner(
            &root,
            &vtest_store::VerifyLayout::new(&root),
            &[test],
            false,
            &FixedResultRunner,
            &coverage,
        )
        .expect("unavailable coverage adapter should still produce evidence");

        assert_eq!(result.evidence.len(), 1);
        let target_coverage = &result.evidence[0].target_coverage;
        assert!(!target_coverage.checked);
        assert_eq!(target_coverage.method, None);
        assert_eq!(target_coverage.result, None);
        assert!(target_coverage.targets.is_empty());
        assert_eq!(target_coverage.count, None);

        let serialized = serde_json::to_value(target_coverage).expect("serialize coverage");
        assert_eq!(serialized["method"], serde_json::Value::Null);
        assert_eq!(serialized["result"], serde_json::Value::Null);
        assert_eq!(serialized["targets"], serde_json::json!([]));
        let round_tripped: TargetCoverage =
            serde_json::from_value(serialized).expect("deserialize coverage");
        assert_eq!(&round_tripped, target_coverage);

        let diagnostic = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "W-EXEC-101")
            .expect("W-EXEC-101 diagnostic");
        assert_eq!(diagnostic.message, "fake coverage tool is unavailable");
        fs::remove_dir_all(root).expect("remove coverage fixture root");
    }

    /// BD-114「`vtest-scan`、`vtest-audit`、`vtest-exec` は、それぞれが
    /// `syn`、`quote`、`rustc-demangle`、Cargo commandを直接所有しない。」の
    /// うち `vtest-exec` に関する範囲だけを claim とする（`vtest-scan` /
    /// `vtest-audit` / `syn` / `quote` はこの crate から観測できない）。
    /// PR Bで `rustc-demangle` 依存と `cargo llvm-cov --version` の直接起動
    /// を `vtest-adapter-rust::RustCargoCoverageAdapter` へ移したことを、
    /// このテスト自身の文字列ではなく実ファイル（`Cargo.toml`・自身の
    /// ソース）を読んで機械的に確認する。`needle` を分割して組み立てるのは、
    /// `include_str!` がこのテスト関数自身のソースも読み込むため、探して
    /// いるリテラルをそのまま埋め込むと自己一致してしまうのを避けるため。
    /// @vtest.id TEST-EXEC-NO-DIRECT-DEMANGLE-DEPENDENCY-OR-CARGO-INVOCATION
    /// @vtest.covers VO-EXEC-NO-RUSTC-DEMANGLE-OR-CARGO-COMMAND
    /// @vtest.target crates/vtest-exec/src/lib.rs::tests::exec_does_not_depend_on_rustc_demangle_or_launch_cargo_directly
    /// @vtest.intent vtest-execのCargo.tomlにrustc-demangle依存が無く、ソースにCargo commandの直接起動が無いことを検証する
    #[test]
    fn exec_does_not_depend_on_rustc_demangle_or_launch_cargo_directly() {
        let manifest = include_str!("../Cargo.toml");
        assert!(
            !manifest.contains("rustc-demangle"),
            "vtest-exec/Cargo.toml must not depend on rustc-demangle (BD-114)"
        );
        let source = include_str!("lib.rs");
        let cargo_invocation = ["Command::new(", "\"cargo\")"].concat();
        assert!(
            !source.contains(&cargo_invocation),
            "vtest-exec must not launch cargo directly (BD-114); command construction \
             belongs to the TestRunnerAdapter/CoverageAdapter it calls"
        );
    }
}
