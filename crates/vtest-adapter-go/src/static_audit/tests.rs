//! Tests of the `go` static audit: for each way a Go Test could be taken for
//! one that checks its target's result when it does not, the Test that does it
//! and the state the clauses give it, and next to it the Test with the one
//! detail that makes the result checked, which is `PASS`. A wrong Test that
//! came out `PASS` would be a Test taken for proof of something it did not do.

use super::*;

const PATH: &str = "app/calc_test.go";
const ADD: &[&str] = &["app/calc.go::Add"];
const MAKE: &[&str] = &["app/calc.go::Make"];
const DOUBLE: &[&str] = &["app/calc.go::Double"];
const SPLIT: &[&str] = &["app/calc.go::Split"];
const RUN: &[&str] = &["app/calc.go::Run"];
const ADD_METHOD: &[&str] = &["app/calc.go::Calc::Add"];
const RESET: &[&str] = &["app/calc.go::Reset"];
const FILL: &[&str] = &["app/calc.go::Fill"];
const INC_METHOD: &[&str] = &["app/calc.go::Counter::Inc"];
const SORTED: &[&str] = &["app/calc.go::Sorted"];
const PTRS: &[&str] = &["app/calc.go::Ptrs"];
const ADD_AND_DOUBLE: &[&str] = &["app/calc.go::Add", "app/calc.go::Double"];
const NO_TARGET: &[&str] = &[];
const NO_MACROS: &[&str] = &[];

fn construct(body: &str) -> String {
    format!("func TestX(t *testing.T) {{\n{body}\n}}")
}

fn file_of(construct: &str, extra_imports: &str, package: &str) -> String {
    format!("package {package}\n\nimport (\n\t\"testing\"\n{extra_imports})\n\n{construct}\n")
}

/// The rules of one Test.
struct Audit {
    evaluation: Evaluation,
}

impl Audit {
    fn state(&self) -> &'static str {
        match compose(&self.evaluation) {
            StaticAnalysisVerdict::Pass(_) => "PASS",
            StaticAnalysisVerdict::Fail(_) => "FAIL",
            StaticAnalysisVerdict::Unknown(_) => "UNKNOWN",
        }
    }

    fn rule(&self, name: &str) -> &'static str {
        let da = match name {
            "DA-001" => self.evaluation.da_001.clone(),
            "DA-003" => fold_targets(&self.evaluation.da_003),
            "DA-004" => self.evaluation.da_004.clone(),
            "DA-005" => self.evaluation.da_005.clone(),
            "DA-006" => self.evaluation.da_006.clone(),
            other => panic!("no rule {other}"),
        };
        match da {
            Da::NoViolation => "OK",
            Da::Fail(_) => "FAIL",
            Da::Unknown(_) => "UNKNOWN",
        }
    }
}

fn audit_text(
    construct: &str,
    file_text: &str,
    targets: &[&str],
    macros: &[&str],
    path: &str,
) -> Audit {
    let macros: Vec<String> = macros.iter().map(|m| (*m).to_owned()).collect();
    let subject = Subject {
        construct,
        test_path: path,
        file_text: Some(file_text),
        specs: targets
            .iter()
            .map(|t| TargetSpec::from_locator(t))
            .collect(),
        macros: &macros,
    };
    Audit {
        evaluation: evaluate(&subject),
    }
}

fn audit(body: &str, targets: &[&str], macros: &[&str]) -> Audit {
    let construct = construct(body);
    let file = file_of(
        &construct,
        "\t\"errors\"\n\t\"reflect\"\n\t\"strings\"\n\t\"example.com/mathx\"\n",
        "app",
    );
    audit_text(&construct, &file, targets, macros, PATH)
}

struct Row {
    name: &'static str,
    body: &'static str,
    targets: &'static [&'static str],
    macros: &'static [&'static str],
    state: &'static str,
    /// The rules that are pinned: (rule, `OK`, `FAIL` or `UNKNOWN`).
    rules: &'static [(&'static str, &'static str)],
}

macro_rules! row {
    ($name:expr, $body:expr, $targets:expr, $state:expr) => {
        Row {
            name: $name,
            body: $body,
            targets: $targets,
            macros: NO_MACROS,
            state: $state,
            rules: &[],
        }
    };
    ($name:expr, $body:expr, $targets:expr, $state:expr, $rules:expr) => {
        Row {
            name: $name,
            body: $body,
            targets: $targets,
            macros: NO_MACROS,
            state: $state,
            rules: $rules,
        }
    };
}

fn check(rows: &[Row]) {
    for row in rows {
        let result = audit(row.body, row.targets, row.macros);
        assert_eq!(
            result.state(),
            row.state,
            "{}: the state of the Test\n{}\n{:#?}",
            row.name,
            row.body,
            result.evaluation
        );
        for (rule, want) in row.rules {
            assert_eq!(
                result.rule(rule),
                *want,
                "{}: {rule}\n{}\n{:#?}",
                row.name,
                row.body,
                result.evaluation
            );
        }
    }
}

/// DS-623, DS-1884, DS-1887, DS-1933, DS-1968, DS-1986, DS-606: a Test whose
/// declared target's result decides whether a failure is reported, or is an
/// argument of a listed assertion call, is the one `PASS` the audit gives. Each
/// form is a way to write that check; every other test below changes one detail
/// of one of them.
/// @vtest.id TEST-ADAPTER-GO-AUDIT-A-CHECKED-RESULT-IS-PASS
/// @vtest.covers VO-ADAPTER-GO-AUDIT-FAILURE-REPORTS-ON-TESTING-T, VO-ADAPTER-GO-AUDIT-PANIC-IS-ASSERT-EQUIVALENT, VO-ADAPTER-GO-AUDIT-DECIDING-POSITIONS-OF-A-FAILURE-REPORT, VO-ADAPTER-GO-AUDIT-T-RUN-LITERAL-IS-BODY-CONTINUATION, VO-ADAPTER-GO-AUDIT-FLOW-THROUGH-BINDINGS-CHAINS-AND-FIELDS, VO-ADAPTER-GO-AUDIT-CONVERSIONS-AND-COMPARISONS-ARE-NOT-DELEGATION, VO-ADAPTER-GO-AUDIT-ALL-NO-VIOLATION-IS-PASS, VO-ADAPTER-GO-AUDIT-ASSIGNMENT-AND-MERGE-OF-DERIVATION, VO-ADAPTER-GO-AUDIT-COMPOSITE-LITERAL-WITH-DERIVED-ELEMENT, VO-ADAPTER-GO-AUDIT-ASSERT-EQUIVALENT-IS-COMMON-TO-ALL-RULES, VO-ADAPTER-GO-AUDIT-REACH-IS-A-DERIVED-FLOW-NOT-A-LITERAL-APPEARANCE, VO-ADAPTER-GO-AUDIT-DECIDING-POSITION-REACH-DEFINITION, VO-ADAPTER-GO-AUDIT-PER-TARGET-ALL-NO-VIOLATION-IS-PASS
/// @vtest.target crates/vtest-adapter-go/src/static_audit.rs::da_003
/// @vtest.intent a Go Test whose declared target's result decides an if, a switch tag, a range expression, a loop body, a t.Run subtest, a conversion, a method chain, a comparison function or a panic, or is an argument of an assertion_macros call, gets oracle_presence PASS with every rule free of a violation, and so does a Test whose derived value passes both branches of a merge, a failure report arm that is left out of a merge, a composite value compared whole, a method target and a target through the package name of an external test package
#[test]
fn a_test_whose_target_result_decides_a_failure_is_pass() {
    let rows = [
        row!("if on the call", "if Add(1, 2) != 3 {\n\tt.Fatal(\"sum\")\n}", ADD, "PASS", &[("DA-001", "OK"), ("DA-003", "OK"), ("DA-004", "OK"), ("DA-005", "OK"), ("DA-006", "OK")]),
        row!("if with an init", "if got := Add(1, 2); got != 3 {\n\tt.Fatalf(\"got %d\", got)\n}", ADD, "PASS"),
        row!("bindings", "got := Add(1, 2)\nwant := 3\nif got != want {\n\tt.Errorf(\"got %d want %d\", got, want)\n}", ADD, "PASS"),
        row!("var binding", "var got = Add(1, 2)\nif got != 3 {\n\tt.Error(\"x\")\n}", ADD, "PASS"),
        row!("table with range", "tests := []struct{ a, b, want int }{{1, 2, 3}, {2, 3, 5}}\nfor _, tc := range tests {\n\tif got := Add(tc.a, tc.b); got != tc.want {\n\t\tt.Errorf(\"Add(%d, %d) = %d\", tc.a, tc.b, got)\n\t}\n}", ADD, "PASS"),
        row!("subtest literal", "got := Add(1, 2)\nt.Run(\"sum\", func(t *testing.T) {\n\tif got != 3 {\n\t\tt.Fatal(\"sum\")\n\t}\n})", ADD, "PASS"),
        row!("call inside the subtest literal", "t.Run(\"sum\", func(t *testing.T) {\n\tif Add(1, 2) != 3 {\n\t\tt.Fatal(\"sum\")\n\t}\n})", ADD, "PASS"),
        row!("switch tag", "switch Add(1, 2) {\ncase 3:\ndefault:\n\tt.Fatal(\"sum\")\n}", ADD, "PASS"),
        row!("tagless switch", "got := Add(1, 2)\nswitch {\ncase got != 3:\n\tt.Fatal(\"sum\")\n}", ADD, "PASS"),
        row!("range expression", "for _, v := range Split(\"a,b\") {\n\tif v == \"\" {\n\t\tt.Error(\"empty\")\n\t}\n}", SPLIT, "PASS"),
        row!("error value", "err := Run()\nif err != nil {\n\tt.Fatal(err)\n}", RUN, "PASS"),
        row!("method chain", "if Make().Value() != 3 {\n\tt.Fatal(\"v\")\n}", MAKE, "PASS"),
        row!("field", "r := Make()\nif r.Total != 3 {\n\tt.Fatal(\"v\")\n}", MAKE, "PASS"),
        row!("conversion", "if int64(Add(1, 2)) != 3 {\n\tt.Fatal(\"x\")\n}", ADD, "PASS"),
        row!("len", "if len(Split(\"a,b\")) != 2 {\n\tt.Fatal(\"x\")\n}", SPLIT, "PASS"),
        row!("comparison function", "got := Split(\"a,b\")\nif !reflect.DeepEqual(got, []string{\"a\", \"b\"}) {\n\tt.Fatalf(\"got %v\", got)\n}", SPLIT, "PASS"),
        row!("errors.Is", "err := Run()\nif !errors.Is(err, ErrX) {\n\tt.Fatal(err)\n}", RUN, "PASS"),
        row!("composite compared whole", "s := []int{Double(2), 1}\nif !reflect.DeepEqual(s, []int{4, 1}) {\n\tt.Fatal(\"no\")\n}", DOUBLE, "PASS"),
        row!("panic", "if Add(1, 2) != 3 {\n\tpanic(\"sum\")\n}", ADD, "PASS"),
        row!("method target", "c := New()\nif c.Add(1, 2) != 3 {\n\tt.Fatal(\"sum\")\n}", ADD_METHOD, "PASS"),
        row!("compound assignment keeps it", "x := Add(1, 2)\nx += 1\nif x != 4 {\n\tt.Fatal(\"x\")\n}", ADD, "PASS"),
        row!("both branches derived", "var x int\nif flag {\n\tx = Add(1, 2)\n} else {\n\tx = Add(2, 2)\n}\nif x != 3 {\n\tt.Fatal(\"x\")\n}", ADD, "PASS"),
        row!("failure arm leaves the merge", "x := Add(1, 2)\nif flag {\n\tx = 5\n\tt.Error(\"e\")\n}\nif x != 3 {\n\tt.Fatal(\"x\")\n}", ADD, "PASS"),
        row!("loop that keeps it", "x := Add(1, 2)\nfor i := 0; i < 2; i++ {\n\tif x != 3 {\n\t\tt.Fatal(\"x\")\n\t}\n}", ADD, "PASS"),
        row!("recover defer without recover", "defer func() {\n}()\nif Add(1, 2) != 3 {\n\tpanic(\"x\")\n}", ADD, "PASS"),
        row!("recover defer does not touch a failure report", "defer func() {\n\trecover()\n}()\nif Add(1, 2) != 3 {\n\tt.Fatal(\"x\")\n}", ADD, "PASS"),
        row!("exit that does not touch the value", "short := false\nif short {\n\treturn\n}\nif Add(1, 2) != 3 {\n\tt.Fatal(\"sum\")\n}", ADD, "PASS"),
        row!("exit decided by the value", "got := Add(1, 2)\nif got == 4 {\n\treturn\n}\nt.Fatal(\"not 4\")", ADD, "PASS"),
        row!("no target", "x := 3\nif x != 4 {\n\tt.Fatal(\"x\")\n}", NO_TARGET, "PASS"),
    ];
    check(&rows);
    // A listed assertion call.
    let with_macros = |body: &str, targets: &[&str], macros: &[&str]| audit(body, targets, macros);
    let a = with_macros("assert.Equal(t, 3, Add(1, 2))", ADD, &["assert.Equal"]);
    assert_eq!(
        (a.state(), a.rule("DA-001"), a.rule("DA-003")),
        ("PASS", "OK", "OK")
    );
    let a = with_macros(
        "err := Run()\nrequire.NoError(t, err)",
        RUN,
        &["require.NoError"],
    );
    assert_eq!(a.state(), "PASS");
    // The target called through the package name of an external test package.
    let construct = construct("if app.Add(1, 2) != 3 {\n\tt.Fatal(\"sum\")\n}");
    let file = file_of(&construct, "\t\"example.com/proj/app\"\n", "app_test");
    let a = audit_text(&construct, &file, ADD, NO_MACROS, PATH);
    assert_eq!(a.state(), "PASS", "{:#?}", a.evaluation);
}

