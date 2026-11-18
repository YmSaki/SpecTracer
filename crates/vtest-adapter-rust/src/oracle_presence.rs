//! `rust-cargo`'s `oracle_presence` static-analysis capability (DS-605-627).
//!
//! `oracle_presence`'s question (REQ-073/077/079/080, verified present this
//! session) is answered by composing five deterministic, adapter-owned
//! rules — DA-001, DA-003, DA-004, DA-005, DA-006 (DS-605) — over the raw
//! bytes of one Test's construct. Core (`vtest-verify`) never interprets
//! Rust syntax itself (本冊:685-703, AGENTS.md "core owns nothing
//! language-specific"); it only asks this adapter capability for a verdict
//! and composes the five verdicts per DS-606/607/608/609/1680 (DS-1680
//! replaces the retired DS-610 as of the canon's fa63065 merge — verified
//! this session).
//!
//! **Disclosed scope narrowing** (the message that requested this capability
//! explicitly scoped it to "標準assert構文とtest frameworkのfailure
//! semanticsを検出する範囲でよい" — standard assert syntax and framework
//! failure semantics; not full delegation-chain analysis):
//! - DA-003's dataflow tracking is bounded to two shapes: a target call
//!   appearing lexically *inside* an assert-equivalent macro's argument
//!   list, or bound by a single `let NAME = ...;` whose `NAME` then appears
//!   as a single identifier in expression position of a *checked* argument of
//!   an assert-equivalent macro (`NAME`, `NAME.field`, `NAME.method()`, `&NAME`,
//!   `(NAME, ..)`; not `x.NAME`, not a string literal). Checked arguments are
//!   `assert!`'s condition and `assert_eq!`/`assert_ne!`'s two compared values;
//!   the message arguments after them only print on failure and do not count.
//!   `NAME` is matched as a whole identifier read from the argument list's
//!   tokens, not as a substring. When it cannot be told to mean the binding DA-003
//!   reports `Unknown` (#281; derivations from DS-623 / DS-615 / DS-1680, not
//!   stated in them): the name is bound again in the function (a second `let`, a
//!   closure parameter, a match arm, an assignment `NAME = ..`); the macro is an
//!   `assertion_macros` entry of the config, whose argument grammar vtest cannot
//!   be told (`assert_matches!(expr, pat)`); or the name appears in a nested macro
//!   other than `matches!` (its first expression), `format!`, `vec!`,
//!   `assert!`/`assert_eq!`/`assert_ne!` and `debug_assert*!` (read as above),
//!   or one of those that does not parse as expressions. This is a real subset of
//!   DS-628 ("let束縛、メソッドチェーン、フィールドアクセス"), not its full
//!   extent — a target call several bindings removed, or reached only
//!   through a closure, is not tracked and reports `Unknown` rather than
//!   `NoViolation` (fail-closed: DS-615 "違反なしと推測しない").
//! - DA-003 finds a target's call as a function call (`name(..)`, `Type::name(..)`)
//!   or as a method call (`receiver.name(..)`, #279). The callee is matched by the
//!   final name only; a method call's receiver type is not resolved, so a method
//!   call cannot be told from a same-named method of another type. When every
//!   call of the target that is not verified is such a method call, DA-003 reports
//!   `Unknown` (DS-1680: neither the call's identity nor the result's non-
//!   verification is proven), not `FAIL`. A verified method call is no violation
//!   as before, but only when no function call of the target stays unverified
//!   beside it; otherwise it is `Unknown` too, since the verifying call may be
//!   another type's same-named method. (These are derivations from DS-1680, not
//!   stated in it.) A target declared by `SrcId` has no call symbol, so `RustCargoStaticAnalysisAdapter::
//!   analyze` reports `Unknown` instead of a PASS composed without it.
//!   A call written inside a macro invocation's tokens is not a syntax-tree node
//!   (#282; derivations from DS-615 / DS-651 / DS-1680): in the checked arguments of
//!   `assert!` / `assert_eq!` / `assert_ne!` (read as expressions, with nested
//!   `matches!`, `format!` and `vec!`) it verifies the target; a call-shaped use of
//!   the target's name (`name(..)`, `name::<T>(..)`) in any other macro, in an
//!   assert's message argument, or in tokens that do not parse as expressions
//!   makes DA-003 `Unknown`. A bare reference or a path prefix (`name::item`) is
//!   not a call there, as it is not a call site outside a macro.
//!   Outside macros, the target's name as an expression path that is not a direct
//!   call's callee (`(name)(..)`, `let f = name;`, `apply(name)`) may be an indirect
//!   call that is not followed, so it makes DA-003 `Unknown` too (#288; derivation
//!   from DS-615 / DS-1680).
//! - §7.2.1's cross-Test delegation-chain search (DA-003/DA-006's "照合の
//!   委譲先") is not implemented: this capability never confirms that a
//!   delegation target terminates (DS-643). For DA-006 it only recognizes
//!   that a delegation target exists: a Test body with no assert-equivalent
//!   construct that calls a function or a method has those calls as its
//!   delegation targets (DS-642), and since their termination is never
//!   confirmed here, DA-006 reports `Unknown` for it, not `FAIL` (DS-626,
//!   DS-652, DS-1345). It therefore never reports `NoViolation` through
//!   delegation. A call written inside a macro invocation is not seen
//!   (the macro's tokens are not parsed), so a body whose only calls are
//!   there still reports `FAIL`. DA-003 treats the same way a body with no
//!   assert-equivalent macro and a call other than the declared targets and
//!   the assert-equivalent methods: `Unknown`, not `FAIL` (DS-642, DS-640).
//!   This deviates from DS-616: the "no assert-equivalent construct" premise
//!   looks at assert macros only, not at DS-619/620's `.unwrap()`/`.expect()`/
//!   `?` and Result signature, so it differs from DA-006's reading. The
//!   verdict only moves FAIL -> UNKNOWN. The target's own call is not counted
//!   as a delegation target (a derivation; DA-006 counts it, which differs
//!   from DS-650's self-delegation reading).
//! - DA-001's constant-expression classifier only recognizes literals
//!   (numeric/string/bool/char) and simple arithmetic/comparison
//!   combinations of literals; any identifier reference (a `const` item
//!   included) is treated as non-constant rather than resolved, which can
//!   only make DA-001 report `NoViolation` where a deeper analysis might
//!   report `Fail` — never the reverse — so this narrowing cannot manufacture
//!   a false `Fail`.

use quote::ToTokens;
use syn::{spanned::Spanned, visit::Visit};
use vtest_adapter_api::{
    RuleOutcome, RuleVerdict, StaticAnalysisAdapter, StaticAnalysisInput, StaticAnalysisVerdict,
};
use vtest_model::{Diagnostic, Locator, TargetRef};

use crate::ADAPTER_ID;

/// One DA rule's verdict, before DS-606/607/608 composes the five into one
/// `oracle_presence` state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DaVerdict {
    /// The rule found no violation (DS-606's "違反なし").
    NoViolation,
    /// The rule's own FAIL condition (verbatim DS id cited per rule) holds.
    Fail(&'static str),
    /// The rule could not decide (its own disclosed UNKNOWN escape, or this
    /// capability's own disclosed narrowing above).
    Unknown(String),
}

/// The five DA rules `oracle_presence` composes (DS-605), each independently
/// evaluated over one Test construct.
#[derive(Clone, Debug)]
pub struct OraclePresenceAnalysis {
    /// DA-001 (DS-621): constant assertion.
    pub da_001: DaVerdict,
    /// DA-003 (DS-623): result unverified.
    pub da_003: DaVerdict,
    /// DA-004 (DS-624): self comparison.
    pub da_004: DaVerdict,
    /// DA-005 (DS-625): empty test.
    pub da_005: DaVerdict,
    /// DA-006 (DS-626): no verification syntax.
    pub da_006: DaVerdict,
}

/// DS-617/618/619/620: `rust-cargo`'s assert-equivalent syntax, shared by
/// all six DA rules (DS-616). `extra_assertion_macros` is DS-617's
/// `rust-cargo` config `assertion_macros` list (this capability's caller
/// supplies it; this module does not read `config.yaml` itself).
const STANDARD_ASSERTION_MACROS: &[&str] = &["assert!", "assert_eq!", "assert_ne!", "panic!"];

/// Runs the five DA rules over `construct_text` (the Test function's raw
/// source bytes — the same bytes `vtest-scan` hashes into the Test subject,
/// read by byte range so this capability never re-parses or re-locates
/// anything core has already located).
///
/// `target_symbols` are the bare identifier names (the final `::`-segment)
/// of the Test's declared targets — DA-003 is evaluated per DS-776 "複数
/// target Testでは…各targetへ個別適用する" (that statement is about
/// DA-002/DA-003; this function folds per-target DA-003 verdicts with the
/// same FAIL-then-UNKNOWN-then-NoViolation priority DS-607/608 use for the
/// outer composition, since DS-605 does not separately name a per-target
/// aggregation rule for DA-003 within one Test — an implementation choice,
/// disclosed here rather than left silent).
pub fn analyze(
    construct_text: &str,
    target_symbols: &[String],
    extra_assertion_macros: &[String],
) -> OraclePresenceAnalysis {
    let macros: Vec<&str> = STANDARD_ASSERTION_MACROS
        .iter()
        .copied()
        .chain(extra_assertion_macros.iter().map(String::as_str))
        .collect();
    let assert_spans = find_assert_spans(construct_text, &macros);
    let syntax = RustFunctionSyntax::parse(construct_text);
    let has_should_panic = syntax
        .as_ref()
        .is_some_and(RustFunctionSyntax::has_should_panic);
    let has_assert_equivalent_method_or_try = syntax
        .as_ref()
        .is_some_and(RustFunctionSyntax::has_assert_equivalent_method_or_try);
    let has_result_return = syntax
        .as_ref()
        .is_some_and(RustFunctionSyntax::returns_result);
    let has_verification_syntax = !assert_spans.is_empty()
        || has_should_panic
        || has_assert_equivalent_method_or_try
        || has_result_return;
    let has_call = syntax.as_ref().is_some_and(RustFunctionSyntax::has_call);

    OraclePresenceAnalysis {
        da_001: da_001_constant_assertion(construct_text, &assert_spans),
        da_003: da_003_result_unverified(
            construct_text,
            target_symbols,
            &assert_spans,
            has_should_panic,
        ),
        da_004: da_004_self_comparison(construct_text),
        da_005: da_005_empty_test(construct_text),
        da_006: da_006_no_verification_syntax(has_verification_syntax, has_call),
    }
}

/// `vtest_adapter_api::StaticAnalysisAdapter` 実装（DES-408、本冊 §5.2
/// `StaticAnalysisAdapter`）。core（`vtest-verify`）はこの capability を
/// registry 経由で解決し、`analyze` を呼ぶだけで Rust 構文を直接解釈しない
/// （AGENTS.md「core owns nothing language-specific」）。関数本体は、
/// 以前 `vtest-verify::evaluate_oracle_presence` が直接呼んでいた
/// `analyze`/`compose`/`target_symbol` への委譲そのもの — 解析ロジック自体は
/// この PR で変更しない（本 PR は capability trait を通した dispatch への
/// 置き換えのみ）。
#[derive(Clone, Copy, Debug, Default)]
pub struct RustCargoStaticAnalysisAdapter;

impl RustCargoStaticAnalysisAdapter {
    pub const fn new() -> Self {
        Self
    }
}

impl StaticAnalysisAdapter for RustCargoStaticAnalysisAdapter {
    fn id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn analyze(&self, input: StaticAnalysisInput<'_>) -> StaticAnalysisVerdict {
        // DS-628-shaped disclosed narrowing（このmodule冒頭のdoc comment
        // 参照）: `TargetRef::Locator` targetだけがDA-003へsymbol名を渡す。
        // `SrcId` targetはsymbol名を持たないのでDA-003へは渡さない（違反を
        // 捏造しない側への narrowing）。ただし除外したままでは他の規則だけで
        // PASSに合成されるので、下で UNKNOWN に引き上げる。以前は
        // `vtest-verify::evaluate_oracle_presence` がこのfilter_mapを行っていた。
        let target_symbols: Vec<String> = input
            .test
            .targets
            .iter()
            .filter_map(|target| match target {
                TargetRef::Locator(locator) => Some(target_symbol(locator)),
                TargetRef::SrcId(_) => None,
            })
            .collect();
        let analysis = analyze(
            input.construct_text,
            &target_symbols,
            input.extra_assertion_macros,
        );
        let (is_fail, mut is_unknown, mut basis) = compose(&analysis);
        // Derivation (not stated in DS-1680 / DS-615): a SrcId target has no
        // call symbol here, so DA-003 cannot look for its call. That is neither
        // a proof of non-verification nor of a verifying device, which DS-1680
        // reads as UNKNOWN: a would-be PASS becomes UNKNOWN. A FAIL from
        // another rule is unaffected.
        let has_unidentified_target = input
            .test
            .targets
            .iter()
            .any(|target| matches!(target, TargetRef::SrcId(_)));
        if has_unidentified_target && !is_fail && !is_unknown {
            is_unknown = true;
            basis = vec![
                "DA-003: a SrcId target is not resolved to a call symbol by this analysis, so \
                 its call cannot be found and the result being unverified is neither proven \
                 nor refuted (DS-1680)"
                    .to_owned(),
            ];
        }
        if is_fail {
            StaticAnalysisVerdict::Fail(basis)
        } else if is_unknown {
            StaticAnalysisVerdict::Unknown(basis)
        } else {
            StaticAnalysisVerdict::Pass(basis)
        }
    }

