//! `rust-cargo`'s `oracle_presence` static-analysis capability (DS-605-627,
//! DS-1950, DS-1951, DS-1954).
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
//!   other than `matches!` (its first expression), `format!`, `vec!` and
//!   `assert!`/`assert_eq!`/`assert_ne!` (read as above), or one of those that does
//!   not parse as expressions (a `debug_assert*!` is not read there: DS-1954,
//!   DS-1962); or the `let mut` binding is
//!   changed through a place other than a projection assignment read below (a
//!   dereference assigned to, a projection whose path is not fixed by syntax, `&mut r`,
//!   `&mut r.p`, `&raw mut r`, a `ref mut` pattern over `r`, `r` as a method receiver
//!   outside an assert, an assignment in a closure, an `async` block or a macro's
//!   tokens, or `r` named in the tokens of a macro that is not read), since the assert
//!   may then read a value that is not the target's result (DS-1959, DS-615). This is a
//!   real subset of
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
//!   `matches!`, `format!` and `vec!`) it verifies the target only in the positions
//!   `CheckedCallVisitor` lists as carrying its value to the compared value, and a
//!   call in any other position (a block statement, `let _ =`, a closure body, a
//!   loop, a branch condition, ...) makes DA-003 `Unknown` (#289). A `let` name
//!   reaches an assert by the same positions (#289); a call-shaped use of
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
//!   delegation target terminates (DS-643). For DA-003 it recognizes a delegation
//!   target where the target's result flows into an argument of a call in an
//!   assert's checked argument or in a `let` binding that reaches one (DS-641,
//!   DS-1960): a function call other than a value conversion (`Some` / `Ok` / `Err`,
//!   `Box::new`, `String::from`, DS-1961) or a declared target, or a method call
//!   whose argument is the result (not DS-619's `.expect(..)` / `.expect_err(..)`,
//!   whose argument is a message, DS-1935). When no result of the target's calls
//!   reaches an assert-equivalent other than through such a call, DA-003 is
//!   `Unknown` (DS-648, DS-652); one result that reaches directly decides it, as
//!   derived from DS-623 and DS-1962. A delegation target that the result
//!   does not flow into (`assert_eq!(double(2), expected())`) is not recognized,
//!   which DS-641 reads more widely ("assert相当の引数…に現れる関数呼出"). For DA-006 it only recognizes
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
//! - A projection assignment `r.p = e` / `r.p op= e` to a `let mut` binding (DS-1956)
//!   keeps the binding's other positions derived when the path `p` is made only of
//!   field names, tuple numbers and integer-literal indexes, the assignment is outside
//!   a closure, an `async` block and macro tokens, and the binding's initializer is the
//!   target's result as a whole (a call of the target, through a method chain, `?`,
//!   `.await` or parentheses, whose positions are all derived: DS-1939). Any other
//!   initializer with such an assignment keeps the whole binding `Unknown`, which is
//!   narrower than DS-1939. A read in an assert's checked argument is then derived
//!   when no assigned path is a prefix of the path it reads (DS-1956). A read of a
//!   value that holds an assigned position is derived only when that value is
//!   compared whole by `assert_eq!` / `assert_ne!` or `==` / `!=` (`assert_eq!(r,
//!   R { .. })`; the false PASS DS-1939 and DS-1956 disclose). As a method receiver
//!   (DS-1939) and in any other use (an operator, `matches!`, `format!`, a
//!   constructor) it is not, nor is a read through an index that is not an integer
//!   literal (DS-1939); the uses other than a method receiver are a narrowing, as
//!   they may read the assigned position alone. A read that leaves an assigned path
//!   at a step of the other kind (an integer-literal index against a field or a
//!   tuple number: `r.0 = 5; assert_eq!(r[0], 5)`, `r[0] = 5; assert_eq!(r.g, 4)`) is
//!   not derived either, as an index of a type other than an array or a slice calls
//!   its `Index` / `IndexMut` method on the whole value; DS-1956 does not tell the two
//!   kinds apart, so this is a narrowing too. Statement order is not read: an assigned
//!   position counts as assigned for every read, before the assignment too, and on
//!   every path, in a branch, a loop or the right side of `&&` / `||` too (DS-1966).
//!   Such a position is never read as derived: the value `e` is not followed, so
//!   `r.f = double(3); assert_eq!(r.f, 6)` is `Unknown` where DS-1956 reads `r.f` as
//!   derived. A compound assignment keeps its position derived (DS-1956) unless a
//!   plain assignment is made to a path that is a prefix of its path or has it as a
//!   prefix, where the order would decide. Each of these narrowings only keeps a
//!   verdict `Unknown`.
//! - DS-1964 is recognized only where the receiver's variant is fixed by syntax
//!   in the narrowest form: a constructor call (`Some(..).unwrap()`,
//!   `Err(..).unwrap_err()`, `Ok(..)?`), or a name bound exactly once by a plain,
//!   non-`mut`, non-`ref` `let` whose initializer is that constructor. An
//!   `.unwrap()` / `.expect(..)` / `?` on a `let mut` binding, on a name bound by
//!   a pattern (`let (o, _) = (Some(x), 1)`, `if let`, `match`), or on a name bound
//!   again (shadowed or rebound) stays an assert-equivalent (DS-619). DA-006 then
//!   stays `NoViolation` for such a body, e.g. `let mut o = Some(f()); o.unwrap();`,
//!   `let (o, _) = (Some(f()), 1); o.unwrap();`, or
//!   `let o = Some(f()); let o = o; o.unwrap();`.
//! - DA-001's constant-expression classifier only recognizes literals
//!   (numeric/string/bool/char), simple arithmetic/comparison
//!   combinations of literals, and an argument that is a logical expression whose
//!   value literals fix (DS-1949: `X || true`, `true || X`, `X && false`,
//!   `false && X`, the same with `|` / `&`, their negations, and such expressions
//!   nested in one another, parenthesized or not). An operand made only of literals
//!   counts as the value it has: `!`, `^`, `==` / `!=` of `bool` literals and the
//!   comparisons of two literals of one kind (integer, string, char) are
//!   evaluated (`X || 1 == 1` is constant). Float literals are not compared, since
//!   the meaning of the comparison (`f32` or `f64`) is not fixed by syntax. A `bool`
//!   operand made only of literals whose value is not evaluated (`X || 1 + 1 == 2`,
//!   `X || 0.1f32 == 0.100000001f32`) may absorb, so DA-001 is `Unknown` for it
//!   (DS-621's escape). Another form with no variable, call, struct literal, closure
//!   or identifier in macro tokens (`if true { .. }`, `{ true }`, `1 as u8 == 1`,
//!   `matches!(1, 1)`) is built from literals only and is not evaluated, so it may
//!   absorb in the same way. A macro other than `matches!` (`m!()`, `env!("X")`,
//!   `double_m!(2)`) may expand to any code (DS-629), so a form holding one is not
//!   literal-only: as an operand of `||` / `|` / `&&` / `&` it may absorb, so
//!   DA-003 does not read its sibling as flowing, and DA-001 does not decide its
//!   constancy (`Unknown`, never `Fail`). A macro whose tokens hold an identifier
//!   (`cfg!(test)`, `m!(x)`) counts as non-literal like a call, which DS-1948
//!   leaves as a false PASS.
//!   Any other identifier
//!   reference (a `const` item included) is treated as non-constant rather than
//!   resolved, which can only make DA-001 report `NoViolation` where a deeper
//!   analysis might report `Fail`. The classifier reads every argument of an
//!   assert macro, its message arguments included, and `panic!`'s arguments too.
//!   A `.unwrap()` / `.expect(..)` / `?` that DS-619 counts (DS-1964 aside) is a
//!   non-constant assert-equivalent, so DA-001 has no violation beside it
//!   (DS-616, DS-621), but only outside a closure, an `async` block and a nested
//!   item. A position in a branch (`if c { x.unwrap(); }`) counts, as in DS-1962
//!   (derivation from the intent of DS-1962, which names those positions for
//!   DA-002 / DA-003; the canon does not name DA-001's positions). One only in a
//!   closure, an `async` block or a nested item is not counted, and
//!   DA-001 is `Unknown` where it would decide the verdict. A counted one also
//!   clears a constant assert under `#[should_panic]` (`let r = f().unwrap();
//!   assert!(false);`), which widens the false PASS DS-1950 / DS-1964 disclose.
//! - DA-003 does not count an operand of `||` / `|` / `&&` / `&` as a position the
//!   value flows from when the other operand fixes the result to the absorbing value
//!   (DS-1948: `true` for `||` / `|`, `false` for `&&` / `&`), or is made only of
//!   literals whose value is not evaluated (as above). A call there is in a position
//!   that drops its value, so DA-003 is `Unknown` for it. That is a derivation that
//!   follows the other dropping positions here: read plainly, DS-1948 with DS-623
//!   could make it `Fail`, and `Unknown` is the safe side. An operand
//!   beside any other expression flows (`X || false`, `X || ok`); an absorption
//!   that is not by literals (`X || always_true()`, `X * 0`) is not seen, which
//!   DS-1948 leaves as a false PASS.
//! - DA-001 reads `unreachable!` / `todo!` / `unimplemented!` (DS-1951) by whether
//!   they run, not by their arguments (DS-1950). One that is a statement of the
//!   function's own block with no earlier statement that may leave the function is
//!   a constant assertion. Any other one has positions that decide whether it
//!   runs (DS-1947, DS-1983), which this analysis does not evaluate, so DA-001 is
//!   `Unknown` when it decides the verdict (DS-621's escape). This is narrower
//!   than DS-1950 for a divergence under a non-constant condition (no violation
//!   there) and for these, each a constant assertion there: one under a constant
//!   condition (`if true { .. }`), one whose deciding positions are all constant,
//!   one in an arm that cannot be refuted (`match r { _ => unreachable!() }`), one
//!   in a `let` initializer (`let x: i32 = todo!();`) and one in an inner block
//!   (`{ unreachable!() }`). A `?` or a `return` (also `return Err(..)`) before the
//!   divergence is read as an exit, so DA-001 is `Unknown` there too, also where
//!   DS-1951 reads the divergence as a constant assertion.
//! - `debug_assert!` / `debug_assert_eq!` / `debug_assert_ne!` are not
//!   assert-equivalents, also when the config lists them (DS-1954): they make
//!   DA-003 and DA-006 `Unknown` where they are the only check.

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
/// all six DA rules (DS-616). DS-617 closes the list of standard macros; any
/// other macro counts only through DS-617's `rust-cargo` config
/// `assertion_macros` list (`extra_assertion_macros`; this capability's caller
/// supplies it; this module does not read `config.yaml` itself).
const STANDARD_ASSERTION_MACROS: &[&str] = &[
    "assert!",
    "assert_eq!",
    "assert_ne!",
    "panic!",
    "unreachable!",
    "todo!",
    "unimplemented!",
];

/// DS-1951's failure divergences among DS-617's macros other than `panic!`. DA-001
/// judges their constancy by whether they run (DS-1950), not by their message
/// arguments (`da_001_constant_assertion`).
const DIVERGING_MACROS: &[&str] = &["unreachable", "todo", "unimplemented"];

/// DS-1954: not assert-equivalents, even when the config lists them.
const DEBUG_ASSERT_MACROS: &[&str] = &["debug_assert", "debug_assert_eq", "debug_assert_ne"];

fn is_debug_assert(name: &str) -> bool {
    DEBUG_ASSERT_MACROS.contains(&name)
}

fn is_diverging_macro(name: &str) -> bool {
    DIVERGING_MACROS.contains(&name)
}

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
    let assert_spans = find_assert_spans(construct_text, &assertion_macros(extra_assertion_macros));
    let syntax = RustFunctionSyntax::parse(construct_text);
    let has_should_panic = syntax
        .as_ref()
        .is_some_and(RustFunctionSyntax::has_should_panic);
    let method_or_try = syntax
        .as_ref()
        .map_or(MethodOrTry::None, RustFunctionSyntax::method_or_try);
    let has_assert_equivalent_method_or_try = method_or_try != MethodOrTry::None;
    let has_result_return = syntax
        .as_ref()
        .is_some_and(RustFunctionSyntax::returns_result);
    let has_verification_syntax = !assert_spans.is_empty()
        || has_should_panic
        || has_assert_equivalent_method_or_try
        || has_result_return;
    let has_call = syntax.as_ref().is_some_and(RustFunctionSyntax::has_call);
    let has_debug_assert = !debug_assert_macros(construct_text).is_empty();

    OraclePresenceAnalysis {
        da_001: da_001_constant_assertion(construct_text, &assert_spans, method_or_try),
        da_003: da_003_result_unverified(
            construct_text,
            target_symbols,
            target_symbols,
            &assert_spans,
            has_should_panic,
        ),
        da_004: da_004_self_comparison(construct_text),
        da_005: da_005_empty_test(construct_text),
        da_006: da_006_no_verification_syntax(has_verification_syntax, has_debug_assert, has_call),
    }
}

/// DA-003 for one declared target `symbol` of a Test whose declared targets are
/// `all_targets` (DS-630's per-target report): the other targets' calls are not
/// delegation targets (DS-1960), as in `analyze`.
fn analyze_da_003_for(
    construct_text: &str,
    symbol: &str,
    all_targets: &[String],
    extra_assertion_macros: &[String],
) -> DaVerdict {
    let assert_spans = find_assert_spans(construct_text, &assertion_macros(extra_assertion_macros));
    let has_should_panic = RustFunctionSyntax::parse(construct_text)
        .as_ref()
        .is_some_and(RustFunctionSyntax::has_should_panic);
    da_003_result_unverified(
        construct_text,
        &[symbol.to_owned()],
        all_targets,
        &assert_spans,
        has_should_panic,
    )
}

/// DS-617's macros. A config entry naming a `debug_assert*` macro (by its last
/// path segment, with or without `!`) is left out (DS-1954).
fn assertion_macros(extra_assertion_macros: &[String]) -> Vec<&str> {
    STANDARD_ASSERTION_MACROS
        .iter()
        .copied()
        .chain(
            extra_assertion_macros
                .iter()
                .map(String::as_str)
                .filter(|name| {
                    let last = name.rsplit("::").next().unwrap_or(name);
                    !is_debug_assert(last.trim_end_matches('!'))
                }),
        )
        .collect()
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
                TargetRef::Locator(locator) => analyze_da_003_for(
                    input.construct_text,
                    &target_symbol(locator),
                    &symbols,
                    input.extra_assertion_macros,
                ),
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

fn da_001_constant_assertion(
    text: &str,
    assert_spans: &[(usize, usize)],
    method_or_try: MethodOrTry,
) -> DaVerdict {
    if assert_spans.is_empty() {
        // No assert-equivalent macro call at all: DA-001's own subject
        // (an assert whose arguments are constant) does not occur, so
        // there is nothing to fail on. `#[should_panic]`/`.unwrap()`-only
        // verification is DA-003/DA-006's territory, not DA-001's.
        return DaVerdict::NoViolation;
    }
    // DS-1950: the constancy of a DS-1951 divergence is decided by the positions
    // that decide whether it runs, never by its message arguments. One that runs
    // whenever the body reaches it has no such position and is a constant
    // assertion. Any other has positions this classifier does not evaluate
    // (DS-1947, DS-1983), so its constancy is DS-621's "定数性を確定できない式".
    //
    // DS-621 fails only when every assert-equivalent is constant, and DS-616 counts
    // DS-619's `.unwrap()` / `.expect(..)` / `?` among them. Such a method or `?` is
    // a non-constant assert-equivalent (this classifier reads any name as
    // non-constant), so DA-001 has no violation beside it, whichever assert macros
    // there are. Derivation from the intent of DS-1962: only one in a position that
    // is outside a closure, an `async` block and a nested item counts, a branch
    // included as in DS-1962 (`MethodOrTry::Counted`).
    if method_or_try == MethodOrTry::Counted {
        return DaVerdict::NoViolation;
    }
    let unconditional = unconditional_divergence_starts(text);
    let mut undetermined = false;
    let mut undetermined_argument = false;
    for &(start, end) in assert_spans {
        if macro_name_before(text, start).is_some_and(is_diverging_macro) {
            undetermined |= !unconditional.contains(&start);
        } else {
            match classify_constancy(&text[start..end]) {
                Constancy::NotAllLiteral => return DaVerdict::NoViolation,
                Constancy::Undetermined => undetermined_argument = true,
                Constancy::AllLiteral => {}
            }
        }
    }
    if undetermined {
        return DaVerdict::Unknown(
            "every other assert-equivalent call is constant, and an `unreachable!` / `todo!` / \
             `unimplemented!` is not a statement of the function's own block before any exit \
             (it is in a condition, an arm, a loop, an inner block or an expression, or after \
             an exit), and this analysis does not decide its constancy there (DS-621, DS-1950)"
                .to_owned(),
        );
    }
    if undetermined_argument {
        return DaVerdict::Unknown(
            "every other assert-equivalent call is constant, and an argument holds a macro \
             whose expansion is not read, or an operand of `||` / `|` / `&&` / `&` made only \
             of literals whose value this analysis does not evaluate, so whether it is \
             constant is not decided (DS-621, DS-629, DS-1949)"
                .to_owned(),
        );
    }
    if method_or_try == MethodOrTry::Uncounted {
        return DaVerdict::Unknown(
            "every assert macro's arguments are constant, and a `.unwrap()` / `.expect(..)` / `?` \
             appears only in a closure, an `async` block or a nested item, where whether it runs \
             is not decided, so it is neither counted as a non-constant assert-equivalent nor \
             ruled out (DS-621; positions derived from the intent of DS-1962)"
                .to_owned(),
        );
    }
    // Any identifier reference resolves to `NotAllLiteral` (module doc — this can
    // only under-report `Fail`, never fabricate one).
    DaVerdict::Fail("every assert-equivalent call's arguments are all literal/constant (DS-621)")
}

/// The argument starts (as in `find_assert_spans`) of the DS-1951 divergences in
/// `DIVERGING_MACROS` that run whenever the body runs: a statement of the
/// function's own block (not nested in a branch, a loop, a closure or an inner
/// block) with no earlier statement of that block that may leave the function
/// first. Derivation from DS-1950 and DS-1983: such a divergence has no position
/// that decides whether it runs.
///
/// An earlier statement may leave when it contains `return`, `?`, `break`,
/// `continue`, a call of a function named `exit`, or a macro other than the
/// assert, `debug_assert*`, divergence and printing macros named here, whose
/// expansion is not read (DS-1983's exits; a macro is read fail-closed, DS-615).
fn unconditional_divergence_starts(text: &str) -> Vec<usize> {
    let Ok(function) = syn::parse_str::<syn::ItemFn>(text) else {
        return Vec::new();
    };
    let mut starts = Vec::new();
    for statement in &function.block.stmts {
        let mac = match statement {
            syn::Stmt::Macro(node) => Some(&node.mac),
            syn::Stmt::Expr(syn::Expr::Macro(node), _) => Some(&node.mac),
            _ => None,
        };
        if let Some(mac) = mac {
            if mac
                .path
                .segments
                .last()
                .is_some_and(|segment| is_diverging_macro(&segment.ident.to_string()))
            {
                if let Some(start) = macro_argument_start(text, mac) {
                    starts.push(start);
                }
            }
        }
        let mut exits = ExitVisitor { found: false };
        exits.visit_stmt(statement);
        if exits.found {
            break;
        }
    }
    starts
}

/// Whether a statement contains a way to leave the function before the next one.
struct ExitVisitor {
    found: bool,
}

impl<'ast> Visit<'ast> for ExitVisitor {
    fn visit_expr_return(&mut self, _node: &'ast syn::ExprReturn) {
        self.found = true;
    }

    fn visit_expr_try(&mut self, _node: &'ast syn::ExprTry) {
        self.found = true;
    }

    fn visit_expr_break(&mut self, _node: &'ast syn::ExprBreak) {
        self.found = true;
    }

    fn visit_expr_continue(&mut self, _node: &'ast syn::ExprContinue) {
        self.found = true;
    }

    // DS-1951 names `process::exit` among the exits; any callee named `exit` is
    // read as one.
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = node.func.as_ref() {
            self.found |= path
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "exit");
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let known = node.path.segments.last().is_some_and(|segment| {
            let name = segment.ident.to_string();
            STANDARD_ASSERTION_MACROS.contains(&format!("{name}!").as_str())
                || is_debug_assert(&name)
                || matches!(
                    name.as_str(),
                    "format" | "print" | "println" | "eprint" | "eprintln" | "vec" | "matches"
                )
        });
        // An argument of a known macro may still hold an exit.
        self.found |= !known || tokens_may_exit(node.tokens.clone());
    }
}

/// Whether macro tokens hold `?`, `return`, `break` or `continue` outside literals.
fn tokens_may_exit(tokens: proc_macro2::TokenStream) -> bool {
    tokens.into_iter().any(|tree| match tree {
        proc_macro2::TokenTree::Group(group) => tokens_may_exit(group.stream()),
        proc_macro2::TokenTree::Punct(punct) => punct.as_char() == '?',
        proc_macro2::TokenTree::Ident(ident) => {
            ident == "return" || ident == "break" || ident == "continue"
        }
        proc_macro2::TokenTree::Literal(_) => false,
    })
}

enum Constancy {
    AllLiteral,
    NotAllLiteral,
    /// DS-621's "定数性を確定できない式".
    Undetermined,
}

/// Whether DS-619's `.unwrap()` / `.expect(..)` / `?` (DS-1964 aside) appears in a
/// Test body, for DA-001.
#[derive(Clone, Copy, PartialEq, Eq)]
enum MethodOrTry {
    None,
    /// Outside a closure, an `async` block and a nested item; a position in a
    /// branch counts, as in DS-1962.
    Counted,
    /// Only in a closure, an `async` block or a nested item.
    Uncounted,
}

/// A conservative literal/constant-expression classifier: recognizes
/// numeric/string/bool/char literals and their combination with arithmetic/
/// comparison operators and parentheses/commas/whitespace, and a logical
/// expression whose value literals fix (`fixed_value`, DS-1949) as one argument.
/// Any other identifier (a variable, a function call, a `const` item reference)
/// makes the whole argument list `NotAllLiteral` — this can only under-report
/// DA-001's `Fail` (treat a truly-constant `const` reference as non-constant),
/// never over-report it, per this module's doc comment.
fn classify_constancy(arguments: &str) -> Constancy {
    if let Constancy::AllLiteral = classify_literal_tokens(arguments) {
        return Constancy::AllLiteral;
    }
    let Ok(parsed) = syn::parse::Parser::parse_str(
        syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated,
        arguments,
    ) else {
        return Constancy::NotAllLiteral;
    };
    let mut undetermined = false;
    for argument in &parsed {
        if let Constancy::AllLiteral =
            classify_literal_tokens(&argument.to_token_stream().to_string())
        {
            continue;
        }
        match argument_constancy(argument) {
            Constancy::AllLiteral => {}
            Constancy::Undetermined => undetermined = true,
            Constancy::NotAllLiteral => return Constancy::NotAllLiteral,
        }
    }
    if undetermined {
        Constancy::Undetermined
    } else {
        Constancy::AllLiteral
    }
}

