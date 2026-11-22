//! End-to-end tests of the static audit of the `go` adapter: a real Go project
//! in a fresh `.verify/`, the real `go` command, and the unmodified `vtest
//! scan`, `vtest run --all` and `vtest verify` through the CLI's composition
//! root.
//!
//! A Go Test is `PASS` as a whole only when its `target_binding` (the target ran
//! under it, measured by `go test` with coverage) and its `oracle_presence` (the
//! static audit finds a check that the target's result decides) are both `PASS`
//! (DS-605 to DS-608, DS-683). Every Test below runs and passes under `go test`
//! and reaches its target, so the only thing that can hold one back is what it
//! checks. Each wrong Test is one way a Test could be taken for a check of its
//! target's result when it is not, with the state the clauses give it; the
//! Tests at the top are the controls, which check the result and are `PASS`.
//!
//! `go` and `git` must be on `PATH`: a missing one fails the test with a message
//! instead of passing without having run anything.

mod support;

use std::fs;
use std::process::Command;

use support::go_project::{codes, outcome_of_test, project, run_and_verify};
use vtest_exec::ExecutionResult;
use vtest_model::{TestResult, VerificationCheck, VerificationState};
use vtest_verify::{NodeKind, TreeNode, VerifyOutcome};

const GO_MOD: &str = "module app\n\ngo 1.21\n";

const CALC: &str = "package app\n\nfunc Add(a, b int) int { return a + b }\n\nfunc Double(a int) int { return a * 2 }\n\ntype Calc struct{}\n\nfunc (c *Calc) Add(a, b int) int { return a + b }\n";

const HELPERS: &str = r#"package app

import (
	"testing"
)

type asserter struct{}

func (asserter) Equal(t *testing.T, want, got int) {
	t.Helper()
	if want != got {
		t.Fatalf("want %d, got %d", want, got)
	}
}

var assert = asserter{}

func check(t *testing.T, got, want int) {
	t.Helper()
	if got != want {
		t.Fatalf("got %d, want %d", got, want)
	}
}
"#;

/// The config of the project: `assert.Equal` is an assertion call (DS-1886).
const CONFIG: &str = "version: 2\nproject:\n  name: lexical-fixture\nadapters:\n  - id: go\n    roots:\n      - '.'\n    scan:\n      include:\n        - 'app'\n      assertion_macros:\n        - 'assert.Equal'\n    run:\n      coverage: go-cover\nverify:\n  full_scope:\n    - chain_integrity\n    - orphan_detection\n    - target_binding\n    - oracle_presence\n";

/// One Test of the project: its name, the target it declares (none when
/// empty), the Go of its body, the state of `oracle_presence` the clauses give
/// it, and whether it is `PASS` as a whole.
struct Case {
    name: &'static str,
    target: &'static str,
    body: &'static str,
    oracle: VerificationState,
    passes: bool,
}

const ADD: &str = "app/calc.go::Add";
const DOUBLE: &str = "app/calc.go::Double";
const CALC_ADD: &str = "app/calc.go::Calc::Add";

