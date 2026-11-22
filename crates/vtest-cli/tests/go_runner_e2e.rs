//! End-to-end tests of the `go` Test Runner: a real Go project, the real `go`
//! command, and the unmodified `vtest run` operation and `vtest verify` core
//! through the CLI's composition root. Each test follows one clause of the go
//! runner from the `go test` output to what is, or is not, recorded.
//!
//! `go` and `git` must be on `PATH`: a missing one fails the test with a message
//! instead of passing without having run anything.
//!
//! The Evidence of a Go Test is never turned into a `PASS` here, whatever the
//! Test did: the Tests are run with `--fast` (DS-1923), so their `target_coverage`
//! is not checked and `target_binding` is `NO_EVIDENCE` (`NOT_CHECKED`, DS-778),
//! and a Test that did not produce Evidence is `NO_EVIDENCE` (`NOT_EXECUTED`,
//! DS-825) or, when the runner reported a failure, `FAIL` (DS-830).
//! `oracle_presence` is the static audit's verdict on the text of the Test
//! (DS-605 to DS-608), which does not depend on the run: the Test that checks
//! its target's result is `PASS`, and the ones below that do not check it are
//! `FAIL` (the result of `_ = Add(1, 2)`, `t.Fatal("..")` and `t.Skip("..")`
//! reaches no assertion, DS-623, DS-1928) or `UNKNOWN` (a Test that does not call
//! the target, DS-634). The wrong-outcome tests below check that the Evidence is
//! the true one and that each state is the one of its clause.
//! `go_coverage_e2e.rs` has the runs that measure coverage and
//! `go_static_audit_e2e.rs` the ones that check the audit.

mod support;

use std::fs;

use support::go_project::{codes, only_evidence, project, run_and_verify, state_of, state_of_test};
use vtest_exec::ExecutionResult;
use vtest_model::{DiagnosticLabel, TestResult, VerificationCheck, VerificationState};
use vtest_scan::scan_project;
use vtest_store::VerifyLayout;
use vtest_verify::{verify_project, VerifyOutcome};

const GO_MOD: &str = "module app\n\ngo 1.21\n";
const CALC: &str = "package app\n\nfunc Add(a, b int) int { return a + b }\n";
const TARGET: &str = "app/calc.go::Add";

/// A declared Test function; `body` is Go.
fn declared(id: &str, name: &str, body: &str) -> String {
    format!(
        "// @vtest.id {id}\n// @vtest.covers VO-FIXTURE-ONE\n// @vtest.target {TARGET}\n// @vtest.intent {name} is checked\nfunc {name}(t *testing.T) {{\n{body}\n}}\n"
    )
}

fn test_file(imports: &[&str], items: &str) -> String {
    let imports = imports
        .iter()
        .map(|import| format!("\t\"{import}\"\n"))
        .collect::<String>();
    format!("package app\n\nimport (\n{imports})\n\n{items}")
}

/// A project of one module with one Test file.
fn calc_project(test_source: &str) -> support::Fixture {
    project(&[
        ("go.mod", GO_MOD),
        ("app/calc.go", CALC),
        ("app/calc_test.go", test_source),
    ])
}

/// Nothing a wrong outcome produced may stand as `PASS`: no `PASS` Evidence, and
/// `target_binding` is the state of its clause: `FAIL` for the Evidence of a
/// failing Test (DS-830) and `NO_EVIDENCE` with `NOT_EXECUTED` for a Test that
/// has no Evidence (DS-825); `oracle_presence` is `oracle`, the state the static
/// audit gives the text of the Test (it does not depend on the run).
fn assert_nothing_passes(
    result: &ExecutionResult,
    outcome: &VerifyOutcome,
    oracle: VerificationState,
) {
    assert!(
        result
            .evidence
            .iter()
            .all(|record| record.result != TestResult::Pass),
        "{:?}",
        result.evidence
    );
    let target_binding = if result.evidence.is_empty() {
        (
            VerificationState::NoEvidence,
            vec![DiagnosticLabel::NotExecuted],
        )
    } else {
        (VerificationState::Fail, Vec::new())
    };
    assert_eq!(
        state_of(outcome, VerificationCheck::TargetBinding),
        target_binding
    );
    assert_eq!(
        state_of(outcome, VerificationCheck::OraclePresence),
        (oracle, Vec::new())
    );
}

