//! Cargo test execution and target coverage attribution.

#![allow(dead_code)]

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::Serialize;
use thiserror::Error;
use vtest_adapter_api::{ExecutionInputDraft, ExecutionStateDraft};
use vtest_model::{
    AdapterId, CheckValue, ContentHash, Diagnostic, EvidenceHashes, EvidenceRecord,
    EvidenceTargetHash, ExecutionStateRecord, Locator, Revision, RunnerInfo, TargetExecution,
    TestEntity, TestResult,
};
use vtest_store::{new_record_id, now_rfc3339, write_new_record, VerifyLayout};

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
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

/// Observation returned across the adapter boundary.  It deliberately
/// contains no canonical Evidence id, revision, log path, or serialized
/// record.  Those belong to the language-neutral execution service.
#[derive(Clone, Debug)]
pub struct RunnerObservation {
    pub result: TestResult,
    pub runner_kind: String,
    pub command: String,
    pub exit_code: i32,
    pub log: String,
    pub target_execution: TargetExecution,
    pub target_executions: Vec<TargetExecution>,
    pub execution_state: ExecutionStateDraft,
    pub diagnostics: Vec<Diagnostic>,
}

/// Execute one Rust Test and return only adapter-owned observations.  The
/// caller is responsible for revision capture, raw-log persistence, Evidence
/// construction, and append-only record storage.
pub fn observe_test(
    root: &Path,
    test: &TestEntity,
    target_locators: &[Option<Locator>],
    fast: bool,
    coverage_mode: &str,
    effective_config: serde_json::Value,
) -> Result<Option<RunnerObservation>, ExecutionError> {
    let llvm_cov_available = !fast && coverage_mode == "llvm-cov" && cargo_llvm_cov_available(root);
    let coverage_path = (!fast && llvm_cov_available).then(|| {
        root.join(".verify/cache/cov")
            .join(coverage_file_name(test))
    });
    if let Some(path) = coverage_path.as_ref() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| ExecutionError::Io {
                path: parent.to_owned(),
                source,
            })?;
        }
    }
    let (mut command, command_line, runner_kind) = if let Some(path) = coverage_path.as_ref() {
        (
            cargo_llvm_cov_command(root, test, path),
            llvm_cov_command_string(root, test, path),
            "cargo-llvm-cov",
        )
    } else {
        (
            cargo_command(root, test),
            command_string(test),
            "cargo-test",
        )
    };
    let output = command.output().map_err(|source| ExecutionError::Io {
        path: root.to_owned(),
        source,
    })?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let raw_log = format!("$ {command_line}\n{}{}", stdout, stderr);
    let mut diagnostics = Vec::new();
    let Some(observed) = parse_result(&stdout, &test.execution.selector) else {
        let code = if !output.status.success() {
            "E-EXEC-001"
        } else {
            "E-EXEC-002"
        };
        diagnostics.push(
            Diagnostic::error(
                code,
                format!("requested Test {} has no result line", test.id),
            )
            .with_location(test.location.clone()),
        );
        return Ok(Some(RunnerObservation {
            result: TestResult::Fail,
            runner_kind: runner_kind.to_owned(),
            command: command_line.clone(),
            exit_code: output.status.code().unwrap_or(-1),
            log: raw_log,
            target_execution: unknown_target_execution(),
            target_executions: Vec::new(),
            execution_state: execution_state_draft(
                root,
                runner_kind,
                &command_line,
                effective_config.clone(),
            ),
            diagnostics,
        }));
    };
    if observed == ObservedResult::Ignored {
        return Ok(None);
    }
    let observed_pass = observed == ObservedResult::Pass;
    let process_pass = output.status.success();
    if observed_pass != process_pass {
        diagnostics.push(
            Diagnostic::error(
                "E-EXEC-003",
                format!("cargo exit status contradicts result for Test {}", test.id),
            )
            .with_location(test.location.clone()),
        );
    }

    let (target_executions, target_diagnostic) = if fast {
        (
            target_locators
                .iter()
                .map(|_| not_checked_target_execution())
                .collect::<Vec<_>>(),
            None,
        )
    } else if let Some(path) = coverage_path.as_ref() {
        (
            target_locators
                .iter()
                .map(|target| target_execution_from_coverage(path, target.as_ref()))
                .collect::<Vec<_>>(),
            None,
        )
    } else {
        (
            target_locators
                .iter()
                .map(|_| unavailable_target_execution().0)
                .collect::<Vec<_>>(),
            Some(unavailable_target_execution().1),
        )
    };
    if let Some(diagnostic) = target_diagnostic {
        diagnostics.push(diagnostic.with_location(test.location.clone()));
    }
    let target_execution = aggregate_target_executions(&target_executions);
    let execution_state = execution_state_draft(root, runner_kind, &command_line, effective_config);
    Ok(Some(RunnerObservation {
        result: if observed_pass {
            TestResult::Pass
        } else {
            TestResult::Fail
        },
        runner_kind: runner_kind.to_owned(),
        command: command_line,
        exit_code: output.status.code().unwrap_or(-1),
        log: raw_log,
        target_execution,
        target_executions,
        execution_state,
        diagnostics,
    }))
}

