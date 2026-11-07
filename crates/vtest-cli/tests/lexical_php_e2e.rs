//! End-to-end check that the `php` adapter (PHP) is registered in the
//! CLI composition root and that its Tests scan and verify through the
//! unmodified core with the discovery-only verdict.

mod support;

const SOURCE: &str = r##"<?php
namespace App;

final class Calc
{
    public function add(int $a, int $b): int
    {
        return $a + $b;
    }
}

// @vtest.id TEST-FIXTURE-PHP-ADD
// @vtest.covers VO-FIXTURE-ONE
// @vtest.target src/Calc.php::App::Calc::add
// @vtest.intent adding two numbers returns their sum
test('adds', function () {
    expect((new Calc())->add(1, 2))->toBe(3);
});
"##;

/// @vtest.id TEST-CLI-PHP-ADAPTER-SCAN-AND-VERIFY
/// @vtest.covers VO-ADAPTER-PHP-REGISTERS-WITHOUT-CORE-CHANGES
/// @vtest.target crates/vtest-cli/src/adapters.rs::builtin_registry
/// @vtest.intent verifies the php adapter is in the built-in registry and a declared PHP Test is scanned, linked to its VO and target, and verified with chain_integrity and orphan_detection PASS and the runner-dependent checks NO_EVIDENCE
#[test]
fn php_adapter_scans_and_verifies_through_the_cli_registry() {
    let fixture = support::fixture("php", &[("src/Calc.php", SOURCE)], &["VO-FIXTURE-ONE"]);
    let (scan, outcome) = fixture.scan_and_verify();
    assert!(!scan.has_errors(), "{:?}", scan.diagnostics);
    assert_eq!(scan.tests.len(), 1);
    assert_eq!(scan.tests[0].id.as_str(), "TEST-FIXTURE-PHP-ADD");
    assert_eq!(scan.tests[0].execution.adapter.as_str(), "php");
    support::assert_discovery_only_verdict(&outcome, "TEST-FIXTURE-PHP-ADD");
}

/// @vtest.id TEST-CLI-PHP-BODY-TAIL-CHANGES-SUBJECT-HASH
/// @vtest.covers VO-ADAPTER-PHP-REGISTERS-WITHOUT-CORE-CHANGES
/// @vtest.target crates/vtest-cli/src/adapters.rs::builtin_registry
/// @vtest.intent verifies that for a PHP Test whose attribute holds braces, an edit to only the end of the method body changes the Test's content hash, so the whole body is hashed
#[test]
fn php_body_tail_edit_changes_the_test_hash() {
    let source = |tail: &str| {
        format!(
            "<?php\nclass TailTest\n{{\n    // @vtest.id TEST-FIXTURE-PHP-TAIL\n    // @vtest.covers VO-FIXTURE-ONE\n    // @vtest.intent the body tail is hashed\n    #[TestWith(['{{'])]\n    public function testTail(): void\n    {{\n        marker();\n        {tail}\n    }}\n}}\n"
        )
    };
    let hash = |tail: &str| {
        let fixture = support::fixture(
            "php",
            &[("tests/TailTest.php", &source(tail))],
            &["VO-FIXTURE-ONE"],
        );
        let (scan, _) = fixture.scan_and_verify();
        assert!(!scan.has_errors(), "{:?}", scan.diagnostics);
        assert_eq!(scan.tests.len(), 1);
        scan.tests[0].content_hash.clone()
    };
    assert_ne!(hash("one();"), hash("two();"));
}
