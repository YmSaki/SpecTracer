//! Cargo test execution, target coverage attribution, and append-only Evidence recording.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::Serialize;
use thiserror::Error;
use vtest_model::{
    CheckValue, ContentHash, Diagnostic, EvidenceHashes, EvidenceRecord, Locator, Revision,
    RunnerInfo, TargetExecution, TestEntity, TestResult, TestTarget,
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
    pub target_hash: ContentHash,
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
        let observation = parse_result(&stdout, &test.entity.filter);
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
                        target_fn: test.target_hash.clone(),
                    },
                    runner: RunnerInfo {
                        kind: runner_kind.to_owned(),
                        command: command_line.clone(),
                        exit_code: output.status.code().unwrap_or(-1),
                    },
                    target_execution,
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
        .arg(&test.package);
    match &test.test_target {
        TestTarget::Lib => {
            command.arg("--lib");
        }
        TestTarget::Bin(name) => {
            command.arg("--bin").arg(name);
        }
        TestTarget::IntegrationTest(name) => {
            command.arg("--test").arg(name);
        }
        TestTarget::Unknown => {}
    }
    command.args(["--", "--exact", &test.filter]);
    command
}

fn cargo_llvm_cov_command(root: &Path, test: &TestEntity, output_path: &Path) -> Command {
    let mut command = Command::new("cargo");
    command
        .current_dir(root)
        .args(["llvm-cov", "test", "-p"])
        .arg(&test.package);
    match &test.test_target {
        TestTarget::Lib => {
            command.arg("--lib");
        }
        TestTarget::Bin(name) => {
            command.arg("--bin").arg(name);
        }
        TestTarget::IntegrationTest(name) => {
            command.arg("--test").arg(name);
        }
        TestTarget::Unknown => {}
    }
    command
        .arg("--json")
        .arg("--output-path")
        .arg(output_path)
        .args(["--", "--exact", &test.filter]);
    command
}

fn command_string(test: &TestEntity) -> String {
    let target = match &test.test_target {
        TestTarget::Lib => "--lib".to_owned(),
        TestTarget::Bin(name) => format!("--bin {name}"),
        TestTarget::IntegrationTest(name) => format!("--test {name}"),
        TestTarget::Unknown => String::new(),
    };
    format!(
        "cargo test -p {} {} -- --exact {}",
        test.package, target, test.filter
    )
}

fn llvm_cov_command_string(root: &Path, test: &TestEntity, output_path: &Path) -> String {
    let target = match &test.test_target {
        TestTarget::Lib => "--lib".to_owned(),
        TestTarget::Bin(name) => format!("--bin {name}"),
        TestTarget::IntegrationTest(name) => format!("--test {name}"),
        TestTarget::Unknown => String::new(),
    };
    let output_path = output_path
        .strip_prefix(root)
        .unwrap_or(output_path)
        .to_string_lossy()
        .replace('\\', "/");
    format!(
        "cargo llvm-cov test -p {} {} --json --output-path {} -- --exact {}",
        test.package, target, output_path, test.filter
    )
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
    }
}

fn evidence_yaml(record: &EvidenceRecord) -> String {
    let target = &record.target_execution;
    format!(
        "id: {id}\ntest_id: {test_id}\nresult: {result}\nexecuted_at: {executed_at}\nrevision:\n  commit: {commit}\n  dirty: {dirty}\nhashes:\n  test_fn: {test_fn}\n  target_fn: {target_fn}\nrunner:\n  kind: {kind}\n  command: {command}\n  exit_code: {exit_code}\ntarget_execution:\n  checked: {checked}\n  method: {method}\n  result: {target_result}\n  count: {count}\nlog_ref: {log_ref}\n",
        id = yaml_scalar(&record.id),
        test_id = yaml_scalar(record.test_id.as_str()),
        result = yaml_scalar(match record.result { TestResult::Pass => "PASS", TestResult::Fail => "FAIL" }),
        executed_at = yaml_scalar(&record.executed_at),
        commit = record.revision.commit.as_deref().map(yaml_scalar).unwrap_or_else(|| "null".to_owned()),
        dirty = record.revision.dirty,
        test_fn = yaml_scalar(record.hashes.test_fn.as_str()),
        target_fn = yaml_scalar(record.hashes.target_fn.as_str()),
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
        log_ref = yaml_scalar(&record.log_ref),
    )
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