fn classify_literal_tokens(arguments: &str) -> Constancy {
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

/// DA-003 for the calls of `target_symbols`. `all_targets` is every declared
/// target of the Test, whose calls are not delegation targets (DS-1960).
fn da_003_result_unverified(
    text: &str,
    target_symbols: &[String],
    all_targets: &[String],
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
    let mut any_delegated = false;
    let mut any_into_target = false;
    let mut any_debug_assert = false;
    for symbol in target_symbols {
        match target_call_result_is_verified(text, symbol, all_targets, assert_spans) {
            TargetCallVerification::NotCalled => {}
            TargetCallVerification::Delegated => {
                any_call_found = true;
                any_delegated = true;
            }
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
            TargetCallVerification::IntoTargetArgument => {
                any_call_found = true;
                any_into_target = true;
            }
            TargetCallVerification::MacroOpaque => {
                any_call_found = true;
                any_macro_opaque = true;
            }
            TargetCallVerification::DebugAssertOnly => {
                any_call_found = true;
                any_debug_assert = true;
            }
        }
    }
    if !any_call_found {
        // No declared target is called at all in this construct — that is
        // DA-002's ("対象未呼出") territory (target_binding, not
        // oracle_presence). DA-003 only fires once a call exists.
        return DaVerdict::NoViolation;
    }
    // A FAIL proven for one target decides before another target's UNKNOWN, as
    // `analyze` composes (DS-623, DS-1680): with no assert-equivalent construct and
    // no other call to delegate to, an unverified result reaches none. A target
    // whose call or result this analysis cannot follow (a method call matched by
    // name only, a binding or macro it does not read, a result handed to another
    // target, a mutable reference) may still carry a result there, so those keep
    // their UNKNOWN (DS-1680).
    if any_unverified
        && !(any_unidentified
            || any_ambiguous
            || any_binding_uncertain
            || any_into_target
            || any_macro_opaque)
        && assert_spans.is_empty()
        && !has_non_target_call(text, target_symbols)
    {
        return DaVerdict::Fail(
            "a declared target's call result reaches no assert-equivalent construct, \
             and no #[should_panic] is present (DS-623)",
        );
    }
    if any_delegated {
        // DS-641 / DS-1960: no result of a declared target's call reaches an
        // assert-equivalent other than through a delegation target (see
        // `target_call_result_is_verified`). This capability does not confirm a
        // delegation target's termination (DS-643), so DA-003 is UNKNOWN by §7.2.1
        // (DS-648, DS-651, DS-652).
        return DaVerdict::Unknown(
            "a declared target's result flows into an argument of a function call that is \
             not a value conversion, or of a method call, in an assert-equivalent's checked \
             argument or a binding that reaches one: that call is a delegation target of \
             this Test (DS-641, DS-1960) whose termination this analysis does not confirm, \
             so the result is neither a violation nor its absence (DS-648, DS-652)"
                .to_owned(),
        );
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
             bound again, a `let mut` binding is changed through a dereference, a path not \
             fixed by syntax, `&mut`, a `ref mut` pattern, a method call, a closure or a \
             macro (DS-1959), the assert reads a position that a projection assignment \
             may have overwritten (DS-1956, DS-1966) or a value holding one as a method \
             receiver or through an index that is not an integer literal (DS-1939), or it \
             appears in tokens this capability does not parse), so the result reaching the \
             assert is neither proven nor refuted (DS-1680)"
                .to_owned(),
        );
    }
    if any_into_target {
        // DS-1960 excludes a declared target's call from the delegation targets
        // and does not carry its argument to its value, so a result that reaches
        // an assert only as such an argument is not shown verified (DS-615).
        return DaVerdict::Unknown(
            "a declared target's result is bound to a name that reaches an \
             assert-equivalent only as an argument of another declared target's call: \
             that call is not a delegation target and does not carry the argument to its \
             value (DS-1960), so the result is not compared directly, and its being \
             unverified is neither proven nor refuted (DS-615)"
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
    if any_debug_assert {
        return DaVerdict::Unknown(
            "a declared target's result reaches no assert-equivalent, but a binding of it is \
             used in a checked argument of a `debug_assert*`, which is not an assert-equivalent \
             but checks in a debug build (DS-1954)"
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
        // The case with no other call returned FAIL above.
        if assert_spans.is_empty() {
            return DaVerdict::Unknown(
                "a call other than the declared target is a delegation target of this Test \
                 (DS-642) and its termination (§7.2.1) is not confirmed, so the result is \
                 neither a violation nor its absence (DS-652)"
                    .to_owned(),
            );
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
    /// The binding that holds a call's result reaches an assert only as an
    /// argument of a declared target's call (DS-1960).
    IntoTargetArgument,
    /// The result flows into a delegation target in an assert-equivalent's
    /// checked argument or in a binding that reaches one (DS-641, DS-1960).
    Delegated,
    /// No assert-equivalent is reached, but a binding of the result is used in a
    /// checked argument of a `debug_assert*` (DS-1954).
    DebugAssertOnly,
}

/// Whether a `let` binding's name reaches an assert-equivalent argument list.
enum BindingReach {
    Reaches,
    NotReached,
    Uncertain,
    /// The name flows into an argument of a delegation target there (DS-1960).
    Delegated,
    /// The name is used there only in an argument of a declared target's call,
    /// which carries no value to the assert (DS-1960).
    IntoTarget,
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
///
/// A `let mut` binding that is changed through a place (`place_changes`) cannot be
/// told to mean the result (DS-1959, DS-615), except through a projection assignment
/// whose path is fixed by syntax, after which only the assigned positions stop being
/// derived (DS-1956): the assert's reads are then judged by the path they read
/// (`read_is_derived`). Only a `let mut` binding is checked. A binding of a target
/// that returns `&mut T` can be changed without `let mut` and is not covered.
fn binding_reach(
    text: &str,
    binding: &DerivedBinding,
    targets: &[String],
    assert_spans: &[(usize, usize)],
) -> BindingReach {
    let name = binding.name.as_str();
    let changes = if binding.mutable {
        place_changes(text, name, assert_spans)
    } else {
        PlaceChanges::default()
    };
    // DS-1956 keeps the other positions derived only for a value whose positions
    // are all derived to begin with (DS-1939); a binding of a literal that holds the
    // result in one element is not followed through an assignment.
    let tracked = !changes.whole && (changes.overwrites.is_empty() || binding.whole_result);
    let overwrites: &[Vec<PlaceStep>] = if tracked { &changes.overwrites } else { &[] };
    let reaches = |outcome: &BindingReach| matches!(outcome, BindingReach::Reaches);
    let outcomes: Vec<BindingReach> = assert_spans
        .iter()
        .map(|&(start, end)| {
            span_binding_reach(
                &text[start..end],
                macro_name_before(text, start),
                name,
                targets,
                overwrites,
            )
        })
        .collect();
    // A name bound again or changed in place is not shown to mean this binding,
    // whether it reaches an assert directly or through a delegation target.
    let name_uncertain = || count_rebindings(text, name) > 1 || !tracked;
    // DS-623 / DS-1962: an assert the name reaches directly decides, and a
    // delegation target in another assert does not make it UNKNOWN.
    if outcomes.iter().any(reaches) {
        return if name_uncertain() {
            BindingReach::Uncertain
        } else {
            BindingReach::Reaches
        };
    }
    if outcomes
        .iter()
        .any(|outcome| matches!(outcome, BindingReach::Delegated))
    {
        return if name_uncertain() {
            BindingReach::Uncertain
        } else {
            BindingReach::Delegated
        };
    }
    if outcomes
        .iter()
        .any(|outcome| matches!(outcome, BindingReach::Uncertain))
    {
        return BindingReach::Uncertain;
    }
    if outcomes
        .iter()
        .any(|outcome| matches!(outcome, BindingReach::IntoTarget))
    {
        return if name_uncertain() {
            BindingReach::Uncertain
        } else {
            BindingReach::IntoTarget
        };
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
        "panic" | "unreachable" | "todo" | "unimplemented" => Some(0),
        "format" | "vec" => Some(usize::MAX),
        _ => None,
    }
}

fn span_binding_reach(
    span_text: &str,
    macro_name: Option<&str>,
    binding: &str,
    targets: &[String],
    overwrites: &[Vec<PlaceStep>],
) -> BindingReach {
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
    let mut visitor = CheckedCallVisitor::new(None, Some(binding), targets);
    visitor.probe_overwrites = overwrites;
    // `assert_eq!` / `assert_ne!` compare their two checked arguments whole.
    let compares = matches!(macro_name, Some("assert_eq" | "assert_ne"));
    for argument in arguments.iter().take(checked) {
        visitor.in_compared = compares;
        visitor.visit_expr(argument);
    }
    // A use of the name where its value is dropped (`{ r; true }`, `let _ = r`, a
    // closure body, a `match` scrutinee) does not carry it to the compared value,
    // which is not shown to be verified either: UNKNOWN (DS-615). So does a read of
    // an assigned position, whose value is not followed (DS-1956, DS-1966).
    // A use that flows to the compared value decides before a use handed to a
    // delegation target (DS-623, DS-1962).
    if visitor.opaque || visitor.probe_rebound {
        BindingReach::Uncertain
    } else if visitor.probe_flows {
        BindingReach::Reaches
    } else if visitor.probe_delegated {
        BindingReach::Delegated
    } else if visitor.probe_dropped || visitor.probe_overwritten {
        BindingReach::Uncertain
    } else if visitor.probe_into_target {
        BindingReach::IntoTarget
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
    /// A call whose value flows into a delegation target in a checked argument of
    /// an assert (DS-1960).
    delegated: bool,
}

/// Derivation (not stated in DS-615 / DS-651 / DS-1680): a macro's tokens are not
/// syntax-tree nodes, so a call written there is found only by reading the
/// tokens. In `assert!` / `assert_eq!` / `assert_ne!` the checked arguments are
/// read as expressions, and a call found there verifies the target as a call
/// outside a macro that reaches an assert does. A call hidden anywhere else (any
/// other macro, a message argument, tokens that do not parse) may or may not exist
/// and may or may not be verified, and DS-651 reads a call hidden in a macro
/// expansion as UNKNOWN, so a use of the name there that is not read as a call
/// whose result an assert checks is `opaque` (`NameUse::tokens_use_name`). Reading
/// macro tokens only moves a verdict toward UNKNOWN or turns "not called" into
/// verified: `target_call_result_is_verified` takes a call read in an assert as
/// verifying only when the Test has no call outside a macro, so it never clears
/// a call outside a macro that stays unverified, and a verified method call
/// (matched by name only) does not clear an `opaque` name that may be a
/// function call.
fn macro_target_use(text: &str, symbol: &str, targets: &[String]) -> MacroUse {
    let Ok(function) = syn::parse_str::<syn::ItemFn>(text) else {
        return MacroUse::default();
    };
    let names = NameUse::new(text, symbol, &function);
    let mut visitor = MacroUseVisitor {
        names: &names,
        targets,
        found: MacroUse::default(),
    };
    visitor.visit_item_fn(&function);
    let mut values = ValueUseVisitor {
        names: &names,
        found: false,
    };
    values.visit_item_fn(&function);
    visitor.found.opaque |= values.found;
    visitor.found
}

/// Which uses of the target's name are the target's own, for `ValueUseVisitor`
/// and the macro readers.
struct NameUse<'a> {
    symbol: &'a str,
    text: &'a str,
    /// Derivation (not stated in DS-615 / DS-1680): a constant or static is not
    /// callable, and Rust names them in upper case, so its value is not an
    /// indirect call.
    is_constant: bool,
    /// The byte ranges in which the name is bound by a `let` statement: from the
    /// end of the `let` to the end of its block. A single-segment path inside one
    /// names that local, not the target (derivation: lexical scope). A binding in
    /// a closure, a match arm, a nested fn or an assignment is no exception, and
    /// neither is a use before the `let` or outside its block.
    local_ranges: Vec<(usize, usize)>,
}

impl<'a> NameUse<'a> {
    fn new(text: &'a str, symbol: &'a str, function: &syn::ItemFn) -> Self {
        let mut scopes = LetScopeVisitor {
            text,
            symbol,
            ranges: Vec::new(),
        };
        scopes.visit_item_fn(function);
        Self {
            symbol,
            text,
            is_constant: is_constant_name(symbol),
            local_ranges: scopes.ranges,
        }
    }

    fn is_local_at(&self, location: proc_macro2::LineColumn) -> bool {
        let offset = span_offset(location, self.text);
        self.local_ranges
            .iter()
            .any(|&(from, to)| offset >= from && offset < to)
    }

    /// An expression path that is a value use of the target.
    fn path_is_value_use(&self, node: &syn::ExprPath) -> bool {
        let names_symbol = node
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == self.symbol);
        let names_local = node.qself.is_none()
            && node.path.get_ident().is_some()
            && self.is_local_at(node.span().start());
        names_symbol && !self.is_constant && !names_local
    }

    /// Whether the tokens of a macro use the name where it is not a direct call
    /// read as an expression: followed by an argument list (`name(..)`) or by
    /// `::<`, or standing alone (a value). Not a use: after `.` (a method or a
    /// field), before `::` (a path prefix), before `!` (a macro name), before a
    /// single `:` (a field label), a constant, or a local bound by a `let` earlier in the same block.
    fn tokens_use_name(&self, tokens: proc_macro2::TokenStream) -> bool {
        use proc_macro2::{Delimiter, TokenTree};
        let trees: Vec<TokenTree> = tokens.into_iter().collect();
        trees.iter().enumerate().any(|(index, tree)| match tree {
            TokenTree::Group(group) => self.tokens_use_name(group.stream()),
            TokenTree::Ident(ident) if ident == self.symbol => {
                let next = trees.get(index + 1);
                let punct_after = |offset: usize, expected: char| {
                    matches!(trees.get(index + offset), Some(TokenTree::Punct(p)) if p.as_char() == expected)
                };
                if matches!(next, Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis)
                    || (punct_after(1, ':') && punct_after(2, ':') && punct_after(3, '<'))
                {
                    return true;
                }
                let after_dot = index > 0
                    && matches!(&trees[index - 1], TokenTree::Punct(p) if p.as_char() == '.');
                let path_prefix = punct_after(1, ':') && punct_after(2, ':');
                let field_label = punct_after(1, ':') && !punct_after(2, ':');
                let macro_name = punct_after(1, '!') && !punct_after(2, '=');
                !(after_dot
                    || path_prefix
                    || field_label
                    || macro_name
                    || self.is_constant
                    || self.is_local_at(ident.span().start()))
            }
            TokenTree::Ident(_) | TokenTree::Punct(_) | TokenTree::Literal(_) => false,
        })
    }
}

struct LetScopeVisitor<'a> {
    text: &'a str,
    symbol: &'a str,
    ranges: Vec<(usize, usize)>,
}

impl<'ast> Visit<'ast> for LetScopeVisitor<'_> {
    fn visit_block(&mut self, node: &'ast syn::Block) {
        let block_end = span_offset(node.brace_token.span.join().end(), self.text);
        for statement in &node.stmts {
            if let syn::Stmt::Local(local) = statement {
                if pattern_binds(&local.pat, self.symbol) {
                    self.ranges
                        .push((span_offset(local.span().end(), self.text), block_end));
                }
            }
        }
        syn::visit::visit_block(self, node);
    }
}

/// Whether a pattern binds `name` as its own identifier (`name`, `mut name`,
/// `name: T`).
fn pattern_binds(pattern: &syn::Pat, name: &str) -> bool {
    match pattern {
        syn::Pat::Ident(ident) => ident.ident == name,
        syn::Pat::Type(typed) => pattern_binds(&typed.pat, name),
        _ => false,
    }
}

fn is_constant_name(name: &str) -> bool {
    name.chars().any(|c| c.is_ascii_uppercase())
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// Derivation (not stated in DS-615 / DS-623 / DS-1680): the target's name as an
/// expression path that is not the callee of a direct call (`(name)(..)`,
/// `let f = name;`, `apply(name)`, `Type::name` passed as a value) may be called
/// later through another name, which this capability does not follow. It is
/// `opaque`, like a call hidden in macro tokens: UNKNOWN, not "not called". A
/// `use`, a type, a path prefix, a field, a method name and a string are not
/// expression paths of the target. Inside macros the same uses are read from the
/// tokens (`NameUse::tokens_use_name`) or, in an assert's checked arguments, from
/// the parsed expressions (`CheckedCallVisitor`).
struct ValueUseVisitor<'a> {
    names: &'a NameUse<'a>,
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
        if self.names.path_is_value_use(node) {
            self.found = true;
        }
    }
}

struct MacroUseVisitor<'a> {
    names: &'a NameUse<'a>,
    targets: &'a [String],
    found: MacroUse,
}

impl<'ast> Visit<'ast> for MacroUseVisitor<'_> {
    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if !self.names.tokens_use_name(node.tokens.clone()) {
            return;
        }
        let is_assert = node.path.segments.last().is_some_and(|segment| {
            ["assert", "assert_eq", "assert_ne"].contains(&segment.ident.to_string().as_str())
        });
        let Some(arguments) = is_assert.then(|| macro_arguments(node)).flatten() else {
            self.found.opaque = true;
            return;
        };
        let mut calls = CheckedCallVisitor::new(Some(self.names), None, self.targets);
        for expression in &arguments.checked {
            calls.visit_expr(expression);
        }
        self.found.verified_path |= calls.path;
        self.found.verified_method |= calls.method;
        self.found.delegated |= calls.delegated;
        self.found.opaque |= calls.opaque
            || arguments
                .unchecked
                .iter()
                .any(|tokens| self.names.tokens_use_name(tokens.clone()));
    }
}

/// Finds calls of the target in the checked arguments of an assert, reading a
/// nested macro as `macro_arguments` does. The target's name as a value there is
/// `opaque`, as outside a macro.
///
/// Derivation (not stated in DS-615 / DS-623 / DS-628 / DS-1680): a call verifies
/// the target only where its value can reach the compared value, which is `flows`,
/// and only the positions listed here flow; any other position is `dropping` and a
/// call there is `opaque` (UNKNOWN, DS-615: that the value is verified is not
/// shown). The listed positions are the argument expression itself; the operand of
/// a unary or binary operator (not of an assignment, nor an operand of `||` / `|` /
/// `&&` / `&` whose other operand fixes the result, DS-1948); the receiver of a method call (DS-1937); the
/// arguments of a value conversion (DS-1960, DS-1961: a call whose callee's final
/// segment is `Some`, `Ok` or `Err`, or `Box::new` / `String::from`); a reference, a cast, `?`,
/// `.await` and parentheses; the base of a field access and the base and the index
/// of an index expression (the index decides the element, as an argument does); the tail
/// expression of a block; the tail expression of each branch of an `if` or a
/// `match`; the initializer of a `let` whose single name the tail expression of the
/// block uses where it flows; and the elements of a tuple, array or struct literal,
/// except that a literal projected directly (`(a, b).1`, `S { a, b }.b`,
/// `[a, b][1]`, `vec![a, b][1]`) passes only the element that is projected, and none
/// when that cannot be told. A closure body is not a flowing position (DS-629 reads
/// dataflow inside a closure as UNKNOWN) and neither is a `match` scrutinee (outside
/// the range DS-628 gives the tracking, so DS-615 reads it as UNKNOWN). A chain of projections (`((a, b), c).0.1`) is applied
/// step by step. The same test applied to the initializer of a `let` decides whether
/// its binding is derived from the target's result (`find_let_bindings_containing_call`).
/// A `let` name is followed only through the tail expression of its own
/// block, one binding deep (DS-628's `let` binding, bounded as the rest of DA-003
/// is). The same positions decide whether the name of a `let` outside the assert
/// reaches it (`probe`, `binding_reach`).
///
/// DS-1960: any other function call (a callee that is not a path, or a path that
/// is neither a value conversion nor a declared target, `Foo(x)` and `T::new(x)`
/// included, as a constructor is not told from the name's shape, DS-1967) and a
/// method call that is not a declared target take a value passed as an argument
/// as the Test's delegation target (DS-641): a value that flows into such an
/// argument is `delegated`, not carried to the compared value. The arguments of a
/// declared target's call are neither a value conversion nor a delegation target
/// (DS-1960), so they are not a flowing position either.
struct CheckedCallVisitor<'a> {
    /// The target whose calls are looked for; `None` while probing a `let` name.
    names: Option<&'a NameUse<'a>>,
    /// The name of a `let` whose use is probed, instead of the target's calls.
    probe: Option<&'a str>,
    /// Every declared target's symbol, whose calls are not delegation targets.
    targets: &'a [String],
    flows: bool,
    /// Inside an argument of a delegation target (DS-1960).
    delegating: bool,
    path: bool,
    method: bool,
    opaque: bool,
    /// A call of the target flows into an argument of a delegation target.
    delegated: bool,
    /// `probe` is used where it flows.
    probe_flows: bool,
    /// `probe` flows into an argument of a delegation target.
    probe_delegated: bool,
    /// `probe` is used where it does not flow.
    probe_dropped: bool,
    /// Inside the argument of a declared target's call (DS-1960).
    into_target: bool,
    /// `probe` is used only inside the argument of a declared target's call.
    probe_into_target: bool,
    /// `probe` is bound or assigned again.
    probe_rebound: bool,
    /// The paths of `probe` assigned by projection assignments (DS-1956).
    probe_overwrites: &'a [Vec<PlaceStep>],
    /// `probe` is read where it flows, but the read value is not shown derived
    /// (`read_is_derived`).
    probe_overwritten: bool,
    /// The expression is a value that `assert_eq!` / `assert_ne!` or `==` / `!=`
    /// compares whole, through parentheses, references and projections only.
    in_compared: bool,
    /// The expression is inside the base of a projection that does not start at
    /// `probe` itself (`(&r).f`, `{ r }.f`), or inside a block-local `let`
    /// initializer, so the path that a read of `probe` there leads to is not known.
    path_unknown: bool,
    /// The expression is the initializer of a `let` (`find_let_bindings_containing_call`), where
    /// calls in macro tokens are not read.
    init_mode: bool,
}

/// One step of a field access or an index expression, applied to a literal.
enum Projection<'e> {
    Member(&'e syn::Member),
    /// An index that is an integer literal, or `None` when it is not one.
    Index(Option<usize>),
}

impl Projection<'_> {
    fn place_step(&self) -> PlaceStep {
        match self {
            Projection::Member(syn::Member::Named(name)) => PlaceStep::Field(name.to_string()),
            Projection::Member(syn::Member::Unnamed(index)) => PlaceStep::Tuple(index.index),
            Projection::Index(Some(index)) => PlaceStep::Index(*index),
            Projection::Index(None) => PlaceStep::Undetermined,
        }
    }
}

/// One step of the path of a place or a read (DS-1956): a field name, a tuple
/// number, an integer-literal index, or a step not fixed by syntax.
#[derive(Clone, Debug, PartialEq, Eq)]
enum PlaceStep {
    Field(String),
    Tuple(u32),
    Index(usize),
    Undetermined,
}