/// DES-647 to DES-652, DS-1905 to DS-1907: a passing Test is run by `go test`
/// and recorded as `PASS` Evidence bound to a complete Execution State, with the
/// whole output kept in the log; target binding stays not checked, so the Test as
/// a whole is not `PASS` although its `oracle_presence` is.
/// @vtest.id TEST-CLI-GO-RUN-PASS-RECORDS-EVIDENCE-AND-TARGET-BINDING-STAYS-NOT-CHECKED
/// @vtest.covers VO-ADAPTER-GO-RESULT-IS-READ-FROM-GO-TEST, VO-ADAPTER-GO-RUNNER-KIND-IS-GO-TEST, VO-ADAPTER-GO-RUNS-ONE-TEST-PER-INVOCATION, VO-ADAPTER-GO-OUTPUT-IS-KEPT-IN-THE-LOG-OF-THE-EVIDENCE, VO-ADAPTER-GO-JUDGEMENT-AUTHORITY-IS-GO-TEST, VO-VERIFY-TARGET-BINDING-NO-COVERAGE-CAPABILITY, VO-ADAPTER-GO-AUDIT-ALL-NO-VIOLATION-IS-PASS
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent a passing Go Test that checks its target result, run through vtest run with fast, records PASS Evidence of runner kind go-test with the go test command line and exit code 0, an unchecked target coverage and a complete go-execution-state, keeps the json output in its log, and verification reports target_binding as NO_EVIDENCE with NOT_CHECKED and oracle_presence as PASS while chain_integrity and orphan_detection pass
#[test]
fn a_passing_go_test_records_pass_evidence_and_target_binding_stays_not_checked() {
    let fixture = calc_project(&test_file(
        &["testing"],
        &declared(
            "TEST-E2E-GO-PASS",
            "TestAdd",
            "\tif Add(1, 2) != 3 {\n\t\tt.Fatal(\"sum\")\n\t}",
        ),
    ));
    let (result, outcome) = run_and_verify(&fixture, true);
    let record = only_evidence(&result);
    assert_eq!(record.result, TestResult::Pass);
    assert_eq!(record.adapter.as_str(), "go");
    assert_eq!(record.runner.kind, "go-test");
    assert_eq!(
        record.runner.command,
        "go test ./app -run ^TestAdd$ -count=1 -json"
    );
    assert_eq!(record.runner.exit_code, 0);
    assert!(!record.target_coverage.checked);
    assert_eq!(record.execution_state.schema, "go-execution-state-v1");
    assert!(
        record.execution_state.complete,
        "{:?}",
        record.execution_state
    );
    assert!(record.execution_state.hash.is_some());
    assert_eq!(record.log_ref, format!("cache/logs/{}.log", record.id));
    let log = fs::read_to_string(
        VerifyLayout::new(&fixture.root)
            .verify_dir()
            .join(&record.log_ref),
    )
    .expect("the log of the run");
    assert!(
        log.contains("\"Action\":\"pass\",\"Package\":\"app/app\",\"Test\":\"TestAdd\""),
        "{log}"
    );
    assert!(codes(&result).is_empty(), "{:?}", result.diagnostics);

    assert_eq!(
        state_of(&outcome, VerificationCheck::ChainIntegrity).0,
        VerificationState::Pass
    );
    assert_eq!(
        state_of(&outcome, VerificationCheck::OrphanDetection).0,
        VerificationState::Pass
    );
    assert_eq!(
        state_of(&outcome, VerificationCheck::TargetBinding),
        (
            VerificationState::NoEvidence,
            vec![DiagnosticLabel::NotChecked]
        )
    );
    assert_eq!(
        state_of(&outcome, VerificationCheck::OraclePresence),
        (VerificationState::Pass, Vec::new())
    );

    // `vtest test show` reads the Evidence on its own: it shows the runner's
    // result as recorded, and the target coverage as not checked, never as PASS.
    let registry = vtest_ops::registry::builtin_registry().expect("builtin registry");
    let scan = scan_project(&fixture.root, &registry).expect("scan");
    let shown = vtest_scan::show_test(&fixture.root, &scan, "TEST-E2E-GO-PASS").expect("show");
    assert_eq!(shown.evidence.len(), 1);
    assert_eq!(shown.evidence[0].result, TestResult::Pass);
    assert_eq!(
        shown.evidence[0].target_coverage,
        VerificationState::NoEvidence
    );
}

