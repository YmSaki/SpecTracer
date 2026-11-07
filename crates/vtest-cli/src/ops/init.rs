//! Canonical `init` operation, shared between the CLI's `run_init` wrapper
//! and `vtest-mcp`'s `init` tool handler (DS-1563).

use std::path::Path;

use vtest_model::ExitCode;
use vtest_store::init_project_with_forms;

/// DS-985 / BD-153: `vtest init` writes the built-in Form Schemas of the
/// registered Structured Test adapters (DES-373); core names none itself.
pub fn execute(root: &Path, project_name: &str) -> (ExitCode, serde_json::Value) {
    let registry = match crate::adapters::builtin_registry() {
        Ok(registry) => registry,
        Err(error) => {
            return (
                ExitCode::Usage,
                crate::envelope_json(
                    false,
                    serde_json::Value::Null,
                    &[vtest_model::Diagnostic::error(
                        "E-ADAPTER-001",
                        error.to_string(),
                    )],
                ),
            )
        }
    };
    let mut ids = registry.structured_test_ids();
    ids.sort_unstable();
    let forms: Vec<(&str, &str)> = ids
        .into_iter()
        .filter_map(|id| registry.structured_test(id))
        .flat_map(|adapter| adapter.built_in_forms().iter())
        .map(|form| (form.kind, form.schema))
        .collect();
    match init_project_with_forms(root, project_name, &forms) {
        Ok(_) => {
            let data = serde_json::json!({ "project": root, "initialized": true });
            (ExitCode::Ok, crate::envelope_json(true, data, &[]))
        }
        // A rejected init (e.g. `.verify/` already exists) is E-CONFIG-001
        // / exit 2; a filesystem failure writing the layout is internal.
        Err(error) => crate::config_error_result(&error),
    }
}
