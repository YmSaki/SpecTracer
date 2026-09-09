use serde::{Deserialize, Serialize};

/// Canonical result state of a verification check.
///
/// Diagnostic labels are tracked separately and are not verification states.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationState {
    Pass,
    Fail,
    Mismatch,
    NoEvidence,
    Unknown,
}

/// Canonical diagnostic label for a verification check.
///
/// These labels provide additional context for verification results.
///
/// `Ord` here is declaration order, used only so a label set can be collected
/// deterministically (`BTreeSet`). It is NOT a severity or precedence order:
/// DS-847「診断ラベル（MISSING / NOT_EXECUTED / NOT_CHECKED / STALE）は順位に
/// 用いず併記する」、DS-872「診断ラベルは充足判定に用いない」.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiagnosticLabel {
    Missing,
    NotChecked,
    NotExecuted,
    Stale,
}

/// Canonical verification check defined by the v0.1 model.
///
/// Each variant identifies a distinct verification concern.
///
/// `Ord` here is declaration order, used only to hold a check set
/// deterministically (`BTreeSet`) and to emit checks in a stable order. It
/// carries no precedence: SPEC-054「各検査は一つの問いを持つ」— the four
/// checks are peers. (DS-873's "5状態に順序・優劣・包含関係を設けない" is
/// about the five *states*, which deliberately have no `Ord` at all.)
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationCheck {
    ChainIntegrity,
    OrphanDetection,
    TargetBinding,
    OraclePresence,
}

/// Result of a canonical verification check.
///
/// Combines the check performed, its verification state, and any diagnostic
/// labels that provide additional context.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VerificationResult {
    pub check: VerificationCheck,
    pub state: VerificationState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostic_labels: Vec<DiagnosticLabel>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @vtest.id TEST-MODEL-VERIFICATION-STATE-VALUES
    /// @vtest.covers VO-MODEL-VERIFICATION-STATE-VALUES
    /// @vtest.target crates/vtest-model/src/verification.rs::VerificationState
    /// @vtest.intent verifies the five verification states serialize to PASS/FAIL/MISMATCH/NO_EVIDENCE/UNKNOWN (REQ-085)
    #[test]
    fn verification_state_serializes_correctly() {
        assert_eq!(
            serde_json::to_string(&VerificationState::Pass).unwrap(),
            "\"PASS\""
        );
        assert_eq!(
            serde_json::to_string(&VerificationState::Fail).unwrap(),
            "\"FAIL\""
        );

        assert_eq!(
            serde_json::to_string(&VerificationState::Mismatch).unwrap(),
            "\"MISMATCH\""
        );
        assert_eq!(
            serde_json::to_string(&VerificationState::NoEvidence).unwrap(),
            "\"NO_EVIDENCE\""
        );

        assert_eq!(
            serde_json::to_string(&VerificationState::Unknown).unwrap(),
            "\"UNKNOWN\""
        );
    }

    /// @vtest.id TEST-MODEL-DIAGNOSTIC-LABEL-VALUES
    /// @vtest.covers VO-MODEL-DIAGNOSTIC-LABEL-VALUES
    /// @vtest.target crates/vtest-model/src/verification.rs::DiagnosticLabel
    /// @vtest.intent verifies the four diagnostic labels serialize to MISSING/NOT_CHECKED/NOT_EXECUTED/STALE (DS-847)
    #[test]
    fn diagnostic_label_serializes_correctly() {
        assert_eq!(
            serde_json::to_string(&DiagnosticLabel::Missing).unwrap(),
            "\"MISSING\""
        );

        assert_eq!(
            serde_json::to_string(&DiagnosticLabel::NotChecked).unwrap(),
            "\"NOT_CHECKED\""
        );
        assert_eq!(
            serde_json::to_string(&DiagnosticLabel::NotExecuted).unwrap(),
            "\"NOT_EXECUTED\""
        );
        assert_eq!(
            serde_json::to_string(&DiagnosticLabel::Stale).unwrap(),
            "\"STALE\""
        );
    }

    /// @vtest.id TEST-MODEL-VERIFICATION-CHECK-VALUES
    /// @vtest.covers VO-MODEL-VERIFICATION-CHECK-VALUES
    /// @vtest.target crates/vtest-model/src/verification.rs::VerificationCheck
    /// @vtest.intent verifies the four verification checks serialize to chain_integrity/orphan_detection/target_binding/oracle_presence (REQ-034)
    #[test]
    fn verification_check_serializes_correctly() {
        assert_eq!(
            serde_json::to_string(&VerificationCheck::ChainIntegrity).unwrap(),
            "\"chain_integrity\""
        );
        assert_eq!(
            serde_json::to_string(&VerificationCheck::OrphanDetection).unwrap(),
            "\"orphan_detection\""
        );
        assert_eq!(
            serde_json::to_string(&VerificationCheck::TargetBinding).unwrap(),
            "\"target_binding\""
        );
        assert_eq!(
            serde_json::to_string(&VerificationCheck::OraclePresence).unwrap(),
            "\"oracle_presence\""
        );
    }

    /// @vtest.id TEST-MODEL-VERIFICATION-RESULT-SHAPE
    /// @vtest.covers VO-MODEL-VERIFICATION-RESULT-SHAPE
    /// @vtest.target crates/vtest-model/src/verification.rs::VerificationResult
    /// @vtest.intent verifies VerificationResult carries check/state/diagnostic_labels as distinct fields, with diagnostic_labels omitted when empty
    #[test]
    fn verification_result_serializes_correctly() {
        let result = VerificationResult {
            check: VerificationCheck::TargetBinding,
            state: VerificationState::Fail,
            diagnostic_labels: vec![DiagnosticLabel::NotExecuted],
        };

        let serialized = serde_json::to_string(&result).unwrap();
        let expected =
            r#"{"check":"target_binding","state":"FAIL","diagnostic_labels":["NOT_EXECUTED"]}"#;
        assert_eq!(serialized, expected);

        let success_result = VerificationResult {
            check: VerificationCheck::ChainIntegrity,
            state: VerificationState::Pass,
            diagnostic_labels: vec![],
        };
        let serialized_success = serde_json::to_string(&success_result).unwrap();
        let expected_success = r#"{"check":"chain_integrity","state":"PASS"}"#;
        assert_eq!(serialized_success, expected_success);
    }
}
