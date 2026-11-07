//! End-to-end check that the `kotlin` adapter (Kotlin) is registered in the
//! CLI composition root and that its Tests scan and verify through the
//! unmodified core with the discovery-only verdict.

mod support;

const SOURCE: &str = r##"class Calc {
    fun add(a: Int, b: Int): Int = a + b
}

class CalcTest {
    // @vtest.id TEST-FIXTURE-KOTLIN-ADD
    // @vtest.covers VO-FIXTURE-ONE
    // @vtest.target src/Calc.kt::Calc::add
    // @vtest.intent adding two numbers returns their sum
    @Test
    fun `adds two numbers`() {
        assertEquals(3, Calc().add(1, 2))
    }
}
"##;

/// @vtest.id TEST-CLI-KOTLIN-ADAPTER-SCAN-AND-VERIFY
/// @vtest.covers VO-ADAPTER-KOTLIN-REGISTERS-WITHOUT-CORE-CHANGES
/// @vtest.target crates/vtest-cli/src/adapters.rs::builtin_registry
/// @vtest.intent verifies the kotlin adapter is in the built-in registry and a declared Kotlin Test is scanned, linked to its VO and target, and verified with chain_integrity and orphan_detection PASS and the runner-dependent checks NO_EVIDENCE
#[test]
fn kotlin_adapter_scans_and_verifies_through_the_cli_registry() {
    let fixture = support::fixture("kotlin", &[("src/Calc.kt", SOURCE)], &["VO-FIXTURE-ONE"]);
    let (scan, outcome) = fixture.scan_and_verify();
    assert!(!scan.has_errors(), "{:?}", scan.diagnostics);
    assert_eq!(scan.tests.len(), 1);
    assert_eq!(scan.tests[0].id.as_str(), "TEST-FIXTURE-KOTLIN-ADD");
    assert_eq!(scan.tests[0].execution.adapter.as_str(), "kotlin");
    support::assert_discovery_only_verdict(&outcome, "TEST-FIXTURE-KOTLIN-ADD");
}