    /// SPEC-387/DS-1208: DA-001/004/005/006 are Test-scoped; DA-002 and
    /// DA-003 are reported once per declared target (DS-630).
    ///
    /// DA-002 (static reachability, DS-622) is not implemented by this
    /// adapter, so every target reports `UNKNOWN` for it — the analysis
    /// boundary, never "not called" (DS-612). `SrcId` targets are not
    /// resolved to a symbol here, so their DA-003 is `UNKNOWN` too.
    fn audit_rules(&self, input: StaticAnalysisInput<'_>) -> Vec<RuleVerdict> {
        let symbols: Vec<String> = input
            .test
            .targets
            .iter()
            .filter_map(|target| match target {
                TargetRef::Locator(locator) => Some(target_symbol(locator)),
                TargetRef::SrcId(_) => None,
            })
            .collect();
        let whole = analyze(input.construct_text, &symbols, input.extra_assertion_macros);
        let mut out = Vec::new();
        let push = |out: &mut Vec<RuleVerdict>,
                    rule: &str,
                    target: Option<String>,
                    verdict: &DaVerdict| {
            let (outcome, reason) = match verdict {
                DaVerdict::Fail(reason) => (RuleOutcome::Fail, (*reason).to_owned()),
                DaVerdict::Unknown(reason) => (RuleOutcome::Unknown, reason.clone()),
                DaVerdict::NoViolation => (RuleOutcome::NoViolation, String::new()),
            };
            out.push(RuleVerdict {
                rule: rule.to_owned(),
                target,
                outcome,
                reason,
            });
        };
        push(&mut out, "DA-001", None, &whole.da_001);
        for target in &input.test.targets {
            let label = match target {
                TargetRef::Locator(locator) => locator.value.clone(),
                TargetRef::SrcId(src_id) => src_id.as_str().to_owned(),
            };
            push(
                &mut out,
                "DA-002",
                Some(label.clone()),
                &DaVerdict::Unknown(
                    "static target reachability (DA-002) is not implemented by rust-cargo; only \
                     runtime measurement can establish it (DS-612/DS-613)"
                        .to_owned(),
                ),
            );
            let da_003 = match target {
                TargetRef::Locator(locator) => {
                    analyze(
                        input.construct_text,
                        &[target_symbol(locator)],
                        input.extra_assertion_macros,
                    )
                    .da_003
                }
                TargetRef::SrcId(_) => DaVerdict::Unknown(
                    "a SrcId target is not resolved to a call symbol by this analysis".to_owned(),
                ),
            };
            push(&mut out, "DA-003", Some(label), &da_003);
        }
        push(&mut out, "DA-004", None, &whole.da_004);
        push(&mut out, "DA-005", None, &whole.da_005);
        push(&mut out, "DA-006", None, &whole.da_006);
        out
    }
}

/// DS-606/607/608: compose the five DA verdicts into one `oracle_presence`
/// outcome. Returns `(is_fail, is_unknown, basis)` — `vtest-verify` (core)
/// maps that into `VerificationState`/`DiagnosticLabel`, since this crate
/// does not construct core's verification types (core owns that vocabulary).
pub fn compose(analysis: &OraclePresenceAnalysis) -> (bool, bool, Vec<String>) {
    let rules: [(&str, &DaVerdict); 5] = [
        ("DA-001", &analysis.da_001),
        ("DA-003", &analysis.da_003),
        ("DA-004", &analysis.da_004),
        ("DA-005", &analysis.da_005),
        ("DA-006", &analysis.da_006),
    ];
    let mut failures = Vec::new();
    let mut unknowns = Vec::new();
    for (name, verdict) in rules {
        match verdict {
            DaVerdict::Fail(reason) => failures.push(format!("{name}: {reason}")),
            DaVerdict::Unknown(reason) => unknowns.push(format!("{name}: {reason}")),
            DaVerdict::NoViolation => {}
        }
    }
    if !failures.is_empty() {
        return (true, false, failures);
    }
    if !unknowns.is_empty() {
        return (false, true, unknowns);
    }
    (
        false,
        false,
        vec!["DA-001/003/004/005/006 all report no violation".to_owned()],
    )
}

/// Extracts the bare final-segment symbol name from a declared target
/// `Locator` (`rust-cargo`'s own opaque `<path>::<item_path>` value shape —
/// this adapter owns that syntax, core does not, per the crate's own
/// `locator_parts`-style precedent in `vtest-exec`).
pub fn target_symbol(locator: &Locator) -> String {
    locator
        .value
        .rsplit("::")
        .next()
        .unwrap_or(&locator.value)
        .to_owned()
}

// ---------------------------------------------------------------------------
// DA-001 (DS-621): constant assertion
// ---------------------------------------------------------------------------

fn da_001_constant_assertion(text: &str, assert_spans: &[(usize, usize)]) -> DaVerdict {
    if assert_spans.is_empty() {
        // No assert-equivalent macro call at all: DA-001's own subject
        // (an assert whose arguments are constant) does not occur, so
        // there is nothing to fail on. `#[should_panic]`/`.unwrap()`-only
        // verification is DA-003/DA-006's territory, not DA-001's.
        return DaVerdict::NoViolation;
    }
    for &(start, end) in assert_spans {
        if let Constancy::NotAllLiteral = classify_constancy(&text[start..end]) {
            return DaVerdict::NoViolation;
        }
    }
    // DS-621's disclosed `Unknown` escape ("定数性を確定できない式") is not
    // reachable by this classifier: any identifier reference already
    // resolves to `NotAllLiteral` (module doc — this can only under-report
    // `Fail`, never fabricate one), so there is no third outcome left to
    // route to `Unknown` here.
    DaVerdict::Fail("every assert-equivalent call's arguments are all literal/constant (DS-621)")
}

enum Constancy {
    AllLiteral,
    NotAllLiteral,
}

/// A conservative literal/constant-expression classifier: recognizes
/// numeric/string/bool/char literals and their combination with arithmetic/
/// comparison operators and parentheses/commas/whitespace. Any identifier
/// (a variable, a function call, a `const` item reference) makes the whole
/// argument list `NotAllLiteral` — this can only under-report DA-001's
/// `Fail` (treat a truly-constant `const` reference as non-constant), never
/// over-report it, per this module's doc comment.
fn classify_constancy(arguments: &str) -> Constancy {
    let mut chars = arguments.chars().peekable();
    let mut saw_identifier_like = false;
    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                // Skip a string literal body.
                while let Some(next) = chars.next() {
                    if next == '\\' {
                        chars.next();
                        continue;
                    }
                    if next == '"' {
                        break;
                    }
                }
            }
            '\'' => {
                // A char literal or a lifetime — either way, not an
                // identifier reference to a runtime value; skip to the
                // closing quote if present.
                while let Some(next) = chars.next() {
                    if next == '\\' {
                        chars.next();
                        continue;
                    }
                    if next == '\'' {
                        break;
                    }
                }
            }
            c if c.is_ascii_digit() => {
                while chars
                    .peek()
                    .is_some_and(|c| c.is_alphanumeric() || *c == '.' || *c == '_')
                {
                    chars.next();
                }
            }
            c if c.is_alphabetic() || c == '_' => {
                saw_identifier_like = true;
                break;
            }
            _ => {}
        }
    }
    if saw_identifier_like {
        // `true`/`false` are the only bare identifiers this classifier
        // must not misclassify as a variable reference.
        let trimmed = arguments.trim();
        if trimmed == "true" || trimmed == "false" {
            return Constancy::AllLiteral;
        }
        Constancy::NotAllLiteral
    } else {
        Constancy::AllLiteral
    }
}

// ---------------------------------------------------------------------------
// DA-003 (DS-623): result unverified
// ---------------------------------------------------------------------------

fn da_003_result_unverified(
    text: &str,
    target_symbols: &[String],
    assert_spans: &[(usize, usize)],
    has_should_panic: bool,
) -> DaVerdict {
    if target_symbols.is_empty() {
        // No declared target to check a call against; DA-003 has no
        // subject. (This capability only reaches Tests with at least one
        // resolved target — `evaluate_oracle_presence`'s caller in
        // `vtest-verify` already holds DS-1664's "no target" case at
        // `NO_EVIDENCE` before ever calling this analysis.)
        return DaVerdict::NoViolation;
    }
    if has_should_panic {
        return DaVerdict::NoViolation;
    }
    let mut any_call_found = false;
    let mut any_ambiguous = false;
    let mut any_unidentified = false;
    let mut any_unverified = false;
    let mut any_binding_uncertain = false;
    let mut any_macro_opaque = false;
    for symbol in target_symbols {
        match target_call_result_is_verified(text, symbol, assert_spans) {
            TargetCallVerification::NotCalled => {}
            TargetCallVerification::Verified => {
                any_call_found = true;
            }
            TargetCallVerification::Unverified => {
                any_call_found = true;
                any_unverified = true;
            }
            TargetCallVerification::Ambiguous => {
                any_call_found = true;
                any_ambiguous = true;
            }
            TargetCallVerification::Unidentified => {
                any_call_found = true;
                any_unidentified = true;
            }
            TargetCallVerification::BindingUncertain => {
                any_call_found = true;
                any_binding_uncertain = true;
            }
            TargetCallVerification::MacroOpaque => {
                any_call_found = true;
                any_macro_opaque = true;
            }
        }
    }
    if !any_call_found {
        // No declared target is called at all in this construct — that is
        // DA-002's ("対象未呼出") territory (target_binding, not
        // oracle_presence). DA-003 only fires once a call exists.
        return DaVerdict::NoViolation;
    }
    if any_ambiguous {
        return DaVerdict::Unknown(
            "a target call's result may be verified through a mutable reference or \
             global state, which this capability does not track (DS-623)"
                .to_owned(),
        );
    }
    if any_binding_uncertain {
        // Derivation (see `binding_reach`): the name of the binding that holds the
        // result appears in an assert, but it cannot be told to mean that binding.
        return DaVerdict::Unknown(
            "a declared target's result is bound to a name that an assert-equivalent \
             construct mentions, but the name cannot be shown to mean that binding (it is \
             bound again, or appears in tokens this capability does not parse), so the \
             result reaching the assert is neither proven nor refuted (DS-1680)"
                .to_owned(),
        );
    }
    if any_macro_opaque {
        // Derivation (see `macro_target_use`): a call hidden in a macro's tokens may
        // or may not exist and may or may not be verified.
        return DaVerdict::Unknown(
            "a declared target is called in the tokens of a macro invocation that is not read \
             as a call whose result an assert checks (a macro other than an assert, an \
             assert's message argument, or tokens that do not parse as expressions), or its \
             name is used as a value (an indirect call), so whether it is called and whether \
             its result is verified is neither proven nor refuted (DS-615, DS-651, DS-1680)"
                .to_owned(),
        );
    }
    if any_unidentified {
        // Derivation (not stated in DS-1680 / DS-615 / DS-623): a method call is
        // matched to the target by name only; the receiver's type is not
        // resolved. A same-named method of another type cannot be told from the
        // target, so neither "the target's result is not verified" (DS-623's
        // FAIL condition) nor "it is verified" (DS-1680's verifying device) can
        // be said deterministically, which DS-1680 reads as UNKNOWN. It arises
        // when every unverified call is a method call, or when the only verified
        // call is a method call and a function call of the target stays
        // unverified.
        return DaVerdict::Unknown(
            "a declared target's call is identified by name only (a method call \
             `receiver.name(..)`, whose receiver type is not resolved): it may be another \
             type's same-named method, or the only verified call may be one, so the result \
             being unverified is neither proven nor refuted (DS-1680)"
                .to_owned(),
        );
    }
    if any_unverified {
        // REQ-074「不成立が構造から証明できる場合のみFAIL」/DS-1680（旧
        // DS-610の退役後の再掲。verified against the merged canon this
        // session, fa63065: "静的解析は不成立の証明であり、不成立を証明
        // できないことだけを理由にUNKNOWNとはしない。照合装置の存在が
        // 決定論的に確認できる場合は違反なしとし、不成立の証明も照合装置
        // の存在の確認もいずれも決定論的に言えない場合に限りUNKNOWNとす
        // る」）/DS-615「adapterが不完全、解析限界…を報告した場合は
        // UNKNOWNとし、違反なしと推測しない」: this capability's dataflow
        // tracking is bounded (module doc — a direct in-assert call or one
        // `let` binding). When at least one assert-equivalent construct
        // exists in the function (`assert_spans` non-empty) but the call
        // cannot be traced into any of them, neither "the result does not
        // reach one" (DS-623's FAIL condition) nor "the result reaches
        // one" (DS-1680's "照合装置の存在が確認できる" -> no violation) can
        // be said deterministically by this capability's bounded tracker —
        // exactly DS-1680's own UNKNOWN condition ("いずれも決定論的に
        //言えない場合"), not a structural proof of non-verification.
        //
        // Only when `assert_spans` is empty is non-reaching a call
        // structurally certain: there is no assert-equivalent construct in
        // the function at all for any path to reach, tracked or not — a
        // genuine DS-623 `Fail`.
        //
        // DS-642: with no assert-equivalent construct, every other call in the
        // body is a delegation target of the Test. Its termination (§7.2.1,
        // #242) is not confirmed here, so DA-003 is neither a violation nor
        // its absence (DS-640, DS-652).
        // Deviation from DS-616 (disclosed): `assert_spans` holds assert
        // macros only, not the full assert-equivalent set of DS-616/619/620
        // (`.unwrap()`/`.expect()`/`?`, Result signature), so "no
        // assert-equivalent construct" is read more widely than DA-006 reads
        // it. This only moves a verdict FAIL -> UNKNOWN, never toward PASS.
        // Derivation (DS-642 does not exclude it): the target's own call is
        // not counted as a delegation target, so a body that only calls and
        // discards the target stays FAIL. DA-006 counts the target as a
        // delegation target, which differs from this reading (cf. DS-650's
        // self-delegation).
        if assert_spans.is_empty() {
            return if has_non_target_call(text, target_symbols) {
                DaVerdict::Unknown(
                    "a call other than the declared target is a delegation target of this Test \
                     (DS-642) and its termination (§7.2.1) is not confirmed, so the result is \
                     neither a violation nor its absence (DS-652)"
                        .to_owned(),
                )
            } else {
                DaVerdict::Fail(
                    "a declared target's call result reaches no assert-equivalent construct, \
                     and no #[should_panic] is present (DS-623)",
                )
            };
        }
        return DaVerdict::Unknown(
            "a declared target's call could not be traced into any of the function's \
             assert-equivalent constructs by this capability's bounded dataflow \
             tracking (a method chain, a second let-binding, or a field access this \
             capability does not follow could still carry the result there) — neither \
             reaching nor non-reaching can be said deterministically, which is DS-1680's \
             own UNKNOWN condition, not a structural proof of non-verification"
                .to_owned(),
        );
    }
    DaVerdict::NoViolation
}

