//! Cargo test execution, target coverage attribution, and append-only Evidence recording.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::Serialize;
use thiserror::Error;
use vtest_model::{
    CheckValue, ContentHash, Diagnostic, EvidenceHashes, EvidenceRecord, EvidenceTargetHash,
    ExecutionStateSubject, Revision, RunnerInfo, TargetExecution, TestEntity, TestResult,
};
use vtest_store::{new_record_id, now_rfc3339, write_new_record, VerifyLayout};

mod rust_runner;
use rust_runner::*;

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
    pub target_locator: Option<RustLocator>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustLocator {
    pub path: String,
    pub item_path: String,
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
                        result: None,
                        targets: Vec::new(),
                        compatibility_count: None,
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
                    adapter: Some(test.entity.execution.adapter.clone()),
                    result: if observed_pass {
                        TestResult::Pass
                    } else {
                        TestResult::Fail
                    },
                    executed_at: now_rfc3339(),
                    revision: revision.clone(),
                    execution_state: Some(ExecutionStateSubject {
                        schema: "rust-cargo-execution-state-v1".to_owned(),
                        complete: false,
                        hash: None,
                    }),
                    hashes: EvidenceHashes {
                        test_subject: Some(test.entity.content_hash.clone()),
                        targets: test
                            .entity
                            .targets
                            .iter()
                            .zip(&test.target_hashes)
                            .map(|(target, hash)| EvidenceTargetHash {
                                target: target.normalized(),
                                target_construct: hash.clone(),
                            })
                            .collect(),
                        compatibility: None,
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

fn evidence_yaml(record: &EvidenceRecord) -> String {
    let target = &record.target_execution;
    let test_subject = record
        .hashes
        .test_subject
        .as_ref()
        .expect("new Evidence always has a Test subject");
    let target_hashes = record
        .hashes
        .targets
        .iter()
        .map(|entry| {
            format!(
                "    - target: {}\n      target_construct: {}\n",
                yaml_scalar(&entry.target),
                yaml_scalar(entry.target_construct.as_str())
            )
        })
        .collect::<String>();
    let target_observations = target
        .targets
        .iter()
        .map(|entry| {
            format!(
                "    - target: {}\n      result: {}\n      count: {}\n",
                yaml_scalar(&entry.target),
                yaml_scalar(check_value_name(Some(entry.result))),
                entry
                    .count
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "null".to_owned())
            )
        })
        .collect::<String>();
    format!(
        "id: {id}\ntest_id: {test_id}\nadapter: {adapter}\nresult: {result}\nexecuted_at: {executed_at}\nrevision:\n  commit: {commit}\n  dirty: {dirty}\nexecution_state:\n  schema: {execution_schema}\n  complete: {execution_complete}\n  hash: {execution_hash}\nhashes:\n  test_subject: {test_subject}\n  targets:\n{target_hashes}runner:\n  kind: {kind}\n  command: {command}\n  exit_code: {exit_code}\ntarget_execution:\n  checked: {checked}\n  method: {method}\n  result: {target_result}\n  targets:\n{target_observations}log_ref: {log_ref}\n",
        id = yaml_scalar(&record.id),
        test_id = yaml_scalar(record.test_id.as_str()),
        adapter = record.adapter.as_ref().map(|value| yaml_scalar(value.as_str())).unwrap_or_else(|| "null".to_owned()),
        result = yaml_scalar(match record.result { TestResult::Pass => "PASS", TestResult::Fail => "FAIL" }),
        executed_at = yaml_scalar(&record.executed_at),
        commit = record.revision.commit.as_deref().map(yaml_scalar).unwrap_or_else(|| "null".to_owned()),
        dirty = record.revision.dirty,
        execution_schema = record.execution_state.as_ref().map(|value| yaml_scalar(&value.schema)).unwrap_or_else(|| "null".to_owned()),
        execution_complete = record.execution_state.as_ref().is_some_and(|value| value.complete),
        execution_hash = record.execution_state.as_ref().and_then(|value| value.hash.as_ref()).map(|value| yaml_scalar(value.as_str())).unwrap_or_else(|| "null".to_owned()),
        test_subject = yaml_scalar(test_subject.as_str()),
        target_hashes = target_hashes,
        kind = yaml_scalar(&record.runner.kind),
        command = yaml_scalar(&record.runner.command),
        exit_code = record.runner.exit_code,
        checked = target.checked,
        method = target.method.as_deref().map(yaml_scalar).unwrap_or_else(|| "null".to_owned()),
        target_result = if target.checked { yaml_scalar(check_value_name(target.result)) } else { "null".to_owned() },
        target_observations = target_observations,
        log_ref = yaml_scalar(&record.log_ref),
    )
}

fn check_value_name(value: Option<CheckValue>) -> &'static str {
    match value {
        Some(CheckValue::Pass) => "PASS",
        Some(CheckValue::Fail) => "FAIL",
        Some(CheckValue::Mismatch) => "MISMATCH",
        Some(CheckValue::Missing) => "MISSING",
        Some(CheckValue::NotChecked) => "NOT_CHECKED",
        Some(CheckValue::NotExecuted) => "NOT_EXECUTED",
        Some(CheckValue::Stale) => "STALE",
        Some(CheckValue::Unknown) => "UNKNOWN",
        None => "null",
    }
}

fn yaml_scalar(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