/// DS-623, DS-625, DS-626, DS-1928, DS-1994, DS-607: a Test with no check, a
/// check that is constant, one whose condition does not hold the result, or no
/// body fails, and one with a failure only in a message does not pass.
/// @vtest.id TEST-ADAPTER-GO-AUDIT-NO-CHECK-OR-A-CONSTANT-CHECK-FAILS
/// @vtest.covers VO-ADAPTER-GO-AUDIT-NO-DECIDING-POSITION-IS-CONSTANT, VO-ADAPTER-GO-AUDIT-EMPTY-TEST-IS-EMPTY-FUNCTION-BODY, VO-ADAPTER-GO-AUDIT-LOGICAL-OPERATORS-ABSORB-BY-LITERAL, VO-ADAPTER-GO-AUDIT-ANY-FAIL-IS-FAIL, VO-ADAPTER-GO-AUDIT-DA-001-CONSTANT-ASSERTION, VO-ADAPTER-GO-AUDIT-DA-003-RESULT-UNVERIFIED, VO-ADAPTER-GO-AUDIT-DA-005-EMPTY-TEST, VO-ADAPTER-GO-AUDIT-DA-006-NO-VERIFICATION-SYNTAX, VO-ADAPTER-GO-AUDIT-DECIDING-POSITIONS-OF-A-FAILURE-REPORT, VO-ADAPTER-GO-AUDIT-ONLY-FAILURE-REPORT-AND-PANIC-DIVERGE, VO-ADAPTER-GO-AUDIT-ABSORBED-OPERAND-NOT-FLOWING, VO-ADAPTER-GO-AUDIT-LITERAL-FIXED-LOGIC-IS-CONSTANT, VO-ADAPTER-GO-AUDIT-CONSTANCY-BY-DECIDING-POSITION, VO-ADAPTER-GO-AUDIT-UNKNOWN-ONLY-WHEN-NEITHER-IS-SHOWN
/// @vtest.target crates/vtest-adapter-go/src/static_audit.rs::da_001
/// @vtest.intent a Go Test that calls its target and has no assertion, one whose only failure report has no deciding position or only literal ones including an operand fixed by a literal true or false, one whose condition holds no value derived from the target, one whose only exit is a skip or a return, one whose result is reassigned before the check, one with the call only in a message, and one with an empty body are FAIL, or UNKNOWN where the result is handed to a call, and none is PASS
#[test]
fn a_test_with_no_check_or_a_constant_check_fails() {
    let rows = [
        row!("no assertion", "Add(1, 2)", ADD, "FAIL", &[("DA-003", "FAIL"), ("DA-006", "FAIL")]),
        row!("a target with no result, and a check of local values only", "Reset()\nx := 5\nif x != 5 {\n\tt.Fatal(\"x\")\n}", RESET, "FAIL", &[("DA-003", "FAIL")]),
        row!("the effect is on the receiver", "c := New()\nc.Inc()\nif c.Value() != 1 {\n\tt.Fatal(\"x\")\n}", INC_METHOD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("the effect is on an argument", "buf := make([]byte, 1)\nFill(buf)\nif buf[0] != 120 {\n\tt.Fatal(\"x\")\n}", FILL, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("the effect is on an argument that nothing reads", "buf := make([]byte, 1)\nFill(buf)\nx := 5\nif x != 5 {\n\tt.Fatal(\"x\")\n}", FILL, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("the effect is read through a call", "Reset()\nif Count() != 0 {\n\tt.Fatal(\"x\")\n}", RESET, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("the effect is read through a package variable", "Reset()\nif counter != 0 {\n\tt.Fatal(\"x\")\n}", RESET, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a cleanup function may check", "Add(1, 2)\nt.Cleanup(verify)", ADD, "UNKNOWN", &[("DA-006", "UNKNOWN")]),
        row!("a log is no check", "Add(1, 2)\nt.Log(\"x\")", ADD, "FAIL", &[("DA-006", "FAIL")]),
        row!("another declared target in the same position", "if Add(1, 2) != Double(1) {\n\tt.Fatal(\"x\")\n}", ADD_AND_DOUBLE, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("discarded", "_ = Add(1, 2)", ADD, "FAIL", &[("DA-003", "FAIL")]),
        row!("unconditional failure report", "Add(1, 2)\nt.Fatal(\"always\")", ADD, "FAIL", &[("DA-001", "FAIL"), ("DA-003", "FAIL")]),
        row!("literal true", "Add(1, 2)\nif true {\n\tt.Error(\"x\")\n}", ADD, "FAIL", &[("DA-001", "FAIL")]),
        row!("literal arithmetic", "Add(1, 2)\nif 1 == 1 {\n\tt.Error(\"x\")\n}", ADD, "FAIL", &[("DA-001", "FAIL")]),
        row!("literal comparison", "Add(1, 2)\nif 1 != 2 {\n\tt.Error(\"x\")\n}", ADD, "FAIL", &[("DA-001", "FAIL")]),
        row!("absorbed by true", "if Add(1, 2) != 3 || true {\n\tt.Fatal(\"x\")\n}", ADD, "FAIL", &[("DA-001", "FAIL"), ("DA-003", "FAIL")]),
        row!("absorbed by false", "if false && Add(1, 2) != 3 {\n\tt.Fatal(\"x\")\n}", ADD, "FAIL", &[("DA-001", "FAIL")]),
        row!("a literal that does not absorb", "if Add(1, 2) != 3 || false {\n\tt.Fatal(\"x\")\n}", ADD, "PASS"),
        row!("unconditional panic", "Add(1, 2)\npanic(\"always\")", ADD, "FAIL", &[("DA-001", "FAIL")]),
        row!("unrelated condition", "Add(1, 2)\nx := 5\nif x != 5 {\n\tt.Fatal(\"x\")\n}", ADD, "FAIL", &[("DA-001", "OK"), ("DA-003", "FAIL")]),
        row!("result unused in the condition", "flag := true\nif x := Add(1, 2); flag {\n\tt.Fatal(\"x\")\n}", ADD, "FAIL", &[("DA-003", "FAIL")]),
        row!("reassigned before the check", "x := Add(1, 2)\nx = 4\nif x != 4 {\n\tt.Error(\"x\")\n}", ADD, "FAIL", &[("DA-003", "FAIL")]),
        row!("redeclared in the subtest", "got := Add(1, 2)\nt.Run(\"x\", func(t *testing.T) {\n\tgot := 5\n\tif got != 5 {\n\t\tt.Fatal(\"x\")\n\t}\n})", ADD, "FAIL", &[("DA-003", "FAIL")]),
        row!("skip is not a failure", "if Add(1, 2) != 3 {\n\tt.Skip(\"odd\")\n}", ADD, "FAIL", &[("DA-003", "FAIL"), ("DA-006", "FAIL")]),
        row!("skipf", "if Add(1, 2) != 3 {\n\tt.Skipf(\"odd %d\", 1)\n}", ADD, "FAIL", &[("DA-006", "FAIL")]),
        row!("skipnow", "if Add(1, 2) != 3 {\n\tt.SkipNow()\n}", ADD, "FAIL", &[("DA-006", "FAIL")]),
        row!("return is not a failure", "if Add(1, 2) != 3 {\n\treturn\n}", ADD, "FAIL", &[("DA-006", "FAIL")]),
        row!("os.Exit is not a failure", "if Add(1, 2) != 3 {\n\tos.Exit(1)\n}", ADD, "FAIL", &[("DA-003", "FAIL"), ("DA-006", "FAIL")]),
        row!("break out of a switch is not a failure", "switch Add(1, 2) {\ncase 3:\n\tbreak\n}", ADD, "FAIL", &[("DA-006", "FAIL")]),
        row!("the call only in the message", "flag := true\nif flag {\n\tt.Fatalf(\"%d\", Add(1, 2))\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("the call only in a log", "t.Log(Add(1, 2))", ADD, "FAIL", &[("DA-003", "UNKNOWN"), ("DA-006", "FAIL")]),
        row!("a switch clause after the failure report", "switch 1 {\ncase 1:\n\tt.Error(\"e\")\ncase Add(1, 2):\n}", ADD, "FAIL", &[("DA-001", "FAIL"), ("DA-003", "FAIL")]),
        row!("an empty body", "", ADD, "FAIL", &[("DA-005", "FAIL"), ("DA-006", "FAIL")]),
        row!("only empty statements", ";", ADD, "FAIL", &[("DA-005", "FAIL")]),
        row!("a call without an assertion and no target", "f()", NO_TARGET, "UNKNOWN", &[("DA-006", "UNKNOWN")]),
    ];
    check(&rows);
}

/// DS-1970, DS-1933, DS-1976: a call of the target or an assertion that is in a
/// go or defer statement, in a function literal that is not a t.Run subtest, in
/// a function the Test calls, or that is captured by one, is not a counted
/// position, so the result is neither shown checked nor shown unchecked.
/// @vtest.id TEST-ADAPTER-GO-AUDIT-UNCOUNTED-POSITIONS-ARE-UNKNOWN
/// @vtest.covers VO-ADAPTER-GO-AUDIT-ONLY-COUNTED-POSITIONS-COUNT, VO-ADAPTER-GO-AUDIT-T-RUN-LITERAL-IS-BODY-CONTINUATION, VO-ADAPTER-GO-AUDIT-ALIASING-FORMS-MAKE-DERIVATION-UNKNOWN, VO-ADAPTER-GO-AUDIT-DA-003-TARGET-NOT-IN-THE-BODY-IS-UNKNOWN, VO-ADAPTER-GO-AUDIT-ALL-NO-VIOLATION-IS-PASS, VO-ADAPTER-GO-AUDIT-ONLY-SYNTAX-FIXED-PATHS-ARE-DERIVED
/// @vtest.target crates/vtest-adapter-go/src/static_audit/interp.rs::Interp::eval_func_lit
/// @vtest.intent a Go Test whose target is called and checked only in a go statement, a defer statement, a function literal that is bound, called in place or passed, a t.Run call that is itself deferred or whose second argument is a function value, or in a function the Test calls, or whose derived result is read by such a literal, is UNKNOWN for DA-003 and never PASS or FAIL
#[test]
fn a_flow_through_parts_that_do_not_count_is_unknown() {
    let rows = [
        row!("go literal", "go func() {\n\tif Add(1, 2) != 3 {\n\t\tt.Error(\"x\")\n\t}\n}()", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("defer literal", "defer func() {\n\tif Add(1, 2) != 3 {\n\t\tt.Error(\"x\")\n\t}\n}()", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("bound literal", "check := func() {\n\tif Add(1, 2) != 3 {\n\t\tt.Fatal(\"x\")\n\t}\n}\ncheck()", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("literal called in place", "func() {\n\tif Add(1, 2) != 3 {\n\t\tt.Error(\"x\")\n\t}\n}()", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("literal passed on", "run(func() {\n\tif Add(1, 2) != 3 {\n\t\tt.Error(\"x\")\n\t}\n})", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("literal never called", "_ = func() {\n\tif Add(1, 2) != 3 {\n\t\tt.Error(\"x\")\n\t}\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("go statement of the target", "go Add(1, 2)\nx := 1\nif x != 2 {\n\tt.Fatal(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("defer statement of the target", "defer Add(1, 2)\nx := 1\nif x != 2 {\n\tt.Fatal(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("go of a helper", "go check(t)\nx := 1\nif x != 2 {\n\tt.Fatal(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("the result read by a go literal", "got := Add(1, 2)\ngo func() {\n\tif got != 3 {\n\t\tt.Error(\"x\")\n\t}\n}()", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("the result read by a deferred literal", "got := Add(1, 2)\ndefer func() {\n\tif got != 3 {\n\t\tt.Error(\"x\")\n\t}\n}()", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("the result assigned after a literal that reads the name", "var got int\ncheck := func() {\n\tif got != 3 {\n\t\tt.Error(\"x\")\n\t}\n}\ngot = Add(1, 2)\ncheck()", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("the result in a captured name", "got := Add(1, 2)\nf := func() int {\n\treturn got\n}\n_ = f\nif got != 3 {\n\tt.Fatal(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("the call in a helper", "checkAdd(t)", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN"), ("DA-006", "UNKNOWN")]),
        row!("the call in a helper and an unrelated check", "checkAdd(t)\nx := 1\nif x != 2 {\n\tt.Fatal(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a subtest in a go statement", "go t.Run(\"x\", func(t *testing.T) {\n\tif Add(1, 2) != 3 {\n\t\tt.Fatal(\"x\")\n\t}\n})", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a deferred subtest", "defer t.Run(\"x\", func(t *testing.T) {\n\tif Add(1, 2) != 3 {\n\t\tt.Fatal(\"x\")\n\t}\n})", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a subtest that is a function value", "Add(1, 2)\nt.Run(\"x\", checkAdd)", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN"), ("DA-006", "UNKNOWN")]),
        row!("a literal in a literal called by a subtest", "t.Run(\"x\", func(t *testing.T) {\n\tf := func() {\n\t\tif Add(1, 2) != 3 {\n\t\t\tt.Fatal(\"x\")\n\t\t}\n\t}\n\tf()\n})", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a failure report in a literal is an assertion for DA-006", "f := func() {\n\tt.Error(\"x\")\n}\nf()", NO_TARGET, "FAIL", &[("DA-001", "FAIL"), ("DA-006", "OK")]),
    ];
    check(&rows);
}

/// DS-1975, DS-1976, DS-629: a derived value that is changed, shared or handed
/// on before the check is not known to be the result any more.
/// @vtest.id TEST-ADAPTER-GO-AUDIT-A-CHANGED-OR-SHARED-VALUE-IS-UNKNOWN
/// @vtest.covers VO-ADAPTER-GO-AUDIT-ASSIGNMENT-AND-MERGE-OF-DERIVATION, VO-ADAPTER-GO-AUDIT-ALIASING-FORMS-MAKE-DERIVATION-UNKNOWN, VO-ADAPTER-GO-AUDIT-FLOW-THROUGH-BINDINGS-CHAINS-AND-FIELDS, VO-ADAPTER-GO-AUDIT-INCOMPLETE-ANALYSIS-IS-UNKNOWN, VO-ADAPTER-GO-AUDIT-ONLY-SYNTAX-FIXED-PATHS-ARE-DERIVED, VO-ADAPTER-GO-AUDIT-UNKNOWN-ONLY-WHEN-NEITHER-IS-SHOWN
/// @vtest.target crates/vtest-adapter-go/src/static_audit/interp/exec.rs::Interp::assign_place
/// @vtest.intent a Go Test whose target result is assigned to a field or an element, has its address taken, is the receiver of a method, is passed to a function or a method, is assigned to another variable, is assigned in only one branch, or is changed in a loop, and is then checked, is UNKNOWN for DA-003 and never PASS
#[test]
fn a_changed_or_shared_value_is_unknown() {
    let rows = [
        row!("a skipped subtest hands on what it assigned", "got := Add(1, 2)\nt.Run(\"r\", func(t *testing.T) {\n\tgot = 0\n\tt.Skip(\"x\")\n})\nif got != 0 {\n\tt.Error(\"e\")\n}", ADD, "FAIL", &[("DA-003", "FAIL")]),
        row!("a subtest that ends by Goexit hands on what it assigned", "got := Add(1, 2)\nt.Run(\"r\", func(t *testing.T) {\n\tgot = 0\n\truntime.Goexit()\n})\nif got != 0 {\n\tt.Error(\"e\")\n}", ADD, "FAIL", &[("DA-003", "FAIL")]),
        row!("a skipped subtest that assigns nothing", "got := Add(1, 2)\nt.Run(\"r\", func(t *testing.T) {\n\tt.Skip(\"x\")\n})\nif got != 3 {\n\tt.Error(\"e\")\n}", ADD, "PASS"),
        row!("a parallel subtest runs after the parent", "got := Add(1, 2)\nt.Run(\"c\", func(t *testing.T) {\n\tt.Parallel()\n\tif got != 3 {\n\t\tt.Error(\"e\")\n\t}\n})\ngot = 3", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a closure that changes the name, made before the result", "var got int\nreset := func() {\n\tgot = 0\n}\ngot = Add(1, 2)\nreset()\nif got != 0 {\n\tt.Error(\"e\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a pointer taken before the result", "var got int\np := &got\ngot = Add(1, 2)\n*p = 0\nif got != 0 {\n\tt.Error(\"e\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a method value of the result", "got := Make()\nreset := got.Reset\nreset()\nif got.F != 0 {\n\tt.Error(\"e\")\n}", MAKE, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a slice of the result", "s := Sorted(x)\nv := s[:]\nv[0] = 9\nif s[0] != 9 {\n\tt.Error(\"e\")\n}", SORTED, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("elements the loop variable changes", "ptrs := Ptrs()\nfor _, p := range ptrs {\n\tp.V = 5\n}\nif ptrs[0].V != 5 {\n\tt.Error(\"e\")\n}", PTRS, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("projection assignment", "r := Make()\nr.F = 5\nif r.F != 5 {\n\tt.Fatal(\"f\")\n}", MAKE, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("element assignment", "s := Make()\ns[0] = 1\nif s[0] != 1 {\n\tt.Fatal(\"x\")\n}", MAKE, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("compound projection assignment", "r := Make()\nr.F += 5\nif r.F != 5 {\n\tt.Fatal(\"f\")\n}", MAKE, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("address taken", "got := Add(1, 2)\np := &got\n*p = 5\nif got != 5 {\n\tt.Fatal(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("address of a field", "r := Make()\np := &r.F\n*p = 5\nif r.F != 5 {\n\tt.Fatal(\"x\")\n}", MAKE, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("method receiver", "got := Make()\ngot.Reset()\nif got.F != 5 {\n\tt.Fatal(\"f\")\n}", MAKE, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("passed to a function", "got := Add(1, 2)\nlog(got)\nif got != 3 {\n\tt.Fatal(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("passed to a log call", "got := Add(1, 2)\nt.Logf(\"got %d\", got)\nif got != 3 {\n\tt.Fatal(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("assigned to another variable", "got := Add(1, 2)\nsave := got\nif got != 3 {\n\tt.Fatal(\"x\")\n}\n_ = save", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("the other variable is checked", "got := Add(1, 2)\nsave := got\nif save != 3 {\n\tt.Fatal(\"x\")\n}", ADD, "PASS"),
        row!("assigned in one branch", "x := Add(1, 2)\nif flag() {\n\tx = 5\n}\nif x != 5 {\n\tt.Fatal(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("assigned in a switch clause", "x := Add(1, 2)\nswitch y {\ncase 1:\n\tx = 5\n}\nif x != 5 {\n\tt.Fatal(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("changed in a loop", "x := Add(1, 2)\nfor i := 0; i < 2; i++ {\n\tif x != 3 {\n\t\tt.Fatal(\"x\")\n\t}\n\tx = 5\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("assigned to a package variable", "global = Add(1, 2)\nif global != 3 {\n\tt.Fatal(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("sent on a channel", "ch := make(chan int, 1)\nch <- Add(1, 2)\nif <-ch != 3 {\n\tt.Fatal(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("passed to another declared target", "got := Add(1, 2)\nif Double(got) != 6 {\n\tt.Fatal(\"x\")\n}", &["app/calc.go::Add", "app/calc.go::Double"], "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a value indexed by the result", "tbl := map[int]int{3: 9}\nif tbl[Add(1, 2)] != 9 {\n\tt.Fatal(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("the target as a value", "f := Add\nif f(1, 2) != 3 {\n\tt.Fatal(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("an absorbing constant that is not a literal", "if Add(1, 2) != 3 || 1+1 == 2 {\n\tt.Fatal(\"x\")\n}", ADD, "UNKNOWN", &[("DA-001", "UNKNOWN"), ("DA-003", "UNKNOWN")]),
    ];
    check(&rows);
}

/// DS-1984: a composite literal holding the result is read only as a whole.
/// @vtest.id TEST-ADAPTER-GO-AUDIT-A-COMPOSITE-LITERAL-IS-READ-WHOLE
/// @vtest.covers VO-ADAPTER-GO-AUDIT-COMPOSITE-LITERAL-WITH-DERIVED-ELEMENT, VO-ADAPTER-GO-AUDIT-CONVERSIONS-AND-COMPARISONS-ARE-NOT-DELEGATION
/// @vtest.target crates/vtest-adapter-go/src/static_audit/interp/eval.rs::Interp::eval_composite
/// @vtest.intent a Go Test that puts the target result in a slice, an array, a map or a struct literal and then reads an element, a field, the length or a converted value, or hands the literal to an assertion call or an operator other than == and !=, is UNKNOWN for DA-003, while the same literal compared whole by DeepEqual or == is PASS
#[test]
fn a_composite_literal_with_the_result_is_read_whole() {
    let rows = [
        row!("compared with nil", "s := []int{Add(1, 2)}\nif s == nil {\n\tt.Error(\"e\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("an element", "s := []int{Double(2), 1}\nif s[1] != 1 {\n\tt.Fatal(\"x\")\n}", DOUBLE, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("the first element", "s := []int{Double(2), 1}\nif s[0] != 4 {\n\tt.Fatal(\"x\")\n}", DOUBLE, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("the length", "s := []int{Double(2), 1}\nif len(s) != 2 {\n\tt.Fatal(\"x\")\n}", DOUBLE, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a field", "p := Pair{A: Double(2), B: 1}\nif p.B != 1 {\n\tt.Fatal(\"x\")\n}", DOUBLE, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a nested literal", "p := [][]int{{Double(2)}, {1}}\nif p[1][0] != 1 {\n\tt.Fatal(\"x\")\n}", DOUBLE, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a map", "m := map[string]int{\"a\": Double(2), \"b\": 1}\nif m[\"b\"] != 1 {\n\tt.Fatal(\"x\")\n}", DOUBLE, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a conversion", "s := []int{Double(2)}\nif int64(len(s)) != 1 {\n\tt.Fatal(\"x\")\n}", DOUBLE, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a variable bound to it again", "s := []int{Double(2), 1}\nu := s\nif u[1] != 1 {\n\tt.Fatal(\"x\")\n}", DOUBLE, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("compared whole by ==", "p := Pair{A: Double(2), B: 1}\nif p != (Pair{A: 4, B: 1}) {\n\tt.Fatal(\"x\")\n}", DOUBLE, "PASS"),
        row!("compared whole by DeepEqual", "s := []int{Double(2), 1}\nif !reflect.DeepEqual(s, []int{4, 1}) {\n\tt.Fatal(\"x\")\n}", DOUBLE, "PASS"),
        row!("compared whole in a binding", "s := []int{Double(2), 1}\nok := reflect.DeepEqual(s, []int{4, 1})\nif !ok {\n\tt.Fatal(\"x\")\n}", DOUBLE, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
    ];
    check(&rows);
    let a = audit(
        "s := []int{Double(2)}\nassert.Len(t, s, 1)",
        DOUBLE,
        &["assert.Len"],
    );
    assert_eq!((a.state(), a.rule("DA-003")), ("UNKNOWN", "UNKNOWN"));
    let a = audit(
        "p := Pair{A: Double(2)}\nassert.Equal(t, Pair{A: 4}, p)",
        DOUBLE,
        &["assert.Equal"],
    );
    assert_eq!((a.state(), a.rule("DA-003")), ("UNKNOWN", "UNKNOWN"));
}

/// DS-1982, DS-1973: a `panic(...)` that a `defer` of its function body may
/// recover is no assertion, and a failure report is not touched by the `defer`.
/// @vtest.id TEST-ADAPTER-GO-AUDIT-A-RECOVERED-PANIC-IS-NO-ASSERTION
/// @vtest.covers VO-ADAPTER-GO-AUDIT-RECOVERED-PANIC-DOES-NOT-COUNT, VO-ADAPTER-GO-AUDIT-PANIC-IS-ASSERT-EQUIVALENT, VO-ADAPTER-GO-AUDIT-ONLY-FAILURE-REPORT-AND-PANIC-DIVERGE
/// @vtest.target crates/vtest-adapter-go/src/static_audit/interp.rs::has_recover_defer
/// @vtest.intent a Go Test whose only assertion is a panic in a body with a deferred function literal that calls recover, or with a deferred call of anything but a function literal, is UNKNOWN for DA-001, DA-003 and DA-006, and the panic in a t.Run literal is judged by the defers of that literal and not of the Test
#[test]
fn a_recovered_panic_is_no_assertion() {
    let rows = [
        row!("recover in a literal", "defer func() {\n\trecover()\n}()\nif Add(1, 2) != 3 {\n\tpanic(\"x\")\n}", ADD, "UNKNOWN", &[("DA-001", "UNKNOWN"), ("DA-003", "UNKNOWN"), ("DA-006", "UNKNOWN")]),
        row!("recover in a nested call", "defer func() {\n\tif r := recover(); r != nil {\n\t\t_ = r\n\t}\n}()\nif Add(1, 2) != 3 {\n\tpanic(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a deferred call that is not a literal", "defer cleanup()\nif Add(1, 2) != 3 {\n\tpanic(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a deferred method", "defer mu.Unlock()\nif Add(1, 2) != 3 {\n\tpanic(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a deferred recover call", "defer recover()\nif Add(1, 2) != 3 {\n\tpanic(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("the defer after the panic", "if Add(1, 2) != 3 {\n\tpanic(\"x\")\n}\ndefer cleanup()", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("the failure report beside it counts", "defer func() {\n\trecover()\n}()\nif Add(1, 2) != 3 {\n\tt.Fatal(\"x\")\n}", ADD, "PASS"),
        row!("a literal without recover", "defer func() {\n\t_ = 1\n}()\nif Add(1, 2) != 3 {\n\tpanic(\"x\")\n}", ADD, "PASS"),
        row!("a defer of the Test does not recover the subtest", "defer func() {\n\trecover()\n}()\nt.Run(\"x\", func(t *testing.T) {\n\tif Add(1, 2) != 3 {\n\t\tpanic(\"x\")\n\t}\n})", ADD, "PASS"),
        row!("a defer of the subtest recovers its panic", "t.Run(\"x\", func(t *testing.T) {\n\tdefer func() {\n\t\trecover()\n\t}()\n\tif Add(1, 2) != 3 {\n\t\tpanic(\"x\")\n\t}\n})", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
    ];
    check(&rows);
}

/// DS-1983, DS-1973, DS-1887: an exit before the check adds its condition to
/// what decides the check, and one the syntax does not fix leaves the check
/// undecided.
/// @vtest.id TEST-ADAPTER-GO-AUDIT-EXITS-BEFORE-THE-CHECK
/// @vtest.covers VO-ADAPTER-GO-AUDIT-EARLIER-EXITS-AND-LOOPS-DECIDE, VO-ADAPTER-GO-AUDIT-DECIDING-POSITIONS-OF-A-FAILURE-REPORT, VO-ADAPTER-GO-AUDIT-DECIDING-LOOP-IS-THE-FOR-STATEMENT, VO-ADAPTER-GO-AUDIT-ONLY-FAILURE-REPORT-AND-PANIC-DIVERGE
/// @vtest.target crates/vtest-adapter-go/src/static_audit/interp/exec.rs::Interp::record_exit
/// @vtest.intent a Go Test whose check comes after a return, break or continue inside a loop body, a goto, a labeled break or continue, a select statement, a loop body that assigns what the loop condition reads, a statement that never runs or code after a return is UNKNOWN and never FAIL or PASS, while a return outside a loop adds its condition to what decides the check
#[test]
fn an_exit_the_syntax_does_not_fix_leaves_the_check_unknown() {
    let rows = [
        row!("an exit in one branch does not decide the other", "got := Add(1, 2)\nmode := 1\nif mode == 1 {\n\tif got == 3 {\n\t\treturn\n\t}\n} else {\n\tt.Error(\"e\")\n}", ADD, "FAIL", &[("DA-003", "FAIL")]),
        row!("a break in one clause does not decide the next", "got := Add(1, 2)\nmode := 1\nswitch mode {\ncase 1:\n\tif got == 3 {\n\t\tbreak\n\t}\ncase 2:\n\tt.Error(\"e\")\n}", ADD, "FAIL", &[("DA-003", "FAIL")]),
        row!("a constant exit leaves the check dead", "got := Add(1, 2)\nif true {\n\treturn\n}\nif got != 3 {\n\tt.Error(\"e\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a loop that never runs", "got := Add(1, 2)\nfor false {\n\tif got != 3 {\n\t\tt.Error(\"e\")\n\t}\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a clause no value reaches", "got := Add(1, 2)\nswitch 1 {\ncase 2:\n\tif got != 3 {\n\t\tt.Error(\"e\")\n\t}\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("an exit that is never taken", "got := Add(1, 2)\nif false {\n\treturn\n}\nif got != 3 {\n\tt.Error(\"e\")\n}", ADD, "PASS"),
        row!("continue in a loop", "for _, v := range []int{1, 2} {\n\tif v == 1 {\n\t\tcontinue\n\t}\n\tif Add(v, 1) != v+1 {\n\t\tt.Fatal(\"x\")\n\t}\n}", ADD, "UNKNOWN", &[("DA-001", "UNKNOWN"), ("DA-003", "UNKNOWN")]),
        row!("break in a loop", "for _, v := range []int{1, 2} {\n\tif v == 1 {\n\t\tbreak\n\t}\n\tif Add(v, 1) != v+1 {\n\t\tt.Fatal(\"x\")\n\t}\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("return in a loop, check after it", "got := Add(1, 2)\nfor _, v := range xs {\n\tif v == got {\n\t\treturn\n\t}\n}\nt.Fatal(\"x\")", ADD, "UNKNOWN", &[("DA-001", "UNKNOWN"), ("DA-003", "UNKNOWN")]),
        row!("goto", "goto done\ndone:\nif Add(1, 2) != 3 {\n\tt.Fatal(\"x\")\n}", ADD, "UNKNOWN", &[("DA-001", "UNKNOWN"), ("DA-003", "UNKNOWN")]),
        row!("labeled break", "outer:\nfor i := 0; i < 2; i++ {\n\tfor j := 0; j < 2; j++ {\n\t\tif j == 1 {\n\t\t\tbreak outer\n\t\t}\n\t}\n}\nif Add(1, 2) != 3 {\n\tt.Fatal(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("labeled continue", "outer:\nfor i := 0; i < 2; i++ {\n\tfor j := 0; j < 2; j++ {\n\t\tcontinue outer\n\t}\n}\nif Add(1, 2) != 3 {\n\tt.Fatal(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("select", "got := Add(1, 2)\nselect {\ncase <-done:\n}\nif got != 3 {\n\tt.Fatal(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a loop that changes what its condition reads", "n := Add(1, 2)\nfor n > 0 {\n\tif n == 1 {\n\t\tt.Fatal(\"x\")\n\t}\n\tn--\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a range that changes what it ranges over", "xs := Split(\"a,b\")\nfor range xs {\n\txs = nil\n\tt.Error(\"x\")\n}", SPLIT, "UNKNOWN", &[("DA-001", "UNKNOWN"), ("DA-003", "UNKNOWN")]),
        row!("a branch that never runs", "if false {\n\tif Add(1, 2) != 3 {\n\t\tt.Fatal(\"x\")\n\t}\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("an else that never runs", "if true {\n} else {\n\tif Add(1, 2) != 3 {\n\t\tt.Fatal(\"x\")\n\t}\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("code after a return", "got := Add(1, 2)\nreturn\nif got != 3 {\n\tt.Fatal(\"x\")\n}", ADD, "UNKNOWN", &[("DA-001", "UNKNOWN"), ("DA-003", "UNKNOWN")]),
        row!("code after a fatal", "got := Add(1, 2)\nt.Fatal(\"x\")\nif got != 3 {\n\tt.Fatal(\"y\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("an exit decided by the value, outside a loop", "got := Add(1, 2)\nif got == 4 {\n\treturn\n}\nt.Fatal(\"not 4\")", ADD, "PASS"),
        row!("an exit decided by something else", "if skip {\n\treturn\n}\nif Add(1, 2) != 3 {\n\tt.Fatal(\"x\")\n}", ADD, "PASS"),
        row!("a break of a switch does not decide the code after the switch", "switch k {\ncase 1:\n\tif Add(1, 2) != 3 {\n\t\tbreak\n\t}\n}\nt.Fatal(\"always\")", ADD, "FAIL", &[("DA-001", "FAIL")]),
        row!("a return in a switch decides the code after it", "got := Add(1, 2)\nswitch got {\ncase 4:\n\treturn\n}\nt.Fatal(\"x\")", ADD, "PASS"),
    ];
    check(&rows);
}

/// DS-641, DS-1936, DS-1986, DS-643, DS-652: a call that is not a conversion or
/// a comparison function (DS-1986) among what decides the check, or that takes
/// the result, is a delegation target whose termination is not confirmed.
/// @vtest.id TEST-ADAPTER-GO-AUDIT-A-DELEGATION-IS-UNKNOWN
/// @vtest.covers VO-ADAPTER-GO-AUDIT-CONVERSIONS-AND-COMPARISONS-ARE-NOT-DELEGATION, VO-ADAPTER-GO-AUDIT-DELEGATION-TARGET-OF-THE-CHECK, VO-ADAPTER-GO-AUDIT-DELEGATION-NOT-CONFIRMED-IS-UNKNOWN, VO-ADAPTER-GO-AUDIT-DELEGATION-WITHOUT-A-DECLARING-TEST-IS-UNKNOWN, VO-ADAPTER-GO-AUDIT-A-CALL-OUTSIDE-THE-TEST-IS-A-DELEGATION, VO-ADAPTER-GO-AUDIT-DA-006-DELEGATION-TARGET-OF-A-BODY-WITHOUT-ASSERTIONS
/// @vtest.target crates/vtest-adapter-go/src/static_audit/interp/eval.rs::Interp::eval_call
/// @vtest.intent a Go Test that hands the target result to a helper that takes the testing T or to another function or method, or compares it with a value that a function call returns, or has no assertion and calls a function or a method, is UNKNOWN and never PASS or FAIL, while len, cap, a conversion to a built-in type and DeepEqual, errors.Is and bytes.Equal are not delegations
#[test]
fn a_delegated_check_is_unknown() {
    let rows = [
        row!(
            "a helper takes the testing T",
            "check(t, Add(1, 2), 3)",
            ADD,
            "UNKNOWN",
            &[("DA-003", "UNKNOWN"), ("DA-006", "UNKNOWN")]
        ),
        row!(
            "a helper takes the bound result",
            "got := Add(1, 2)\ncheck(t, got)",
            ADD,
            "UNKNOWN",
            &[("DA-003", "UNKNOWN"), ("DA-006", "UNKNOWN")]
        ),
        row!(
            "a method helper",
            "got := Add(1, 2)\nh.Check(t, got)",
            ADD,
            "UNKNOWN",
            &[("DA-003", "UNKNOWN")]
        ),
        row!(
            "a function in the condition takes the result",
            "got := Add(1, 2)\nif !valid(got) {\n\tt.Fatal(\"bad\")\n}",
            ADD,
            "UNKNOWN",
            &[("DA-003", "UNKNOWN")]
        ),
        row!(
            "a standard function in the condition",
            "got := Add(1, 2)\nif strings.ToUpper(got) != \"X\" {\n\tt.Fatal(\"x\")\n}",
            ADD,
            "UNKNOWN",
            &[("DA-003", "UNKNOWN")]
        ),
        row!(
            "a call in the other operand",
            "want := compute()\ngot := Add(1, 2)\nif got != want {\n\tt.Fatal(\"x\")\n}",
            ADD,
            "UNKNOWN",
            &[("DA-003", "UNKNOWN")]
        ),
        row!(
            "a call in the other operand inline",
            "if Add(1, 2) != expected() {\n\tt.Fatal(\"x\")\n}",
            ADD,
            "UNKNOWN",
            &[("DA-003", "UNKNOWN")]
        ),
        row!(
            "a call in an exit before",
            "if skip() {\n\treturn\n}\nif Add(1, 2) != 3 {\n\tt.Fatal(\"x\")\n}",
            ADD,
            "UNKNOWN",
            &[("DA-003", "UNKNOWN")]
        ),
        row!(
            "a method call on a non-derived value in the condition",
            "got := Add(1, 2)\nif got != cfg.Want() {\n\tt.Fatal(\"x\")\n}",
            ADD,
            "UNKNOWN",
            &[("DA-003", "UNKNOWN")]
        ),
        row!(
            "a helper and no assertion",
            "helper(t)",
            NO_TARGET,
            "UNKNOWN",
            &[("DA-006", "UNKNOWN")]
        ),
        row!(
            "a method value bound to the result",
            "got := Make()\nf := got.Value\nif f() != 3 {\n\tt.Fatal(\"x\")\n}",
            MAKE,
            "UNKNOWN",
            &[("DA-003", "UNKNOWN")]
        ),
        row!(
            "a method and no assertion",
            "s.Check()",
            NO_TARGET,
            "UNKNOWN",
            &[("DA-006", "UNKNOWN")]
        ),
        row!(
            "a conversion to a named type is a call",
            "got := Add(1, 2)\nif Celsius(got) != 3 {\n\tt.Fatal(\"x\")\n}",
            ADD,
            "UNKNOWN",
            &[("DA-003", "UNKNOWN")]
        ),
        row!(
            "a conversion of a slice is a call",
            "got := Add(1, 2)\nif string([]byte{byte(got)}) != \"c\" {\n\tt.Fatal(\"x\")\n}",
            ADD,
            "UNKNOWN",
            &[("DA-003", "UNKNOWN")]
        ),
        row!(
            "a conversion to a built-in type",
            "got := Add(1, 2)\nif float64(got) != 3.0 {\n\tt.Fatal(\"x\")\n}",
            ADD,
            "PASS"
        ),
        row!(
            "cap",
            "got := Make()\nif cap(got) != 3 {\n\tt.Fatal(\"x\")\n}",
            MAKE,
            "PASS"
        ),
        row!(
            "bytes.Equal",
            "got := Make()\nif !bytes.Equal(got, want) {\n\tt.Fatal(\"x\")\n}",
            MAKE,
            "PASS"
        ),
    ];
    check(&rows);
}

/// DS-1979, DS-1886, DS-1888, DS-1884: names are read where a clause names
/// them and nowhere else: a local that is named like the target, a function of
/// another package with its name, a method of another type, a spelling of an
/// assertion that is not in the config, a `panic` or a `t` that is not the
/// built-in or a `testing.T`, and a `testing` imported under another name are
/// not read as the target or as an assertion.
/// @vtest.id TEST-ADAPTER-GO-AUDIT-NAMES-ARE-READ-ONLY-WHERE-NAMED
/// @vtest.covers VO-ADAPTER-GO-AUDIT-NAMES-READ-ONLY-WHERE-CLAUSES-NAME-THEM, VO-ADAPTER-GO-AUDIT-ASSERTION-MACROS-BY-CALLEE-SPELLING, VO-ADAPTER-GO-AUDIT-ASSERTION-MACRO-ARGUMENTS-EXCLUDE-TESTING-T, VO-ADAPTER-GO-AUDIT-ASSERTION-MACROS-NAME-IS-CALLEE-NAME, VO-ADAPTER-GO-AUDIT-GO-NAME-SPELLINGS-ARE-GO-ONLY, VO-ADAPTER-GO-AUDIT-FAILURE-REPORTS-ON-TESTING-T, VO-ADAPTER-GO-AUDIT-PANIC-IS-ASSERT-EQUIVALENT, VO-ADAPTER-GO-AUDIT-DA-003-TARGET-NOT-IN-THE-BODY-IS-UNKNOWN
/// @vtest.target crates/vtest-adapter-go/src/static_audit/interp/eval.rs::Interp::classify_call
/// @vtest.intent a Go Test whose check calls a local function, a function of another package or a method of another type that is named like the declared target, calls an assertion spelled differently from the config entry, reports through a shadowed t, calls a shadowed panic, takes its T under another package name, or calls the target through a package it does not import is UNKNOWN or FAIL and never PASS, a call spelled as a config entry is an assertion with the testing T left out of its arguments, and a constant assertion call fails
#[test]
fn names_are_read_only_where_a_clause_names_them() {
    let rows = [
        row!("a local named like the target", "Add := func(a, b int) int {\n\treturn 0\n}\nif Add(1, 2) != 3 {\n\tt.Fatal(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a parameter named like the target", "run := func(Add func(int, int) int) {\n\tif Add(1, 2) != 3 {\n\t\tt.Fatal(\"x\")\n\t}\n}\n_ = run", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a named result named like the target", "run := func() (Add func(int, int) int) {\n\tif Add(1, 2) != 3 {\n\t\tt.Fatal(\"x\")\n\t}\n\treturn nil\n}\n_ = run", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a local type named like the target", "type Add int\nx := Add(3)\nif x != 3 {\n\tt.Fatal(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a function of another package", "if mathx.Add(1, 2) != 3 {\n\tt.Fatal(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a package that is not imported", "if other.Add(1, 2) != 3 {\n\tt.Fatal(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a method of another type for a function target", "c := New()\nif c.Add(1, 2) != 3 {\n\tt.Fatal(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a method target called through a package-level name", "if calc.Add(1, 2) != 3 {\n\tt.Fatal(\"x\")\n}", ADD_METHOD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("another spelling of an assertion", "got := Add(1, 2)\nassert.Equals(t, 3, got)", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN"), ("DA-006", "UNKNOWN")]),
        row!("a shadowed panic", "panic := func(s string) {}\nif Add(1, 2) != 3 {\n\tpanic(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN"), ("DA-006", "UNKNOWN")]),
        row!("a shadowed t", "{\n\tt := &fakeT{}\n\tif Add(1, 2) != 3 {\n\t\tt.Fatal(\"x\")\n\t}\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN"), ("DA-006", "UNKNOWN")]),
        row!("a failure report on a field", "if Add(1, 2) != 3 {\n\ts.t.Fatal(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN"), ("DA-006", "UNKNOWN")]),
        row!("a variable declared a testing.TB", "var tb testing.TB = t\nif Add(1, 2) != 3 {\n\ttb.Fatal(\"x\")\n}", ADD, "PASS"),
        row!("a variable declared with its type is a T", "var u *testing.T\nu = t\nif Add(1, 2) != 3 {\n\tu.Error(\"x\")\n}", ADD, "PASS"),
        row!("a copy of t is a T", "u := t\nif Add(1, 2) != 3 {\n\tu.Error(\"x\")\n}", ADD, "PASS"),
        row!("a subtest parameter is a T", "t.Run(\"x\", func(tt *testing.T) {\n\tif Add(1, 2) != 3 {\n\t\ttt.Error(\"x\")\n\t}\n})", ADD, "PASS"),
        row!("a testing.TB parameter", "run := func(tb testing.TB) {\n\tif Add(1, 2) != 3 {\n\t\ttb.Error(\"x\")\n\t}\n}\nrun(t)", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
    ];
    check(&rows);
    let listed = |body: &str, macros: &[&str]| audit(body, ADD, macros);
    // A call spelled as the config entry, whatever the variable `assert` is.
    let a = listed(
        "assert := newAssert(t)\nassert.Equal(3, Add(1, 2))",
        &["assert.Equal"],
    );
    assert_eq!(a.state(), "PASS", "{:#?}", a.evaluation);
    // Another entry of the config does not make this call an assertion.
    let a = listed("assert.Equal(t, 3, Add(1, 2))", &["require.Equal"]);
    assert_eq!(a.state(), "UNKNOWN", "{:#?}", a.evaluation);
    // The testing T is left out of the arguments an assertion call checks.
    let a = listed("Add(1, 2)\nassert.Equal(t, 3, 3)", &["assert.Equal"]);
    assert_eq!(a.rule("DA-001"), "FAIL");
    // A `testing` imported under another name is not recognized.
    let construct =
        "func TestX(t *tt.T) {\n\tif Add(1, 2) != 3 {\n\t\tt.Fatal(\"x\")\n\t}\n}".to_owned();
    let file = format!("package app\n\nimport tt \"testing\"\n\n{construct}\n");
    let a = audit_text(&construct, &file, ADD, NO_MACROS, PATH);
    assert_eq!(a.state(), "UNKNOWN", "{:#?}", a.evaluation);
    // A bare call in an external test package, and a target in another directory.
    let construct = construct_of("if Add(1, 2) != 3 {\n\tt.Fatal(\"x\")\n}");
    let file = file_of(&construct, "", "app_test");
    let a = audit_text(&construct, &file, ADD, NO_MACROS, PATH);
    assert_eq!(a.rule("DA-003"), "UNKNOWN", "{:#?}", a.evaluation);
    let file = file_of(&construct, "", "app");
    let a = audit_text(&construct, &file, &["other/calc.go::Add"], NO_MACROS, PATH);
    assert_eq!(a.rule("DA-003"), "UNKNOWN", "{:#?}", a.evaluation);
    // A file whose header was not read cannot place a bare call.
    let a = {
        let macros: Vec<String> = Vec::new();
        let subject = Subject {
            construct: &construct,
            test_path: PATH,
            file_text: None,
            specs: vec![TargetSpec::from_locator(ADD[0])],
            macros: &macros,
        };
        Audit {
            evaluation: evaluate(&subject),
        }
    };
    assert_eq!(a.rule("DA-003"), "UNKNOWN", "{:#?}", a.evaluation);
}

fn construct_of(body: &str) -> String {
    construct(body)
}

/// DS-1889, DS-624: a failure report or panic enclosed by a condition that
/// compares a value with itself with == or != fails, and a comparison of two
/// different values does not.
/// @vtest.id TEST-ADAPTER-GO-AUDIT-A-SELF-COMPARISON-FAILS
/// @vtest.covers VO-ADAPTER-GO-AUDIT-SELF-COMPARISON-IN-ENCLOSING-CONDITION, VO-ADAPTER-GO-AUDIT-DA-004-SELF-COMPARISON
/// @vtest.target crates/vtest-adapter-go/src/static_audit.rs::da_004
/// @vtest.intent a Go Test whose failure report or panic is enclosed by a condition with a == or != whose two sides are the same tokens, in an if, an else if, a nested if, a tagless switch case or a for condition, is FAIL for DA-004 even when the result reaches the condition, and a comparison of two different values or of the same name with another operator is not
#[test]
fn a_self_comparison_fails() {
    let rows = [
        row!(
            "the bound result",
            "got := Add(1, 2)\nif got != got {\n\tt.Fatal(\"x\")\n}",
            ADD,
            "FAIL",
            &[("DA-004", "FAIL"), ("DA-003", "OK")]
        ),
        row!(
            "the call twice",
            "if Add(1, 2) == Add(1, 2) {\n\tt.Error(\"same\")\n}",
            ADD,
            "FAIL",
            &[("DA-004", "FAIL")]
        ),
        row!(
            "spacing does not matter",
            "got := Add(1,2)\nif got   !=   got {\n\tt.Fatal(\"x\")\n}",
            ADD,
            "FAIL",
            &[("DA-004", "FAIL")]
        ),
        row!(
            "inside a conjunction",
            "got := Add(1, 2)\nif ok && got == got {\n\tt.Fatal(\"x\")\n}",
            ADD,
            "FAIL",
            &[("DA-004", "FAIL")]
        ),
        row!(
            "an else if",
            "got := Add(1, 2)\nif got == 3 {\n} else if got != got {\n\tt.Fatal(\"x\")\n}",
            ADD,
            "FAIL",
            &[("DA-004", "FAIL")]
        ),
        row!(
            "an outer condition",
            "got := Add(1, 2)\nif got == got {\n\tif got != 3 {\n\t\tt.Fatal(\"x\")\n\t}\n}",
            ADD,
            "FAIL",
            &[("DA-004", "FAIL")]
        ),
        row!(
            "a tagless switch case",
            "got := Add(1, 2)\nswitch {\ncase got == got:\n\tt.Fatal(\"x\")\n}",
            ADD,
            "FAIL",
            &[("DA-004", "FAIL")]
        ),
        row!(
            "a panic",
            "got := Add(1, 2)\nif got != got {\n\tpanic(\"x\")\n}",
            ADD,
            "FAIL",
            &[("DA-004", "FAIL")]
        ),
        row!(
            "two different values",
            "got := Add(1, 2)\nif got != want {\n\tt.Fatal(\"x\")\n}",
            ADD,
            "PASS"
        ),
        row!(
            "another operator",
            "got := Add(1, 2)\nif got < got {\n\tt.Fatal(\"x\")\n}",
            ADD,
            "PASS"
        ),
        row!(
            "a comparison that encloses no failure",
            "got := Add(1, 2)\nok := got == got\n_ = ok\nif got != 3 {\n\tt.Fatal(\"x\")\n}",
            ADD,
            "PASS"
        ),
    ];
    check(&rows);
}

/// DS-615, DS-1680, DS-605 to DS-608, DS-630 to DS-633: text that is not Go, a
/// target declared by a permanent SRC ID, and several targets are composed as
/// the clauses say.
/// @vtest.id TEST-ADAPTER-GO-AUDIT-UNREADABLE-TEXT-AND-SEVERAL-TARGETS
/// @vtest.covers VO-ADAPTER-GO-AUDIT-INCOMPLETE-ANALYSIS-IS-UNKNOWN, VO-ADAPTER-GO-AUDIT-ORACLE-PRESENCE-IS-COMPOSITION-OF-FIVE-RULES, VO-ADAPTER-GO-AUDIT-ALL-NO-VIOLATION-IS-PASS, VO-ADAPTER-GO-AUDIT-ANY-FAIL-IS-FAIL, VO-ADAPTER-GO-AUDIT-UNKNOWN-WITHOUT-FAIL-IS-UNKNOWN, VO-ADAPTER-GO-AUDIT-PER-TARGET-INDIVIDUAL-APPLICATION, VO-ADAPTER-GO-AUDIT-PER-TARGET-ANY-FAIL-IS-FAIL, VO-ADAPTER-GO-AUDIT-PER-TARGET-UNKNOWN-WITHOUT-FAIL-IS-UNKNOWN, VO-ADAPTER-GO-AUDIT-PER-TARGET-ALL-NO-VIOLATION-IS-PASS, VO-ADAPTER-GO-STATIC-AUDIT-IS-DETERMINISTIC
/// @vtest.target crates/vtest-adapter-go/src/static_audit.rs::compose
/// @vtest.intent a Go Test whose text does not parse is UNKNOWN for every rule, one with a target declared by a permanent SRC ID is UNKNOWN even when its check would pass, a Test with two targets of which one is checked and one is not is FAIL, with one checked and one hidden in a helper is UNKNOWN, with both checked is PASS, and repeating the audit gives the same rules and reasons
#[test]
fn unreadable_text_and_several_targets_compose_as_the_clauses_say() {
    for text in [
        "func TestX(t *testing.T) {\n\tif {\n}",
        "func TestX(t *testing.T) {\n\tx := \"open\n}",
        "func TestX(t *testing.T) {\n\tif Add(1, 2) != 3 {\n\t\tt.Fatal(\"x\")\n\t}",
        "func TestX(t *testing.T) {\n}\nfunc Other() {}",
        "not go at all",
    ] {
        let file = file_of(text, "", "app");
        let a = audit_text(text, &file, ADD, NO_MACROS, PATH);
        assert_eq!(a.state(), "UNKNOWN", "{text}");
        for rule in ["DA-001", "DA-003", "DA-004", "DA-005", "DA-006"] {
            assert_eq!(a.rule(rule), "UNKNOWN", "{text}: {rule}");
        }
    }
    let both = &["app/calc.go::Add", "app/calc.go::Double"];
    let a = audit(
        "if Add(1, 2) != 3 {\n\tt.Fatal(\"x\")\n}\nif Double(2) != 4 {\n\tt.Fatal(\"y\")\n}",
        both,
        NO_MACROS,
    );
    assert_eq!(a.state(), "PASS", "{:#?}", a.evaluation);
    let a = audit(
        "if got := Add(1, 2); got != 3 {\n\tt.Fatal(\"x\")\n}\nDouble(2)",
        both,
        NO_MACROS,
    );
    assert_eq!(
        (a.state(), a.rule("DA-003")),
        ("FAIL", "FAIL"),
        "{:#?}",
        a.evaluation
    );
    let a = audit(
        "if got := Add(1, 2); got != 3 {\n\tt.Fatal(\"x\")\n}\ncheckDouble(t)",
        both,
        NO_MACROS,
    );
    assert_eq!(
        (a.state(), a.rule("DA-003")),
        ("UNKNOWN", "UNKNOWN"),
        "{:#?}",
        a.evaluation
    );
    // A FAIL for one target decides before another target's UNKNOWN.
    let a = audit(
        "x := 1\nif x != 2 {\n\tt.Fatal(\"x\")\n}\nDouble(2)\ncheckAdd(t)",
        both,
        NO_MACROS,
    );
    assert_eq!(
        (a.state(), a.rule("DA-003")),
        ("FAIL", "FAIL"),
        "{:#?}",
        a.evaluation
    );
    // With no assertion at all, a call the Test makes may delegate the check.
    let a = audit("Double(2)\ncheckAdd(t)", both, NO_MACROS);
    assert_eq!(a.rule("DA-003"), "UNKNOWN", "{:#?}", a.evaluation);

    // A permanent SRC ID has no call name.
    let construct = construct("if Add(1, 2) != 3 {\n\tt.Fatal(\"x\")\n}");
    let file = file_of(&construct, "", "app");
    let macros: Vec<String> = Vec::new();
    let subject = Subject {
        construct: &construct,
        test_path: PATH,
        file_text: Some(&file),
        specs: vec![None],
        macros: &macros,
    };
    let a = Audit {
        evaluation: evaluate(&subject),
    };
    assert_eq!(a.rule("DA-003"), "UNKNOWN");
    assert_eq!(a.state(), "UNKNOWN");

    // The same input gives the same reasons.
    let first = format!(
        "{:?}",
        audit(
            "got := Add(1, 2)\nif got != 3 {\n\tt.Fatal(\"x\")\n}",
            ADD,
            NO_MACROS
        )
        .evaluation
    );
    let second = format!(
        "{:?}",
        audit(
            "got := Add(1, 2)\nif got != 3 {\n\tt.Fatal(\"x\")\n}",
            ADD,
            NO_MACROS
        )
        .evaluation
    );
    assert_eq!(first, second);
}

/// DS-615, DS-1680: a Test nested deeper than the reader follows, and one whose
/// nested loops make the walk exceed its budget, are UNKNOWN for every rule and
/// never PASS, and neither overflows the stack or runs without end.
/// @vtest.id TEST-ADAPTER-GO-AUDIT-A-TEST-BEYOND-THE-READER-OR-THE-WALK-IS-UNKNOWN
/// @vtest.covers VO-ADAPTER-GO-AUDIT-INCOMPLETE-ANALYSIS-IS-UNKNOWN, VO-ADAPTER-GO-AUDIT-UNKNOWN-ONLY-WHEN-NEITHER-IS-SHOWN
/// @vtest.target crates/vtest-adapter-go/src/static_audit.rs::evaluate
/// @vtest.intent a Go Test with thousands of nested parentheses, or with nested loops that need more statements than the walk reads, or a condition that is a function literal nested as deep as the reader goes is UNKNOWN for every rule, where the same check with a shallow nesting is PASS
#[test]
fn a_test_beyond_the_reader_or_the_walk_is_unknown() {
    let shallow = audit(
        "if Add(1, 2) != (((3))) {\n\tt.Fatal(\"x\")\n}",
        ADD,
        NO_MACROS,
    );
    assert_eq!(shallow.state(), "PASS");
    let deep = format!("{}3{}", "(".repeat(5000), ")".repeat(5000));
    let a = audit(
        &format!("if Add(1, 2) != {deep} {{\n\tt.Fatal(\"x\")\n}}"),
        ADD,
        NO_MACROS,
    );
    assert_eq!(a.state(), "UNKNOWN", "{:#?}", a.evaluation);
    for rule in ["DA-001", "DA-003", "DA-006"] {
        assert_eq!(a.rule(rule), "UNKNOWN", "{rule}");
    }
    let mut body = String::from("got := Add(1, 2)\n");
    for i in 0..24 {
        body.push_str(&format!("for i{i} := 0; i{i} < 2; i{i}++ {{\n"));
    }
    body.push_str("if got != 3 {\n\tt.Fatal(\"x\")\n}\n");
    for _ in 0..24 {
        body.push_str("}\n");
    }
    let a = audit(&body, ADD, NO_MACROS);
    assert_eq!(a.state(), "UNKNOWN", "{:#?}", a.evaluation);
    for rule in ["DA-001", "DA-003", "DA-006"] {
        assert_eq!(a.rule(rule), "UNKNOWN", "{rule}");
    }
    // A few loops are walked to the end.
    let a = audit(
        "got := Add(1, 2)\nfor i := 0; i < 2; i++ {\n\tfor j := 0; j < 2; j++ {\n\t\tif got != 3 {\n\t\t\tt.Fatal(\"x\")\n\t\t}\n\t}\n}",
        ADD,
        NO_MACROS,
    );
    assert_eq!(a.state(), "PASS", "{:#?}", a.evaluation);
}

/// DS-1887, DS-1928: a failure report in a clause is decided by the case
/// expressions of the clauses up to its own and the tag, and by all the case
/// expressions when a default is among them.
/// @vtest.id TEST-ADAPTER-GO-AUDIT-SWITCH-CLAUSES-DECIDE-WHAT-THEY-SEE
/// @vtest.covers VO-ADAPTER-GO-AUDIT-DECIDING-POSITIONS-OF-A-FAILURE-REPORT, VO-ADAPTER-GO-AUDIT-NO-DECIDING-POSITION-IS-CONSTANT
/// @vtest.target crates/vtest-adapter-go/src/static_audit/interp/exec.rs::Interp::exec_switch
/// @vtest.intent in a Go switch a failure report in a clause is decided by the tag and the case expressions of the clauses before it and its own, the case expressions after it do not decide it, and a default clause is decided by all of them, with a fallthrough keeping the clause it leaves
#[test]
fn a_switch_clause_is_decided_by_the_clauses_up_to_it() {
    let rows = [
        row!("a later case does not decide an earlier clause", "switch 1 {\ncase 1:\n\tt.Error(\"e\")\ncase Add(1, 2):\n}", ADD, "FAIL", &[("DA-003", "FAIL")]),
        row!("the case of the clause itself decides", "switch {\ncase Add(1, 2) != 3:\n\tt.Error(\"e\")\n}", ADD, "PASS"),
        row!("an earlier case decides a later clause", "switch {\ncase Add(1, 2) != 3:\ncase y > 1:\n\tt.Error(\"e\")\n}", ADD, "PASS"),
        row!("a default is decided by all the cases", "switch {\ncase y > 1:\ndefault:\n\tt.Error(\"e\")\ncase Add(1, 2) != 3:\n}", ADD, "PASS"),
        row!("a default before the derived case", "switch k {\ndefault:\n\tt.Error(\"e\")\ncase Add(1, 2):\n}", ADD, "PASS"),
        row!("a clause before the default is not decided by a case after it", "k := 1\nswitch k {\ncase 1:\n\tt.Error(\"e\")\ndefault:\ncase Add(1, 2):\n}", ADD, "FAIL", &[("DA-003", "FAIL")]),
        row!("a type switch guard is not a tag", "switch v := any(Add(1, 2)).(type) {\ncase int:\n\t_ = v\ndefault:\n\tt.Fatal(\"x\")\n}", ADD, "UNKNOWN", &[("DA-003", "UNKNOWN")]),
        row!("a fallthrough keeps the clause", "switch Add(1, 2) {\ncase 3:\n\tfallthrough\ncase 4:\n\tt.Error(\"e\")\n}", ADD, "PASS"),
    ];
    check(&rows);
}

/// DES-408, DS-1208: `audit_rules` gives DA-001, DA-004, DA-005 and DA-006 once
/// and DA-002 and DA-003 for each declared target, and DA-002 is the contract's
/// default, which proves no reach.
/// @vtest.id TEST-ADAPTER-GO-AUDIT-RULES-ARE-REPORTED-PER-TARGET
/// @vtest.covers VO-ADAPTER-GO-AUDIT-PER-TARGET-INDIVIDUAL-APPLICATION, VO-ADAPTER-GO-AUDIT-DA-002-UNKNOWN-IS-NOT-UNREACHED
/// @vtest.target crates/vtest-adapter-go/src/static_audit.rs::rule_verdict
/// @vtest.intent the rule rows of a Go Test with two targets are DA-001, then DA-002 and DA-003 for the first target, DA-002 and DA-003 for the second, then DA-004, DA-005 and DA-006, DA-002 is UNKNOWN for every target, and a failing DA-003 is reported on the target it is for
#[test]
fn the_rules_are_reported_once_and_per_target() {
    let construct = construct("if got := Add(1, 2); got != 3 {\n\tt.Fatal(\"x\")\n}\nDouble(2)");
    let file = file_of(&construct, "", "app");
    let macros: Vec<String> = Vec::new();
    let subject = Subject {
        construct: &construct,
        test_path: PATH,
        file_text: Some(&file),
        specs: vec![
            TargetSpec::from_locator("app/calc.go::Add"),
            TargetSpec::from_locator("app/calc.go::Double"),
        ],
        macros: &macros,
    };
    let evaluation = evaluate(&subject);
    let rows: Vec<(String, Option<&str>, RuleOutcome)> = [
        ("DA-001", None, &evaluation.da_001),
        ("DA-003", Some("Add"), &evaluation.da_003[0]),
        ("DA-003", Some("Double"), &evaluation.da_003[1]),
    ]
    .into_iter()
    .map(|(rule, target, da)| {
        let verdict = rule_verdict(rule, target.map(str::to_owned), da);
        (verdict.rule, target, verdict.outcome)
    })
    .collect();
    assert_eq!(
        rows,
        vec![
            ("DA-001".to_owned(), None, RuleOutcome::NoViolation),
            ("DA-003".to_owned(), Some("Add"), RuleOutcome::NoViolation),
            ("DA-003".to_owned(), Some("Double"), RuleOutcome::Fail),
        ]
    );
    assert_eq!(
        compose(&evaluation),
        StaticAnalysisVerdict::Fail(vec![format!(
            "DA-003: {}",
            match &evaluation.da_003[1] {
                Da::Fail(reason) => reason.clone(),
                other => panic!("{other:?}"),
            }
        )])
    );
}