/// `true` when the body calls a function or method that is neither a
/// declared target nor an assert-equivalent method (DS-642 delegation).
fn has_non_target_call(text: &str, target_symbols: &[String]) -> bool {
    let Ok(function) = syn::parse_str::<syn::ItemFn>(text) else {
        return false;
    };
    let mut visitor = NonTargetCallVisitor {
        symbols: target_symbols,
        found: false,
    };
    visitor.visit_block(&function.block);
    visitor.found
}

struct NonTargetCallVisitor<'a> {
    symbols: &'a [String],
    found: bool,
}

impl<'ast> Visit<'ast> for NonTargetCallVisitor<'_> {
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        let is_target =
            match node.func.as_ref() {
                syn::Expr::Path(path) => path.path.segments.last().is_some_and(|segment| {
                    self.symbols.iter().any(|symbol| segment.ident == symbol)
                }),
                _ => false,
            };
        if !is_target {
            self.found = true;
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        let name = node.method.to_string();
        let is_excluded = matches!(
            name.as_str(),
            "unwrap" | "expect" | "unwrap_err" | "expect_err"
        ) || self.symbols.contains(&name);
        if !is_excluded {
            self.found = true;
        }
        syn::visit::visit_expr_method_call(self, node);
    }
}

enum TargetCallVerification {
    NotCalled,
    Verified,
    Unverified,
    Ambiguous,
    /// Every unverified call is a method call matched by name only (#279).
    Unidentified,
    /// The binding that holds a call's result is named in an assert, but the name
    /// cannot be shown to mean that binding (#281).
    BindingUncertain,
    /// The target is called in macro tokens that are not read as a call whose
    /// result is checked (#282).
    MacroOpaque,
}

/// Whether a `let` binding's name reaches an assert-equivalent argument list.
enum BindingReach {
    Reaches,
    NotReached,
    Uncertain,
}

/// Derivation (not stated in DS-623 / DS-628 / DS-1680 / DS-615): a binding's name
/// reaches an assert only as a whole identifier in expression position, read from
/// the tokens of the assert's argument list. A field or method named like the
/// binding (`x.r`, `x.r()`) and a string literal are not the binding. Only the
/// arguments that are checked count (`assert!`: the condition; `assert_eq!` /
/// `assert_ne!`: the two compared values); the message arguments after them only
/// print on failure and verify nothing. The name cannot be told to mean the
/// binding (`Uncertain`, which DS-1680 reads as UNKNOWN) when the name is bound
/// again anywhere in the function (a second `let`, a closure parameter, a match
/// arm, a parameter, an assignment `name = ..` or `name += ..`), when the
/// argument list is not a list of expressions, when the macro is an
/// `assertion_macros` entry of the project config (its argument grammar is not
/// known), or when the name appears in a nested macro other than `matches!`,
/// `format!`, `vec!` and the assert macros, or in one of those whose tokens do not
/// parse as expressions.
fn binding_reach(text: &str, binding: &str, assert_spans: &[(usize, usize)]) -> BindingReach {
    let reaches = |outcome: &BindingReach| matches!(outcome, BindingReach::Reaches);
    let outcomes: Vec<BindingReach> = assert_spans
        .iter()
        .map(|&(start, end)| {
            span_binding_reach(&text[start..end], macro_name_before(text, start), binding)
        })
        .collect();
    if outcomes.iter().any(reaches) {
        return if count_rebindings(text, binding) > 1 {
            BindingReach::Uncertain
        } else {
            BindingReach::Reaches
        };
    }
    if outcomes
        .iter()
        .any(|outcome| matches!(outcome, BindingReach::Uncertain))
    {
        return BindingReach::Uncertain;
    }
    BindingReach::NotReached
}

/// The name of the macro whose argument list starts at `args_start` (the byte after
/// its opening delimiter), read from the text before it.
fn macro_name_before(text: &str, args_start: usize) -> Option<&str> {
    let before = text[..args_start]
        .trim_end()
        .strip_suffix(['(', '[', '{'])?
        .trim_end()
        .strip_suffix('!')?
        .trim_end();
    let name_start = before
        .rfind(|c: char| !(c.is_alphanumeric() || c == '_'))
        .map_or(0, |index| index + 1);
    let name = &before[name_start..];
    (!name.is_empty()).then_some(name)
}

/// How many leading arguments of a macro are checked (the rest is message), or
/// `None` for a macro whose argument grammar is not known. `usize::MAX`: every
/// argument is a value.
fn checked_argument_count(macro_name: &str) -> Option<usize> {
    match macro_name {
        "assert" | "debug_assert" => Some(1),
        "assert_eq" | "assert_ne" | "debug_assert_eq" | "debug_assert_ne" => Some(2),
        "panic" => Some(0),
        "format" | "vec" => Some(usize::MAX),
        _ => None,
    }
}

fn span_binding_reach(span_text: &str, macro_name: Option<&str>, binding: &str) -> BindingReach {
    let Ok(tokens) = span_text.parse::<proc_macro2::TokenStream>() else {
        return BindingReach::Uncertain;
    };
    if !tokens_mention_ident(tokens, binding) {
        return BindingReach::NotReached;
    }
    // An `assertion_macros` entry of the config is not a built-in assert: its
    // argument grammar (`assert_matches!(expr, pat)`) is not known (DS-615).
    let Some(checked) = macro_name
        .filter(|name| STANDARD_ASSERTION_MACROS.contains(&format!("{name}!").as_str()))
        .and_then(checked_argument_count)
    else {
        return BindingReach::Uncertain;
    };
    let Ok(arguments) = syn::parse::Parser::parse_str(
        syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated,
        span_text,
    ) else {
        return BindingReach::Uncertain;
    };
    let mut visitor = BindingUseVisitor {
        name: binding,
        path_use: false,
        opaque: false,
    };
    for argument in arguments.iter().take(checked) {
        visitor.visit_expr(argument);
    }
    if visitor.opaque {
        BindingReach::Uncertain
    } else if visitor.path_use {
        BindingReach::Reaches
    } else {
        BindingReach::NotReached
    }
}

fn tokens_mention_ident(tokens: proc_macro2::TokenStream, name: &str) -> bool {
    tokens.into_iter().any(|tree| match tree {
        proc_macro2::TokenTree::Ident(ident) => ident == name,
        proc_macro2::TokenTree::Group(group) => tokens_mention_ident(group.stream(), name),
        proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => false,
    })
}

/// The expressions of a macro whose argument grammar is known, split into the
/// arguments that are checked or computed and those that are not.
struct MacroArguments {
    checked: Vec<syn::Expr>,
    unchecked: Vec<proc_macro2::TokenStream>,
}

/// Derivation (not stated in DS-615 / DS-1680): `matches!`, `format!`, `vec!` and
/// the assert macros take expressions in a fixed grammar, so a name in an
/// expression position of a checked argument is read as in an assert's own
/// argument list; message arguments (and `matches!`'s pattern, in which a name is
/// bound by the arm) are returned as `unchecked`. `None` for any other macro, and
/// for one of these whose tokens do not parse that way.
fn macro_arguments(node: &syn::Macro) -> Option<MacroArguments> {
    let name = node.path.segments.last()?.ident.to_string();
    if name == "matches" {
        return syn::parse::Parser::parse2(
            |input: syn::parse::ParseStream<'_>| {
                let first: syn::Expr = input.parse()?;
                let rest: proc_macro2::TokenStream = input.parse()?;
                Ok(MacroArguments {
                    checked: vec![first],
                    unchecked: vec![rest],
                })
            },
            node.tokens.clone(),
        )
        .ok();
    }
    let checked_count = checked_argument_count(&name)?;
    let parsed = syn::parse::Parser::parse2(
        syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated,
        node.tokens.clone(),
    )
    .ok()?;
    let mut arguments = MacroArguments {
        checked: Vec::new(),
        unchecked: Vec::new(),
    };
    for (index, argument) in parsed.into_iter().enumerate() {
        if index >= checked_count {
            arguments.unchecked.push(argument.into_token_stream());
            continue;
        }
        arguments.checked.push(match argument {
            // `format!("{r}", r = value)`: a named argument. The left side names
            // the placeholder, not a binding; only the value is read.
            syn::Expr::Assign(named) if name == "format" => *named.right,
            other => other,
        });
    }
    Some(arguments)
}

/// How a target is called in the tokens of the macro invocations of a Test.
#[derive(Default)]
struct MacroUse {
    /// A call `name(..)` / `Type::name(..)` in a checked argument of an assert.
    verified_path: bool,
    /// A call `receiver.name(..)` in a checked argument of an assert.
    verified_method: bool,
    /// A call-shaped use of the name where it is not read as a call whose result
    /// is checked: in a macro other than an assert, in an assert's message
    /// argument or in tokens that do not parse as expressions; or the name used as
    /// a value outside a macro (`ValueUseVisitor`).
    opaque: bool,
}

/// Derivation (not stated in DS-615 / DS-651 / DS-1680): a macro's tokens are not
/// syntax-tree nodes, so a call written there is found only by reading the
/// tokens. In `assert!` / `assert_eq!` / `assert_ne!` the checked arguments are
/// read as expressions, and a call found there verifies the target as a call
/// outside a macro that reaches an assert does. A call hidden anywhere else (any
/// other macro, a message argument, tokens that do not parse) may or may not exist
/// and may or may not be verified, and DS-651 reads a call hidden in a macro
/// expansion as UNKNOWN, so a call-shaped use of the name there is `opaque`
/// (`tokens_call_ident`; a bare reference or a path prefix is not). Reading macro
/// tokens only moves a verdict toward UNKNOWN or turns "not called" into
/// verified: `target_call_result_is_verified` takes a call read in an assert as
/// verifying only when the Test has no call outside a macro, so it never clears
/// a call outside a macro that stays unverified, and a verified method call
/// (matched by name only) does not clear an `opaque` name that may be a
/// function call.
fn macro_target_use(text: &str, symbol: &str) -> MacroUse {
    let Ok(function) = syn::parse_str::<syn::ItemFn>(text) else {
        return MacroUse::default();
    };
    let mut visitor = MacroUseVisitor {
        symbol,
        found: MacroUse::default(),
    };
    visitor.visit_item_fn(&function);
    let mut values = ValueUseVisitor {
        symbol,
        found: false,
    };
    values.visit_item_fn(&function);
    visitor.found.opaque |= values.found;
    visitor.found
}

/// Derivation (not stated in DS-615 / DS-623 / DS-1680): the target's name as an
/// expression path that is not the callee of a direct call (`(name)(..)`,
/// `let f = name;`, `apply(name)`, `Type::name` passed as a value) may be called
/// later through another name, which this capability does not follow. It is
/// `opaque`, like a call hidden in macro tokens: UNKNOWN, not "not called". A
/// `use`, a type, a path prefix, a field, a method name and a string are not
/// expression paths of the target. Inside macro tokens only a call-shaped name is
/// read (`tokens_call_ident`), so a bare reference there is not a value use.
struct ValueUseVisitor<'a> {
    symbol: &'a str,
    found: bool,
}

impl<'ast> Visit<'ast> for ValueUseVisitor<'_> {
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        // A path callee is a direct call, read by `find_call_sites`; any other
        // callee expression (`(name)`) is visited as a value.
        if !matches!(node.func.as_ref(), syn::Expr::Path(_)) {
            self.visit_expr(&node.func);
        }
        for argument in &node.args {
            self.visit_expr(argument);
        }
    }

    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        if node
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == self.symbol)
        {
            self.found = true;
        }
    }
}

struct MacroUseVisitor<'a> {
    symbol: &'a str,
    found: MacroUse,
}

impl<'ast> Visit<'ast> for MacroUseVisitor<'_> {
    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if !tokens_call_ident(node.tokens.clone(), self.symbol) {
            return;
        }
        let is_assert = node.path.segments.last().is_some_and(|segment| {
            ["assert", "assert_eq", "assert_ne"].contains(&segment.ident.to_string().as_str())
        });
        let Some(arguments) = is_assert.then(|| macro_arguments(node)).flatten() else {
            self.found.opaque = true;
            return;
        };
        let mut calls = CheckedCallVisitor {
            symbol: self.symbol,
            path: false,
            method: false,
            opaque: false,
        };
        for expression in &arguments.checked {
            calls.visit_expr(expression);
        }
        self.found.verified_path |= calls.path;
        self.found.verified_method |= calls.method;
        self.found.opaque |= calls.opaque
            || arguments
                .unchecked
                .iter()
                .any(|tokens| tokens_call_ident(tokens.clone(), self.symbol));
    }
}

/// Finds calls of the target in the checked arguments of an assert, reading a
/// nested macro as `macro_arguments` does.
struct CheckedCallVisitor<'a> {
    symbol: &'a str,
    path: bool,
    method: bool,
    opaque: bool,
}

impl<'ast> Visit<'ast> for CheckedCallVisitor<'_> {
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = node.func.as_ref() {
            if path
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == self.symbol)
            {
                self.path = true;
            }
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if node.method == self.symbol {
            self.method = true;
        }
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if !tokens_call_ident(node.tokens.clone(), self.symbol) {
            return;
        }
        let Some(arguments) = macro_arguments(node) else {
            self.opaque = true;
            return;
        };
        for expression in &arguments.checked {
            self.visit_expr(expression);
        }
        self.opaque |= arguments
            .unchecked
            .iter()
            .any(|tokens| tokens_call_ident(tokens.clone(), self.symbol));
    }
}