/// DS-1923, DS-1926, DS-1102: with `--fast` no coverage is measured and the
/// command specifies none; without it the default mode measures coverage.
/// @vtest.id TEST-CLI-GO-FAST-RUNS-WITHOUT-COVERAGE-AND-THE-DEFAULT-MEASURES-IT
/// @vtest.covers VO-ADAPTER-GO-FAST-MODE-HAS-NO-COVERAGE, VO-ADAPTER-GO-DEFAULT-MODE-MEASURES-COVERAGE, VO-ADAPTER-GO-RUNNER-KIND-IS-GO-TEST
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent a Go Test run with fast records the plain go test command of runner kind go-test and an unchecked target_coverage with no method, result, targets or count, and the run without fast records the go test command with the coverage options of runner kind go-test-cover and a target_coverage checked with the method go-cover, and neither reports a diagnostic
#[test]
fn a_go_test_is_run_without_coverage_with_fast_and_with_it_by_default() {
    let fixture = calc_project(&test_file(
        &["testing"],
        &declared("TEST-E2E-GO-FAST", "TestAdd", "\t_ = Add(1, 2)"),
    ));
    let fast = run_and_verify(&fixture, true).0;
    let record = only_evidence(&fast);
    assert_eq!(record.result, TestResult::Pass);
    assert_eq!(record.runner.kind, "go-test");
    assert_eq!(
        record.runner.command,
        "go test ./app -run ^TestAdd$ -count=1 -json"
    );
    let coverage = &record.target_coverage;
    assert!(!coverage.checked);
    assert!(coverage.method.is_none() && coverage.result.is_none());
    assert!(coverage.targets.is_empty() && coverage.count.is_none());
    assert!(codes(&fast).is_empty());

    let measured = run_and_verify(&fixture, false).0;
    let record = only_evidence(&measured);
    assert_eq!(record.result, TestResult::Pass);
    assert_eq!(record.runner.kind, "go-test-cover");
    assert!(
        record.runner.command.starts_with(
            "go test ./app -run ^TestAdd$ -count=1 -json -covermode=atomic -coverpkg=./... \
             -coverprofile=.verify/cache/cov/"
        ),
        "{}",
        record.runner.command
    );
    assert!(record.target_coverage.checked);
    assert_eq!(record.target_coverage.method.as_deref(), Some("go-cover"));
    assert!(codes(&measured).is_empty(), "{:?}", measured.diagnostics);
}

/// DS-1907, DS-830: a failing Test is `FAIL` Evidence and nothing is `PASS`.
/// @vtest.id TEST-CLI-GO-FAILING-TEST-IS-FAIL-EVIDENCE
/// @vtest.covers VO-ADAPTER-GO-RESULT-IS-READ-FROM-GO-TEST, VO-ADAPTER-GO-RESULT-IS-THE-EXACT-NAME-EVENT, VO-ADAPTER-GO-JUDGEMENT-AUTHORITY-IS-GO-TEST
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent a Go Test that fails is recorded as FAIL Evidence with exit code 1 and no diagnostic, and verification reports target_binding FAIL and oracle_presence FAIL, since its only failure report has no deciding position and the target is not called
#[test]
fn a_failing_go_test_is_fail_evidence_and_nothing_passes() {
    let fixture = calc_project(&test_file(
        &["testing"],
        &declared("TEST-E2E-GO-FAIL", "TestAdd", "\tt.Fatal(\"wrong sum\")"),
    ));
    let (result, outcome) = run_and_verify(&fixture, true);
    let record = only_evidence(&result);
    assert_eq!(record.result, TestResult::Fail);
    assert_eq!(record.runner.exit_code, 1);
    assert!(codes(&result).is_empty(), "{:?}", result.diagnostics);
    assert_nothing_passes(&result, &outcome, VerificationState::Fail);
}

/// DS-1933 is a static-analysis clause; what the runner owes subtests is DES-649:
/// the result is the parent's own event, so a failing subtest fails the parent
/// and a passing one does not make a failing parent pass.
/// @vtest.id TEST-CLI-GO-SUBTEST-FAILURE-FAILS-THE-PARENT
/// @vtest.covers VO-ADAPTER-GO-RESULT-IS-THE-EXACT-NAME-EVENT
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent a Go Test whose second subtest fails is recorded as FAIL Evidence for the parent only, and one whose subtests pass is PASS
#[test]
fn a_failing_subtest_fails_its_parent_test() {
    let sub = |second: &str| {
        declared(
            "TEST-E2E-GO-SUB",
            "TestSub",
            &format!(
                "\tt.Run(\"a\", func(t *testing.T) {{}})\n\tt.Run(\"b\", func(t *testing.T) {{ {second} }})"
            ),
        )
    };
    let failing = calc_project(&test_file(&["testing"], &sub("t.Fatal(\"x\")")));
    let (result, outcome) = run_and_verify(&failing, true);
    assert_eq!(only_evidence(&result).result, TestResult::Fail);
    assert_nothing_passes(&result, &outcome, VerificationState::Fail);
    let passing = calc_project(&test_file(&["testing"], &sub("")));
    assert_eq!(
        only_evidence(&run_and_verify(&passing, true).0).result,
        TestResult::Pass
    );
}

/// DES-677, DS-1910: a package that does not build is E-EXEC-001 with no
/// Evidence, for every Test of the package.
/// @vtest.id TEST-CLI-GO-BUILD-FAILURE-IS-E-EXEC-001
/// @vtest.covers VO-ADAPTER-GO-BUILD-FAILURE-IS-E-EXEC-001, VO-ADAPTER-GO-BUILD-FAILURE-OUTRANKS-RESULT-EVENTS
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent a Go Test in a package that does not compile records no Evidence and is reported as E-EXEC-001, and verification reports no PASS
#[test]
fn a_package_that_does_not_build_is_e_exec_001_and_records_no_evidence() {
    let mut source = test_file(
        &["testing"],
        &declared("TEST-E2E-GO-BUILD", "TestAdd", "\t_ = Add(1, 2)"),
    );
    source.push_str("\nfunc broken() int { return \"not an int\" }\n");
    let fixture = calc_project(&source);
    let (result, outcome) = run_and_verify(&fixture, true);
    assert!(result.evidence.is_empty());
    assert_eq!(codes(&result), ["E-EXEC-001"]);
    assert_nothing_passes(&result, &outcome, VerificationState::Fail);
}

