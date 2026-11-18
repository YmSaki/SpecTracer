//! End-to-end check that the `swift` adapter (Swift) is registered in the
//! CLI composition root and that its Tests scan and verify through the
//! unmodified core with the discovery-only verdict.

mod support;

const SOURCE: &str = r##"struct Calc {
    func add(_ a: Int, _ b: Int) -> Int {
        return a + b
    }
}

final class CalcTests: XCTestCase {
    // @vtest.id TEST-FIXTURE-SWIFT-ADD
    // @vtest.covers VO-FIXTURE-ONE
    // @vtest.target Sources/Calc.swift::Calc::add
    // @vtest.intent adding two numbers returns their sum
    func testAdd() {
        XCTAssertEqual(Calc().add(1, 2), 3)
    }
}
"##;

/// @vtest.id TEST-CLI-SWIFT-ADAPTER-SCAN-AND-VERIFY
/// @vtest.covers VO-ADAPTER-SWIFT-REGISTERS-WITHOUT-CORE-CHANGES
/// @vtest.target crates/vtest-ops/src/registry.rs::builtin_registry
/// @vtest.intent verifies the swift adapter is in the built-in registry and a declared Swift Test is scanned, linked to its VO and target, and verified with chain_integrity and orphan_detection PASS and the runner-dependent checks NO_EVIDENCE
#[test]
fn swift_adapter_scans_and_verifies_through_the_cli_registry() {
    let fixture = support::fixture(
        "swift",
        &[("Sources/Calc.swift", SOURCE)],
        &["VO-FIXTURE-ONE"],
    );
    let (scan, outcome) = fixture.scan_and_verify();
    assert!(!scan.has_errors(), "{:?}", scan.diagnostics);
    assert_eq!(scan.tests.len(), 1);
    assert_eq!(scan.tests[0].id.as_str(), "TEST-FIXTURE-SWIFT-ADD");
    assert_eq!(scan.tests[0].execution.adapter.as_str(), "swift");
    support::assert_discovery_only_verdict(&outcome, "TEST-FIXTURE-SWIFT-ADD");
}

/// @vtest.id TEST-CLI-SWIFT-BODY-TAIL-CHANGES-SUBJECT-HASH
/// @vtest.covers VO-ADAPTER-SWIFT-REGISTERS-WITHOUT-CORE-CHANGES
/// @vtest.target crates/vtest-ops/src/registry.rs::builtin_registry
/// @vtest.intent verifies that for a Swift Test with a default argument closure, an edit to only the end of the function body changes the Test's content hash, so the whole body is hashed
#[test]
fn swift_body_tail_edit_changes_the_test_hash() {
    let source = |tail: &str| {
        format!(
            "// @vtest.id TEST-FIXTURE-SWIFT-TAIL\n// @vtest.covers VO-FIXTURE-ONE\n// @vtest.intent the body tail is hashed\n@Test\nfunc tail(make: () -> Int = {{ 1 }}) {{\n    marker()\n    {tail}\n}}\n"
        )
    };
    let hash = |tail: &str| {
        let fixture = support::fixture(
            "swift",
            &[("Tests/TailTests.swift", &source(tail))],
            &["VO-FIXTURE-ONE"],
        );
        let (scan, _) = fixture.scan_and_verify();
        assert!(!scan.has_errors(), "{:?}", scan.diagnostics);
        assert_eq!(scan.tests.len(), 1);
        scan.tests[0].content_hash.clone()
    };
    assert_ne!(hash("one()"), hash("two()"));
}
