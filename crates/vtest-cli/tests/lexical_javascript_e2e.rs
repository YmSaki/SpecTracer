//! End-to-end check that the `javascript` adapter (JavaScript/TypeScript) is registered in the
//! CLI composition root and that its Tests scan and verify through the
//! unmodified core with the discovery-only verdict.

mod support;

const SOURCE: &str = r##"export function add(a, b) {
  return a + b;
}

// @vtest.id TEST-FIXTURE-JAVASCRIPT-ADD
// @vtest.covers VO-FIXTURE-ONE
// @vtest.target src/calc.js::add
// @vtest.intent adding two numbers returns their sum
test("adds", () => {
  expect(add(1, 2)).toBe(3);
});
"##;

/// @vtest.id TEST-CLI-JAVASCRIPT-ADAPTER-SCAN-AND-VERIFY
/// @vtest.covers VO-ADAPTER-JAVASCRIPT-REGISTERS-WITHOUT-CORE-CHANGES
/// @vtest.target crates/vtest-cli/src/adapters.rs::builtin_registry
/// @vtest.intent verifies the javascript adapter is in the built-in registry and a declared JavaScript/TypeScript Test is scanned, linked to its VO and target, and verified with chain_integrity and orphan_detection PASS and the runner-dependent checks NO_EVIDENCE
#[test]
fn javascript_adapter_scans_and_verifies_through_the_cli_registry() {
    let fixture = support::fixture(
        "javascript",
        &[("src/calc.js", SOURCE)],
        &["VO-FIXTURE-ONE"],
    );
    let (scan, outcome) = fixture.scan_and_verify();
    assert!(!scan.has_errors(), "{:?}", scan.diagnostics);
    assert_eq!(scan.tests.len(), 1);
    assert_eq!(scan.tests[0].id.as_str(), "TEST-FIXTURE-JAVASCRIPT-ADD");
    assert_eq!(scan.tests[0].execution.adapter.as_str(), "javascript");
    support::assert_discovery_only_verdict(&outcome, "TEST-FIXTURE-JAVASCRIPT-ADD");
}