/// DES-678, DS-1911, DS-750: the exit status of `go test` contradicting the
/// Test's result, in both directions, is E-EXEC-003 with no Evidence.
/// @vtest.id TEST-CLI-GO-EXIT-STATUS-CONTRADICTION-IS-E-EXEC-003
/// @vtest.covers VO-ADAPTER-GO-EXIT-CODE-CONTRADICTION-IS-E-EXEC-003, VO-ADAPTER-GO-EXIT-CODE-CONTRADICTION-IS-DETECTED
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent a Go Test that passes while TestMain exits 1 after m.Run, and one that fails while TestMain exits 0, each record no Evidence and are reported as E-EXEC-003
#[test]
fn an_exit_status_that_contradicts_the_result_is_e_exec_003_in_both_directions() {
    let test_main =
        |exit: &str| format!("func TestMain(m *testing.M) {{\n\tm.Run()\n\tos.Exit({exit})\n}}\n");
    for (name, body, exit) in [
        ("passes, TestMain exits 1", "\t_ = Add(1, 2)", "1"),
        ("fails, TestMain exits 0", "\tt.Fatal(\"fails\")", "0"),
    ] {
        let source = test_file(
            &["os", "testing"],
            &format!(
                "{}\n{}",
                declared("TEST-E2E-GO-EXIT", "TestAdd", body),
                test_main(exit)
            ),
        );
        let fixture = calc_project(&source);
        let (result, outcome) = run_and_verify(&fixture, true);
        assert!(result.evidence.is_empty(), "{name}");
        assert_eq!(codes(&result), ["E-EXEC-003"], "{name}");
        assert_nothing_passes(&result, &outcome, VerificationState::Fail);
    }
}

/// DES-650, DS-1908: a skipped Test was not executed: no Evidence, so
/// `target_binding` reads it as not executed rather than as `PASS`.
/// @vtest.id TEST-CLI-GO-SKIPPED-TEST-RECORDS-NO-EVIDENCE
/// @vtest.covers VO-ADAPTER-GO-SKIP-IS-NOT-EXECUTED, VO-ADAPTER-GO-ACTION-MEANS-PASS-FAIL-OR-SKIPPED
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent a Go Test that calls t.Skip records no Evidence and no diagnostic, and verification reports no PASS
#[test]
fn a_skipped_go_test_records_no_evidence() {
    let fixture = calc_project(&test_file(
        &["testing"],
        &declared("TEST-E2E-GO-SKIP", "TestAdd", "\tt.Skip(\"later\")"),
    ));
    let (result, outcome) = run_and_verify(&fixture, true);
    assert!(result.evidence.is_empty());
    assert!(codes(&result).is_empty(), "{:?}", result.diagnostics);
    assert_nothing_passes(&result, &outcome, VerificationState::Fail);
}

/// DES-651, DS-1909: a run that reports no result for the Test, though `go test`
/// succeeded, is E-EXEC-002.
/// @vtest.id TEST-CLI-GO-NO-RESULT-IS-E-EXEC-002
/// @vtest.covers VO-ADAPTER-GO-NO-RESULT-IS-E-EXEC-002, VO-ADAPTER-GO-NO-OR-SEVERAL-RESULT-EVENTS
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent a Go Test whose TestMain exits 0 without running any test records no Evidence and is reported as E-EXEC-002 although go test exited 0
#[test]
fn a_run_that_reports_no_result_is_e_exec_002() {
    let fixture = calc_project(&test_file(
        &["os", "testing"],
        &format!(
            "{}\nfunc TestMain(m *testing.M) {{ os.Exit(0) }}\n",
            declared("TEST-E2E-GO-NORESULT", "TestAdd", "\t_ = Add(1, 2)")
        ),
    ));
    let (result, outcome) = run_and_verify(&fixture, true);
    assert!(result.evidence.is_empty());
    assert_eq!(codes(&result), ["E-EXEC-002"]);
    assert_nothing_passes(&result, &outcome, VerificationState::Fail);
}

