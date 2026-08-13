//! Cargo test execution, target coverage attribution, and append-only Evidence recording.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::Serialize;
use thiserror::Error;
use vtest_adapter_api::{ExecutionStateDraft, RunnerOutcome, TestRunnerAdapter};
use vtest_model::{
    hash_execution_state_subject, AdapterId, CanonicalProjection, CheckValue, ContentHash,
    Diagnostic, EvidenceHashes, EvidenceRecord, EvidenceTargetHash, ExecutionInputSubject,
    ExecutionStateSubject, ExecutionStateSubjectInput, Revision, TestEntity, TestResult,
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
}

#[derive(Clone, Debug, Serialize)]
pub struct ExecutionResult {
    pub evidence: Vec<EvidenceRecord>,
    pub diagnostics: Vec<Diagnostic>,
    /// Per-Evidence repository input manifest paths, surfaced for the run
    /// response view only (the persisted record binds just the subject hash).
    #[serde(skip)]
    pub repository_inputs: Vec<EvidenceManifest>,
}

#[derive(Clone, Debug)]
pub struct EvidenceManifest {
    pub evidence_id: String,
    pub paths: Vec<String>,
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
    adapter: &dyn TestRunnerAdapter,
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
    // Core owns the config -> projection mapping; `fast` collapses to
    // coverage off, and the adapter only obeys the projection.
    let exec_config = CanonicalProjection::Map(BTreeMap::from([(
        "coverage".to_owned(),
        CanonicalProjection::String(if fast { "off" } else { "llvm-cov" }.to_owned()),
    )]));
    let mut evidence = Vec::new();
    let mut diagnostics = Vec::new();
    let mut repository_inputs = Vec::new();
    for test in tests {
        let record_id = new_record_id();
        let outcome = adapter
            .run(root, &exec_config, &test.entity)
            .map_err(|error| ExecutionError::Io {
                path: root.to_owned(),
                source: std::io::Error::other(error.to_string()),
            })?;
        // Persist the log for every outcome; only Evidence references it.
        let log = match &outcome {
            RunnerOutcome::Completed(observation) => observation.log.clone(),
            RunnerOutcome::Ignored { log, .. } | RunnerOutcome::MissingResult { log, .. } => {
                log.clone()
            }
        };
        let log_path = log_dir.join(format!("{record_id}.log"));
        fs::write(&log_path, &log).map_err(|source| ExecutionError::Io {
            path: log_path.clone(),
            source,
        })?;
        match outcome {
            RunnerOutcome::Ignored { .. } => {}
            RunnerOutcome::MissingResult { runner, .. } => {
                // Build failure vs a requested filter that produced no result
                // line is discriminated by the process exit code (§1182/§1194).
                let code = if runner.exit_code != 0 {
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
            RunnerOutcome::Completed(observation) => {
                let observation = *observation;
                let observed_pass = matches!(observation.result, TestResult::Pass);
                let process_pass = observation.runner.exit_code == 0;
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
                // §783: any input file that changed during the run, or a HEAD
                // that moved, means the pre-run snapshot no longer describes the
                // execution — reject Evidence with E-EXEC-004.
                if execution_state_mutated(root, &observation.execution_state) {
                    diagnostics.push(
                        Diagnostic::error(
                            "E-EXEC-004",
                            format!(
                                "Execution State changed during the run for Test {}",
                                test.entity.id
                            ),
                        )
                        .with_location(test.entity.location.clone()),
                    );
                    continue;
                }
                let record = EvidenceRecord {
                    id: record_id.clone(),
                    test_id: test.entity.id.clone(),
                    adapter: Some(test.entity.execution.adapter.clone()),
                    result: observation.result,
                    executed_at: now_rfc3339(),
                    revision: revision.clone(),
                    execution_state: Some(execution_state_subject(
                        &test.entity.execution.adapter,
                        &observation.execution_state,
                    )),
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
                    runner: observation.runner,
                    target_execution: observation.target_execution,
                    log_ref: format!("cache/logs/{record_id}.log"),
                };
                let path = layout.evidence_dir().join(format!("{record_id}.yaml"));
                write_new_record(&path, &evidence_yaml(&record)).map_err(|error| {
                    ExecutionError::Io {
                        path,
                        source: std::io::Error::other(error.to_string()),
                    }
                })?;
                repository_inputs.push(EvidenceManifest {
                    evidence_id: record_id.clone(),
                    paths: observation
                        .execution_state
                        .inputs
                        .iter()
                        .map(|input| input.root_relative_path.clone())
                        .collect(),
                });
                evidence.push(record);
            }
        }
    }
    Ok(ExecutionResult {
        evidence,
        diagnostics,
        repository_inputs,
    })
}

/// Compute the persisted Execution State subject from the adapter's draft. The
/// core owns the hash: an incomplete draft records the run as history only
/// (`hash: null`) and never passes a fresh `evidence_validity`.
fn execution_state_subject(
    adapter: &AdapterId,
    draft: &ExecutionStateDraft,
) -> ExecutionStateSubject {
    let hash = draft.complete.then(|| {
        let inputs = draft
            .inputs
            .iter()
            .map(|input| ExecutionInputSubject {
                root_identity: &input.root_identity,
                root_relative_path: &input.root_relative_path,
                kind: &input.kind,
                bytes: &input.bytes,
            })
            .collect::<Vec<_>>();
        hash_execution_state_subject(&ExecutionStateSubjectInput {
            adapter,
            schema_id: &draft.schema_id,
            schema_version: &draft.schema_version,
            head_revision: draft.head_revision.as_deref(),
            runner_kind: &draft.runner_kind,
            invocation: &draft.invocation,
            toolchain_identity: &draft.toolchain_identity,
            effective_config: &draft.effective_config,
            inputs: &inputs,
        })
    });
    ExecutionStateSubject {
        schema: draft.schema_id.clone(),
        complete: draft.complete,
        hash,
    }
}

/// Post-run consistency check for E-EXEC-004: compare the current state of
/// every declared input against the pre-run snapshot the adapter captured, and
/// re-check HEAD. Any changed or unreadable input, or a moved HEAD, means the
/// run's inputs were not stable and no Evidence may be recorded.
fn execution_state_mutated(root: &Path, draft: &ExecutionStateDraft) -> bool {
    if git_revision(root).commit.as_deref() != draft.head_revision.as_deref() {
        return true;
    }
    draft.inputs.iter().any(|input| {
        fs::read(root.join(&input.root_relative_path))
            .map(|bytes| bytes != input.bytes)
            .unwrap_or(true)
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