/// Whether `name` appears in the tokens in the form of a call: followed by an
/// argument list (`name(..)`, `.name(..)`) or by `::<` (`name::<T>(..)`). A path
/// prefix (`name::item`), a macro name (`name!`) and a bare reference are not
/// calls, as they are not call sites outside a macro.
fn tokens_call_ident(tokens: proc_macro2::TokenStream, name: &str) -> bool {
    use proc_macro2::{Delimiter, TokenTree};
    let trees: Vec<TokenTree> = tokens.into_iter().collect();
    trees.iter().enumerate().any(|(index, tree)| match tree {
        TokenTree::Group(group) => tokens_call_ident(group.stream(), name),
        TokenTree::Ident(ident) if ident == name => match trees.get(index + 1) {
            Some(TokenTree::Group(group)) => group.delimiter() == Delimiter::Parenthesis,
            Some(TokenTree::Punct(first)) if first.as_char() == ':' => {
                matches!(
                    (trees.get(index + 2), trees.get(index + 3)),
                    (Some(TokenTree::Punct(second)), Some(TokenTree::Punct(angle)))
                        if second.as_char() == ':' && angle.as_char() == '<'
                )
            }
            _ => false,
        },
        TokenTree::Ident(_) | TokenTree::Punct(_) | TokenTree::Literal(_) => false,
    })
}

struct BindingUseVisitor<'a> {
    name: &'a str,
    path_use: bool,
    opaque: bool,
}

impl<'ast> Visit<'ast> for BindingUseVisitor<'_> {
    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        if node.qself.is_none() && node.path.is_ident(self.name) {
            self.path_use = true;
        }
    }

    fn visit_pat_ident(&mut self, node: &'ast syn::PatIdent) {
        if node.ident == self.name {
            self.opaque = true;
        }
        syn::visit::visit_pat_ident(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if !tokens_mention_ident(node.tokens.clone(), self.name) {
            return;
        }
        // Any macro `macro_arguments` does not read, or reads but cannot parse,
        // keeps the name uncertain.
        let Some(arguments) = macro_arguments(node) else {
            self.opaque = true;
            return;
        };
        for expression in &arguments.checked {
            self.visit_expr(expression);
        }
    }

    fn visit_expr_assign(&mut self, node: &'ast syn::ExprAssign) {
        if is_assignment_to(&node.left, self.name) {
            self.opaque = true;
        }
        syn::visit::visit_expr_assign(self, node);
    }

    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        if is_compound_assignment(node.op) && is_assignment_to(&node.left, self.name) {
            self.opaque = true;
        }
        syn::visit::visit_expr_binary(self, node);
    }
}

fn is_assignment_to(left: &syn::Expr, name: &str) -> bool {
    matches!(left, syn::Expr::Path(path) if path.qself.is_none() && path.path.is_ident(name))
}

fn is_compound_assignment(op: syn::BinOp) -> bool {
    matches!(
        op,
        syn::BinOp::AddAssign(_)
            | syn::BinOp::SubAssign(_)
            | syn::BinOp::MulAssign(_)
            | syn::BinOp::DivAssign(_)
            | syn::BinOp::RemAssign(_)
            | syn::BinOp::BitXorAssign(_)
            | syn::BinOp::BitAndAssign(_)
            | syn::BinOp::BitOrAssign(_)
            | syn::BinOp::ShlAssign(_)
            | syn::BinOp::ShrAssign(_)
    )
}

/// Number of places in the function that bind `name` again: patterns (`let`,
/// closure parameter, match arm, parameter, `for`) and assignments `name = ..` /
/// `name += ..`.
fn count_rebindings(text: &str, name: &str) -> usize {
    struct Counter<'a> {
        name: &'a str,
        count: usize,
    }
    impl<'ast> Visit<'ast> for Counter<'_> {
        fn visit_pat_ident(&mut self, node: &'ast syn::PatIdent) {
            if node.ident == self.name {
                self.count += 1;
            }
            syn::visit::visit_pat_ident(self, node);
        }

        fn visit_expr_assign(&mut self, node: &'ast syn::ExprAssign) {
            if is_assignment_to(&node.left, self.name) {
                self.count += 1;
            }
            syn::visit::visit_expr_assign(self, node);
        }

        fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
            if is_compound_assignment(node.op) && is_assignment_to(&node.left, self.name) {
                self.count += 1;
            }
            syn::visit::visit_expr_binary(self, node);
        }
    }
    let Ok(function) = syn::parse_str::<syn::ItemFn>(text) else {
        return 0;
    };
    let mut counter = Counter { name, count: 0 };
    counter.visit_item_fn(&function);
    counter.count
}

/// One call of a declared target found in the Test body.
struct CallSite {
    /// Byte offset where the callee name starts (the `ExprCall` function path,
    /// or the method name of an `ExprMethodCall`). `call_open_position` reads
    /// the argument list from here.
    callee_start: usize,
    /// Byte offset where the whole call expression starts (for a method call,
    /// its receiver).
    expr_start: usize,
    /// `true` for `receiver.name(..)`: identified by name only.
    is_method: bool,
}

fn target_call_result_is_verified(
    text: &str,
    symbol: &str,
    assert_spans: &[(usize, usize)],
) -> TargetCallVerification {
    let call_sites = find_call_sites(text, symbol);
    let macro_use = macro_target_use(text, symbol);
    if call_sites.is_empty() {
        // Derivation (#282): a call read inside an assert verifies the target only
        // when no call outside a macro exists, so reading macro tokens never
        // clears a call that stays unverified (DS-615: it only moves toward UNKNOWN).
        return if macro_use.verified_path || (macro_use.verified_method && !macro_use.opaque) {
            TargetCallVerification::Verified
        } else if macro_use.opaque {
            // A verified method call is matched by name only: it does not clear a
            // call that a macro may make as a function.
            TargetCallVerification::MacroOpaque
        } else {
            TargetCallVerification::NotCalled
        };
    }
    // A call whose result is consumed by an assert-equivalent construct
    // (directly, through DS-626's method chain, or through the single `let`
    // binding below) verifies the target wherever it appears; the order of
    // call sites in the function does not change that. Only when no call is
    // verified this way does a call inside a loop make the outcome
    // ambiguous.
    let mut path_verified = false;
    let mut method_verified = false;
    for site in &call_sites {
        let call_start = site.callee_start;
        if assert_spans
            .iter()
            .any(|&(start, end)| call_start >= start && call_start < end)
            || target_call_has_assert_method_chain(text, call_start, symbol)
        {
            if site.is_method {
                method_verified = true;
            } else {
                path_verified = true;
            }
        }
    }
    // DS-628's disclosed bound: a single `let NAME = ...<call>...;` binding,
    // where `NAME` (optionally followed by `.field`/`.method()`) later
    // appears inside an assert span as an identifier of its own (#281).
    let mut binding_uncertain = false;
    for (binding, has_path_call) in find_let_bindings_containing_call(text, symbol) {
        match binding_reach(text, &binding, assert_spans) {
            BindingReach::Reaches => {
                if has_path_call {
                    path_verified = true;
                } else {
                    method_verified = true;
                }
            }
            BindingReach::Uncertain => binding_uncertain = true,
            BindingReach::NotReached => {}
        }
    }
    if path_verified {
        return TargetCallVerification::Verified;
    }
    if binding_uncertain {
        return TargetCallVerification::BindingUncertain;
    }
    if method_verified {
        // #279: a method call is matched by name only, so a verified method call
        // is evidence for the target only when no function call of the target
        // is left unverified beside it. Otherwise the verifying device may
        // belong to another type's same-named method, and the function call's
        // result is neither shown verified nor shown unverified (DS-1680).
        // A name in a macro that is not read may be a function call (#282).
        return if macro_use.opaque || call_sites.iter().any(|site| !site.is_method) {
            TargetCallVerification::Unidentified
        } else {
            TargetCallVerification::Verified
        };
    }
    if macro_use.opaque {
        return TargetCallVerification::MacroOpaque;
    }
    let loop_bodies = loop_body_spans(text);
    if call_sites.iter().any(|site| {
        loop_bodies
            .iter()
            .any(|&(start, end)| site.callee_start >= start && site.callee_start < end)
    }) {
        return TargetCallVerification::Ambiguous;
    }
    // A mutable-reference or global-state argument to the call is this
    // capability's disclosed ambiguity escape (DS-623's own UNKNOWN
    // example). Scoped to the call's own argument list (the parenthesized
    // span right after `symbol`), not the rest of the function text —
    // scanning past the call site would flag an unrelated `&mut ` written
    // anywhere later in the function as if it belonged to this call.
    if call_sites.iter().any(|site| {
        call_argument_list(text, site.callee_start, symbol)
            .is_some_and(|arguments| arguments.contains("&mut "))
    }) {
        return TargetCallVerification::Ambiguous;
    }
    if call_sites.iter().any(|site| {
        target_call_is_passed_to_other_function(text, site.expr_start, site.callee_start, symbol)
    }) {
        return TargetCallVerification::Ambiguous;
    }
    if call_sites.iter().all(|site| site.is_method) {
        return TargetCallVerification::Unidentified;
    }
    TargetCallVerification::Unverified
}

/// A target result consumed by one of DS-626's assertion-equivalent methods is
/// verified even when it is not wrapped in an assert macro. This is deliberately
/// limited to the method chain immediately following the target call.
fn target_call_has_assert_method_chain(text: &str, call_start: usize, symbol: &str) -> bool {
    let Some(close) = call_close_position(text, call_start, symbol) else {
        return false;
    };
    let suffix = text[close + 1..].trim_start();
    [
        ".expect(",
        ".unwrap()",
        ".expect_err(",
        ".unwrap_err()",
        "?",
    ]
    .iter()
    .any(|token| suffix.starts_with(token))
}

fn call_close_position(text: &str, call_start: usize, symbol: &str) -> Option<usize> {
    let open = call_open_position(text, call_start, symbol)?;
    find_matching_close(text, open, '(', ')')
}

/// Finds the argument-list opener for a call whose AST span starts at the
/// beginning of its callee path. `find_call_sites` deliberately records the
/// `ExprCall` function span, so a qualified call such as `Type::from_yaml`
/// starts before the bare target symbol. Trim the callee path before matching
/// its final segment, then use the same opener for all call-site consumers.
fn call_open_position(text: &str, call_start: usize, symbol: &str) -> Option<usize> {
    let after = &text[call_start..];
    let open_offset = after.find('(')?;
    let callee = after[..open_offset].trim_end();
    callee.ends_with(symbol).then_some(call_start + open_offset)
}

/// Byte spans of every `for` / `while` / `loop` body in the function, found
/// from the syntax tree (not by searching the text, which would also match
/// the keywords inside string literals and identifiers such as
/// `default_for`). Iteration and control flow can determine whether every
/// result reaches a verifier, so a call inside one of these bodies is
/// outside this bounded analysis.
fn loop_body_spans(text: &str) -> Vec<(usize, usize)> {
    let Ok(function) = syn::parse_str::<syn::ItemFn>(text) else {
        return Vec::new();
    };
    let mut visitor = LoopBodyVisitor {
        text,
        spans: Vec::new(),
    };
    visitor.visit_item_fn(&function);
    visitor.spans
}

struct LoopBodyVisitor<'a> {
    text: &'a str,
    spans: Vec<(usize, usize)>,
}

impl LoopBodyVisitor<'_> {
    fn record(&mut self, body: &syn::Block) {
        let span = body.brace_token.span.join();
        self.spans.push((
            span_offset(span.start(), self.text),
            span_offset(span.end(), self.text),
        ));
    }
}

impl<'ast> Visit<'ast> for LoopBodyVisitor<'_> {
    fn visit_expr_for_loop(&mut self, node: &'ast syn::ExprForLoop) {
        self.record(&node.body);
        syn::visit::visit_expr_for_loop(self, node);
    }

    fn visit_expr_while(&mut self, node: &'ast syn::ExprWhile) {
        self.record(&node.body);
        syn::visit::visit_expr_while(self, node);
    }

    fn visit_expr_loop(&mut self, node: &'ast syn::ExprLoop) {
        self.record(&node.body);
        syn::visit::visit_expr_loop(self, node);
    }
}

/// Passing a target result to an ordinary function is a delegation/data-flow
/// shape this adapter cannot resolve, so DS-1680 requires UNKNOWN.
fn target_call_is_passed_to_other_function(
    text: &str,
    expr_start: usize,
    callee_start: usize,
    symbol: &str,
) -> bool {
    let Some(close) = call_close_position(text, callee_start, symbol) else {
        return false;
    };
    let before = text[..expr_start].trim_end();
    before.ends_with('(')
        || before.ends_with(',')
        || text[close + 1..].trim_start().starts_with(",")
}

/// Returns the argument-list text (inside the parens) of a call to `symbol`
/// whose name starts at byte offset `call_start` in `text`.
fn call_argument_list<'a>(text: &'a str, call_start: usize, symbol: &str) -> Option<&'a str> {
    let open_pos = call_open_position(text, call_start, symbol)?;
    let close_pos = find_matching_close(text, open_pos, '(', ')')?;
    Some(&text[open_pos + 1..close_pos])
}

fn find_call_sites(text: &str, symbol: &str) -> Vec<CallSite> {
    let Ok(function) = syn::parse_str::<syn::ItemFn>(text) else {
        return Vec::new();
    };
    let mut visitor = CallSiteVisitor {
        text,
        symbol,
        sites: Vec::new(),
    };
    visitor.visit_item_fn(&function);
    visitor.sites
}

struct CallSiteVisitor<'a> {
    text: &'a str,
    symbol: &'a str,
    sites: Vec<CallSite>,
}

impl<'ast> Visit<'ast> for CallSiteVisitor<'_> {
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = node.func.as_ref() {
            if path
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == self.symbol)
            {
                let start = span_offset(node.func.span().start(), self.text);
                self.sites.push(CallSite {
                    callee_start: start,
                    expr_start: start,
                    is_method: false,
                });
            }
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if node.method == self.symbol {
            self.sites.push(CallSite {
                callee_start: span_offset(node.method.span().start(), self.text),
                expr_start: span_offset(node.span().start(), self.text),
                is_method: true,
            });
        }
        syn::visit::visit_expr_method_call(self, node);
    }
}

/// Names bound by `let NAME = ...<call>...;`, each with whether the initializer
/// holds a function call of `symbol` (not only a method call).
fn find_let_bindings_containing_call(text: &str, symbol: &str) -> Vec<(String, bool)> {
    let Ok(function) = syn::parse_str::<syn::ItemFn>(text) else {
        return Vec::new();
    };
    let mut visitor = LetBindingVisitor {
        text,
        symbol,
        bindings: Vec::new(),
    };
    visitor.visit_item_fn(&function);
    visitor.bindings
}

