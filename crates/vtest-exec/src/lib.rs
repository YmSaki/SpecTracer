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
    RunnerInfo, TargetExecution, TestEntity, TestResult,
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

// TODO: Review fail-closed handling of an absent/unresolved suite. Execution
// must not silently fall back to an unscoped Cargo target. This TODO moved
// here from `vtest_model::TestEntity` (旧 `test_target: TestTarget` field,
// `TestTarget::Unknown` arm) when `TestTarget` moved out of `vtest-model`
// into `vtest-adapter-rust` — the underlying concern (this crate silently
// omitting `--lib`/`--bin`/`--test` and running an unscoped `cargo test`)
// is unresolved either way, and now also applies to `execution.project`
// being absent (see `suite_args` below).

/// 本冊 §9.2「`rust-cargo` adapterは`TestEntity.execution`を次のCargo実行
/// 座標として解釈する」の、この crate 側での再現。
///
/// **注意（`validate_desired_test` と同型の、上流未報告の欠陥）**:
/// 本冊:688「coreは `project`、`suite.kind`、`suite.name`、`selector` の
/// 文字列を解釈しない」の「core」に `vtest-exec` が含まれるなら、この
/// 関数（`suite.kind` の文字列 `"lib"`/`"bin"`/`"integration"` を読んで
/// 分岐する）はその禁止の対象になる。本冊 §9.2 はこの解釈を
/// `rust-cargo` `TestRunnerAdapter`（＝ `rust-cargo` adapter 自身）の
/// 責務と書いているが、`vtest-exec` は workspace 構成上 adapter crate
/// （`vtest-adapter-rust`）とは別 crate であり、この reshape 以前から
/// 一貫してCargoコマンドを直接組み立ててきた（`cargo_command` 等、この
/// 関数の前身）。`TestRunnerAdapter` の実装場所をこの crate から
/// `vtest-adapter-rust` へ移すことはこの PR の範囲外（詳細設計に新しい
/// trait／DTOが無く、`validate_desired_test` の除去理由と同じ形の
/// 論点）。この関数は既存の振る舞い（旧 `TestTarget` enum による分岐）を
/// 型が変わった後も等価に保つだけで、新しい解釈を追加しない。
///
/// `suite` が `None`（`kind` が `"lib"`/`"bin"`/`"integration"` のいずれ
/// でもない、または `suite` 自体が無い）場合と、`kind` が `"bin"`/
/// `"integration"` なのに `name` が無い場合は、どちらも旧
/// `TestTarget::Unknown` と同じ「フラグを付けない」扱いにする（unscoped
/// `cargo test` — 上のTODO参照）。
fn suite_args(test: &TestEntity) -> Vec<String> {
    let Some(suite) = test.execution.suite.as_ref() else {
        return Vec::new();
    };
    match suite.kind.as_str() {
        "lib" => vec!["--lib".to_owned()],
        "bin" => suite
            .name
            .as_deref()
            .map(|name| vec!["--bin".to_owned(), name.to_owned()])
            .unwrap_or_default(),
        "integration" => suite
            .name
            .as_deref()
            .map(|name| vec!["--test".to_owned(), name.to_owned()])
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn cargo_command(root: &Path, test: &TestEntity) -> Command {
    let mut command = Command::new("cargo");
    command
        .current_dir(root)
        .arg("test")
        .arg("-p")
        .arg(test.execution.project.as_deref().unwrap_or_default());
    command.args(suite_args(test));
    command.args(["--", "--exact", &test.execution.selector]);
    command
}

fn cargo_llvm_cov_command(root: &Path, test: &TestEntity, output_path: &Path) -> Command {
    let mut command = Command::new("cargo");
    command
        .current_dir(root)
        .args(["llvm-cov", "test", "-p"])
        .arg(test.execution.project.as_deref().unwrap_or_default());
    command.args(suite_args(test));
    command
        .arg("--json")
        .arg("--output-path")
        .arg(output_path)
        .args(["--", "--exact", &test.execution.selector]);
    command
}

fn command_string(test: &TestEntity) -> String {
    format!(
        "cargo test -p {} {} -- --exact {}",
        test.execution.project.as_deref().unwrap_or_default(),
        suite_args(test).join(" "),
        test.execution.selector
    )
}

fn llvm_cov_command_string(root: &Path, test: &TestEntity, output_path: &Path) -> String {
    let output_path = output_path
        .strip_prefix(root)
        .unwrap_or(output_path)
        .to_string_lossy()
        .replace('\\', "/");
    format!(
        "cargo llvm-cov test -p {} {} --json --output-path {} -- --exact {}",
        test.execution.project.as_deref().unwrap_or_default(),
        suite_args(test).join(" "),
        output_path,
        test.execution.selector
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
        "id: {id}\ntest_id: {test_id}\nresult: {result}\nexecuted_at: {executed_at}\nrevision:\n  commit: {commit}\n  dirty: {dirty}\nhashes:\n  test_fn: {test_fn}\n  target_fn: {target_fn}\n  target_fns:\n{target_fns}runner:\n  kind: {kind}\n  command: {command}\n  exit_code: {exit_code}\ntarget_execution:\n  checked: {checked}\n  method: {method}\n  result: {target_result}\n  count: {count}\nlog_ref: {log_ref}\n",
        id = yaml_scalar(&record.id),
        test_id = yaml_scalar(record.test_id.as_str()),
        result = yaml_scalar(match record.result { TestResult::Pass => "PASS", TestResult::Fail => "FAIL" }),
        executed_at = yaml_scalar(&record.executed_at),
        commit = record.revision.commit.as_deref().map(yaml_scalar).unwrap_or_else(|| "null".to_owned()),
        dirty = record.revision.dirty,
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
    use vtest_model::AdapterId;

    fn rust_locator(path: &str, item_path: &str) -> Locator {
        Locator {
            adapter: AdapterId::new("rust-cargo"),
            value: format!("{path}::{item_path}"),
        }
    }

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
