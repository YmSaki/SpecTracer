//! End-to-end tests of the coverage of the `go` adapter (`go-cover`): a real Go
//! project, the real `go` command, and the unmodified `vtest run` operation and
//! `vtest verify` core through the CLI's composition root.
//!
//! `target_binding` is `PASS` for a Go Test only when the declared target ran
//! under that Test (DS-683). Each test below is one way a target could be taken
//! for reached when it was not, with the project that makes it so, the state the
//! clause gives it, and a control: the same project with the one detail that
//! makes the target reached, which is `PASS`. A wrong outcome that passed would
//! be a Test taken for proof of something it did not do.
//!
//! `oracle_presence` stays `NO_EVIDENCE` (`NOT_CHECKED`) for every Go Test: the
//! adapter has no static analysis (DS-614), so the state of a Test is never
//! better than `NO_EVIDENCE`, whatever its `target_binding` is.
//!
//! `go` and `git` must be on `PATH`: a missing one fails the test with a message
//! instead of passing without having run anything.

mod support;

use std::fs;

use support::go_project::{codes, only_evidence, outcome_of_test, project, run_and_verify};
use vtest_exec::ExecutionResult;
use vtest_model::{
    DiagnosticLabel, EvidenceRecord, ExitCode, TargetCoverageResult, TestResult, VerificationCheck,
    VerificationState,
};
use vtest_verify::VerifyOutcome;

const GO_MOD: &str = "module app\n\ngo 1.21\n";

/// `Add` and `Sub` are neighbours; `Helper` runs when the package is
/// initialized, so it runs before any Test; `OnlyBaseline` is run by the
/// `TestMain` of the tests that need a run of their own.
const CALC: &str = "package app\n\nfunc Add(a, b int) int { return a + b }\n\nfunc Sub(a, b int) int { return a - b }\n\nvar initDone = Helper()\n\nfunc Helper() int { return 3 }\n\nfunc OnlyBaseline() int { return 9 }\n";

/// A declared Test function; `body` is Go.
fn declared(id: &str, name: &str, targets: &[&str], body: &str) -> String {
    let targets: String = targets
        .iter()
        .map(|target| format!("// @vtest.target {target}\n"))
        .collect();
    format!(
        "// @vtest.id {id}\n// @vtest.covers VO-FIXTURE-ONE\n{targets}// @vtest.intent {name} is checked\nfunc {name}(t *testing.T) {{\n{body}\n}}\n"
    )
}

fn test_file(imports: &[&str], items: &str) -> String {
    let imports = imports
        .iter()
        .map(|import| format!("\t\"{import}\"\n"))
        .collect::<String>();
    format!("package app\n\nimport (\n{imports})\n\n{items}")
}

/// A project of the module `app` with `CALC` and one Test file.
fn calc_project(test_source: &str) -> support::Fixture {
    project(&[
        ("go.mod", GO_MOD),
        ("app/calc.go", CALC),
        ("app/calc_test.go", test_source),
    ])
}

/// The Test `TestAdd` of the id `id` that declares `targets` and runs `body`.
fn add_test(id: &str, targets: &[&str], body: &str) -> String {
    declared(id, "TestAdd", targets, body)
}

const CALLS_ADD: &str = "\tif Add(1, 2) != 3 {\n\t\tt.Fatal(\"sum\")\n\t}";
const CALLS_SUB: &str = "\tif Sub(3, 1) != 2 {\n\t\tt.Fatal(\"difference\")\n\t}";
const CALLS_NOTHING: &str = "\tt.Log(\"calls nothing\")";

type Entry = (String, TargetCoverageResult, Option<u64>);

fn entries(record: &EvidenceRecord) -> Vec<Entry> {
    record
        .target_coverage
        .targets
        .iter()
        .map(|entry| {
            (
                entry
                    .target
                    .strip_prefix("go::")
                    .unwrap_or(&entry.target)
                    .to_owned(),
                entry.result,
                entry.count,
            )
        })
        .collect()
}

fn evidence_of<'a>(result: &'a ExecutionResult, test_id: &str) -> &'a EvidenceRecord {
    result
        .evidence
        .iter()
        .find(|record| record.test_id.as_str() == test_id)
        .unwrap_or_else(|| panic!("no Evidence of {test_id}: {:?}", result.diagnostics))
}

fn binding(outcome: &VerifyOutcome, test_id: &str) -> (VerificationState, Vec<DiagnosticLabel>) {
    let (state, labels, _) = outcome_of_test(outcome, test_id, VerificationCheck::TargetBinding);
    (state, labels)
}

const NOT_EXECUTED: DiagnosticLabel = DiagnosticLabel::NotExecuted;

