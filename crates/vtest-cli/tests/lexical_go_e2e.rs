//! End-to-end check that the `go` adapter (Go) is registered in the
//! CLI composition root and that its Tests scan and verify through the
//! unmodified core with the discovery-only verdict.

mod support;

use vtest_model::{DiagnosticLabel, ManagedTestLink, VerificationCheck, VerificationState};

const SOURCE: &str = r##"package calc

func Add(a, b int) int {
	return a + b
}
"##;

const TEST_SOURCE: &str = r##"package calc

import "testing"

// @vtest.id TEST-FIXTURE-GO-ADD
// @vtest.covers VO-FIXTURE-ONE
// @vtest.target calc/calc.go::Add
// @vtest.intent adding two numbers returns their sum
func TestAdd(t *testing.T) {
	if Add(1, 2) != 3 {
		t.Fatal("wrong sum")
	}
}
"##;

/// @vtest.id TEST-CLI-GO-ADAPTER-SCAN-AND-VERIFY
/// @vtest.covers VO-ADAPTER-GO-REGISTERS-WITHOUT-CORE-CHANGES
/// @vtest.target crates/vtest-ops/src/registry.rs::builtin_registry
/// @vtest.intent verifies the go adapter is in the built-in registry and a declared Go Test is scanned, linked to its VO and target, and verified with chain_integrity and orphan_detection PASS and the runner-dependent checks NO_EVIDENCE
#[test]
fn go_adapter_scans_and_verifies_through_the_cli_registry() {
    let fixture = support::fixture(
        "go",
        &[("calc/calc.go", SOURCE), ("calc/calc_test.go", TEST_SOURCE)],
        &["VO-FIXTURE-ONE"],
    );
    let (scan, outcome) = fixture.scan_and_verify();
    assert!(!scan.has_errors(), "{:?}", scan.diagnostics);
    assert_eq!(scan.tests.len(), 1);
    assert_eq!(scan.tests[0].id.as_str(), "TEST-FIXTURE-GO-ADD");
    assert_eq!(scan.tests[0].execution.adapter.as_str(), "go");
    support::assert_discovery_only_verdict(&outcome, "TEST-FIXTURE-GO-ADD");
}

/// @vtest.id TEST-CLI-GO-BODY-TAIL-CHANGES-SUBJECT-HASH
/// @vtest.covers VO-ADAPTER-GO-REGISTERS-WITHOUT-CORE-CHANGES
/// @vtest.target crates/vtest-ops/src/registry.rs::builtin_registry
/// @vtest.intent verifies that for a Go Test whose result type is a struct, an edit to only the end of the function body changes the Test's content hash, so the whole body is hashed
#[test]
fn go_body_tail_edit_changes_the_test_hash() {
    let source = |tail: &str| {
        format!(
            "package calc\n\n// @vtest.id TEST-FIXTURE-GO-TAIL\n// @vtest.covers VO-FIXTURE-ONE\n// @vtest.intent the body tail is hashed\nfunc TestTail(t *testing.T) struct{{ X int }} {{\n\tmarker()\n\t{tail}\n\treturn struct{{ X int }}{{}}\n}}\n"
        )
    };
    let hash = |tail: &str| {
        let fixture = support::fixture(
            "go",
            &[("calc/tail_test.go", &source(tail))],
            &["VO-FIXTURE-ONE"],
        );
        let (scan, _) = fixture.scan_and_verify();
        assert!(!scan.has_errors(), "{:?}", scan.diagnostics);
        assert_eq!(scan.tests.len(), 1);
        scan.tests[0].content_hash.clone()
    };
    assert_ne!(hash("one()"), hash("two()"));
}