const CASES: &[Case] = &[
    // The controls: each checks the result of its target.
    Case {
        name: "TestCorrect",
        target: ADD,
        body: "\tif got := Add(1, 2); got != 3 {\n\t\tt.Fatalf(\"got %d, want 3\", got)\n\t}",
        oracle: VerificationState::Pass,
        passes: true,
    },
    Case {
        name: "TestTable",
        target: ADD,
        body: "\ttests := []struct {\n\t\tname string\n\t\ta, b  int\n\t\twant int\n\t}{\n\t\t{\"one\", 1, 2, 3},\n\t\t{\"two\", 2, 3, 5},\n\t}\n\tfor _, tc := range tests {\n\t\tt.Run(tc.name, func(t *testing.T) {\n\t\t\tif got := Add(tc.a, tc.b); got != tc.want {\n\t\t\t\tt.Errorf(\"Add(%d, %d) = %d, want %d\", tc.a, tc.b, got, tc.want)\n\t\t\t}\n\t\t})\n\t}",
        oracle: VerificationState::Pass,
        passes: true,
    },
    Case {
        name: "TestMethod",
        target: CALC_ADD,
        body: "\tc := &Calc{}\n\tif c.Add(1, 2) != 3 {\n\t\tt.Fatal(\"sum\")\n\t}",
        oracle: VerificationState::Pass,
        passes: true,
    },
    Case {
        name: "TestAssertionCall",
        target: ADD,
        body: "\tassert.Equal(t, 3, Add(1, 2))",
        oracle: VerificationState::Pass,
        passes: true,
    },
    // No check at all, or a check that is constant or does not hold the result.
    Case {
        name: "TestNoAssertion",
        target: ADD,
        body: "\t_ = Add(1, 2)",
        oracle: VerificationState::Fail,
        passes: false,
    },
    Case {
        name: "TestConstantCheck",
        target: ADD,
        body: "\t_ = Add(1, 2)\n\tif false {\n\t\tt.Fatal(\"never\")\n\t}",
        oracle: VerificationState::Fail,
        passes: false,
    },
    Case {
        name: "TestUnrelatedCheck",
        target: ADD,
        body: "\t_ = Add(1, 2)\n\tx := 5\n\tif x != 5 {\n\t\tt.Fatal(\"x\")\n\t}",
        oracle: VerificationState::Fail,
        passes: false,
    },
    Case {
        name: "TestSelfComparison",
        target: ADD,
        body: "\tgot := Add(1, 2)\n\tif got != got {\n\t\tt.Fatal(\"x\")\n\t}",
        oracle: VerificationState::Fail,
        passes: false,
    },
    Case {
        name: "TestSkipIsNotAFailure",
        target: ADD,
        body: "\tif Add(1, 2) != 3 {\n\t\tt.Skip(\"odd\")\n\t}",
        oracle: VerificationState::Fail,
        passes: false,
    },
    Case {
        name: "TestSubtestRedeclares",
        target: ADD,
        body: "\tgot := Add(1, 2)\n\t_ = got\n\tt.Run(\"x\", func(t *testing.T) {\n\t\tgot := 5\n\t\tif got != 5 {\n\t\t\tt.Fatal(\"x\")\n\t\t}\n\t})",
        oracle: VerificationState::Fail,
        passes: false,
    },
    // A check the audit cannot tie to the result.
    Case {
        name: "TestInAGoroutine",
        target: ADD,
        body: "\tdone := make(chan bool)\n\tgo func() {\n\t\tif Add(1, 2) != 3 {\n\t\t\tt.Error(\"sum\")\n\t\t}\n\t\tdone <- true\n\t}()\n\t<-done",
        oracle: VerificationState::Unknown,
        passes: false,
    },
    Case {
        name: "TestInADefer",
        target: ADD,
        body: "\tdefer func() {\n\t\tif Add(1, 2) != 3 {\n\t\t\tt.Error(\"sum\")\n\t\t}\n\t}()",
        oracle: VerificationState::Unknown,
        passes: false,
    },
    Case {
        name: "TestResultAliased",
        target: ADD,
        body: "\tgot := Add(1, 2)\n\tp := &got\n\t*p = 5\n\tif got != 5 {\n\t\tt.Fatal(\"x\")\n\t}",
        oracle: VerificationState::Unknown,
        passes: false,
    },
    Case {
        name: "TestResultMerged",
        target: ADD,
        body: "\tx := Add(1, 2)\n\tif x > 100 {\n\t\tx = 3\n\t}\n\tif x != 3 {\n\t\tt.Fatal(\"x\")\n\t}",
        oracle: VerificationState::Unknown,
        passes: false,
    },
    Case {
        name: "TestRecoveredPanic",
        target: ADD,
        body: "\tdefer func() {\n\t\trecover()\n\t}()\n\tif Add(1, 2) != 3 {\n\t\tpanic(\"sum\")\n\t}",
        oracle: VerificationState::Unknown,
        passes: false,
    },
    Case {
        name: "TestCompositeElement",
        target: DOUBLE,
        body: "\ts := []int{Double(2), 1}\n\tif s[1] != 1 {\n\t\tt.Fatal(\"x\")\n\t}",
        oracle: VerificationState::Unknown,
        passes: false,
    },
    Case {
        name: "TestDelegatedToAHelper",
        target: ADD,
        body: "\tcheck(t, Add(1, 2), 3)",
        oracle: VerificationState::Unknown,
        passes: false,
    },
    // The name of the target is a local function: nothing here calls the target.
    Case {
        name: "TestLocalNamedLikeTheTarget",
        target: ADD,
        body: "\tAdd := func(a, b int) int { return a + b }\n\tif Add(1, 2) != 3 {\n\t\tt.Fatal(\"sum\")\n\t}",
        oracle: VerificationState::Unknown,
        passes: false,
    },
    // A Test that declares no target: its check is found, and its target binding
    // is not checked (DS-1664), so it is not `PASS` as a whole.
    Case {
        name: "TestWithoutATarget",
        target: "",
        body: "\tif Add(1, 2) != 3 {\n\t\tt.Fatal(\"sum\")\n\t}",
        oracle: VerificationState::Pass,
        passes: false,
    },
];

