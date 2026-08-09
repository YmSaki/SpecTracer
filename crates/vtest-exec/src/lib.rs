//! Language-neutral execution orchestration.
//!
//! Runner construction, Cargo invocation, coverage parsing, and source
//! observation are adapter capabilities.  This crate owns the boundary after
//! an observation: revision capture, raw-log persistence, Evidence material,
//! execution-state hashing, and append-only canonical record storage.

use std::{collections::BTreeMap, fs, path::Path, process::Command};

use serde::Serialize;
use thiserror::Error;
use vtest_adapter_api::{AdapterConfig, AdapterRegistry, Capability, RunnerObservation};
use vtest_model::{
    CheckValue, ContentHash, Diagnostic, EvidenceHashes, EvidenceRecord, EvidenceTargetHash,
    ExecutionStateRecord, Revision, RunnerInfo, TargetExecution,
    TargetExecutionEntry as ModelTargetExecutionEntry, TestEntity,
};
use vtest_store::{load_config, new_record_id, now_rfc3339, write_new_record, VerifyLayout};

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("adapter error {code}: {message}")]
    Adapter { code: String, message: String },
    #[error("I/O error at {path}: {source}")]
    Io {
        path: std::path::PathBuf,
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
    let registry = vtest_adapter_rust::registry().map_err(|error| ExecutionError::Adapter {
        code: error.code,
        message: error.message,
    })?;
    run_tests_with_registry(root, layout, tests, fast, &registry)
}

