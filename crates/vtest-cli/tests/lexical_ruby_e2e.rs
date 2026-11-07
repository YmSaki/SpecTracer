//! End-to-end check that the `ruby` adapter (Ruby) is registered in the
//! CLI composition root and that its Tests scan and verify through the
//! unmodified core with the discovery-only verdict.

mod support;

const SOURCE: &str = r##"class Calc
  def add(a, b)
    a + b
  end
end

class CalcTest < Minitest::Test
  # @vtest.id TEST-FIXTURE-RUBY-ADD
  # @vtest.covers VO-FIXTURE-ONE
  # @vtest.target lib/calc.rb::Calc::add
  # @vtest.intent adding two numbers returns their sum
  def test_add
    assert_equal 3, Calc.new.add(1, 2)
  end
end
"##;

/// @vtest.id TEST-CLI-RUBY-ADAPTER-SCAN-AND-VERIFY
/// @vtest.covers VO-ADAPTER-RUBY-REGISTERS-WITHOUT-CORE-CHANGES
/// @vtest.target crates/vtest-cli/src/adapters.rs::builtin_registry
/// @vtest.intent verifies the ruby adapter is in the built-in registry and a declared Ruby Test is scanned, linked to its VO and target, and verified with chain_integrity and orphan_detection PASS and the runner-dependent checks NO_EVIDENCE
#[test]
fn ruby_adapter_scans_and_verifies_through_the_cli_registry() {
    let fixture = support::fixture("ruby", &[("lib/calc.rb", SOURCE)], &["VO-FIXTURE-ONE"]);
    let (scan, outcome) = fixture.scan_and_verify();
    assert!(!scan.has_errors(), "{:?}", scan.diagnostics);
    assert_eq!(scan.tests.len(), 1);
    assert_eq!(scan.tests[0].id.as_str(), "TEST-FIXTURE-RUBY-ADD");
    assert_eq!(scan.tests[0].execution.adapter.as_str(), "ruby");
    support::assert_discovery_only_verdict(&outcome, "TEST-FIXTURE-RUBY-ADD");
}

/// @vtest.id TEST-CLI-RUBY-BODY-TAIL-CHANGES-SUBJECT-HASH
/// @vtest.covers VO-ADAPTER-RUBY-REGISTERS-WITHOUT-CORE-CHANGES
/// @vtest.target crates/vtest-cli/src/adapters.rs::builtin_registry
/// @vtest.intent verifies that for a Ruby Test whose body holds a modifier if and an end inside a string, an edit to only the end of the body changes the Test's content hash, so the whole body is hashed
#[test]
fn ruby_body_tail_edit_changes_the_test_hash() {
    let source = |tail: &str| {
        format!(
            "# @vtest.id TEST-FIXTURE-RUBY-TAIL\n# @vtest.covers VO-FIXTURE-ONE\n# @vtest.intent the body tail is hashed\nit \"keeps the tail\" do\n  marker if ready\n  label = \"end\"\n  {tail}\nend\n"
        )
    };
    let hash = |tail: &str| {
        let fixture = support::fixture(
            "ruby",
            &[("spec/tail_spec.rb", &source(tail))],
            &["VO-FIXTURE-ONE"],
        );
        let (scan, _) = fixture.scan_and_verify();
        assert!(!scan.has_errors(), "{:?}", scan.diagnostics);
        assert_eq!(scan.tests.len(), 1);
        scan.tests[0].content_hash.clone()
    };
    assert_ne!(hash("one"), hash("two"));
}
