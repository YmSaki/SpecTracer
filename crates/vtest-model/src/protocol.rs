use crate::Diagnostic;
use serde::{Deserialize, Serialize};

/// Identifies the kind of verification check being performed.
///
/// Each variant represents a distinct verification concern whose result is
/// recorded separately.
#[deprecated(note = "Predecessor model: replace with the canonical v0.1 verification checks")]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum CheckItem {
    #[serde(rename = "spec_coverage")]
    SpecCoverage,
    #[serde(rename = "vo_decomposition")]
    VoDecomposition,
    #[serde(rename = "vo_coverage")]
    VoCoverage,
    #[serde(rename = "test_existence")]
    TestExistence,
    #[serde(rename = "static_audit")]
    StaticAudit,
    #[serde(rename = "semantic_audit")]
    SemanticAudit,
    #[serde(rename = "impl_consistency")]
    ImplConsistency,
    #[serde(rename = "test_execution")]
    TestExecution,
    #[serde(rename = "runtime_result")]
    RuntimeResult,
    #[serde(rename = "target_execution")]
    TargetExecution,
    #[serde(rename = "evidence_validity")]
    EvidenceValidity,
}

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
