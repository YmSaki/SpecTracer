//! The `go` Static Audit capability (DES-619): `oracle_presence` from DA-001,
//! DA-003, DA-004, DA-005 and DA-006 (DS-605 to DS-608, DS-1680).
//!
//! Responsibility: read one Go Test function (`syntax`), walk it (`interp`) and
//! turn what the walk found into the verdict of each rule, composed as DS-606
//! to DS-608 say: `FAIL` if a rule fails, else `UNKNOWN` if a rule is unknown,
//! else `PASS`. The assert-equivalent syntax is DS-1883 to DS-1889, the
//! positions that count are DS-1970 and DS-1933, the data flow is DS-1968,
//! DS-1975, DS-1976, DS-1984 and DS-1986, and the exits and panics that do not
//! count are DS-1973, DS-1982 and DS-1983.
//!
//! Boundary: the audit reads only the Test's own text and the header of its
//! file (the input of DES-408). A function other than the target is a
//! delegation target (DS-641, DS-1936) and it is never confirmed to terminate
//! (DS-643), so a result handed to one is `UNKNOWN`, not `PASS`. DA-002 is not
//! provided: `target_reach` keeps the default of the contract, which proves no
//! reach, so `target_binding` rests on `go-cover` alone (DS-612, DS-613). A
//! text that does not parse, a form the walk does not follow and a call that
//! cannot be tied to the target give `UNKNOWN` (DS-615, DS-1680): `PASS` needs a
//! derived value shown to reach an assert-equivalent.

mod constant;
mod interp;
mod target;
mod visit;

use vtest_adapter_api::{
    RuleOutcome, RuleVerdict, StaticAnalysisAdapter, StaticAnalysisInput, StaticAnalysisVerdict,
    TargetReach,
};
use vtest_model::TargetRef;

use constant::Constness;
use interp::{run, Config, Facts, Structure};
use target::{FileInfo, TargetSpec};

use crate::syntax::parse_func_decl;
use crate::ADAPTER_ID;

/// One rule's verdict before DS-606 to DS-608 compose them.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Da {
    NoViolation,
    Fail(String),
    Unknown(String),
}

/// The `go` adapter's Static Analysis capability.
#[derive(Clone, Copy, Debug, Default)]
pub struct GoStaticAnalysisAdapter;

/// The verdicts of the rules for one Test.
#[derive(Debug)]
struct Evaluation {
    da_001: Da,
    /// DA-003 per declared target, in the order of `test.targets` (DS-630).
    da_003: Vec<Da>,
    da_004: Da,
    da_005: Da,
    da_006: Da,
}

fn unreadable(reason: &str) -> Da {
    Da::Unknown(format!(
        "the audit could not decide the Test ({reason}), so no rule is decided (DS-615)"
    ))
}

fn da_001(structure: &Structure) -> Da {
    let effective: Vec<_> = structure.sites.iter().filter(|s| !s.neutralized).collect();
    let neutralized = structure.sites.iter().any(|s| s.neutralized);
    if effective.is_empty() {
        return if neutralized {
            Da::Unknown(
                "the only assert-equivalent calls are `panic(...)` calls that a `defer` of their \
                 function body may recover, which DS-1982 does not count"
                    .to_owned(),
            )
        } else {
            Da::NoViolation
        };
    }
    if effective.iter().any(|s| s.constness == Constness::NonConst) {
        return Da::NoViolation;
    }
    if structure.maybe_asserts > 0 {
        return Da::Unknown(
            "every assert-equivalent call found is constant, and a call named like a failure \
             report is made on a receiver not declared a testing.T, which DS-1979 does not read"
                .to_owned(),
        );
    }
    if effective
        .iter()
        .any(|s| s.constness == Constness::Undecided)
    {
        return Da::Unknown(
            "every assert-equivalent call found is constant or not decided: an exit before it \
             whose condition the syntax does not fix (a loop body, a label, goto, select), code \
             that never runs, or an operand of || or && built from literals whose value is not \
             evaluated (DS-621, DS-1949, DS-1983)"
                .to_owned(),
        );
    }
    Da::Fail(
        "every assert-equivalent call has no position that decides whether it runs, or only \
         literal ones (DS-621, DS-1928, DS-1950)"
            .to_owned(),
    )
}