/// `entries` of `record` for exactly `expected`, with `Some(n)` standing for any
/// count above 0.
fn assert_entries(record: &EvidenceRecord, expected: &[(&str, TargetCoverageResult, Option<u64>)]) {
    let actual = entries(record);
    assert_eq!(actual.len(), expected.len(), "{actual:?}");
    for ((target, result, count), (want_target, want_result, want_count)) in
        actual.iter().zip(expected)
    {
        assert_eq!(target, want_target, "{actual:?}");
        assert_eq!(result, want_result, "{target}: {actual:?}");
        match want_count {
            Some(0) => assert_eq!(*count, Some(0), "{target}"),
            Some(_) => assert!(count.is_some_and(|count| count > 0), "{target}: {count:?}"),
            None => assert_eq!(*count, None, "{target}"),
        }
    }
}

/// DS-1926, DES-663, DES-675, DS-683: a Test that calls its declared target is
/// recorded with the measured count and the target is reached, which is the one
/// way a Go Test gets a `target_binding` of `PASS`; the command is the measured
/// one and the other run's profile is a file of its own; `oracle_presence` and so
/// the state of the Test are still not `PASS`.
/// @vtest.id TEST-CLI-GO-COVERAGE-A-TEST-THAT-CALLS-ITS-TARGET-REACHES-IT
/// @vtest.covers VO-ADAPTER-GO-DEFAULT-MODE-MEASURES-COVERAGE, VO-ADAPTER-GO-COVERAGE-USES-GO-TEST-COVER, VO-ADAPTER-GO-MEASURED-RUN-ADDS-COVER-OPTIONS, VO-ADAPTER-GO-RECORDS-TARGET-COVERAGE-AS-GO-COVER, VO-ADAPTER-GO-MEASURES-THE-RUN-THAT-SELECTS-NO-TEST, VO-ADAPTER-GO-PROFILES-ARE-SEPARATE-FILES-UNDER-CACHE-COV, VO-ADAPTER-GO-TARGET-COUNT-IS-THE-BLOCKS-STARTING-IN-ITS-RANGE, VO-EXEC-TARGET-COVERAGE-COUNT-JUDGEMENT
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent a Go Test that calls the function it declares as its target is recorded with the measured command of runner kind go-test-cover, a target_coverage checked with the method go-cover and one entry of the canonical Locator with result PASS and a count above 0, two profile files with no extension under cache/cov, and verification reports target_binding PASS while oracle_presence stays NO_EVIDENCE with NOT_CHECKED and the Test as a whole NO_EVIDENCE
#[test]
fn a_test_that_calls_its_target_reaches_it() {
    let fixture = calc_project(&test_file(
        &["testing"],
        &add_test("TEST-COV-ADD", &["app/calc.go::Add"], CALLS_ADD),
    ));
    let (result, outcome) = run_and_verify(&fixture, false);
    let record = only_evidence(&result);
    assert_eq!(record.result, TestResult::Pass);
    assert_eq!(record.runner.kind, "go-test-cover");
    assert_eq!(record.runner.exit_code, 0);
    assert!(
        record.execution_state.complete,
        "{:?}",
        record.execution_state
    );
    let coverage = &record.target_coverage;
    assert!(coverage.checked);
    assert_eq!(coverage.method.as_deref(), Some("go-cover"));
    assert_eq!(coverage.result, Some(TargetCoverageResult::Pass));
    assert_entries(
        record,
        &[("app/calc.go::Add", TargetCoverageResult::Pass, Some(1))],
    );
    assert!(codes(&result).is_empty(), "{:?}", result.diagnostics);

    // DES-673: two profiles, each a file named by a record id alone.
    let cov = fixture.root.join(".verify/cache/cov");
    let mut names: Vec<String> = fs::read_dir(&cov)
        .expect("the cache directory")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    assert_eq!(names.len(), 2, "{names:?}");
    assert!(
        names
            .iter()
            .all(|name| name.len() == 26 && !name.contains('.')),
        "{names:?}"
    );
    assert!(names.contains(&record.id), "{names:?}");
    // The log keeps both runs.
    let log = fs::read_to_string(
        vtest_store::VerifyLayout::new(&fixture.root)
            .verify_dir()
            .join(&record.log_ref),
    )
    .expect("the log");
    assert!(log.contains("-run ^TestAdd$"), "{log}");
    assert!(log.contains("-run ^$ "), "{log}");

    assert_eq!(
        binding(&outcome, "TEST-COV-ADD"),
        (VerificationState::Pass, vec![])
    );
    assert_eq!(
        outcome_of_test(&outcome, "TEST-COV-ADD", VerificationCheck::OraclePresence).0,
        VerificationState::NoEvidence
    );
    assert_eq!(
        outcome_of_test(&outcome, "TEST-COV-ADD", VerificationCheck::OraclePresence).1,
        vec![DiagnosticLabel::NotChecked]
    );
    assert_eq!(
        test_node_state(&outcome, "TEST-COV-ADD"),
        VerificationState::NoEvidence
    );
}

