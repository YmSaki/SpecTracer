//! End-to-end check that the `python` adapter (Python) is registered in the
//! CLI composition root and that its Tests scan and verify through the
//! unmodified core with the discovery-only verdict.

mod support;

const SOURCE: &str = r##"def add(a, b):
    return a + b


# @vtest.id TEST-FIXTURE-PYTHON-ADD
# @vtest.covers VO-FIXTURE-ONE
# @vtest.target src/calc.py::add
# @vtest.intent adding two numbers returns their sum
def test_add():
    assert add(1, 2) == 3
"##;

/// @vtest.id TEST-CLI-PYTHON-ADAPTER-SCAN-AND-VERIFY
/// @vtest.covers VO-ADAPTER-PYTHON-REGISTERS-WITHOUT-CORE-CHANGES
/// @vtest.target crates/vtest-ops/src/registry.rs::builtin_registry
/// @vtest.intent verifies the python adapter is in the built-in registry and a declared Python Test is scanned, linked to its VO and target, and verified with chain_integrity and orphan_detection PASS and the runner-dependent checks NO_EVIDENCE
#[test]
fn python_adapter_scans_and_verifies_through_the_cli_registry() {
    let fixture = support::fixture("python", &[("src/calc.py", SOURCE)], &["VO-FIXTURE-ONE"]);
    let (scan, outcome) = fixture.scan_and_verify();
    assert!(!scan.has_errors(), "{:?}", scan.diagnostics);
    assert_eq!(scan.tests.len(), 1);
    assert_eq!(scan.tests[0].id.as_str(), "TEST-FIXTURE-PYTHON-ADD");
    assert_eq!(scan.tests[0].execution.adapter.as_str(), "python");
    support::assert_discovery_only_verdict(&outcome, "TEST-FIXTURE-PYTHON-ADD");
}

/// @vtest.id TEST-CLI-PYTHON-BODY-TAIL-CHANGES-SUBJECT-HASH
/// @vtest.covers VO-ADAPTER-PYTHON-REGISTERS-WITHOUT-CORE-CHANGES
/// @vtest.target crates/vtest-ops/src/registry.rs::builtin_registry
/// @vtest.intent verifies that for a Python Test whose body holds dedented string text, an edit to only the end of the body changes the Test's content hash, so the whole body is hashed
#[test]
fn python_body_tail_edit_changes_the_test_hash() {
    let source = |tail: &str| {
        format!(
            "# @vtest.id TEST-FIXTURE-PYTHON-TAIL\n# @vtest.covers VO-FIXTURE-ONE\n# @vtest.intent the body tail is hashed\ndef test_tail():\n    text = \"\"\"\nnot indented\n\"\"\"\n    {tail}\n"
        )
    };
    let hash = |tail: &str| {
        let fixture = support::fixture(
            "python",
            &[("tests/test_tail.py", &source(tail))],
            &["VO-FIXTURE-ONE"],
        );
        let (scan, _) = fixture.scan_and_verify();
        assert!(!scan.has_errors(), "{:?}", scan.diagnostics);
        assert_eq!(scan.tests.len(), 1);
        scan.tests[0].content_hash.clone()
    };
    assert_ne!(hash("one()"), hash("two()"));
}