fn da_004(structure: &Structure) -> Da {
    if structure.sites.iter().any(|s| !s.neutralized && s.self_cmp) {
        Da::Fail(
            "a failure report or panic is enclosed by a condition that compares a value with \
             itself with == or != (DS-1889, DS-624)"
                .to_owned(),
        )
    } else if structure.macro_self_compare {
        Da::Unknown(
            "an assertion_macros call has two equal arguments or one that compares a value with \
             itself, which DA-004 may read as a value checked against itself (DS-1888, DS-1889)"
                .to_owned(),
        )
    } else {
        Da::NoViolation
    }
}

fn da_005(structure: &Structure) -> Da {
    if structure.empty_body {
        Da::Fail("the Test function's body has no statement (DS-625, DS-1994)".to_owned())
    } else {
        Da::NoViolation
    }
}

fn da_006(structure: &Structure) -> Da {
    if structure.sites.iter().any(|s| !s.neutralized) {
        return Da::NoViolation;
    }
    if structure.sites.iter().any(|s| s.neutralized) {
        return Da::Unknown(
            "the only assert-equivalent calls are `panic(...)` calls that a `defer` of their \
             function body may recover, which DS-1982 does not count"
                .to_owned(),
        );
    }
    if structure.maybe_asserts > 0 {
        return Da::Unknown(
            "a call named like a failure report is made on a receiver not declared a \
             testing.T, which DS-1979 does not read as an assert-equivalent"
                .to_owned(),
        );
    }
    if structure.candidates > 0 {
        return Da::Unknown(
            "the function has no assert-equivalent call, and a call it makes is a delegation \
             target (DS-642) whose termination this analysis does not confirm (DS-643, DS-652)"
                .to_owned(),
        );
    }
    Da::Fail(
        "the function has no assert-equivalent call and no call that could delegate the check \
         (DS-626)"
            .to_owned(),
    )
}

/// DS-623, DS-634, DS-1970: DA-003 for one declared target.
fn da_003(structure: &Structure, facts: &Facts) -> Da {
    if facts.taint {
        return Da::Unknown(
            "the function holds a form this analysis does not follow (goto, a labeled break or \
             continue, select, a loop that does not settle), so where the target's result flows \
             is not decided (DS-629, DS-1983)"
                .to_owned(),
        );
    }
    if facts.counted_calls == 0 {
        return Da::Unknown(
            "the declared target has no call at a counted position of the Test (DS-1970): it is \
             called in a function the Test calls, in a go or defer statement, in a function \
             literal that is not a t.Run continuation, through a name this analysis cannot tie \
             to the target, or not at all, so neither its being unverified nor its result \
             reaching an assert is shown (DS-634, DS-636)"
                .to_owned(),
        );
    }
    if facts.reach_clean {
        return Da::NoViolation;
    }
    if facts.reach_deleg {
        return Da::Unknown(
            "a value derived from the target's result reaches an assert-equivalent only with a \
             function call that is not a value conversion or comparison (DS-1986) among its \
             deciding positions or bindings: that call is a delegation target whose termination \
             this analysis does not confirm (DS-641, DS-648, DS-652)"
                .to_owned(),
        );
    }
    if facts.out_channel || facts.ext_observed {
        return Da::Unknown(
            "a call of the target takes an argument that is not a literal or has a receiver, or \
             a deciding position reads a call or a package-level name, so the target's effect \
             may be checked through a mutable argument, its receiver or state it changes, which \
             is not the result this analysis follows (DS-623, DS-1680)"
                .to_owned(),
        );
    }
    if structure.sites.iter().any(|s| s.undecided) {
        return Da::Unknown(
            "an assert-equivalent call of the function may not run, or whether it runs is not \
             decided by the syntax (an exit before it from inside a loop, by a label or goto; \
             code that never runs), so the target's result not reaching it is not a proof \
             (DS-1983)"
                .to_owned(),
        );
    }
    if facts.reach_unk
        || facts.uncertain
        || facts.uncounted_flow
        || facts.uncounted_calls > 0
        || facts.uncounted_assert
        || facts.ambiguous_calls > 0
        || structure.maybe_asserts > 0
    {
        return Da::Unknown(
            "the target's result may reach an assert-equivalent where this analysis cannot tell \
             (a value that may or may not be derived, a derived value handed to a function, a \
             pointer, a field or an element, a part that DS-1970 does not count, a call that may \
             be of another function with the target's name, or an assert-equivalent whose exits \
             are not decided), so it is neither shown reaching nor shown not reaching (DS-1975, \
             DS-1976, DS-1984, DS-1970, DS-1983)"
                .to_owned(),
        );
    }
    if structure.sites.iter().all(|s| s.neutralized) && structure.candidates > 0 {
        return Da::Unknown(
            "the function has no assert-equivalent call that counts and calls a delegation \
             target (DS-642, DS-1982)"
                .to_owned(),
        );
    }
    Da::Fail(
        "a declared target's call result reaches no assert-equivalent: no value derived from it \
         decides whether a failure report or panic runs or is an argument of an \
         assertion_macros call (DS-623, DS-1935, DS-1947)"
            .to_owned(),
    )
}

