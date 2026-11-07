//! End-to-end check that the `java` adapter (Java) is registered in the
//! CLI composition root and that its Tests scan and verify through the
//! unmodified core with the discovery-only verdict.

mod support;

const SOURCE: &str = r##"public class Calc {
    public int add(int a, int b) { return a + b; }
}

class CalcTest {
    // @vtest.id TEST-FIXTURE-JAVA-ADD
    // @vtest.covers VO-FIXTURE-ONE
    // @vtest.target src/Calc.java::Calc::add
    // @vtest.intent adding two numbers returns their sum
    @Test
    void adds() { assertEquals(3, new Calc().add(1, 2)); }
}
"##;

/// @vtest.id TEST-CLI-JAVA-ADAPTER-SCAN-AND-VERIFY
/// @vtest.covers VO-ADAPTER-JAVA-REGISTERS-WITHOUT-CORE-CHANGES
/// @vtest.target crates/vtest-cli/src/adapters.rs::builtin_registry
/// @vtest.intent verifies the java adapter is in the built-in registry and a declared Java Test is scanned, linked to its VO and target, and verified with chain_integrity and orphan_detection PASS and the runner-dependent checks NO_EVIDENCE
#[test]
fn java_adapter_scans_and_verifies_through_the_cli_registry() {
    let fixture = support::fixture("java", &[("src/Calc.java", SOURCE)], &["VO-FIXTURE-ONE"]);
    let (scan, outcome) = fixture.scan_and_verify();
    assert!(!scan.has_errors(), "{:?}", scan.diagnostics);
    assert_eq!(scan.tests.len(), 1);
    assert_eq!(scan.tests[0].id.as_str(), "TEST-FIXTURE-JAVA-ADD");
    assert_eq!(scan.tests[0].execution.adapter.as_str(), "java");
    support::assert_discovery_only_verdict(&outcome, "TEST-FIXTURE-JAVA-ADD");
}
