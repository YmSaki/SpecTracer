//! Test execution, target coverage attribution, and append-only Evidence recording.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::Serialize;
use thiserror::Error;
use vtest_adapter_api::{RunnerOutput, RunnerTestResult, TestRunnerAdapter};
use vtest_adapter_rust::RustCargoTestRunner;
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
    run_tests_with_runner(root, layout, tests, fast, &RustCargoTestRunner::new())
}

pub fn run_tests_with_runner(
    root: &Path,
    layout: &VerifyLayout,
    tests: &[RunnableTest],
    fast: bool,
    runner: &dyn TestRunnerAdapter,
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
    let llvm_cov_available = !fast && coverage_tool_available(root);
    let cov_dir = layout.cache_dir().join("cov");
    if llvm_cov_available {
        fs::create_dir_all(&cov_dir).map_err(|source| ExecutionError::Io {
            path: cov_dir.clone(),
            source,
        })?;
    }
    let mut evidence = Vec::new();
    let mut diagnostics = Vec::new();
    for test in tests {
        let record_id = new_record_id();
        let coverage_path = llvm_cov_available.then(|| cov_dir.join(format!("{record_id}.json")));
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
                    TargetCoverage {
                        checked: false,
                        method: None,
                        result: None,
                        targets: Vec::new(),
                        count: None,
                    }
                } else if let Some(coverage_path) = &coverage_path {
                    target_coverage_from_coverage(coverage_path, test.target_locator.as_ref())
                } else {
                    let (target_coverage, diagnostic) = unavailable_target_coverage();
                    diagnostics.push(diagnostic.with_location(test.entity.location.clone()));
                    target_coverage
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

fn target_coverage_from_coverage(coverage_path: &Path, target: Option<&Locator>) -> TargetCoverage {
    let Some(target) = target else {
        return unknown_target_coverage(None);
    };
    let output = match fs::read_to_string(coverage_path) {
        Ok(output) => output,
        Err(_) => return unknown_target_coverage(Some(target)),
    };
    let Some(count) = llvm_cov_function_count(&output, target) else {
        return unknown_target_coverage(Some(target));
    };
    measured_target_coverage(Some(target), count)
}

/// `target.value` は `rust-cargo` adapter が所有する opaque locator 文字列
/// （`<path>.rs::<item_path>`）。この crate は adapter の内部構文を正式には
/// 所有しないが（crate 冒頭コメント「`vtest-scan`、`vtest-audit`、
/// `vtest-exec` はadapterを選択・委譲するorchestrationであり、rustc-demangle
/// を直接所有しない」）、llvm-cov 出力との突き合わせに `path`/`item_path`
/// の分解がすでに必要だった既存コードであり、PR3 の範囲（`TargetRef::
/// Locator`のadapter-neutral化）はこの crate のRust結合自体の解消を含まな
/// い。分解は最初の `::` で区切るだけで、`RustLocator::parse`の妥当性検査
/// （`.rs`拡張子など）は行わない — この値は常にこの adapter 自身の
/// scanner が構築したものであり、構文は保証されている。
fn locator_parts(locator: &Locator) -> (&str, &str) {
    locator
        .value
        .split_once("::")
        .unwrap_or((locator.value.as_str(), ""))
}

fn llvm_cov_function_count(output: &str, target: &Locator) -> Option<u64> {
    let value = serde_json::from_str::<serde_json::Value>(output).ok()?;
    let data = value.get("data")?.as_array()?;
    let mut total = 0_u64;
    let mut matched = false;
    let (target_path, target_item_path) = locator_parts(target);
    for item in data {
        let Some(functions) = item.get("functions").and_then(serde_json::Value::as_array) else {
            continue;
        };
        for function in functions {
            let Some(name) = function.get("name").and_then(serde_json::Value::as_str) else {
                continue;
            };
            if !llvm_name_matches(name, target_item_path)
                || !llvm_filenames_match(function, target_path)
            {
                continue;
            }
            let function_count = function
                .get("count")
                .and_then(serde_json::Value::as_u64)
                .or_else(|| {
                    function
                        .get("regions")
                        .and_then(serde_json::Value::as_array)
                        .map(|regions| {
                            regions
                                .iter()
                                .filter_map(|region| region.as_array()?.get(4))
                                .filter_map(serde_json::Value::as_u64)
                                .max()
                                .unwrap_or(0)
                        })
                })?;
            matched = true;
            total = total.saturating_add(function_count);
        }
    }
    matched.then_some(total)
}

fn llvm_name_matches(name: &str, item_path: &str) -> bool {
    let demangled = format!("{:#}", rustc_demangle::demangle(name));
    if demangled == item_path || demangled.ends_with(&format!("::{item_path}")) {
        return true;
    }

    let generic_path = format!("{item_path}::<");
    demangled
        .strip_prefix(&generic_path)
        .or_else(|| {
            demangled
                .rsplit_once(&format!("::{generic_path}"))
                .map(|(_, arguments)| arguments)
        })
        .is_some_and(|arguments| !arguments.is_empty() && arguments.ends_with('>'))
}

fn llvm_filenames_match(function: &serde_json::Value, target_path: &str) -> bool {
    function
        .get("filenames")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|filenames| {
            filenames.iter().any(|filename| {
                filename
                    .as_str()
                    .is_some_and(|filename| path_suffix_matches(filename, target_path))
            })
        })
}

fn path_suffix_matches(candidate: &str, expected: &str) -> bool {
    candidate
        .replace('\\', "/")
        .ends_with(&expected.replace('\\', "/"))
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

fn measured_target_coverage(target: Option<&Locator>, count: u64) -> TargetCoverage {
    let result = if count > 0 {
        TargetCoverageResult::Pass
    } else {
        TargetCoverageResult::Fail
    };
    TargetCoverage {
        checked: true,
        method: Some("llvm-cov".to_owned()),
        result: Some(result),
        targets: target
            .map(|target| {
                vec![TargetCoverageTarget {
                    target: canonical_locator(target),
                    result,
                    count: Some(count),
                }]
            })
            .unwrap_or_default(),
        count: Some(count),
    }
}

fn unavailable_target_coverage() -> (TargetCoverage, Diagnostic) {
    (
        not_checked_target_coverage(),
        Diagnostic::warning(
            "W-EXEC-101",
            "cargo-llvm-cov is unavailable; target_coverage is NOT_CHECKED",
        ),
    )
}

fn unknown_target_coverage(target: Option<&Locator>) -> TargetCoverage {
    TargetCoverage {
        checked: true,
        method: Some("llvm-cov".to_owned()),
        result: Some(TargetCoverageResult::Unknown),
        targets: target
            .map(|target| {
                vec![TargetCoverageTarget {
                    target: canonical_locator(target),
                    result: TargetCoverageResult::Unknown,
                    count: None,
                }]
            })
            .unwrap_or_default(),
        count: None,
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

fn coverage_tool_available(root: &Path) -> bool {
    Command::new("cargo")
        .current_dir(root)
        .args(["llvm-cov", "--version"])
        .output()
        .is_ok_and(|output| output.status.success())
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

    /// @vtest.id TEST-EXEC-EVIDENCE-BUILT-FROM-RUNNER
    /// @vtest.covers VO-EXEC-RUNNER-OUTPUT-RESULT-PARSING
    /// @vtest.target crates/vtest-exec/src/lib.rs::tests::evidence_is_built_from_runner_observation
    /// @vtest.intent adapterが返した固定結果からexecがEvidenceとrunner情報を組み立てることを検証する
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

    /// @vtest.id TEST-EXEC-LLVM-COV-FUNCTION-COUNT-MATCH-AND-SUM
    /// @vtest.covers VO-EXEC-LLVM-COV-FUNCTIONS-LOOKUP, VO-EXEC-LLVM-COV-LOCATOR-SUFFIX-MATCH, VO-EXEC-LLVM-COV-GENERIC-COUNTS-SUM
    /// @vtest.target crates/vtest-exec/src/lib.rs::llvm_cov_function_count
    /// @vtest.intent llvm-cov export JSONからlocatorに一致する関数（複数ジェネリックインスタンス含む）のcountを合算し、一致しないtargetはNoneを返すことを検証する
    #[test]
    fn llvm_cov_parser_extracts_target_function_count() {
        let target = rust_locator("src/lib.rs", "add");
        let output = r#"{
            "data": [{
                "functions": [
                    {
                        "name": "calc::add::<i32>",
                        "filenames": ["C:/workspace/calc/src/lib.rs"],
                        "count": 2
                    },
                    {
                        "name": "calc::add::<u64>",
                        "filenames": ["C:/workspace/calc/src/lib.rs"],
                        "regions": [[1, 0, 1, 10, 3]]
                    },
                    {
                        "name": "other::add",
                        "filenames": ["C:/workspace/calc/src/other.rs"],
                        "count": 99
                    }
                ]
            }]
        }"#;
        assert_eq!(llvm_cov_function_count(output, &target), Some(5));

        let absent = rust_locator("src/lib.rs", "subtract");
        assert_eq!(llvm_cov_function_count(output, &absent), None);
    }

    /// @vtest.id TEST-EXEC-LLVM-COV-ZERO-COUNT-NOT-CONFUSED-WITH-UNKNOWN
    /// @vtest.covers VO-EXEC-LLVM-COV-ZERO-COUNT-DISTINCT-FROM-UNKNOWN
    /// @vtest.target crates/vtest-exec/src/lib.rs::llvm_cov_function_count
    /// @vtest.intent 対象関数が発見されcountが0のときSome(0)を返し、対象関数自体が見つからない場合のNoneと区別されることを検証する
    #[test]
    fn llvm_cov_zero_count_is_preserved_as_a_measured_failure() {
        let target = rust_locator("src/lib.rs", "add");
        let output = r#"{
            "data": [{
                "functions": [{
                    "name": "calc::add",
                    "filenames": ["src/lib.rs"],
                    "count": 0
                }]
            }]
        }"#;
        assert_eq!(llvm_cov_function_count(output, &target), Some(0));
    }

    /// @vtest.id TEST-EXEC-LLVM-COV-DEMANGLE-RUST-V0-NAME-MATCH
    /// @vtest.covers VO-EXEC-LLVM-COV-DEMANGLE-MATCH
    /// @vtest.target crates/vtest-exec/src/lib.rs::llvm_name_matches
    /// @vtest.intent Rust v0 mangled関数名をdemangleした末尾がlocatorのitem-pathと一致するときだけ真を返すことを検証する
    #[test]
    fn llvm_cov_parser_demangles_rust_v0_symbols() {
        assert!(llvm_name_matches(
            "_RNvCs119z72hoDxF_12calc_fixture3add",
            "add"
        ));
        assert!(!llvm_name_matches(
            "_RNvCs119z72hoDxF_12calc_fixture8evaluate",
            "add"
        ));
    }

    /// @vtest.id TEST-EXEC-UNAVAILABLE-COVERAGE-NOT-CHECKED
    /// @vtest.covers VO-EXEC-COVERAGE-UNAVAILABLE-NOT-CHECKED
    /// @vtest.target crates/vtest-exec/src/lib.rs::unavailable_target_coverage
    /// @vtest.intent カバレッジツールが利用不能なとき、target_coverageがchecked:false・method:null・result:null・targets:[]となり、診断W-EXEC-101が出ることを検証する
    #[test]
    fn unavailable_coverage_is_not_checked_and_never_passes() {
        let (target_coverage, diagnostic) = unavailable_target_coverage();
        assert!(!target_coverage.checked);
        assert_eq!(target_coverage.method, None);
        assert_eq!(target_coverage.result, None);
        assert!(target_coverage.targets.is_empty());
        assert_eq!(target_coverage.count, None);
        assert_eq!(diagnostic.code, "W-EXEC-101");

        let serialized = serde_json::to_value(&target_coverage).expect("serialize coverage");
        assert_eq!(serialized["method"], serde_json::Value::Null);
        assert_eq!(serialized["result"], serde_json::Value::Null);
        assert_eq!(serialized["targets"], serde_json::json!([]));
        let round_tripped: TargetCoverage =
            serde_json::from_value(serialized).expect("deserialize coverage");
        assert_eq!(round_tripped, target_coverage);
    }

    /// @vtest.id TEST-EXEC-MEASURED-TARGET-COVERAGE-COUNT-JUDGEMENT
    /// @vtest.covers VO-EXEC-TARGET-COVERAGE-COUNT-JUDGEMENT
    /// @vtest.target crates/vtest-exec/src/lib.rs::measured_target_coverage
    /// @vtest.intent 計測countが正のときresult:PASS、countが0のときresult:FAILとなることを検証する
    #[test]
    fn measured_target_coverage_requires_a_positive_count() {
        let called = measured_target_coverage(None, 1);
        assert!(called.checked);
        assert_eq!(called.result, Some(TargetCoverageResult::Pass));
        assert_eq!(called.count, Some(1));

        let not_called = measured_target_coverage(None, 0);
        assert!(not_called.checked);
        assert_eq!(not_called.result, Some(TargetCoverageResult::Fail));
        assert_eq!(not_called.count, Some(0));
    }
}