fn test_node_state(outcome: &VerifyOutcome, test_id: &str) -> VerificationState {
    fn find(nodes: &[vtest_verify::TreeNode], test_id: &str) -> Option<VerificationState> {
        nodes.iter().find_map(|node| {
            if node.kind == vtest_verify::NodeKind::Test && node.id == test_id {
                Some(node.state)
            } else {
                find(&node.children, test_id)
            }
        })
    }
    find(&outcome.tree, test_id).unwrap_or_else(|| panic!("no node of {test_id}"))
}

/// DS-767, DS-832: a Test that does not call its target, whose neighbour in the
/// same file it calls instead, did not reach it: count 0, `FAIL`.
/// @vtest.id TEST-CLI-GO-COVERAGE-A-TEST-THAT-DOES-NOT-CALL-ITS-TARGET-FAILS
/// @vtest.covers VO-ADAPTER-GO-TARGET-COUNT-IS-THE-BLOCKS-STARTING-IN-ITS-RANGE, VO-EXEC-TARGET-COVERAGE-COUNT-JUDGEMENT
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent a Go Test that declares Add and calls only Sub, the next function of the file, or nothing, is recorded with Add at count 0 and result FAIL and verification reports target_binding FAIL with NOT_EXECUTED, while the Test that declares Sub and calls it, and the Test that declares both and calls both, are PASS, and one that declares both and calls one is FAIL
#[test]
fn a_test_that_does_not_call_its_target_does_not_reach_it() {
    let items = [
        add_test("TEST-COV-NEIGHBOUR", &["app/calc.go::Add"], CALLS_SUB),
        declared(
            "TEST-COV-NOTHING",
            "TestNothing",
            &["app/calc.go::Add"],
            CALLS_NOTHING,
        ),
        declared("TEST-COV-SUB", "TestSub", &["app/calc.go::Sub"], CALLS_SUB),
        declared(
            "TEST-COV-BOTH",
            "TestBoth",
            &["app/calc.go::Add", "app/calc.go::Sub"],
            &format!("{CALLS_ADD}\n{CALLS_SUB}"),
        ),
        declared(
            "TEST-COV-ONE-OF-TWO",
            "TestOneOfTwo",
            &["app/calc.go::Add", "app/calc.go::Sub"],
            CALLS_ADD,
        ),
    ]
    .join("\n");
    let fixture = calc_project(&test_file(&["testing"], &items));
    let (result, outcome) = run_and_verify(&fixture, false);
    assert_eq!(result.evidence.len(), 5, "{:?}", result.diagnostics);

    for test_id in ["TEST-COV-NEIGHBOUR", "TEST-COV-NOTHING"] {
        assert_entries(
            evidence_of(&result, test_id),
            &[("app/calc.go::Add", TargetCoverageResult::Fail, Some(0))],
        );
        assert_eq!(
            binding(&outcome, test_id),
            (VerificationState::Fail, vec![NOT_EXECUTED]),
            "{test_id}"
        );
    }
    assert_eq!(
        binding(&outcome, "TEST-COV-SUB"),
        (VerificationState::Pass, vec![])
    );
    assert_eq!(
        binding(&outcome, "TEST-COV-BOTH"),
        (VerificationState::Pass, vec![])
    );
    assert_entries(
        evidence_of(&result, "TEST-COV-ONE-OF-TWO"),
        &[
            ("app/calc.go::Add", TargetCoverageResult::Pass, Some(1)),
            ("app/calc.go::Sub", TargetCoverageResult::Fail, Some(0)),
        ],
    );
    assert_eq!(
        binding(&outcome, "TEST-COV-ONE-OF-TWO"),
        (VerificationState::Fail, vec![NOT_EXECUTED])
    );
}