/// DES-651, DS-1930, DS-1931: two results for one name, with one Test construct
/// of that name, is E-EXEC-005 after the run.
/// @vtest.id TEST-CLI-GO-TWO-RESULTS-FOR-ONE-NAME-IS-E-EXEC-005
/// @vtest.covers VO-ADAPTER-GO-NO-OR-SEVERAL-RESULT-EVENTS, VO-ADAPTER-GO-SAME-NAME-TESTS-RECORD-NO-EVIDENCE
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent a Go Test whose TestMain calls m.Run twice, so go test reports two results for its name, records no Evidence and is reported as E-EXEC-005
#[test]
fn two_results_for_one_name_are_e_exec_005_and_record_no_evidence() {
    let fixture = calc_project(&test_file(
        &["os", "testing"],
        &format!(
            "{}\nfunc TestMain(m *testing.M) {{\n\tm.Run()\n\tos.Exit(m.Run())\n}}\n",
            declared("TEST-E2E-GO-TWICE", "TestAdd", "\t_ = Add(1, 2)")
        ),
    ));
    let (result, outcome) = run_and_verify(&fixture, true);
    assert!(result.evidence.is_empty());
    assert_eq!(codes(&result), ["E-EXEC-005"]);
    assert_nothing_passes(&result, &outcome, VerificationState::Fail);
}

/// DS-1930, DS-2011: two Test constructs with one name in one package directory
/// record no Evidence for either, before `go test` is run.
/// @vtest.id TEST-CLI-GO-SAME-NAME-TESTS-ARE-E-EXEC-005
/// @vtest.covers VO-ADAPTER-GO-SAME-NAME-TESTS-RECORD-NO-EVIDENCE, VO-ADAPTER-GO-SAME-NAME-COUNT-READS-EVERY-TEST-FILE
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent a Go Test whose name is also the name of a Test in package x_test, or of one in a file a build constraint leaves out, records no Evidence and is reported as E-EXEC-005, and the same Test with a name of its own records PASS Evidence
#[test]
fn tests_that_share_a_name_record_no_evidence() {
    let declared_add = declared("TEST-E2E-GO-DUP", "TestAdd", "\t_ = Add(1, 2)");
    let external = "package app_test\n\nimport \"testing\"\n\nfunc TestAdd(t *testing.T) { t.Fatal(\"the other one\") }\n";
    let constrained = "//go:build never_built\n\npackage app\n\nimport \"testing\"\n\nfunc TestAdd(t *testing.T) { t.Fatal(\"the other one\") }\n";
    for (name, other) in [("x_test", external), ("build constraint", constrained)] {
        let fixture = project(&[
            ("go.mod", GO_MOD),
            ("app/calc.go", CALC),
            ("app/calc_test.go", &test_file(&["testing"], &declared_add)),
            ("app/other_test.go", other),
        ]);
        let (result, outcome) = run_and_verify(&fixture, true);
        assert!(result.evidence.is_empty(), "{name}: {:?}", result.evidence);
        assert_eq!(codes(&result), ["E-EXEC-005"], "{name}");
        assert_nothing_passes(&result, &outcome, VerificationState::Fail);
    }
    // The control: the second Test under a name of its own.
    let renamed = external.replace("TestAdd", "TestOther");
    let fixture = project(&[
        ("go.mod", GO_MOD),
        ("app/calc.go", CALC),
        ("app/calc_test.go", &test_file(&["testing"], &declared_add)),
        ("app/other_test.go", &renamed),
    ]);
    assert_eq!(
        only_evidence(&run_and_verify(&fixture, true).0).result,
        TestResult::Pass
    );
}

/// DS-2011, DS-2012: the count reads a `_test.go` file the discovery does not
/// read (outside the adapter's include list), and one that cannot be read
/// leaves the whole package directory without Evidence.
/// @vtest.id TEST-CLI-GO-FILE-OUTSIDE-DISCOVERY-IS-COUNTED-AND-UNREADABLE-ONE-BLOCKS
/// @vtest.covers VO-ADAPTER-GO-SAME-NAME-COUNT-READS-EVERY-TEST-FILE, VO-ADAPTER-GO-UNREADABLE-COUNTED-FILE-RECORDS-NO-EVIDENCE, VO-ADAPTER-GO-SAME-NAME-TESTS-RECORD-NO-EVIDENCE
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent with discovery limited to one test file, a same-named Test in another file of the directory is counted and gives E-EXEC-005, a file that is not UTF-8 gives no Evidence and no diagnostic, and with that file made harmless the Test records PASS Evidence
#[test]
fn a_test_file_outside_discovery_is_counted_and_an_unreadable_one_blocks_the_directory() {
    let first = test_file(
        &["testing"],
        &declared("TEST-E2E-GO-OUTSIDE", "TestAdd", "\t_ = Add(1, 2)"),
    );
    let fixture = project(&[
        ("go.mod", GO_MOD),
        ("app/calc.go", CALC),
        ("app/a_test.go", &first),
        (
            "app/b_test.go",
            "package app\n\nimport \"testing\"\n\nfunc TestAdd(t *testing.T) {}\n",
        ),
    ]);
    // Discovery reads only these two files; b_test.go is outside it.
    let limited = "version: 2\nproject:\n  name: lexical-fixture\nadapters:\n  - id: go\n    roots:\n      - '.'\n    scan:\n      include:\n        - 'app/a_test.go'\n        - 'app/calc.go'\n      assertion_macros: []\n    run:\n      coverage: go-cover\nverify:\n  full_scope:\n    - chain_integrity\n    - orphan_detection\n    - target_binding\n    - oracle_presence\n";
    fs::write(fixture.root.join(".verify/config.yaml"), limited).expect("write config");
    let (result, _) = run_and_verify(&fixture, true);
    assert!(result.evidence.is_empty());
    assert_eq!(codes(&result), ["E-EXEC-005"]);

    fs::write(
        fixture.root.join("app/b_test.go"),
        [0xff_u8, 0xfe, 0x00, 0x41],
    )
    .expect("write a file that is not text");
    let (result, outcome) = run_and_verify(&fixture, true);
    assert!(result.evidence.is_empty());
    assert!(codes(&result).is_empty(), "{:?}", result.diagnostics);
    assert_nothing_passes(&result, &outcome, VerificationState::Fail);

    fs::write(
        fixture.root.join("app/b_test.go"),
        "package app\n\nimport \"testing\"\n\nfunc TestOther(t *testing.T) {}\n",
    )
    .expect("make the file harmless");
    assert_eq!(
        only_evidence(&run_and_verify(&fixture, true).0).result,
        TestResult::Pass
    );
}