/// Execute through an explicitly composed registry.  Product callers use the
/// built-in Rust registry; hosts and boundary tests can supply a synthetic or
/// third-party registry without changing the language-neutral service.
pub fn run_tests_with_registry(
    root: &Path,
    layout: &VerifyLayout,
    tests: &[RunnableTest],
    fast: bool,
    registry: &AdapterRegistry,
) -> Result<ExecutionResult, ExecutionError> {
    // Capture the repository state before creating derived cache/log
    // directories.  Those generated files are not execution inputs and must
    // not turn an otherwise clean baseline into a dirty Evidence revision.
    let run_revision = git_revision(root);
    let log_dir = layout.cache_dir().join("logs");
    fs::create_dir_all(&log_dir).map_err(|source| ExecutionError::Io {
        path: log_dir.clone(),
        source,
    })?;
    fs::create_dir_all(layout.evidence_dir()).map_err(|source| ExecutionError::Io {
        path: layout.evidence_dir(),
        source,
    })?;

    let mut evidence = Vec::new();
    let mut diagnostics = Vec::new();
    for runnable in tests {
        let registration = registry
            .require(&runnable.entity.execution.adapter, Capability::Runner)
            .map_err(|error| ExecutionError::Adapter {
                code: error.code,
                message: error.message,
            })?;
        let runner = registration
            .runner
            .as_ref()
            .ok_or_else(|| ExecutionError::Adapter {
                code: "E-ADAPTER-004".to_owned(),
                message: format!(
                    "adapter {} does not implement runner",
                    runnable.entity.execution.adapter
                ),
            })?;
        let adapter_config = adapter_config_for(root, &runnable.entity.execution.adapter)?;
        let outcome = runner.run(root, &runnable.entity, &adapter_config, fast);
        match outcome {
            Ok(observation) => {
                diagnostics.extend(observation.diagnostics.clone());
                if observation.diagnostics.iter().any(Diagnostic::is_error) {
                    continue;
                }
                let record_id = new_record_id();
                let log_path = log_dir.join(format!("{record_id}.log"));
                fs::write(&log_path, &observation.log).map_err(|source| ExecutionError::Io {
                    path: log_path.clone(),
                    source,
                })?;
                let revision = run_revision.clone();
                let target_hashes =
                    normalized_target_hashes(&runnable.entity, &runnable.target_hashes);
                let execution_state = execution_state_record(
                    root,
                    &runnable.entity,
                    &observation,
                    &revision,
                    &target_hashes,
                );
                let record = EvidenceRecord {
                    id: record_id.clone(),
                    test_id: runnable.entity.id.clone(),
                    result: observation.result,
                    executed_at: now_rfc3339(),
                    revision,
                    hashes: EvidenceHashes {
                        test_fn: runnable.entity.content_hash.clone(),
                        target_fn: target_hashes
                            .first()
                            .cloned()
                            .unwrap_or_else(|| ContentHash::from_text("")),
                        target_fns: target_hashes.clone(),
                        test_subject: Some(runnable.entity.content_hash.clone()),
                        targets: runnable
                            .entity
                            .targets
                            .iter()
                            .cloned()
                            .zip(target_hashes.iter().cloned())
                            .map(|(target, target_construct)| EvidenceTargetHash {
                                target,
                                target_construct,
                            })
                            .collect(),
                    },
                    runner: RunnerInfo {
                        kind: observation.runner_kind.clone(),
                        command: observation.command.clone(),
                        exit_code: observation.exit_code,
                    },
                    target_execution: TargetExecution {
                        checked: observation.target_execution.checked,
                        method: observation.target_execution.method.clone(),
                        result: observation.target_execution.result,
                        count: observation.target_execution.count,
                        targets: observation
                            .target_execution
                            .targets
                            .iter()
                            .map(|entry| ModelTargetExecutionEntry {
                                target: entry.target.clone(),
                                result: entry.result,
                                count: entry.count,
                            })
                            .collect(),
                    },
                    log_ref: format!("cache/logs/{record_id}.log"),
                    adapter: Some(observation.adapter.clone()),
                    execution_state: Some(execution_state),
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
            Err(error) if error.code == "E-ADAPTER-006" => {}
            Err(error) => diagnostics.push(Diagnostic::error(error.code, error.message)),
        }
    }
    Ok(ExecutionResult {
        evidence,
        diagnostics,
    })
}

fn adapter_config_for(
    root: &Path,
    adapter_id: &vtest_model::AdapterId,
) -> Result<AdapterConfig, ExecutionError> {
    if !root.join(".verify/config.yaml").is_file() {
        return Ok(AdapterConfig::default());
    }
    let project = match load_config(root) {
        Ok(project) => project,
        Err(vtest_store::StoreError::NotInitialized(_)) => return Ok(AdapterConfig::default()),
        Err(error) => {
            return Err(ExecutionError::Adapter {
                code: "E-ADAPTER-002".to_owned(),
                message: error.to_string(),
            })
        }
    };
    let (roots, include, assertion_macros, coverage) = project
        .adapters
        .iter()
        .find(|entry| entry.id == adapter_id.as_str())
        .map(|entry| {
            (
                entry.roots.clone(),
                entry.scan.include.clone(),
                entry.scan.assertion_macros.clone(),
                entry.run.coverage.clone(),
            )
        })
        .unwrap_or_else(|| {
            (
                vec![".".to_owned()],
                project.scan.include.clone(),
                project.scan.assertion_macros.clone(),
                project.run.coverage.clone(),
            )
        });
    let mut config = AdapterConfig::default();
    config.insert("roots", roots.join(","));
    config.insert("include", include.join(","));
    config.insert("assertion_macros", assertion_macros.join(","));
    config.insert("coverage", coverage);
    Ok(config)
}

fn normalized_target_hashes(test: &TestEntity, hashes: &[ContentHash]) -> Vec<ContentHash> {
    test.targets
        .iter()
        .enumerate()
        .map(|(index, _)| {
            hashes
                .get(index)
                .cloned()
                .unwrap_or_else(|| ContentHash::from_text(""))
        })
        .collect()
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
        .is_some_and(|output| output.status.success() && !output.stdout.is_empty());
    Revision { commit, dirty }
}

fn execution_state_record(
    root: &Path,
    test: &TestEntity,
    observation: &RunnerObservation,
    _revision: &Revision,
    target_hashes: &[ContentHash],
) -> ExecutionStateRecord {
    let manifest = execution_manifest(&observation.execution_state.inputs);
    let target_material = target_hashes
        .iter()
        .map(ContentHash::as_str)
        .collect::<Vec<_>>()
        .join("\n");
    let execution = serde_json::to_vec(&test.execution).unwrap_or_default();
    let invocation =
        serde_json::to_vec(&observation.execution_state.invocation).unwrap_or_default();
    let effective_config =
        serde_json::to_vec(&observation.execution_state.effective_config).unwrap_or_default();
    let config_hash = execution_config_material(root);
    let hash = ContentHash::from_domain_fields(
        "vtest:execution-state:v1",
        &[
            ("adapter", observation.adapter.as_str().as_bytes()),
            (
                "schema_id",
                observation.execution_state.schema_id.as_bytes(),
            ),
            (
                "schema_version",
                observation.execution_state.schema_version.as_bytes(),
            ),
            ("runner", observation.runner_kind.as_bytes()),
            ("command", observation.command.as_bytes()),
            ("invocation", &invocation),
            (
                "toolchain",
                observation.execution_state.toolchain_identity.as_bytes(),
            ),
            ("effective_config", &effective_config),
            ("test_subject", test.content_hash.as_str().as_bytes()),
            ("target_subjects", target_material.as_bytes()),
            ("execution", &execution),
            ("config", config_hash.as_bytes()),
            ("manifest", manifest.as_bytes()),
        ],
    );
    // A disposable/non-Git project can still provide a complete input
    // snapshot and a deterministic state hash.  Evidence validity remains
    // fail-closed because revision matching separately requires a current
    // commit; absence of a commit must not erase the provenance record.
    let complete = observation.execution_state.complete;
    ExecutionStateRecord {
        schema: format!(
            "{}/{}",
            observation.execution_state.schema_id, observation.execution_state.schema_version
        ),
        complete,
        hash: complete.then_some(hash),
    }
}

fn execution_config_material(root: &Path) -> String {
    fs::read_to_string(root.join(".verify/config.yaml")).unwrap_or_default()
}

fn execution_manifest(inputs: &[vtest_adapter_api::ExecutionInputDraft]) -> String {
    let entries = inputs
        .iter()
        .map(|input| {
            (
                (
                    stable_root_identity(&input.root_identity),
                    input.root_relative_path.clone(),
                    input.kind.clone(),
                ),
                ContentHash::from_bytes(&input.bytes),
            )
        })
        .collect::<BTreeMap<_, _>>();
    entries
        .iter()
        .map(|((root, path, kind), hash)| format!("{root}\t{path}\t{kind}\t{}\n", hash.as_str()))
        .collect()
}

fn stable_root_identity(value: &str) -> String {
    if value.is_empty() || std::path::Path::new(value).is_absolute() {
        "workspace".to_owned()
    } else {
        value.replace('\\', "/")
    }
}

fn evidence_yaml(record: &EvidenceRecord) -> String {
    let target = &record.target_execution;
    format!(
        "id: {id}\ntest_id: {test_id}\nresult: {result}\nexecuted_at: {executed_at}\nrevision:\n  commit: {commit}\n  dirty: {dirty}\nadapter: {adapter}\nhashes:\n  test_fn: {test_fn}\n  target_fn: {target_fn}\n  target_fns:\n{target_fns}  targets:\n{target_subjects}  test_subject: {test_subject}\nrunner:\n  kind: {kind}\n  command: {command}\n  exit_code: {exit_code}\ntarget_execution:\n  checked: {checked}\n  method: {method}\n  result: {target_result}\n  count: {count}\n  targets:\n{target_execution_targets}execution_state:\n  schema: {execution_schema}\n  complete: {execution_complete}\n  hash: {execution_hash}\nlog_ref: {log_ref}\n",
        id = yaml_scalar(&record.id),
        test_id = yaml_scalar(record.test_id.as_str()),
        result = yaml_scalar(match record.result {
            vtest_model::TestResult::Pass => "PASS",
            vtest_model::TestResult::Fail => "FAIL",
        }),
        executed_at = yaml_scalar(&record.executed_at),
        commit = record
            .revision
            .commit
            .as_deref()
            .map(yaml_scalar)
            .unwrap_or_else(|| "null".to_owned()),
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
        target_subjects = yaml_target_subjects(&record.hashes.targets),
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
        method = target
            .method
            .as_deref()
            .map(yaml_scalar)
            .unwrap_or_else(|| "null".to_owned()),
        target_result = yaml_scalar(check_value_name(target.result)),
        count = target
            .count
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_owned()),
        target_execution_targets = yaml_target_execution_targets(&target.targets),
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

fn yaml_target_subjects(targets: &[EvidenceTargetHash]) -> String {
    if targets.is_empty() {
        return "    []\n".to_owned();
    }
    targets
        .iter()
        .map(|entry| {
            format!(
                "    - target:\n        adapter: {}\n        value: {}\n      target_construct: {}\n",
                yaml_scalar(entry.target.adapter.as_str()),
                yaml_scalar(&entry.target.value),
                yaml_scalar(entry.target_construct.as_str())
            )
        })
        .collect()
}

fn yaml_target_execution_targets(targets: &[ModelTargetExecutionEntry]) -> String {
    if targets.is_empty() {
        return "    []\n".to_owned();
    }
    targets
        .iter()
        .map(|entry| {
            format!(
                "    - target:\n        adapter: {}\n        value: {}\n      result: {}\n      count: {}\n",
                yaml_scalar(entry.target.adapter.as_str()),
                yaml_scalar(&entry.target.value),
                yaml_scalar(check_value_name(entry.result)),
                entry
                    .count
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "null".to_owned())
            )
        })
        .collect()
}

fn check_value_name(value: CheckValue) -> &'static str {
    match value {
        CheckValue::Pass => "PASS",
        CheckValue::Fail => "FAIL",
        CheckValue::Mismatch => "MISMATCH",
        CheckValue::Missing => "MISSING",
        CheckValue::NotChecked => "NOT_CHECKED",
        CheckValue::NotExecuted => "NOT_EXECUTED",
        CheckValue::Stale => "STALE",
        CheckValue::Unknown => "UNKNOWN",
    }
}

fn yaml_scalar(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use std::{fs, sync::Arc};

    use super::*;
    use vtest_adapter_api::{
        AdapterDescriptor, AdapterRegistration, AdapterRegistry, ExecutionInputDraft,
        ExecutionStateDraft, TargetExecutionEntry, TargetExecutionObservation, TestRunnerAdapter,
    };
    use vtest_model::{AdapterId, ExecutionDescriptor, NeutralTargetRef, SourceLocation, TestId};

    struct SyntheticRunner;

    impl TestRunnerAdapter for SyntheticRunner {
        fn descriptor(&self) -> AdapterDescriptor {
            AdapterDescriptor::new("synthetic-runner", ["fixture"])
                .with_namespace("synthetic")
                .with_capabilities([Capability::Runner])
        }

        fn run(
            &self,
            _root: &Path,
            test: &TestEntity,
            _config: &AdapterConfig,
            _fast: bool,
        ) -> Result<RunnerObservation, vtest_adapter_api::AdapterError> {
            if test.targets.is_empty() {
                return Err(vtest_adapter_api::AdapterError::new(
                    "E-ADAPTER-002",
                    "missing synthetic target",
                ));
            }
            let targets = test.targets.clone();
            let target_entries = targets
                .iter()
                .cloned()
                .map(|target| TargetExecutionEntry {
                    target,
                    result: CheckValue::NotChecked,
                    count: None,
                })
                .collect();
            Ok(RunnerObservation {
                adapter: AdapterId::from("synthetic-runner"),
                result: vtest_model::TestResult::Pass,
                runner_kind: "synthetic-fixed".to_owned(),
                command: "synthetic run".to_owned(),
                exit_code: 0,
                log: "synthetic PASS\n".to_owned(),
                target_execution: TargetExecutionObservation {
                    checked: false,
                    method: None,
                    result: CheckValue::NotChecked,
                    count: None,
                    targets: target_entries,
                },
                execution_state: ExecutionStateDraft {
                    schema_id: "vtest-execution-state".to_owned(),
                    schema_version: "v1".to_owned(),
                    complete: true,
                    head_revision: None,
                    runner_kind: "synthetic-fixed".to_owned(),
                    invocation: serde_json::json!({"command": "synthetic run"}),
                    toolchain_identity: "synthetic-fixed".to_owned(),
                    effective_config: serde_json::json!({"metadata": "fixture.meta"}),
                    inputs: vec![ExecutionInputDraft {
                        root_identity: "fixture".to_owned(),
                        root_relative_path: "fixture.meta".to_owned(),
                        kind: "metadata".to_owned(),
                        bytes: b"intent: one".to_vec(),
                    }],
                },
                diagnostics: Vec::new(),
            })
        }
    }

    #[test]
    fn synthetic_runner_without_coverage_writes_hash_bound_not_checked_evidence() {
        let root =
            std::env::temp_dir().join(format!("vtest-exec-synthetic-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".verify/evidence")).expect("create evidence directory");
        fs::create_dir_all(root.join(".verify/cache")).expect("create cache directory");
        let layout = VerifyLayout::new(&root);
        let test = TestEntity {
            id: TestId::new("TEST-SYNTHETIC-RUNNER"),
            covers: vec!["VO-SYNTHETIC".into()],
            targets: vec![
                NeutralTargetRef::new("synthetic-runner", "fixture::target"),
                NeutralTargetRef::new("synthetic-runner", "fixture::other_target"),
            ],
            intent: "synthetic runner".to_owned(),
            input: None,
            expect: None,
            kind: Some("fixture".to_owned()),
            cases: Vec::new(),
            related: Vec::new(),
            location: SourceLocation::neutral(
                "synthetic-runner",
                "fixture.spec",
                "case",
                vtest_model::ByteRange { start: 0, end: 1 },
            ),
            content_hash: ContentHash::from_text("synthetic test"),
            execution: ExecutionDescriptor {
                adapter: AdapterId::from("synthetic-runner"),
                project: None,
                suite: None,
                selector: "case".to_owned(),
                runner: Some("synthetic-fixed".to_owned()),
                working_root: None,
            },
        };
        let adapter = Arc::new(SyntheticRunner);
        let mut registry = AdapterRegistry::new();
        registry
            .register(AdapterRegistration::new(adapter.descriptor()).with_runner(adapter))
            .expect("register synthetic runner");
        let mut before_observation = SyntheticRunner
            .run(&root, &test, &AdapterConfig::default(), false)
            .expect("synthetic observation");
        let before_state = execution_state_record(
            &root,
            &test,
            &before_observation,
            &Revision {
                commit: None,
                dirty: false,
            },
            &[
                ContentHash::from_text("synthetic target"),
                ContentHash::from_text("synthetic other target"),
            ],
        );
        before_observation.execution_state.inputs[0].bytes = b"intent: changed".to_vec();
        let after_state = execution_state_record(
            &root,
            &test,
            &before_observation,
            &Revision {
                commit: None,
                dirty: false,
            },
            &[
                ContentHash::from_text("synthetic target"),
                ContentHash::from_text("synthetic other target"),
            ],
        );
        assert_ne!(before_state.hash, after_state.hash);
        let result = run_tests_with_registry(
            &root,
            &layout,
            &[RunnableTest {
                entity: test.clone(),
                target_hashes: vec![
                    ContentHash::from_text("synthetic target"),
                    ContentHash::from_text("synthetic other target"),
                ],
            }],
            false,
            &registry,
        )
        .expect("run synthetic observation");
        assert_eq!(result.evidence.len(), 1);
        assert_eq!(
            result.evidence[0].target_execution.result,
            CheckValue::NotChecked
        );
        let record_path = layout
            .evidence_dir()
            .join(format!("{}.yaml", result.evidence[0].id));
        let reread = vtest_store::read_evidence(&record_path).expect("read canonical evidence");
        assert_eq!(reread.hashes.targets.len(), 2);
        assert_eq!(reread.target_execution.targets.len(), 2);
        assert_eq!(reread.target_execution.targets[0].target, test.targets[0]);
        assert!(result.evidence[0]
            .execution_state
            .as_ref()
            .is_some_and(|state| state.complete && state.hash.is_some()));
        assert!(root
            .join(".verify")
            .join(&result.evidence[0].log_ref)
            .is_file());
        let _ = fs::remove_dir_all(root);
    }
}
