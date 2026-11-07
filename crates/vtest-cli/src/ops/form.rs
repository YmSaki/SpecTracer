//! Canonical `form_get` operation (DS-1205): one Form `kind` resolved
//! through the Form registry (`vtest_scan::forms::resolve_form`) to its
//! schema and owner adapter. Shared so any CLI surface added later returns
//! the same envelope as the MCP tool (DS-1563).

use std::path::Path;

use vtest_adapter_api::AdapterRegistry;
use vtest_model::ExitCode;
use vtest_scan::forms::resolve_form;

pub fn get(root: &Path, registry: &AdapterRegistry, kind: &str) -> (ExitCode, serde_json::Value) {
    match resolve_form(root, registry, kind) {
        Ok(form) => {
            let data = serde_json::json!({
                "kind": form.schema.kind,
                "adapter": form.adapter,
                "schema": form.schema,
            });
            (ExitCode::Ok, crate::envelope_json(true, data, &[]))
        }
        Err(diagnostic) => (
            ExitCode::Usage,
            crate::envelope_json(false, serde_json::Value::Null, &[diagnostic]),
        ),
    }
}
