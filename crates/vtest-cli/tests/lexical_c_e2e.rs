//! End-to-end check that the `c` adapter (C) is registered in the
//! CLI composition root and that its Tests scan and verify through the
//! unmodified core with the discovery-only verdict.

mod support;

const SOURCE: &str = r##"#include "unity.h"

int add(int a, int b)
{
    return a + b;
}

// @vtest.id TEST-FIXTURE-C-ADD
// @vtest.covers VO-FIXTURE-ONE
// @vtest.target src/calc.c::add
// @vtest.intent adding two numbers returns their sum
void test_add(void)
{
    TEST_ASSERT_EQUAL(3, add(1, 2));
}
"##;

/// @vtest.id TEST-CLI-C-ADAPTER-SCAN-AND-VERIFY
/// @vtest.covers VO-ADAPTER-C-REGISTERS-WITHOUT-CORE-CHANGES
/// @vtest.target crates/vtest-cli/src/adapters.rs::builtin_registry
/// @vtest.intent verifies the c adapter is in the built-in registry and a declared C Test is scanned, linked to its VO and target, and verified with chain_integrity and orphan_detection PASS and the runner-dependent checks NO_EVIDENCE
#[test]
fn c_adapter_scans_and_verifies_through_the_cli_registry() {
    let fixture = support::fixture("c", &[("src/calc.c", SOURCE)], &["VO-FIXTURE-ONE"]);
    let (scan, outcome) = fixture.scan_and_verify();
    assert!(!scan.has_errors(), "{:?}", scan.diagnostics);
    assert_eq!(scan.tests.len(), 1);
    assert_eq!(scan.tests[0].id.as_str(), "TEST-FIXTURE-C-ADD");
    assert_eq!(scan.tests[0].execution.adapter.as_str(), "c");
    support::assert_discovery_only_verdict(&outcome, "TEST-FIXTURE-C-ADD");
}
