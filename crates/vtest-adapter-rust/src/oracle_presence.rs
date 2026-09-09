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
//!   list, or bound by a single `let NAME = ...;` whose `NAME` (or a
//!   `NAME.field`/`NAME.method()` chain starting with `NAME`) then appears
//!   inside an assert-equivalent argument list. This is a real subset of
//!   DS-628 ("let束縛、メソッドチェーン、フィールドアクセス"), not its full
//!   extent — a target call several bindings removed, or reached only
//!   through a closure, is not tracked and reports `Unknown` rather than
//!   `NoViolation` (fail-closed: DS-615 "違反なしと推測しない").
//! - §7.2.1's cross-Test delegation-chain search (DA-003/DA-006's "照合の
//!   委譲先") is not implemented at all. This capability therefore never
//!   identifies a delegation target, which is consistent with DA-006's FAIL
//!   condition's own literal wording ("…かつ§7.2.1の照合の委譲先も同定でき
//!   ないこと" — not identifying one is not a violation of the rule, it
//!   forecloses only the delegation-based `Unknown` outcome DS-623/626 name
//!   for when a delegation candidate exists but cannot be confirmed to
//!   terminate). A Test that in fact delegates its verification to another
//!   Test's assertions is not recognized as doing so and may be reported
//!   `FAIL` where a delegation-aware analysis would report `Unknown`.
//! - DA-001's constant-expression classifier only recognizes literals
//!   (numeric/string/bool/char) and simple arithmetic/comparison
//!   combinations of literals; any identifier reference (a `const` item
//!   included) is treated as non-constant rather than resolved, which can
//!   only make DA-001 report `NoViolation` where a deeper analysis might
//!   report `Fail` — never the reverse — so this narrowing cannot manufacture
//!   a false `Fail`.

use syn::spanned::Spanned;
use vtest_model::{Diagnostic, Locator};

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
    let has_should_panic = construct_text.contains("#[should_panic");
    let has_unwrap_or_expect_or_try = construct_text.contains(".unwrap()")
        || construct_text.contains(".unwrap_err()")
        || construct_text.contains(".expect(")
        || construct_text.contains(".expect_err(")
        || contains_try_operator(construct_text);
    let has_result_return = signature_returns_result(construct_text);
    let has_verification_syntax = !assert_spans.is_empty()
        || has_should_panic
        || has_unwrap_or_expect_or_try
        || has_result_return;

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
        da_006: da_006_no_verification_syntax(has_verification_syntax),
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
    let mut any_unverified = false;
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
        return if assert_spans.is_empty() {
            DaVerdict::Fail(
                "a declared target's call result reaches no assert-equivalent construct, \
                 and no #[should_panic] is present (DS-623)",
            )
        } else {
            DaVerdict::Unknown(
                "a declared target's call could not be traced into any of the function's \
                 assert-equivalent constructs by this capability's bounded dataflow \
                 tracking (a method chain, a second let-binding, or a field access this \
                 capability does not follow could still carry the result there) — neither \
                 reaching nor non-reaching can be said deterministically, which is DS-1680's \
                 own UNKNOWN condition, not a structural proof of non-verification"
                    .to_owned(),
            )
        };
    }
    DaVerdict::NoViolation
}

enum TargetCallVerification {
    NotCalled,
    Verified,
    Unverified,
    Ambiguous,
}