fn declared(case: &Case) -> String {
    let target = if case.target.is_empty() {
        String::new()
    } else {
        format!("// @vtest.target {}\n", case.target)
    };
    format!(
        "// @vtest.id TEST-AUDIT-{}\n// @vtest.covers VO-FIXTURE-ONE\n{target}// @vtest.intent {} is checked\nfunc {}(t *testing.T) {{\n{}\n}}\n",
        case.name.to_ascii_uppercase(),
        case.name,
        case.name,
        case.body
    )
}

fn test_file() -> String {
    let items: String = CASES.iter().map(declared).collect::<Vec<_>>().join("\n");
    format!("package app\n\nimport (\n\t\"testing\"\n)\n\n{items}")
}

fn find<'a>(nodes: &'a [TreeNode], id: &str) -> Option<&'a TreeNode> {
    nodes.iter().find_map(|node| {
        if node.kind == NodeKind::Test && node.id == id {
            Some(node)
        } else {
            find(&node.children, id)
        }
    })
}

fn node_state(outcome: &VerifyOutcome, id: &str) -> VerificationState {
    find(&outcome.tree, id)
        .unwrap_or_else(|| panic!("no node of {id}"))
        .state
}

fn run_project() -> (support::Fixture, ExecutionResult, VerifyOutcome) {
    let fixture = project(&[
        ("go.mod", GO_MOD),
        ("app/calc.go", CALC),
        ("app/helpers_test.go", HELPERS),
        ("app/calc_test.go", &test_file()),
    ]);
    fs::write(fixture.root.join(".verify/config.yaml"), CONFIG).expect("write the config");
    let (result, outcome) = run_and_verify(&fixture, false);
    (fixture, result, outcome)
}

/// DS-605 to DS-608, DS-623, DS-683, DS-1884 to DS-1889, DS-1933, DS-1970,
/// DS-1973, DS-1975, DS-1976, DS-1979, DS-1982, DS-1984: a Go Test that checks
/// its target's result is `PASS` as a whole, and a Test that does not, in each of
/// the ways below, is not, with the state of its `oracle_presence` that the
/// clauses give it, although it ran, passed and reached its target.
/// @vtest.id TEST-CLI-GO-AUDIT-A-TEST-THAT-CHECKS-ITS-RESULT-IS-PASS-AND-THE-WRONG-ONES-ARE-NOT
/// @vtest.covers VO-ADAPTER-GO-AUDIT-ALL-NO-VIOLATION-IS-PASS, VO-ADAPTER-GO-AUDIT-ANY-FAIL-IS-FAIL, VO-ADAPTER-GO-AUDIT-UNKNOWN-WITHOUT-FAIL-IS-UNKNOWN, VO-ADAPTER-GO-AUDIT-ADAPTER-OWNS-DISCOVERY-AUDIT-RUNNER-AND-COVERAGE, VO-ADAPTER-GO-AUDIT-DA-003-RESULT-UNVERIFIED, VO-ADAPTER-GO-AUDIT-ONLY-COUNTED-POSITIONS-COUNT, VO-ADAPTER-GO-AUDIT-ALIASING-FORMS-MAKE-DERIVATION-UNKNOWN, VO-ADAPTER-GO-AUDIT-ASSIGNMENT-AND-MERGE-OF-DERIVATION, VO-ADAPTER-GO-AUDIT-RECOVERED-PANIC-DOES-NOT-COUNT, VO-ADAPTER-GO-AUDIT-COMPOSITE-LITERAL-WITH-DERIVED-ELEMENT, VO-ADAPTER-GO-AUDIT-ASSERTION-MACROS-BY-CALLEE-SPELLING, VO-ADAPTER-GO-AUDIT-NAMES-READ-ONLY-WHERE-CLAUSES-NAME-THEM, VO-ADAPTER-GO-AUDIT-DA-001-CONSTANT-ASSERTION, VO-ADAPTER-GO-AUDIT-DA-004-SELF-COMPARISON, VO-ADAPTER-GO-AUDIT-ONLY-FAILURE-REPORT-AND-PANIC-DIVERGE, VO-ADAPTER-GO-AUDIT-T-RUN-LITERAL-IS-BODY-CONTINUATION, VO-ADAPTER-GO-AUDIT-DELEGATION-NOT-CONFIRMED-IS-UNKNOWN
/// @vtest.target crates/vtest-adapter-go/src/static_audit.rs::compose
/// @vtest.intent in a real Go project run with go test and its coverage, a Test whose failure report is decided by the target's result, through a binding, a table with subtests, a method target or a listed assertion call, is PASS for oracle_presence and for target_binding and as a whole, and a Test that has no assertion, a constant or unrelated or self-comparing check, a skip as its only divergence or a subtest that redeclares the name is FAIL for oracle_presence, and one whose check is in a goroutine or a defer, whose result has its address taken or is merged with another value, whose panic is recovered, whose result is in a composite literal or in a helper, or that calls a local function named like the target is UNKNOWN, and none of them is PASS as a whole, nor is one that declares no target
#[test]
fn a_go_test_that_checks_its_result_is_pass_and_the_wrong_ones_are_not() {
    let (_fixture, result, outcome) = run_project();
    assert_eq!(
        result.evidence.len(),
        CASES.len(),
        "{:?}",
        result.diagnostics
    );
    assert!(codes(&result).is_empty(), "{:?}", result.diagnostics);
    assert!(
        result
            .evidence
            .iter()
            .all(|record| record.result == TestResult::Pass),
        "every Test passes under go test"
    );
    for case in CASES {
        let id = format!("TEST-AUDIT-{}", case.name.to_ascii_uppercase());
        let (oracle, labels, basis) =
            outcome_of_test(&outcome, &id, VerificationCheck::OraclePresence);
        assert_eq!(
            oracle, case.oracle,
            "{}: oracle_presence {labels:?} {basis:?}",
            case.name
        );
        assert!(labels.is_empty(), "{}: {labels:?}", case.name);
        let node = node_state(&outcome, &id);
        if case.passes {
            assert_eq!(node, VerificationState::Pass, "{}", case.name);
        } else {
            assert_ne!(node, VerificationState::Pass, "{}: {basis:?}", case.name);
        }
        // What holds the wrong Tests back is what they check: the target ran.
        let (binding, _, _) = outcome_of_test(&outcome, &id, VerificationCheck::TargetBinding);
        match case.name {
            "TestLocalNamedLikeTheTarget" => {
                assert_eq!(binding, VerificationState::Fail, "{}", case.name);
            }
            "TestWithoutATarget" => {
                assert_eq!(binding, VerificationState::NoEvidence, "{}", case.name);
            }
            _ => assert_eq!(binding, VerificationState::Pass, "{}", case.name),
        }
    }
}

