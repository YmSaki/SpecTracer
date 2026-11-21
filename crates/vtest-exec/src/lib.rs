//! Test execution, target coverage attribution, and append-only Evidence recording.

mod state_diff;

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::Serialize;
use thiserror::Error;
use vtest_adapter_api::{
    AdapterRegistry, CoverageAdapter, CoverageTargetMeasurement, CoverageUnavailable, RunnerOutput,
    RunnerTestResult, TestRunnerAdapter,
};
use vtest_model::{
    ContentHash, Diagnostic, EvidenceHashes, EvidenceRecord, Locator, Revision, RunnerInfo,
    TargetCoverage, TargetCoverageResult, TargetCoverageTarget, TestEntity, TestResult,
};
use vtest_store::{
    execution_state::materialize_execution_state, git_head_commit, new_record_id, now_rfc3339,
    write_evidence, VerifyLayout,
};

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// DS-745/DS-923「`E-ADAPTER-003`はerrorであり、Testのexecution
    /// descriptorと選択adapterが不一致であることである」。この variant は
    /// 以前から存在した `TestRunnerAdapter::command` の
    /// `vtest_adapter_api::TestRunnerError`（登録済み adapter が Test の
    /// execution descriptor の形（project/suite欠落・unsupported kind等）
    /// を解釈できない）をそのまま運ぶ — この PR は「未登録」
    /// （E-ADAPTER-001）・「明示操作に必須のcapability欠落」
    /// （E-ADAPTER-004、下記 `run_tests`）とは別の既存経路として、コードを
    /// E-ADAPTER-003 と確定させた（導出: 正本はE-ADAPTER-003の該当箇所を
    /// 「rust-cargoなのにrustのexecution形でない」の例でしか示さないため、
    /// registry解決は成功したが記述子の形が合わない、というこの既存条件を
    /// 充てた）。バッチ全体を中断する既存の挙動（`run_tests_with_runner`の
    /// `?`）はこの PR で変えない。
    #[error("runner adapter error: {0}")]
    Runner(#[from] vtest_adapter_api::TestRunnerError),
}