/// DES-676: whatever runs before any Test (a package variable, `init`, a
/// `TestMain`) is not a Test's reach: the target is `UNKNOWN`, whether the Test
/// calls it too or not.
/// @vtest.id TEST-CLI-GO-COVERAGE-A-TARGET-THAT-RAN-BEFORE-ANY-TEST-IS-UNKNOWN
/// @vtest.covers VO-ADAPTER-GO-REACH-BEFORE-ANY-TEST-IS-UNKNOWN, VO-ADAPTER-GO-MEASURES-THE-RUN-THAT-SELECTS-NO-TEST
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent a function that the initialization of the package calls, and one that the TestMain calls only in the run that selects no Test, are recorded UNKNOWN with no count and verification reports target_binding UNKNOWN for a Test that declares them, whether or not the Test calls them, while a Test of the same project that calls Add is PASS
#[test]
fn a_target_that_ran_before_any_test_is_not_reached_by_the_test() {
    let items = [
        declared("TEST-COV-INIT-ONLY", "TestInitOnly", &["app/calc.go::Helper"], CALLS_NOTHING),
        declared(
            "TEST-COV-INIT-AND-CALLED",
            "TestInitAndCalled",
            &["app/calc.go::Helper"],
            "\tif Helper() != 3 {\n\t\tt.Fatal(\"helper\")\n\t}",
        ),
        declared(
            "TEST-COV-BASELINE-ONLY",
            "TestBaselineOnly",
            &["app/calc.go::OnlyBaseline"],
            CALLS_NOTHING,
        ),
        add_test("TEST-COV-CONTROL", &["app/calc.go::Add"], CALLS_ADD),
        "func TestMain(m *testing.M) {\n\tflag.Parse()\n\tif flag.Lookup(\"test.run\").Value.String() == \"^$\" {\n\t\t_ = OnlyBaseline()\n\t}\n\tos.Exit(m.Run())\n}\n".to_owned(),
    ]
    .join("\n");
    let fixture = calc_project(&test_file(&["flag", "os", "testing"], &items));
    let (result, outcome) = run_and_verify(&fixture, false);
    assert_eq!(result.evidence.len(), 4, "{:?}", result.diagnostics);
    for (test_id, target) in [
        ("TEST-COV-INIT-ONLY", "app/calc.go::Helper"),
        ("TEST-COV-INIT-AND-CALLED", "app/calc.go::Helper"),
        ("TEST-COV-BASELINE-ONLY", "app/calc.go::OnlyBaseline"),
    ] {
        assert_entries(
            evidence_of(&result, test_id),
            &[(target, TargetCoverageResult::Unknown, None)],
        );
        assert_eq!(
            binding(&outcome, test_id),
            (VerificationState::Unknown, vec![]),
            "{test_id}"
        );
    }
    assert_eq!(
        binding(&outcome, "TEST-COV-CONTROL"),
        (VerificationState::Pass, vec![])
    );
}

/// DES-679: the run that selects no Test did not end with exit code 0, left no
/// profile, or wrote into the project: its count is not read as 0. The Test that
/// calls its target, which is `PASS` with the run as it is in the control, is
/// `UNKNOWN`, or has no Evidence when the project changed under the run.
/// @vtest.id TEST-CLI-GO-COVERAGE-A-RUN-THAT-SELECTS-NO-TEST-AND-FAILS-MAKES-TARGETS-UNKNOWN
/// @vtest.covers VO-ADAPTER-GO-UNUSABLE-NO-TEST-RUN-MAKES-TARGETS-UNKNOWN, VO-EXEC-STATE-PREPOST-STABILITY
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent a Go Test that calls its target is UNKNOWN when its package's TestMain ends the run that selects no Test with exit code 1, and when it ends that run with exit code 0 before a profile is written, records no Evidence and is E-EXEC-004 when that run writes a file into the project, and is PASS with the same TestMain that does none of these
#[test]
fn a_run_that_selects_no_test_and_is_unusable_makes_the_target_unknown() {
    let test_main = |on_baseline: &str| {
        format!("\nfunc TestMain(m *testing.M) {{\n\tflag.Parse()\n\tif flag.Lookup(\"test.run\").Value.String() == \"^$\" {{\n{on_baseline}\n\t}}\n\tos.Exit(m.Run())\n}}\n")
    };
    let source = |on_baseline: &str| {
        test_file(
            &["flag", "os", "testing"],
            &format!(
                "{}{}",
                add_test("TEST-COV-UNUSABLE", &["app/calc.go::Add"], CALLS_ADD),
                test_main(on_baseline)
            ),
        )
    };
    // The control: the run that selects no Test is as good as the other.
    let (result, outcome) = run_and_verify(&calc_project(&source("\t\t_ = 0")), false);
    only_evidence(&result);
    assert_eq!(
        binding(&outcome, "TEST-COV-UNUSABLE"),
        (VerificationState::Pass, vec![])
    );

    for (name, on_baseline) in [
        ("exit code 1", "\t\tos.Exit(1)"),
        ("exit code 0 and no profile", "\t\tos.Exit(0)"),
    ] {
        let (result, outcome) = run_and_verify(&calc_project(&source(on_baseline)), false);
        let record = only_evidence(&result);
        assert_eq!(record.result, TestResult::Pass, "{name}");
        assert_entries(
            record,
            &[("app/calc.go::Add", TargetCoverageResult::Unknown, None)],
        );
        assert_eq!(
            binding(&outcome, "TEST-COV-UNUSABLE"),
            (VerificationState::Unknown, vec![]),
            "{name}"
        );
    }

    // The run that selects no Test writes into the project: the state around the
    // Test's runs is not the same (DES-371), and there is no Evidence.
    let writes = "\t\t_ = os.WriteFile(\"../written-by-the-run.txt\", []byte(\"x\"), 0o644)";
    let fixture = calc_project(&source(writes));
    let (result, outcome) = run_and_verify(&fixture, false);
    assert!(result.evidence.is_empty());
    assert_eq!(codes(&result), ["E-EXEC-004"]);
    assert_eq!(
        binding(&outcome, "TEST-COV-UNUSABLE"),
        (VerificationState::NoEvidence, vec![NOT_EXECUTED])
    );
}

