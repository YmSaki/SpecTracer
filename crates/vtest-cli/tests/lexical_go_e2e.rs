//! End-to-end check that the `go` adapter (Go) is registered in the
//! CLI composition root and that its Tests scan and verify through the
//! unmodified core with the discovery-only verdict.

mod support;

const SOURCE: &str = r##"package calc

import "testing"

func Add(a, b int) int {
	return a + b
}

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
/// @vtest.target crates/vtest-cli/src/adapters.rs::builtin_registry
/// @vtest.intent verifies the go adapter is in the built-in registry and a declared Go Test is scanned, linked to its VO and target, and verified with chain_integrity and orphan_detection PASS and the runner-dependent checks NO_EVIDENCE
#[test]
fn go_adapter_scans_and_verifies_through_the_cli_registry() {
    let fixture = support::fixture("go", &[("calc/calc.go", SOURCE)], &["VO-FIXTURE-ONE"]);
    let (scan, outcome) = fixture.scan_and_verify();
    assert!(!scan.has_errors(), "{:?}", scan.diagnostics);
    assert_eq!(scan.tests.len(), 1);
    assert_eq!(scan.tests[0].id.as_str(), "TEST-FIXTURE-GO-ADD");
    assert_eq!(scan.tests[0].execution.adapter.as_str(), "go");
    support::assert_discovery_only_verdict(&outcome, "TEST-FIXTURE-GO-ADD");
}