struct LetBindingVisitor<'a> {
    text: &'a str,
    symbol: &'a str,
    bindings: Vec<(String, bool)>,
}

impl<'ast> Visit<'ast> for LetBindingVisitor<'_> {
    fn visit_local(&mut self, node: &'ast syn::Local) {
        if let (syn::Pat::Ident(pattern), Some(init)) = (&node.pat, &node.init) {
            let mut calls = CallSiteVisitor {
                text: self.text,
                symbol: self.symbol,
                sites: Vec::new(),
            };
            calls.visit_expr(init.expr.as_ref());
            if !calls.sites.is_empty() {
                let has_path_call = calls.sites.iter().any(|site| !site.is_method);
                self.bindings
                    .push((pattern.ident.to_string(), has_path_call));
            }
        }
        syn::visit::visit_local(self, node);
    }
}

// ---------------------------------------------------------------------------
// DA-004 (DS-624): self comparison
// ---------------------------------------------------------------------------

fn da_004_self_comparison(text: &str) -> DaVerdict {
    let spans = find_macro_call_spans(text, "assert_eq!");
    for (start, end) in spans {
        let arguments = &text[start..end];
        if let Some((left, right)) = split_top_level_comma(arguments) {
            if normalize_tokens(left) == normalize_tokens(right)
                && !normalize_tokens(left).is_empty()
            {
                return DaVerdict::Fail("assert_eq!'s two arguments are token-identical (DS-624)");
            }
        }
    }
    DaVerdict::NoViolation
}

fn normalize_tokens(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn split_top_level_comma(arguments: &str) -> Option<(&str, &str)> {
    let mut depth: i32 = 0;
    let mut in_string = false;
    for (index, ch) in arguments.char_indices() {
        match ch {
            '"' => in_string = !in_string,
            '(' | '[' | '{' if !in_string => depth += 1,
            ')' | ']' | '}' if !in_string => depth -= 1,
            ',' if !in_string && depth == 0 => {
                return Some((&arguments[..index], &arguments[index + 1..]));
            }
            _ => {}
        }
    }
    None
}

// ---------------------------------------------------------------------------
// DA-005 (DS-625): empty test
// ---------------------------------------------------------------------------

fn da_005_empty_test(text: &str) -> DaVerdict {
    if syn::parse_str::<syn::ItemFn>(text)
        .ok()
        .is_some_and(|function| function.block.stmts.is_empty())
    {
        DaVerdict::Fail("the Test function body has no statements (DS-625)")
    } else {
        DaVerdict::NoViolation
    }
}

struct RustFunctionSyntax {
    function: syn::ItemFn,
}

impl RustFunctionSyntax {
    fn parse(text: &str) -> Option<Self> {
        syn::parse_str(text).ok().map(|function| Self { function })
    }

    fn has_should_panic(&self) -> bool {
        self.function
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("should_panic"))
    }

    fn has_assert_equivalent_method_or_try(&self) -> bool {
        let mut visitor = VerificationSyntaxVisitor {
            has_verifier: false,
        };
        visitor.visit_item_fn(&self.function);
        visitor.has_verifier
    }

    /// `true` when the function calls a function or a method anywhere in
    /// its body. In a body without an assert-equivalent construct, such a
    /// call is a delegation target of the Test (DS-642).
    fn has_call(&self) -> bool {
        let mut visitor = CallVisitor { has_call: false };
        visitor.visit_block(&self.function.block);
        visitor.has_call
    }

    fn returns_result(&self) -> bool {
        match &self.function.sig.output {
            syn::ReturnType::Type(_, ty) => match ty.as_ref() {
                syn::Type::Path(path) => path
                    .path
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == "Result"),
                _ => false,
            },
            syn::ReturnType::Default => false,
        }
    }
}

struct CallVisitor {
    has_call: bool,
}

impl<'ast> Visit<'ast> for CallVisitor {
    fn visit_expr_call(&mut self, _node: &'ast syn::ExprCall) {
        self.has_call = true;
    }

    fn visit_expr_method_call(&mut self, _node: &'ast syn::ExprMethodCall) {
        self.has_call = true;
    }
}

struct VerificationSyntaxVisitor {
    has_verifier: bool,
}

impl<'ast> Visit<'ast> for VerificationSyntaxVisitor {
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if matches!(
            node.method.to_string().as_str(),
            "unwrap" | "expect" | "unwrap_err" | "expect_err"
        ) {
            self.has_verifier = true;
        }
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_try(&mut self, _node: &'ast syn::ExprTry) {
        self.has_verifier = true;
    }
}

// ---------------------------------------------------------------------------
// DA-006 (DS-626): no verification syntax
// ---------------------------------------------------------------------------

fn da_006_no_verification_syntax(has_verification_syntax: bool, has_call: bool) -> DaVerdict {
    if has_verification_syntax {
        DaVerdict::NoViolation
    } else if has_call {
        // DS-642: with no assert-equivalent construct, the calls in the body
        // are the Test's delegation targets. This capability never confirms
        // that one terminates (module doc), so the result is neither a
        // violation nor its absence (DS-626, DS-652, DS-1345).
        DaVerdict::Unknown(
            "no assert-equivalent construct is present and the body delegates to a function \
             call whose termination (§7.2.1) this analysis does not confirm (DS-626, DS-652)"
                .to_owned(),
        )
    } else {
        DaVerdict::Fail(
            "no assert-equivalent construct (standard macro, #[should_panic], \
             .unwrap()/.expect()/.expect_err()/.unwrap_err()/?, or a Result-returning signature) is present, \
             and no §7.2.1 delegation target is identified (DS-626)",
        )
    }
}

// ---------------------------------------------------------------------------
// Shared lexical helpers
// ---------------------------------------------------------------------------

/// Finds every occurrence of any macro in `macros` and returns the byte
/// range of its argument-list contents (inside the outermost matching
/// parens/brackets/braces the macro invocation uses).
fn find_assert_spans(text: &str, macros: &[&str]) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    for macro_name in macros {
        spans.extend(find_macro_call_spans(text, macro_name));
    }
    spans.sort_by_key(|&(start, _)| start);
    spans
}

fn find_macro_call_spans(text: &str, macro_name: &str) -> Vec<(usize, usize)> {
    let Ok(function) = syn::parse_str::<syn::ItemFn>(text) else {
        return Vec::new();
    };
    let mut visitor = MacroVisitor {
        text,
        macro_name,
        spans: Vec::new(),
    };
    syn::visit::Visit::visit_item_fn(&mut visitor, &function);
    visitor.spans
}

struct MacroVisitor<'a> {
    text: &'a str,
    macro_name: &'a str,
    spans: Vec<(usize, usize)>,
}

impl<'ast> syn::visit::Visit<'ast> for MacroVisitor<'_> {
    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if node
            .path
            .segments
            .last()
            .is_some_and(|segment| format!("{}!", segment.ident) == self.macro_name)
        {
            let start = span_offset(node.span().start(), self.text);
            let end = span_offset(node.span().end(), self.text);
            let invocation = &self.text[start..end];
            if let Some(open) = invocation.find(['(', '[', '{']) {
                if let Some(close) = find_matching_close(
                    invocation,
                    open,
                    invocation.as_bytes()[open] as char,
                    match invocation.as_bytes()[open] as char {
                        '(' => ')',
                        '[' => ']',
                        '{' => '}',
                        _ => return,
                    },
                ) {
                    self.spans.push((start + open + 1, start + close));
                }
            }
        }
        syn::visit::visit_macro(self, node);
    }
}

/// Byte offset of a `proc_macro2` position in `text`. The column counts
/// characters, so it goes through the crate's character-aware conversion:
/// adding it to the line start as bytes lands inside a multi-byte character
/// and the slice panics (#264). A position outside `text` clamps to its end.
fn span_offset(location: proc_macro2::LineColumn, text: &str) -> usize {
    crate::line_column_byte_offset(
        text,
        &crate::line_offsets(text),
        location.line,
        location.column,
    )
    .unwrap_or(text.len())
}

fn find_matching_close(text: &str, open_pos: usize, open: char, close: char) -> Option<usize> {
    let mut depth: i32 = 0;
    let mut in_string = false;
    for (index, ch) in text[open_pos..].char_indices() {
        let absolute = open_pos + index;
        match ch {
            '"' => in_string = !in_string,
            c if !in_string && c == open => depth += 1,
            c if !in_string && c == close => {
                depth -= 1;
                if depth == 0 {
                    return Some(absolute);
                }
            }
            _ => {}
        }
    }
    None
}