/// DES-664, DES-665: two packages with a file of one name and the same text hold
/// a function of one name at one place; the Test calls one of them, and the
/// count of its package is not the count of the other.
/// @vtest.id TEST-CLI-GO-COVERAGE-THE-SAME-FILE-NAME-IN-TWO-PACKAGES-IS-TWO-TARGETS
/// @vtest.covers VO-ADAPTER-GO-PROFILE-FILE-IS-CONVERTED-TO-A-PROJECT-PATH, VO-ADAPTER-GO-TARGET-COUNT-IS-THE-BLOCKS-STARTING-IN-ITS-RANGE
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent with util.go in package a and util.go of the same text in package b that a imports, the Test that calls a's F is PASS for the target in a, and the Test that calls a's F and declares b's F as its target is recorded with count 0 and FAIL, not PASS
#[test]
fn the_same_file_name_in_two_packages_is_two_targets() {
    let util = |package: &str| format!("package {package}\n\nfunc F() int {{ return 1 }}\n");
    let test_source = |id: &str, name: &str, target: &str| {
        format!(
            "package a\n\nimport \"testing\"\n\n// @vtest.id {id}\n// @vtest.covers VO-FIXTURE-ONE\n// @vtest.target {target}\n// @vtest.intent {name} is checked\nfunc {name}(t *testing.T) {{\n\tif F() != 1 {{\n\t\tt.Fatal(\"F\")\n\t}}\n}}\n"
        )
    };
    let fixture = project(&[
        ("go.mod", GO_MOD),
        ("a/util.go", &util("a")),
        ("b/util.go", &util("b")),
        ("a/dep.go", "package a\n\nimport _ \"app/b\"\n"),
        (
            "a/util_test.go",
            &format!(
                "{}\n{}",
                test_source("TEST-COV-A", "TestA", "a/util.go::F"),
                test_source("TEST-COV-B", "TestB", "b/util.go::F")
                    .replace("package a\n\nimport \"testing\"\n\n", "")
            ),
        ),
    ]);
    let (result, outcome) = run_and_verify(&fixture, false);
    assert_eq!(result.evidence.len(), 2, "{:?}", result.diagnostics);
    assert_entries(
        evidence_of(&result, "TEST-COV-A"),
        &[("a/util.go::F", TargetCoverageResult::Pass, Some(1))],
    );
    assert_eq!(
        binding(&outcome, "TEST-COV-A"),
        (VerificationState::Pass, vec![])
    );
    assert_entries(
        evidence_of(&result, "TEST-COV-B"),
        &[("b/util.go::F", TargetCoverageResult::Fail, Some(0))],
    );
    assert_eq!(
        binding(&outcome, "TEST-COV-B"),
        (VerificationState::Fail, vec![NOT_EXECUTED])
    );
}