fn target_call_result_is_verified(
    text: &str,
    symbol: &str,
    assert_spans: &[(usize, usize)],
) -> TargetCallVerification {
    let call_sites = find_call_sites(text, symbol);
    if call_sites.is_empty() {
        return TargetCallVerification::NotCalled;
    }
    for &call_start in &call_sites {
        if assert_spans
            .iter()
            .any(|&(start, end)| call_start >= start && call_start < end)
        {
            return TargetCallVerification::Verified;
        }
        if call_is_inside_loop(text, call_start) {
            return TargetCallVerification::Ambiguous;
        }
        if target_call_has_assert_method_chain(text, call_start, symbol) {
            return TargetCallVerification::Verified;
        }
    }
    // DS-628's disclosed bound: a single `let NAME = ...<call>...;` binding,
    // where `NAME` (optionally followed by `.field`/`.method()`) later
    // appears inside an assert span.
    for binding in find_let_bindings_containing_call(text, symbol) {
        if assert_spans.iter().any(|&(start, end)| {
            text[start..end].contains(&binding) || text[start..end].contains(&format!("{binding}."))
        }) {
            return TargetCallVerification::Verified;
        }
    }
    // A mutable-reference or global-state argument to the call is this
    // capability's disclosed ambiguity escape (DS-623's own UNKNOWN
    // example). Scoped to the call's own argument list (the parenthesized
    // span right after `symbol`), not the rest of the function text —
    // scanning past the call site would flag an unrelated `&mut ` written
    // anywhere later in the function as if it belonged to this call.
    if call_sites.iter().any(|&start| {
        call_argument_list(text, start, symbol).is_some_and(|arguments| arguments.contains("&mut "))
    }) {
        return TargetCallVerification::Ambiguous;
    }
    if call_sites
        .iter()
        .any(|&start| target_call_is_passed_to_other_function(text, start, symbol))
    {
        return TargetCallVerification::Ambiguous;
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
        ".expect_err(",
        ".unwrap()",
        ".unwrap_err()",
        "?",
    ]
    .iter()
    .any(|token| suffix.starts_with(token))
}

fn call_close_position(text: &str, call_start: usize, symbol: &str) -> Option<usize> {
    let after = &text[call_start + symbol.len()..];
    let trimmed = after.trim_start();
    let open = call_start + symbol.len() + after.len() - trimmed.len();
    if !text[open..].starts_with('(') {
        return None;
    }
    find_matching_close(text, open, '(', ')')
}

/// Calls in a loop are outside this bounded analysis: iteration and control
/// flow can determine whether every result reaches a verifier.
fn call_is_inside_loop(text: &str, call_start: usize) -> bool {
    let prefix = &text[..call_start];
    let loop_start = ["loop", "for ", "while "]
        .iter()
        .filter_map(|keyword| prefix.rfind(keyword))
        .max();
    let Some(loop_start) = loop_start else {
        return false;
    };
    let Some(open) = text[loop_start..]
        .find('{')
        .map(|offset| loop_start + offset)
    else {
        return false;
    };
    find_matching_close(text, open, '{', '}').is_some_and(|close| call_start < close)
}

/// Passing a target result to an ordinary function is a delegation/data-flow
/// shape this adapter cannot resolve, so DS-1680 requires UNKNOWN.
fn target_call_is_passed_to_other_function(text: &str, call_start: usize, symbol: &str) -> bool {
    let Some(close) = call_close_position(text, call_start, symbol) else {
        return false;
    };
    let before = text[..call_start].trim_end();
    before.ends_with('(')
        || before.ends_with(',')
        || text[close + 1..].trim_start().starts_with(",")
}

/// Returns the argument-list text (inside the parens) of a call to `symbol`
/// whose name starts at byte offset `call_start` in `text`.
fn call_argument_list<'a>(text: &'a str, call_start: usize, symbol: &str) -> Option<&'a str> {
    let after = &text[call_start + symbol.len()..];
    let trimmed = after.trim_start();
    let skip = after.len() - trimmed.len();
    let open_pos = call_start + symbol.len() + skip;
    if !text[open_pos..].starts_with('(') {
        return None;
    }
    let close_pos = find_matching_close(text, open_pos, '(', ')')?;
    Some(&text[open_pos + 1..close_pos])
}

fn find_call_sites(text: &str, symbol: &str) -> Vec<usize> {
    let mut sites = Vec::new();
    let mut search_from = 0;
    while let Some(relative) = text[search_from..].find(symbol) {
        let start = search_from + relative;
        let before_ok = start == 0
            || !text[..start]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == ':');
        let after = &text[start + symbol.len()..];
        let after_ok = after.trim_start().starts_with('(');
        if before_ok && after_ok {
            sites.push(start);
        }
        search_from = start + symbol.len();
    }
    sites
}

