//! The envelope and the failure mapping both entrypoints share.
//!
//! An operation that completes reports through [`envelope_json`]; one that
//! is rejected is a [`Failure`]: the exit code the CLI returns and the one
//! error diagnostic the CLI's `--format json` and the MCP tool both emit.
//! Building both from this module is what keeps `ok` / `data` /
//! `diagnostics` identical for identical input (DS-1563).

use vtest_model::{Diagnostic, ExitCode, JsonEnvelope};

/// The JSON `JsonEnvelope::new(ok, data, diagnostics)` serializes to.
pub fn envelope_json<T: serde::Serialize>(
    ok: bool,
    data: T,
    diagnostics: &[Diagnostic],
) -> serde_json::Value {
    serde_json::to_value(JsonEnvelope::new(ok, data, diagnostics.to_vec()))
        .unwrap_or_else(|error| serde_json::json!({"ok": false, "data": null, "diagnostics": [{"code": vtest_model::INTERNAL_ERROR_CODE, "severity": "error", "message": error.to_string()}]}))
}

/// The envelope of a rejected operation: `ok: false`, `data: null` and one
/// error diagnostic (DS-941).
pub fn failure_envelope(code: &str, message: impl Into<String>) -> serde_json::Value {
    failure_envelope_with_candidates(code, message, None)
}

/// DS-1154/DS-1191: a rejected input that names one of a closed set carries
/// `{code, message, candidates}`. `candidates` is emitted whenever there is
/// such a set (the defined gate names, `[]` when no gate is defined) and
/// omitted when there is none, so a missing field never reads as "no valid
/// value exists".
pub fn failure_envelope_with_candidates(
    code: &str,
    message: impl Into<String>,
    candidates: Option<&[String]>,
) -> serde_json::Value {
    let mut diagnostic = serde_json::json!({
        "code": code,
        "severity": "error",
        "message": message.into(),
    });
    if let Some(candidates) = candidates {
        diagnostic["candidates"] = serde_json::json!(candidates);
    }
    serde_json::json!({
        "ok": false,
        "data": null,
        "diagnostics": [diagnostic],
    })
}

/// A rejected operation: the process exit code the CLI returns for it and
/// the error diagnostic both entrypoints report.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Failure {
    pub exit: ExitCode,
    pub diagnostic: Diagnostic,
}

impl Failure {
    /// A rejection the caller can correct: exit 2 with a defined code.
    pub fn usage(code: &str, message: impl Into<String>) -> Self {
        Self {
            exit: ExitCode::Usage,
            diagnostic: Diagnostic::error(code, message),
        }
    }

    /// A failure with no assigned code: E-INTERNAL-001 with exit 3
    /// (DS-1689/DS-1690).
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            exit: ExitCode::Internal,
            diagnostic: Diagnostic::error(vtest_model::INTERNAL_ERROR_CODE, message),
        }
    }

    /// The failure envelope for this rejection.
    pub fn envelope(&self) -> serde_json::Value {
        envelope_json(
            false,
            serde_json::Value::Null,
            std::slice::from_ref(&self.diagnostic),
        )
    }

    /// Exit code and envelope, the pair an `execute` operation returns.
    pub fn result(&self) -> (ExitCode, serde_json::Value) {
        (self.exit, self.envelope())
    }
}

/// A config that cannot be loaded: a rejected config is E-CONFIG-001 with
/// exit 2 (DS-929, DS-935); a config file that cannot be read at all is
/// E-INTERNAL-001 with exit 3 (DS-930, DS-1689/DS-1690).
pub fn config_failure(error: &vtest_store::StoreError) -> Failure {
    match error.config_load_code() {
        Some(code) => Failure::usage(code, error.to_string()),
        None => Failure::internal(error.to_string()),
    }
}

/// A scan that was rejected: an E-ADAPTER-* / E-CONFIG-* rejection of the
/// registry, the config, the adapter contract or an adapter call is exit 2
/// (DS-935); an error with no assigned code is E-INTERNAL-001 with exit 3.
pub fn scan_failure(error: &vtest_scan::ScanError) -> Failure {
    match error.code() {
        Some(code) => Failure::usage(code, error.to_string()),
        None => Failure::internal(error.to_string()),
    }
}