/// DES-663, DES-666: only the packages of the module `go test` runs in are
/// measured. A function of another module that the Test does call (a module
/// nested below it and reached with a local `replace`), and one of a package no
/// Test code links, have no block in the profile: `UNKNOWN`, not `FAIL` and not
/// `PASS`.
/// @vtest.id TEST-CLI-GO-COVERAGE-A-TARGET-OUTSIDE-THE-MEASURED-PACKAGES-IS-UNKNOWN
/// @vtest.covers VO-ADAPTER-GO-TARGET-WITH-NO-BLOCK-IS-UNKNOWN, VO-ADAPTER-GO-PROFILE-FILE-IS-CONVERTED-TO-A-PROJECT-PATH, VO-EXEC-COVERAGE-TARGET-NOT-LOCATED-IS-UNKNOWN
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent a Go Test that calls a function of a nested module whose module path is another and declares it, and one that declares a function of a package that no code of the Test links, are recorded UNKNOWN with no count and verification reports target_binding UNKNOWN, while the Test that calls Add is PASS
#[test]
fn a_target_outside_the_measured_packages_is_unknown() {
    let items = [
        declared(
            "TEST-COV-NESTED",
            "TestNested",
            &["lib/lib.go::Twice"],
            "\tif lib.Twice(2) != 4 {\n\t\tt.Fatal(\"twice\")\n\t}",
        ),
        declared(
            "TEST-COV-UNLINKED",
            "TestUnlinked",
            &["other/other.go::Other"],
            CALLS_NOTHING,
        ),
        add_test("TEST-COV-CONTROL", &["app/calc.go::Add"], CALLS_ADD),
    ]
    .join("\n");
    let fixture = project(&[
        (
            "go.mod",
            "module app\n\ngo 1.21\n\nrequire example.com/lib v0.0.0\n\nreplace example.com/lib => ./lib\n",
        ),
        ("lib/go.mod", "module example.com/lib\n\ngo 1.21\n"),
        ("lib/lib.go", "package lib\n\nfunc Twice(x int) int { return 2 * x }\n"),
        ("other/other.go", "package other\n\nfunc Other() int { return 1 }\n"),
        ("app/calc.go", CALC),
        (
            "app/calc_test.go",
            &test_file(&["example.com/lib", "testing"], &items).replace("\"example.com/lib\"", "lib \"example.com/lib\""),
        ),
    ]);
    let (result, outcome) = run_and_verify(&fixture, false);
    assert_eq!(result.evidence.len(), 3, "{:?}", result.diagnostics);
    for (test_id, target) in [
        ("TEST-COV-NESTED", "lib/lib.go::Twice"),
        ("TEST-COV-UNLINKED", "other/other.go::Other"),
    ] {
        assert_entries(
            evidence_of(&result, test_id),
            &[(target, TargetCoverageResult::Unknown, None)],
        );
        assert_eq!(
            binding(&outcome, test_id),
            (VerificationState::Unknown, vec![]),
            "{test_id}"
        );
    }
    assert_eq!(
        binding(&outcome, "TEST-COV-CONTROL"),
        (VerificationState::Pass, vec![])
    );
}

/// DS-830, DS-1394: a Test that fails after calling its target is recorded with
/// the count it measured, and is not `PASS`: the runner's failure is `FAIL`.
/// @vtest.id TEST-CLI-GO-COVERAGE-A-FAILING-TEST-THAT-CALLED-ITS-TARGET-IS-FAIL
/// @vtest.covers VO-VERIFY-EVIDENCE-RUNNER-FAIL
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent a Go Test that calls Add and then fails is recorded as FAIL Evidence whose target_coverage has Add at PASS with a count above 0, and verification reports target_binding FAIL and not PASS
#[test]
fn a_failing_test_that_called_its_target_is_fail() {
    let fixture = calc_project(&test_file(
        &["testing"],
        &add_test(
            "TEST-COV-FAILING",
            &["app/calc.go::Add"],
            "\t_ = Add(1, 2)\n\tt.Fatal(\"fails after the call\")",
        ),
    ));
    let (result, outcome) = run_and_verify(&fixture, false);
    let record = only_evidence(&result);
    assert_eq!(record.result, TestResult::Fail);
    assert_eq!(record.runner.exit_code, 1);
    assert_entries(
        record,
        &[("app/calc.go::Add", TargetCoverageResult::Pass, Some(1))],
    );
    assert_eq!(
        binding(&outcome, "TEST-COV-FAILING"),
        (VerificationState::Fail, vec![])
    );
}

/// DES-673, DS-1394: a profile left by an earlier run, in which the target ran,
/// is not read by a later run: the Test no longer calls its target, the earlier
/// profiles are still in the cache directory, and the target is `FAIL`.
/// @vtest.id TEST-CLI-GO-COVERAGE-A-PROFILE-OF-AN-EARLIER-RUN-IS-NOT-READ
/// @vtest.covers VO-ADAPTER-GO-PROFILES-ARE-SEPARATE-FILES-UNDER-CACHE-COV
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent after a run in which the Test called Add, the Test is changed to call nothing and run again with the earlier profiles still under cache/cov, the new Evidence has Add at count 0 and FAIL, four profile files exist, and verification reports target_binding FAIL
#[test]
fn a_profile_of_an_earlier_run_is_not_read_by_a_later_one() {
    let fixture = calc_project(&test_file(
        &["testing"],
        &add_test("TEST-COV-LATER", &["app/calc.go::Add"], CALLS_ADD),
    ));
    let (first, outcome) = run_and_verify(&fixture, false);
    assert_eq!(
        only_evidence(&first).target_coverage.result,
        Some(TargetCoverageResult::Pass)
    );
    assert_eq!(
        binding(&outcome, "TEST-COV-LATER"),
        (VerificationState::Pass, vec![])
    );

    fs::write(
        fixture.root.join("app/calc_test.go"),
        test_file(
            &["testing"],
            &add_test("TEST-COV-LATER", &["app/calc.go::Add"], CALLS_NOTHING),
        ),
    )
    .expect("change the Test");
    let (second, outcome) = run_and_verify(&fixture, false);
    assert_entries(
        only_evidence(&second),
        &[("app/calc.go::Add", TargetCoverageResult::Fail, Some(0))],
    );
    let profiles = fs::read_dir(fixture.root.join(".verify/cache/cov"))
        .expect("the cache directory")
        .count();
    assert_eq!(profiles, 4);
    assert_eq!(
        binding(&outcome, "TEST-COV-LATER"),
        (VerificationState::Fail, vec![NOT_EXECUTED])
    );
}

