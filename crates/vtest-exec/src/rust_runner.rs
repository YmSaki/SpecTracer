//! Rust/Cargo test runner and llvm-cov coverage attribution.
//!
//! This module holds the language-specific runner logic isolated out of the
//! Evidence orchestrator; it moves to vtest-adapter-rust behind
//! TestRunnerAdapter / CoverageAdapter in a later increment.

use std::{fs, path::Path, process::Command};

use vtest_model::{CheckValue, Diagnostic, TargetExecution, TestEntity};

use crate::RustLocator;

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