/// What the rules are decided over: the Test's text, the file it is in, and
/// its declared targets (`None` for one declared by a permanent SRC ID).
#[derive(Debug)]
struct Subject<'a> {
    construct: &'a str,
    test_path: &'a str,
    file_text: Option<&'a str>,
    specs: Vec<Option<TargetSpec>>,
    macros: &'a [String],
}

impl<'a> Subject<'a> {
    fn of(input: &StaticAnalysisInput<'a>) -> Self {
        Self {
            construct: input.construct_text,
            test_path: input.test.location.path.as_str(),
            file_text: input.file_text,
            specs: input
                .test
                .targets
                .iter()
                .map(|target| match target {
                    TargetRef::Locator(locator) => TargetSpec::from_locator(&locator.value),
                    TargetRef::SrcId(_) => None,
                })
                .collect(),
            macros: input.extra_assertion_macros,
        }
    }
}

/// The stack of the thread the audit runs on: the reader and the walk recurse
/// with the nesting of the Test, and the thread a verification runs on may have
/// a stack of a megabyte or less.
const AUDIT_STACK_BYTES: usize = 64 * 1024 * 1024;

fn evaluate(subject: &Subject<'_>) -> Evaluation {
    std::thread::scope(|scope| {
        match std::thread::Builder::new()
            .name("go-static-audit".to_owned())
            .stack_size(AUDIT_STACK_BYTES)
            .spawn_scoped(scope, || evaluate_text(subject))
        {
            // A walk that panics is UNKNOWN, never a crash of a verification and
            // never a verdict (DS-615).
            Ok(handle) => handle
                .join()
                .unwrap_or_else(|_| all_unknown(subject, "the audit stopped on an internal error")),
            Err(_) => all_unknown(subject, "the audit could not start a thread to run on"),
        }
    })
}

fn all_unknown(subject: &Subject<'_>, reason: &str) -> Evaluation {
    Evaluation {
        da_001: unreadable(reason),
        da_003: subject.specs.iter().map(|_| unreadable(reason)).collect(),
        da_004: unreadable(reason),
        da_005: unreadable(reason),
        da_006: unreadable(reason),
    }
}

fn evaluate_text(subject: &Subject<'_>) -> Evaluation {
    let unreadable_all = |reason: &str| all_unknown(subject, reason);
    let decl = match parse_func_decl(subject.construct) {
        Ok(decl) => decl,
        Err(error) => return unreadable_all(&error.to_string()),
    };
    let file = FileInfo::new(subject.test_path, subject.file_text);
    let all: Vec<TargetSpec> = subject.specs.iter().flatten().cloned().collect();
    let cfg = Config {
        src: subject.construct,
        file: &file,
        macros: subject.macros,
        all_targets: &all,
    };
    let (structure, _) = run(&cfg, &decl, None);
    if structure.exhausted {
        return unreadable_all("the walk of its statements reached its budget");
    }
    if structure.unrecognized_t {
        return unreadable_all(
            "its testing.T is taken through a package name other than `testing`",
        );
    }
    if structure.t_replaced {
        return unreadable_all(
            "the testing.T of the Test is replaced, has its address taken, or is used other than as the receiver of a method, an argument of a call or the value of a binding",
        );
    }
    let da_003 = subject
        .specs
        .iter()
        .map(|spec| match spec {
            Some(spec) => {
                let (_, facts) = run(&cfg, &decl, Some(spec));
                da_003(&structure, &facts)
            }
            None => Da::Unknown(
                "a target declared by a permanent SRC ID has no call name to look for, so \
                 whether its result is checked is neither proven nor refuted (DS-1680)"
                    .to_owned(),
            ),
        })
        .collect();
    Evaluation {
        da_001: da_001(&structure),
        da_003,
        da_004: da_004(&structure),
        da_005: da_005(&structure),
        da_006: da_006(&structure),
    }
}

