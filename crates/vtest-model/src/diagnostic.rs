//! Diagnostics: the `Diagnostic` value every operation reports through the
//! JSON envelope, its two severities, and the internal-error code.
//!
//! Boundary: this module defines the shape only. Which code an operation
//! emits, and how codes map to exit codes, is decided by the operation
//! (`vtest-cli` / `vtest-mcp` ops). Verification states are not
//! diagnostics; DS-322 keeps the two apart.
//!
//! Clauses: DS-941 (envelope `diagnostics`), DS-942 / DS-1191 (diagnostic
//! and MCP tool-error shape, `candidates`), DS-1689 / DS-1690
//! (`E-INTERNAL-001`).

use crate::SourceLocation;
use serde::{Deserialize, Serialize};

/// DS-1689/DS-1690: the single diagnostic code for an internal error — a
/// failure of the tool itself (store I/O, serialization, a broken
/// implementation invariant) that is neither invalid input nor an operation
/// rejection. It always maps to exit code 3 and is never a verification
/// state.
pub const INTERNAL_ERROR_CODE: &str = "E-INTERNAL-001";

/// Severity level of a diagnostic message.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "warning")]
    Warning,
}

/// A structured diagnostic emitted during scanning, validation, or verification.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: DiagnosticSeverity,
    pub message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<String>,
    /// Where the diagnostic points, when it has a location. Boxed so that a
    /// `Diagnostic` without one stays small: diagnostics travel by value in
    /// `Result` errors and inside `vtest-store::StoreError` /
    /// `vtest-scan::ScanError`, where an inline `SourceLocation` would widen
    /// every such value. The serialized form is unaffected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Box<SourceLocation>>,
}

impl Diagnostic {
    /// An error-severity diagnostic with no location and no candidates.
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: DiagnosticSeverity::Error,
            message: message.into(),
            candidates: Vec::new(),
            location: None,
        }
    }

    /// A warning-severity diagnostic with no location and no candidates.
    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: DiagnosticSeverity::Warning,
            message: message.into(),
            candidates: Vec::new(),
            location: None,
        }
    }

    /// Attaches the source location the diagnostic points at.
    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = Some(Box::new(location));
        self
    }

    /// Attaches the values the rejected input could have meant (`candidates`,
    /// DS-942 / DS-1191).
    pub fn with_candidates(mut self, candidates: impl IntoIterator<Item = String>) -> Self {
        self.candidates = candidates.into_iter().collect();
        self
    }

    /// `true` for error severity. Operations use this to decide `ok` and the
    /// exit code; warnings do not fail an operation.
    pub fn is_error(&self) -> bool {
        self.severity == DiagnosticSeverity::Error
    }
}