/// Whether a read of the path `read` from a binding whose whole value was derived
/// is derived, after projection assignments to the paths `overwrites` (DS-1956).
///
/// A read under an assigned path is not shown derived: the assigned value is not
/// followed, and statement order and branches are not read, so the read may see
/// the assigned value (DS-1956, DS-1966). A read whose value holds an assigned
/// position and other positions is derived only where it is the value compared
/// whole (`compared`: the false PASS DS-1939 and DS-1956 disclose). Anywhere else
/// (a method receiver, an operand of another operator, a pattern match, a macro or a
/// constructor argument) it is not shown derived: DS-1939 reads a method over such
/// a value as UNKNOWN, and the other uses may take the assigned position alone. A
/// read through a step not fixed by syntax may take an assigned position or not
/// (DS-1939: UNKNOWN unless every position is derived). An index and a field of one
/// value are not told apart by syntax as positions: an index of a type other than an
/// array or a slice is a call of its `Index` / `IndexMut` method on the whole value,
/// which may read or write any field (DS-1939's method over a value whose positions
/// differ), so a read that leaves an assigned path at a step of the other kind is not
/// derived either.
fn read_is_derived(overwrites: &[Vec<PlaceStep>], read: &[PlaceStep], compared: bool) -> bool {
    let fixed = read
        .iter()
        .position(|step| *step == PlaceStep::Undetermined)
        .unwrap_or(read.len());
    let prefix = &read[..fixed];
    let is_index = |step: &PlaceStep| matches!(step, PlaceStep::Index(_));
    let leaves_by_other_kind = |path: &Vec<PlaceStep>| {
        let common = path.iter().zip(prefix).take_while(|(a, b)| a == b).count();
        path.get(common)
            .zip(prefix.get(common))
            .is_some_and(|(assigned, read)| is_index(assigned) != is_index(read))
    };
    if overwrites
        .iter()
        .any(|path| prefix.starts_with(path) || leaves_by_other_kind(path))
    {
        return false;
    }
    let holds_assigned = overwrites.iter().any(|path| path.starts_with(prefix));
    !(holds_assigned && (!compared || fixed < read.len()))
}

/// How the value of a call's argument goes on (DS-1960).
#[derive(Clone, Copy)]
enum ArgumentFlow {
    /// A value conversion: the argument flows as the call's value does.
    Flows,
    /// A message argument (DS-1935): the argument is neither carried nor delegated.
    Dropped,
    /// A declared target's call: the argument is neither carried nor delegated
    /// (DS-1960).
    IntoTarget,
    /// A delegation target: the argument is handed to it (DS-641).
    Delegated,
}

impl<'a> CheckedCallVisitor<'a> {
    fn new(names: Option<&'a NameUse<'a>>, probe: Option<&'a str>, targets: &'a [String]) -> Self {
        Self {
            names,
            probe,
            targets,
            flows: true,
            delegating: false,
            path: false,
            method: false,
            opaque: false,
            delegated: false,
            probe_flows: false,
            probe_delegated: false,
            probe_dropped: false,
            into_target: false,
            probe_into_target: false,
            probe_rebound: false,
            probe_overwrites: &[],
            probe_overwritten: false,
            in_compared: false,
            path_unknown: false,
            init_mode: false,
        }
    }

    /// Records a read of `probe` that projects the path `steps` (innermost first)
    /// where `self.flows` says.
    fn probe_read(&mut self, steps: &[Projection<'_>]) {
        let mut path: Vec<PlaceStep> = steps.iter().map(Projection::place_step).collect();
        if self.path_unknown {
            path.push(PlaceStep::Undetermined);
        }
        let derived = read_is_derived(self.probe_overwrites, &path, self.in_compared);
        if !derived && self.flows && !self.delegating {
            self.probe_overwritten = true;
        } else if self.flows && self.delegating {
            self.probe_delegated = true;
        } else if self.flows {
            self.probe_flows = true;
        } else if self.into_target {
            self.probe_into_target = true;
        } else {
            self.probe_dropped = true;
        }
    }

    /// Visits `visit` with `self.path_unknown` set.
    fn path_unknown(&mut self, visit: impl FnOnce(&mut Self)) {
        let outer = self.path_unknown;
        self.path_unknown = true;
        visit(self);
        self.path_unknown = outer;
    }

    /// Records a call of the looked-for target found where `self.flows` says.
    fn found_target_call(&mut self, by_method: bool) {
        if !self.flows {
            self.opaque = true;
        } else if self.delegating {
            self.delegated = true;
        } else if by_method {
            self.method = true;
        } else {
            self.path = true;
        }
    }

    fn visit_argument(&mut self, argument: &syn::Expr, flow: ArgumentFlow) {
        match flow {
            ArgumentFlow::Flows => self.visit_expr(argument),
            ArgumentFlow::Dropped => self.dropping(|visitor| visitor.visit_expr(argument)),
            ArgumentFlow::IntoTarget => {
                let outer = self.into_target;
                self.into_target = true;
                self.dropping(|visitor| visitor.visit_expr(argument));
                self.into_target = outer;
            }
            ArgumentFlow::Delegated => {
                let outer = self.delegating;
                self.delegating = true;
                self.visit_expr(argument);
                self.delegating = outer;
            }
        }
    }

    fn is_declared_target(&self, name: &syn::Ident) -> bool {
        self.targets.iter().any(|target| name == target)
    }

    /// Visits the index of `node` as flowing and returns the step it makes.
    ///
    /// Derivation (from DS-623): the value of the index decides which element is
    /// compared (`function[range]`), so it flows.
    fn index_projection(&mut self, node: &syn::ExprIndex) -> Projection<'static> {
        let outer = self.in_compared;
        self.in_compared = false;
        self.visit_expr(&node.index);
        self.in_compared = outer;
        Projection::Index(match node.index.as_ref() {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Int(index),
                ..
            }) => index.base10_parse::<usize>().ok(),
            _ => None,
        })
    }

    /// Visits a chain of field accesses and index expressions over `base` (`first`
    /// is the outermost step). Derivation (DS-623: the value of a projection is the
    /// projected element's): the steps are applied to a literal one after another, so
    /// only the element they select flows (`((a, b), c).0.1` passes `b`).
    fn visit_projections<'e>(&mut self, base: &'e syn::Expr, mut steps: Vec<Projection<'e>>) {
        let mut base = unwrap_parens(base);
        loop {
            match base {
                syn::Expr::Field(field) => {
                    steps.push(Projection::Member(&field.member));
                    base = unwrap_parens(&field.base);
                }
                syn::Expr::Index(index) => {
                    let step = self.index_projection(index);
                    steps.push(step);
                    base = unwrap_parens(&index.expr);
                }
                _ => break,
            }
        }
        steps.reverse();
        self.visit_projected(base, &steps);
    }

    /// Visits `base` with the elements that `steps` (innermost first) select as
    /// flowing, and the others as dropped. A base that is not a literal passes its
    /// value whole.
    fn visit_projected(&mut self, base: &syn::Expr, steps: &[Projection<'_>]) {
        let base = unwrap_parens(base);
        if let (Some(name), syn::Expr::Path(path)) = (self.probe, base) {
            if path.qself.is_none() && path.path.is_ident(name) && !steps.is_empty() {
                return self.probe_read(steps);
            }
        }
        let Some((first, rest)) = steps.split_first() else {
            return self.visit_expr(base);
        };
        let selected = match first {
            Projection::Index(index) => *index,
            Projection::Member(syn::Member::Unnamed(member)) => Some(member.index as usize),
            Projection::Member(syn::Member::Named(_)) => None,
        };
        match base {
            syn::Expr::Tuple(tuple) => {
                let by_member = matches!(first, Projection::Member(_));
                for (index, element) in tuple.elems.iter().enumerate() {
                    self.visit_selected(element, by_member && selected == Some(index), rest);
                }
            }
            syn::Expr::Array(array) => {
                let by_index = matches!(first, Projection::Index(_));
                for (index, element) in array.elems.iter().enumerate() {
                    self.visit_selected(element, by_index && selected == Some(index), rest);
                }
            }
            syn::Expr::Struct(literal) => {
                for field in &literal.fields {
                    let projected = match (first, &field.member) {
                        (Projection::Member(syn::Member::Named(a)), syn::Member::Named(b)) => {
                            a == b
                        }
                        (Projection::Member(syn::Member::Unnamed(a)), syn::Member::Unnamed(b)) => {
                            a.index == b.index
                        }
                        _ => false,
                    };
                    self.visit_selected(&field.expr, projected, rest);
                }
                if let Some(base) = &literal.rest {
                    self.visit_element(base, false);
                }
            }
            syn::Expr::Macro(node) if node.mac.path.is_ident("vec") => {
                if self.init_mode || !self.macro_relevant(&node.mac) {
                    return;
                }
                let Some(arguments) = macro_arguments(&node.mac) else {
                    self.opaque = true;
                    return;
                };
                let by_index = matches!(first, Projection::Index(_));
                for (index, element) in arguments.checked.iter().enumerate() {
                    self.visit_selected(element, by_index && selected == Some(index), rest);
                }
            }
            syn::Expr::Repeat(_) => self.dropping(|visitor| visitor.visit_expr(base)),
            // The steps are not applied to a value read here, so a read of `probe`
            // in it does not know its path (DS-1939).
            _ => self.path_unknown(|visitor| visitor.visit_expr(base)),
        }
    }

    /// Visits an element of a literal: the selected one goes on with the remaining
    /// steps, any other is dropped.
    fn visit_selected(&mut self, element: &syn::Expr, selected: bool, rest: &[Projection<'_>]) {
        if selected {
            self.visit_projected(element, rest);
        } else {
            self.visit_element(element, false);
        }
    }

    /// Whether the tokens of a macro mention what is looked for.
    fn macro_relevant(&self, node: &syn::Macro) -> bool {
        match (self.probe, self.names) {
            (Some(name), _) => tokens_mention_ident(node.tokens.clone(), name),
            (None, Some(names)) => names.tokens_use_name(node.tokens.clone()),
            (None, None) => false,
        }
    }

    /// Visits `element` as `flows` says, for a literal's elements.
    fn visit_element(&mut self, element: &syn::Expr, flows: bool) {
        if flows {
            self.visit_expr(element);
        } else {
            self.dropping(|visitor| visitor.visit_expr(element));
        }
    }

    /// Visits the qualified self type and the generic arguments of a path, where a
    /// call (`f::<{ g() }>`) is not carried to the path's value.
    fn visit_path_generics(&mut self, node: &syn::ExprPath) {
        self.dropping(|visitor| {
            if let Some(qself) = &node.qself {
                visitor.visit_qself(qself);
            }
            for segment in &node.path.segments {
                visitor.visit_path_arguments(&segment.arguments);
            }
        });
    }

    fn dropping(&mut self, visit: impl FnOnce(&mut Self)) {
        let outer = self.flows;
        self.flows = false;
        visit(self);
        self.flows = outer;
    }

    /// Whether the value of `local`'s initializer reaches the tail expression of
    /// its block: a single name bound once, used where the tail expression flows,
    /// and not assigned or bound again in the statements after it. `Delegated`
    /// when the tail hands the name to a delegation target (DS-1960).
    fn local_reaches_tail(&self, local: &syn::Local, rest: &[syn::Stmt]) -> TailReach {
        let mut pattern = &local.pat;
        while let syn::Pat::Type(typed) = pattern {
            pattern = &typed.pat;
        }
        let syn::Pat::Ident(binding) = pattern else {
            return TailReach::No;
        };
        let Some((tail, statements)) = rest.split_last() else {
            return TailReach::No;
        };
        let syn::Stmt::Expr(tail, None) = tail else {
            return TailReach::No;
        };
        // A mutable binding may be changed before the tail expression reads it.
        if binding.by_ref.is_some()
            || binding.mutability.is_some()
            || binding.subpat.is_some()
            || local
                .init
                .as_ref()
                .is_none_or(|init| init.diverge.is_some())
        {
            return TailReach::No;
        }
        let name = binding.ident.to_string();
        let mut probe = CheckedCallVisitor::new(None, Some(&name), self.targets);
        probe.flows = false;
        for statement in statements {
            probe.visit_stmt(statement);
        }
        probe.flows = true;
        probe.visit_expr(tail);
        if probe.probe_rebound {
            TailReach::No
        } else if probe.probe_flows {
            // DS-623 / DS-1962: a use that flows decides before a delegated one.
            TailReach::Flows
        } else if probe.probe_delegated {
            TailReach::Delegated
        } else {
            TailReach::No
        }
    }
}

/// How the name of a block-local `let` reaches the tail expression of its block.
enum TailReach {
    No,
    Flows,
    /// The tail hands the name to a delegation target (DS-1960).
    Delegated,
}

impl<'ast> Visit<'ast> for CheckedCallVisitor<'_> {
    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        let outer_compared = self.in_compared;
        self.in_compared &= matches!(
            node,
            syn::Expr::Paren(_)
                | syn::Expr::Group(_)
                | syn::Expr::Reference(_)
                | syn::Expr::Field(_)
                | syn::Expr::Index(_)
                | syn::Expr::Path(_)
        );
        match node {
            syn::Expr::Path(_)
            | syn::Expr::Lit(_)
            | syn::Expr::Paren(_)
            | syn::Expr::Group(_)
            | syn::Expr::Reference(_)
            | syn::Expr::Cast(_)
            | syn::Expr::Try(_)
            | syn::Expr::Await(_)
            | syn::Expr::Unary(_)
            | syn::Expr::Binary(_)
            | syn::Expr::Call(_)
            | syn::Expr::MethodCall(_)
            | syn::Expr::Field(_)
            | syn::Expr::Index(_)
            | syn::Expr::Block(_)
            | syn::Expr::Unsafe(_)
            | syn::Expr::Const(_)
            | syn::Expr::If(_)
            | syn::Expr::Match(_)
            | syn::Expr::Tuple(_)
            | syn::Expr::Array(_)
            | syn::Expr::Struct(_)
            | syn::Expr::Macro(_) => syn::visit::visit_expr(self, node),
            // Every other position (a closure, a loop, a range, `let`, `return`,
            // `break`, an assignment, `async`, ...) drops or holds the value.
            _ => self.dropping(|visitor| syn::visit::visit_expr(visitor, node)),
        }
        self.in_compared = outer_compared;
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        let flow = if let syn::Expr::Path(path) = node.func.as_ref() {
            let last = path.path.segments.last().map(|segment| &segment.ident);
            if self
                .names
                .is_some_and(|names| last.is_some_and(|name| name == names.symbol))
            {
                self.found_target_call(false);
            }
            // A probed `let` name may be the callee (`let f = |x| ..; f(2)`).
            if self.probe.is_some() {
                self.visit_expr(&node.func);
            } else {
                self.visit_path_generics(path);
            }
            if is_value_conversion(&path.path) {
                ArgumentFlow::Flows
            } else if last.is_some_and(|name| self.is_declared_target(name)) {
                ArgumentFlow::IntoTarget
            } else {
                ArgumentFlow::Delegated
            }
        } else {
            self.visit_expr(&node.func);
            ArgumentFlow::Delegated
        };
        for argument in &node.args {
            self.visit_argument(argument, flow);
        }
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if self.names.is_some_and(|names| node.method == names.symbol) {
            self.found_target_call(true);
        }
        // DS-1937: the receiver flows to the method's value.
        self.visit_expr(&node.receiver);
        if let Some(turbofish) = &node.turbofish {
            self.dropping(|visitor| visitor.visit_angle_bracketed_generic_arguments(turbofish));
        }
        // DS-619's `.expect(..)` / `.expect_err(..)` is an assert-equivalent, not a
        // delegation target (DS-641), and its argument is a message (DS-1935).
        let is_assert_equivalent = node.method == "expect" || node.method == "expect_err";
        let flow = if is_assert_equivalent {
            ArgumentFlow::Dropped
        } else if self.is_declared_target(&node.method) {
            ArgumentFlow::IntoTarget
        } else {
            ArgumentFlow::Delegated
        };
        for argument in &node.args {
            self.visit_argument(argument, flow);
        }
    }

    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        self.visit_path_generics(node);
        if let Some(name) = self.probe {
            if node.qself.is_none() && node.path.is_ident(name) {
                self.probe_read(&[]);
            }
            return;
        }
        if self
            .names
            .is_some_and(|names| names.path_is_value_use(node))
        {
            self.opaque = true;
        }
    }

    fn visit_expr_field(&mut self, node: &'ast syn::ExprField) {
        self.visit_projections(&node.base, vec![Projection::Member(&node.member)]);
    }

    fn visit_expr_index(&mut self, node: &'ast syn::ExprIndex) {
        let projection = self.index_projection(node);
        self.visit_projections(&node.expr, vec![projection]);
    }

    fn visit_expr_struct(&mut self, node: &'ast syn::ExprStruct) {
        for field in &node.fields {
            self.visit_expr(&field.expr);
        }
        if let Some(rest) = &node.rest {
            self.visit_element(rest, false);
        }
    }

    fn visit_pat_ident(&mut self, node: &'ast syn::PatIdent) {
        if self.probe.is_some_and(|name| node.ident == name) {
            self.probe_rebound = true;
        }
        syn::visit::visit_pat_ident(self, node);
    }

    fn visit_block(&mut self, node: &'ast syn::Block) {
        let outer = self.flows;
        let outer_delegating = self.delegating;
        for (index, statement) in node.stmts.iter().enumerate() {
            let mut delegates = false;
            self.flows = outer
                && match statement {
                    syn::Stmt::Expr(_, None) => index + 1 == node.stmts.len(),
                    syn::Stmt::Local(local) => {
                        match self.local_reaches_tail(local, &node.stmts[index + 1..]) {
                            TailReach::Flows => true,
                            TailReach::Delegated => {
                                delegates = true;
                                true
                            }
                            TailReach::No => false,
                        }
                    }
                    syn::Stmt::Expr(_, Some(_)) | syn::Stmt::Item(_) | syn::Stmt::Macro(_) => false,
                };
            self.delegating = outer_delegating || delegates;
            if matches!(statement, syn::Stmt::Local(_)) {
                // The initializer's value goes on under another name, whose
                // projections are not followed back to `probe` (DS-1939).
                self.path_unknown(|visitor| visitor.visit_stmt(statement));
            } else {
                self.visit_stmt(statement);
            }
        }
        self.flows = outer;
        self.delegating = outer_delegating;
    }

    fn visit_expr_if(&mut self, node: &'ast syn::ExprIf) {
        self.dropping(|visitor| visitor.visit_expr(&node.cond));
        self.visit_block(&node.then_branch);
        if let Some((_, other)) = &node.else_branch {
            self.visit_expr(other);
        }
    }

    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        self.dropping(|visitor| visitor.visit_expr(&node.expr));
        for arm in &node.arms {
            self.visit_pat(&arm.pat);
            if let Some((_, guard)) = &arm.guard {
                self.dropping(|visitor| visitor.visit_expr(guard));
            }
            self.visit_expr(&arm.body);
        }
    }

    fn visit_expr_assign(&mut self, node: &'ast syn::ExprAssign) {
        if self
            .probe
            .is_some_and(|name| is_assignment_to(&node.left, name))
        {
            self.probe_rebound = true;
        }
        self.dropping(|visitor| syn::visit::visit_expr_assign(visitor, node));
    }

    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        if is_compound_assignment(node.op) {
            if self
                .probe
                .is_some_and(|name| is_assignment_to(&node.left, name))
            {
                self.probe_rebound = true;
            }
            self.dropping(|visitor| syn::visit::visit_expr_binary(visitor, node));
        } else if let Some(absorbing) = absorbing_literal(node.op) {
            // DS-1948: an operand whose sibling fixes the result is not a position
            // the value flows from.
            for (operand, sibling) in [(&node.left, &node.right), (&node.right, &node.left)] {
                self.visit_element(operand, operand_flows(sibling, absorbing));
            }
        } else if matches!(node.op, syn::BinOp::Eq(_) | syn::BinOp::Ne(_)) {
            // `==` / `!=` compares each operand whole, as `assert_eq!` does.
            for operand in [&node.left, &node.right] {
                let outer = self.in_compared;
                self.in_compared = true;
                self.visit_expr(operand);
                self.in_compared = outer;
            }
        } else {
            syn::visit::visit_expr_binary(self, node);
        }
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if self.init_mode || !self.macro_relevant(node) {
            return;
        }
        let name = node
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_default();
        // `unreachable!` / `todo!` / `unimplemented!` inside an assert are not read:
        // a use of the value there is opaque (DS-615).
        let Some(arguments) = macro_arguments(node).filter(|_| !is_diverging_macro(&name)) else {
            self.opaque = true;
            return;
        };
        // DS-1954, DS-1962: a `debug_assert*` is not one of DS-617's macros, so a
        // value in its arguments is in no position that reaches the assert.
        let reaches = !is_debug_assert(&name);
        for expression in &arguments.checked {
            self.visit_element(expression, reaches);
        }
        if let Some(names) = self.names {
            self.opaque |= arguments
                .unchecked
                .iter()
                .any(|tokens| names.tokens_use_name(tokens.clone()));
        }
    }
}

/// Whether a callee path is a value conversion of DS-1960: a constructor known by
/// name (`Some`, `Ok`, `Err`, DS-1961) or the standard conversion `Box::new` /
/// `String::from` (DS-1961). Any other path, a tuple struct or a variant included,
/// is a function call, since a constructor is not told from the shape of a name
/// (DS-1960, DS-1967).
fn is_value_conversion(path: &syn::Path) -> bool {
    let mut segments = path.segments.iter().rev();
    let Some(last) = segments.next() else {
        return false;
    };
    if last.ident == "Some" || last.ident == "Ok" || last.ident == "Err" {
        return true;
    }
    let owner = segments.next();
    owner.is_some_and(|owner| {
        (owner.ident == "Box" && last.ident == "new")
            || (owner.ident == "String" && last.ident == "from")
    })
}

/// The expression inside any parentheses around `expression`.
fn unwrap_parens(expression: &syn::Expr) -> &syn::Expr {
    match expression {
        syn::Expr::Paren(inner) => unwrap_parens(&inner.expr),
        syn::Expr::Group(inner) => unwrap_parens(&inner.expr),
        other => other,
    }
}

/// DS-1948, DS-1949: the literal that fixes the result of `rust-cargo`'s `||` / `|`
/// (`true`) or `&&` / `&` (`false`) whatever the other operand is.
fn absorbing_literal(op: syn::BinOp) -> Option<bool> {
    match op {
        syn::BinOp::Or(_) | syn::BinOp::BitOr(_) => Some(true),
        syn::BinOp::And(_) | syn::BinOp::BitAnd(_) => Some(false),
        _ => None,
    }
}

/// Whether literals fix the value of an expression whatever its other operands are.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Fixed {
    /// Fixed to this `bool`.
    Value(bool),
    /// A `bool` made only of literals, whose value is not evaluated here.
    Literal,
    /// Made only of literals, but not a `bool` by syntax (`0`, `1 + 1`).
    Constant,
    /// Holds a macro other than `matches!` whose tokens are literals only
    /// (`m!()`, `four_m!() == 4`): its expansion may run any code (DS-629), so it
    /// may absorb and its constancy is not decided.
    Opaque,
    /// Depends on something other than literals.
    Open,
}

