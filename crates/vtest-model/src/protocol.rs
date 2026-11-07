//! Process-level protocol values shared by the CLI and MCP: the JSON
//! envelope, the exit codes, and the scan summary counts.
//!
//! Boundary: no mapping logic lives here. Each operation decides its own
//! `ok`, exit code and diagnostics; the CLI and MCP only serialize the same
//! envelope (BD-100, BD-260, DS-1563).
//!
//! Clauses: DS-941 (`{ ok, data, diagnostics }`), DS-322 (exit codes
//! 0 / 1 / 2 / 3), DS-1174 / DS-1217 / DS-1539 (`ok` under `--gate`).

use crate::Diagnostic;
use serde::{Deserialize, Serialize};

/// JSON response envelope containing result data and diagnostics.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct JsonEnvelope<T> {
    /// Set by the operation, not derived from `diagnostics`: `true` when the
    /// operation completed and reported no error-severity diagnostic;
    /// warnings leave it `true`. `verify` and `report` tie it to their exit
    /// code (`true` ↔ 0), so with `--gate` it means gate satisfaction
    /// (DS-1174, DS-1217, DS-1539). A rejected operation emits `ok: false`
    /// with `data: null` and the error diagnostic (DS-941).
    pub ok: bool,
    /// The operation's result; `null` when the operation was rejected.
    pub data: T,
    /// Diagnostics in emission order. May be non-empty while `ok` is `true`
    /// (warnings).
    pub diagnostics: Vec<Diagnostic>,
}

impl<T> JsonEnvelope<T> {
    /// Builds an envelope. `ok` is the caller's decision (see the field doc).
    pub fn new(ok: bool, data: T, diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            ok,
            data,
            diagnostics,
        }
    }
}

/// Process exit codes returned by the vtest CLI.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ExitCode {
    /// The requested scope is OK (or the gate is satisfied).
    Ok = 0,
    /// Verification produced a result and it is NG.
    VerificationFailed = 1,
    /// Rejected before producing a result: invalid input, config, adapter
    /// precondition or capability (E-OP-* / E-CONFIG-* / E-APPROVAL-* /
    /// E-ADAPTER-*).
    Usage = 2,
    /// A failure of the tool itself (`E-INTERNAL-001`).
    Internal = 3,
}

/// Summary counts produced by a source scan.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScanSummary {
    pub files: u64,
    pub tests: u64,
    pub sources: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    // The target is `JsonEnvelope::new` because a `@vtest.target` locator
    // names a function; the assertion is on the serialized shape (DS-941).

    /// @vtest.id TEST-MODEL-JSON-ENVELOPE-SHAPE
    /// @vtest.covers VO-MODEL-JSON-ENVELOPE-SHAPE
    /// @vtest.target crates/vtest-model/src/protocol.rs::JsonEnvelope::new
    /// @vtest.intent verifies that the serialized JSON envelope contains its required top-level fields
    #[test]
    fn envelope_has_required_top_level_fields() {
        let value = serde_json::to_value(JsonEnvelope::new(
            true,
            ScanSummary {
                files: 1,
                tests: 2,
                sources: 3,
            },
            vec![],
        ))
        .unwrap();
        assert_eq!(value["ok"], true);
        assert!(value.get("data").is_some());
        assert!(value.get("diagnostics").is_some());
    }
}
