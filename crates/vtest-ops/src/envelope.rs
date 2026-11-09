//! Shared envelope construction — the operation modules build the same
//! `JsonEnvelope` shape the CLI's own `emit`/`emit_failure` produce, so a
//! caller (CLI text renderer or `vtest-mcp`'s tool handler) sees identical
//! `ok` / `data` / `diagnostics` for identical input (DS-1563).

use vtest_model::{Diagnostic, ExitCode, JsonEnvelope};

/// Builds the same JSON shape `JsonEnvelope::new` serializes to, without
/// requiring the caller to hold a typed `Diagnostic` slice reference.
pub(crate) fn envelope_json<T: serde::Serialize>(
    ok: bool,
    data: T,
    diagnostics: &[Diagnostic],
) -> serde_json::Value {
    serde_json::to_value(JsonEnvelope::new(ok, data, diagnostics.to_vec()))
        .unwrap_or_else(|error| serde_json::json!({"ok": false, "data": null, "diagnostics": [{"code": vtest_model::INTERNAL_ERROR_CODE, "severity": "error", "message": error.to_string()}]}))
}

/// The config-load failure result shared by `scan` / `doctor` / `run`
/// (CLI and MCP): a rejected config is E-CONFIG-001 with exit 2 (DS-929,
/// DS-935); a config file that cannot be read at all is E-INTERNAL-001 with
/// exit 3 (DS-930, DS-1689/DS-1690).
pub fn config_error_result(error: &vtest_store::StoreError) -> (ExitCode, serde_json::Value) {
    let (exit, code) = match error.config_load_code() {
        Some(code) => (ExitCode::Usage, code),
        None => (ExitCode::Internal, vtest_model::INTERNAL_ERROR_CODE),
    };
    let envelope = envelope_json(
        false,
        serde_json::Value::Null,
        &[Diagnostic::error(code, error.to_string())],
    );
    (exit, envelope)
}

/// DS-935「`vtest scan` / `vtest doctor`では、registry・config・adapter契約の
/// 検証またはadapter呼出しがE-ADAPTER-* / E-CONFIG-*で拒否された場合は2とする」。
pub(crate) fn scan_error_result(error: &vtest_scan::ScanError) -> (ExitCode, serde_json::Value) {
    let code = error.code().unwrap_or(vtest_model::INTERNAL_ERROR_CODE);
    let exit = if error.code().is_some() {
        ExitCode::Usage
    } else {
        ExitCode::Internal
    };
    let envelope = envelope_json(
        false,
        serde_json::Value::Null,
        &[Diagnostic::error(code, error.to_string())],
    );
    (exit, envelope)
}
