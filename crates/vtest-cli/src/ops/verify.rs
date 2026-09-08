//! Canonical `verify` operation, shared between the CLI's `run_verify`
//! wrapper (which additionally renders human-readable text) and
//! `vtest-mcp`'s `verify` tool handler.
//!
//! DS-1563「別紙A（§12〜§15）が定める全 MCP tool が同じ入力に対する CLI
//! JSON と同じ data / diagnostics を返す」— this module is the single
//! place that produces that `data`, so both callers stay identical by
//! construction rather than by convention.

use std::path::Path;

use vtest_model::{Diagnostic, ExitCode, VerificationCheck, VerificationState};
use vtest_scan::{scan_project, ScanResult};
use vtest_store::{load_config, GateConfig, ProjectConfig};
use vtest_verify::{
    parse_check, verify_project, CheckOutcome, EntityScope, ScopeReport, TreeNode, VerifyOutcome,
};

/// Owned counterpart of the CLI's `VerifyData` — owned so it can be handed
/// to a caller (CLI or MCP) instead of borrowing from a stack-local
/// `VerifyOutcome`.
#[derive(Clone, serde::Serialize)]
pub struct VerifyData {
    pub scope: ScopeReport,
    pub state: &'static str,
    pub result: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate: Option<GateEvaluation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub structural: Vec<CheckOutcome>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unevaluated: Vec<CheckOutcome>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tree: Vec<TreeNode>,
    pub non_pass: usize,
}

/// DES-554「`--gate` を指定した `verify` / `report` の JSON は `data.gate`
/// に `name`・`verification.{required, actual, satisfied}`・
/// `approvals[].{role, satisfied, missing_subjects}`・`satisfied` を返す」。
#[derive(Clone, serde::Serialize)]
pub struct GateEvaluation {
    pub name: String,
    pub verification: GateVerification,
    pub approvals: Vec<GateApproval>,
    pub satisfied: bool,
}

#[derive(Clone, serde::Serialize)]
pub struct GateVerification {
    pub required: String,
    pub actual: String,
    pub satisfied: bool,
}

/// One required approval role's evaluation.
///
/// `missing_subjects` is always empty in this slice: DES-554 requires this
/// field, but nothing in `config.yaml`'s `gates[].require.approvals` (a
/// bare role-name list, DS-372) or `approval_roles` (role name -> approver
/// identity list, DS-373) names *which entity/entities* the gate's target
/// scope requires that role to approve — resolving that needs an entity
/// scope -> required-approval-subject mapping this slice's config schema
/// does not carry. `satisfied` stays fail-closed (`false`) whenever a role
/// is required, matching the existing rule that an unevaluable role must
/// never read as satisfied. **Consequence**: with this slice's config
/// schema, a gate whose `require.approvals` is non-empty can therefore
/// never reach `GateEvaluation.satisfied == true`, regardless of how many
/// approval records exist — the role is permanently unevaluable, not
/// merely currently unsatisfied. See `reports/closure-trace.md`'s
/// stopped_on list.
#[derive(Clone, serde::Serialize)]
pub struct GateApproval {
    pub role: String,
    pub satisfied: bool,
    pub missing_subjects: Vec<String>,
}

#[derive(Debug)]
pub enum VerifyOpError {
    /// Usage error (E-OP-001 / E-CONFIG-002): exit code 2, no verification run.
    Usage { code: &'static str, message: String },
    /// scan failed; caller maps to the appropriate exit code / diagnostic code.
    Scan(vtest_scan::ScanError),
}

#[allow(clippy::too_many_arguments)]
pub fn execute(
    root: &Path,
    items: &[String],
    doc: Option<String>,
    vo: Option<String>,
    test: Option<String>,
    gate: Option<&str>,
    summary: bool,
) -> Result<(ExitCode, VerifyData, Vec<Diagnostic>), VerifyOpError> {
    let entity = entity_scope(doc, vo, test).map_err(|message| VerifyOpError::Usage {
        code: "E-OP-001",
        message,
    })?;
    let requested = parse_items(items).map_err(|message| VerifyOpError::Usage {
        code: "E-OP-001",
        message,
    })?;

    let config = load_config(root).map_err(|error| VerifyOpError::Usage {
        code: "E-CONFIG-001",
        message: error.to_string(),
    })?;

    let gate_config = resolve_gate(&config, gate).map_err(|message| VerifyOpError::Usage {
        code: "E-CONFIG-002",
        message,
    })?;

    let scan: ScanResult = scan_project(root).map_err(VerifyOpError::Scan)?;

    let outcome = verify_project(root, &scan, requested.as_deref(), entity);
    let gate_evaluation = gate_config.map(|config| evaluate_gate(config, &outcome));
    let non_pass = outcome
        .all_outcomes()
        .iter()
        .filter(|check| check.state != VerificationState::Pass)
        .count();

    let data = VerifyData {
        scope: outcome.scope.clone(),
        state: state_name(outcome.state),
        result: if outcome.ok { "OK" } else { "NG" },
        gate: gate_evaluation,
        structural: if summary {
            Vec::new()
        } else {
            outcome.structural.clone()
        },
        unevaluated: if summary {
            Vec::new()
        } else {
            outcome.unevaluated.clone()
        },
        tree: if summary {
            Vec::new()
        } else {
            outcome.tree.clone()
        },
        non_pass,
    };

    let exit = match &data.gate {
        Some(evaluation) if evaluation.satisfied => ExitCode::Ok,
        Some(_) => ExitCode::VerificationFailed,
        None if outcome.ok => ExitCode::Ok,
        None => ExitCode::VerificationFailed,
    };

    Ok((exit, data, scan.diagnostics.clone()))
}

fn entity_scope(
    doc: Option<String>,
    vo: Option<String>,
    test: Option<String>,
) -> Result<Option<EntityScope>, String> {
    let selected = [
        doc.map(EntityScope::Doc),
        vo.map(EntityScope::Vo),
        test.map(EntityScope::Test),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    match selected.len() {
        0 => Ok(None),
        1 => Ok(selected.into_iter().next()),
        _ => Err("verify accepts at most one of --doc, --vo, or --test".to_owned()),
    }
}

fn parse_items(items: &[String]) -> Result<Option<Vec<VerificationCheck>>, String> {
    if items.is_empty() {
        return Ok(None);
    }
    let mut checks = Vec::new();
    for item in items {
        let name = item.trim();
        if name.is_empty() {
            continue;
        }
        let Some(check) = parse_check(name) else {
            return Err(format!(
                "unknown check '{name}'; the fixed four are chain_integrity, \
                 orphan_detection, target_binding, oracle_presence"
            ));
        };
        if !checks.contains(&check) {
            checks.push(check);
        }
    }
    if checks.is_empty() {
        return Ok(None);
    }
    Ok(Some(checks))
}

fn resolve_gate<'a>(
    config: &'a ProjectConfig,
    gate: Option<&str>,
) -> Result<Option<&'a GateConfig>, String> {
    let Some(name) = gate else {
        return Ok(None);
    };
    config
        .gates
        .iter()
        .find(|candidate| candidate.name == name)
        .map(Some)
        .ok_or_else(|| format!("no gate named '{name}' is defined in config.yaml"))
}

