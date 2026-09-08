//! Canonical `init` operation, shared between the CLI's `run_init` wrapper
//! and `vtest-mcp`'s `init` tool handler (DS-1563).

use std::path::Path;

use vtest_model::ExitCode;
use vtest_store::init_project;

pub fn execute(root: &Path, project_name: &str) -> (ExitCode, serde_json::Value) {
    match init_project(root, project_name) {
        Ok(_) => {
            let data = serde_json::json!({ "project": root, "initialized": true });
            (ExitCode::Ok, crate::envelope_json(true, data, &[]))
        }
        Err(error) => (
            ExitCode::Usage,
            crate::config_failure_envelope(&error.to_string()),
        ),
    }
}