impl ExecutionError {
    pub fn code(&self) -> Option<&'static str> {
        match self {
            Self::Runner(_) => Some("E-ADAPTER-003"),
            Self::Io { .. } => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RunnableTest {
    pub entity: TestEntity,
    pub target_hashes: Vec<ContentHash>,
    /// Every declared target's resolved canonical Locator, in declaration
    /// order (DS-831/DS-1411: each declared target is measured and recorded).
    pub target_locators: Vec<Locator>,
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

/// `CoverageAdapter` capability が未提供の adapter（DS-1580「明示操作に
/// 必須のcapability欠落=E-ADAPTER-004、変更・判断記録・Evidenceを生成
/// しない」は runner capability の話であり、coverage 単体の欠落は明示
/// 操作を拒否する理由にならない — DS-473/DS-760/DS-1581 の「capability
/// unavailable」と同じ扱いを、生きた `CoverageAdapter` インスタンスが
/// 存在しない場合にも与えるための stand-in）。`availability` が常に Err を
/// 返すため、`run_tests_with_runner` は既存の「coverage 不可時」経路
/// （`checked:false`・W-EXEC-101、`coverage_unavailable_produces_not_
/// checked_target_coverage_and_w_exec_101` が観測する経路）をそのまま辿る
/// — この stand-in を導入するために既存関数を変更しない。
#[derive(Clone, Copy, Debug, Default)]
struct NoCoverageCapability;

impl CoverageAdapter for NoCoverageCapability {
    fn id(&self) -> &'static str {
        ""
    }

    fn method(&self) -> &'static str {
        ""
    }

    fn availability(&self, _root: &Path) -> Result<(), CoverageUnavailable> {
        Err(CoverageUnavailable::new(
            "adapter declares no Coverage capability (DS-925/DS-1581)",
        ))
    }

    fn measure(
        &self,
        _root: &Path,
        _coverage_output_path: &Path,
        _targets: &[Locator],
    ) -> Vec<CoverageTargetMeasurement> {
        Vec::new()
    }
}

/// `registry` は呼び出し元（cli / mcp の composition root）が組み立てた
/// 登録済み adapter 一覧を渡す — この crate（core）は自分で adapter を
/// 生成しない（BD-007、BD-102）。以前この関数は `vtest_adapter_rust::
/// RustCargoTestRunner`/`RustCargoCoverageAdapter` を直接構築していたが、
/// それは registry を介さず core が唯一の adapter を選ぶ経路だった。
///
/// Test を宣言 `execution.adapter` ごとに束ね、束ねた adapter id ごとに
/// registry へ解決する:
/// - 未登録（DS-1663、E-ADAPTER-001）: 該当 Test 群は Evidence を作らず
///   診断だけを記録し、次の束へ進む。
/// - 登録済みだが Test Runner capability 無し（DS-1580、E-ADAPTER-004）:
///   同様にスキップして続行する（brief「他の Test は実行される」）。
/// - 実行可能: `run_tests_with_runner`（既存、変更しない）へ委譲する。
///   Coverage capability が無ければ [`NoCoverageCapability`] を渡し、
///   既存の「coverage 不可」経路をそのまま使う。
///
/// `run_tests_with_runner` 自体が返す `Err`（`TestRunnerError` 由来、
/// E-ADAPTER-003）は、この関数もそのまま伝播しバッチ全体を中断する —
/// 「既存の欠落入力の扱いは変えない」を守るため、この経路の abort-whole-
/// batch 挙動は変更しない。
pub fn run_tests(
    root: &Path,
    layout: &VerifyLayout,
    tests: &[RunnableTest],
    fast: bool,
    registry: &AdapterRegistry,
) -> Result<ExecutionResult, ExecutionError> {
    let mut order: Vec<String> = Vec::new();
    let mut buckets: std::collections::BTreeMap<String, Vec<RunnableTest>> =
        std::collections::BTreeMap::new();
    for test in tests {
        let adapter_id = test.entity.execution.adapter.as_str().to_owned();
        if !buckets.contains_key(&adapter_id) {
            order.push(adapter_id.clone());
        }
        buckets.entry(adapter_id).or_default().push(test.clone());
    }

    let mut evidence = Vec::new();
    let mut diagnostics = Vec::new();
    for adapter_id in order {
        let bucket = buckets.remove(&adapter_id).unwrap_or_default();
        if registry.get(adapter_id.as_str()).is_none() {
            for test in &bucket {
                diagnostics.push(
                    Diagnostic::error(
                        "E-ADAPTER-001",
                        format!(
                            "Test {} declares unregistered adapter {adapter_id:?}",
                            test.entity.id
                        ),
                    )
                    .with_location(test.entity.location.clone()),
                );
            }
            continue;
        }
        let Some(runner) = registry.test_runner(adapter_id.as_str()) else {
            for test in &bucket {
                diagnostics.push(
                    Diagnostic::error(
                        "E-ADAPTER-004",
                        format!(
                            "Test {} adapter {adapter_id:?} has no Test Runner capability \
                             (DS-1580)",
                            test.entity.id
                        ),
                    )
                    .with_location(test.entity.location.clone()),
                );
            }
            continue;
        };
        let no_coverage = NoCoverageCapability;
        let coverage: &dyn CoverageAdapter = registry
            .coverage(adapter_id.as_str())
            .unwrap_or(&no_coverage);
        let batch = run_tests_with_runner(root, layout, &bucket, fast, runner, coverage)?;
        evidence.extend(batch.evidence);
        diagnostics.extend(batch.diagnostics);
    }
    Ok(ExecutionResult {
        evidence,
        diagnostics,
    })
}

// DES-099 / DES-371: manifest order is not part of the execution subject.
// Keep raw fields for incomplete snapshots: their missing hashes cannot expose
// real changes. Tie-break duplicate identities without deduplicating them, so
// malformed manifests still fail validation and retain every observed value.
fn canonicalize_execution_manifest(
    mut draft: vtest_model::ExecutionStateDraft,
) -> vtest_model::ExecutionStateDraft {
    draft.inputs.sort_by(|a, b| {
        (&a.root_identity, &a.root_relative_path, &a.kind, &a.bytes).cmp(&(
            &b.root_identity,
            &b.root_relative_path,
            &b.kind,
            &b.bytes,
        ))
    });
    draft
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
        let command_spec = match runner.command(
            root,
            &test.entity.execution,
            coverage_path.is_some(),
            coverage_path.as_deref(),
        ) {
            Ok(command_spec) => command_spec,
            // A Test the runner declines does not stop the others. For a Test
            // construct the runner cannot execute that is DS-1903. For a Test
            // that cannot be selected uniquely (DS-1930) or whose selection is
            // undetermined (DS-2012) the canon states no such rule: continuing
            // is derived from the other E-EXEC errors, after which the run goes
            // on with the next Test. It is a derivation, not a clause. No
            // Evidence is recorded for the Test, and the diagnostic is the one
            // the refusal names (DS-1901, DS-1930); a refusal that names none
            // (DS-2012) records nothing.
            Err(error) if error.refuses_one_test() => {
                if let Some(diagnostic) = refusal_diagnostic(test, &error) {
                    diagnostics.push(diagnostic);
                }
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        let before_draft = runner
            .execution_state(
                root,
                &test.entity.execution,
                revision.commit.as_deref(),
                &command_spec.runner_kind,
            )
            .map(canonicalize_execution_manifest);
        let before_state = before_draft.as_ref().map(|draft| {
            let mut checked = draft.clone();
            checked.complete &= checked.head_revision == revision.commit
                && checked.runner_kind == command_spec.runner_kind;
            materialize_execution_state(&test.entity.execution.adapter, &checked)
        });
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
        let after_revision = git_revision(root);
        let after_draft = runner
            .execution_state(
                root,
                &test.entity.execution,
                after_revision.commit.as_deref(),
                &command_spec.runner_kind,
            )
            .map(canonicalize_execution_manifest);
        let after_state = after_draft.as_ref().map(|draft| {
            let mut checked = draft.clone();
            checked.complete &= checked.head_revision == after_revision.commit
                && checked.runner_kind == command_spec.runner_kind;
            materialize_execution_state(&test.entity.execution.adapter, &checked)
        });
        // DS-532 / DES-371: never bind a result to inputs captured only after
        // execution. Even incomplete drafts may reveal an input change.
        if before_draft != after_draft || before_state != after_state {
            let summary = state_diff::summarize_execution_state_change(
                before_draft.as_ref(),
                after_draft.as_ref(),
                before_state.as_ref(),
                after_state.as_ref(),
            );
            diagnostics.push(
                Diagnostic::error(
                    "E-EXEC-004",
                    format!(
                        "Execution State changed while running Test {}: {summary}",
                        test.entity.id
                    ),
                )
                .with_location(test.entity.location.clone()),
            );
            continue;
        }
        let execution_state = before_state.unwrap_or(vtest_model::ExecutionState {
            schema: format!("{}-execution-state-unavailable", runner.id()),
            complete: false,
            hash: None,
        });
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
                    let measurements = if test.target_locators.is_empty() {
                        Vec::new()
                    } else {
                        coverage.measure(root, coverage_path, &test.target_locators)
                    };
                    target_coverage_from_measurements(
                        coverage.method(),
                        &test.target_locators,
                        measurements,
                    )
                } else {
                    let reason = coverage_availability
                        .as_ref()
                        .and_then(|result| result.as_ref().err())
                        .map_or_else(
                            || "coverage capability unavailable".to_owned(),
                            ToString::to_string,
                        );
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
                for locator in &test.target_locators {
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
                    adapter: adapter_id,
                    result: if observed_pass {
                        TestResult::Pass
                    } else {
                        TestResult::Fail
                    },
                    executed_at: now_rfc3339(),
                    revision: revision.clone(),
                    execution_state,
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
                write_evidence(&path, &record).map_err(|error| ExecutionError::Io {
                    path,
                    source: std::io::Error::other(error.to_string()),
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
            // The runner classified the run itself, so the process status does
            // not choose the code (DS-1909, DS-1910, DS-1931).
            RunnerTestResult::BuildFailed => diagnostics.push(
                Diagnostic::error(
                    "E-EXEC-001",
                    format!(
                        "the package of requested Test {} did not build (DS-1910)",
                        test.entity.id
                    ),
                )
                .with_location(test.entity.location.clone()),
            ),
            RunnerTestResult::NoResult => diagnostics.push(
                Diagnostic::error(
                    "E-EXEC-002",
                    format!(
                        "the runner reported no result for requested Test {} (DS-1909)",
                        test.entity.id
                    ),
                )
                .with_location(test.entity.location.clone()),
            ),
            RunnerTestResult::Ambiguous => diagnostics.push(
                Diagnostic::error(
                    "E-EXEC-005",
                    format!(
                        "the runner reported more than one result for requested Test {} (DS-1930)",
                        test.entity.id
                    ),
                )
                .with_location(test.entity.location.clone()),
            ),
        }
    }
    Ok(ExecutionResult {
        evidence,
        diagnostics,
    })
}

/// The diagnostic a runner's refusal of one Test is reported as: `W-EXEC-102`
/// for a Test construct it cannot execute (DS-1901) and `E-EXEC-005` for a Test
/// it cannot select uniquely (DS-1930). `None` for a refusal the canon names no
/// code for (DS-2012).
fn refusal_diagnostic(
    test: &RunnableTest,
    error: &vtest_adapter_api::TestRunnerError,
) -> Option<Diagnostic> {
    use vtest_adapter_api::TestRunnerError;
    let diagnostic = match error {
        TestRunnerError::NotExecutable { reason } => Diagnostic::warning(
            "W-EXEC-102",
            format!("Test {} is not executed: {reason}", test.entity.id),
        ),
        TestRunnerError::NotUniquelySelectable { reason } => Diagnostic::error(
            "E-EXEC-005",
            format!("Test {} is not run: {reason}", test.entity.id),
        ),
        TestRunnerError::SelectionUndetermined { .. }
        | TestRunnerError::MissingProject
        | TestRunnerError::MissingSuite
        | TestRunnerError::MissingSuiteName { .. }
        | TestRunnerError::MissingCoverageOutputPath
        | TestRunnerError::UnsupportedSuiteKind { .. } => return None,
    };
    Some(diagnostic.with_location(test.entity.location.clone()))
}

pub fn git_revision(root: &Path) -> Revision {
    let commit = git_head_commit(root);
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

/// Maps the [`vtest_adapter_api::CoverageTargetMeasurement`]s (already
/// judged PASS/FAIL/UNKNOWN by the `CoverageAdapter`, DS-832/DS-1404/
/// DS-1405) onto the Evidence wire's `target_coverage` shape (DES-187/
/// DES-188): one entry per declared target, keyed by its resolved canonical
/// Locator (DS-1412), in declaration order.
///
/// A declared target the adapter returned no measurement for is recorded as
/// `UNKNOWN` with a null count (DES-189) rather than dropped, so a reader
/// sees every declared target (DS-1411). The aggregate `result` is `PASS`
/// only when every entry is `PASS` (DS-1410); otherwise it is `FAIL` when an
/// entry is `FAIL`, else `UNKNOWN`. The aggregate `count` is the smallest
/// per-target count (so a target that was never reached keeps it at 0), or
/// null when any target is `UNKNOWN`.
///
/// No declared target at all is `checked: true` with an `UNKNOWN` result and
/// no `targets` entries — the same shape the pre-split code used for "no
/// target to measure" (distinct from `not_checked_target_coverage`, which is
/// `checked: false` for "could not measure at all").
fn target_coverage_from_measurements(
    method: &str,
    targets: &[Locator],
    measurements: Vec<CoverageTargetMeasurement>,
) -> TargetCoverage {
    let mut entries: Vec<TargetCoverageTarget> = Vec::new();
    for target in targets {
        let canonical = target.to_string();
        if entries.iter().any(|entry| entry.target == canonical) {
            continue;
        }
        let measurement = measurements
            .iter()
            .find(|measurement| &measurement.target == target);
        let (result, count) = match measurement {
            Some(measurement) if measurement.result != TargetCoverageResult::Unknown => {
                (measurement.result, measurement.count)
            }
            _ => (TargetCoverageResult::Unknown, None),
        };
        entries.push(TargetCoverageTarget {
            target: canonical,
            result,
            count,
        });
    }
    if entries.is_empty() {
        return TargetCoverage {
            checked: true,
            method: Some(method.to_owned()),
            result: Some(TargetCoverageResult::Unknown),
            targets: Vec::new(),
            count: None,
        };
    }
    let result = TargetCoverageResult::aggregate(entries.iter().map(|entry| &entry.result));
    let count = entries
        .iter()
        .map(|entry| entry.count)
        .collect::<Option<Vec<u64>>>()
        .and_then(|counts| counts.into_iter().min());
    TargetCoverage {
        checked: true,
        method: Some(method.to_owned()),
        result: Some(result),
        targets: entries,
        count,
    }
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
        availability: Result<(), CoverageUnavailable>,
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

        fn availability(&self, _root: &Path) -> Result<(), CoverageUnavailable> {
            self.availability.clone()
        }

        fn measure(
            &self,
            _root: &Path,
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
            availability: Err(CoverageUnavailable::new(reason)),
            result: TargetCoverageResult::Unknown,
            count: None,
        }
    }

    /// DS-331: Evidence is built from whatever registered runner observed the
    /// Test; nothing in the model is tied to rust-cargo.
    /// @vtest.id TEST-EXEC-EVIDENCE-FROM-NEUTRAL-RUNNER
    /// @vtest.covers VO-EXEC-EVIDENCE-MODEL-RUNNER-NEUTRAL
    /// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
    /// @vtest.intent a non-Rust fake runner's observation becomes Evidence with that adapter and runner kind
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
                target_locators: Vec::new(),
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

    /// A fake runner that answers by the selector of the Test, so one run can
    /// hold a Test that triggers the arm under test and one that must still
    /// run after it. The selector `next` passes. A selector that ends in
    /// `-failing` is started as a process that exits with a failure status, any
    /// other as one that exits 0, so no result below can be told from the
    /// process status.
    #[derive(Clone, Copy, Debug)]
    struct SelectorRunner;

    impl TestRunnerAdapter for SelectorRunner {
        fn id(&self) -> &'static str {
            "fake-runner"
        }

        fn command(
            &self,
            root: &Path,
            execution: &ExecutionDescriptor,
            coverage: bool,
            coverage_output_path: Option<&Path>,
        ) -> Result<RunnerCommand, TestRunnerError> {
            let reason = || format!("refused {}", execution.selector);
            match execution.selector.as_str() {
                "refuse-not-executable" => Err(TestRunnerError::NotExecutable { reason: reason() }),
                "refuse-not-unique" => {
                    Err(TestRunnerError::NotUniquelySelectable { reason: reason() })
                }
                "refuse-undetermined" => {
                    Err(TestRunnerError::SelectionUndetermined { reason: reason() })
                }
                selector => {
                    let mut command = FixedResultRunner.command(
                        root,
                        execution,
                        coverage,
                        coverage_output_path,
                    )?;
                    if selector.ends_with("-failing") {
                        command.args = vec!["--no-such-option-for-vtest".to_owned()];
                    }
                    Ok(command)
                }
            }
        }

        fn parse(
            &self,
            execution: &ExecutionDescriptor,
            _output: RunnerOutput<'_>,
        ) -> RunnerTestResult {
            match execution.selector.trim_end_matches("-failing") {
                "build-failed" => RunnerTestResult::BuildFailed,
                "no-result" => RunnerTestResult::NoResult,
                "ambiguous" => RunnerTestResult::Ambiguous,
                _ => RunnerTestResult::Pass,
            }
        }
    }

    /// Runs a Test with `selector` and then the Test `next`, and returns the
    /// result of the run.
    fn run_selector_then_next(selector: &str) -> ExecutionResult {
        let root = std::env::temp_dir().join(format!("vtest-exec-selector-{}", new_record_id()));
        fs::create_dir_all(&root).expect("create runner fixture root");
        let test = |id: &str, selector: &str| {
            let mut runnable = runnable_test_with_adapter(id, "fake-runner");
            runnable.entity.execution.selector = selector.to_owned();
            runnable
        };
        let result = run_tests_with_runner(
            &root,
            &vtest_store::VerifyLayout::new(&root),
            &[
                test("TEST-EXEC-SUBJECT", selector),
                test("TEST-EXEC-NEXT", "next"),
            ],
            true,
            &SelectorRunner,
            &never_available_coverage("unused in fast mode"),
        )
        .expect("the run goes on to the end");
        fs::remove_dir_all(root).expect("remove runner fixture root");
        result
    }

    fn evidence_ids(result: &ExecutionResult) -> Vec<&str> {
        result
            .evidence
            .iter()
            .map(|record| record.test_id.as_str())
            .collect()
    }

    fn diagnostic_codes(result: &ExecutionResult) -> Vec<&str> {
        result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect()
    }

    /// DS-1909, DS-1910, DS-1930, DS-1931: a runner that classifies the run
    /// itself gets the code it names, whatever the process status was, and no
    /// Evidence; the next Test is still run and recorded.
    /// @vtest.id TEST-EXEC-CLASSIFIED-RESULTS-HAVE-THEIR-CODE-AND-NO-EVIDENCE
    /// @vtest.covers VO-ADAPTER-GO-BUILD-FAILURE-IS-E-EXEC-001, VO-ADAPTER-GO-NO-RESULT-IS-E-EXEC-002, VO-ADAPTER-GO-SAME-NAME-TESTS-RECORD-NO-EVIDENCE
    /// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
    /// @vtest.intent a runner observation of BuildFailed, NoResult or Ambiguous is reported as E-EXEC-001, E-EXEC-002 or E-EXEC-005 whether the process exited 0 or failed, records no Evidence for that Test, and the next Test of the run is recorded
    #[test]
    fn a_classified_result_has_its_code_whatever_the_status_and_the_next_test_runs() {
        for (selector, code) in [
            ("build-failed", "E-EXEC-001"),
            ("build-failed-failing", "E-EXEC-001"),
            ("no-result", "E-EXEC-002"),
            ("no-result-failing", "E-EXEC-002"),
            ("ambiguous", "E-EXEC-005"),
            ("ambiguous-failing", "E-EXEC-005"),
        ] {
            let result = run_selector_then_next(selector);
            assert_eq!(diagnostic_codes(&result), [code], "{selector}");
            assert_eq!(evidence_ids(&result), ["TEST-EXEC-NEXT"], "{selector}");
            assert!(result.has_errors(), "{selector}");
        }
    }

    /// DS-1901, DS-1903, DS-1930, DS-2012: a runner that refuses one Test gets
    /// the diagnostic the refusal names, or none for DS-2012, no Evidence, and
    /// the next Test is still run and recorded.
    /// @vtest.id TEST-EXEC-REFUSED-TEST-RECORDS-NO-EVIDENCE-AND-THE-RUN-GOES-ON
    /// @vtest.covers VO-ADAPTER-GO-UNEXECUTABLE-TEST-DOES-NOT-STOP-THE-RUN, VO-ADAPTER-GO-UNEXECUTABLE-TEST-RECORDS-NO-EVIDENCE, VO-ADAPTER-GO-SAME-NAME-TESTS-RECORD-NO-EVIDENCE, VO-ADAPTER-GO-UNREADABLE-COUNTED-FILE-RECORDS-NO-EVIDENCE
    /// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
    /// @vtest.intent a command refused as not executable gives the warning W-EXEC-102, as not uniquely selectable the error E-EXEC-005, and as undetermined no diagnostic, each without Evidence for that Test, and the next Test of the run is recorded
    #[test]
    fn a_refused_test_has_no_evidence_and_the_run_goes_on_with_the_next_test() {
        for (selector, code, is_error) in [
            ("refuse-not-executable", Some("W-EXEC-102"), false),
            ("refuse-not-unique", Some("E-EXEC-005"), true),
            ("refuse-undetermined", None, false),
        ] {
            let result = run_selector_then_next(selector);
            assert_eq!(
                diagnostic_codes(&result),
                code.into_iter().collect::<Vec<_>>()
            );
            assert_eq!(result.has_errors(), is_error, "{selector}");
            assert_eq!(evidence_ids(&result), ["TEST-EXEC-NEXT"], "{selector}");
        }
    }

    fn runnable_test_with_target(id: &str, target_locator: Option<Locator>) -> RunnableTest {
        // round 2 レビュー指摘（開示要7、後半）: この fixture の
        // execution.adapter / location.adapter は、`rust_locator` が組む
        // target locator の adapter（"rust-cargo"）に合わせる。以前は
        // "fake-runner" のままで、target_locator が Some のとき DES-213
        // の不一致検査が毎回 W-EXEC-102 を発行していた（テストは
        // diagnostics を assert しないので握り潰されていた）。
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
                    adapter: AdapterId::new("rust-cargo"),
                    path: ProjectPath::new("fixture.test"),
                    locator: "fixed".to_owned(),
                    byte_range: SourceRange { start: 0, end: 1 },
                },
                content_hash: ContentHash::from_text(id),
                execution: ExecutionDescriptor {
                    adapter: AdapterId::new("rust-cargo"),
                    project: None,
                    suite: None,
                    selector: "fixed".to_owned(),
                },
            },
            target_hashes: Vec::new(),
            target_locators: target_locator.into_iter().collect(),
        }
    }