/// DES-687, DS-1901, DS-1903, DS-1932: a Test in no Go module gets W-EXEC-102
/// and no Evidence, and the Tests of the run that can be executed still are.
/// @vtest.id TEST-CLI-GO-TEST-WITHOUT-GO-MOD-IS-W-EXEC-102
/// @vtest.covers VO-ADAPTER-GO-TEST-WITHOUT-GO-MOD-IS-NOT-EXECUTED, VO-ADAPTER-GO-UNEXECUTABLE-TEST-RECORDS-NO-EVIDENCE, VO-ADAPTER-GO-UNEXECUTABLE-TEST-DOES-NOT-STOP-THE-RUN
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent a Go Test with no go.mod above its file records no Evidence and gives the warning W-EXEC-102, while a Test of the same run in a module records PASS Evidence, and the run has no error
#[test]
fn a_test_in_no_go_module_is_w_exec_102_and_the_rest_of_the_run_goes_on() {
    let fixture = project(&[
        ("a/go.mod", "module a\n\ngo 1.21\n"),
        (
            "a/calc.go",
            "package a\n\nfunc Add(x, y int) int { return x + y }\n",
        ),
        (
            "a/calc_test.go",
            &format!(
                "package a\n\nimport \"testing\"\n\n{}",
                declared("TEST-E2E-GO-MODULE", "TestInModule", "\t_ = Add(1, 2)")
                    .replace("app/calc.go::Add", "a/calc.go::Add")
            ),
        ),
        ("app/calc.go", CALC),
        (
            "app/calc_test.go",
            &test_file(
                &["testing"],
                &declared("TEST-E2E-GO-NOMOD", "TestNoModule", "\t_ = Add(1, 2)"),
            ),
        ),
    ]);
    let (result, outcome) = run_and_verify(&fixture, true);
    let record = only_evidence(&result);
    assert_eq!(record.test_id.as_str(), "TEST-E2E-GO-MODULE");
    assert_eq!(record.result, TestResult::Pass);
    assert_eq!(
        record.runner.command,
        "go test ./. -run ^TestInModule$ -count=1 -json"
    );
    assert_eq!(codes(&result), ["W-EXEC-102"]);
    assert!(!result.has_errors());
    // The Test that was run has unchecked coverage; the one that was not has no
    // Evidence at all.
    assert_eq!(
        state_of_test(
            &outcome,
            "TEST-E2E-GO-MODULE",
            VerificationCheck::TargetBinding
        ),
        (
            VerificationState::NoEvidence,
            vec![DiagnosticLabel::NotChecked]
        )
    );
    assert_eq!(
        state_of_test(
            &outcome,
            "TEST-E2E-GO-NOMOD",
            VerificationCheck::TargetBinding
        ),
        (
            VerificationState::NoEvidence,
            vec![DiagnosticLabel::NotExecuted]
        )
    );
}