/// DS-775, DES-460: coverage measured by `go test` is the coverage of the test
/// binary's own process: a function that only a process the Test started ran has
/// no count. That is not `PASS` (the limit is `FAIL` with count 0, where DS-775
/// asks for `UNKNOWN`).
/// @vtest.id TEST-CLI-GO-COVERAGE-A-TARGET-RUN-ONLY-BY-A-SUBPROCESS-IS-NOT-PASS
/// @vtest.covers VO-EXEC-TARGET-COVERAGE-COUNT-JUDGEMENT
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent a Go Test that starts the test binary again as a child process in which Add is called, and does not call Add itself, is recorded with Add at count 0 and verification reports target_binding FAIL and not PASS
#[test]
fn a_target_run_only_by_a_subprocess_is_not_pass() {
    let items = [
        declared(
            "TEST-COV-SUBPROCESS",
            "TestSubprocess",
            &["app/calc.go::Add"],
            "\tcmd := exec.Command(os.Args[0], \"-test.run=^TestHelperProcess$\")\n\tcmd.Env = append(os.Environ(), \"APP_HELPER=1\")\n\tif out, err := cmd.CombinedOutput(); err != nil {\n\t\tt.Fatalf(\"%v: %s\", err, out)\n\t}",
        ),
        declared(
            "TEST-COV-HELPER",
            "TestHelperProcess",
            &["app/calc.go::Add"],
            "\tif os.Getenv(\"APP_HELPER\") != \"1\" {\n\t\treturn\n\t}\n\t_ = Add(1, 2)",
        ),
    ]
    .join("\n");
    let fixture = calc_project(&test_file(&["os", "os/exec", "testing"], &items));
    let (result, outcome) = run_and_verify(&fixture, false);
    assert_eq!(result.evidence.len(), 2, "{:?}", result.diagnostics);
    assert_entries(
        evidence_of(&result, "TEST-COV-SUBPROCESS"),
        &[("app/calc.go::Add", TargetCoverageResult::Fail, Some(0))],
    );
    assert_eq!(
        binding(&outcome, "TEST-COV-SUBPROCESS"),
        (VerificationState::Fail, vec![NOT_EXECUTED])
    );
}

/// DS-778, DS-1923, DS-1103: with `--fast` no coverage is measured, so the Test
/// that calls its target has no runtime proof of it: `NO_EVIDENCE` with
/// `NOT_CHECKED`, the Test that has `PASS` without `--fast`.
/// @vtest.id TEST-CLI-GO-COVERAGE-FAST-IS-NOT-CHECKED
/// @vtest.covers VO-ADAPTER-GO-FAST-MODE-HAS-NO-COVERAGE, VO-EXEC-FAST-MODE-HAS-NO-COVERAGE-NOT-CHECKED
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent the Go Test that calls its target is PASS for target_binding when run without fast and NO_EVIDENCE with NOT_CHECKED when run with fast, whose Evidence has an unchecked target_coverage and the plain command, and no profile file is written
#[test]
fn a_fast_run_has_no_runtime_proof() {
    let fixture = calc_project(&test_file(
        &["testing"],
        &add_test("TEST-COV-FAST", &["app/calc.go::Add"], CALLS_ADD),
    ));
    let (result, outcome) = run_and_verify(&fixture, true);
    let record = only_evidence(&result);
    assert!(!record.target_coverage.checked);
    assert_eq!(
        record.runner.command,
        "go test ./app -run ^TestAdd$ -count=1 -json"
    );
    assert_eq!(
        binding(&outcome, "TEST-COV-FAST"),
        (
            VerificationState::NoEvidence,
            vec![DiagnosticLabel::NotChecked]
        )
    );
    assert_eq!(profiles(&fixture), Vec::<String>::new());
    let (result, outcome) = run_and_verify(&fixture, false);
    assert!(only_evidence(&result).target_coverage.checked);
    assert_eq!(
        binding(&outcome, "TEST-COV-FAST"),
        (VerificationState::Pass, vec![])
    );
}

/// The names of the files in `directory`, none when there is no directory, and
/// without the placeholders `vtest init` leaves in the directories it makes.
fn file_names(directory: &std::path::Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| !name.starts_with('.'))
        .collect();
    names.sort();
    names
}