    // このテストの claim のうち「CoverageAdapter が返す method 名と
    // Target 別到達計測が target_coverage（method/result/targets/count）へ
    // そのまま写ること」自体を正本で検索したが該当する VO が見当たらない。
    // 候補: DES-352（adapter は DTO を返す責務を負う、という一般則の具体
    // 化）だが、「exec が adapter の出力をそのまま転記する」という writer
    // 側の配線自体を claim する条文は見つけられていない — 未確定。
    //
    // ただし count>0/count==0 の2ケースをそれぞれ checked:true と共に
    // 観測する部分は `VO-EXEC-TARGET-COVERAGE-COUNT-JUDGEMENT`
    // （DS-766/DS-767「計測されたtarget別countが正のときはchecked:true・
    // result:PASSとし、countが0のときはchecked:true・result:FAILとする」）
    // の claim をそのまま満たす。round 2 レビュー指摘（欠陥E-2）: この VO
    // の唯一の観測者が `CoverageTargetMeasurement`（`checked` field を
    // 持たない型）だけを assert する adapter-rust 側テストに変わっており、
    // `checked:true` 側の観測者が消えていた。本テストを count==0 の
    // ケースまで拡張し、covers を付けて観測者を復元する。
    /// @vtest.id TEST-EXEC-TARGET-COVERAGE-BUILT-FROM-COVERAGE-ADAPTER-MEASUREMENT
    /// @vtest.covers VO-EXEC-TARGET-COVERAGE-COUNT-JUDGEMENT
    /// @vtest.target crates/vtest-exec/src/lib.rs::target_coverage_from_measurements
    /// @vtest.intent CoverageAdapterが返すcount別到達計測が、checked:trueと共にPASS（count>0）/FAIL（count==0）としてtarget_coverageへ写ることを検証する
    #[test]
    fn target_coverage_is_built_from_a_fake_coverage_adapter_measurement() {
        for (count, expected_result) in [
            (3_u64, TargetCoverageResult::Pass),
            (0_u64, TargetCoverageResult::Fail),
        ] {
            let root = std::env::temp_dir()
                .join(format!("vtest-exec-coverage-{count}-{}", new_record_id()));
            fs::create_dir_all(&root).expect("create coverage fixture root");
            let target = rust_locator("src/lib.rs", "add");
            let test = runnable_test_with_target("TEST-EXEC-FAKE-COVERAGE", Some(target.clone()));
            let coverage = FakeCoverageAdapter {
                method: "fake-cov",
                availability: Ok(()),
                result: expected_result,
                count: Some(count),
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
            // DES-213 の adapter 整合検査を誘発しない fixture であること
            // （round 2 開示要7）も、この場で確かめる。
            assert!(result.diagnostics.is_empty());
            let target_coverage = &result.evidence[0].target_coverage;
            assert!(target_coverage.checked);
            assert_eq!(target_coverage.method.as_deref(), Some("fake-cov"));
            assert_eq!(target_coverage.result, Some(expected_result));
            assert_eq!(target_coverage.count, Some(count));
            assert_eq!(target_coverage.targets.len(), 1);
            assert_eq!(target_coverage.targets[0].target, target.to_string());
            assert_eq!(target_coverage.targets[0].result, expected_result);
            assert_eq!(target_coverage.targets[0].count, Some(count));
            fs::remove_dir_all(root).expect("remove coverage fixture root");
        }
    }

    /// @vtest.id TEST-EXEC-TARGET-COVERAGE-RECORDS-EVERY-DECLARED-TARGET
    /// @vtest.covers VO-EXEC-TARGET-COVERAGE-COUNT-JUDGEMENT
    /// @vtest.target crates/vtest-exec/src/lib.rs::target_coverage_from_measurements
    /// @vtest.intent 複数targetを宣言したTestで全targetを計測・記録し、1件未到達ならaggregateをFAIL、計測結果の無いtargetはUNKNOWNのentryとして残すことを確認する
    #[test]
    fn target_coverage_records_an_entry_for_every_declared_target() {
        let double = rust_locator("src/lib.rs", "double");
        let unused = rust_locator("src/lib.rs", "unused");
        let missing = rust_locator("src/lib.rs", "missing");
        let measurements = vec![
            CoverageTargetMeasurement {
                target: unused.clone(),
                result: TargetCoverageResult::Fail,
                count: Some(0),
            },
            CoverageTargetMeasurement {
                target: double.clone(),
                result: TargetCoverageResult::Pass,
                count: Some(2),
            },
        ];
        let coverage = target_coverage_from_measurements(
            "fake-cov",
            &[double.clone(), unused.clone(), missing.clone()],
            measurements.clone(),
        );
        assert!(coverage.checked);
        let entries: Vec<_> = coverage
            .targets
            .iter()
            .map(|entry| (entry.target.clone(), entry.result, entry.count))
            .collect();
        assert_eq!(
            entries,
            vec![
                (double.to_string(), TargetCoverageResult::Pass, Some(2)),
                (unused.to_string(), TargetCoverageResult::Fail, Some(0)),
                (missing.to_string(), TargetCoverageResult::Unknown, None),
            ]
        );
        assert_eq!(coverage.result, Some(TargetCoverageResult::Fail));
        assert_eq!(coverage.count, None);

        let both =
            target_coverage_from_measurements("fake-cov", &[double.clone(), unused], measurements);
        assert_eq!(both.result, Some(TargetCoverageResult::Fail));
        assert_eq!(both.count, Some(0));
        let reached = target_coverage_from_measurements(
            "fake-cov",
            std::slice::from_ref(&double),
            vec![CoverageTargetMeasurement {
                target: double.clone(),
                result: TargetCoverageResult::Pass,
                count: Some(2),
            }],
        );
        assert_eq!(reached.result, Some(TargetCoverageResult::Pass));
        assert_eq!(reached.count, Some(2));
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

    // -----------------------------------------------------------------
    // Registry-based dispatch (`run_tests`): E-ADAPTER-001/003/004
    // -----------------------------------------------------------------

    fn runnable_test_with_adapter(id: &str, adapter_id: &str) -> RunnableTest {
        RunnableTest {
            entity: TestEntity {
                id: vtest_model::TestId::new(id),
                covers: Vec::new(),
                targets: Vec::new(),
                intent: "registry dispatch fixture".to_owned(),
                input: None,
                expect: None,
                kind: None,
                cases: Vec::new(),
                related: Vec::new(),
                location: SourceLocation {
                    adapter: AdapterId::new(adapter_id),
                    path: ProjectPath::new("fixture.test"),
                    locator: id.to_owned(),
                    byte_range: SourceRange { start: 0, end: 1 },
                },
                content_hash: ContentHash::from_text(id),
                execution: ExecutionDescriptor {
                    adapter: AdapterId::new(adapter_id),
                    project: None,
                    suite: None,
                    selector: "fixed".to_owned(),
                },
            },
            target_hashes: Vec::new(),
            target_locators: Vec::new(),
        }
    }

    /// `fake-runner` capability を宣言・実装する registry fixture adapter。
    struct RunnerOnlyAdapter;

    impl vtest_adapter_api::Adapter for RunnerOnlyAdapter {
        fn descriptor(&self) -> vtest_adapter_api::AdapterDescriptor {
            vtest_adapter_api::AdapterDescriptor {
                id: "fake-runner".to_owned(),
                languages: vec!["fake".to_owned()],
                capabilities: vec![vtest_adapter_api::Capability::TestRunner],
                config_namespace: "fake-runner".to_owned(),
            }
        }
        fn as_test_runner(&self) -> Option<&dyn TestRunnerAdapter> {
            Some(&FixedResultRunner)
        }
    }

    /// discovery capability だけを宣言する registry fixture adapter —
    /// registry には登録されている（未登録ではない）が、runner capability
    /// を一切持たない。DS-1580「明示操作に必須のcapability欠落=
    /// E-ADAPTER-004」を、DS-1663の「未登録」（E-ADAPTER-001）と区別して
    /// 観測するための対照 fixture。
    struct NoRunnerCapabilityAdapter;

    impl vtest_adapter_api::Adapter for NoRunnerCapabilityAdapter {
        fn descriptor(&self) -> vtest_adapter_api::AdapterDescriptor {
            vtest_adapter_api::AdapterDescriptor {
                id: "no-runner".to_owned(),
                languages: vec!["fake".to_owned()],
                capabilities: Vec::new(),
                config_namespace: "no-runner".to_owned(),
            }
        }
        // discovery すら宣言・実装しない — この adapter は「登録済みだが
        // runner capability が無い」という一点だけを表す。
    }

    fn registry_with_runner_and_no_runner_adapters() -> AdapterRegistry {
        let mut registry = AdapterRegistry::new();
        registry
            .register(Box::new(RunnerOnlyAdapter))
            .expect("fake-runner must register cleanly");
        registry
            .register(Box::new(NoRunnerCapabilityAdapter))
            .expect("no-runner must register cleanly");
        registry
    }

    /// DS-1663/E-ADAPTER-001（未登録）・DS-1580/E-ADAPTER-004（登録済み
    /// だがrunner capability欠落）を、健全な Test と同じバッチの中で区別し、
    /// いずれも「他の Test は実行される」ことを確認する（brief: E-ADAPTER-004
    /// について明示。E-ADAPTER-001 も同じ理由で続行する、というこの PR の
    /// 導出をあわせて観測する）。
    /// @vtest.id TEST-EXEC-RUN-TESTS-SKIPS-UNREGISTERED-AND-CAPABILITY-MISSING-ADAPTERS
    /// @vtest.covers VO-EXEC-RUN-TESTS-UNREGISTERED-ADAPTER-E-ADAPTER-001, VO-EXEC-RUN-TESTS-MISSING-RUNNER-CAPABILITY-E-ADAPTER-004
    /// @vtest.target crates/vtest-exec/src/lib.rs::run_tests
    /// @vtest.intent verifies run_tests reports E-ADAPTER-001 for a Test declaring an unregistered adapter and E-ADAPTER-004 for a Test whose registered adapter has no Test Runner capability, producing no Evidence for either, while a healthy Test in the same batch still executes and gets Evidence
    #[test]
    fn run_tests_skips_unregistered_and_capability_missing_adapters_but_runs_the_rest() {
        let root =
            std::env::temp_dir().join(format!("vtest-exec-run-tests-skip-{}", new_record_id()));
        fs::create_dir_all(&root).expect("create fixture root");
        let registry = registry_with_runner_and_no_runner_adapters();
        let tests = vec![
            runnable_test_with_adapter("TEST-UNREGISTERED", "totally-unregistered-adapter"),
            runnable_test_with_adapter("TEST-NO-RUNNER-CAPABILITY", "no-runner"),
            runnable_test_with_adapter("TEST-HEALTHY", "fake-runner"),
        ];
        let result = run_tests(
            &root,
            &vtest_store::VerifyLayout::new(&root),
            &tests,
            true,
            &registry,
        )
        .expect("run_tests must not abort the whole batch for a skip-only condition");

        assert_eq!(
            result.evidence.len(),
            1,
            "only the healthy Test must produce Evidence"
        );
        assert_eq!(result.evidence[0].test_id.as_str(), "TEST-HEALTHY");

        let has_code = |code: &str, test_id: &str| {
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == code && diagnostic.message.contains(test_id))
        };
        assert!(
            has_code("E-ADAPTER-001", "TEST-UNREGISTERED"),
            "an unregistered adapter must be reported as E-ADAPTER-001: {:?}",
            result.diagnostics
        );
        assert!(
            has_code("E-ADAPTER-004", "TEST-NO-RUNNER-CAPABILITY"),
            "a registered adapter with no Test Runner capability must be reported as \
             E-ADAPTER-004: {:?}",
            result.diagnostics
        );
        fs::remove_dir_all(root).expect("remove fixture root");
    }

    /// registered adapter の `TestRunnerAdapter::command` が、この Test の
    /// execution descriptor を解釈できない（`TestRunnerError`）場合の
    /// 経路。DS-745/DS-923「Testのexecution descriptorと選択adapterが
    /// 不一致であることである」— この PR の導出（brief参照）は既存の
    /// `TestRunnerError`→`ExecutionError::Runner` 経路をそのまま
    /// E-ADAPTER-003 とコード付けし、バッチ全体を中断する既存の挙動は
    /// 変えない。
    struct DescriptorMismatchRunner;

    impl TestRunnerAdapter for DescriptorMismatchRunner {
        fn id(&self) -> &'static str {
            "descriptor-mismatch"
        }
        fn command(
            &self,
            _root: &Path,
            _execution: &ExecutionDescriptor,
            _coverage: bool,
            _coverage_output_path: Option<&Path>,
        ) -> Result<RunnerCommand, TestRunnerError> {
            Err(TestRunnerError::MissingProject)
        }
        fn parse(
            &self,
            _execution: &ExecutionDescriptor,
            _output: RunnerOutput<'_>,
        ) -> RunnerTestResult {
            unimplemented!("command always fails before parse is reached")
        }
    }