fn find_let_bindings_containing_call(text: &str, symbol: &str) -> Vec<String> {
    let mut bindings = Vec::new();
    let mut search_from = 0;
    while let Some(relative) = text[search_from..].find("let ") {
        let let_start = search_from + relative;
        let after_let = &text[let_start + 4..];
        let Some(name_end) = after_let.find(|c: char| !c.is_alphanumeric() && c != '_') else {
            search_from = let_start + 4;
            continue;
        };
        let name = after_let[..name_end].trim();
        let Some(semicolon_relative) = after_let.find(';') else {
            search_from = let_start + 4;
            continue;
        };
        let statement = &after_let[..semicolon_relative];
        if !name.is_empty()
            && statement.contains(symbol)
            && !find_call_sites(statement, symbol).is_empty()
        {
            bindings.push(name.to_owned());
        }
        search_from = let_start + 4 + semicolon_relative;
    }
    bindings
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
    let Some(body_start) = text.find('{') else {
        return DaVerdict::NoViolation;
    };
    let Some(body_end) = text.rfind('}') else {
        return DaVerdict::NoViolation;
    };
    if body_end <= body_start {
        return DaVerdict::NoViolation;
    }
    let body = &text[body_start + 1..body_end];
    let stripped = strip_comments(body);
    if stripped.trim().is_empty() {
        DaVerdict::Fail("the Test function body has no statements (DS-625)")
    } else {
        DaVerdict::NoViolation
    }
}

fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '/' && chars.peek() == Some(&'/') {
            for next in chars.by_ref() {
                if next == '\n' {
                    break;
                }
            }
            continue;
        }
        out.push(ch);
    }
    out
}

// ---------------------------------------------------------------------------
// DA-006 (DS-626): no verification syntax
// ---------------------------------------------------------------------------

fn da_006_no_verification_syntax(has_verification_syntax: bool) -> DaVerdict {
    if has_verification_syntax {
        DaVerdict::NoViolation
    } else {
        // This capability does not identify §7.2.1 delegation targets at
        // all (module doc): DS-626's FAIL condition ("…かつ§7.2.1の照合の
        // 委譲先も同定できないこと") is literally satisfied whenever no
        // delegation-identification logic exists to name one.
        DaVerdict::Fail(
            "no assert-equivalent construct (standard macro, #[should_panic], \
             .unwrap()/.expect()/?, or a Result-returning signature) is present, \
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

fn span_offset(location: proc_macro2::LineColumn, text: &str) -> usize {
    text.lines()
        .take(location.line.saturating_sub(1))
        .map(|line| line.len() + 1)
        .sum::<usize>()
        + location.column
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

fn contains_try_operator(text: &str) -> bool {
    // A bare `?` right after an expression (not inside a string/char
    // literal, and not `?` used inside a type like `Option<T>?` generics —
    // Rust does not use `?` in type position, so this simple scan is
    // sound for the try operator specifically). Comments/strings are not
    // stripped first, which can over-count a `?` appearing inside a
    // string literal as a try-operator use; that only widens
    // `has_verification_syntax`, which per this module's doc comment can
    // only move DA-006 toward `NoViolation`, never fabricate a `Fail`.
    text.contains('?')
}

fn signature_returns_result(text: &str) -> bool {
    let Some(body_start) = text.find('{') else {
        return false;
    };
    text[..body_start].contains("-> Result")
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
        let (is_fail, is_unknown, basis) = compose(&analysis);
        assert!(!is_fail, "{basis:?}");
        assert!(!is_unknown, "{basis:?}");
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
    /// @vtest.covers VO-ORACLE-DA-003-DIRECT-ASSERT-METHOD
    /// @vtest.target crates/vtest-adapter-rust/src/oracle_presence.rs::da_003_result_unverified
    /// @vtest.intent target呼出結果へ直接expect_errを連鎖した場合に到達と判定することを確認する
    #[test]
    fn a_direct_expect_err_call_passes_da_003_and_da_006() {
        let text = "#[test]\nfn rejects() {\n    parse(input).expect_err(\"invalid\");\n}\n";
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