/// The value literals fix (DS-1949): a `bool` literal; `!` of a fixed value;
/// `||` / `|` / `&&` / `&` with an absorbing operand (DS-1948) or with both
/// operands fixed; `^`, `==`, `!=` of fixed `bool`s; a comparison of two integer,
/// string or char literals of one kind (not floats). Parentheses do not change it.
/// Only literals, `!`, binary operators and parentheses are evaluated; another form
/// built from literals only (`if true { .. }`, `{ true }`, `matches!(1, 1)`) is
/// `Fixed::Literal`; one holding another macro is `Fixed::Opaque`.
///
/// Derivation from DS-621 and DS-1949: an operand fixed this way (`!false`,
/// `1 == 1`, `(true || ok)`) absorbs as the literal it equals, so `X || 1 == 1` is
/// not read as `X` flowing. An operand made only of literals whose value is not
/// evaluated here (`1 + 1 == 2`) may absorb (`Fixed::Literal`).
fn fixed_value(expression: &syn::Expr) -> Fixed {
    match unwrap_parens(expression) {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Bool(literal),
            ..
        }) => Fixed::Value(literal.value),
        syn::Expr::Lit(_) => Fixed::Constant,
        syn::Expr::Unary(unary) => match (unary.op, fixed_value(&unary.expr)) {
            (syn::UnOp::Not(_), Fixed::Value(value)) => Fixed::Value(!value),
            (syn::UnOp::Not(_), Fixed::Literal) => Fixed::Literal,
            (_, inner @ (Fixed::Open | Fixed::Opaque)) => inner,
            _ => Fixed::Constant,
        },
        syn::Expr::Binary(binary) => {
            let left = fixed_value(&binary.left);
            let right = fixed_value(&binary.right);
            if let Some(absorbing) = absorbing_literal(binary.op) {
                if left == Fixed::Value(absorbing) || right == Fixed::Value(absorbing) {
                    return Fixed::Value(absorbing);
                }
                if let (Fixed::Value(_), Fixed::Value(_)) = (left, right) {
                    return Fixed::Value(!absorbing);
                }
            }
            if let (Fixed::Value(a), Fixed::Value(b)) = (left, right) {
                match binary.op {
                    syn::BinOp::BitXor(_) | syn::BinOp::Ne(_) => return Fixed::Value(a != b),
                    syn::BinOp::Eq(_) => return Fixed::Value(a == b),
                    _ => {}
                }
            }
            if let Some(value) = compare_literals(binary) {
                return Fixed::Value(value);
            }
            if left == Fixed::Open || right == Fixed::Open {
                return Fixed::Open;
            }
            if left == Fixed::Opaque || right == Fixed::Opaque {
                return Fixed::Opaque;
            }
            let is_bool = |side: Fixed| matches!(side, Fixed::Value(_) | Fixed::Literal);
            match binary.op {
                syn::BinOp::Eq(_)
                | syn::BinOp::Ne(_)
                | syn::BinOp::Lt(_)
                | syn::BinOp::Le(_)
                | syn::BinOp::Gt(_)
                | syn::BinOp::Ge(_)
                | syn::BinOp::And(_)
                | syn::BinOp::Or(_) => Fixed::Literal,
                syn::BinOp::BitOr(_) | syn::BinOp::BitAnd(_) | syn::BinOp::BitXor(_)
                    if is_bool(left) || is_bool(right) =>
                {
                    Fixed::Literal
                }
                _ => Fixed::Constant,
            }
        }
        // Any other form with no variable, call or identifier in macro tokens
        // (`if true { true } else { false }`, `{ true }`, `matches!(1, 1)`) is built
        // from literals only, so it may absorb (DS-1948 leaves only non-literal
        // constants as a false PASS). A macro other than `matches!` may expand to
        // any code (DS-629), so a form holding one is `Opaque`, not a literal.
        other => {
            let mut finder = NonLiteralFinder {
                found: false,
                opaque_macro: false,
            };
            finder.visit_expr(other);
            if finder.found {
                Fixed::Open
            } else if finder.opaque_macro {
                Fixed::Opaque
            } else {
                Fixed::Literal
            }
        }
    }
}

/// Whether an expression refers to a variable, calls something (a struct literal
/// and a closure included, whose comparison or call runs user code), or has an
/// identifier other than `true` / `false` in macro tokens.
struct NonLiteralFinder {
    found: bool,
    /// A macro other than `matches!` with literal-only tokens.
    opaque_macro: bool,
}

impl<'ast> Visit<'ast> for NonLiteralFinder {
    fn visit_expr_path(&mut self, _node: &'ast syn::ExprPath) {
        self.found = true;
    }

    fn visit_expr_struct(&mut self, _node: &'ast syn::ExprStruct) {
        self.found = true;
    }

    fn visit_expr_closure(&mut self, _node: &'ast syn::ExprClosure) {
        self.found = true;
    }

    fn visit_expr_call(&mut self, _node: &'ast syn::ExprCall) {
        self.found = true;
    }

    fn visit_expr_method_call(&mut self, _node: &'ast syn::ExprMethodCall) {
        self.found = true;
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        fn has_ident(tokens: proc_macro2::TokenStream) -> bool {
            tokens.into_iter().any(|tree| match tree {
                proc_macro2::TokenTree::Ident(ident) => ident != "true" && ident != "false",
                proc_macro2::TokenTree::Group(group) => has_ident(group.stream()),
                proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => false,
            })
        }
        self.found |= has_ident(node.tokens.clone());
        self.opaque_macro |= !node.path.is_ident("matches");
    }
}

/// The value of a comparison of two literals of one kind (integer, string, char),
/// or `None` when it is not one. Float literals are not compared: whether the
/// comparison is in `f32` or `f64` is not fixed by syntax, so such a comparison is
/// left unevaluated (`Fixed::Literal`).
fn compare_literals(binary: &syn::ExprBinary) -> Option<bool> {
    use std::cmp::Ordering;
    let literal = |expression: &syn::Expr| match unwrap_parens(expression) {
        syn::Expr::Lit(syn::ExprLit { lit, .. }) => Some(lit.clone()),
        _ => None,
    };
    let ordering = match (literal(&binary.left)?, literal(&binary.right)?) {
        (syn::Lit::Int(a), syn::Lit::Int(b)) => a
            .base10_parse::<i128>()
            .ok()?
            .cmp(&b.base10_parse::<i128>().ok()?),
        (syn::Lit::Str(a), syn::Lit::Str(b)) => a.value().cmp(&b.value()),
        (syn::Lit::Char(a), syn::Lit::Char(b)) => a.value().cmp(&b.value()),
        _ => return None,
    };
    match binary.op {
        syn::BinOp::Eq(_) => Some(ordering == Ordering::Equal),
        syn::BinOp::Ne(_) => Some(ordering != Ordering::Equal),
        syn::BinOp::Lt(_) => Some(ordering == Ordering::Less),
        syn::BinOp::Le(_) => Some(ordering != Ordering::Greater),
        syn::BinOp::Gt(_) => Some(ordering == Ordering::Greater),
        syn::BinOp::Ge(_) => Some(ordering != Ordering::Less),
        _ => None,
    }
}

/// Whether an operand of `||` / `|` / `&&` / `&` beside `sibling` is a position
/// the value flows from (DS-1948): not when `sibling` is fixed to the absorbing
/// value, nor when it is made only of literals whose value is not evaluated.
fn operand_flows(sibling: &syn::Expr, absorbing: bool) -> bool {
    match fixed_value(sibling) {
        Fixed::Value(value) => value != absorbing,
        Fixed::Literal | Fixed::Opaque => false,
        Fixed::Constant | Fixed::Open => true,
    }
}

/// DA-001's constancy of one argument (DS-621, DS-1949): `AllLiteral` when literals
/// fix it; `Undetermined` when it holds a macro whose expansion is not read
/// (DS-629), or when an operand of `||` / `|` / `&&` / `&` in it is made only of
/// literals whose value is not evaluated or holds such a macro, so it may absorb
/// the rest.
fn argument_constancy(argument: &syn::Expr) -> Constancy {
    match fixed_value(argument) {
        Fixed::Open => {}
        Fixed::Opaque => return Constancy::Undetermined,
        Fixed::Value(_) | Fixed::Literal | Fixed::Constant => return Constancy::AllLiteral,
    }
    let mut finder = UnevaluatedAbsorberFinder { found: false };
    finder.visit_expr(argument);
    if finder.found {
        Constancy::Undetermined
    } else {
        Constancy::NotAllLiteral
    }
}

struct UnevaluatedAbsorberFinder {
    found: bool,
}

impl<'ast> Visit<'ast> for UnevaluatedAbsorberFinder {
    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        if absorbing_literal(node.op).is_some() {
            let may_absorb =
                |side: &syn::Expr| matches!(fixed_value(side), Fixed::Literal | Fixed::Opaque);
            self.found |= may_absorb(&node.left) || may_absorb(&node.right);
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
    syn::parse_str::<syn::ItemFn>(text).map_or(0, |function| count_rebindings_in(&function, name))
}

fn count_rebindings_in(function: &syn::ItemFn, name: &str) -> usize {
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
    let mut counter = Counter { name, count: 0 };
    counter.visit_item_fn(function);
    counter.count
}

/// How the function changes the value of a `let mut` binding through something other
/// than the name itself (`name = ..` is `count_rebindings`').
#[derive(Default)]
struct PlaceChanges {
    /// The whole binding is not shown to keep the target's result (DS-1959).
    whole: bool,
    /// The paths assigned by projection assignments whose path is fixed by syntax
    /// (DS-1956), and those of compound assignments whose order against such an
    /// assignment would decide their derivation.
    overwrites: Vec<Vec<PlaceStep>>,
}

/// The changes the function makes to the `let mut` binding `name` through a place.
///
/// A projection assignment `name.p = e` or `name.p op= e` whose path `p` is made of
/// field names, tuple numbers and integer-literal indexes only, outside a closure, an
/// `async` block and macro tokens, changes only the position `p` (DS-1956). A
/// compound assignment keeps `p` derived, as the original value is, unless a plain
/// assignment is made to a path that is a prefix of `p` or has `p` as a prefix: with
/// statement order not read, which one comes first is not known, so `p` is listed
/// as assigned too. Every other change below makes the `whole` binding not shown to
/// be the target's result (DS-1959, DS-615) when the binding is
/// - the base of an assigned place whose path is not fixed by syntax (`r[i] = ..`,
///   `*r = ..`, `(*r).x = ..`, or an element of a destructuring assignment), or a
///   projection assignment in a closure, an `async` block or macro tokens,
/// - borrowed mutably (`&mut r`, `&mut r.0`, `&raw mut r`, or a `ref mut` pattern
///   whose matched value is `r`: `let (ref mut a, _) = r`, `match r { .. }`,
///   `if let .. = r`),
/// - the receiver of a method call outside an assert (`r.clear()`): whether the
///   method takes `&mut self` is not decided by this capability, so every method
///   call on the `let mut` binding counts. A receiver inside `assert!` /
///   `assert_eq!` / `assert_ne!` is not read as a change made before the assert's
///   comparison; one inside `panic!` / `unreachable!` / `todo!` / `unimplemented!`
///   counts, or
/// - assigned as the plain name (`r = ..`, `r += ..`, also in a closure) inside a
///   macro's arguments, which `count_rebindings` does not read.
///
/// Macro arguments of `matches!`, `format!`, `vec!` and the assert macros are read
/// as expressions. DS-629 reads dataflow inside a macro expansion as UNKNOWN, so
/// the name in the tokens of any other macro outside an assert (`set!{ r.0 = 1 }`,
/// `reset!(r)`, `addr_of_mut!(r)`) counts as a change too, as its tokens are not
/// read.
///
/// Not covered: a binding of a target that returns `&mut T` need not be `let mut`
/// to be changed.
fn place_changes(text: &str, name: &str, assert_spans: &[(usize, usize)]) -> PlaceChanges {
    struct Changes<'a> {
        text: &'a str,
        name: &'a str,
        assert_spans: &'a [(usize, usize)],
        /// Depth of macro arguments being visited, where `count_rebindings` does not
        /// see an assignment to the plain name.
        in_macro: usize,
        /// Depth of closures and `async` blocks being visited, which capture the
        /// binding (DS-1959).
        in_closure: usize,
        found: bool,
        assigned: Vec<Vec<PlaceStep>>,
        compound: Vec<Vec<PlaceStep>>,
    }
    impl Changes<'_> {
        /// The path of a projection assignment to `name` that DS-1956 reads, or
        /// `None` when the left side changes the binding otherwise or not at all.
        fn fixed_projection(&self, left: &syn::Expr) -> Option<Vec<PlaceStep>> {
            if self.in_macro > 0 || self.in_closure > 0 {
                return None;
            }
            let path = fixed_place_path(left, self.name)?;
            (!path.is_empty()).then_some(path)
        }
    }
    impl<'ast> Visit<'ast> for Changes<'_> {
        fn visit_expr_assign(&mut self, node: &'ast syn::ExprAssign) {
            if let Some(path) = self.fixed_projection(&node.left) {
                self.assigned.push(path);
            } else {
                self.found |= assigned_place_roots_at(&node.left, self.name, false)
                    || (self.in_macro > 0 && is_assignment_to(&node.left, self.name));
            }
            syn::visit::visit_expr_assign(self, node);
        }

        fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
            if is_compound_assignment(node.op) {
                if let Some(path) = self.fixed_projection(&node.left) {
                    self.compound.push(path);
                } else {
                    self.found |= assigned_place_roots_at(&node.left, self.name, false)
                        || (self.in_macro > 0 && is_assignment_to(&node.left, self.name));
                }
            }
            syn::visit::visit_expr_binary(self, node);
        }

        fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
            self.in_closure += 1;
            syn::visit::visit_expr_closure(self, node);
            self.in_closure -= 1;
        }

        fn visit_expr_async(&mut self, node: &'ast syn::ExprAsync) {
            self.in_closure += 1;
            syn::visit::visit_expr_async(self, node);
            self.in_closure -= 1;
        }

        fn visit_expr_reference(&mut self, node: &'ast syn::ExprReference) {
            self.found |= node.mutability.is_some() && place_roots_at(&node.expr, self.name);
            syn::visit::visit_expr_reference(self, node);
        }

        fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
            let offset = span_offset(node.method.span().start(), self.text);
            let in_assert = self
                .assert_spans
                .iter()
                .any(|&(start, end)| offset >= start && offset < end);
            self.found |= !in_assert && place_roots_at(&node.receiver, self.name);
            syn::visit::visit_expr_method_call(self, node);
        }

        fn visit_expr_raw_addr(&mut self, node: &'ast syn::ExprRawAddr) {
            self.found |= matches!(node.mutability, syn::PointerMutability::Mut(_))
                && place_roots_at(&node.expr, self.name);
            syn::visit::visit_expr_raw_addr(self, node);
        }

        fn visit_local(&mut self, node: &'ast syn::Local) {
            self.found |= node.init.as_ref().is_some_and(|init| {
                place_roots_at(&init.expr, self.name) && pattern_borrows_mutably(&node.pat)
            });
            syn::visit::visit_local(self, node);
        }

        fn visit_expr_let(&mut self, node: &'ast syn::ExprLet) {
            self.found |=
                place_roots_at(&node.expr, self.name) && pattern_borrows_mutably(&node.pat);
            syn::visit::visit_expr_let(self, node);
        }

        fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
            self.found |= place_roots_at(&node.expr, self.name)
                && node
                    .arms
                    .iter()
                    .any(|arm| pattern_borrows_mutably(&arm.pat));
            syn::visit::visit_expr_match(self, node);
        }

        fn visit_macro(&mut self, node: &'ast syn::Macro) {
            self.in_macro += 1;
            self.visit_macro_tokens(node);
            self.in_macro -= 1;
        }
    }
    impl Changes<'_> {
        fn visit_macro_tokens(&mut self, node: &syn::Macro) {
            let offset = span_offset(node.path.span().start(), self.text);
            let in_assert = self
                .assert_spans
                .iter()
                .any(|&(start, end)| offset >= start && offset < end);
            let read = macro_arguments(node);
            self.found |= !in_assert
                && read.is_none()
                && tokens_mention_ident(node.tokens.clone(), self.name);
            if node.path.is_ident("matches") {
                // Only the first argument is an expression; the pattern and its guard
                // may bind `ref mut` or mutate the value, so the name there is a change.
                if let Some(arguments) = read {
                    for expression in &arguments.checked {
                        self.visit_expr(expression);
                    }
                    self.found |= arguments
                        .unchecked
                        .iter()
                        .any(|tokens| tokens_mention_ident(tokens.clone(), self.name));
                    let parsed = syn::parse::Parser::parse2(
                        |input: syn::parse::ParseStream<'_>| {
                            let scrutinee: syn::Expr = input.parse()?;
                            input.parse::<syn::Token![,]>()?;
                            let pattern = syn::Pat::parse_multi_with_leading_vert(input)?;
                            let guard = if input.peek(syn::Token![if]) {
                                input.parse::<syn::Token![if]>()?;
                                Some(input.parse::<syn::Expr>()?)
                            } else {
                                None
                            };
                            input.parse::<Option<syn::Token![,]>>()?;
                            Ok((scrutinee, pattern, guard))
                        },
                        node.tokens.clone(),
                    );
                    match parsed {
                        Ok((scrutinee, pattern, guard)) => {
                            self.found |= place_roots_at(&scrutinee, self.name)
                                && pattern_borrows_mutably(&pattern);
                            if let Some(guard) = guard {
                                self.visit_expr(&guard);
                            }
                        }
                        Err(_) => {
                            self.found |= tokens_mention_ident(node.tokens.clone(), self.name);
                        }
                    }
                } else {
                    self.found |= tokens_mention_ident(node.tokens.clone(), self.name);
                }
                return;
            }
            if let Ok(arguments) = syn::parse::Parser::parse2(
                syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated,
                node.tokens.clone(),
            ) {
                for argument in &arguments {
                    self.visit_expr(argument);
                }
            }
        }
    }
    let Ok(function) = syn::parse_str::<syn::ItemFn>(text) else {
        return PlaceChanges::default();
    };
    // The arguments of `panic!` / `unreachable!` / `todo!` / `unimplemented!` are
    // checked like the body: a method receiver or an unread macro there counts as a
    // change, as these macros compare nothing (DS-1959, DS-615).
    let assert_spans: Vec<(usize, usize)> = assert_spans
        .iter()
        .copied()
        .filter(|&(start, _)| {
            !macro_name_before(text, start)
                .is_some_and(|name| name == "panic" || is_diverging_macro(name))
        })
        .collect();
    let mut changes = Changes {
        text,
        name,
        assert_spans: &assert_spans,
        in_macro: 0,
        in_closure: 0,
        found: false,
        assigned: Vec::new(),
        compound: Vec::new(),
    };
    changes.visit_item_fn(&function);
    let related = |a: &[PlaceStep], b: &[PlaceStep]| a.starts_with(b) || b.starts_with(a);
    let mut overwrites = changes.assigned.clone();
    overwrites.extend(changes.compound.into_iter().filter(|path| {
        changes
            .assigned
            .iter()
            .any(|assigned| related(path, assigned))
    }));
    PlaceChanges {
        whole: changes.found,
        overwrites,
    }
}

/// The path of the place `expression` when it is `name` followed by field names,
/// tuple numbers and integer-literal indexes only (DS-1956), innermost step first;
/// `None` for any other place.
fn fixed_place_path(expression: &syn::Expr, name: &str) -> Option<Vec<PlaceStep>> {
    match unwrap_parens(expression) {
        syn::Expr::Path(path) => (path.qself.is_none() && path.path.is_ident(name)).then(Vec::new),
        syn::Expr::Field(field) => {
            let mut path = fixed_place_path(&field.base, name)?;
            path.push(match &field.member {
                syn::Member::Named(ident) => PlaceStep::Field(ident.to_string()),
                syn::Member::Unnamed(index) => PlaceStep::Tuple(index.index),
            });
            Some(path)
        }
        syn::Expr::Index(index) => {
            let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Int(literal),
                ..
            }) = unwrap_parens(&index.index)
            else {
                return None;
            };
            let mut path = fixed_place_path(&index.expr, name)?;
            path.push(PlaceStep::Index(literal.base10_parse().ok()?));
            Some(path)
        }
        _ => None,
    }
}

/// Whether a pattern binds a part of the matched value as `ref mut`.
fn pattern_borrows_mutably(pattern: &syn::Pat) -> bool {
    struct RefMut(bool);
    impl<'ast> Visit<'ast> for RefMut {
        fn visit_pat_ident(&mut self, node: &'ast syn::PatIdent) {
            self.0 |= node.by_ref.is_some() && node.mutability.is_some();
            syn::visit::visit_pat_ident(self, node);
        }
    }
    let mut found = RefMut(false);
    found.visit_pat(pattern);
    found.0
}

/// Whether `expression` is `name` or a field, index or dereference of a place that is.
fn place_roots_at(expression: &syn::Expr, name: &str) -> bool {
    match unwrap_parens(expression) {
        syn::Expr::Path(path) => path.qself.is_none() && path.path.is_ident(name),
        syn::Expr::Field(field) => place_roots_at(&field.base, name),
        syn::Expr::Index(index) => place_roots_at(&index.expr, name),
        syn::Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => {
            place_roots_at(&unary.expr, name)
        }
        _ => false,
    }
}