/// @vtest.id TEST-CLI-GO-UNDECLARED-TEST-IS-UNMANAGED
/// @vtest.covers VO-SCAN-UNREGISTERED-TEST-W-SCAN-101, VO-SCAN-DISCOVERED-SET-RETAINS-UNMANAGED-CONSTRUCTS
/// @vtest.target crates/vtest-ops/src/registry.rs::builtin_registry
/// @vtest.intent verifies that beside a declared Go Test whose chain passes, a Test function without any @vtest. line is reported as W-SCAN-101 without an error, is kept in the discovered set as a construct without a managed Test, and turns chain_integrity to MISMATCH with the MISSING label
#[test]
fn go_undeclared_test_function_is_unmanaged_and_breaks_the_chain() {
    let undeclared = format!("{TEST_SOURCE}\nfunc TestSub(t *testing.T) {{\n\tcheck(t)\n}}\n");
    let fixture = support::fixture(
        "go",
        &[("calc/calc.go", SOURCE), ("calc/calc_test.go", &undeclared)],
        &["VO-FIXTURE-ONE"],
    );
    let (scan, outcome) = fixture.scan_and_verify();
    assert!(!scan.has_errors(), "{:?}", scan.diagnostics);
    let codes = scan
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();
    assert_eq!(codes, vec!["W-SCAN-101"]);
    assert_eq!(scan.tests.len(), 1);
    let unmanaged = scan
        .discovered
        .iter()
        .filter(|test| test.managed == ManagedTestLink::Missing)
        .map(|test| test.location.locator.as_str())
        .collect::<Vec<_>>();
    assert_eq!(unmanaged, vec!["TestSub"]);
    assert_eq!(scan.discovered.len(), 2);

    let chain = outcome
        .all_outcomes()
        .into_iter()
        .filter(|candidate| candidate.check == VerificationCheck::ChainIntegrity)
        .collect::<Vec<_>>();
    assert!(
        chain
            .iter()
            .any(|candidate| candidate.state == VerificationState::Mismatch
                && candidate.labels.contains(&DiagnosticLabel::Missing)),
        "{chain:?}"
    );
    assert!(chain
        .iter()
        .all(|candidate| candidate.state != VerificationState::Pass));
}

/// @vtest.id TEST-CLI-GO-AMBIGUOUS-TARGET-IS-E-SCAN-004
/// @vtest.covers VO-SCAN-TARGET-UNRESOLVABLE-E-SCAN-004
/// @vtest.target crates/vtest-ops/src/registry.rs::builtin_registry
/// @vtest.intent verifies that a Go Test declaring a target that two functions in one file share (init) is reported by the core as the unresolved target E-SCAN-004 with the declared value in the message, while a target that matches one function resolves
#[test]
fn go_ambiguous_target_is_e_scan_004() {
    let calc = "package calc\n\nfunc init() {}\n\nfunc init() {}\n\nfunc Add(a, b int) int { return a + b }\n";
    let tests = "package calc\n\n// @vtest.id TEST-FIXTURE-GO-INIT\n// @vtest.covers VO-FIXTURE-ONE\n// @vtest.target calc/calc.go::init\n// @vtest.intent init is declared twice\nfunc TestInit(t *testing.T) {}\n\n// @vtest.id TEST-FIXTURE-GO-ADD\n// @vtest.covers VO-FIXTURE-ONE\n// @vtest.target calc/calc.go::Add\n// @vtest.intent Add is declared once\nfunc TestAdd(t *testing.T) {}\n";
    let fixture = support::fixture(
        "go",
        &[("calc/calc.go", calc), ("calc/calc_test.go", tests)],
        &["VO-FIXTURE-ONE"],
    );
    let (scan, _) = fixture.scan_and_verify();
    let ambiguous = scan
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "E-SCAN-004")
        .collect::<Vec<_>>();
    assert_eq!(ambiguous.len(), 1, "{:?}", scan.diagnostics);
    assert!(
        ambiguous[0].message.contains("calc/calc.go::init"),
        "{}",
        ambiguous[0].message
    );
    assert_eq!(scan.tests.len(), 2);
}