/// DS-631 to DS-633: the rule's result from its per-target results.
fn fold_targets(results: &[Da]) -> Da {
    if let Some(Da::Fail(reason)) = results.iter().find(|d| matches!(d, Da::Fail(_))) {
        return Da::Fail(reason.clone());
    }
    if let Some(Da::Unknown(reason)) = results.iter().find(|d| matches!(d, Da::Unknown(_))) {
        return Da::Unknown(reason.clone());
    }
    Da::NoViolation
}

/// DS-606 to DS-608: the five rules composed into `oracle_presence`.
fn compose(evaluation: &Evaluation) -> StaticAnalysisVerdict {
    let da_003 = fold_targets(&evaluation.da_003);
    let rules: [(&str, &Da); 5] = [
        ("DA-001", &evaluation.da_001),
        ("DA-003", &da_003),
        ("DA-004", &evaluation.da_004),
        ("DA-005", &evaluation.da_005),
        ("DA-006", &evaluation.da_006),
    ];
    let mut failures = Vec::new();
    let mut unknowns = Vec::new();
    for (name, verdict) in rules {
        match verdict {
            Da::Fail(reason) => failures.push(format!("{name}: {reason}")),
            Da::Unknown(reason) => unknowns.push(format!("{name}: {reason}")),
            Da::NoViolation => {}
        }
    }
    if !failures.is_empty() {
        StaticAnalysisVerdict::Fail(failures)
    } else if !unknowns.is_empty() {
        StaticAnalysisVerdict::Unknown(unknowns)
    } else {
        StaticAnalysisVerdict::Pass(vec![
            "DA-001/003/004/005/006 all report no violation".to_owned()
        ])
    }
}

fn rule_verdict(rule: &str, target: Option<String>, da: &Da) -> RuleVerdict {
    let (outcome, reason) = match da {
        Da::Fail(reason) => (RuleOutcome::Fail, reason.clone()),
        Da::Unknown(reason) => (RuleOutcome::Unknown, reason.clone()),
        Da::NoViolation => (RuleOutcome::NoViolation, String::new()),
    };
    RuleVerdict {
        rule: rule.to_owned(),
        target,
        outcome,
        reason,
    }
}

impl StaticAnalysisAdapter for GoStaticAnalysisAdapter {
    fn id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn analyze(&self, input: StaticAnalysisInput<'_>) -> StaticAnalysisVerdict {
        compose(&evaluate(&Subject::of(&input)))
    }

    /// SPEC-387/DS-1208: DA-001, DA-004, DA-005 and DA-006 are Test-scoped;
    /// DA-002 and DA-003 are reported once per declared target (DS-630). DA-002
    /// is the contract's default, which proves no reach (DS-612).
    fn audit_rules(&self, input: StaticAnalysisInput<'_>) -> Vec<RuleVerdict> {
        let evaluation = evaluate(&Subject::of(&input));
        let mut out = vec![rule_verdict("DA-001", None, &evaluation.da_001)];
        let reach = self.target_reach(input);
        for (
            TargetReach {
                target,
                outcome,
                reason,
            },
            da_003,
        ) in reach.into_iter().zip(&evaluation.da_003)
        {
            let label = match &target {
                TargetRef::Locator(locator) => locator.value.clone(),
                TargetRef::SrcId(src_id) => src_id.as_str().to_owned(),
            };
            out.push(RuleVerdict {
                rule: "DA-002".to_owned(),
                target: Some(label.clone()),
                outcome: outcome.into(),
                reason,
            });
            out.push(rule_verdict("DA-003", Some(label), da_003));
        }
        out.push(rule_verdict("DA-004", None, &evaluation.da_004));
        out.push(rule_verdict("DA-005", None, &evaluation.da_005));
        out.push(rule_verdict("DA-006", None, &evaluation.da_006));
        out
    }
}

#[cfg(test)]
mod tests;