/// The profiles under `.verify/cache/cov`.
fn profiles(fixture: &support::Fixture) -> Vec<String> {
    file_names(&fixture.root.join(".verify/cache/cov"))
}

/// Sets the `run.coverage` of the project's one adapter to `coverage`.
fn write_config(fixture: &support::Fixture, coverage: &str) {
    let path = fixture.root.join(".verify/config.yaml");
    let config = fs::read_to_string(&path).expect("config");
    let mut changed = 0;
    let rewritten: Vec<String> = config
        .lines()
        .map(|line| {
            if line.trim_start().starts_with("coverage:") {
                changed += 1;
                format!(
                    "{}coverage: {coverage}",
                    &line[..line.len() - line.trim_start().len()]
                )
            } else {
                line.to_owned()
            }
        })
        .collect();
    assert_eq!(changed, 1, "{config}");
    fs::write(path, rewritten.join("\n") + "\n").expect("write config");
}

/// DS-1879, DS-1878: `run.coverage: off` turns the measurement off: the Test is
/// run as `--fast` runs it, and `target_binding` is not checked.
/// @vtest.id TEST-CLI-GO-COVERAGE-OFF-IS-NOT-CHECKED
/// @vtest.covers VO-ADAPTER-GO-RUN-COVERAGE-VALUES-ARE-GO-COVER-OR-OFF
/// @vtest.target crates/vtest-ops/src/run.rs::run
/// @vtest.intent with run.coverage off a Go Test run without fast records the plain go test command of runner kind go-test and an unchecked target_coverage, reports no diagnostic, writes no profile, and verification reports target_binding NO_EVIDENCE with NOT_CHECKED
#[test]
fn run_coverage_off_is_not_checked() {
    let fixture = calc_project(&test_file(
        &["testing"],
        &add_test("TEST-COV-OFF", &["app/calc.go::Add"], CALLS_ADD),
    ));
    write_config(&fixture, "off");
    let (result, outcome) = run_and_verify(&fixture, false);
    let record = only_evidence(&result);
    assert!(!record.target_coverage.checked);
    assert_eq!(record.runner.kind, "go-test");
    assert_eq!(
        record.runner.command,
        "go test ./app -run ^TestAdd$ -count=1 -json"
    );
    assert!(codes(&result).is_empty(), "{:?}", result.diagnostics);
    assert_eq!(profiles(&fixture), Vec::<String>::new());
    assert_eq!(
        binding(&outcome, "TEST-COV-OFF"),
        (
            VerificationState::NoEvidence,
            vec![DiagnosticLabel::NotChecked]
        )
    );
}

/// DS-1879, DS-1881, DS-352: a value of `run.coverage` that `go` does not define
/// (another adapter's, or a misspelling) is an invalid adapter setting,
/// `E-CONFIG-001`, which stops the operation before any Test is run: no Evidence.
/// @vtest.id TEST-CLI-GO-COVERAGE-AN-UNKNOWN-VALUE-IS-E-CONFIG-001
/// @vtest.covers VO-ADAPTER-GO-RUN-COVERAGE-VALUES-ARE-GO-COVER-OR-OFF, VO-STORE-RUN-COVERAGE-OUTSIDE-THE-ADAPTERS-VALUES-IS-E-CONFIG-001
/// @vtest.target crates/vtest-scan/src/lib.rs::scan_project_with_config
/// @vtest.intent a go adapter whose run.coverage is llvm-cov, gocover, none or empty is rejected by the scan with E-CONFIG-001 and exit code 2 through the operation both entry points use, no Evidence is written, and go-cover and off are accepted
#[test]
fn an_unknown_run_coverage_value_is_e_config_001() {
    let fixture = calc_project(&test_file(
        &["testing"],
        &add_test("TEST-COV-CONFIG", &["app/calc.go::Add"], CALLS_ADD),
    ));
    for accepted in ["off", "go-cover"] {
        write_config(&fixture, accepted);
        vtest_ops::project::load_scanned(&fixture.root)
            .unwrap_or_else(|failure| panic!("{accepted}: {:?}", failure.diagnostic));
    }
    for value in ["llvm-cov", "gocover", "none", "''", "GO-COVER"] {
        write_config(&fixture, value);
        let failure = vtest_ops::project::load_scanned(&fixture.root)
            .err()
            .unwrap_or_else(|| panic!("{value} must be rejected"));
        assert_eq!(failure.exit, ExitCode::Usage, "{value}");
        assert_eq!(failure.diagnostic.code, "E-CONFIG-001", "{value}");
        assert!(
            failure.diagnostic.message.contains("run.coverage"),
            "{value}"
        );
    }
    assert_eq!(
        file_names(&vtest_store::VerifyLayout::new(&fixture.root).evidence_dir()),
        Vec::<String>::new()
    );
}
