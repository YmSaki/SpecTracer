//! Rust/Cargo `TestRunnerAdapter`: cargo test execution and llvm-cov target
//! coverage attribution. Returns a hash-free `RunnerOutcome`; the core owns
//! Evidence generation, log persistence, and every fail-closed decision.

use std::{fs, path::Path, process::Command};

use vtest_adapter_api::{
    AdapterError, ExecutionInputDraft, ExecutionStateDraft, RunnerObservation, RunnerOutcome,
    TestRunnerAdapter,
};
use vtest_model::{
    CanonicalProjection, CheckValue, Diagnostic, RunnerInfo, TargetExecution, TestEntity,
    TestResult,
};

use crate::discovery::Locator;

/// A Rust source function coordinate used to match llvm-cov output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RustLocator {
    pub path: String,
    pub item_path: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ObservedResult {
    Pass,
    Fail,
    Ignored,
}

pub(crate) fn parse_result(output: &str, filter: &str) -> Option<ObservedResult> {
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

pub(crate) fn cargo_command(root: &Path, test: &TestEntity) -> Command {
    let mut command = Command::new("cargo");
    command
        .current_dir(root)
        .arg("test")
        .arg("-p")
        .arg(test.execution.project.as_deref().unwrap_or_default());
    match test
        .execution
        .suite
        .as_ref()
        .map(|suite| suite.kind.as_str())
    {
        Some("lib") => {
            command.arg("--lib");
        }
        Some("bin") => {
            if let Some(name) = test
                .execution
                .suite
                .as_ref()
                .and_then(|suite| suite.name.as_ref())
            {
                command.arg("--bin").arg(name);
            }
        }
        Some("integration") => {
            if let Some(name) = test
                .execution
                .suite
                .as_ref()
                .and_then(|suite| suite.name.as_ref())
            {
                command.arg("--test").arg(name);
            }
        }
        _ => {}
    }
    command.args(["--", "--exact", &test.execution.selector]);
    command
}

pub(crate) fn cargo_llvm_cov_command(
    root: &Path,
    test: &TestEntity,
    output_path: &Path,
) -> Command {
    let mut command = Command::new("cargo");
    command
        .current_dir(root)
        .args(["llvm-cov", "test", "-p"])
        .arg(test.execution.project.as_deref().unwrap_or_default());
    match test
        .execution
        .suite
        .as_ref()
        .map(|suite| suite.kind.as_str())
    {
        Some("lib") => {
            command.arg("--lib");
        }
        Some("bin") => {
            if let Some(name) = test
                .execution
                .suite
                .as_ref()
                .and_then(|suite| suite.name.as_ref())
            {
                command.arg("--bin").arg(name);
            }
        }
        Some("integration") => {
            if let Some(name) = test
                .execution
                .suite
                .as_ref()
                .and_then(|suite| suite.name.as_ref())
            {
                command.arg("--test").arg(name);
            }
        }
        _ => {}
    }
    command
        .arg("--json")
        .arg("--output-path")
        .arg(output_path)
        .args(["--", "--exact", &test.execution.selector]);
    command
}

pub(crate) fn command_string(test: &TestEntity) -> String {
    let target = rust_suite_argument(test);
    format!(
        "cargo test -p {} {} -- --exact {}",
        test.execution.project.as_deref().unwrap_or_default(),
        target,
        test.execution.selector
    )
}

pub(crate) fn llvm_cov_command_string(
    root: &Path,
    test: &TestEntity,
    output_path: &Path,
) -> String {
    let target = rust_suite_argument(test);
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

pub(crate) fn rust_suite_argument(test: &TestEntity) -> String {
    match test.execution.suite.as_ref() {
        Some(suite) if suite.kind == "lib" => "--lib".to_owned(),
        Some(suite) if suite.kind == "bin" => suite
            .name
            .as_ref()
            .map(|name| format!("--bin {name}"))
            .unwrap_or_default(),
        Some(suite) if suite.kind == "integration" => suite
            .name
            .as_ref()
            .map(|name| format!("--test {name}"))
            .unwrap_or_default(),
        _ => String::new(),
    }
}

pub(crate) fn cargo_llvm_cov_available(root: &Path) -> bool {
    Command::new("cargo")
        .current_dir(root)
        .args(["llvm-cov", "--version"])
        .output()
        .is_ok_and(|output| output.status.success())
}

pub(crate) fn target_execution_from_coverage(
    coverage_path: &Path,
    target: Option<&RustLocator>,
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

pub(crate) fn llvm_cov_function_count(output: &str, target: &RustLocator) -> Option<u64> {
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

pub(crate) fn llvm_name_matches(name: &str, item_path: &str) -> bool {
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

pub(crate) fn llvm_filenames_match(function: &serde_json::Value, target_path: &str) -> bool {
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

pub(crate) fn path_suffix_matches(candidate: &str, expected: &str) -> bool {
    candidate
        .replace('\\', "/")
        .ends_with(&expected.replace('\\', "/"))
}

pub(crate) fn not_checked_target_execution() -> TargetExecution {
    TargetExecution {
        checked: false,
        method: None,
        result: None,
        targets: Vec::new(),
        compatibility_count: None,
    }
}

pub(crate) fn measured_target_execution(count: u64) -> TargetExecution {
    TargetExecution {
        checked: true,
        method: Some("llvm-cov".to_owned()),
        result: Some(if count > 0 {
            CheckValue::Pass
        } else {
            CheckValue::Fail
        }),
        targets: Vec::new(),
        compatibility_count: Some(count),
    }
}

pub(crate) fn unavailable_target_execution() -> (TargetExecution, Diagnostic) {
    (
        not_checked_target_execution(),
        Diagnostic::warning(
            "W-EXEC-101",
            "cargo-llvm-cov is unavailable; target_execution is NOT_CHECKED",
        ),
    )
}

pub(crate) fn unknown_target_execution() -> TargetExecution {
    TargetExecution {
        checked: true,
        method: Some("llvm-cov".to_owned()),
        result: Some(CheckValue::Unknown),
        targets: Vec::new(),
        compatibility_count: None,
    }
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
        let target = RustLocator {
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

        let absent = RustLocator {
            path: "src/lib.rs".to_owned(),
            item_path: "subtract".to_owned(),
        };
        assert_eq!(llvm_cov_function_count(output, &absent), None);
    }

    #[test]
    fn llvm_cov_zero_count_is_preserved_as_a_measured_failure() {
        let target = RustLocator {
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
        assert_eq!(target_execution.result, None);
        assert_eq!(target_execution.compatibility_count, None);
        assert_eq!(diagnostic.code, "W-EXEC-101");
    }

    #[test]
    fn measured_target_execution_requires_a_positive_count() {
        let called = measured_target_execution(1);
        assert!(called.checked);
        assert_eq!(called.result, Some(CheckValue::Pass));
        assert_eq!(called.compatibility_count, Some(1));

        let not_called = measured_target_execution(0);
        assert!(not_called.checked);
        assert_eq!(not_called.result, Some(CheckValue::Fail));
        assert_eq!(not_called.compatibility_count, Some(0));
    }
}

// ---------------------------------------------------------------------------
// TestRunnerAdapter implementation
// ---------------------------------------------------------------------------

/// The built-in `rust-cargo` test runner.
#[derive(Default)]
pub struct RustCargoRunner;

/// The core folds `fast` into the execution-config projection as
/// `coverage = "off"`, so the adapter only obeys the projection.
fn coverage_is_off(config: &CanonicalProjection) -> bool {
    match config {
        CanonicalProjection::Map(map) => {
            matches!(map.get("coverage"), Some(CanonicalProjection::String(value)) if value == "off")
        }
        _ => true,
    }
}

/// Resolve the declared target to a Rust source coordinate for llvm-cov
/// attribution. Locator targets are parsed directly; SRC-ID targets are not
/// resolved here (coverage falls back to UNKNOWN), matching the safe default.
fn resolve_runner_target(test: &TestEntity) -> Option<RustLocator> {
    match test.targets.first()? {
        vtest_model::TargetRef::Locator { adapter, value }
            if adapter.as_str() == crate::RUST_CARGO_ADAPTER_ID =>
        {
            let locator = Locator::parse(value)?;
            Some(RustLocator {
                path: locator.path,
                item_path: locator.item_path,
            })
        }
        _ => None,
    }
}

/// The `rust-cargo-execution-state-v1` schema identity.
const EXECUTION_STATE_SCHEMA: &str = "rust-cargo-execution-state-v1";
const EXECUTION_STATE_VERSION: &str = "1";

/// Build the Execution State draft: the repository input manifest (the file
/// tree of the executed package, excluding generated products), the canonical
/// cargo invocation, the toolchain identity, and the HEAD revision. `complete`
/// is only true when the manifest was enumerated in full; the core forbids a
/// fresh `evidence_validity` over an incomplete state.
///
/// Called BEFORE cargo launches so the manifest bytes are the pre-run snapshot
/// the core compares against the post-run state for E-EXEC-004.
fn build_execution_state(
    root: &Path,
    config: &CanonicalProjection,
    test: &TestEntity,
    runner_kind: &str,
) -> ExecutionStateDraft {
    let inputs = collect_manifest_inputs(root);
    let complete = inputs.is_some();
    ExecutionStateDraft {
        schema_id: EXECUTION_STATE_SCHEMA.to_owned(),
        schema_version: EXECUTION_STATE_VERSION.to_owned(),
        complete,
        head_revision: git_head(root),
        runner_kind: runner_kind.to_owned(),
        invocation: invocation_projection(test),
        toolchain_identity: rustc_identity(root),
        effective_config: config.clone(),
        inputs: inputs.unwrap_or_default(),
    }
}

/// Enumerate every input file of the executed package as an `ExecutionInputDraft`.
/// Generated products (`.git/`, `.verify/`, and the Cargo `target/` directory)
/// are excluded, or every run would mutate its own inputs and self-trigger
/// E-EXEC-004. Returns `None` if any directory or file could not be read, which
/// marks the snapshot incomplete.
fn collect_manifest_inputs(root: &Path) -> Option<Vec<ExecutionInputDraft>> {
    let root_identity = root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("root")
        .to_owned();
    let mut inputs = Vec::new();
    walk_manifest(root, root, &root_identity, &mut inputs)?;
    inputs.sort_by(|left, right| left.root_relative_path.cmp(&right.root_relative_path));
    Some(inputs)
}

fn walk_manifest(
    root: &Path,
    dir: &Path,
    root_identity: &str,
    inputs: &mut Vec<ExecutionInputDraft>,
) -> Option<()> {
    for entry in fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        let name = entry.file_name();
        let name = name.to_str()?;
        let path = entry.path();
        let file_type = entry.file_type().ok()?;
        if file_type.is_dir() {
            if matches!(name, ".git" | ".verify" | "target") {
                continue;
            }
            walk_manifest(root, &path, root_identity, inputs)?;
        } else if file_type.is_file() {
            let relative = path
                .strip_prefix(root)
                .ok()?
                .to_string_lossy()
                .replace('\\', "/");
            let bytes = fs::read(&path).ok()?;
            inputs.push(ExecutionInputDraft {
                root_identity: root_identity.to_owned(),
                root_relative_path: relative.clone(),
                kind: classify_input_kind(&relative),
                bytes,
            });
        }
    }
    Some(())
}

fn classify_input_kind(relative: &str) -> String {
    match relative.rsplit('.').next() {
        Some("rs") => "rust-source",
        Some("toml") => "cargo-manifest",
        Some("lock") => "cargo-lockfile",
        _ => "resource",
    }
    .to_owned()
}

/// The canonical cargo invocation coordinate, machine-independent: package,
/// suite, and selector only. Absolute paths and the display command stay out.
fn invocation_projection(test: &TestEntity) -> CanonicalProjection {
    let mut map = std::collections::BTreeMap::new();
    map.insert(
        "package".to_owned(),
        match test.execution.project.as_deref() {
            Some(project) => CanonicalProjection::String(project.to_owned()),
            None => CanonicalProjection::Null,
        },
    );
    map.insert(
        "suite".to_owned(),
        match test.execution.suite.as_ref() {
            Some(suite) => CanonicalProjection::Map(std::collections::BTreeMap::from([
                (
                    "kind".to_owned(),
                    CanonicalProjection::String(suite.kind.clone()),
                ),
                (
                    "name".to_owned(),
                    match suite.name.as_deref() {
                        Some(name) => CanonicalProjection::String(name.to_owned()),
                        None => CanonicalProjection::Null,
                    },
                ),
            ])),
            None => CanonicalProjection::Null,
        },
    );
    map.insert(
        "selector".to_owned(),
        CanonicalProjection::String(test.execution.selector.clone()),
    );
    CanonicalProjection::Map(map)
}

fn git_head(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let commit = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (!commit.is_empty()).then_some(commit)
}

fn rustc_identity(root: &Path) -> String {
    Command::new("rustc")
        .current_dir(root)
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .unwrap_or_default()
}

impl TestRunnerAdapter for RustCargoRunner {
    fn run(
        &self,
        root: &Path,
        config: &CanonicalProjection,
        test: &TestEntity,
    ) -> Result<RunnerOutcome, AdapterError> {
        let fast = coverage_is_off(config);
        let llvm_cov_available = !fast && cargo_llvm_cov_available(root);
        let coverage_path = llvm_cov_available.then(|| {
            std::env::temp_dir().join(format!(
                "vtest-cov-{}-{}.json",
                std::process::id(),
                test.id.as_str()
            ))
        });
        let (mut command, command_line, runner_kind) = if let Some(coverage_path) = &coverage_path {
            (
                cargo_llvm_cov_command(root, test, coverage_path),
                llvm_cov_command_string(root, test, coverage_path),
                "cargo-llvm-cov",
            )
        } else {
            (
                cargo_command(root, test),
                command_string(test),
                "cargo-test",
            )
        };
        // Snapshot the Execution State (input manifest included) BEFORE cargo
        // runs, so the core can compare it against the post-run state.
        let execution_state = build_execution_state(root, config, test, runner_kind);
        let output = command.output().map_err(|error| {
            AdapterError::Operation(format!("cargo invocation failed: {error}"))
        })?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let log = format!("$ {command_line}\n{stdout}{stderr}").into_bytes();
        let runner = RunnerInfo {
            kind: runner_kind.to_owned(),
            command: command_line,
            exit_code: output.status.code().unwrap_or(-1),
        };

        match parse_result(&stdout, &test.execution.selector) {
            Some(ObservedResult::Ignored) => Ok(RunnerOutcome::Ignored { runner, log }),
            None => Ok(RunnerOutcome::MissingResult { runner, log }),
            Some(observed) => {
                let result = if matches!(observed, ObservedResult::Pass) {
                    TestResult::Pass
                } else {
                    TestResult::Fail
                };
                let target_execution = if fast {
                    not_checked_target_execution()
                } else if let Some(coverage_path) = &coverage_path {
                    target_execution_from_coverage(
                        coverage_path,
                        resolve_runner_target(test).as_ref(),
                    )
                } else {
                    unavailable_target_execution().0
                };
                Ok(RunnerOutcome::Completed(Box::new(RunnerObservation {
                    result,
                    runner,
                    target_execution,
                    execution_state,
                    log,
                })))
            }
        }
    }
}
