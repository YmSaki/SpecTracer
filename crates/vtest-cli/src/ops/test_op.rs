//! Canonical Structured Test Operation entry points (`vtest test create` /
//! `vtest test edit`, SPEC-385 / SPEC-386), shared between the CLI's
//! `run_test` wrapper and `vtest-mcp`'s `test_create` / `test_edit` tools so
//! both return the same `data` / `diagnostics` for the same input (DS-1563).
//!
//! The syntax work itself belongs to the Test's adapter (BD-256, BD-310) and
//! lives in `vtest_scan::operations`; this module only adapts inputs and
//! shapes the JSON result: the Test ID, the insertion / replacement position
//! and a diff (DS-1206, DS-1207). A failed operation leaves every file
//! unchanged (DS-1547, DS-1555) and returns no `data`.

use std::{collections::BTreeMap, path::Path};

use vtest_adapter_api::AdapterRegistry;
use vtest_model::{Diagnostic, ExitCode};
use vtest_scan::operations::{create_test, edit_test, TestMutationResult};
use vtest_store::{load_config, FormAnswers, FormValue};

/// `vtest test create` / `test_create`.
pub fn create(
    root: &Path,
    registry: &AdapterRegistry,
    form: &str,
    answers: &FormAnswers,
    explicit_id: Option<&str>,
    dry_run: bool,
) -> (ExitCode, serde_json::Value) {
    if let Err(error) = load_config(root) {
        return crate::config_error_result(&error);
    }
    result_envelope(create_test(
        root,
        form,
        answers,
        explicit_id,
        dry_run,
        registry,
    ))
}

/// `vtest test edit` / `test_edit`.
pub fn edit(
    root: &Path,
    registry: &AdapterRegistry,
    test_id: &str,
    answers: Option<&FormAnswers>,
    set: &BTreeMap<String, FormValue>,
    body: Option<&str>,
    dry_run: bool,
) -> (ExitCode, serde_json::Value) {
    if let Err(error) = load_config(root) {
        return crate::config_error_result(&error);
    }
    result_envelope(edit_test(
        root, test_id, answers, set, body, dry_run, registry,
    ))
}

/// An operation-input failure (E-OP-001 and friends) in the same envelope
/// shape the operations themselves return, for callers that reject input
/// before reaching [`create`] / [`edit`] (an unreadable answers file, a
/// malformed `--set`).
pub fn input_failure(diagnostic: Diagnostic) -> (ExitCode, serde_json::Value) {
    failure(diagnostic)
}

fn result_envelope(
    result: Result<TestMutationResult, Diagnostic>,
) -> (ExitCode, serde_json::Value) {
    match result {
        Ok(result) => {
            let data = serde_json::json!({
                "test_id": result.test_id,
                "file": result.file,
                "position": {
                    "start_byte": result.start_byte,
                    "end_byte": result.end_byte,
                },
                "dry_run": result.dry_run,
                "changed": result.changed,
                "diff": unified_diff(&result.file, &result.replaced, &result.rendered),
            });
            (ExitCode::Ok, crate::envelope_json(true, data, &[]))
        }
        Err(diagnostic) => failure(diagnostic),
    }
}

fn failure(diagnostic: Diagnostic) -> (ExitCode, serde_json::Value) {
    // DS-929: operation rejections exit 2; only an internal failure exits 3.
    let exit = if diagnostic.code == vtest_model::INTERNAL_ERROR_CODE {
        ExitCode::Internal
    } else {
        ExitCode::Usage
    };
    (
        exit,
        crate::envelope_json(false, serde_json::Value::Null, &[diagnostic]),
    )
}

/// A whole-hunk diff of the one changed range: every replaced line as `-`,
/// every new line as `+`. The change is always a single Test's range, so one
/// hunk describes it exactly. An unchanged range (`changed: false`, a
/// re-applied desired state) has no diff at all (DS-1554).
fn unified_diff(file: &str, before: &str, after: &str) -> String {
    if before == after {
        return String::new();
    }
    let mut diff = format!("--- a/{file}\n+++ b/{file}\n");
    for line in before.lines() {
        diff.push('-');
        diff.push_str(line);
        diff.push('\n');
    }
    for line in after.lines() {
        diff.push('+');
        diff.push_str(line);
        diff.push('\n');
    }
    diff
}

/// Converts a JSON `answers` object (MCP `test_create` / `test_edit`) into
/// Form values: a string is a scalar, an array of strings a list.
pub fn form_values_from_json(
    value: &serde_json::Value,
    argument: &str,
) -> Result<BTreeMap<String, FormValue>, Diagnostic> {
    let Some(object) = value.as_object() else {
        return Err(Diagnostic::error(
            "E-OP-001",
            format!("argument `{argument}` must be an object"),
        ));
    };
    let mut values = BTreeMap::new();
    for (key, value) in object {
        let value = match value {
            serde_json::Value::String(text) => FormValue::Scalar(text.clone()),
            serde_json::Value::Array(items) => FormValue::List(
                items
                    .iter()
                    .map(|item| item.as_str().map(str::to_owned))
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| {
                        Diagnostic::error(
                            "E-OP-001",
                            format!("`{argument}.{key}` must contain only strings"),
                        )
                    })?,
            ),
            _ => {
                return Err(Diagnostic::error(
                    "E-OP-001",
                    format!("`{argument}.{key}` must be a string or an array of strings"),
                ))
            }
        };
        values.insert(key.clone(), value);
    }
    Ok(values)
}
