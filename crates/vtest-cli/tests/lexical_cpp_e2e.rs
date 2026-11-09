//! End-to-end check that the `cpp` adapter (C++) is registered in the
//! CLI composition root and that its Tests scan and verify through the
//! unmodified core with the discovery-only verdict.

mod support;

const SOURCE: &str = r##"namespace demo {
int add(int a, int b) { return a + b; }
}

// @vtest.id TEST-FIXTURE-CPP-ADD
// @vtest.covers VO-FIXTURE-ONE
// @vtest.target src/calc.cpp::demo::add
// @vtest.intent adding two numbers returns their sum
TEST(CalcTest, Adds) {
    EXPECT_EQ(demo::add(1, 2), 3);
}
"##;

/// @vtest.id TEST-CLI-CPP-ADAPTER-SCAN-AND-VERIFY
/// @vtest.covers VO-ADAPTER-CPP-REGISTERS-WITHOUT-CORE-CHANGES
/// @vtest.target crates/vtest-ops/src/registry.rs::builtin_registry
/// @vtest.intent verifies the cpp adapter is in the built-in registry and a declared C++ Test is scanned, linked to its VO and target, and verified with chain_integrity and orphan_detection PASS and the runner-dependent checks NO_EVIDENCE
#[test]
fn cpp_adapter_scans_and_verifies_through_the_cli_registry() {
    let fixture = support::fixture("cpp", &[("src/calc.cpp", SOURCE)], &["VO-FIXTURE-ONE"]);
    let (scan, outcome) = fixture.scan_and_verify();
    assert!(!scan.has_errors(), "{:?}", scan.diagnostics);
    assert_eq!(scan.tests.len(), 1);
    assert_eq!(scan.tests[0].id.as_str(), "TEST-FIXTURE-CPP-ADD");
    assert_eq!(scan.tests[0].execution.adapter.as_str(), "cpp");
    support::assert_discovery_only_verdict(&outcome, "TEST-FIXTURE-CPP-ADD");
}

/// @vtest.id TEST-CLI-CPP-BODY-TAIL-CHANGES-SUBJECT-HASH
/// @vtest.covers VO-ADAPTER-CPP-REGISTERS-WITHOUT-CORE-CHANGES
/// @vtest.target crates/vtest-ops/src/registry.rs::builtin_registry
/// @vtest.intent verifies that for a C++ Test whose constructor initializer list holds braces, an edit to only the end of the body changes the Test's content hash, so the whole body is hashed
#[test]
fn cpp_body_tail_edit_changes_the_test_hash() {
    let source = |tail: &str| {
        format!(
            "struct Fixture {{\n    // @vtest.id TEST-FIXTURE-CPP-TAIL\n    // @vtest.covers VO-FIXTURE-ONE\n    // @vtest.intent the body tail is hashed\n    Fixture() : values_{{1, 2}} {{ marker(); {tail} }}\n    int values_[2];\n}};\n"
        )
    };
    let hash = |tail: &str| {
        let fixture = support::fixture(
            "cpp",
            &[("src/tail.cpp", &source(tail))],
            &["VO-FIXTURE-ONE"],
        );
        let (scan, _) = fixture.scan_and_verify();
        assert!(!scan.has_errors(), "{:?}", scan.diagnostics);
        assert_eq!(scan.tests.len(), 1);
        scan.tests[0].content_hash.clone()
    };
    assert_ne!(hash("one();"), hash("two();"));
}
