//! Acceptance coverage for 別紙C §18.3.9 フェーズゲート評価
//! (DES-551/552/553/554).
//!
//! **Known field-name gap, disclosed rather than hidden** (same nature as
//! `acceptance_18_3_8.rs`'s): DES-554 names the JSON shape literally as
//! `data.gate.verification.{required, actual, satisfied}` and
//! `data.gate.approvals[].{role, satisfied, missing_subjects}`. The
//! current implementation (`crates/vtest-cli/src/lib.rs`'s
//! `GateEvaluation`, predating this closure-slice) instead emits a flatter
//! shape: `name` / `required_verification` / `verification_satisfied` /
//! `approvals_satisfied` / `satisfied` / `reasons` — no per-role
//! `approvals[]` array with `missing_subjects` at all (the gate config's
//! own doc comment in `crates/vtest-store/src/lib.rs` already discloses
//! that this slice has no effective-approval-state reader wired into gate
//! evaluation, so a per-role breakdown isn't available yet). This test
//! asserts the behavior DES-551/552/553 require (satisfied/unsatisfied
//! fixtures, no order/subset-interpretation leniency) against the fields
//! that actually exist, not DES-554's literal names.

use std::path::{Path, PathBuf};

use vtest_cli::{run, Cli, Command, OutputFormat};
use vtest_model::ExitCode;
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

fn cli(root: &Path, command: Command) -> Cli {
    Cli {
        project: root.to_path_buf(),
        format: OutputFormat::Json,
        quiet: true,
        command,
    }
}

/// DES-552's "条件不足" half, DES-553's "順序・包含解釈による充足を認めない"
/// (an undefined gate name is rejected outright, not fuzzily matched):
/// `verify_acceptance.rs`'s existing
/// `an_undefined_gate_name_is_rejected_before_verification_runs` already
/// covers this exact DS-1116 rule; this file adds the `--gate` exit-code
/// axis DES-551/554 name specifically (0/1 keyed to gate satisfaction, not
/// to the aggregate verification state — DS-931/932/933).
#[test]
fn an_unsatisfied_gate_is_exit_one_even_though_verification_itself_also_failed() {
    let root = temp_root("gate-unsatisfied");
    let layout = init_project(&root, "acceptance-18-3-9-fixture").expect("init .verify/ layout");
    let mut config = vtest_store::load_config(&root).expect("load default config");
    config.gates.push(vtest_store::GateConfig {
        name: "release".to_owned(),
        require: vtest_store::GateRequirement {
            verification: "PASS".to_owned(),
            approvals: Vec::new(),
        },
    });
    std::fs::write(layout.config(), config.to_yaml()).expect("write gate config");

    let exit = run(cli(
        &root,
        Command::Verify {
            items: Vec::new(),
            doc: None,
            vo: None,
            test: None,
            gate: Some("release".to_owned()),
            summary: false,
        },
    ));
    assert_eq!(
        exit,
        ExitCode::VerificationFailed,
        "DES-551/554: an empty project can never satisfy a gate requiring PASS"
    );
}