fn evaluate_gate(config: &GateConfig, outcome: &VerifyOutcome) -> GateEvaluation {
    let actual = state_name(outcome.state);
    let verification_satisfied = config.require.verification == actual;

    // DES-554's approvals[] is role-keyed. `missing_subjects` stays empty
    // (see GateApproval's doc comment) and `satisfied` stays fail-closed
    // (false) for every required role -- an unevaluable role must never
    // read as satisfied.
    let approvals: Vec<GateApproval> = config
        .require
        .approvals
        .iter()
        .map(|role| GateApproval {
            role: role.clone(),
            satisfied: false,
            missing_subjects: Vec::new(),
        })
        .collect();
    let approvals_satisfied = approvals.is_empty();

    GateEvaluation {
        name: config.name.clone(),
        verification: GateVerification {
            required: config.require.verification.clone(),
            actual: actual.to_owned(),
            satisfied: verification_satisfied,
        },
        approvals,
        satisfied: verification_satisfied && approvals_satisfied,
    }
}

pub fn state_name(state: VerificationState) -> &'static str {
    match state {
        VerificationState::Pass => "PASS",
        VerificationState::Fail => "FAIL",
        VerificationState::Mismatch => "MISMATCH",
        VerificationState::NoEvidence => "NO_EVIDENCE",
        VerificationState::Unknown => "UNKNOWN",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @vtest.id TEST-VERIFY-PARSE-ITEMS-REJECT-UNKNOWN
    /// @vtest.covers VO-VERIFY-RETIRED-CHECK-NAME-USAGE-ERROR
    /// @vtest.target crates/vtest-cli/src/ops/verify.rs::parse_items
    /// @vtest.intent parse_items rejects a retired check name and accepts a current one
    #[test]
    fn items_reject_an_unknown_check_rather_than_narrowing_the_scope() {
        assert!(parse_items(&["spec_coverage".to_owned()]).is_err());
        assert!(parse_items(&["chain_integrity".to_owned()]).is_ok());
    }

    /// @vtest.id TEST-VERIFY-PARSE-ITEMS-OMITTED-FOUR
    /// @vtest.covers VO-VERIFY-SCOPE-REQUESTED-WIRE-SHAPE
    /// @vtest.target crates/vtest-cli/src/ops/verify.rs::parse_items
    /// @vtest.intent parse_items on an empty --items list returns None, mapping to the fixed four checks
    #[test]
    fn omitted_items_map_to_the_fixed_four() {
        assert!(parse_items(&[]).expect("empty is valid").is_none());
    }

    /// @vtest.id TEST-VERIFY-ENTITY-AXIS-EXCLUSIVE
    /// @vtest.covers VO-VERIFY-ENTITY-AXIS-SINGLE-SELECTOR
    /// @vtest.target crates/vtest-cli/src/ops/verify.rs::entity_scope
    /// @vtest.intent entity_scope rejects two simultaneous selectors and accepts zero or one
    #[test]
    fn entity_axis_is_exclusive() {
        assert!(entity_scope(Some("D".to_owned()), Some("V".to_owned()), None).is_err());
        assert!(entity_scope(None, Some("V".to_owned()), None).is_ok());
        assert!(entity_scope(None, None, None)
            .expect("none is valid")
            .is_none());
    }
}