fn coverage_file_name(test: &TestEntity) -> String {
    let id = test
        .id
        .as_str()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>();
    format!("{}-{}.json", std::process::id(), id)
}

fn aggregate_target_executions(targets: &[TargetExecution]) -> TargetExecution {
    if targets.is_empty() {
        return unknown_target_execution();
    }
    let result = targets.iter().fold(CheckValue::Pass, |current, target| {
        combine_check_values(current, target.result)
    });
    let checked = targets.iter().all(|target| target.checked);
    let method = targets.iter().find_map(|target| target.method.clone());
    let count = targets.iter().filter_map(|target| target.count).sum();
    TargetExecution {
        checked,
        method,
        result,
        count: targets
            .iter()
            .all(|target| target.count.is_some())
            .then_some(count),
        targets: Vec::new(),
    }
}

fn combine_check_values(left: CheckValue, right: CheckValue) -> CheckValue {
    use CheckValue::*;
    if left == Fail || right == Fail {
        Fail
    } else if left == Mismatch || right == Mismatch {
        Mismatch
    } else if left == Stale || right == Stale {
        Stale
    } else if left == Missing || right == Missing {
        Missing
    } else if left == NotExecuted || right == NotExecuted {
        NotExecuted
    } else if left == Unknown || right == Unknown {
        Unknown
    } else if left == NotChecked || right == NotChecked {
        NotChecked
    } else {
        Pass
    }
}

fn execution_state_draft(
    root: &Path,
    runner: &str,
    command: &str,
    effective_config: serde_json::Value,
) -> ExecutionStateDraft {
    let inputs = execution_inputs(root);
    let complete = inputs.is_ok();
    ExecutionStateDraft {
        schema_id: "vtest-execution-state".to_owned(),
        schema_version: "v1".to_owned(),
        complete,
        head_revision: None,
        runner_kind: runner.to_owned(),
        invocation: serde_json::json!({"command": command}),
        toolchain_identity: runner.to_owned(),
        effective_config,
        inputs: inputs.unwrap_or_default(),
    }
}