/// Whether the left side of an assignment changes `name` other than as the plain
/// name at its top (`nested` is true inside a destructuring pattern, where the plain
/// name is assigned as well).
fn assigned_place_roots_at(left: &syn::Expr, name: &str, nested: bool) -> bool {
    match unwrap_parens(left) {
        syn::Expr::Tuple(tuple) => tuple
            .elems
            .iter()
            .any(|element| assigned_place_roots_at(element, name, true)),
        syn::Expr::Array(array) => array
            .elems
            .iter()
            .any(|element| assigned_place_roots_at(element, name, true)),
        syn::Expr::Struct(literal) => literal
            .fields
            .iter()
            .any(|field| assigned_place_roots_at(&field.expr, name, true)),
        syn::Expr::Call(call) => call
            .args
            .iter()
            .any(|argument| assigned_place_roots_at(argument, name, true)),
        other => place_roots_at(other, name) && (nested || !is_assignment_to(other, name)),
    }
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
    targets: &[String],
    assert_spans: &[(usize, usize)],
) -> TargetCallVerification {
    let call_sites = find_call_sites(text, symbol);
    let macro_use = macro_target_use(text, symbol, targets);
    let bindings: Vec<(DerivedBinding, BindingReach)> =
        find_let_bindings_containing_call(text, symbol, targets)
            .into_iter()
            .map(|binding| {
                let reach = binding_reach(text, &binding, targets, assert_spans);
                (binding, reach)
            })
            .collect();
    // Derivation from DS-623 and DS-1962: DA-003 fails only when the result is not
    // verified at all (DS-623), and one result that reaches an assert-equivalent
    // keeps the other calls from making it UNKNOWN (DS-1962). A delegation target
    // (DS-641, DS-1960) is read the same way: a result that reaches an
    // assert-equivalent directly decides DA-003, and only when none does is a path
    // through a delegation target judged by §7.2.1.
    let delegated = macro_use.delegated
        || bindings.iter().any(|(binding, reach)| {
            matches!(reach, BindingReach::Delegated)
                || (binding.delegated
                    && matches!(reach, BindingReach::Reaches | BindingReach::Uncertain))
        });
    if call_sites.is_empty() {
        // Derivation (#282): a call read inside an assert verifies the target only
        // when no call outside a macro exists, so reading macro tokens never
        // clears a call that stays unverified (DS-615: it only moves toward UNKNOWN).
        return if macro_use.verified_path || (macro_use.verified_method && !macro_use.opaque) {
            TargetCallVerification::Verified
        } else if delegated {
            TargetCallVerification::Delegated
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
    let mut binding_into_target = false;
    for (binding, reach) in &bindings {
        match reach {
            BindingReach::Reaches => {
                if binding.has_path_call {
                    path_verified = true;
                } else if binding.has_method_call {
                    method_verified = true;
                }
            }
            BindingReach::Uncertain => binding_uncertain = true,
            BindingReach::IntoTarget => binding_into_target = true,
            BindingReach::NotReached | BindingReach::Delegated => {}
        }
    }
    if path_verified {
        return TargetCallVerification::Verified;
    }
    if binding_uncertain {
        return TargetCallVerification::BindingUncertain;
    }
    if binding_into_target {
        return TargetCallVerification::IntoTargetArgument;
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
    if delegated {
        return TargetCallVerification::Delegated;
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
    if bindings
        .iter()
        .any(|(binding, _)| reaches_debug_assert(text, &binding.name, targets))
    {
        return TargetCallVerification::DebugAssertOnly;
    }
    TargetCallVerification::Unverified
}

/// Whether the name of a binding derived from the target's result is used in a
/// checked argument of a `debug_assert*` where it may flow (DS-1954). A use whose
/// value is dropped there, or one in the message arguments (DS-1935), is not.
fn reaches_debug_assert(text: &str, name: &str, targets: &[String]) -> bool {
    debug_assert_macros(text).iter().any(|node| {
        let Some(arguments) = macro_arguments(node) else {
            return tokens_mention_ident(node.tokens.clone(), name);
        };
        let mut probe = CheckedCallVisitor::new(None, Some(name), targets);
        for expression in &arguments.checked {
            probe.visit_expr(expression);
        }
        probe.probe_flows
            || probe.probe_delegated
            || probe.probe_into_target
            || probe.probe_rebound
            || probe.opaque
    })
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

/// Names bound by `let NAME = ...<call>...;` where the call's value can flow to
/// the initializer's value, each with whether such a call is a function call of
/// `symbol` (not only a method call).
///
/// Derivation (not stated in DS-615 / DS-623 / DS-628 / DS-1680): a binding is
/// derived from the target's result (DS-623) only where the call's value can reach
/// the initializer's value, which is the flow test of `CheckedCallVisitor` applied to
/// the initializer. A call whose value is dropped there (a block statement, an
/// operand that a literal absorbs (DS-1948), a `match` scrutinee, a projection that leaves it out, a closure
/// body) leaves the binding underived, so the call stays unverified, UNKNOWN
/// (DS-615). A closure body is UNKNOWN because DS-629 reads dataflow inside a
/// closure as UNKNOWN. A `match` scrutinee is outside the range DS-628 gives the
/// tracking (a `let` binding, a method chain, a field access), so DS-615 reads it as
/// UNKNOWN. Calls in macro tokens are not read here: `macro_target_use` reads them.
fn find_let_bindings_containing_call(
    text: &str,
    symbol: &str,
    targets: &[String],
) -> Vec<DerivedBinding> {
    let Ok(function) = syn::parse_str::<syn::ItemFn>(text) else {
        return Vec::new();
    };
    let names = NameUse::new(text, symbol, &function);
    let mut visitor = LetBindingVisitor {
        names: &names,
        targets,
        bindings: Vec::new(),
    };
    visitor.visit_item_fn(&function);
    visitor.bindings
}

/// A `let` binding whose initializer carries the target's result, or hands it to
/// a delegation target (DS-1960).
struct DerivedBinding {
    name: String,
    /// The result comes from a function call of the target, not a method call.
    has_path_call: bool,
    /// The result comes from a method call of the target.
    has_method_call: bool,
    /// The initializer hands the result to a delegation target (DS-641: a
    /// delegation target in a binding that reaches an assert-equivalent).
    delegated: bool,
    /// Declared `let mut`.
    mutable: bool,
    /// The initializer is the target's result as a whole, so every position of the
    /// binding is derived (DS-1939).
    whole_result: bool,
}

/// Whether `expression` is a call of the target, or a method chain, `?`, `.await` or
/// parentheses over one: a value with no positions of its own, all derived (DS-1939).
fn is_whole_result(expression: &syn::Expr, symbol: &str) -> bool {
    match unwrap_parens(expression) {
        syn::Expr::Call(call) => matches!(
            call.func.as_ref(),
            syn::Expr::Path(path) if path.path.segments.last().is_some_and(|segment| segment.ident == symbol)
        ),
        syn::Expr::MethodCall(call) => {
            call.method == symbol || is_whole_result(&call.receiver, symbol)
        }
        syn::Expr::Try(node) => is_whole_result(&node.expr, symbol),
        syn::Expr::Await(node) => is_whole_result(&node.base, symbol),
        _ => false,
    }
}

struct LetBindingVisitor<'a> {
    names: &'a NameUse<'a>,
    targets: &'a [String],
    bindings: Vec<DerivedBinding>,
}

impl<'ast> Visit<'ast> for LetBindingVisitor<'_> {
    fn visit_local(&mut self, node: &'ast syn::Local) {
        if let (syn::Pat::Ident(pattern), Some(init)) = (&node.pat, &node.init) {
            let mut calls = CheckedCallVisitor::new(Some(self.names), None, self.targets);
            calls.init_mode = true;
            calls.visit_expr(init.expr.as_ref());
            if calls.path || calls.method || calls.delegated {
                self.bindings.push(DerivedBinding {
                    name: pattern.ident.to_string(),
                    has_path_call: calls.path,
                    has_method_call: calls.method,
                    delegated: calls.delegated,
                    mutable: pattern.mutability.is_some(),
                    whole_result: is_whole_result(&init.expr, self.names.symbol),
                });
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

    /// Where DS-619's methods and `?` appear (DS-1964 aside). Counted only outside
    /// a closure, an `async` block and a nested item, the positions DS-1962 leaves
    /// out because they may not run (derivation from the intent of DS-1962 for
    /// DA-001).
    fn method_or_try(&self) -> MethodOrTry {
        let mut constructed = ConstructedBindings {
            variants: Vec::new(),
        };
        constructed.visit_item_fn(&self.function);
        constructed
            .variants
            .retain(|(name, _)| count_rebindings_in(&self.function, name) == 1);
        let mut visitor = VerificationSyntaxVisitor {
            constructed: &constructed.variants,
            nested: false,
            has_verifier: false,
            counted: false,
        };
        visitor.visit_item_fn(&self.function);
        if visitor.counted {
            MethodOrTry::Counted
        } else if visitor.has_verifier {
            MethodOrTry::Uncounted
        } else {
            MethodOrTry::None
        }
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

/// DS-619's assert-equivalent methods and `?`, except where DS-1964 says they
/// cannot fail: a receiver whose variant is `Some` / `Ok` (for `.unwrap()`,
/// `.expect(..)`, `?`) or `Err` (for `.unwrap_err()`, `.expect_err(..)`) by syntax.
struct VerificationSyntaxVisitor<'a> {
    /// `let` names bound once to a constructor, with the constructor's name.
    constructed: &'a [(String, String)],
    /// Inside a closure, an `async` block or a nested item.
    nested: bool,
    has_verifier: bool,
    /// One is found outside a closure, an `async` block and a nested item.
    counted: bool,
}

impl VerificationSyntaxVisitor<'_> {
    /// Whether the variant of `receiver` is one of `variants` by syntax (DS-1964,
    /// DS-1939): a constructor call, or a name bound once by a `let` to one. A
    /// receiver whose variant is not told this way stays an assert-equivalent
    /// (DS-619), as DS-1964 covers only a variant fixed by syntax.
    fn is_constructed(&self, receiver: &syn::Expr, variants: &[&str]) -> bool {
        match unwrap_parens(receiver) {
            syn::Expr::Call(call) => {
                constructor_name(call).is_some_and(|name| variants.contains(&name.as_str()))
            }
            syn::Expr::Path(path) if path.qself.is_none() => {
                path.path.get_ident().is_some_and(|ident| {
                    self.constructed.iter().any(|(name, variant)| {
                        ident == name && variants.contains(&variant.as_str())
                    })
                })
            }
            _ => false,
        }
    }

    fn found(&mut self, counts: bool) {
        self.has_verifier |= counts;
        self.counted |= counts && !self.nested;
    }

    fn nesting(&mut self, visit: impl FnOnce(&mut Self)) {
        let outer = self.nested;
        self.nested = true;
        visit(self);
        self.nested = outer;
    }
}

impl<'ast> Visit<'ast> for VerificationSyntaxVisitor<'_> {
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        let counts = match node.method.to_string().as_str() {
            "unwrap" | "expect" => !self.is_constructed(&node.receiver, &["Some", "Ok"]),
            "unwrap_err" | "expect_err" => !self.is_constructed(&node.receiver, &["Err"]),
            _ => false,
        };
        self.found(counts);
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_try(&mut self, node: &'ast syn::ExprTry) {
        self.found(!self.is_constructed(&node.expr, &["Some", "Ok"]));
        syn::visit::visit_expr_try(self, node);
    }

    fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
        self.nesting(|visitor| syn::visit::visit_expr_closure(visitor, node));
    }

    fn visit_expr_async(&mut self, node: &'ast syn::ExprAsync) {
        self.nesting(|visitor| syn::visit::visit_expr_async(visitor, node));
    }

    fn visit_item(&mut self, node: &'ast syn::Item) {
        self.nesting(|visitor| syn::visit::visit_item(visitor, node));
    }
}

/// The constructor name (`Some`, `Ok`, `Err`) a call's callee ends with, by name
/// (DS-1961, DS-1967).
fn constructor_name(call: &syn::ExprCall) -> Option<String> {
    let syn::Expr::Path(path) = call.func.as_ref() else {
        return None;
    };
    let name = path.path.segments.last()?.ident.to_string();
    matches!(name.as_str(), "Some" | "Ok" | "Err").then_some(name)
}

/// `let NAME = Some(..) / Ok(..) / Err(..);` bindings (not `mut`, not `ref`), with
/// the constructor's name.
struct ConstructedBindings {
    variants: Vec<(String, String)>,
}

impl<'ast> Visit<'ast> for ConstructedBindings {
    fn visit_local(&mut self, node: &'ast syn::Local) {
        let mut pattern = &node.pat;
        while let syn::Pat::Type(typed) = pattern {
            pattern = &typed.pat;
        }
        if let (syn::Pat::Ident(binding), Some(init)) = (pattern, &node.init) {
            let plain = binding.by_ref.is_none()
                && binding.mutability.is_none()
                && binding.subpat.is_none()
                && init.diverge.is_none();
            if let (true, syn::Expr::Call(call)) = (plain, unwrap_parens(&init.expr)) {
                if let Some(variant) = constructor_name(call) {
                    self.variants.push((binding.ident.to_string(), variant));
                }
            }
        }
        syn::visit::visit_local(self, node);
    }
}

// ---------------------------------------------------------------------------
// DA-006 (DS-626): no verification syntax
// ---------------------------------------------------------------------------

fn da_006_no_verification_syntax(
    has_verification_syntax: bool,
    has_debug_assert: bool,
    has_call: bool,
) -> DaVerdict {
    if has_verification_syntax {
        DaVerdict::NoViolation
    } else if has_debug_assert {
        // DS-1954 (before DS-626 / DS-642 / DS-647): a `debug_assert*` checks in a
        // debug build only, so it is neither an assert-equivalent nor its absence.
        DaVerdict::Unknown(
            "the only checking construct is a `debug_assert*`, which is not an \
             assert-equivalent but checks in a debug build (DS-1954)"
                .to_owned(),
        )
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

/// The `debug_assert*` invocations the function's syntax tree holds (not those
/// inside the tokens of another macro).
fn debug_assert_macros(text: &str) -> Vec<syn::Macro> {
    struct Finder(Vec<syn::Macro>);
    impl<'ast> Visit<'ast> for Finder {
        fn visit_macro(&mut self, node: &'ast syn::Macro) {
            if node
                .path
                .segments
                .last()
                .is_some_and(|segment| is_debug_assert(&segment.ident.to_string()))
            {
                self.0.push(node.clone());
            }
        }
    }
    let Ok(function) = syn::parse_str::<syn::ItemFn>(text) else {
        return Vec::new();
    };
    let mut finder = Finder(Vec::new());
    finder.visit_item_fn(&function);
    finder.0
}

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
            if let Some(span) = macro_argument_span(self.text, node) {
                self.spans.push(span);
            }
        }
        syn::visit::visit_macro(self, node);
    }
}

/// The byte range of a macro invocation's argument-list contents (inside its
/// outermost delimiters).
fn macro_argument_span(text: &str, node: &syn::Macro) -> Option<(usize, usize)> {
    let start = span_offset(node.span().start(), text);
    let end = span_offset(node.span().end(), text);
    let invocation = &text[start..end];
    let open = invocation.find(['(', '[', '{'])?;
    let open_char = invocation.as_bytes()[open] as char;
    let close_char = match open_char {
        '(' => ')',
        '[' => ']',
        _ => '}',
    };
    let close = find_matching_close(invocation, open, open_char, close_char)?;
    Some((start + open + 1, start + close))
}

fn macro_argument_start(text: &str, node: &syn::Macro) -> Option<usize> {
    macro_argument_span(text, node).map(|(start, _)| start)
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

    /// A `let mut` binding changed through a place (a field, an index, a dereference),
    /// through `&mut`, or through a method receiver is not the target's result when the
    /// assert reads it.
    /// @vtest.id TEST-ORACLE-DA-003-OVERWRITE-THROUGH-PLACE-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW, VO-ORACLE-ANALYSIS-LIMIT-UNKNOWN, VO-ORACLE-DA-003-BINDING-MUTATION-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent `let mut` の束縛を、assert が読む位置への代入、参照外しへの代入、`&mut`、メソッドの受け手、クロージャ・マクロの中の代入で変更する Test は、assert が読む値が target の結果と決められず、PASS にならず UNKNOWN になることを確認する
    #[test]
    fn a_let_mut_binding_changed_through_a_place_is_unknown() {
        for body in [
            "let mut r = double(2); r.0 = other(); assert_eq!(r.0, 1);",
            "let mut r = double(2); r[0] = other(); assert_eq!(r[0], 1);",
            "let mut r = double(2); *r = other(); assert_eq!(*r, 1);",
            "let mut r = double(2); (*r).x = other(); assert_eq!(r.x, 1);",
            "let mut r = double(2); *(&mut r) = other(); assert_eq!(r, 1);",
            "let mut r = double(2); let p = &mut r; *p = other(); assert_eq!(r, 1);",
            "let mut r = double(2); touch(&mut r.0); assert_eq!(r.0, 1);",
            "let mut r = double(2); (r.0, x) = (other(), 1); assert_eq!(r.0, 1);",
            "let mut r = double(2); r.clear(); assert_eq!(r, 1);",
            "let mut r = double(2); r.0.push(1); assert_eq!(r.0, 1);",
            "let mut r = double(2); r[0].push(1); assert_eq!(r[0], 1);",
            "let mut r = double(2); let f = || r.0 = other(); f(); assert_eq!(r.0, 1);",
            "let mut r = double(2); assert_eq!(touch(&mut r), 1);",
            "let mut r = double(2); assert!({ r.0 = other(); r.0 == 1 });",
            "let mut r = double(2); set!{ r.0 = other(); } assert_eq!(r.0, 1);",
            "let mut r = double(2); reset!(r); assert_eq!(r, 1);",
            "let mut r = double(2); let p = addr_of_mut!(r); assert_eq!(r, 1);",
            "let mut r = double(2); let p = &raw mut r; assert_eq!(r, 1);",
            "let mut r = double(2); let (ref mut a, _) = r; *a = other(); assert_eq!(r.0, 1);",
            "let mut r = double(2); let ref mut a = r; *a = other(); assert_eq!(r, 1);",
            "let mut r = double(2); match r { (ref mut a, _) => *a = other() } assert_eq!(r.0, 1);",
            "let mut r = double(2); if let Some(ref mut a) = r { *a = other(); } assert_eq!(r, 1);",
            "let mut r = double(2); let b = matches!(r.take(), Some(ref a)); assert_eq!(r, None);",
            "let mut r = double(2); let b = matches!(r, Some(ref mut a) if { *a = 1; true }); assert_eq!(r, Some(1));",
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

    /// A binding that cannot be changed through a place (not `let mut`), or a `let mut`
    /// that is only read, stays as it was.
    /// @vtest.id TEST-ORACLE-DA-003-UNCHANGED-BINDING-STAYS-NO-VIOLATION
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent `let` の束縛、変更のない `let mut` の束縛、assert の中だけで受け手になる `let mut` の束縛が assert に届く Test は、違反なしのままであることを確認する
    #[test]
    fn a_binding_that_is_only_read_stays_no_violation() {
        for body in [
            "let r = double(2); assert_eq!(r.0, 4);",
            "let r = double(2); let n = r.len(); assert_eq!(r, n);",
            "let mut r = double(2); assert_eq!(r, 4);",
            "let mut r = double(2); assert_eq!(r.0, 4);",
            "let mut r = double(2); assert_eq!(r.len(), 4);",
            "let mut r = double(2); let mut v = other(); v.0 = 1; assert_eq!(r, 4);",
            "let mut r = double(2); v[r] = 1; assert_eq!(r, 4);",
            "let mut r = double(2); let c = &r; assert_eq!(r, 4);",
            "let mut r = double(2); let s = format!(\"{}\", r); assert_eq!(r, 4);",
            "let mut r = double(2); assert!(matches!(r, Some(_)));",
            "let mut r = double(2); let b = matches!(r, Some(_)); assert_eq!(r, 4);",
            "let mut r = double(2); match r { (a, _) => drop(a) } assert_eq!(r, 4);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::NoViolation),
                "{body}: {:?}",
                analysis.da_003
            );
        }
    }

    fn da_003_of(body: &str) -> DaVerdict {
        let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
        analyze(&text, &["double".to_owned()], &[]).da_003
    }

    /// DS-1956: after a projection assignment whose path is fixed by syntax, the
    /// positions not assigned keep the target's result; DS-1966: an assignment in a
    /// branch, a loop or the right side of `&&` / `||` leaves the other positions
    /// derived on every path.
    /// @vtest.id TEST-ORACLE-DA-003-PROJECTION-ASSIGNMENT-KEEPS-OTHER-POSITIONS
    /// @vtest.covers VO-ORACLE-DA-003-PROJECTION-ASSIGNMENT, VO-ORACLE-DA-003-CONDITIONAL-ASSIGNMENT-MERGE
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent target の結果を束縛した `let mut` の束縛に、フィールド名・タプルの番号・整数リテラルの添字だけからなる経路で射影代入（条件付き・ループ内・`&&`/`||` の右辺を含む）をしたあと、代入していない位置を assert が読む Test と、複合代入した位置を読む Test は、DA-003 が違反なしになることを確認する
    #[test]
    fn a_projection_assignment_keeps_the_positions_it_does_not_assign() {
        for body in [
            "let mut r = double(2); r.f = 5; assert_eq!(r.g, 4);",
            "let mut r = double(2); r.0 = 5; assert_eq!(r.1, 4);",
            "let mut v = double(2); v[0] = 7; assert_eq!(v[1], 4);",
            "let mut r = double(2); r.f.g = 5; assert_eq!(r.f.h, 4);",
            "let mut r = double(2); r.f.g = 5; assert_eq!(r.h, 4);",
            "let mut r = double(2); r[0][1] = 5; assert_eq!(r[0][0], 4);",
            "let mut r = double(2); r.f = 5; assert_eq!(r.g.len(), 4);",
            "let mut r = double(2); r.f = 5; assert_eq!(Some(&r.g), Some(&4));",
            "let mut r = double(2); r.f = 5; assert_eq!(r.g, 4, \"{}\", r.f);",
            "let mut r = double(2); r.f = 5; assert_eq!(r.g, 4); assert_eq!(r.f, 5);",
            "let mut r = double(2).clone(); r.f = 5; assert_eq!(r.g, 4);",
            "let mut r = double(2); r.0 += 1; assert_eq!(r.0, 5);",
            "let mut r = double(2); r.f += x(); assert_eq!(r.f, 5);",
            "let mut r = double(2); if c() { r.f = 5; } assert_eq!(r.g, 4);",
            "let mut r = double(2); match k() { 1 => r.f = 5, _ => {} } assert_eq!(r.g, 4);",
            "let mut r = double(2); for x in xs() { r.f = x; } assert_eq!(r.g, 4);",
            "let mut r = double(2); while c() { r.f = 5; } assert_eq!(r.g, 4);",
            "let mut r = double(2); let ok = c() || { r.f = 5; true }; assert_eq!(r.g, 4);",
        ] {
            assert_eq!(da_003_of(body), DaVerdict::NoViolation, "{body}");
        }
    }

    /// DS-1956: an assigned position holds the assigned value, not the target's
    /// result; DS-1966: one assigned on some paths only is neither. Reading it in an
    /// assert is not PASS, also through a reference, a block, a conversion or a
    /// nested macro.
    /// @vtest.id TEST-ORACLE-DA-003-ASSIGNED-POSITION-IS-NOT-PASS
    /// @vtest.covers VO-ORACLE-DA-003-PROJECTION-ASSIGNMENT, VO-ORACLE-DA-003-CONDITIONAL-ASSIGNMENT-MERGE
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent 射影代入した位置（条件付き・ループ内・`&&` の右辺の代入を含む）を assert が読む Test、平の代入のあとに複合代入した位置を読む Test、assert の中で参照・ブロック・変換・入れ子のマクロを通して読む Test は、DA-003 が違反なしにならず UNKNOWN になることを確認する
    #[test]
    fn a_read_of_an_assigned_position_is_not_pass() {
        for body in [
            "let mut r = double(2); r.f = 5; assert_eq!(r.f, 5);",
            "let mut v = double(2); v[0] = 7; assert_eq!(v[0], 7);",
            "let mut r = double(2); r.f.g = 5; assert_eq!(r.f.g, 5);",
            "let mut r = double(2); r.f = 5; assert_eq!(r.f.0, 5);",
            "let mut r = double(2); r.f = 5; r.f += 1; assert_eq!(r.f, 6);",
            "let mut r = double(2); r.f = 5; r.f.g += 1; assert_eq!(r.f.g, 6);",
            "let mut r = double(2); if c() { r.f = 5; } assert_eq!(r.f, 5);",
            "let mut r = double(2); if c() { r.f = 5; } else { r.f = 6; } assert_eq!(r.f, 5);",
            "let mut r = double(2); match k() { 1 => r.f = 5, _ => {} } assert_eq!(r.f, 5);",
            "let mut r = double(2); let ok = c() && { r.f = 5; true }; assert_eq!(r.f, 5);",
            "let mut r = double(2); for x in xs() { r.f = x; } assert_eq!(r.f, 5);",
            "let mut r = double(2); while c() { r.f = 5; } assert_eq!(r.f, 5);",
            "let mut r = double(2); loop { r.f = 5; break; } assert_eq!(r.f, 5);",
            "let mut r = double(2); r.f = 5; assert_eq!((&r).f, 5);",
            "let mut r = double(2); r.f = 5; assert_eq!({ r }.f, 5);",
            "let mut r = double(2); r.f = 5; assert_eq!({ let a = r; a.f }, 5);",
            "let mut r = double(2); r.f = 5; assert_eq!((r, 1).0.f, 5);",
            "let mut r = double(2); r.f = 5; assert_eq!(Some(r.f), Some(5));",
            "let mut r = double(2); r.f = 5; assert!(matches!(r.f, 5));",
            "let mut r = double(2); r.f = 5; assert_eq!(vec![r.f][0], 5);",
        ] {
            assert!(
                matches!(da_003_of(body), DaVerdict::Unknown(_)),
                "{body}: {:?}",
                da_003_of(body)
            );
        }
    }

    /// DS-1956: a projection whose path is not fixed by syntax makes the whole
    /// binding UNKNOWN; DS-1959: so do `&mut x.p`, a closure or `async` block that
    /// assigns it, a method receiver and `ref mut`, and an assignment in macro
    /// tokens or by dereference is not read as a projection assignment.
    /// @vtest.id TEST-ORACLE-DA-003-UNFIXED-PATH-OR-MUTATION-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-PROJECTION-ASSIGNMENT, VO-ORACLE-DA-003-BINDING-MUTATION-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::place_changes
    /// @vtest.intent 経路を構文から確定できない射影代入（変数・文字列・式の添字、参照外し、分割代入）と、`&mut x.p`・可変でキャプチャするクロージャと async ブロック・メソッドの受け手・`ref mut`・マクロの中の代入を経た束縛は、代入していない位置を読む Test でも DA-003 が UNKNOWN になることを確認する
    #[test]
    fn an_unfixed_path_or_a_mutation_keeps_the_whole_binding_unknown() {
        for body in [
            "let mut v = double(2); let i = 0; v[i] = 7; assert_eq!(v[1], 4);",
            "let mut r = double(2); r[\"k\"] = 5; assert_eq!(r.g, 4);",
            "let mut r = double(2); r[k()] = 5; assert_eq!(r.g, 4);",
            "let mut r = double(2); r.f[i] = 5; assert_eq!(r.g, 4);",
            "let mut r = double(2); (*r).f = 5; assert_eq!(r.g, 4);",
            "let mut r = double(2); *r.f = 5; assert_eq!(r.g, 4);",
            "let mut r = double(2); (r.f, x) = (5, 1); assert_eq!(r.g, 4);",
            "let mut r = double(2); touch(&mut r.f); assert_eq!(r.g, 4);",
            "let mut r = double(2); let p = &mut r.f; *p = 5; assert_eq!(r.g, 4);",
            "let mut r = double(2); let f = || r.f = 5; f(); assert_eq!(r.g, 4);",
            "let mut r = double(2); let f = async { r.f = 5; }; assert_eq!(r.g, 4);",
            "let mut r = double(2); set!{ r.f = 5 }; assert_eq!(r.g, 4);",
            "let mut r = double(2); r.f.push(1); assert_eq!(r.g, 4);",
            "let mut r = double(2); r.f = 5; let (ref mut a, _) = r; assert_eq!(r.g, 4);",
            "let mut r = double(2); r = other(); r.f = 5; assert_eq!(r.g, 4);",
        ] {
            assert!(
                matches!(da_003_of(body), DaVerdict::Unknown(_)),
                "{body}: {:?}",
                da_003_of(body)
            );
        }
    }

    /// DS-1939: a value whose positions differ is UNKNOWN as a method receiver
    /// (also an operator, a formatting trait or a pattern over it, whose positions
    /// are not fixed by syntax) and through an index that is not an integer literal;
    /// compared whole it reaches the assert (the false PASS DS-1939 and DS-1956
    /// disclose). A binding of a literal that holds the result in one element is not
    /// followed through an assignment, so a read of an element that is not the
    /// result does not pass.
    /// @vtest.id TEST-ORACLE-DA-003-MIXED-POSITIONS-ARE-READ-BY-POSITION
    /// @vtest.covers VO-ORACLE-DA-003-PER-POSITION-DERIVATION
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::read_is_derived
    /// @vtest.intent 一部の位置を射影代入した値を、メソッドの受け手・演算子の被演算子・`matches!` と `format!` の引数にする Test、整数リテラルでない添字で読む Test は UNKNOWN になり、`assert_eq!` や `==` で値全体を比べる Test は開示された偽 PASS のとおり違反なしになることを確認する。target の値を一つの要素に持つ構築式の束縛で、target の値の要素に射影代入したあと別の要素を読む Test が違反なしにならないことも確認する
    #[test]
    fn a_value_with_an_assigned_position_is_read_by_position() {
        for body in [
            "let mut v = double(2); v[0] = 7; assert_eq!(v.len(), 1);",
            "let mut r = double(2); r.f = 5; assert!(r.is_ok());",
            "let mut r = double(2); r.f = 5; assert!((&r).is_ok());",
            "let mut r = double(2); r.f = 5; assert!((*r).is_ok());",
            "let mut r = double(2); r.f.g = 5; assert_eq!(r.f.len(), 2);",
            "let mut r = double(2); r.f = 5; assert!(matches!(r, R { f: 5, .. }));",
            "let mut r = double(2); r.f = 5; assert_eq!(format!(\"{}\", r), \"5\");",
            "let mut r = double(2); r.f = 5; assert!(!r);",
            "let mut r = double(2); r.f = 5; assert_eq!(r + 1, x());",
            "let mut r = double(2); r.f = 5; assert_eq!(Some(r), Some(x()));",
            "let mut v = double(2); v[0] = 7; let i = 1; assert_eq!(v[i], 4);",
            "let mut v = double(2); v[0] = 7; assert_eq!(v[..1], [7]);",
            "let mut t = (double(2), 1); t.0 = 5; assert_eq!(t.1, 1);",
            "let mut t = S { a: double(2), b: 1 }; t.a = 5; assert_eq!(t.b, 1);",
        ] {
            assert!(
                matches!(da_003_of(body), DaVerdict::Unknown(_)),
                "{body}: {:?}",
                da_003_of(body)
            );
        }
        for body in [
            "let mut r = double(2); r.f = 5; assert_eq!(r, R { f: 5, g: 4 });",
            "let mut r = double(2); r.f = 5; r.g = 6; assert_eq!(r, R { f: 5, g: 6 });",
            "let mut r = double(2); if c() { r.f = 5; } assert_eq!(r, R { f: 5, g: 4 });",
            "let mut r = double(2); r.f = 5; assert!(r == R { f: 5, g: 4 });",
            "let mut r = double(2); r.f = 5; assert_ne!(&r, &R { f: 5, g: 3 });",
            "let mut r = double(2); r.f = 5; assert!(r.g.is_ok());",
        ] {
            assert_eq!(da_003_of(body), DaVerdict::NoViolation, "{body}");
        }
    }

    /// Narrowings below DS-1956: the assigned value is not followed, statement
    /// order is not read, an integer-literal index is not told apart from a field
    /// or a tuple number at the same step, and an assignment inside the assert is
    /// not read as a projection assignment. Each keeps the verdict UNKNOWN where
    /// DS-1956 alone might derive the read.
    /// @vtest.id TEST-ORACLE-DA-003-PROJECTION-ASSIGNMENT-NARROWINGS-ARE-UNKNOWN
    /// @vtest.covers VO-ORACLE-ANALYSIS-LIMIT-UNKNOWN, VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::read_is_derived
    /// @vtest.intent 代入した値を追わない・文の順序を読まない・同じ段の整数リテラル添字とフィールドを区別しない・assert の中の代入を射影代入として読まない、という解析の限界に当たる Test が、違反なしにならず UNKNOWN になることを確認する
    #[test]
    fn projection_assignment_narrowings_are_unknown() {
        for body in [
            "let mut r = double(2); r.f = double(3); assert_eq!(r.f, 6);",
            "let mut r = double(2); assert_eq!(r.f, 4); r.f = 5;",
            "let mut r = double(2); r.0 = 5; assert_eq!(r[0], 5);",
            "let mut r = double(2); r[0] = 5; assert_eq!(r.0, 5);",
            "let mut r = double(2); r[0] = 5; assert_eq!(r.g, 4);",
            "let mut r = double(2); assert!({ r.f = 5; r.g == 4 });",
        ] {
            assert!(
                matches!(da_003_of(body), DaVerdict::Unknown(_)),
                "{body}: {:?}",
                da_003_of(body)
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

    /// A name inside macro tokens that is not a use of the target is not read as
    /// one: a path prefix, a field, a method of another name, a macro of the same
    /// name and a field label do not make DA-003 UNKNOWN; the turbofish call does.
    /// @vtest.id TEST-ORACLE-DA-003-NON-CALL-NAME-IN-MACRO-IS-NOT-A-CALL
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::NameUse::tokens_use_name
    /// @vtest.intent マクロのトークンの中の target の名前が、パスの前置(double::VALUE)、フィールド、メソッド名、同名のマクロ、構造体のフィールドのラベルであるときは使用と読まず、turbofish 付きの呼出(double::<u8>(2))は使用と読むことを確認する
    #[test]
    fn a_name_in_macro_tokens_that_is_not_a_use_is_not_read_as_one() {
        for text in [
            "#[test]\nfn t() {\n    println!(\"{}\", double::VALUE);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    println!(\"{}\", s.double);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    println!(\"{:?}\", Cfg { double: 1 });\n    assert_eq!(other(), 1);\n}\n",
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
            "#[test]\nfn t() {\n    let fs = vec![double];\n    fs[0](2);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    println!(\"{}\", apply(double, 2));\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    run_with!(double);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    assert_eq!(apply(double), 4);\n}\n",
            "#[test]\nfn t() {\n    assert_eq!(vec![double][0](2), 4);\n}\n",
            "#[test]\nfn t() {\n    assert_eq!(other(), 1, \"{}\", apply(double));\n}\n",
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

    /// The local exception covers only a `let` and the uses after it in its own
    /// block: a use outside that block or before the `let`, and a binding in a
    /// closure, a match arm, a nested fn or an assignment, are no exception.
    /// @vtest.id TEST-ORACLE-DA-003-LOCAL-EXCEPTION-IS-SCOPED
    /// @vtest.covers VO-ORACLE-ANALYSIS-LIMIT-UNKNOWN, VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::NameUse::new
    /// @vtest.intent 同名のローカルの例外は、let とその同じブロックの中の後ろの使用だけに効き、ブロックの外の使用、let より前の使用、クロージャの引数、match のアーム、ネストした fn の引数、代入には効かず、DA-003 が UNKNOWN になることを確認する
    #[test]
    fn the_local_name_exception_is_scoped_to_its_let() {
        let in_block = "#[test]\nfn t() {\n    for i in 0..2 {\n        let double = make(i);\n        assert_eq!(double[\"a\"], 1);\n    }\n    assert_eq!(other(), 1);\n}\n";
        let analysis = analyze(in_block, &["double".to_owned()], &[]);
        assert!(
            matches!(analysis.da_003, DaVerdict::NoViolation),
            "{:?}",
            analysis.da_003
        );
        for text in [
            "#[test]\nfn t() {\n    { let double = 1; }\n    apply(double);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let f = double;\n    let double = 5;\n    f(2);\n    assert_eq!(other(), double);\n}\n",
            "#[test]\nfn t() {\n    let g = |double: i32| double;\n    apply(double);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    match x { Some(double) => {}, _ => {} }\n    apply(double);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    fn h(double: i32) -> i32 { double }\n    apply(double);\n    assert_eq!(h(1), 1);\n}\n",
            "#[test]\nfn t() {\n    double = 3;\n    apply(double);\n    assert_eq!(other(), 1);\n}\n",
            "#[test]\nfn t() {\n    let double = 1;\n    apply(self::double);\n    assert_eq!(other(), double);\n}\n",
            "#[test]\nfn t() {\n    let double = 1;\n    println!(\"{}\", apply(double));\n    assert_eq!(other(), 1);\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            let expected_unknown = !text.contains("println!");
            assert_eq!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                expected_unknown,
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A constant target is not callable and a local that is named like the
    /// target is not the target: neither is an indirect call. A path to the target
    /// through a type still is.
    /// @vtest.id TEST-ORACLE-DA-003-CONSTANT-OR-LOCAL-NAME-IS-NOT-AN-INDIRECT-CALL
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED, VO-ORACLE-ANALYSIS-LIMIT-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::is_constant_name
    /// @vtest.intent 大文字の名前の target(定数)の値の使用と、同じブロックの前の let で束縛された同名のローカル変数の使用は、間接呼出と読まず DA-003 が違反なしのままで、型を経由する同名の経路は値の使用として UNKNOWN になることを確認する
    #[test]
    fn a_constant_or_local_name_is_not_an_indirect_call() {
        let constant = "#[test]\nfn t() {\n    let form = parse(RUST_FORM);\n    assert_eq!(form.kind, 1);\n}\n";
        let analysis = analyze(constant, &["RUST_FORM".to_owned()], &[]);
        assert!(
            matches!(analysis.da_003, DaVerdict::NoViolation),
            "{:?}",
            analysis.da_003
        );
        let local = "#[test]\nfn t() {\n    let bundle = &envelope;\n    let n = count(bundle);\n    assert_eq!(n, 1);\n}\n";
        let analysis = analyze(local, &["bundle".to_owned()], &[]);
        assert!(
            matches!(analysis.da_003, DaVerdict::NoViolation),
            "{:?}",
            analysis.da_003
        );
        let through_type = "#[test]\nfn t() {\n    let bundle = 1;\n    let f = Calc::bundle;\n    assert_eq!(other(), bundle);\n}\n";
        let analysis = analyze(through_type, &["bundle".to_owned()], &[]);
        assert!(
            matches!(analysis.da_003, DaVerdict::Unknown(_)),
            "{:?}",
            analysis.da_003
        );
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
            "#[test]\nfn t() {\n    let x = s.double(2);\n    assert_eq!(x, 4);\n}\n",
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

    /// A call in a position of an assert's checked argument where its value is
    /// dropped (a statement of a block, a `let _`, a closure body, a loop, a branch
    /// condition) does not verify the target.
    /// @vtest.id TEST-ORACLE-DA-003-DISCARDED-CALL-IN-ASSERT-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-ANALYSIS-LIMIT-UNKNOWN, VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::CheckedCallVisitor
    /// @vtest.intent assert の検査される引数の中で、ブロックの文、let _、使われない let の束縛、クロージャの本体、ループ、if と match の条件と被検査式、代入の右辺にある target の呼出は、結果が比較される値へ流れないので検証済みと数えず、DA-003 が UNKNOWN で合成結果が PASS にならないことを確認する
    #[test]
    fn a_call_whose_value_is_dropped_inside_an_assert_argument_is_unknown() {
        for text in [
            "assert!({ double(2); true });",
            "assert_eq!({ let _ = double(2); 1 }, 1);",
            "assert_ne!({ let _ = double(2); 1 }, 2);",
            "assert_eq!(1, { double(2); 1 });",
            "assert!((|| { double(2); true })());",
            "assert!((|| double(2) == 4)());",
            "assert_eq!(xs.iter().map(|x| double(*x)).sum::<i32>(), 4);",
            "assert!({ for i in 0..2 { double(i); } true });",
            "assert!({ while c { double(2); } true });",
            "assert!({ loop { double(2); break; } true });",
            "assert!({ let x = double(2); true });",
            "assert!({ let x = double(2); let _y = 1; true });",
            "assert_eq!({ let (a, b) = (double(2), 1); b }, 1);",
            "assert!({ let r = double(2); true && { r; false } });",
            "assert!({ let r = double(2); (|| r)() == 4 });",
            "assert!({ let r = double(2); let r = 4; r == 4 });",
            "assert!({ let mut r = 0; r = double(2); r == 4 });",
            "assert!({ let mut r = 0; r += double(2); r == 4 });",
            "assert!(if double(2) > 0 { true } else { false });",
            "assert!(match double(2) { 4 => true, _ => false });",
            "assert!(match c { _ => { double(2); true } });",
            "assert!({ match c { _ => { double(2); } } true });",
            "assert_eq!(unsafe { double(2); 1 }, 1);",
            "assert!({ format!(\"{}\", double(2)); true });",
            "assert!({ let _f = async { double(2) }; true });",
            "assert!({ let r = double(2); let s = r + 1; s == 5 });",
            "assert!({ s.double(2); true });",
            "assert!({ let _ = s.double(2); true });",
            "assert!({ let _ = Calc::double(2); true });",
            "assert!({ let _ = double::<u8>(2); true });",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {text}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(
                !is_fail && is_unknown,
                "{text}: not UNKNOWN: {basis:?} / {:?}",
                analysis.da_003
            );
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A call whose value can reach the compared value still verifies the target.
    /// @vtest.id TEST-ORACLE-DA-003-CALL-REACHING-THE-ASSERT-STAYS-NO-VIOLATION
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::CheckedCallVisitor
    /// @vtest.intent assert の検査される引数の式そのもの、二項演算、メソッドチェーン、フィールド、参照、キャスト、? の被演算子、ブロックの末尾の式、末尾で使われる let の束縛、if と match の各分岐の末尾の式にある target の呼出は、検証済みと数えられ、DA-003 が違反なしのままであることを確認する
    #[test]
    fn a_call_whose_value_reaches_the_compared_value_inside_an_assert_stays_no_violation() {
        for text in [
            "assert_eq!(double(2), 4);",
            "assert!(double(2) > 0);",
            "assert_eq!(1, double(2));",
            "assert_eq!(double(2) + 1, 5);",
            "assert_eq!(-double(2), -4);",
            "assert!(!(double(2) == 5));",
            "assert_eq!(double(2).abs(), 4);",
            "assert_eq!(double(2).0, 4);",
            "assert_eq!(&double(2), &4);",
            "assert_eq!(double(2) as u8, 4);",
            "assert_eq!(double(2)?, 4);",
            "assert_eq!(Calc::double(2), 4);",
            "assert_eq!(double::<u8>(2), 4);",
            "assert_eq!(s.double(2), 4);",
            "assert_eq!((double(2), 1), (4, 1));",
            "assert_eq!(vec![double(2)], vec![4]);",
            "assert_eq!(format!(\"{}\", double(2)), \"4\");",
            "assert!(matches!(double(2), 4));",
            "assert!({ double(2) == 4 });",
            "assert!(unsafe { double(2) } == 4);",
            "assert_eq!({ let _unused = 1; double(2) }, 4);",
            "assert_eq!({ let r = double(2); r }, 4);",
            "assert_eq!({ let r: i32 = double(2); r }, 4);",
            "assert_eq!({ let r = double(2); r + 1 }, 5);",
            "assert_eq!({ let r = double(2); r.value }, 4);",
            "assert_eq!({ let r = double(2); &r }, &4);",
            "assert!(if c { double(2) == 4 } else { false });",
            "assert_eq!(if c { double(2) } else { 0 }, 4);",
            "assert!(match c { true => double(2) == 4, false => false });",
            "assert_eq!(match c { 0 => 1, _ => double(2) }, 4);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {text}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::NoViolation),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// Only the listed positions carry a call's value to the compared value: a
    /// position that is not listed (a literal's element that a
    /// projection leaves out or that an unresolved index may leave out, a `match` scrutinee, a `let` that is not
    /// used by the tail) does not.
    /// @vtest.id TEST-ORACLE-DA-003-UNLISTED-POSITION-IN-ASSERT-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-ANALYSIS-LIMIT-UNKNOWN, VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::CheckedCallVisitor
    /// @vtest.intent assert の検査される引数の中で、流れる位置として列挙されていない位置(射影で残らない集約リテラルの要素、match の被検査式、ガード、if let、let-else、範囲、ラベル付きブロックの break、可変の束縛、末尾で使われない束縛、matches! のパターン側)にある target の呼出は検証済みと数えず、DA-003 が UNKNOWN になることを確認する
    #[test]
    fn a_call_in_a_position_that_is_not_listed_as_flowing_is_unknown() {
        for text in [
            "assert_eq!((double(2), 1).1, 1);",
            "assert_eq!((1, double(2)).0, 1);",
            "assert_eq!(Pair { a: double(2), b: 1 }.b, 1);",
            "assert_eq!([double(2), 1][1], 1);",
            "assert_eq!([double(2), 1][i], 1);",
            "assert_eq!({ let r = double(2); (r, 1).1 }, 1);",
            "assert!(match double(2) { _ => true });",
            "assert!(match c { _ if double(2) == 4 => true, _ => false });",
            "assert!(if let Some(x) = double(2) { x == 4 } else { false });",
            "assert!({ let Some(x) = double(2) else { return; }; x == 4 });",
            "assert_eq!('a: { break 'a double(2); }, 4);",
            "assert_eq!({ { double(2) }; 1 }, 1);",
            "assert_eq!({ let mut r = double(2); r.clear(); r }, 0);",
            "assert_eq!({ let r = double(2); { let r = 1; r } }, 1);",
            "assert_eq!({ let r = double(2); format!(\"{r}\") }, \"4\");",
            "assert!(matches!(x, Some(_) if double(2) == 4));",
            "assert!(xs.iter().any(|x| double(*x) == 4));",
            "assert_eq!(v, 0..double(2));",
            "assert!({ println!(\"{}\", double(2)); true });",
            "assert!(other(), \"{}\", double(2));",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {text}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(
                !is_fail && is_unknown,
                "{text}: not UNKNOWN: {basis:?} / {:?}",
                analysis.da_003
            );
        }
    }

    /// The projected element of a literal and the listed positions of a `let` name
    /// still verify the target.
    /// @vtest.id TEST-ORACLE-DA-003-LISTED-POSITION-IN-ASSERT-STAYS-NO-VIOLATION
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::CheckedCallVisitor
    /// @vtest.intent assert の検査される引数の中で、射影された集約リテラルの要素、射影されない集約リテラル、末尾の式で format! の引数や参照として使われる束縛、定数ブロックの演算子の被演算子にある target の呼出は検証済みと数えられ、DA-003 が違反なしのままであることを確認する
    #[test]
    fn a_call_in_a_listed_position_in_an_assert_stays_no_violation() {
        for text in [
            "assert_eq!((double(2), 1).0, 4);",
            "assert_eq!(Pair { a: double(2), b: 1 }.a, 4);",
            "assert_eq!([double(2), 1][0], 4);",
            "assert_eq!(xs[double(2)], 4);",
            "assert_eq!(&function[rust.double(2)], \"x\");",
            "assert_eq!((double(2), 1), (4, 1));",
            "assert_eq!(double(2)[0], 4);",
            "assert_eq!(double(2) && ok, true);",
            "assert_eq!({ let r = double(2); format!(\"{}\", r) }, \"4\");",
            "assert_eq!({ { double(2) } }, 4);",
            "assert_eq!(const { 1 } + double(2), 5);",
            "assert!(if double(2) > 0 { double(2) == 4 } else { false });",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {text}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::NoViolation),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// DS-1948: an operand of `||` / `|` / `&&` / `&` that a literal absorbs is not a
    /// position the value flows from; beside any other operand it flows.
    /// @vtest.id TEST-ORACLE-DA-003-LITERAL-ABSORBED-OPERAND-DOES-NOT-FLOW
    /// @vtest.covers VO-ORACLE-DA-003-LITERAL-ABSORBED-OPERAND-NOT-FLOWING
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::CheckedCallVisitor
    /// @vtest.intent X || true、true || X、X && false、false && X と | / & の同じ形、その否定、括弧や入れ子でリテラルに値が確定する兄弟の横にある target の呼出と束縛は DA-003 で検証済みと数えず UNKNOWN になり、X || false、X || ok、ok && X、(X || true) && Y の Y、整数の X | 0 のように吸収されない位置では違反なしのままであることを確認する
    #[test]
    fn an_operand_that_a_literal_absorbs_does_not_flow_and_any_other_operand_does() {
        for body in [
            "assert!(double(2) == 4 || true);",
            "assert!(true || double(2) == 4);",
            "assert!(double(2) == 4 && false);",
            "assert!(false && double(2) == 4);",
            "assert!((double(2) == 4) | true);",
            "assert!(true | (double(2) == 4));",
            "assert!((double(2) == 4) & false);",
            "assert!(false & (double(2) == 4));",
            "assert!(!(double(2) == 4 && false));",
            "assert!(double(2) == 4 || (true));",
            "assert!(double(2) == 4 || !false);",
            "assert!(double(2) == 4 || (true || ok));",
            "assert!((double(2) == 4 || true) && ok);",
            "assert!(ok && (double(2) == 4 || true));",
            "assert!((double(2) == 4 && false) || ok);",
            "assert_eq!(false && double(2) == 4, false);",
            "assert!(s.double(2) || true);",
            "let r = double(2);\n    assert!(r == 4 && false);",
            "let r = double(2);\n    assert!(r == 4 || true);",
            "let r = double(2) == 4 && false;\n    assert!(r);",
            "let r = (double(2) == 4) | true;\n    assert!(r);",
            "assert!({ let r = double(2); r == 4 || true });",
            "assert!(double(2) == 4 || 1 == 1);",
            "assert!(double(2) == 4 || (1 < 2));",
            "assert!(double(2) == 4 || \"a\" == \"a\");",
            "assert!(double(2) == 4 || true == true);",
            "assert!(double(2) == 4 || (true ^ false));",
            "assert!(double(2) == 4 && 1 > 2);",
            "assert!(double(2) == 4 || 1 + 1 == 2);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{body}: {:?}",
                analysis.da_003
            );
        }
        for body in [
            "assert!(double(2) == 4 || false);",
            "assert!(false || double(2) == 4);",
            "assert!(double(2) == 4 && true);",
            "assert!((double(2) == 4) | false);",
            "assert!((double(2) == 4) & true);",
            "assert!(double(2) == 4 || ok);",
            "assert!(ok || double(2) == 4);",
            "assert!(ok && double(2) == 4);",
            "assert!((double(2) == 4) & ok);",
            "assert!(double(2) == 4 || (false && ok));",
            "assert!((ok || true) && double(2) == 4);",
            "assert!((double(3) == 6 || true) && double(2) == 4);",
            "assert_eq!(double(2) | 0, 4);",
            "assert_eq!(double(2) & 0, 0);",
            "assert_eq!(0 | double(2), 4);",
            "assert!(s.double(2) || false);",
            "let r = double(2);\n    assert!(r == 4 || ok);",
            "let r = double(2) == 4 || false;\n    assert!(r);",
            "assert!({ let r = double(2); r == 4 || false });",
            "assert!(double(2) == 4 || 1 == 2);",
            "assert!(double(2) == 4 && 1 < 2);",
            "assert!(double(2) == 4 || (true ^ true));",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(!is_fail && !is_unknown, "{body}: {basis:?}");
        }
    }

    /// DS-1949: a logical expression whose value literals fix is a constant
    /// expression for DA-001; one that an operand can change is not.
    /// @vtest.id TEST-ORACLE-DA-001-LITERAL-FIXED-LOGIC-IS-CONSTANT
    /// @vtest.covers VO-ORACLE-DA-001-LITERAL-FIXED-LOGIC-IS-CONSTANT
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::classify_constancy
    /// @vtest.intent X || true、true || X、X && false、false && X と | / & の同じ形、その否定、括弧や入れ子でリテラルに値が確定する論理式だけを引数に持つ assert 相当は DA-001 の定数アサーションになって合成結果が FAIL になり、X || false、X || ok、(X || true) && ok、(X || true) == true、整数の X | 0 や X & 0、メッセージ引数に識別子がある assert は定数アサーションにならず、matches! 以外のマクロを含む引数は定数アサーションにならずに FAIL を出さず、X || m!() は DA-001 と DA-003 が UNKNOWN になることを確認する
    #[test]
    fn a_logical_expression_that_literals_fix_is_a_constant_assertion() {
        for body in [
            "assert!(double(2) == 4 || true);",
            "assert!(true || double(2) == 4);",
            "assert!(double(2) == 4 && false);",
            "assert!(false && double(2) == 4);",
            "assert!((double(2) == 4) | true);",
            "assert!(true | (double(2) == 4));",
            "assert!((double(2) == 4) & false);",
            "assert!(false & (double(2) == 4));",
            "assert!(!(double(2) == 4 || true));",
            "assert!(!(false & (double(2) == 4)));",
            "assert!(!!(double(2) == 4 || true));",
            "assert!(double(2) == 4 || ((true)));",
            "assert!(double(2) == 4 || !false);",
            "assert!((double(2) == 4 && false) || true);",
            "assert!(ok || double(2) == 4 || true);",
            "assert!((double(2) == 4 || true) && (double(3) == 6 || true));",
            "assert_eq!(double(2) == 4 || true, true);",
            "assert_ne!(double(2) == 4 && false, true);",
            "assert!(double(2) == 4 || true, \"message\");",
            "assert!(double(2).is_some() || true);",
            "assert!(true && true);",
            "assert!(!false);",
            "double(2);\n    assert!(true == false || true);",
            "assert!(double(2) == 4 || true);\n    assert!(ok || true);",
            "assert!(double(2) == 4 || 1 == 1);",
            "assert!(double(2) == 4 || (1 < 2));",
            "assert!(double(2) == 4 || \"a\" == \"a\");",
            "assert!(double(2) == 4 || true == true);",
            "assert!(double(2) == 4 || (true ^ false));",
            "assert!(double(2) == 4 && 'a' > 'b');",
            "assert!((double(2) == 4 || true) == true);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_001, DaVerdict::Fail(_)),
                "{body}: {:?}",
                analysis.da_001
            );
            assert!(compose(&analysis).0, "{body}");
        }
        for body in [
            "assert!(double(2) == 4 || false);",
            "assert!(double(2) == 4 || ok);",
            "assert!((double(2) == 4 || true) && ok);",
            "assert!(double(2) == 4 || always_true());",
            "assert_eq!(double(2) | 0, 4);",
            "assert_eq!(double(2) & 0, 0);",
            "assert_eq!(double(2) == 4 || true, ok);",
            "assert!(double(2) == 4 || true, \"{}\", x);",
            "assert!(double(2) == 4 || true);\n    assert_eq!(double(2), 4);",
            "assert!(double(2) == 4 || 1 == 2);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_001, DaVerdict::NoViolation),
                "{body}: {:?}",
                analysis.da_001
            );
        }
        // An operand made only of literals whose value is not evaluated may absorb
        // the rest: DA-001's escape (DS-621).
        // Float literals are not compared, as `f32` and `f64` give different answers.
        for body in [
            "assert!(double(2) == 4 || 1 + 1 == 2);",
            "assert!(double(2) == 4 || 0.1f32 == 0.100000001f32);",
            "assert!(double(2) == 4 && 0.1f32 != 0.100000001f32);",
            "assert!(double(2) == 4 || 0.5 < 1.5);",
            "assert!(double(2) == 4 || if true { true } else { false });",
            "assert!(double(2) == 4 || { true });",
            "assert!(double(2) == 4 || matches!(1, 1));",
            "assert!(double(2) == 4 && (1 as u8 == 2));",
            "assert!(double(2) == 4 || m!());",
            "assert!(m!() || double(2) == 4);",
            "assert!(double(2) == 4 || !m!());",
            "assert!(double(2) == 4 && env!(\"X\") == \"1\");",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_001, DaVerdict::Unknown(_))
                    && matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{body}: {:?} / {:?}",
                analysis.da_001,
                analysis.da_003
            );
        }
        // A macro other than `matches!` may expand to any code (DS-629), so an
        // argument holding one is not a constant expression.
        for body in [
            "macro_rules! double_m { ($x:expr) => { double($x) } }\n    assert_eq!(double_m!(2), 4);",
            "assert_eq!(compute!(2, 3), 5);",
            "assert!(check_double!());",
            "assert!(four_m!() == 4);",
            "assert_eq!(env!(\"X\"), \"1\");",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                !matches!(analysis.da_001, DaVerdict::Fail(_)),
                "{body}: {:?}",
                analysis.da_001
            );
            assert!(!compose(&analysis).0, "{body}");
        }
    }

    /// A `let` outside the assert reaches it only where the name flows to the
    /// compared value.
    /// @vtest.id TEST-ORACLE-DA-003-BINDING-USE-DROPPED-IN-ASSERT-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-ANALYSIS-LIMIT-UNKNOWN, VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::binding_reach
    /// @vtest.intent let で束縛された target の結果の名前が、assert の引数の中でブロックの文、let _ =、クロージャの本体、match の被検査式にだけ現れるとき、比較される値へ届いたとは数えず DA-003 が UNKNOWN になり、比較される値として使われる同じ束縛は違反なしのままであることを確認する
    #[test]
    fn a_binding_used_where_its_value_is_dropped_in_an_assert_is_unknown() {
        for text in [
            "let r = double(2);\n    assert!({ let _ = r; true });",
            "let r = double(2);\n    assert!({ r; true });",
            "let r = double(2);\n    assert!((|| { r; true })());",
            "let r = double(2);\n    assert!(match r { _ => true });",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {text}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(
                !is_fail && is_unknown,
                "{text}: not UNKNOWN: {basis:?} / {:?}",
                analysis.da_003
            );
        }
        for text in [
            "let r = double(2);\n    assert_eq!(r, 4);",
            "let r = double(2);\n    assert!({ r == 4 });",
            "let r = double(2);\n    assert_eq!({ let t = r; t }, 4);",
            "let range = double(2);\n    assert_eq!(&function[range], \"x\");",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {text}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::NoViolation),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A `let` outside the assert is derived from the target's result only where the
    /// target's value can flow to the initializer's value; a value dropped inside the
    /// initializer (a block statement, an operand a literal absorbs, a `match` scrutinee, a
    /// projection that leaves it out, a closure body) leaves the binding underived.
    /// @vtest.id TEST-ORACLE-DA-003-LET-INITIALIZER-DROPPING-CALL-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-ANALYSIS-LIMIT-UNKNOWN, VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::find_let_bindings_containing_call
    /// @vtest.intent let の初期化式の中で target の値が捨てられる位置(ブロックの文、リテラルに吸収される || の被演算子、match の被検査式、射影で残らない要素、入れ子の射影、vec! の添字で残らない要素、クロージャの本体)にあるとき、その束縛は target の結果から派生した束縛と数えず、束縛が assert に届いても DA-003 が UNKNOWN になり合成結果が PASS にならないことを確認する
    #[test]
    fn a_let_initializer_that_drops_the_call_is_unknown_not_pass() {
        for text in [
            "let r = { double(2); 1 };\n    assert_eq!(r, 1);",
            "let r = (double(2), 1).1;\n    assert_eq!(r, 1);",
            "let r = double(2) == 4 || true;\n    assert!(r);",
            "let r = match double(2) { _ => 1 };\n    assert_eq!(r, 1);",
            "let f = |x| { double(x); 1 };\n    assert_eq!(f(2), 1);",
            "let f = |x| double(x);\n    assert_eq!(f(2), 4);",
            "let r = ((double(2), 1), 2).0.1;\n    assert_eq!(r, 1);",
            "let r = vec![double(2), 1][1];\n    assert_eq!(r, 1);",
            "let r = [double(2), 1][1];\n    assert_eq!(r, 1);",
            "let r = Pair { a: double(2), b: 1 }.b;\n    assert_eq!(r, 1);",
            "let r = { let _ = double(2); 1 };\n    assert_eq!(r, 1);",
            "let r = { let x = double(2); 1 };\n    assert_eq!(r, 1);",
            "let r = if c { { double(2); 1 } } else { 0 };\n    assert_eq!(r, 1);",
            "let r = s.double(2).is_some() || true;\n    assert!(r);",
            "let r = { s.double(2); 1 };\n    assert_eq!(r, 1);",
            "let r = xs.iter().map(|x| double(*x)).count();\n    assert_eq!(r, 1);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {text}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(
                !is_fail && is_unknown,
                "{text}: not UNKNOWN: {basis:?} / {:?}",
                analysis.da_003
            );
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// A closure body and a nested projection inside an assert's checked argument
    /// do not carry the call's value (DS-629; a projection selects one element).
    /// @vtest.id TEST-ORACLE-DA-003-CLOSURE-AND-NESTED-PROJECTION-IN-ASSERT-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-ANALYSIS-LIMIT-UNKNOWN, VO-ORACLE-DA-003-UNKNOWN-BOUNDED-DATAFLOW
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::CheckedCallVisitor
    /// @vtest.intent assert の検査される引数の中で、インラインのクロージャの本体、入れ子の射影で残らない要素、vec! の添字で残らない要素にある target の呼出は検証済みと数えず、DA-003 が UNKNOWN になり合成結果が PASS にならないことを確認する
    #[test]
    fn a_closure_or_nested_projection_inside_an_assert_argument_is_unknown() {
        for text in [
            "assert_eq!((|x| double(x))(2), 4);",
            "assert_eq!(((double(2), 1), 2).0.1, 1);",
            "assert_eq!(vec![double(2), 1][1], 1);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {text}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(
                !is_fail && is_unknown,
                "{text}: not UNKNOWN: {basis:?} / {:?}",
                analysis.da_003
            );
        }
    }

    /// A `let` initializer in which the target's value flows to the initializer's
    /// value still derives the binding from the result.
    /// @vtest.id TEST-ORACLE-DA-003-LET-INITIALIZER-FLOWING-CALL-STAYS-NO-VIOLATION
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::find_let_bindings_containing_call
    /// @vtest.intent let の初期化式の中で target の値が初期化式の値へ流れる位置(式そのもの、二項演算、メソッドチェーン、? と unwrap、if と match の分岐の末尾、ブロックの末尾、射影された要素、構築子の引数)にあるとき、束縛は結果から派生したものと数えられ、assert に届けば DA-003 が違反なしのままであることを確認する
    #[test]
    fn a_let_initializer_that_carries_the_call_stays_no_violation() {
        for text in [
            "let r = double(2) + 1;\n    assert_eq!(r, 5);",
            "let r = c.double(2).unwrap();\n    assert_eq!(r, 4);",
            "let r = if c { double(2) } else { 0 };\n    assert_eq!(r, 4);",
            "let r = match c { 0 => 1, _ => double(2) };\n    assert_eq!(r, 4);",
            "let r = { let q = double(2); q };\n    assert_eq!(r, 4);",
            "let r = { double(1); double(2) };\n    assert_eq!(r, 4);",
            "let r = (double(2), 1).0;\n    assert_eq!(r, 4);",
            "let r = ((double(2), 1), 2).0.0;\n    assert_eq!(r, 4);",
            "let r = ((1, double(2)), 2).0.1;\n    assert_eq!(r, 4);",
            "let r = Pair { a: double(2), b: 1 }.a;\n    assert_eq!(r, 4);",
            "let r = [double(2), 1][0];\n    assert_eq!(r, 4);",
            "let r = Some(double(2));\n    assert_eq!(r, Some(4));",
            "let r = s.double(2)?;\n    assert_eq!(r, 4);",
            "let r = &double(2);\n    assert_eq!(r, &4);",
            "let r = double(2) as u8;\n    assert_eq!(r, 4);",
            "let r = Calc::double(2);\n    assert_eq!(r, 4);",
            "let mut r = double(2);\n    assert_eq!(r, 4);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {text}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::NoViolation),
                "{text}: {:?}",
                analysis.da_003
            );
        }
    }

    /// DS-1960: a call that is not a value conversion takes the result handed to
    /// its argument as a delegation target (DS-641), whose termination this
    /// capability does not confirm, so DA-003 is UNKNOWN (DS-652) when no result
    /// reaches an assert-equivalent directly.
    /// @vtest.id TEST-ORACLE-DA-003-RESULT-INTO-DELEGATION-TARGET-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-DELEGATE-FUNCTION-ARGS, VO-ORACLE-DA-003-DELEGATION-NOT-FAIL
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::CheckedCallVisitor
    /// @vtest.intent target の結果が、assert の検査される引数またはそこへ届く let 束縛の中で、値の変換でない関数呼出(利用者の関数、クロージャ、タプル構造体、T::new、PathBuf::from)の引数かメソッド呼出の引数へ流れ、assert へ直接届く結果が 1 つもない Test は、DA-003 が委譲(DS-1960)を理由に UNKNOWN になり、合成結果が PASS にならないことを確認する
    #[test]
    fn a_result_flowing_into_a_delegation_target_is_unknown() {
        for body in [
            "assert_eq!(always_3(double(2)), 3);",
            "assert!(check(double(2)));",
            "assert!(std::mem::drop(double(2)) == ());",
            "assert_eq!(identity(double(2)), 4);",
            "assert!((|x: i32| x == 4)(double(2)));",
            "assert_eq!(Wrapper(double(2)), Wrapper(4));",
            "assert_eq!(T::new(double(2)), T::new(4));",
            "assert_eq!(PathBuf::from(double(2)), p);",
            "assert!(v.contains(&double(2)));",
            "assert_eq!(Some(always_3(double(2))), Some(3));",
            "assert_eq!(always_3(double(2)).abs(), 3);",
            "assert!(matches!(always_3(double(2)), 3));",
            "let r = double(2);\n    assert_eq!(always_3(r), 3);",
            "let r = double(2);\n    assert!(v.contains(&r));",
            "let r = always_3(double(2));\n    assert_eq!(r, 3);",
            "assert_eq!({ let r = double(2); check(r) }, true);",
            "let r = double(2);\n    assert!(check(r));\n    assert!(check(double(3)));",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(!is_fail && is_unknown, "{body}: not UNKNOWN: {basis:?}");
            let DaVerdict::Unknown(reason) = &analysis.da_003 else {
                panic!("{body}: {:?}", analysis.da_003);
            };
            assert!(reason.contains("DS-1960"), "{body}: {reason}");
        }
        // A call in a method's turbofish is not carried to the method's value.
        for body in [
            "assert!({ x.f::<{ double(2) }>(); true });",
            "assert!(x.f::<{ double(2) }>() || true);",
            "assert!(f::<{ double(2) }>(1) == 1 || true);",
            "assert!({ f::<{ double(2) }>(); true });",
            "assert!({ (pick(double(2)))(); true });",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{body}: {:?}",
                analysis.da_003
            );
        }
        // The arguments of a declared target's call are not a value conversion
        // either (DS-1960), so the inner target's result is not carried there; it
        // is not a delegation target (DS-1960) either.
        let text = "#[test]\nfn t() {\n    assert_eq!(half(double(2)), 2);\n}\n";
        let analysis = analyze(text, &["double".to_owned(), "half".to_owned()], &[]);
        let DaVerdict::Unknown(reason) = &analysis.da_003 else {
            panic!("{:?}", analysis.da_003);
        };
        assert!(!reason.contains("DS-1960"), "{reason}");
    }

    /// A name handed to a delegation target, or to a declared target's call, is
    /// not shown to mean the binding when it is bound again or changed in place,
    /// so a verified method call beside it does not clear DA-003 (DS-1680).
    /// @vtest.id TEST-ORACLE-DA-003-DELEGATED-BINDING-REBOUND-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DA-003-DELEGATE-FUNCTION-ARGS
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::binding_reach
    /// @vtest.intent 委譲先か宣言 target の引数に渡る束縛の名前が再束縛や可変変更を受ける Test は、別の呼出が照合されていても DA-003 が違反なしにならず、宣言 target の引数に入るだけの束縛は、その結果が直接照合されないことを示す理由で UNKNOWN になることを確認する
    #[test]
    fn a_delegated_binding_that_is_bound_again_stays_unknown() {
        for body in [
            "let r = c.m(2);\n    assert!(check(r));\n    let r = 0;\n    c.m(3).unwrap();",
            "let mut r = c.m(2);\n    r.push(1);\n    assert!(check(r));\n    c.m(3).unwrap();",
            "let r = c.m(2);\n    assert!(v.contains(&r));\n    for r in xs {}\n    c.m(3)?;",
            "let r = c.m(2);\n    assert!(check(r));\n    let s = c.m(3);\n    assert_eq!(s, 1);\n    let r = 1;",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["m".to_owned()], &[]);
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(!is_fail && is_unknown, "{body}: not UNKNOWN: {basis:?}");
        }
        let text = "#[test]\nfn t() {\n    let yaml = to_yaml(&r);\n    assert_eq!(from_yaml(&yaml), r);\n}\n";
        let analysis = analyze(text, &["to_yaml".to_owned(), "from_yaml".to_owned()], &[]);
        let DaVerdict::Unknown(reason) = &analysis.da_003 else {
            panic!("{:?}", analysis.da_003);
        };
        assert!(
            reason.contains("only as an argument of another declared target"),
            "{reason}"
        );
    }

    /// The other side of DS-1960: a value conversion and a method's receiver
    /// carry the result, and a call that the result does not flow into, or that
    /// sits in a message argument (DS-1935), is not read as a delegation target.
    /// @vtest.id TEST-ORACLE-DA-003-VALUE-CONVERSION-STAYS-NO-VIOLATION
    /// @vtest.covers VO-ORACLE-DA-003-DELEGATE-FUNCTION-ARGS, VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::is_value_conversion
    /// @vtest.intent target の結果が Some / Ok / Err、Box::new、String::from の引数、メソッドの受け手、検証されない位置の外の関数呼出を通って assert に届く Test、結果が流れ込まない関数呼出やメッセージ引数の関数呼出だけがある Test、assert へ直接届く結果が 1 つ以上あり別の結果が委譲先へ渡る Test は、DA-003 が違反なしのままであることを確認する
    #[test]
    fn a_result_through_a_value_conversion_or_a_receiver_stays_no_violation() {
        for body in [
            "assert_eq!(Some(double(2)), Some(4));",
            "assert_eq!(Ok::<_, ()>(double(2)), Ok(4));",
            "assert_eq!(Err::<(), _>(double(2)), Err(4));",
            "assert_eq!(Option::Some(double(2)), Some(4));",
            "assert_eq!(Box::new(double(2)), Box::new(4));",
            "assert_eq!(std::boxed::Box::new(double(2)), b);",
            "assert_eq!(String::from(double(2)), s);",
            "assert_eq!(Some(Box::new(double(2))), b);",
            "assert!(double(2).contains(&x));",
            "assert_eq!(double(2).get(1), Some(&4));",
            "assert_eq!(double(2), expected());",
            "assert_eq!(double(2), 4, \"{}\", describe(x));",
            "assert_eq!(Some(double(2)).unwrap(), 4);",
            "let r = double(2);\n    assert!(r.contains(&x));",
            "let r = Some(double(2));\n    assert_eq!(r, Some(4));",
            "let r = Box::new(double(2));\n    assert_eq!(*r, 4);",
            "let r = double(2);\n    log(r);\n    assert_eq!(r, 4);",
            // The inner result is not carried into the target's own call (DS-1960),
            // but the outer result reaches the assert, which is enough (DS-1962).
            "assert_eq!(double(double(2)), 8);",
            // One result that reaches an assert-equivalent directly decides DA-003;
            // another result handed to a delegation target does not make it UNKNOWN
            // (derived from DS-623 and DS-1962).
            "let r = (double(2), check(double(3)));\n    assert_eq!(r.0, 4);",
            "let r = double(2);\n    assert_eq!(r, 4);\n    assert!(check(double(3)));",
            "assert!(check(double(3)));\n    let r = double(2);\n    assert_eq!(r, 4);",
            "assert_eq!(double(2), 4);\n    assert!(check(double(3)));",
            "assert!(check(double(3)));\n    assert_eq!(double(2), 4);",
            "assert!({ let r = double(2); r == 4 && check(r) });",
            "let r = double(2);\n    assert_eq!(r, 4);\n    assert!(check(r));",
            "let r = double(2);\n    assert!(r == 4 && check(r));",
            "assert_eq!(double(2).max(always_3(double(3))), 4);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::NoViolation),
                "{body}: {:?}",
                analysis.da_003
            );
        }
    }

    /// DS-1935: a message argument (of an assert macro, and of DS-619's
    /// `.expect(..)` / `.expect_err(..)`) is not a position the value flows
    /// through, so a call there neither verifies the target nor is a delegation
    /// target (DS-641 names only calls outside the assert-equivalents).
    /// @vtest.id TEST-ORACLE-DA-003-MESSAGE-ARGUMENT-IS-NOT-A-FLOWING-POSITION
    /// @vtest.covers VO-ORACLE-DA-003-MESSAGE-ARGUMENT-NOT-FLOWING
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::CheckedCallVisitor
    /// @vtest.intent target の結果が assert のメッセージ引数、または expect / expect_err の引数にだけ現れる Test は DA-003 が違反なしにならず、検査される引数で結果が届く Test はメッセージ引数の関数呼出があっても違反なしのままであることを確認する
    #[test]
    fn a_message_argument_is_not_a_flowing_position() {
        for body in [
            "assert!(other() > 0, \"{}\", double(2));",
            "assert!(true, \"{}\", double(2));",
            "let r = double(2);\n    assert!(other() > 0, \"{}\", r);",
            "assert_eq!(x.expect(&describe(double(2))), 4);",
            "let r = double(2);\n    assert_eq!(x.expect(&describe(r)), 4);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            let (_, _, basis) = compose(&analysis);
            assert!(
                matches!(analysis.da_003, DaVerdict::Unknown(_)),
                "{body}: {:?} {basis:?}",
                analysis.da_003
            );
        }
        for body in [
            "assert_eq!(double(2), 4, \"{}\", describe(double(3)));",
            "let r = double(2);\n    assert_eq!(r, 4, \"{}\", describe(r));",
            "let r = double(2);\n    assert_eq!(r.checked().expect(&describe(r)), 4);",
            "assert_eq!(double(2).expect_err(&describe(x)), e);",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_003, DaVerdict::NoViolation),
                "{body}: {:?}",
                analysis.da_003
            );
        }
    }

    /// The per-target DA-003 of `audit_rules` excludes every declared target from
    /// the delegation targets, as `analyze` does.
    /// @vtest.id TEST-ORACLE-DA-003-PER-TARGET-AGREES-WITH-ANALYZE
    /// @vtest.covers VO-ORACLE-DA-003-DELEGATE-FUNCTION-ARGS
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::analyze_da_003_for
    /// @vtest.intent 宣言 target が 2 つある Test で、一方の結果が他方の target の呼出の引数に渡る形を、target ごとの DA-003 が委譲と読まず、analyze の DA-003 と同じ判定になることを確認する
    #[test]
    fn the_per_target_da_003_does_not_read_another_target_as_a_delegation_target() {
        let targets = ["a".to_owned(), "b".to_owned()];
        let text = "#[test]\nfn t() {\n    assert_eq!(a(1), 2);\n    assert!(b(a(3)) == 4);\n}\n";
        assert_eq!(analyze(text, &targets, &[]).da_003, DaVerdict::NoViolation);
        for symbol in &targets {
            assert_eq!(
                analyze_da_003_for(text, symbol, &targets, &[]),
                DaVerdict::NoViolation,
                "{symbol}"
            );
        }
    }

    /// DS-1964: `.unwrap()` / `.expect(..)` / `?` on a value whose variant is
    /// `Some` / `Ok` by syntax, and `.unwrap_err()` / `.expect_err(..)` on one that
    /// is `Err`, cannot fail, so they are not assert-equivalents: a Test whose only
    /// verification is one of them does not get DA-006's no violation.
    /// @vtest.id TEST-ORACLE-DA-006-INFALLIBLE-UNWRAP-IS-NOT-VERIFICATION
    /// @vtest.covers VO-ORACLE-INFALLIBLE-UNWRAP-NOT-ASSERT-EQUIVALENT, VO-ORACLE-DA-006-NO-VERIFICATION-SYNTAX
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::VerificationSyntaxVisitor
    /// @vtest.intent 受け手の列挙子が構文から Some / Ok と決まる値(構築式と、それを一度だけ束縛した変数)への unwrap / expect / ?、Err と決まる値への unwrap_err / expect_err だけを検証構文に持つ Test は、DA-006 が違反なしにならず合成結果が PASS にならないことを確認する
    #[test]
    fn an_unwrap_that_cannot_fail_is_not_verification_syntax() {
        for text in [
            "#[test]\nfn t() {\n    Some(double(2)).unwrap();\n}\n",
            "#[test]\nfn t() {\n    Some(double(2)).expect(\"x\");\n}\n",
            "#[test]\nfn t() {\n    (Ok::<_, ()>(double(2))).unwrap();\n}\n",
            "#[test]\nfn t() {\n    Err::<(), _>(double(2)).unwrap_err();\n}\n",
            "#[test]\nfn t() {\n    Err::<(), _>(double(2)).expect_err(\"x\");\n}\n",
            "#[test]\nfn t() {\n    let o = Some(double(2));\n    o.unwrap();\n}\n",
            "#[test]\nfn t() {\n    let o: Option<i32> = Some(double(2));\n    o.expect(\"x\");\n}\n",
            "#[test]\nfn t() -> Option<()> {\n    Some(double(2))?;\n    Some(())\n}\n",
            "#[test]\nfn t() -> Option<()> {\n    let o = Some(double(2));\n    o?;\n    Some(())\n}\n",
        ] {
            let analysis = analyze(text, &["double".to_owned()], &[]);
            assert!(
                !matches!(analysis.da_006, DaVerdict::NoViolation),
                "{text}: {:?}",
                analysis.da_006
            );
            let (is_fail, is_unknown, basis) = compose(&analysis);
            assert!(is_fail || is_unknown, "{text}: composed to PASS: {basis:?}");
        }
    }

    /// The other side of DS-1964: a receiver whose variant is not fixed by syntax
    /// (a call's result, a branch, a `let mut` or a name bound again), or one whose
    /// unwrap can fail (`Ok(..).unwrap_err()`, `Err(..).unwrap()`), stays an
    /// assert-equivalent (DS-619).
    /// @vtest.id TEST-ORACLE-DA-006-FALLIBLE-UNWRAP-STAYS-VERIFICATION
    /// @vtest.covers VO-ORACLE-INFALLIBLE-UNWRAP-NOT-ASSERT-EQUIVALENT, VO-ORACLE-DA-003-DIRECT-ASSERT-METHOD
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::VerificationSyntaxVisitor
    /// @vtest.intent 受け手の列挙子が構文から決まらない値(呼出の結果、分岐、let mut、同じ名前の再束縛)、または失敗しうる組(Ok への unwrap_err、Err への unwrap)への unwrap / expect / ? は assert 相当のままで、DA-006 が違反なしのままであることを確認する
    #[test]
    fn an_unwrap_that_can_fail_stays_verification_syntax() {
        for body in [
            "double(2).unwrap();",
            "let o = double(2);\n    o.unwrap();",
            "let o = if c { Some(double(2)) } else { None };\n    o.unwrap();",
            "let mut o = Some(double(2));\n    o = None;\n    o.unwrap();",
            "let o = Some(double(2));\n    let o = other();\n    o.unwrap();",
            "Ok::<_, ()>(double(2)).unwrap_err();",
            "Err::<(), _>(double(2)).unwrap();",
            // The narrowed reach of DS-1964 disclosed in the module comment.
            "let mut o = Some(double(2));\n    o.unwrap();",
            "let (o, _) = (Some(double(2)), 1);\n    o.unwrap();",
            "let o = Some(double(2));\n    let o = o;\n    o.unwrap();",
        ] {
            let text = format!("#[test]\nfn t() {{\n    {body}\n}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_006, DaVerdict::NoViolation),
                "{body}: {:?}",
                analysis.da_006
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

    /// @vtest.id TEST-ORACLE-ASSERT-MACRO-CLOSED-LIST
    /// @vtest.covers VO-ORACLE-ASSERT-MACRO-CLOSED-LIST
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_006_no_verification_syntax
    /// @vtest.intent unreachable!・todo!・unimplemented! を含む本体は DA-006 が違反なしになり、DS-617 の列挙と config の assertion_macros にないマクロ（assert_matches!、debug_assert!）だけの本体は違反なしにならないことを確認する
    #[test]
    fn only_the_closed_list_of_macros_is_verification_syntax() {
        for name in ["unreachable", "todo", "unimplemented"] {
            let text = format!(
                "#[test]\nfn t() {{\n    let x = compute();\n    if x != 4 {{\n        {name}!();\n    }}\n}}\n"
            );
            let analysis = analyze(&text, &[], &[]);
            assert!(matches!(analysis.da_006, DaVerdict::NoViolation), "{text}");
        }
        let custom = "#[test]\nfn t() {\n    let x = 4;\n    assert_matches!(x, 4);\n}\n";
        assert!(matches!(
            analyze(custom, &[], &[]).da_006,
            DaVerdict::Fail(_)
        ));
        assert!(matches!(
            analyze(custom, &[], &["assert_matches!".to_owned()]).da_006,
            DaVerdict::NoViolation
        ));
        let debug = "#[test]\nfn t() {\n    let x = 4;\n    debug_assert!(x == 4);\n}\n";
        assert!(!matches!(
            analyze(debug, &[], &[]).da_006,
            DaVerdict::NoViolation
        ));
    }

    /// @vtest.id TEST-ORACLE-DIVERGENCE-ARGUMENT-METHOD-CALL-IS-A-CHANGE
    /// @vtest.covers VO-ORACLE-ASSERT-MACRO-CLOSED-LIST, VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent let mut の束縛を unreachable! などのメッセージ引数で読むだけなら assert_eq! への到達を保って DA-003 が違反なしになり、メソッドの受け手にすると UNKNOWN になることを確認する
    #[test]
    fn a_divergence_reading_a_mut_binding_keeps_it_but_a_method_call_does_not() {
        for name in ["unreachable", "todo", "unimplemented"] {
            let body = |argument: &str| {
                format!(
                    "#[test]\nfn t() {{\n    let mut r = double(2);\n    if c() {{\n        {name}!(\"{{}}\", {argument});\n    }}\n    assert_eq!(r, 4);\n}}\n"
                )
            };
            let read = body("r");
            assert!(
                matches!(
                    analyze(&read, &["double".to_owned()], &[]).da_003,
                    DaVerdict::NoViolation
                ),
                "{read}"
            );
            let changed = body("r.pow(2)");
            assert!(
                matches!(
                    analyze(&changed, &["double".to_owned()], &[]).da_003,
                    DaVerdict::Unknown(_)
                ),
                "{changed}"
            );
        }
    }

    /// @vtest.id TEST-ORACLE-CHANGE-INSIDE-DIVERGENCE-ARGUMENTS-IS-A-CHANGE
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::place_changes
    /// @vtest.intent panic! / unreachable! / todo! / unimplemented! の引数の中で let mut の束縛を代入・複合代入・クロージャ内の代入・&mut・メソッドの受け手・読まないマクロで変えると DA-003 が UNKNOWN になり、読むだけなら違反なしのままであることを確認する
    #[test]
    fn a_change_inside_a_divergence_argument_keeps_da_003_unknown() {
        let wrap = |inner: &str| {
            format!(
                "#[test]\nfn t() {{\n    let mut r = double(2);\n{inner}    assert_eq!(r, 4);\n}}\n"
            )
        };
        for name in ["panic", "unreachable", "todo", "unimplemented"] {
            for inner in [
                format!("    loop {{\n        {name}!(\"{{:?}}\", {{ r = 4; break; }});\n    }}\n"),
                format!(
                    "    for _ in 0..1 {{\n        {name}!(\"{{:?}}\", {{ r = 4; continue; }});\n    }}\n"
                ),
                format!("    if c() {{\n        {name}!(\"{{:?}}\", r += 0);\n    }}\n"),
                format!("    if c() {{\n        {name}!(\"{{:?}}\", (|| r = 4)());\n    }}\n"),
                format!("    if c() {{\n        {name}!(\"{{:?}}\", {{ let p = &mut r; *p = 4; }});\n    }}\n"),
                format!("    if c() {{\n        {name}!(\"{{}}\", r.pow(2));\n    }}\n"),
                format!("    if c() {{\n        {name}!(\"{{:?}}\", reset!(r));\n    }}\n"),
                format!("    if c() {{\n        {name}!(\"{{:?}}\", r +);\n    }}\n"),
                format!("    if c() {{\n        {name}!(\"{{:?}}\", format!(\"{{}}\", {{ r = 4; r }}));\n    }}\n"),
            ] {
                let text = wrap(&inner);
                assert!(
                    matches!(
                        analyze(&text, &["double".to_owned()], &[]).da_003,
                        DaVerdict::Unknown(_)
                    ),
                    "{text}"
                );
            }
            let read = wrap(&format!(
                "    if c() {{\n        {name}!(\"{{}}\", r);\n    }}\n"
            ));
            assert!(
                matches!(
                    analyze(&read, &["double".to_owned()], &[]).da_003,
                    DaVerdict::NoViolation
                ),
                "{read}"
            );
        }
        // The plain name assigned inside an assert's argument is a change too.
        let in_assert = wrap("    assert!({ r = 4; true });\n");
        assert!(matches!(
            analyze(&in_assert, &["double".to_owned()], &[]).da_003,
            DaVerdict::Unknown(_)
        ));
    }

    /// @vtest.id TEST-ORACLE-DA-003-FAIL-OF-ONE-TARGET-DECIDES
    /// @vtest.covers VO-ORACLE-DA-003-RESULT-UNVERIFIED
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent 宣言 target が複数あり、1 つの結果が debug_assert にだけ流れて UNKNOWN、もう 1 つの結果がどこにも届かず FAIL のとき、DA-003 が FAIL になり、名前でしか照合できないメソッド呼出があるときは UNKNOWN のままであることを確認する
    #[test]
    fn a_fail_of_one_target_decides_over_another_targets_unknown() {
        let targets = ["double".to_owned(), "triple".to_owned()];
        let text = "#[test]\nfn t() {\n    let a = double(2);\n    let b = triple(2);\n    debug_assert_eq!(a, 4);\n}\n";
        assert!(matches!(
            analyze(text, &targets, &[]).da_003,
            DaVerdict::Fail(_)
        ));
        let targets_with_check = ["double".to_owned(), "check".to_owned()];
        for (targets, body) in [
            (
                &targets_with_check,
                "    let a = double(2);\n    v.check(a);\n",
            ),
            (
                &targets_with_check,
                "    let a = double(2);\n    v.check(&a);\n",
            ),
            (
                &targets,
                "    let a = double(2);\n    let mut b = 0;\n    x.triple(&mut b, a);\n",
            ),
            (
                &targets,
                "    let a = double(2);\n    let b = x.triple(2);\n",
            ),
        ] {
            let text = format!("#[test]\nfn t() {{\n{body}}}\n");
            assert!(
                matches!(analyze(&text, targets, &[]).da_003, DaVerdict::Unknown(_)),
                "{text}"
            );
        }
        let one = "#[test]\nfn t() {\n    let a = double(2);\n    debug_assert_eq!(a, 4);\n}\n";
        assert!(matches!(
            analyze(one, &targets, &[]).da_003,
            DaVerdict::Unknown(_)
        ));
    }

    /// @vtest.id TEST-ORACLE-DA-001-UNCONDITIONAL-DIVERGENCE-IS-CONSTANT
    /// @vtest.covers VO-ORACLE-DIVERGENCE-CONSTANCY-BY-DECIDING-POSITION
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_001_constant_assertion
    /// @vtest.intent 無条件に実行される unreachable!・todo!・unimplemented! は、メッセージ引数に識別子があっても DA-001 の定数アサーションとして FAIL になり、#[should_panic] があっても合成結果が PASS にならないことを確認する
    #[test]
    fn an_unconditional_divergence_is_a_constant_assertion_whatever_its_message() {
        for name in ["unreachable", "todo", "unimplemented"] {
            for body in [
                format!("    {name}!();\n"),
                format!("    let x = double(0);\n    {name}!(\"{{}}\", x);\n"),
                format!("    println!(\"{{:?}}\", 1);\n    {name}!(\"{{}}\", y)\n"),
            ] {
                let text = format!("#[should_panic]\n#[test]\nfn t() {{\n{body}}}\n");
                let analysis = analyze(&text, &["double".to_owned()], &[]);
                assert!(matches!(analysis.da_001, DaVerdict::Fail(_)), "{text}");
                assert!(compose(&analysis).0, "{text}");
            }
        }
    }

    /// @vtest.id TEST-ORACLE-DA-001-CONDITIONAL-DIVERGENCE-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-ANALYSIS-LIMIT-UNKNOWN
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_001_constant_assertion
    /// @vtest.intent 条件・分岐・先行する return / ? / exit の後にある unreachable! は、決める位置を評価しないため DA-001 が FAIL でも違反なしでもなく UNKNOWN になり、識別子を含む assert が他にあれば違反なしになることを確認する
    #[test]
    fn a_divergence_under_a_condition_or_after_an_exit_is_unknown_for_da_001() {
        for body in [
            "    let x = double(2);\n    if x != 4 {\n        unreachable!();\n    }\n",
            "    if true {\n        unreachable!(\"{}\", y);\n    }\n",
            "    match double(2) {\n        4 => {}\n        _ => unreachable!(),\n    }\n",
            "    if double(2) == 4 {\n        return;\n    }\n    unreachable!();\n",
            "    while c() {\n        break;\n    }\n    todo!();\n",
            "    my_macro!();\n    unimplemented!();\n",
            "    if double(2) != 4 {\n        std::process::exit(1);\n    }\n    unreachable!();\n",
        ] {
            let text = format!("#[test]\nfn t() {{\n{body}}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(
                matches!(analysis.da_001, DaVerdict::Unknown(_)),
                "{text}: {:?}",
                analysis.da_001
            );
        }
        let with_assert =
            "#[test]\nfn t() {\n    let x = double(2);\n    assert_eq!(x, 4);\n    unreachable!();\n}\n";
        assert!(matches!(
            analyze(with_assert, &["double".to_owned()], &[]).da_001,
            DaVerdict::NoViolation
        ));
    }

    /// @vtest.id TEST-ORACLE-DA-001-DIVERGENCE-WITH-UNWRAP-IS-NOT-CONSTANT
    /// @vtest.covers VO-ORACLE-DA-001-CONSTANT-ASSERTION
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_001_constant_assertion
    /// @vtest.intent assert マクロが unreachable! などや定数の assert! だけでも、識別子を受け手にする .unwrap()・.expect(..)・? があれば DA-001 が違反なしになり、失敗しえない Some(..).unwrap() は数えず assert!(true) の本体が FAIL のままであり、クロージャ・async ブロック・入れ子の fn の中にだけある .unwrap() は数えず DA-001 が UNKNOWN になることを確認する
    #[test]
    fn a_divergence_beside_an_unwrap_or_try_is_not_a_constant_assertion() {
        for body in [
            "    let v = parse(\"x\").unwrap();\n    unreachable!();\n",
            "    let v = double(2).checked_add(1).expect(\"p\");\n    match v {\n        5 => {}\n        _ => todo!(),\n    }\n",
            "    let v = parse(\"x\")?;\n    if v != 4 {\n        unimplemented!();\n    }\n",
            "    let v = parse(\"x\").unwrap();\n    assert!(true);\n    unreachable!();\n",
            "    let v = double(2).unwrap();\n    assert!(v == 4 || true);\n",
            "    let v = double(2)?;\n    assert!(v == 4 || true);\n",
        ] {
            let text = format!("#[test]\nfn t() {{\n{body}}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(matches!(analysis.da_001, DaVerdict::NoViolation), "{text}");
        }
        // DS-1964: an `.unwrap()` that cannot fail is not an assert-equivalent.
        let constant = "#[test]\nfn t() {\n    let v = Some(1).unwrap();\n    assert!(true);\n}\n";
        assert!(matches!(
            analyze(constant, &[], &[]).da_001,
            DaVerdict::Fail(_)
        ));
        // One only in a closure, an `async` block or a nested item may not run, so it
        // is not counted, and DA-001 is neither FAIL nor no violation.
        for body in [
            "    let _f = || \"x\".parse::<i32>().unwrap();\n    assert!(true);\n",
            "    fn helper() -> i32 {\n        \"4\".parse().unwrap()\n    }\n    assert!(true);\n",
            "    assert!(true);\n    let _x = async { cfg().unwrap() };\n",
            "    let _f = |x: Option<i32>| x.unwrap();\n    unreachable!();\n",
        ] {
            let text = format!("#[test]\nfn t() {{\n{body}}}\n");
            let analysis = analyze(&text, &[], &[]);
            assert!(
                matches!(analysis.da_001, DaVerdict::Unknown(_)),
                "{text}: {:?}",
                analysis.da_001
            );
            assert!(!compose(&analysis).0 && compose(&analysis).1, "{text}");
        }
    }

    /// @vtest.id TEST-ORACLE-DEBUG-ASSERT-ALONE-IS-UNKNOWN
    /// @vtest.covers VO-ORACLE-DEBUG-ASSERT-NOT-ASSERT-EQUIVALENT
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent target の結果が debug_assert* にだけ流れる Test は DA-003 と DA-006 が FAIL でも違反なしでもなく UNKNOWN になり、config の assertion_macros に debug_assert* を挙げても同じであることを確認する（#294）
    #[test]
    fn a_result_checked_only_by_debug_assert_is_unknown() {
        for config in [
            Vec::new(),
            vec!["debug_assert_eq".to_owned()],
            vec!["debug_assert_eq!".to_owned()],
        ] {
            for body in [
                "    debug_assert_eq!(double(2), 4);\n",
                "    let r = double(2);\n    debug_assert_eq!(r, 4);\n",
                "    let r = double(2);\n    debug_assert!(r == 4);\n",
                "    let r = double(2);\n    debug_assert_ne!(r.clone(), 5);\n",
            ] {
                let text = format!("#[test]\nfn t() {{\n{body}}}\n");
                let analysis = analyze(&text, &["double".to_owned()], &config);
                assert!(
                    matches!(analysis.da_003, DaVerdict::Unknown(_)),
                    "{text}: {:?}",
                    analysis.da_003
                );
                assert!(
                    matches!(analysis.da_006, DaVerdict::Unknown(_)),
                    "{text}: {:?}",
                    analysis.da_006
                );
                let (is_fail, is_unknown, _) = compose(&analysis);
                assert!(!is_fail && is_unknown, "{text}");
            }
        }
    }

    /// @vtest.id TEST-ORACLE-DEBUG-ASSERT-IS-NOT-AN-ASSERT-EQUIVALENT
    /// @vtest.covers VO-ORACLE-DEBUG-ASSERT-NOT-ASSERT-EQUIVALENT
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent debug_assert* は assert 相当として違反なしの根拠にならず（assert の引数の中の debug_assert* も同じ）、結果が流れない debug_assert* やメッセージ引数だけの debug_assert* では DA-003 が FAIL のままで、他の assert 相当に到達すれば違反なしになることを確認する
    #[test]
    fn a_debug_assert_never_counts_as_an_assert_equivalent() {
        let nested =
            "#[test]\nfn t() {\n    assert_eq!({ debug_assert_eq!(double(2), 4) }, ());\n}\n";
        assert!(matches!(
            analyze(nested, &["double".to_owned()], &[]).da_003,
            DaVerdict::Unknown(_)
        ));
        for body in [
            "    let r = double(2);\n    debug_assert!(true);\n",
            "    let r = double(2);\n    debug_assert!(true, \"{}\", r);\n",
        ] {
            let text = format!("#[test]\nfn t() {{\n{body}}}\n");
            let analysis = analyze(&text, &["double".to_owned()], &[]);
            assert!(matches!(analysis.da_003, DaVerdict::Fail(_)), "{text}");
        }
        let checked =
            "#[test]\nfn t() {\n    let mut r = double(2);\n    debug_assert!(r > 0);\n    assert_eq!(r, 4);\n}\n";
        let analysis = analyze(checked, &["double".to_owned()], &[]);
        assert!(matches!(analysis.da_003, DaVerdict::NoViolation));
        assert!(matches!(analysis.da_006, DaVerdict::NoViolation));
    }
}