    struct DescriptorMismatchAdapter;

    impl vtest_adapter_api::Adapter for DescriptorMismatchAdapter {
        fn descriptor(&self) -> vtest_adapter_api::AdapterDescriptor {
            vtest_adapter_api::AdapterDescriptor {
                id: "descriptor-mismatch".to_owned(),
                languages: vec!["fake".to_owned()],
                capabilities: vec![vtest_adapter_api::Capability::TestRunner],
                config_namespace: "descriptor-mismatch".to_owned(),
            }
        }
        fn as_test_runner(&self) -> Option<&dyn TestRunnerAdapter> {
            Some(&DescriptorMismatchRunner)
        }
    }

    /// @vtest.id TEST-EXEC-RUN-TESTS-DESCRIPTOR-MISMATCH-IS-E-ADAPTER-003
    /// @vtest.covers VO-EXEC-RUN-TESTS-DESCRIPTOR-MISMATCH-E-ADAPTER-003
    /// @vtest.target crates/vtest-exec/src/lib.rs::ExecutionError::code
    /// @vtest.intent verifies a registered adapter whose runner cannot interpret the Test's execution descriptor surfaces as ExecutionError::Runner with code E-ADAPTER-003
    #[test]
    fn run_tests_reports_e_adapter_003_for_a_descriptor_the_runner_cannot_interpret() {
        let root =
            std::env::temp_dir().join(format!("vtest-exec-run-tests-mismatch-{}", new_record_id()));
        fs::create_dir_all(&root).expect("create fixture root");
        let mut registry = AdapterRegistry::new();
        registry
            .register(Box::new(DescriptorMismatchAdapter))
            .expect("descriptor-mismatch must register cleanly");
        let tests = vec![runnable_test_with_adapter(
            "TEST-MISMATCHED-DESCRIPTOR",
            "descriptor-mismatch",
        )];

        let error = run_tests(
            &root,
            &vtest_store::VerifyLayout::new(&root),
            &tests,
            true,
            &registry,
        )
        .expect_err("a runner that cannot interpret the descriptor must abort the batch");
        assert!(matches!(
            error,
            ExecutionError::Runner(TestRunnerError::MissingProject)
        ));
        assert_eq!(error.code(), Some("E-ADAPTER-003"));
        fs::remove_dir_all(root).expect("remove fixture root");
    }
    struct SnapshotRunner {
        change: bool,
        complete: bool,
        reverse: bool,
        calls: std::sync::atomic::AtomicUsize,
    }
    impl TestRunnerAdapter for SnapshotRunner {
        fn id(&self) -> &'static str {
            "synthetic"
        }
        fn execution_state(
            &self,
            _root: &Path,
            _execution: &ExecutionDescriptor,
            head: Option<&str>,
            runner_kind: &str,
        ) -> Option<vtest_model::ExecutionStateDraft> {
            let call = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let mut draft = vtest_model::ExecutionStateDraft {
                schema_id: "synthetic-inputs".into(),
                schema_version: "1".into(),
                complete: self.complete,
                head_revision: head.map(str::to_owned),
                runner_kind: runner_kind.into(),
                invocation: serde_json::json!(["synthetic", "test"]),
                toolchain_identity: "synthetic-toolchain".into(),
                effective_config: serde_json::Value::Null,
                inputs: vec![vtest_model::ExecutionInputDraft {
                    root_identity: "workspace".into(),
                    root_relative_path: "helper.input".into(),
                    kind: "file".into(),
                    bytes: vec![if self.change {
                        call.to_le_bytes()[0]
                    } else {
                        0
                    }],
                }],
            };
            let mut second = draft.inputs[0].clone();
            second.root_relative_path = "second.input".into();
            second.bytes = vec![42];
            draft.inputs.push(second);
            if self.reverse && call > 0 {
                draft.inputs.reverse();
            }
            Some(draft)
        }
        fn command(
            &self,
            root: &Path,
            execution: &ExecutionDescriptor,
            coverage: bool,
            output: Option<&Path>,
        ) -> Result<RunnerCommand, TestRunnerError> {
            FixedResultRunner.command(root, execution, coverage, output)
        }
        fn parse(
            &self,
            execution: &ExecutionDescriptor,
            output: RunnerOutput<'_>,
        ) -> RunnerTestResult {
            FixedResultRunner.parse(execution, output)
        }
    }

    /// DS-532 / DES-099 / DES-371: a changed input never produces Evidence;
    /// reordered manifests are equivalent, including incomplete snapshots.
    /// Synthetic inputs prove core needs neither Rust syntax nor rustc probes.
    /// @vtest.id TEST-EXEC-STATE-PREPOST-STABILITY
    /// @vtest.covers VO-EXEC-STATE-PREPOST-STABILITY
    /// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
    /// @vtest.intent execution input change rejects evidence and stable inputs are recorded
    #[test]
    fn execution_input_change_rejects_evidence_and_stable_inputs_are_recorded() {
        for (complete, reverse, change) in [
            (true, false, false),
            (true, false, true),
            (true, true, false),
            (true, true, true),
            (false, false, false),
            (false, false, true),
            (false, true, false),
            (false, true, true),
        ] {
            let root = std::env::temp_dir().join(format!("vtest-snapshot-{}", new_record_id()));
            fs::create_dir_all(&root).unwrap();
            assert!(Command::new("git")
                .current_dir(&root)
                .args(["init", "-q"])
                .status()
                .unwrap()
                .success());
            assert!(Command::new("git")
                .current_dir(&root)
                .args([
                    "-c",
                    "user.name=fixture",
                    "-c",
                    "user.email=fixture@example.com",
                    "-c",
                    "commit.gpgsign=false",
                    "commit",
                    "--allow-empty",
                    "-qm",
                    "fixture"
                ])
                .status()
                .unwrap()
                .success());
            let runner = SnapshotRunner {
                change,
                complete,
                reverse,
                calls: std::sync::atomic::AtomicUsize::new(0),
            };
            let test = runnable_test_with_target("TEST-SNAPSHOT", None);
            let layout = VerifyLayout::new(&root);
            let result = run_tests_with_runner(
                &root,
                &layout,
                &[test],
                true,
                &runner,
                &never_available_coverage("fast"),
            )
            .unwrap();
            assert_eq!(runner.calls.load(std::sync::atomic::Ordering::SeqCst), 2);
            if change {
                assert!(result.evidence.is_empty());
                assert!(result.diagnostics.iter().any(|d| d.code == "E-EXEC-004"));
                assert_eq!(fs::read_dir(layout.evidence_dir()).unwrap().count(), 0);
            } else {
                assert_eq!(result.evidence.len(), 1);
                assert_eq!(result.evidence[0].execution_state.complete, complete);
                assert_eq!(result.evidence[0].execution_state.hash.is_some(), complete);
                assert!(!result.diagnostics.iter().any(|d| d.code == "E-EXEC-004"));
                assert_eq!(fs::read_dir(layout.evidence_dir()).unwrap().count(), 1);
                assert_eq!(
                    result.evidence[0].execution_state.schema,
                    "synthetic-inputs-v1"
                );
            }
            fs::remove_dir_all(root).unwrap();
        }
    }

    struct SequenceDraftRunner {
        before: Option<vtest_model::ExecutionStateDraft>,
        after: Option<vtest_model::ExecutionStateDraft>,
        calls: std::sync::atomic::AtomicUsize,
    }

    impl TestRunnerAdapter for SequenceDraftRunner {
        fn id(&self) -> &'static str {
            "synthetic"
        }

        fn execution_state(
            &self,
            _root: &Path,
            _execution: &ExecutionDescriptor,
            _head: Option<&str>,
            _runner_kind: &str,
        ) -> Option<vtest_model::ExecutionStateDraft> {
            let call = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if call == 0 {
                self.before.clone()
            } else {
                self.after.clone()
            }
        }

        fn command(
            &self,
            root: &Path,
            execution: &ExecutionDescriptor,
            coverage: bool,
            output: Option<&Path>,
        ) -> Result<RunnerCommand, TestRunnerError> {
            FixedResultRunner.command(root, execution, coverage, output)
        }

        fn parse(
            &self,
            execution: &ExecutionDescriptor,
            output: RunnerOutput<'_>,
        ) -> RunnerTestResult {
            FixedResultRunner.parse(execution, output)
        }
    }

    fn base_draft() -> vtest_model::ExecutionStateDraft {
        vtest_model::ExecutionStateDraft {
            schema_id: "synthetic-inputs".into(),
            schema_version: "1".into(),
            complete: true,
            head_revision: Some("abc1234".into()),
            runner_kind: "fake-runner".into(),
            invocation: serde_json::json!(["synthetic", "test"]),
            toolchain_identity: "synthetic-toolchain-1.0".into(),
            effective_config: serde_json::json!({
                "environment": {"RUSTFLAGS": "--cfg initial"},
                "cargo_config": []
            }),
            inputs: vec![vtest_model::ExecutionInputDraft {
                root_identity: "workspace-root".into(),
                root_relative_path: "src/lib.rs".into(),
                kind: "file".into(),
                bytes: b"initial".to_vec(),
            }],
        }
    }

    fn run_with_drafts(
        root: &Path,
        before: Option<vtest_model::ExecutionStateDraft>,
        after: Option<vtest_model::ExecutionStateDraft>,
    ) -> ExecutionResult {
        let runner = SequenceDraftRunner {
            before,
            after,
            calls: std::sync::atomic::AtomicUsize::new(0),
        };
        let test = runnable_test_with_target("TEST-DIFF", None);
        let layout = VerifyLayout::new(root);
        run_tests_with_runner(
            root,
            &layout,
            &[test],
            true,
            &runner,
            &never_available_coverage("fast"),
        )
        .unwrap()
    }

    /// @vtest.id TEST-EXEC-STATE-DIFF-ADDED-REMOVED-MODIFIED
    /// @vtest.covers VO-EXEC-STATE-PREPOST-STABILITY
    /// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
    /// @vtest.intent E-EXEC-004 names the relative paths of inputs added, removed, or modified during execution without emitting file contents
    #[test]
    fn e_exec_004_reports_added_removed_and_modified_relative_paths() {
        let root = std::env::temp_dir().join(format!("vtest-diff-paths-{}", new_record_id()));
        fs::create_dir_all(&root).unwrap();

        let mut before = base_draft();
        before.inputs.push(vtest_model::ExecutionInputDraft {
            root_identity: "workspace-root".into(),
            root_relative_path: "stale.tmp".into(),
            kind: "file".into(),
            bytes: b"SECRET_REMOVED_CONTENT".to_vec(),
        });

        let mut after = base_draft();
        after.inputs[0].bytes = b"SECRET_MODIFIED_CONTENT".to_vec();
        after.inputs.push(vtest_model::ExecutionInputDraft {
            root_identity: "workspace-root".into(),
            root_relative_path: "generated/output.txt".into(),
            kind: "file".into(),
            bytes: b"SECRET_ADDED_CONTENT".to_vec(),
        });

        let result = run_with_drafts(&root, Some(before), Some(after));
        assert!(result.evidence.is_empty());
        let diag = result
            .diagnostics
            .iter()
            .find(|d| d.code == "E-EXEC-004")
            .expect("E-EXEC-004 diagnostic");
        assert!(
            diag.message
                .contains("added: workspace-root:generated/output.txt"),
            "missing added path in: {}",
            diag.message
        );
        assert!(
            diag.message.contains("removed: workspace-root:stale.tmp"),
            "missing removed path in: {}",
            diag.message
        );
        assert!(
            diag.message.contains("modified: workspace-root:src/lib.rs"),
            "missing modified path in: {}",
            diag.message
        );
        for secret in [
            "SECRET_REMOVED_CONTENT",
            "SECRET_MODIFIED_CONTENT",
            "SECRET_ADDED_CONTENT",
        ] {
            assert!(
                !diag.message.contains(secret),
                "diagnostic leaked file content {secret}: {}",
                diag.message
            );
        }
        assert!(
            !diag.message.contains(&root.to_string_lossy().into_owned()),
            "diagnostic leaked root absolute path: {}",
            diag.message
        );

        fs::remove_dir_all(root).unwrap();
    }

    /// @vtest.id TEST-EXEC-STATE-DIFF-TRUNCATION-AND-ORDER
    /// @vtest.covers VO-EXEC-STATE-PREPOST-STABILITY
    /// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
    /// @vtest.intent E-EXEC-004 lists changed input paths in deterministic sorted order, caps listed entries, and reports the omitted count
    #[test]
    fn e_exec_004_truncates_input_paths_above_limit_with_omitted_count() {
        let (message, evidence_empty) = added_files_message("limit", 15);
        assert!(evidence_empty);
        // The listed entries are the first ten in lexicographic order, in order.
        let expected = (1..=10)
            .map(|index| format!("workspace-root:generated/file-{index:02}.txt"))
            .collect::<Vec<_>>()
            .join(", ");
        assert!(
            message.contains(&format!("added: {expected} (+5 omitted)")),
            "{message}"
        );
        for index in 11..=15 {
            let omitted = format!("generated/file-{index:02}.txt");
            assert!(
                !message.contains(&omitted),
                "expected {omitted} to be truncated in {message}"
            );
        }
    }

    /// @vtest.id TEST-EXEC-STATE-DIFF-LIMIT-BOUNDARY
    /// @vtest.covers VO-EXEC-STATE-PREPOST-STABILITY
    /// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
    /// @vtest.intent E-EXEC-004 lists exactly the maximum number of changed input paths without an omitted count, and omits exactly one when one more path changed
    #[test]
    fn e_exec_004_limit_boundary_exact_and_one_over() {
        let (exact, evidence_empty) = added_files_message("exact", 10);
        assert!(evidence_empty);
        assert!(exact.contains("generated/file-10.txt"), "{exact}");
        assert!(!exact.contains("omitted"), "{exact}");

        let (over, evidence_empty) = added_files_message("over", 11);
        assert!(evidence_empty);
        assert!(over.contains("generated/file-10.txt"), "{over}");
        assert!(!over.contains("generated/file-11.txt"), "{over}");
        assert!(over.contains("(+1 omitted)"), "{over}");
    }

    /// Runs with `count` added files (pushed in reverse order) and returns the
    /// E-EXEC-004 message and whether no Evidence was produced.
    fn added_files_message(tag: &str, count: u8) -> (String, bool) {
        let root = std::env::temp_dir().join(format!("vtest-diff-{tag}-{}", new_record_id()));
        fs::create_dir_all(&root).unwrap();
        let before = base_draft();
        let mut after = base_draft();
        for index in (1..=count).rev() {
            after.inputs.push(vtest_model::ExecutionInputDraft {
                root_identity: "workspace-root".into(),
                root_relative_path: format!("generated/file-{index:02}.txt"),
                kind: "file".into(),
                bytes: vec![index],
            });
        }
        let result = run_with_drafts(&root, Some(before), Some(after));
        fs::remove_dir_all(&root).unwrap();
        let message = result
            .diagnostics
            .iter()
            .find(|d| d.code == "E-EXEC-004")
            .expect("E-EXEC-004 diagnostic")
            .message
            .clone();
        (message, result.evidence.is_empty())
    }

    /// @vtest.id TEST-EXEC-STATE-DIFF-REDACTION-AND-ELEMENTS
    /// @vtest.covers VO-EXEC-STATE-PREPOST-STABILITY
    /// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
    /// @vtest.intent E-EXEC-004 names changed non-input elements and never leaks absolute paths, file contents, or environment variable values
    #[test]
    fn e_exec_004_names_changed_elements_without_leaking_paths_or_env_values() {
        let root = std::env::temp_dir().join(format!("vtest-diff-redact-{}", new_record_id()));
        fs::create_dir_all(&root).unwrap();

        let before = base_draft();
        let mut after = base_draft();
        after.head_revision = Some("def5678".into());
        after.toolchain_identity = "synthetic-toolchain-2.0".into();
        after.invocation = serde_json::json!(["synthetic", "test", "--extra"]);
        after.effective_config = serde_json::json!({
            "environment": {"RUSTFLAGS": "--cfg SECRET_ENV_VALUE"},
            "cargo_config": [{"location": "cargo-home/config.toml", "text": "SECRET_CFG_TEXT"}]
        });
        after.inputs.push(vtest_model::ExecutionInputDraft {
            root_identity: "workspace-root".into(),
            root_relative_path: root.join("inside/added.txt").to_string_lossy().into_owned(),
            kind: "file".into(),
            bytes: b"SECRET_INSIDE_BYTES".to_vec(),
        });
        after.inputs.push(vtest_model::ExecutionInputDraft {
            root_identity: "workspace-root".into(),
            root_relative_path: format!(
                "{}/inside/slash.txt",
                root.to_string_lossy().replace('\\', "/")
            ),
            kind: "file".into(),
            bytes: b"SECRET_SLASH_BYTES".to_vec(),
        });
        after.inputs.push(vtest_model::ExecutionInputDraft {
            root_identity: "workspace-root".into(),
            root_relative_path: "/secret/outside/private.key".into(),
            kind: "file".into(),
            bytes: b"SECRET_OUTSIDE_BYTES".to_vec(),
        });

        let result = run_with_drafts(&root, Some(before), Some(after));
        assert!(result.evidence.is_empty());
        let diag = result
            .diagnostics
            .iter()
            .find(|d| d.code == "E-EXEC-004")
            .expect("E-EXEC-004 diagnostic");

        for element in ["head_revision", "invocation", "toolchain", "config"] {
            assert!(
                diag.message.contains(element),
                "expected changed element {element} in {}",
                diag.message
            );
        }
        assert!(
            diag.message.contains("workspace-root:<non-relative-path>"),
            "expected redacted placeholder for non-relative paths in {}",
            diag.message
        );
        let root_native = root.to_string_lossy().into_owned();
        let root_slash = root_native.replace('\\', "/");
        for forbidden in [
            root_native.as_str(),
            root_slash.as_str(),
            "inside/added.txt",
            "inside/slash.txt",
            "/secret/outside/private.key",
            "private.key",
            "SECRET_ENV_VALUE",
            "SECRET_CFG_TEXT",
            "SECRET_INSIDE_BYTES",
            "SECRET_OUTSIDE_BYTES",
        ] {
            assert!(
                !diag.message.contains(forbidden),
                "diagnostic leaked forbidden value {forbidden:?}: {}",
                diag.message
            );
        }

        fs::remove_dir_all(root).unwrap();
    }
}