/// DS-1884, DS-1887, DS-2012: the same project through the `vtest` binary: init
/// is the fixture's, then `vtest run --all` and `vtest verify`; a project with
/// only checking Tests is OK with exit code 0 and one with a Test that does not
/// check its target's result is not, with exit code 1.
/// @vtest.id TEST-CLI-GO-AUDIT-THE-BINARY-VERIFY-IS-OK-ONLY-FOR-CHECKING-TESTS
/// @vtest.covers VO-ADAPTER-GO-AUDIT-ALL-NO-VIOLATION-IS-PASS, VO-ADAPTER-GO-AUDIT-ANY-FAIL-IS-FAIL, VO-ADAPTER-GO-AUDIT-ORACLE-PRESENCE-IS-COMPOSITION-OF-FIVE-RULES
/// @vtest.target crates/vtest-ops/src/verify.rs::execute
/// @vtest.intent vtest run --all then vtest verify through the binary on a Go project whose Tests check their target's result exits 0 with ok true, and on the same project with one Test that only calls its target exits 1 with ok false
#[test]
fn the_binary_verify_is_ok_only_for_tests_that_check_their_result() {
    let checking = "// @vtest.id TEST-BIN-ADD\n// @vtest.covers VO-FIXTURE-ONE\n// @vtest.target app/calc.go::Add\n// @vtest.intent Add is checked\nfunc TestAdd(t *testing.T) {\n\tif got := Add(1, 2); got != 3 {\n\t\tt.Fatalf(\"got %d\", got)\n\t}\n}\n";
    let calling = "// @vtest.id TEST-BIN-CALL\n// @vtest.covers VO-FIXTURE-ONE\n// @vtest.target app/calc.go::Double\n// @vtest.intent Double is called\nfunc TestDouble(t *testing.T) {\n\t_ = Double(2)\n}\n";
    for (name, tests, ok) in [
        ("checking", checking.to_owned(), true),
        ("calling", format!("{checking}\n{calling}"), false),
    ] {
        let fixture = project(&[
            ("go.mod", GO_MOD),
            ("app/calc.go", CALC),
            (
                "app/calc_test.go",
                &format!("package app\n\nimport (\n\t\"testing\"\n)\n\n{tests}"),
            ),
        ]);
        let vtest = |args: &[&str]| {
            Command::new(env!("CARGO_BIN_EXE_vtest"))
                .arg("--project")
                .arg(&fixture.root)
                .args(["--format", "json"])
                .args(args)
                .output()
                .expect("run the vtest binary")
        };
        let run = vtest(&["run", "--all"]);
        assert_eq!(
            run.status.code(),
            Some(0),
            "{name}: {}",
            String::from_utf8_lossy(&run.stdout)
        );
        let verify = vtest(&["verify"]);
        let envelope: serde_json::Value =
            serde_json::from_slice(&verify.stdout).expect("a verify envelope");
        assert_eq!(envelope["ok"], ok, "{name}: {envelope}");
        assert_eq!(
            verify.status.code(),
            Some(i32::from(!ok)),
            "{name}: {envelope}"
        );
    }
}