/// DS-1904, DS-1901: a Test construct of a shape `go test` does not run (type
/// parameters) is W-EXEC-102 with no Evidence.
/// @vtest.id TEST-CLI-GO-TEST-OF-THE-WRONG-SHAPE-IS-W-EXEC-102
/// @vtest.covers VO-ADAPTER-GO-RUNNABLE-TEST-CONSTRUCT-SHAPE, VO-ADAPTER-GO-UNEXECUTABLE-TEST-RECORDS-NO-EVIDENCE
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent a declared Go Test construct with type parameters records no Evidence and gives the warning W-EXEC-102
#[test]
fn a_test_construct_of_a_shape_go_test_does_not_run_is_w_exec_102() {
    let generic = declared("TEST-E2E-GO-SHAPE", "TestAdd", "\t_ = Add(1, 2)")
        .replace("func TestAdd(", "func TestAdd[T any](");
    let fixture = calc_project(&test_file(&["testing"], &generic));
    let (result, outcome) = run_and_verify(&fixture, true);
    assert!(result.evidence.is_empty());
    assert_eq!(codes(&result), ["W-EXEC-102"]);
    assert_nothing_passes(&result, &outcome, VerificationState::Fail);
}

/// DES-643, DES-644, DES-684: a Test below a nested go.mod runs in that module,
/// from its root, against the package it is in.
/// @vtest.id TEST-CLI-GO-NESTED-MODULE-IS-THE-WORKING-DIRECTORY
/// @vtest.covers VO-ADAPTER-GO-MODULE-IS-THE-NEAREST-GO-MOD, VO-ADAPTER-GO-EXECUTION-PROJECT-IS-THE-MODULE, VO-ADAPTER-GO-EXECUTION-SUITE-IS-THE-PACKAGE, VO-ADAPTER-GO-TEST-ENTITY-HAS-NO-GO-FIELD
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent a Go Test in a package of a module nested in the project is scanned with that module as project and that package as suite, and is run by go test for that package in the nested module and recorded PASS
#[test]
fn a_test_in_a_nested_module_runs_in_that_module() {
    let fixture = project(&[
        ("go.mod", GO_MOD),
        ("tools/go.mod", "module tools\n\ngo 1.21\n"),
        (
            "tools/calc/calc.go",
            "package calc\n\nfunc Add(x, y int) int { return x + y }\n",
        ),
        (
            "tools/calc/calc_test.go",
            &format!(
                "package calc\n\nimport \"testing\"\n\n{}",
                declared("TEST-E2E-GO-NESTED", "TestAdd", "\t_ = Add(1, 2)")
                    .replace("app/calc.go::Add", "tools/calc/calc.go::Add")
            ),
        ),
    ]);
    let registry = vtest_ops::registry::builtin_registry().expect("builtin registry");
    let scan = scan_project(&fixture.root, &registry).expect("scan");
    // DES-646: the Go coordinates are in the neutral descriptor; the Test entity
    // has no Go field of its own.
    let entity = serde_json::to_value(&scan.tests[0]).expect("serialize the Test");
    let mut fields: Vec<&str> = entity
        .as_object()
        .expect("an object")
        .keys()
        .map(String::as_str)
        .collect();
    fields.sort_unstable();
    assert_eq!(
        fields,
        [
            "cases",
            "content_hash",
            "covers",
            "execution",
            "expect",
            "id",
            "input",
            "intent",
            "kind",
            "location",
            "related",
            "targets"
        ]
    );
    let mut descriptor: Vec<&str> = entity["execution"]
        .as_object()
        .expect("an object")
        .keys()
        .map(String::as_str)
        .collect();
    descriptor.sort_unstable();
    assert_eq!(descriptor, ["adapter", "project", "selector", "suite"]);
    let execution = &scan.tests[0].execution;
    assert_eq!(execution.project.as_deref(), Some("tools"));
    let suite = execution.suite.as_ref().expect("a suite");
    assert_eq!(
        (suite.kind.as_str(), suite.name.as_deref()),
        ("package", Some("calc"))
    );
    let (result, _) = run_and_verify(&fixture, true);
    let record = only_evidence(&result);
    assert_eq!(record.result, TestResult::Pass);
    assert_eq!(
        record.runner.command,
        "go test ./calc -run ^TestAdd$ -count=1 -json"
    );
}

/// DS-532, DS-1375, DES-371: a Test that changes a file of the project while it
/// runs changes the Execution State, so no Evidence is recorded.
/// @vtest.id TEST-CLI-GO-TEST-THAT-WRITES-INTO-THE-PROJECT-IS-E-EXEC-004
/// @vtest.covers VO-EXEC-STATE-PREPOST-STABILITY
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent a Go Test that creates a file in the project while it runs records no Evidence and is reported as E-EXEC-004, and run again with the file there and unchanged records PASS Evidence
#[test]
fn a_test_that_writes_into_the_project_is_e_exec_004_and_records_no_evidence() {
    let fixture = calc_project(&test_file(
        &["os", "testing"],
        &declared(
            "TEST-E2E-GO-WRITES",
            "TestAdd",
            "\tif err := os.WriteFile(\"../written-by-the-test.txt\", []byte(\"x\"), 0o644); err != nil {\n\t\tt.Fatal(err)\n\t}",
        ),
    ));
    let (result, outcome) = run_and_verify(&fixture, true);
    assert!(result.evidence.is_empty());
    assert_eq!(codes(&result), ["E-EXEC-004"]);
    // The Test reports a failure on an error that is not the target's, and never
    // calls the target: the result of the audit is UNKNOWN (DS-634).
    assert_nothing_passes(&result, &outcome, VerificationState::Unknown);
    // The file is there now, the second run writes the same bytes, and the
    // snapshot around the run is stable.
    assert_eq!(
        only_evidence(&run_and_verify(&fixture, true).0).result,
        TestResult::Pass
    );
}