/// Diagnostic code this capability's caller uses when it cannot run this
/// analysis at all (no `fn` signature/body recognizable in the construct
/// text) — DS-615's "adapterが不完全…を報告した場合" fail-closed escape.
pub fn unanalyzable_diagnostic(reason: &str) -> Diagnostic {
    Diagnostic::warning(
        "W-DA-102",
        format!("oracle_presence static analysis could not run: {reason}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn locator(value: &str) -> Locator {
        Locator {
            adapter: vtest_model::AdapterId::new("rust-cargo"),
            value: value.to_owned(),
        }
    }

    /// @vtest.id TEST-ORACLE-TARGET-SYMBOL-FINAL-SEGMENT
    /// @vtest.covers VO-ORACLE-TARGET-SYMBOL-FINAL-SEGMENT
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::target_symbol
    /// @vtest.intent target候補名としてitem-pathの末尾segmentを抽出することを確認する
    #[test]
    fn target_symbol_takes_the_final_segment() {
        assert_eq!(
            target_symbol(&locator("src/lib.rs::double")),
            "double".to_owned()
        );
    }

    /// @vtest.id TEST-ORACLE-COMPOSE-ALL-NO-VIOLATION-PASSES
    /// @vtest.covers VO-ORACLE-COMPOSE-PASS
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::compose
    /// @vtest.intent DA-001/003/004/005/006の全ルールが違反なしのときcomposeがFAILでもUNKNOWNでもないことを確認する
    #[test]
    fn a_direct_assert_eq_call_passes_all_five_rules() {
        let text = "#[test]\nfn it_doubles() {\n    assert_eq!(double(2), 4);\n}\n";
        let analysis = analyze(text, &["double".to_owned()], &[]);
        let composed = compose(&analysis);
        assert!(!composed.0, "{:?}", composed.2);
        assert!(!composed.1, "{:?}", composed.2);
    }

    /// @vtest.id TEST-ORACLE-DA-005-EMPTY-BODY-FAILS
    /// @vtest.covers VO-ORACLE-DA-005-EMPTY-TEST
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_005_empty_test
    /// @vtest.intent 文を含まない関数本体がDA-005のFAILになることを確認する
    #[test]
    fn an_empty_test_fails_da_005() {
        let text = "#[test]\nfn empty() {\n}\n";
        let analysis = analyze(text, &[], &[]);
        assert!(matches!(analysis.da_005, DaVerdict::Fail(_)));
        let (is_fail, _, _) = compose(&analysis);
        assert!(is_fail);
    }

    /// @vtest.id TEST-ORACLE-DA-006-NO-ASSERT-FAILS
    /// @vtest.covers VO-ORACLE-DA-006-NO-VERIFICATION-SYNTAX
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_006_no_verification_syntax
    /// @vtest.intent 検証構文が1つも無い関数本体がDA-006のFAILになることを確認する
    #[test]
    fn no_verification_syntax_fails_da_006() {
        let text = "#[test]\nfn calls_but_checks_nothing() {\n    let _ = 1 + 1;\n}\n";
        let analysis = analyze(text, &[], &[]);
        assert!(matches!(analysis.da_006, DaVerdict::Fail(_)));
    }

    /// @vtest.id TEST-ORACLE-DA-006-DELEGATING-BODY-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-006-DELEGATION-NOT-FAIL
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_006_no_verification_syntax
    /// @vtest.intent a Test body with no assert-equivalent construct that calls a function or a method gets DA-006 Unknown, neither Fail nor NoViolation, and the composed verdict is Unknown, while a body with a call only inside a string stays Fail (DS-1345)
    #[test]
    fn a_body_that_only_delegates_gets_da_006_unknown_not_fail() {
        for text in [
            "#[test]\nfn delegates() {\n    check_all(1, 2);\n}\n",
            "#[test]\nfn delegates() {\n    let run = |x: u8| helper.compare(x);\n    let _ = run;\n}\n",
        ] {
            let analysis = analyze(text, &[], &[]);
            assert!(matches!(analysis.da_006, DaVerdict::Unknown(_)), "{text}");
            let (is_fail, is_unknown, _) = compose(&analysis);
            assert!(!is_fail && is_unknown, "{text}");
        }

        let no_call = "#[test]\nfn mentions() {\n    let _ = \"check_all(1, 2)\";\n}\n";
        assert!(matches!(
            analyze(no_call, &[], &[]).da_006,
            DaVerdict::Fail(_)
        ));
    }

    /// @vtest.id TEST-ORACLE-TEXT-LITERAL-IS-NOT-VERIFICATION-SYNTAX
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED, VO-ORACLE-DA-006-NO-VERIFICATION-SYNTAX
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::analyze
    /// @vtest.intent 文字列literal内のtarget呼出しとunwrap表記を実行call・検証構文として数えないことを確認する
    #[test]
    fn text_literal_target_and_unwrap_are_not_verification_syntax() {
        let text = "#[test]\nfn mentions_only() {\n    let _ = \"parse(input).unwrap()\";\n}\n";
        let analysis = analyze(text, &["parse".to_owned()], &[]);
        assert!(matches!(analysis.da_003, DaVerdict::NoViolation));
        assert!(matches!(analysis.da_006, DaVerdict::Fail(_)));
    }

    /// @vtest.id TEST-ORACLE-DA-006-SHOULD-PANIC-PASSES
    /// @vtest.covers VO-ORACLE-DA-006-NO-VERIFICATION-SYNTAX
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_006_no_verification_syntax
    /// @vtest.intent #[should_panic]属性が検証構文として認められDA-006が違反なしになることを確認する
    #[test]
    fn a_should_panic_test_with_no_target_call_passes_da_006_via_the_attribute() {
        let text = "#[should_panic]\n#[test]\nfn panics() {\n    panic!(\"boom\");\n}\n";
        let analysis = analyze(text, &[], &[]);
        assert!(matches!(analysis.da_006, DaVerdict::NoViolation));
    }

    /// REQ-074「不成立が構造から証明できる場合のみFAIL」: no
    /// assert-equivalent construct exists anywhere in the function, so no
    /// path — tracked or not — could carry the call's result to one. This
    /// is the one case this capability can prove structurally.
    /// @vtest.id TEST-ORACLE-DA-003-NO-ASSERT-ANYWHERE-FAILS
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent target呼出結果を検証するassert相当が関数内に一つも無い場合にDA-003がFAILになることを確認する
    #[test]
    fn a_target_call_with_no_assert_construct_anywhere_fails_da_003() {
        let text = "#[test]\nfn calls_but_asserts_nothing() {\n    double(2);\n}\n";
        let analysis = analyze(text, &["double".to_owned()], &[]);
        assert!(
            matches!(analysis.da_003, DaVerdict::Fail(_)),
            "{:?}",
            analysis.da_003
        );
    }

    /// DS-615「解析限界…を報告した場合はUNKNOWNとし、違反なしと推測しない」:
    /// an assert-equivalent construct exists in the function (so a real
    /// verification path is structurally possible), but this capability's
    /// bounded dataflow tracking (direct-in-assert or one `let` binding)
    /// cannot trace the call's result into it. That is an analysis limit,
    /// not proof of non-verification — `Unknown`, not `Fail`.
    /// @vtest.id TEST-ORACLE-DA-003-UNTRACEABLE-CALL-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent assertが存在するがtarget呼出結果をそこへ追跡できない場合、DA-003がFAILではなくUNKNOWNになることを確認する
    #[test]
    fn a_target_call_untraceable_into_an_unrelated_assert_is_unknown_not_fail() {
        let text = "#[test]\nfn calls_but_ignores_result() {\n    double(2);\n    assert!(true == false || true);\n}\n";
        let analysis = analyze(text, &["double".to_owned()], &[]);
        assert!(
            matches!(analysis.da_003, DaVerdict::Unknown(_)),
            "{:?}",
            analysis.da_003
        );
    }

    /// @vtest.id TEST-ORACLE-DA-003-SHOULD-PANIC-PASSES
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent #[should_panic]があるときassertが無くてもDA-003が違反なしになることを確認する
    #[test]
    fn a_should_panic_target_call_passes_da_003_without_an_assert() {
        let text = "#[should_panic]\n#[test]\nfn panics_on_call() {\n    divide(1, 0);\n}\n";
        let analysis = analyze(text, &["divide".to_owned()], &[]);
        assert!(matches!(analysis.da_003, DaVerdict::NoViolation));
    }

    /// @vtest.id TEST-ORACLE-DA-003-LET-BOUND-CALL-PASSES
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent target呼出結果がlet束縛経由でassertへ到達する場合にDA-003が違反なしになることを確認する
    #[test]
    fn a_let_bound_call_used_in_an_assert_passes_da_003() {
        let text = "#[test]\nfn binds_then_asserts() {\n    let result = double(2);\n    assert_eq!(result, 4);\n}\n";
        let analysis = analyze(text, &["double".to_owned()], &[]);
        assert!(
            matches!(analysis.da_003, DaVerdict::NoViolation),
            "{:?}",
            analysis.da_003
        );
    }

    /// @vtest.id TEST-ORACLE-DA-003-DIRECT-EXPECT-ERR-PASSES
    /// @vtest.covers VO-ORACLE-DA-003-DIRECT-ASSERT-METHOD,VO-ORACLE-DA-006-NO-VERIFICATION-SYNTAX
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent target呼出結果へ直接expectを連鎖した場合に到達と判定することを確認する
    #[test]
    fn a_direct_expect_err_call_passes_da_003_and_da_006() {
        let text = "#[test]\nfn rejects() {\n    parse(input).expect_err(\"invalid\");\n}\n";
        let analysis = analyze(text, &["parse".to_owned()], &[]);
        assert!(matches!(analysis.da_003, DaVerdict::NoViolation));
        assert!(matches!(analysis.da_006, DaVerdict::NoViolation));
    }

    /// @vtest.id TEST-ORACLE-DA-003-PATH-QUALIFIED-EXPECT-ERR-PASSES
    /// @vtest.covers VO-ORACLE-DA-003-DIRECT-ASSERT-METHOD,VO-ORACLE-DA-006-NO-VERIFICATION-SYNTAX
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent パス付きtarget呼出しへexpect_errを連鎖した場合にDA-003が違反なしになることを確認する
    #[test]
    fn a_path_qualified_expect_err_call_passes_da_003_and_da_006() {
        let text =
            "#[test]\nfn rejects() {\n    Type::from_yaml(input).expect_err(\"invalid\");\n}\n";
        let analysis = analyze(text, &["from_yaml".to_owned()], &[]);
        assert!(matches!(analysis.da_003, DaVerdict::NoViolation));
        assert!(matches!(analysis.da_006, DaVerdict::NoViolation));
    }

    /// @vtest.id TEST-ORACLE-DA-003-DIRECT-EXPECT-PASSES
    /// @vtest.covers VO-ORACLE-DA-003-DIRECT-ASSERT-METHOD,VO-ORACLE-DA-006-NO-VERIFICATION-SYNTAX
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent target呼出結果へ直接expectを連鎖した場合に到達と判定することを確認する
    #[test]
    fn a_direct_expect_call_passes_da_003_and_da_006() {
        let text = "#[test]\nfn accepts() {\n    parse(input).expect(\"valid\");\n}\n";
        let analysis = analyze(text, &["parse".to_owned()], &[]);
        assert!(matches!(analysis.da_003, DaVerdict::NoViolation));
        assert!(matches!(analysis.da_006, DaVerdict::NoViolation));
    }

    /// @vtest.id TEST-ORACLE-DA-003-LOOP-CALL-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent loop内のtarget呼出はbounded解析でUNKNOWNに退避することを確認する
    #[test]
    fn a_target_call_inside_a_loop_is_unknown() {
        let text = "#[test]\nfn repeated() {\n    for item in inputs {\n        parse(item);\n    }\n    assert!(true);\n}\n";
        let analysis = analyze(text, &["parse".to_owned()], &[]);
        assert!(matches!(analysis.da_003, DaVerdict::Unknown(_)));
    }

    /// @vtest.id TEST-ORACLE-DA-003-LOOP-CALL-WITH-ASSERT-CHAIN
    /// @vtest.covers VO-ORACLE-DA-003-DIRECT-ASSERT-METHOD
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent a loop call whose result goes through an assert-equivalent method or a let binding that is asserted, or a later directly verified call, makes DA-003 no violation regardless of call order
    #[test]
    fn a_verified_call_is_no_violation_even_when_another_call_is_in_a_loop() {
        for text in [
            "#[test]\nfn chained() {\n    for item in inputs {\n        parse(item).expect(\"valid\");\n    }\n}\n",
            "#[test]\nfn bound() {\n    for item in inputs {\n        let parsed = parse(item);\n        assert!(parsed.is_ok());\n    }\n}\n",
            "#[test]\nfn later() {\n    for item in inputs {\n        let _ = parse(item);\n    }\n    parse(\"x\").unwrap();\n}\n",
        ] {
            let analysis = analyze(text, &["parse".to_owned()], &[]);
            assert!(matches!(analysis.da_003, DaVerdict::NoViolation), "{text}");
        }
    }

    /// @vtest.id TEST-ORACLE-DA-003-LOOP-KEYWORD-IN-STRING
    /// @vtest.covers VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::loop_body_spans
    /// @vtest.intent only real for / while / loop bodies count as loops; the keyword inside a string literal or an identifier such as default_for does not
    #[test]
    fn a_loop_keyword_in_a_string_or_identifier_is_not_a_loop() {
        let text = "#[test]\nfn plain() {\n    let default_for = 1;\n    assert!(default_for == 1, \"for {}\", 1);\n    parse(input);\n}\n";
        assert!(loop_body_spans(text).is_empty());
        let looped =
            "#[test]\nfn looped() {\n    for item in inputs {\n        parse(item);\n    }\n}\n";
        assert_eq!(loop_body_spans(looped).len(), 1);
    }

    /// @vtest.id TEST-ORACLE-DA-003-DELEGATED-CALL-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent target結果を別関数へ渡す形は委譲先を追跡せずUNKNOWNにすることを確認する
    #[test]
    fn a_target_call_passed_to_another_function_is_unknown() {
        let text = "#[test]\nfn delegated() {\n    check(parse(input));\n}\n";
        let analysis = analyze(text, &["parse".to_owned()], &[]);
        assert!(matches!(analysis.da_003, DaVerdict::Unknown(_)));
    }

    /// DS-642: a body with no assert-equivalent construct delegates to every
    /// non-target call in it; the delegation target's termination is not
    /// confirmed (DS-640, DS-652), so DA-003 is UNKNOWN, not FAIL.
    /// @vtest.id TEST-ORACLE-DA-003-RESULT-FLOWING-TO-OTHER-CALL-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-DELEGATION-NOT-FAIL
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent assert macroが無く、target以外の関数へ結果が流れる本体(#277の実物の形で.expect付きを含む)はDA-003がFAILではなくUNKNOWNになり、basisが委譲(DS-642、DS-652)を理由に挙げることを確認する
    #[test]
    fn a_result_flowing_into_a_non_target_call_without_assert_is_unknown() {
        for text in [
            "#[test]\nfn real() {\n    let data = build(&scan);\n    let mut json = to_json(&data).expect(\"serialize data\");\n    json.push('\\n');\n    golden(\"data.json\", &json);\n}\n",
            "#[test]\nfn flows() {\n    let data = build(&scan);\n    let json = to_json(&data);\n    golden(\"a\", &json);\n}\n",
            "#[test]\nfn method_flows() {\n    let data = build(&scan);\n    data.render();\n}\n",
        ] {
            let analysis = analyze(text, &["build".to_owned()], &[]);
            let DaVerdict::Unknown(basis) = &analysis.da_003 else {
                panic!("{text}: {:?}", analysis.da_003);
            };
            assert!(
                basis.contains("DS-642") && basis.contains("DS-652") && basis.contains("delegation"),
                "{basis}"
            );
        }
    }

    /// The target's own call is not a delegation target (a derivation: DS-642
    /// does not exclude it), so a discarded result stays FAIL.
    /// @vtest.id TEST-ORACLE-DA-003-DISCARDED-RESULT-STAYS-FAIL
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent target呼出だけで結果を捨てる本体(double(2); 、let _ = double(2); 、let _ = Type::double(2); の3形)はDA-003がFAILのままであることを確認する
    #[test]
    fn a_discarded_target_result_with_no_other_call_stays_fail() {
        for text in [
            "#[test]\nfn bare() {\n    double(2);\n}\n",
            "#[test]\nfn discarded() {\n    let _ = double(2);\n}\n",
            "#[test]\nfn qualified() {\n    let _ = Type::double(2);\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::Fail(_)),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// #279: a target called as a method (`c.double(2)`) is a `syn::ExprMethodCall`,
    /// which DA-003 did not read as a call, so a discarded result beside an assert
    /// on something else composed to PASS. The composed verdict must not be PASS.
    /// @vtest.id TEST-ORACLE-DA-003-METHOD-TARGET-DISCARDED-IS-NOT-PASS
    /// @vtest.covers VO-ORACLE-DA-003-UNIDENTIFIED-CALL-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent target をメソッドとして呼び結果を捨て、target と無関係な assert_eq! が本体にある Test は、DA-001〜006 の合成結果が PASS にならない(UNKNOWN)ことを確認する
    #[test]
    fn a_method_target_whose_result_is_discarded_beside_an_unrelated_assert_is_not_pass() {
        for text in [
            "#[test]\nfn t() {\n    let c = Calc::default();\n    c.double(2);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let c = Calc::default();\n    let _ = c.double(2);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    Calc::default().double(2);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let c = Calc::default();\n    c.double(2);\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(is_fail || is_unknown, "{text}: composed to PASS: {basis:?}");
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A method call whose result reaches an assert-equivalent construct is still
    /// no violation: the method-call reading only moves NoViolation toward
    /// FAIL/UNKNOWN, never the reverse, and does not turn verified calls UNKNOWN.
    /// @vtest.id TEST-ORACLE-DA-003-METHOD-TARGET-VERIFIED-STAYS-NO-VIOLATION
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent メソッドとして呼んだ target の結果が let 束縛経由の assert、.unwrap()、.expect()、? の連鎖に到達し、関数呼出が混ざらない Test は、DA-003 が違反なしのままであることを確認する
    #[test]
    fn a_method_target_whose_result_reaches_an_assert_is_no_violation() {
        for text in [
            "#[test]\nfn t() {\n    let r = c.double(2);\n    assert_eq!(r, 4);\n}\n",
            "#[test]\nfn t() {\n    c.double(2).unwrap();\n}\n",
            "#[test]\nfn t() {\n    Calc::default().double(2).expect(\"ok\");\n}\n",
            "#[test]\nfn t() -> Result<(), E> {\n    c.double(2)?;\n    Ok(())\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::NoViolation),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// B1: a verified method call is matched to the target by name only, so it
    /// cannot stand in for a function call of the target that stays unverified.
    /// The composed verdict must not be PASS for any of these.
    /// @vtest.id TEST-ORACLE-DA-003-VERIFIED-METHOD-CANNOT-CLEAR-UNVERIFIED-CALL
    /// @vtest.covers VO-ORACLE-DA-003-UNIDENTIFIED-CALL-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent 検証済みの根拠がメソッド呼出だけで、検証されていない関数呼出が別に残る本体(.unwrap() の連鎖、assert と併存、let 束縛経由)は、DA-001〜006 の合成結果が PASS にならないことを確認する
    #[test]
    fn a_verified_method_call_does_not_clear_an_unverified_function_call() {
        for text in [
            "#[test]\nfn t() {\n    double(2);\n    o.double(3).unwrap();\n}\n",
            "#[test]\nfn t() {\n    double(2);\n    o.double(3).unwrap();\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    double(2);\n    let r = o.double(3);\n    assert_eq!(r, 6);\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(is_fail || is_unknown, "{text}: composed to PASS: {basis:?}");
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A method call is identified by name only (the receiver's type is not
    /// resolved), so a call to a same-named method of another type cannot be told
    /// from the target: the discard is neither proven a violation (FAIL) nor
    /// absent. A path call keeps its FAIL.
    /// @vtest.id TEST-ORACLE-DA-003-METHOD-CALL-IS-IDENTIFIED-BY-NAME-ONLY
    /// @vtest.covers VO-ORACLE-DA-003-UNIDENTIFIED-CALL-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent メソッド呼出は名前だけで target と同定できないため、結果を捨てる形は FAIL でも違反なしでもなく UNKNOWN になり、関数呼出の形は FAIL のままであることを確認する
    #[test]
    fn a_method_call_matched_by_name_only_is_unknown_not_fail() {
        let method = "#[test]\nfn t() {\n    other.double(2);\n}\n";
        let analysis = analyze(method, &["double".to_owned()], &[]);
        assert!(
            matches!(analysis.da_003, DaVerdict::Unknown(_)),
            "{:?}",
            analysis.da_003
        );
        let path = "#[test]\nfn t() {\n    Other::double(2);\n}\n";
        let analysis = analyze(path, &["double".to_owned()], &[]);
        assert!(
            matches!(analysis.da_003, DaVerdict::Fail(_)),
            "{:?}",
            analysis.da_003
        );
    }

    /// #281: a let binding reaches an assert only when its name appears there as
    /// an identifier of its own. `r` inside `other` does not reach.
    /// @vtest.id TEST-ORACLE-DA-003-BINDING-NAME-INSIDE-ANOTHER-IDENTIFIER-IS-NOT-PASS
    /// @vtest.covers VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent let 束縛名が assert 内の別の識別子(other)の一部にすぎない Test は、結果が assert に到達したと読まれず、DA-001〜006 の合成結果が PASS にならないことを確認する
    #[test]
    fn a_binding_name_inside_another_identifier_does_not_reach_the_assert() {
        for text in [
            "#[test]\nfn t() {\n    let r = double(2);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let c = Calc::default();\n    let t = (c.double(2), 1);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let r = double(2);\n    assert_eq!(x.r, 1);\n}\n",
            "#[test]\nfn t() {\n    let r = double(2);\n    assert_eq!(x.r(), 1);\n}\n",
            "#[test]\nfn t() {\n    let r = double(2);\n    assert_eq!(\"r\", \"r\");\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(is_fail || is_unknown, "{text}: composed to PASS: {basis:?}");
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A binding that reaches an assert by its exact name stays no violation,
    /// through any expression shape the name can take there.
    /// @vtest.id TEST-ORACLE-DA-003-BINDING-IDENTIFIER-IN-ASSERT-STAYS-NO-VIOLATION
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent 束縛名が assert の引数に識別子としてそのまま現れる Test(比較、メソッド、フィールド、参照、メッセージ引数)は、DA-003 が違反なしのままであることを確認する
    #[test]
    fn a_binding_named_exactly_in_the_assert_is_no_violation() {
        for body in [
            "assert_eq!(r, 4);",
            "assert_eq!(4, r);",
            "assert!(r > 0);",
            "assert!(r.is_ok());",
            "assert_eq!(r.len(), 1);",
            "assert_eq!(r.field, 1);",
            "assert_eq!(&r, &4);",
            "assert_eq!(*r, 4);",
            "assert_ne!(r, 5, \"msg\");",
            "assert!(r > 0, \"got {}\", other());",
            "assert_eq!((r, 1), (4, 1));",
        ] {
            let text = format!("#[test]\nfn t() {{\n    let r = double(2);\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::NoViolation),
                "{body}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A binding made again under the same name cannot be followed to the assert
    /// by name alone: the name in the assert may belong to either binding.
    /// @vtest.id TEST-ORACLE-DA-003-SHADOWED-BINDING-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent 同じ名前の束縛を作り直す(let、closure 引数、match 腕)Test は、assert 内の名前がどの束縛を指すか決められないため、DA-001〜006 の合成結果が PASS にならず UNKNOWN になることを確認する
    #[test]
    fn a_shadowed_binding_is_unknown_not_no_violation() {
        for text in [
            "#[test]\nfn t() {\n    let r = double(2);\n    let r = 1;\n    assert_eq!(r, 1);\n}\n",
            "#[test]\nfn t() {\n    let r = double(2);\n    assert!(xs.iter().all(|r| *r > 0));\n}\n",
            "#[test]\nfn t() {\n    let r = double(2);\n    match o {\n        Some(r) => assert_eq!(r, 1),\n        None => {}\n    }\n}\n",
            "#[test]\nfn t(r: u8) {\n    let r = double(r);\n    assert_eq!(r, 1);\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(is_fail || is_unknown, "{text}: composed to PASS: {basis:?}");
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A name inside a nested macro's tokens is not parsed, so whether it is the
    /// binding cannot be told: UNKNOWN, not a reach.
    /// @vtest.id TEST-ORACLE-DA-003-BINDING-IN-NESTED-MACRO-TOKENS-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent 束縛名が assert 内の入れ子のマクロのトークンの中にしか現れない Test は、識別子を確実に判定できないため PASS にならず UNKNOWN になることを確認する
    #[test]
    fn a_binding_only_inside_nested_macro_tokens_is_unknown() {
        for body in [
            "assert!(my_check!(r));",
            "assert!(matches!(r, Some(_)) && my_check!(r));",
            "assert!(vec![r; 2].len() == 2);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    let r = double(2);\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{body}: {:?}",
                analysis.da_003
            );
        }
    }

    /// The expression macros `matches!`, `format!`, `vec!` nested in an assert are
    /// read as expressions, so the binding named there reaches the assert; a
    /// binding that is not named in an expression position does not.
    /// @vtest.id TEST-ORACLE-DA-003-BINDING-IN-EXPRESSION-MACRO-IS-READ
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED, VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent assert 内の matches!/format!/vec! の式位置に束縛名がある Test は違反なしのままであり、束縛名が現れない、または matches! のパターン側にしか現れない Test は PASS にならないことを確認する
    #[test]
    fn a_binding_in_an_expression_macro_inside_an_assert_is_read_as_an_expression() {
        for body in [
            "assert!(matches!(r, Some(_)));",
            "assert!(matches!(r.get(), Some(1)));",
            "assert!(matches!(r, Some(x) if x > 0));",
            "assert!(format!(\"{}\", r) == \"4\");",
            "assert_eq!(vec![r], vec![4]);",
            "assert_eq!(vec![r, 1], other());",
        ] {
            let text = format!("#[test]\nfn t() {{\n    let r = double(2);\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::NoViolation),
                "{body}: {:?}",
                analysis.da_003
            );
        }
        for body in [
            "assert!(matches!(other(), Some(_)));",
            "assert!(matches!(other(), Some(r)));",
            "assert!(format!(\"{}\", other()) == \"4\");",
            "assert_eq!(vec![other()], vec![4]);",
            "assert!(matches!(other(), Some(_)), \"{}\", my_check!(r));",
        ] {
            let text = format!("#[test]\nfn t() {{\n    let r = double(2);\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(is_fail || is_unknown, "{body}: composed to PASS: {basis:?}");
        }
    }

    /// A binding that appears only in the message arguments of an assert, or as the
    /// left side of a named format argument, verifies nothing: it is printed on
    /// failure only.
    /// @vtest.id TEST-ORACLE-DA-003-BINDING-IN-ASSERT-MESSAGE-IS-NOT-PASS
    /// @vtest.covers VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent 束縛名が assert のメッセージ引数、または名前付き書式引数の左辺にしか現れない Test は、結果が検証されたと読まれず、DA-001〜006 の合成結果が PASS にならないことを確認する
    #[test]
    fn a_binding_only_in_an_assert_message_does_not_reach_the_assert() {
        for body in [
            "assert!(other() > 0, \"got {}\", r);",
            "assert!(other() > 0, \"{r}\", r = other());",
            "assert_eq!(other(), 1, \"{r}\", r = 5);",
            "assert_eq!(other(), 1, \"{}\", r);",
            "assert!(format!(\"{r}\", r = other()) == \"4\");",
            "assert!(matches!(other(), Some(_)), \"{}\", r);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    let r = double(2);\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(is_fail || is_unknown, "{body}: composed to PASS: {basis:?}");
        }
    }

    /// A binding assigned again is a new value under the same name, like a second
    /// `let`: the name in the assert cannot be told to mean the target's result.
    /// @vtest.id TEST-ORACLE-DA-003-REASSIGNED-BINDING-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent 束縛を同じ名前で再代入する Test(文での代入、複合代入、assert 内の代入)は、assert 内の名前が target の結果を指すと決められず、PASS にならず UNKNOWN になることを確認する
    #[test]
    fn a_reassigned_binding_is_unknown_not_no_violation() {
        for body in [
            "let mut r = double(2); r = other(); assert_eq!(r, 1);",
            "let mut r = double(2); r += 1; assert_eq!(r, 1);",
            "let mut r = double(2); assert!({ r = other(); r == 1 });",
            "let mut r = double(2); assert!({ r += 1; r == 1 });",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{body}: {:?}",
                analysis.da_003
            );
        }
    }

    /// An `assertion_macros` entry of the config has a grammar vtest does not know,
    /// so a binding named in it cannot be read as a checked expression; the same
    /// shape in a built-in assert is read.
    /// @vtest.id TEST-ORACLE-DA-003-CONFIG-ASSERT-MACRO-ARGUMENTS-ARE-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent config の assertion_macros に挙げたマクロの引数に束縛名がある Test は、引数の文法が分からないため UNKNOWN になり、組込の assert では同じ形が違反なしのままであることを確認する
    #[test]
    fn a_binding_in_a_config_assertion_macro_is_unknown() {
        let config = ["assert_matches!".to_owned()];
        let body = |name: &str| {
            format!("#[test]\nfn t() {{\n    let r = double(2);\n    {name}!(r, Some(_));\n}}\n")
        };
        let analysis = analyze(&body("assert_matches"), &["double".to_owned()], &config);
        assert!(
            matches!(analysis.da_003, DaVerdict::Unknown(_)),
            "{:?}",
            analysis.da_003
        );
        let analysis = analyze(&body("assert_eq"), &["double".to_owned()], &config);
        assert!(
            matches!(analysis.da_003, DaVerdict::NoViolation),
            "{:?}",
            analysis.da_003
        );
    }

    /// #282: a target named in the tokens of a macro that is not an assert is not
    /// read, so the call is neither proven nor refuted: UNKNOWN, never PASS.
    /// @vtest.id TEST-ORACLE-DA-003-TARGET-IN-NON-ASSERT-MACRO-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-ANALYSIS-LIMIT-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent target の名前が assert 以外のマクロ(println!、dbg!、vec!、独自マクロ)のトークンに現れ、target と無関係な assert_eq! が本体にある Test は、DA-003 が UNKNOWN になり合成結果が PASS にならないことを確認する
    #[test]
    fn a_target_named_in_a_non_assert_macro_is_unknown_not_pass() {
        for text in [
            "#[test]\nfn t() {\n    println!(\"{}\", c.double(2));\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    println!(\"{}\", double(2));\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    dbg!(double(2));\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    dbg!(double(2));\n}\n",
            "#[test]\nfn t() {\n    let v = vec![double(2)];\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let v = vec![double(2)];\n    assert_eq!(v.len(), 1);\n}\n",
            "#[test]\nfn t() {\n    let s = format!(\"{}\", double(2));\n    assert_eq!(s, \"4\");\n}\n",
            "#[test]\nfn t() {\n    my_macro! { double(2) }\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let _ = {\n        eprintln!(\"{}\", Calc::double(2));\n    };\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    println!(\"{}\", assert_eq!(double(2), 4));\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    assert_eq!(other(), 1);\n    println!(\"{}\", double(2));\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(is_fail || is_unknown, "{text}: composed to PASS: {basis:?}");
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A call written inside a built-in assert's checked arguments is read from
    /// the arguments' expressions: it verifies the target, as a call outside a
    /// macro that reaches an assert does.
    /// @vtest.id TEST-ORACLE-DA-003-TARGET-CALL-IN-ASSERT-STAYS-NO-VIOLATION
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent assert!、assert_eq!、assert_ne! の検証される引数の中で target を呼ぶ Test(入れ子の matches!、format!、vec! を含む)は、DA-003 が違反なしのままであることを確認する
    #[test]
    fn a_target_called_in_a_checked_assert_argument_is_no_violation() {
        for text in [
            "#[test]\nfn t() {\n    assert_eq!(double(2), 4);\n}\n",
            "#[test]\nfn t() {\n    assert_ne!(Calc::double(2), 5);\n}\n",
            "#[test]\nfn t() {\n    assert!(double(2) == 4);\n}\n",
            "#[test]\nfn t() {\n    assert_eq!(4, c.double(2));\n}\n",
            "#[test]\nfn t() {\n    assert!(matches!(double(2), 4));\n}\n",
            "#[test]\nfn t() {\n    assert_eq!(vec![double(2)], vec![4]);\n}\n",
            "#[test]\nfn t() {\n    assert_eq!(format!(\"{}\", double(2)), \"4\");\n}\n",
            "#[test]\nfn t() {\n    assert_eq!(double(2), 4, \"message {}\", 1);\n}\n",
            "#[test]\nfn t() {\n    let r = double(2);\n    assert_eq!(r, 4);\n    assert_eq!(double(3), 6);\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::NoViolation),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A target named in an assert's message arguments, in an assert the argument
    /// grammar of which is not known, or in tokens that do not parse as
    /// expressions is not shown verified.
    /// @vtest.id TEST-ORACLE-DA-003-TARGET-IN-UNCHECKED-ASSERT-TOKENS-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-ANALYSIS-LIMIT-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent target の名前が assert のメッセージ引数、config の assertion_macros のマクロ、式として読めない assert の引数にだけ現れる Test は、DA-003 が UNKNOWN になることを確認する
    #[test]
    fn a_target_named_only_in_unchecked_assert_tokens_is_unknown() {
        let config = ["assert_matches!".to_owned()];
        for text in [
            "#[test]\nfn t() {\n    assert_eq!(other(), 1, \"{}\", double(2));\n}\n",
            "#[test]\nfn t() {\n    assert!(other(), \"{x}\", x = double(2));\n}\n",
            "#[test]\nfn t() {\n    assert_matches!(double(2), Some(_));\n}\n",
            "#[test]\nfn t() {\n    assert_eq!(double(2) => 4);\n}\n",
            "#[test]\nfn t() {\n    assert_eq!(my_macro!(double(2)), 4);\n}\n",
            "#[test]\nfn t() {\n    assert_eq!(other(), 1);\n    assert_eq!(foo(), 2, \"{}\", double(2));\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &config);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(is_fail || is_unknown, "{text}: composed to PASS: {basis:?}");
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// Reading a call inside an assert never clears a call outside a macro that
    /// stays unverified: the verdict is the one without the assert's call, so no
    /// shape moves from UNKNOWN or FAIL to PASS.
    /// @vtest.id TEST-ORACLE-DA-003-ASSERT-CALL-DOES-NOT-CLEAR-UNVERIFIED-CALL
    /// @vtest.covers VO-ORACLE-DA-003-UNIDENTIFIED-CALL-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent マクロの外に検証されない target 呼出が残る Test は、別の target 呼出が assert の検証される引数にあっても DA-003 が違反なしにならない(UNKNOWN)ことを確認する
    #[test]
    fn an_assert_call_does_not_clear_an_unverified_call_outside_a_macro() {
        for text in [
            "#[test]\nfn t() {\n    double(3);\n    assert_eq!(double(2), 4);\n}\n",
            "#[test]\nfn t() {\n    let _ = double(3);\n    assert_eq!(double(2), 4);\n}\n",
            "#[test]\nfn t() {\n    c.double(3);\n    assert_eq!(c.double(2), 4);\n}\n",
            "#[test]\nfn t() {\n    let Some(x) = double(3) else { return };\n    assert_eq!(double(2), 4);\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A method call verified inside an assert is matched by name only: it does
    /// not clear a function call of the target that stays unverified, nor a name
    /// that a non-assert macro may call.
    /// @vtest.id TEST-ORACLE-DA-003-METHOD-IN-ASSERT-CANNOT-CLEAR-UNREAD-CALL
    /// @vtest.covers VO-ORACLE-DA-003-UNIDENTIFIED-CALL-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent assert の中のメソッド呼出だけが検証済みの根拠で、検証されない関数呼出や非 assert マクロ内の target の名前が別に残る Test は、DA-003 が UNKNOWN になることを確認する
    #[test]
    fn a_method_call_verified_in_an_assert_does_not_clear_other_uses() {
        for text in [
            "#[test]\nfn t() {\n    double(2);\n    assert_eq!(o.double(3), 6);\n}\n",
            "#[test]\nfn t() {\n    println!(\"{}\", double(2));\n    assert_eq!(o.double(3), 6);\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A name inside macro tokens that is not call-shaped is not a call, as it is
    /// not a call site outside a macro: a path prefix, a bare reference and a
    /// macro of the same name do not make DA-003 UNKNOWN; the turbofish call does.
    /// @vtest.id TEST-ORACLE-DA-003-NON-CALL-NAME-IN-MACRO-IS-NOT-A-CALL
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::tokens_call_ident
    /// @vtest.intent マクロのトークンの中の target の名前が、パスの前置(double::VALUE)、呼出でない参照、同名のマクロであるときは呼出と読まず、turbofish 付きの呼出(double::<u8>(2))は呼出と読むことを確認する
    #[test]
    fn a_name_in_macro_tokens_that_is_not_call_shaped_is_not_a_call() {
        for text in [
            "#[test]\nfn t() {\n    println!(\"{}\", double::VALUE);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let f = vec![double];\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    double!(1);\n    assert_eq!(other(), 1);\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::NoViolation),
                "{text}: {:?}",
                analysis.da_003
            );
        }
        let turbofish = "#[test]\nfn t() {\n    println!(\"{}\", double::<u8>(2));\n    assert_eq!(other(), 1);\n}\n";
        let analysis = analyze(turbofish, &["double".to_owned()], &[]);
        assert!(
            matches!(analysis.da_003, DaVerdict::Unknown(_)),
            "{:?}",
            analysis.da_003
        );
    }

    /// #288: the target's name as a value (not the callee of a direct call) may be
    /// an indirect call, which is not read: UNKNOWN, never PASS.
    /// @vtest.id TEST-ORACLE-DA-003-TARGET-USED-AS-VALUE-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-ANALYSIS-LIMIT-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent target の名前が直接呼出の callee 以外の値の位置(括弧つきの callee、let への代入、引数、配列、型付きの束縛、パス)に現れ、target と無関係な assert_eq! が本体にある Test は、DA-003 が UNKNOWN になり合成結果が PASS にならないことを確認する
    #[test]
    fn a_target_used_as_a_value_is_unknown_not_pass() {
        for text in [
            "#[test]\nfn t() {\n    (double)(2);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let f = double;\n    f(2);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    apply(double);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let f: fn(i32) -> i32 = double;\n    let _ = f(2);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let v = [double];\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let r: Vec<_> = xs.iter().map(Calc::double).collect();\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let f = double as fn(i32) -> i32;\n    assert_eq!(f(2), 4);\n}\n",
            "#[test]\nfn t() {\n    let f = double;\n    assert_eq!(f(2), 4);\n}\n",
            "#[test]\nfn t() {\n    let f = double;\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(is_fail || is_unknown, "{text}: composed to PASS: {basis:?}");
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A name that is not a value use of the target is not an indirect call: a
    /// `use`, a type, a path prefix, a field, a method name, a string, and a
    /// direct call that is verified stay no violation.
    /// @vtest.id TEST-ORACLE-DA-003-NON-VALUE-NAME-IS-NOT-AN-INDIRECT-CALL
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent use 文、型、パスの前置、フィールド名、メソッド名、文字列に現れる target の名前と、検証された直接呼出に併存する値の使用は、間接呼出と読まず DA-003 が違反なしのままであることを確認する
    #[test]
    fn a_name_that_is_not_a_value_use_is_not_an_indirect_call() {
        for text in [
            "#[test]\nfn t() {\n    use helpers::double;\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let x: double::Kind = other();\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let x: Vec<double> = other();\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let x = double::VALUE;\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let x = s.double;\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let x = s.double_it();\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let x = \"double\";\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    assert_eq!(double(2), 4);\n}\n",
            "#[test]\nfn t() {\n    let r = double(2);\n    assert_eq!(r, 4);\n    let f = double;\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::NoViolation),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A SrcId target has no call symbol, so its call cannot be found: the
    /// static-analysis verdict is UNKNOWN, not the PASS the other rules compose to.
    /// @vtest.id TEST-ORACLE-DA-003-SRC-ID-TARGET-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-UNIDENTIFIED-CALL-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::RustCargoStaticAnalysisAdapter
    /// @vtest.intent SrcId で宣言された target は呼出箇所を同定できないため、他の規則が違反なしでも静的解析の判定が PASS ではなく UNKNOWN になり、Locator の target だけの同じ本体は PASS のままであることを確認する
    #[test]
    fn a_src_id_target_makes_the_static_analysis_verdict_unknown_not_pass() {
        use vtest_model::{
            AdapterId, ContentHash, ExecutionDescriptor, ProjectPath, SourceLocation, SourceRange,
            SrcId, TestEntity, TestId,
        };

        let construct = "#[test]\nfn t() {\n    assert_eq!(double(2), 4);\n}\n";
        let hash = ContentHash::from_text(construct);
        let entity = |targets: Vec<TargetRef>| TestEntity {
            id: TestId::new("TEST-X"),
            covers: Vec::new(),
            targets,
            intent: "x".to_owned(),
            input: None,
            expect: None,
            kind: None,
            cases: Vec::new(),
            related: Vec::new(),
            location: SourceLocation {
                adapter: AdapterId::new("rust-cargo"),
                path: ProjectPath("tests/x.rs".to_owned()),
                locator: "x".to_owned(),
                byte_range: SourceRange { start: 0, end: 1 },
            },
            content_hash: hash.clone(),
            execution: ExecutionDescriptor {
                adapter: AdapterId::new("rust-cargo"),
                project: None,
                suite: None,
                selector: "x".to_owned(),
            },
        };
        let verdict = |targets: Vec<TargetRef>| {
            let test = entity(targets);
            RustCargoStaticAnalysisAdapter::new().analyze(StaticAnalysisInput {
                test: &test,
                construct_text: construct,
                content_hash: &hash,
                extra_assertion_macros: &[],
            })
        };
        let locator_target = TargetRef::Locator(locator("src/lib.rs::double"));
        assert!(matches!(
            verdict(vec![locator_target.clone()]),
            StaticAnalysisVerdict::Pass(_)
        ));
        let src_id_target = TargetRef::SrcId(SrcId::new("SRC-DOUBLE"));
        assert!(matches!(
            verdict(vec![src_id_target.clone()]),
            StaticAnalysisVerdict::Unknown(_)
        ));
        assert!(matches!(
            verdict(vec![locator_target, src_id_target]),
            StaticAnalysisVerdict::Unknown(_)
        ));
    }

    /// @vtest.id TEST-ORACLE-DA-004-IDENTICAL-ARGUMENTS-FAILS
    /// @vtest.covers VO-ORACLE-DA-004-SELF-COMPARISON
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_004_self_comparison
    /// @vtest.intent assert_eq!の両引数がトークン列として同一のときDA-004がFAILになることを確認する
    #[test]
    fn identical_assert_eq_arguments_fail_da_004() {
        let text = "#[test]\nfn tautology() {\n    assert_eq!(compute(), compute());\n}\n";
        let analysis = analyze(text, &[], &[]);
        assert!(matches!(analysis.da_004, DaVerdict::Fail(_)));
    }

    /// @vtest.id TEST-ORACLE-DA-001-ALL-LITERAL-FAILS
    /// @vtest.covers VO-ORACLE-DA-001-CONSTANT-ASSERTION
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_001_constant_assertion
    /// @vtest.intent assertの引数がすべてリテラルのときDA-001がFAILになることを確認する
    #[test]
    fn an_all_literal_assert_fails_da_001() {
        let text = "#[test]\nfn tautology() {\n    assert_eq!(1, 1);\n}\n";
        let analysis = analyze(text, &[], &[]);
        assert!(matches!(analysis.da_001, DaVerdict::Fail(_)));
    }

    /// @vtest.id TEST-ORACLE-LITERAL-FIXTURE-IS-NOT-CODE
    /// @vtest.covers VO-ORACLE-DA-004-SELF-COMPARISON
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::analyze
    /// @vtest.intent fixture source embedded in a string literal is not analyzed as executable Rust
    #[test]
    fn assert_macros_inside_fixture_strings_are_ignored() {
        let text = r##"#[test]
fn real_check() {
    let fixture = "assert_eq!(1, 1);";
    assert!(fixture.len() > 0);
}
"##;
        let analysis = analyze(text, &[], &[]);
        assert!(matches!(analysis.da_001, DaVerdict::NoViolation));
        assert!(matches!(analysis.da_004, DaVerdict::NoViolation));
    }

    /// @vtest.id TEST-ORACLE-DA-001-CALL-RESULT-PASSES
    /// @vtest.covers VO-ORACLE-DA-001-CONSTANT-ASSERTION
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_001_constant_assertion
    /// @vtest.intent assertが識別子（関数呼出結果）を参照するときDA-001が違反なしになることを確認する
    #[test]
    fn an_assert_referencing_a_call_result_does_not_fail_da_001() {
        let text = "#[test]\nfn real_check() {\n    assert_eq!(double(2), 4);\n}\n";
        let analysis = analyze(text, &[], &[]);
        assert!(matches!(analysis.da_001, DaVerdict::NoViolation));
    }

    /// @vtest.id TEST-ORACLE-DA-001-NON-ASCII-LINE
    /// @vtest.covers VO-ORACLE-DA-001-CONSTANT-ASSERTION
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::span_offset
    /// @vtest.intent an assert that follows non-ASCII text on its line, with LF or CRLF line ends, is read at its own byte range: a call-result assert is no violation and an all-literal assert still fails DA-001 (#264)
    #[test]
    fn an_assert_after_non_ascii_text_on_its_line_is_read_at_its_own_range() {
        for newline in ["\n", "\r\n"] {
            let real = format!(
                "#[test]{newline}fn real_check() {{{newline}    let label = \"日本語é\"; assert_eq!(double(2), 4, \"é\");{newline}}}{newline}"
            );
            let analysis = analyze(&real, &["double".to_owned()], &[]);
            assert!(matches!(analysis.da_001, DaVerdict::NoViolation), "{real}");
            assert!(matches!(analysis.da_003, DaVerdict::NoViolation), "{real}");

            let constant = format!(
                "#[test]{newline}fn constant() {{{newline}    let label = \"日本語é\"; assert_eq!(1, 1);{newline}}}{newline}"
            );
            let analysis = analyze(&constant, &[], &[]);
            assert!(matches!(analysis.da_001, DaVerdict::Fail(_)), "{constant}");
        }
    }

    /// @vtest.id TEST-ORACLE-DA-006-RESULT-RETURNING-SIGNATURE-PASSES
    /// @vtest.covers VO-ORACLE-DA-006-NO-VERIFICATION-SYNTAX
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_006_no_verification_syntax
    /// @vtest.intent Resultを返すTest関数シグネチャが検証構文として認められDA-006が違反なしになることを確認する
    #[test]
    fn a_result_returning_test_signature_counts_as_verification_syntax() {
        let text =
            "#[test]\nfn it_parses() -> Result<(), String> {\n    let _ = parse(\"x\")?;\n    Ok(())\n}\n";
        let analysis = analyze(text, &[], &[]);
        assert!(matches!(analysis.da_006, DaVerdict::NoViolation));
    }
}