fn execution_inputs(root: &Path) -> Result<Vec<ExecutionInputDraft>, std::io::Error> {
    let mut files = BTreeMap::new();
    collect_execution_files(root, root, &mut files)?;
    Ok(files
        .into_iter()
        .map(|(path, bytes)| ExecutionInputDraft {
            root_identity: root.to_string_lossy().into_owned(),
            root_relative_path: path,
            kind: "workspace-file".to_owned(),
            bytes,
        })
        .collect())
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
    let llvm_cov_available = !fast && cargo_llvm_cov_available(root);
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
        let (mut command, command_line, runner_kind) = if let Some(coverage_path) = &coverage_path {
            (
                cargo_llvm_cov_command(root, &test.entity, coverage_path),
                llvm_cov_command_string(root, &test.entity, coverage_path),
                "cargo-llvm-cov",
            )
        } else {
            (
                cargo_command(root, &test.entity),
                command_string(&test.entity),
                "cargo-test",
            )
        };
        let output = command.output().map_err(|source| ExecutionError::Io {
            path: root.to_owned(),
            source,
        })?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let raw_log = format!("$ {command_line}\n{}{}", stdout, stderr);
        let log_path = log_dir.join(format!("{record_id}.log"));
        fs::write(&log_path, raw_log).map_err(|source| ExecutionError::Io {
            path: log_path.clone(),
            source,
        })?;
        let observation = parse_result(&stdout, &test.entity.execution.selector);
        match observation {
            Some(ObservedResult::Ignored) => {}
            Some(ObservedResult::Pass) | Some(ObservedResult::Fail) => {
                let observed_pass = matches!(observation, Some(ObservedResult::Pass));
                let process_pass = output.status.success();
                if observed_pass != process_pass {
                    diagnostics.push(
                        Diagnostic::error(
                            "E-EXEC-003",
                            format!(
                                "cargo exit status contradicts result for Test {}",
                                test.entity.id
                            ),
                        )
                        .with_location(test.entity.location.clone()),
                    );
                    continue;
                }
                let target_execution = if fast {
                    TargetExecution {
                        checked: false,
                        method: None,
                        result: CheckValue::NotChecked,
                        count: None,
                        targets: Vec::new(),
                    }
                } else if let Some(coverage_path) = &coverage_path {
                    target_execution_from_coverage(coverage_path, test.target_locator.as_ref())
                } else {
                    let (target_execution, diagnostic) = unavailable_target_execution();
                    diagnostics.push(diagnostic.with_location(test.entity.location.clone()));
                    target_execution
                };
                let record = EvidenceRecord {
                    id: record_id.clone(),
                    test_id: test.entity.id.clone(),
                    result: if observed_pass {
                        TestResult::Pass
                    } else {
                        TestResult::Fail
                    },
                    executed_at: now_rfc3339(),
                    revision: revision.clone(),
                    hashes: EvidenceHashes {
                        test_fn: test.entity.content_hash.clone(),
                        target_fn: test
                            .target_hashes
                            .first()
                            .cloned()
                            .unwrap_or_else(|| ContentHash::from_text("")),
                        target_fns: test.target_hashes.clone(),
                        test_subject: Some(test.entity.content_hash.clone()),
                        targets: test
                            .entity
                            .targets
                            .iter()
                            .zip(test.target_hashes.iter())
                            .map(|(target, hash)| EvidenceTargetHash {
                                target: target.clone(),
                                target_construct: hash.clone(),
                            })
                            .collect(),
                    },
                    runner: RunnerInfo {
                        kind: runner_kind.to_owned(),
                        command: command_line.clone(),
                        exit_code: output.status.code().unwrap_or(-1),
                    },
                    target_execution,
                    log_ref: format!("cache/logs/{record_id}.log"),
                    adapter: Some(AdapterId::from("rust-cargo")),
                    execution_state: Some(execution_state_record(
                        root,
                        &test.entity,
                        runner_kind,
                        &command_line,
                        &test.target_hashes,
                    )),
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
            None => {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ObservedResult {
    Pass,
    Fail,
    Ignored,
}

fn parse_result(output: &str, filter: &str) -> Option<ObservedResult> {
    output.lines().find_map(|line| {
        let line = line.trim();
        let rest = line.strip_prefix("test ")?;
        let (name, result) = rest.split_once(" ... ")?;
        if name != filter && !name.ends_with(&format!("::{filter}")) {
            return None;
        }
        match result {
            "ok" => Some(ObservedResult::Pass),
            "FAILED" => Some(ObservedResult::Fail),
            "ignored" => Some(ObservedResult::Ignored),
            _ => None,
        }
    })
}

fn cargo_command(root: &Path, test: &TestEntity) -> Command {
    let mut command = Command::new("cargo");
    command
        .current_dir(root)
        .arg("test")
        .arg("-p")
        .arg(test.execution.project.as_deref().unwrap_or_default());
    append_cargo_suite(&mut command, test);
    command.args(["--", "--exact", &test.execution.selector]);
    command
}

fn cargo_llvm_cov_command(root: &Path, test: &TestEntity, output_path: &Path) -> Command {
    let mut command = Command::new("cargo");
    command
        .current_dir(root)
        .args(["llvm-cov", "test", "-p"])
        .arg(test.execution.project.as_deref().unwrap_or_default());
    append_cargo_suite(&mut command, test);
    command
        .arg("--json")
        .arg("--output-path")
        .arg(output_path)
        .args(["--", "--exact", &test.execution.selector]);
    command
}

fn command_string(test: &TestEntity) -> String {
    let target = cargo_suite_string(test);
    format!(
        "cargo test -p {} {} -- --exact {}",
        test.execution.project.as_deref().unwrap_or_default(),
        target,
        test.execution.selector
    )
}

fn llvm_cov_command_string(root: &Path, test: &TestEntity, output_path: &Path) -> String {
    let target = cargo_suite_string(test);
    let output_path = output_path
        .strip_prefix(root)
        .unwrap_or(output_path)
        .to_string_lossy()
        .replace('\\', "/");
    format!(
        "cargo llvm-cov test -p {} {} --json --output-path {} -- --exact {}",
        test.execution.project.as_deref().unwrap_or_default(),
        target,
        output_path,
        test.execution.selector
    )
}

fn append_cargo_suite(command: &mut Command, test: &TestEntity) {
    let Some(suite) = &test.execution.suite else {
        return;
    };
    match (suite.kind.as_str(), suite.name.as_deref()) {
        ("lib", _) => {
            command.arg("--lib");
        }
        ("bin", Some(name)) => {
            command.arg("--bin").arg(name);
        }
        ("integration", Some(name)) => {
            command.arg("--test").arg(name);
        }
        _ => {}
    }
}

fn cargo_suite_string(test: &TestEntity) -> String {
    let Some(suite) = &test.execution.suite else {
        return String::new();
    };
    match (suite.kind.as_str(), suite.name.as_deref()) {
        ("lib", _) => "--lib".to_owned(),
        ("bin", Some(name)) => format!("--bin {name}"),
        ("integration", Some(name)) => format!("--test {name}"),
        _ => String::new(),
    }
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

fn cargo_llvm_cov_available(root: &Path) -> bool {
    Command::new("cargo")
        .current_dir(root)
        .args(["llvm-cov", "--version"])
        .output()
        .is_ok_and(|output| output.status.success())
}

fn target_execution_from_coverage(
    coverage_path: &Path,
    target: Option<&Locator>,
) -> TargetExecution {
    let Some(target) = target else {
        return unknown_target_execution();
    };
    let output = match fs::read_to_string(coverage_path) {
        Ok(output) => output,
        Err(_) => return unknown_target_execution(),
    };
    let Some(count) = llvm_cov_function_count(&output, target) else {
        return unknown_target_execution();
    };
    measured_target_execution(count)
}

fn llvm_cov_function_count(output: &str, target: &Locator) -> Option<u64> {
    let value = serde_json::from_str::<serde_json::Value>(output).ok()?;
    let data = value.get("data")?.as_array()?;
    let mut total = 0_u64;
    let mut matched = false;
    for item in data {
        let Some(functions) = item.get("functions").and_then(serde_json::Value::as_array) else {
            continue;
        };
        for function in functions {
            let Some(name) = function.get("name").and_then(serde_json::Value::as_str) else {
                continue;
            };
            if !llvm_name_matches(name, &target.item_path)
                || !llvm_filenames_match(function, &target.path)
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

fn not_checked_target_execution() -> TargetExecution {
    TargetExecution {
        checked: false,
        method: Some("llvm-cov".to_owned()),
        result: CheckValue::NotChecked,
        count: None,
        targets: Vec::new(),
    }
}

fn measured_target_execution(count: u64) -> TargetExecution {
    TargetExecution {
        checked: true,
        method: Some("llvm-cov".to_owned()),
        result: if count > 0 {
            CheckValue::Pass
        } else {
            CheckValue::Fail
        },
        count: Some(count),
        targets: Vec::new(),
    }
}

fn unavailable_target_execution() -> (TargetExecution, Diagnostic) {
    (
        not_checked_target_execution(),
        Diagnostic::warning(
            "W-EXEC-101",
            "cargo-llvm-cov is unavailable; target_execution is NOT_CHECKED",
        ),
    )
}

fn unknown_target_execution() -> TargetExecution {
    TargetExecution {
        checked: true,
        method: Some("llvm-cov".to_owned()),
        result: CheckValue::Unknown,
        count: None,
        targets: Vec::new(),
    }
}

fn evidence_yaml(record: &EvidenceRecord) -> String {
    let target = &record.target_execution;
    format!(
        "id: {id}\ntest_id: {test_id}\nresult: {result}\nexecuted_at: {executed_at}\nrevision:\n  commit: {commit}\n  dirty: {dirty}\nadapter: {adapter}\nhashes:\n  test_fn: {test_fn}\n  target_fn: {target_fn}\n  target_fns:\n{target_fns}  test_subject: {test_subject}\nrunner:\n  kind: {kind}\n  command: {command}\n  exit_code: {exit_code}\ntarget_execution:\n  checked: {checked}\n  method: {method}\n  result: {target_result}\n  count: {count}\nexecution_state:\n  schema: {execution_schema}\n  complete: {execution_complete}\n  hash: {execution_hash}\nlog_ref: {log_ref}\n",
        id = yaml_scalar(&record.id),
        test_id = yaml_scalar(record.test_id.as_str()),
        result = yaml_scalar(match record.result { TestResult::Pass => "PASS", TestResult::Fail => "FAIL" }),
        executed_at = yaml_scalar(&record.executed_at),
        commit = record.revision.commit.as_deref().map(yaml_scalar).unwrap_or_else(|| "null".to_owned()),
        dirty = record.revision.dirty,
        adapter = record
            .adapter
            .as_ref()
            .map(|adapter| yaml_scalar(adapter.as_str()))
            .unwrap_or_else(|| "null".to_owned()),
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
        test_subject = record
            .hashes
            .test_subject
            .as_ref()
            .map(|hash| yaml_scalar(hash.as_str()))
            .unwrap_or_else(|| "null".to_owned()),
        kind = yaml_scalar(&record.runner.kind),
        command = yaml_scalar(&record.runner.command),
        exit_code = record.runner.exit_code,
        checked = target.checked,
        method = target.method.as_deref().map(yaml_scalar).unwrap_or_else(|| "null".to_owned()),
        target_result = yaml_scalar(match target.result {
            CheckValue::Pass => "PASS",
            CheckValue::Fail => "FAIL",
            CheckValue::Mismatch => "MISMATCH",
            CheckValue::Missing => "MISSING",
            CheckValue::NotChecked => "NOT_CHECKED",
            CheckValue::NotExecuted => "NOT_EXECUTED",
            CheckValue::Stale => "STALE",
            CheckValue::Unknown => "UNKNOWN",
        }),
        count = target.count.map(|value| value.to_string()).unwrap_or_else(|| "null".to_owned()),
        execution_schema = record
            .execution_state
            .as_ref()
            .map(|state| yaml_scalar(&state.schema))
            .unwrap_or_else(|| "null".to_owned()),
        execution_complete = record
            .execution_state
            .as_ref()
            .map(|state| state.complete.to_string())
            .unwrap_or_else(|| "false".to_owned()),
        execution_hash = record
            .execution_state
            .as_ref()
            .and_then(|state| state.hash.as_ref())
            .map(|hash| yaml_scalar(hash.as_str()))
            .unwrap_or_else(|| "null".to_owned()),
        log_ref = yaml_scalar(&record.log_ref),
    )
}

fn execution_state_record(
    root: &Path,
    test: &TestEntity,
    runner: &str,
    command: &str,
    target_hashes: &[ContentHash],
) -> ExecutionStateRecord {
    let Some(manifest) = execution_manifest(root) else {
        return ExecutionStateRecord {
            schema: "vtest-execution-state/v1".to_owned(),
            complete: false,
            hash: None,
        };
    };
    let target_material = target_hashes
        .iter()
        .map(ContentHash::as_str)
        .collect::<Vec<_>>()
        .join("\n");
    let execution = serde_json::to_vec(&test.execution).unwrap_or_default();
    let hash = ContentHash::from_domain_fields(
        "vtest:execution-state:v1",
        &[
            ("adapter", test.execution.adapter.as_str().as_bytes()),
            ("runner", runner.as_bytes()),
            ("command", command.as_bytes()),
            ("test_subject", test.content_hash.as_str().as_bytes()),
            ("target_subjects", target_material.as_bytes()),
            ("execution", &execution),
            ("manifest", manifest.as_bytes()),
        ],
    );
    ExecutionStateRecord {
        schema: "vtest-execution-state/v1".to_owned(),
        complete: true,
        hash: Some(hash),
    }
}

fn execution_manifest(root: &Path) -> Option<String> {
    let mut files = BTreeMap::new();
    collect_execution_files(root, root, &mut files).ok()?;
    Some(
        files
            .into_iter()
            .map(|(path, bytes)| {
                format!("{}\t{}\n", path, ContentHash::from_bytes(&bytes).as_str())
            })
            .collect(),
    )
}

fn collect_execution_files(
    root: &Path,
    directory: &Path,
    files: &mut BTreeMap<String, Vec<u8>>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap_or(&path);
        if relative.components().any(|component| {
            component
                .as_os_str()
                .to_str()
                .is_some_and(|name| matches!(name, ".verify" | ".git" | "target"))
        }) {
            continue;
        }
        if path.is_dir() {
            collect_execution_files(root, &path, files)?;
        } else if path.is_file() {
            let key = relative.to_string_lossy().replace('\\', "/");
            files.insert(key, fs::read(path)?);
        }
    }
    Ok(())
}

fn yaml_scalar(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_distinguishes_pass_fail_and_ignored() {
        assert_eq!(
            parse_result("test calc::x ... ok", "x"),
            Some(ObservedResult::Pass)
        );
        assert_eq!(
            parse_result("test x ... FAILED", "x"),
            Some(ObservedResult::Fail)
        );
        assert_eq!(
            parse_result("test x ... ignored", "x"),
            Some(ObservedResult::Ignored)
        );
        assert_eq!(parse_result("test y ... ok", "x"), None);
    }

    #[test]
    fn llvm_cov_parser_extracts_target_function_count() {
        let target = Locator {
            path: "src/lib.rs".to_owned(),
            item_path: "add".to_owned(),
        };
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

        let absent = Locator {
            path: "src/lib.rs".to_owned(),
            item_path: "subtract".to_owned(),
        };
        assert_eq!(llvm_cov_function_count(output, &absent), None);
    }

    #[test]
    fn llvm_cov_zero_count_is_preserved_as_a_measured_failure() {
        let target = Locator {
            path: "src/lib.rs".to_owned(),
            item_path: "add".to_owned(),
        };
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

    #[test]
    fn unavailable_coverage_is_not_checked_and_never_passes() {
        let (target_execution, diagnostic) = unavailable_target_execution();
        assert!(!target_execution.checked);
        assert_eq!(target_execution.result, CheckValue::NotChecked);
        assert_eq!(target_execution.count, None);
        assert_eq!(diagnostic.code, "W-EXEC-101");
    }

    #[test]
    fn measured_target_execution_requires_a_positive_count() {
        let called = measured_target_execution(1);
        assert!(called.checked);
        assert_eq!(called.result, CheckValue::Pass);
        assert_eq!(called.count, Some(1));

        let not_called = measured_target_execution(0);
        assert!(not_called.checked);
        assert_eq!(not_called.result, CheckValue::Fail);
        assert_eq!(not_called.count, Some(0));
    }
}