/// DS-821, DS-1394: Evidence of a Go Test is read for `target_binding`, so a
/// change of a file of the project after the run makes it stale: `NO_EVIDENCE`
/// with `STALE`, not the `NOT_CHECKED` of the Evidence as it was; the binding
/// itself is the snapshot's (see the adapter's Execution State tests).
/// @vtest.id TEST-CLI-GO-CHANGE-AFTER-THE-RUN-IS-STALE
/// @vtest.covers VO-VERIFY-EXECUTION-STATE-HASH-MISMATCH-STALE
/// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_target_binding
/// @vtest.intent after a Go Test was run and a file of the project then changed, verification reports target_binding as NO_EVIDENCE with STALE only, not as PASS
#[test]
fn a_change_after_the_run_is_stale() {
    let fixture = calc_project(&test_file(
        &["testing"],
        &declared("TEST-E2E-GO-STALE", "TestAdd", "\t_ = Add(1, 2)"),
    ));
    let (result, _) = run_and_verify(&fixture, true);
    assert_eq!(only_evidence(&result).result, TestResult::Pass);
    fs::write(
        fixture.root.join("go.sum"),
        "example.com/x v1.0.0 h1:changed=\n",
    )
    .expect("change go.sum");
    let registry = vtest_ops::registry::builtin_registry().expect("builtin registry");
    let scan = scan_project(&fixture.root, &registry).expect("scan");
    let outcome = verify_project(&fixture.root, &scan, None, None, &registry);
    assert_eq!(
        state_of(&outcome, VerificationCheck::TargetBinding),
        (VerificationState::NoEvidence, vec![DiagnosticLabel::Stale])
    );
}

/// DS-1905: `./a...b` is a go wildcard that also matches `axb`; the Test of a
/// package that go could not select by its own path must not take the passing
/// result of another package's Test of the same name.
/// @vtest.id TEST-CLI-GO-WILDCARD-DIRECTORY-DOES-NOT-TAKE-ANOTHER-PACKAGES-RESULT
/// @vtest.covers VO-ADAPTER-GO-SELECTS-ONE-TEST-BY-EXACT-NAME, VO-ADAPTER-GO-UNEXECUTABLE-TEST-RECORDS-NO-EVIDENCE
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent a Go Test in directory a...b whose only file is left out of the build by a constraint records no Evidence and gives W-EXEC-102 although package axb has a passing Test of the same name, which records its own PASS Evidence
#[test]
fn a_directory_with_a_wildcard_in_its_name_takes_no_other_packages_result() {
    let build_excluded = format!(
        "//go:build never_built\n\n{}",
        test_file(
            &["testing"],
            &declared("TEST-E2E-GO-WILD-A", "TestAdd", "\t_ = Add(1, 2)"),
        )
    );
    let fixture = project(&[
        ("go.mod", GO_MOD),
        ("app/calc.go", CALC),
        ("a...b/a_test.go", &build_excluded),
        (
            "axb/b_test.go",
            &test_file(
                &["testing"],
                &declared("TEST-E2E-GO-WILD-B", "TestAdd", "\tt.Log(\"passes\")"),
            ),
        ),
    ]);
    let (result, outcome) = run_and_verify(&fixture, true);
    let ids: Vec<&str> = result
        .evidence
        .iter()
        .map(|record| record.test_id.as_str())
        .collect();
    assert_eq!(ids, ["TEST-E2E-GO-WILD-B"], "{:?}", result.diagnostics);
    assert_eq!(
        result.evidence[0].runner.command,
        "go test ./axb -run ^TestAdd$ -count=1 -json"
    );
    assert_eq!(codes(&result), ["W-EXEC-102"]);
    // The Test that has Evidence of its own is unchecked; the one that was not
    // run has none, and takes nothing from the other.
    assert_eq!(
        state_of_test(
            &outcome,
            "TEST-E2E-GO-WILD-B",
            VerificationCheck::TargetBinding
        ),
        (
            VerificationState::NoEvidence,
            vec![DiagnosticLabel::NotChecked]
        )
    );
    assert_eq!(
        state_of_test(
            &outcome,
            "TEST-E2E-GO-WILD-A",
            VerificationCheck::TargetBinding
        ),
        (
            VerificationState::NoEvidence,
            vec![DiagnosticLabel::NotExecuted]
        )
    );
}