/// SPEC-387, DS-1208, DES-408: `vtest audit static` reports the rules of a Go
/// Test, DA-001, DA-004, DA-005 and DA-006 once and DA-002 and DA-003 for each
/// declared target, from the same audit `vtest verify` reads, and DA-002 is
/// UNKNOWN for a Go target, which leaves its reach to coverage.
/// @vtest.id TEST-CLI-GO-AUDIT-AUDIT-STATIC-REPORTS-THE-RULES-OF-A-GO-TEST
/// @vtest.covers VO-ADAPTER-GO-AUDIT-DA-002-UNKNOWN-IS-NOT-UNREACHED, VO-ADAPTER-GO-AUDIT-PER-TARGET-INDIVIDUAL-APPLICATION, VO-ADAPTER-GO-AUDIT-DA-003-RESULT-UNVERIFIED, VO-ADAPTER-GO-AUDIT-DA-006-NO-VERIFICATION-SYNTAX
/// @vtest.target crates/vtest-adapter-go/src/static_audit.rs::rule_verdict
/// @vtest.intent vtest audit static --all through the binary on a Go project reports for a Test that checks its target's result DA-003 NO_VIOLATION on the target and DA-001, DA-004, DA-005 and DA-006 NO_VIOLATION, for one that only calls its target DA-003 FAIL and DA-006 FAIL, for one whose check is in a goroutine DA-003 UNKNOWN, and DA-002 UNKNOWN on every target
#[test]
fn audit_static_reports_the_rules_of_a_go_test() {
    let fixture = project(&[
        ("go.mod", GO_MOD),
        ("app/calc.go", CALC),
        ("app/helpers_test.go", HELPERS),
        ("app/calc_test.go", &test_file()),
    ]);
    fs::write(fixture.root.join(".verify/config.yaml"), CONFIG).expect("write the config");
    let output = Command::new(env!("CARGO_BIN_EXE_vtest"))
        .arg("--project")
        .arg(&fixture.root)
        .args(["--format", "json", "audit", "static", "--all"])
        .output()
        .expect("run the vtest binary");
    let envelope: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("an audit envelope");
    assert_eq!(output.status.code(), Some(0), "{envelope}");
    let tests = envelope["data"]["tests"].as_array().expect("tests");
    assert_eq!(tests.len(), CASES.len(), "{envelope}");
    let verdict = |test: &str, rule: &str, target: Option<&str>| -> String {
        let id = format!("TEST-AUDIT-{}", test.to_ascii_uppercase());
        let entry = tests
            .iter()
            .find(|entry| entry["test"] == id.as_str())
            .unwrap_or_else(|| panic!("no {id} in {envelope}"));
        entry["rules"]
            .as_array()
            .expect("rules")
            .iter()
            .find(|row| row["rule"] == rule && row["target"].as_str() == target)
            .and_then(|row| row["verdict"].as_str())
            .unwrap_or_else(|| panic!("no {rule} {target:?} of {id} in {entry}"))
            .to_owned()
    };
    for rule in ["DA-001", "DA-004", "DA-005", "DA-006"] {
        assert_eq!(verdict("TestCorrect", rule, None), "NO_VIOLATION", "{rule}");
    }
    assert_eq!(verdict("TestCorrect", "DA-003", Some(ADD)), "NO_VIOLATION");
    assert_eq!(verdict("TestCorrect", "DA-002", Some(ADD)), "UNKNOWN");
    assert_eq!(verdict("TestNoAssertion", "DA-003", Some(ADD)), "FAIL");
    assert_eq!(verdict("TestNoAssertion", "DA-006", None), "FAIL");
    assert_eq!(verdict("TestInAGoroutine", "DA-003", Some(ADD)), "UNKNOWN");
    assert_eq!(verdict("TestSelfComparison", "DA-004", None), "FAIL");
    assert_eq!(
        verdict("TestMethod", "DA-003", Some(CALC_ADD)),
        "NO_VIOLATION"
    );
}
