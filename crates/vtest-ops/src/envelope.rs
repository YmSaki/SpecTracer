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

/// Runs `operation`, turning a panic that escapes it into the
/// E-INTERNAL-001 failure with exit 3 (DS-1689/DS-1690). Both entrypoints
/// call this around a whole command or tool call, so a panic never leaves
/// the process with the runtime's exit code 101 or kills the MCP server,
/// and is never reported as one of the five verification states.
///
/// The message is fixed: a panic payload can carry absolute paths or
/// environment values, so it is not copied into the user-facing output.
/// The default panic hook has already written the payload and location to
/// stderr by the time this returns, which is where an investigation reads
/// it.
pub fn catch_internal<T>(operation: impl FnOnce() -> T) -> Result<T, Failure> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation)).map_err(|_| {
        Failure::internal("internal error: the operation panicked (details are on stderr)")
    })
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

/// A canonical record that cannot be read or written: an invalid record
/// rejects the requested operation with the code the store assigns it
/// (exit 2); a filesystem failure is E-INTERNAL-001 with exit 3 (DS-1676,
/// DS-929, DS-930).
pub fn record_failure(error: &vtest_store::StoreError) -> Failure {
    match error.record_read_code() {
        Some(code) => Failure::usage(code, error.to_string()),
        None => Failure::internal(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @vtest.id TEST-OPS-CATCH-INTERNAL-PANIC
    /// @vtest.covers VO-INTERNAL-ERROR-CODE-CLI-MCP
    /// @vtest.target crates/vtest-ops/src/envelope.rs::catch_internal
    /// @vtest.intent a panic inside an operation becomes the E-INTERNAL-001 failure with exit 3 and a fixed message that does not carry the panic payload, while a normal return passes through
    #[test]
    fn a_panic_is_an_internal_failure_without_its_payload() {
        let failure = catch_internal(|| -> u8 { panic!("secret-payload") })
            .expect_err("a panic is a failure");
        assert_eq!(failure.exit, ExitCode::Internal);
        assert_eq!(failure.diagnostic.code, vtest_model::INTERNAL_ERROR_CODE);
        assert!(!failure.diagnostic.message.contains("secret"));
        assert_eq!(catch_internal(|| 7).expect("no panic"), 7);
    }
}
