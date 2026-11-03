//! Canonical `doctor` operation, shared between the CLI's `run_doctor`
//! wrapper and `vtest-mcp`'s `doctor` tool handler (DS-1563).

use std::path::Path;

use vtest_adapter_api::AdapterRegistry;
use vtest_model::ExitCode;
use vtest_scan::scan_project;
use vtest_store::load_config;

pub fn execute(root: &Path, registry: &AdapterRegistry) -> (ExitCode, serde_json::Value) {
    let config = match load_config(root) {
        Ok(config) => config,
        Err(error) => {
            return (
                ExitCode::Usage,
                crate::config_failure_envelope(&error.to_string()),
            )
        }
    };
    match scan_project(root, registry) {
        Ok(result) => {
            let has_errors = result.has_errors();
            let data = serde_json::json!({
                "project": config.project.name,
                "adapters": config.adapters.iter().map(|a| a.id.clone()).collect::<Vec<_>>(),
                "gates": config.gates.iter().map(|g| g.name.clone()).collect::<Vec<_>>(),
            });
            let envelope = crate::envelope_json(!has_errors, data, &result.diagnostics);
            let exit = if has_errors {
                ExitCode::VerificationFailed
            } else {
                ExitCode::Ok
            };
            (exit, envelope)
        }
        Err(error) => crate::scan_error_result(&error),
    }
}
