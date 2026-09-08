//! Canonical `scan` operation, shared between the CLI's `run_scan` wrapper
//! and `vtest-mcp`'s `scan` tool handler (DS-1563: MCP tools return the same
//! `data` / `diagnostics` as the CLI JSON envelope for the same input).
//!
//! Returns the exit code and the JSON envelope value directly; printing and
//! text rendering stay in the CLI's `lib.rs`.

use std::path::Path;

use vtest_model::ExitCode;
use vtest_scan::scan_project;
use vtest_store::load_config;

pub fn execute(root: &Path) -> (ExitCode, serde_json::Value) {
    // config の拒否は操作拒否であって内部エラーではない。DS-935。
    if let Err(error) = load_config(root) {
        return (
            ExitCode::Usage,
            crate::config_failure_envelope(&error.to_string()),
        );
    }
    match scan_project(root) {
        Ok(result) => {
            let has_errors = result.has_errors();
            let data = serde_json::json!({
                "files": result.summary.files,
                "tests": result.summary.tests,
                "sources": result.summary.sources,
                "discovered": result.discovered.len(),
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
