//! Acceptance coverage for 別紙C §18.3.9 フェーズゲート評価
//! (DES-551/552/553/554).
//!
//! DES-554's wire shape (`data.gate.verification.{required, actual,
//! satisfied}`, `data.gate.approvals[].{role, satisfied, missing_subjects}`)
//! is asserted here literally — `ops::verify::GateEvaluation` was renamed
//! to this exact shape (previously a flatter `required_verification`/
//! `verification_satisfied`/`approvals_satisfied`/`reasons` form with no
//! per-role breakdown at all).
//!
//! `missing_subjects` is always an empty list in this slice's output
//! (disclosed in `GateApproval`'s own doc comment and
//! `reports/closure-trace.md`'s stopped_on list): nothing in `config.yaml`
//! names which entity/entities a required role must approve, so there is no
//! subject set to compute a "missing" list from. `satisfied` for a required
//! role stays fail-closed (`false`).

use std::path::PathBuf;

use vtest_cli::ops;
use vtest_store::init_project;

fn temp_root(name: &str) -> PathBuf {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vtest-cli-acceptance-18-3-9-{name}-{suffix}"));
    std::fs::create_dir_all(&root).expect("create fixture root");
    root
}

fn write_gate_config(root: &std::path::Path, approvals: Vec<String>) -> vtest_store::VerifyLayout {
    let layout = init_project(root, "acceptance-18-3-9-fixture").expect("init .verify/ layout");
    let mut config = vtest_store::load_config(root).expect("load default config");
    if !approvals.is_empty() {
        config
            .approval_roles
            .insert("reviewer".to_owned(), vec!["reviewer-agent-01".to_owned()]);
    }
    config.gates.push(vtest_store::GateConfig {
        name: "release".to_owned(),
        require: vtest_store::GateRequirement {
            verification: "PASS".to_owned(),
            approvals,
        },
    });
    std::fs::write(layout.config(), config.to_yaml()).expect("write gate config");
    layout
}

/// DES-552's "条件不足" half, DES-553's "順序・包含解釈による充足を認めない"
/// (an undefined gate name is rejected outright, not fuzzily matched):
/// `verify_acceptance.rs`'s existing
/// `an_undefined_gate_name_is_rejected_before_verification_runs` already
/// covers this exact DS-1116 rule; this test adds the `--gate` exit-code
/// axis DES-551/554 name specifically (0/1 keyed to gate satisfaction, not
/// to the aggregate verification state — DS-931/932/933), and asserts the
/// literal `data.gate.verification.{required,actual,satisfied}` shape.
#[test]
fn an_unsatisfied_gate_reports_verification_required_actual_satisfied() {
    let root = temp_root("gate-unsatisfied");
    write_gate_config(&root, Vec::new());

    let (exit, data, _diagnostics) =
        ops::verify::execute(&root, &[], None, None, None, Some("release"), false)
            .expect("verify must run to completion on an empty (non-scan-error) project");
    assert_eq!(
        exit,
        vtest_model::ExitCode::VerificationFailed,
        "DES-551/554: an empty project can never satisfy a gate requiring PASS"
    );
    let gate = data.gate.expect("--gate must populate data.gate");
    assert_eq!(gate.name, "release");
    assert_eq!(gate.verification.required, "PASS");
    assert!(
        !gate.verification.satisfied,
        "an empty project's aggregate state cannot be PASS"
    );
    assert!(!gate.satisfied);
}

/// DES-552's satisfied side: a gate whose `require.verification` names the
/// aggregate state the fixture project actually reaches (`NO_EVIDENCE`, an
/// empty project's real representative-selected state -- see
/// `verify_acceptance.rs`'s `an_empty_project_is_never_a_complete_verification_ok`
/// for the same fixture reaching this exact state) and whose
/// `require.approvals` is empty (vacuously satisfied, no role to evaluate)
/// must actually report `satisfied: true` end to end, not merely fail to
/// crash. Every other test in this file only exercises the unsatisfied
/// side, which cannot by itself distinguish "correctly computes
/// satisfaction" from "always reports false".
#[test]
fn a_gate_whose_conditions_are_actually_met_reports_satisfied() {
    let root = temp_root("gate-satisfied");
    let layout =
        init_project(&root, "acceptance-18-3-9-satisfied-fixture").expect("init .verify/ layout");
    let mut config = vtest_store::load_config(&root).expect("load default config");
    config.gates.push(vtest_store::GateConfig {
        name: "release".to_owned(),
        require: vtest_store::GateRequirement {
            verification: "NO_EVIDENCE".to_owned(),
            approvals: Vec::new(),
        },
    });
    std::fs::write(layout.config(), config.to_yaml()).expect("write gate config");

    let (exit, data, _diagnostics) =
        ops::verify::execute(&root, &[], None, None, None, Some("release"), true)
            .expect("verify must run to completion on an empty (non-scan-error) project");
    let gate = data.gate.expect("--gate must populate data.gate");
    assert_eq!(
        data.state, "NO_EVIDENCE",
        "this test's own premise: the fixture must actually reach the state the gate requires"
    );
    assert!(
        gate.verification.satisfied,
        "the aggregate state matches require.verification, so this half must be satisfied"
    );
    assert!(
        gate.approvals.is_empty(),
        "no approval role is required, so there is nothing to evaluate"
    );
    assert!(
        gate.satisfied,
        "both halves are met (verification matches, no required approvals), so the gate as a \
         whole must report satisfied"
    );
    assert_eq!(
        exit,
        vtest_model::ExitCode::Ok,
        "DES-551: exit code is keyed to gate satisfaction, and this gate is satisfied"
    );
}

/// DES-554's `approvals[].{role, satisfied, missing_subjects}` half: a
/// required role that this slice cannot evaluate is reported by name, not
/// merely folded into an aggregate "approvals unsatisfied" boolean.
#[test]
fn a_required_approval_role_appears_in_approvals_by_name() {
    let root = temp_root("gate-role");
    write_gate_config(&root, vec!["reviewer".to_owned()]);

    let (_exit, data, _diagnostics) =
        ops::verify::execute(&root, &[], None, None, None, Some("release"), false)
            .expect("verify must run to completion on an empty (non-scan-error) project");
    let gate = data.gate.expect("--gate must populate data.gate");
    assert_eq!(gate.approvals.len(), 1);
    assert_eq!(gate.approvals[0].role, "reviewer");
    assert!(
        !gate.approvals[0].satisfied,
        "DES-554: an unevaluated required role must never read as satisfied"
    );
    assert!(!gate.satisfied);
}
