//! End-to-end check that the `csharp` adapter (C#) is registered in the
//! CLI composition root and that its Tests scan and verify through the
//! unmodified core with the discovery-only verdict.

mod support;

const SOURCE: &str = r##"namespace Demo;

public static class Calc
{
    public static int Add(int a, int b) => a + b;
}

public class CalcTests
{
    // @vtest.id TEST-FIXTURE-CSHARP-ADD
    // @vtest.covers VO-FIXTURE-ONE
    // @vtest.target src/Calc.cs::Demo::Calc::Add
    // @vtest.intent adding two numbers returns their sum
    [Fact]
    public void Adds()
    {
        Assert.Equal(3, Calc.Add(1, 2));
    }
}
"##;

/// @vtest.id TEST-CLI-CSHARP-ADAPTER-SCAN-AND-VERIFY
/// @vtest.covers VO-ADAPTER-CSHARP-REGISTERS-WITHOUT-CORE-CHANGES
/// @vtest.target crates/vtest-ops/src/registry.rs::builtin_registry
/// @vtest.intent verifies the csharp adapter is in the built-in registry and a declared C# Test is scanned, linked to its VO and target, and verified with chain_integrity and orphan_detection PASS and the runner-dependent checks NO_EVIDENCE
#[test]
fn csharp_adapter_scans_and_verifies_through_the_cli_registry() {
    let fixture = support::fixture("csharp", &[("src/Calc.cs", SOURCE)], &["VO-FIXTURE-ONE"]);
    let (scan, outcome) = fixture.scan_and_verify();
    assert!(!scan.has_errors(), "{:?}", scan.diagnostics);
    assert_eq!(scan.tests.len(), 1);
    assert_eq!(scan.tests[0].id.as_str(), "TEST-FIXTURE-CSHARP-ADD");
    assert_eq!(scan.tests[0].execution.adapter.as_str(), "csharp");
    support::assert_discovery_only_verdict(&outcome, "TEST-FIXTURE-CSHARP-ADD");
}

/// @vtest.id TEST-CLI-CSHARP-BODY-TAIL-CHANGES-SUBJECT-HASH
/// @vtest.covers VO-ADAPTER-CSHARP-REGISTERS-WITHOUT-CORE-CHANGES
/// @vtest.target crates/vtest-ops/src/registry.rs::builtin_registry
/// @vtest.intent verifies that for a C# Test whose attribute holds braces, an edit to only the end of the method body changes the Test's content hash, so the whole body is hashed
#[test]
fn csharp_body_tail_edit_changes_the_test_hash() {
    let source = |tail: &str| {
        format!(
            "public class CalcTests\n{{\n    // @vtest.id TEST-FIXTURE-CSHARP-TAIL\n    // @vtest.covers VO-FIXTURE-ONE\n    // @vtest.intent the body tail is hashed\n    [Values(new int[] {{ 1, 2 }})]\n    public void Adds() {{ Marker(); {tail} }}\n}}\n"
        )
    };
    let hash = |tail: &str| {
        let fixture = support::fixture(
            "csharp",
            &[("src/CalcTests.cs", &source(tail))],
            &["VO-FIXTURE-ONE"],
        );
        let (scan, _) = fixture.scan_and_verify();
        assert!(!scan.has_errors(), "{:?}", scan.diagnostics);
        assert_eq!(scan.tests.len(), 1);
        scan.tests[0].content_hash.clone()
    };
    assert_ne!(hash("One();"), hash("Two();"));
}
