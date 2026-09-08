use crate::Diagnostic;
use serde::{Deserialize, Serialize};

/// JSON response envelope containing result data and diagnostics.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct JsonEnvelope<T> {
    // CHECKME: Is `ok` independently meaningful, or should it be derived from diagnostics?
    pub ok: bool,
    pub data: T,
    pub diagnostics: Vec<Diagnostic>,
}

impl<T> JsonEnvelope<T> {
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
    Ok = 0,
    VerificationFailed = 1,
    Usage = 2,
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

    // CHECKME: The test verifies the serialized type shape, not JsonEnvelope::new.
    // The current source-target model may not express this verification target precisely.

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
