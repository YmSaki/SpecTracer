//! `kotlin`: Kotlin sources and scripts (`.kt`, `.kts`).
//!
//! Functions are `fun` declarations with a block body or an expression body
//! (`fun f() = expr`), including extension functions (named without their
//! receiver) and backtick-quoted names (`` fun `adds numbers`() ``).
//! `class`, `interface`, `object`, and `companion object` (named
//! `Companion` when unnamed) qualify them. Annotations (`@Test`,
//! `@ParameterizedTest`) are part of the construct, so JUnit, kotlin.test,
//! Kotest (`"adds" { ... }`), or Spek are discovered the same way.

use crate::{
    header::{
        any_top_level, find_top_level_word, first_paren, matching_close, name_before,
        skip_attributes, AttributeForms, HTok, HeaderKind,
    },
    lexer::{LexicalSyntax, StringSyntax, Structure, TokKind},
    LanguageProfile, LexicalAdapter,
};

pub const ADAPTER_ID: &str = "kotlin";

const SYNTAX: LexicalSyntax = LexicalSyntax {
    line_comments: &["//"],
    line_comment_exclusions: &[],
    block_comments: &[("/*", "*/")],
    nested_block_comments: true,
    strings: &[
        StringSyntax::multi("\"\"\"", "\"\"\"", false),
        StringSyntax::single("\"", "\""),
        StringSyntax::single("'", "'"),
        StringSyntax::single("`", "`"),
    ],
    preprocessor: false,
    cpp_raw_strings: false,
    word_suffix_chars: &[],
    newline_terminates: true,
    structure: Structure::Braces,
};

const SCOPE_KEYWORDS: &[&str] = &["class", "interface", "object"];

#[derive(Clone, Copy, Debug, Default)]
pub struct Kotlin;

impl LanguageProfile for Kotlin {
    fn adapter_id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn languages(&self) -> &'static [&'static str] {
        &["kotlin"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["kt", "kts"]
    }

    fn excluded_dirs(&self) -> &'static [&'static str] {
        &["build", "out", ".gradle"]
    }

    fn syntax(&self) -> &'static LexicalSyntax {
        &SYNTAX
    }

    fn classify(&self, header: &[HTok<'_>], has_body: bool) -> HeaderKind {
        let start = skip_attributes(
            header,
            AttributeForms {
                at_sign: true,
                ..AttributeForms::default()
            },
        );
        let header = &header[start..];
        if any_top_level(header, 0, header.len(), |token| token.is_punct('='))
            && !header_declares_fun(header)
        {
            return HeaderKind::Other;
        }
        if let Some(fun) = find_top_level_word(header, 0, header.len(), &["fun"]) {
            return classify_fun(header, fun, has_body);
        }
        let Some(keyword) = find_top_level_word(header, 0, header.len(), SCOPE_KEYWORDS) else {
            return HeaderKind::Other;
        };
        if !has_body {
            return HeaderKind::Other;
        }
        match header.get(keyword + 1) {
            Some(token) if token.kind == TokKind::Word => {
                HeaderKind::Scope(vec![token.text.to_owned()])
            }
            _ if keyword > 0 && header[keyword - 1].is_word("companion") => {
                HeaderKind::Scope(vec!["Companion".to_owned()])
            }
            _ => HeaderKind::Other,
        }
    }
}

fn header_declares_fun(header: &[HTok<'_>]) -> bool {
    find_top_level_word(header, 0, header.len(), &["fun"]).is_some_and(|fun| {
        header[..fun]
            .iter()
            .all(|token| token.kind == TokKind::Word)
    })
}

fn classify_fun(header: &[HTok<'_>], fun: usize, has_body: bool) -> HeaderKind {
    // Everything before `fun` must be modifiers (`override`, `suspend`, ...).
    if !header[..fun]
        .iter()
        .all(|token| token.kind == TokKind::Word)
    {
        return HeaderKind::Other;
    }
    let Some(paren) = first_paren(header, fun + 1) else {
        return HeaderKind::Other;
    };
    let name = match header.get(paren.wrapping_sub(1)) {
        Some(token) if token.kind == TokKind::Str && token.text.starts_with('`') => {
            token.text.trim_matches('`').to_owned()
        }
        _ => match name_before(header, paren) {
            Some((_, segments)) => segments.last().cloned().unwrap_or_default(),
            None => return HeaderKind::Other,
        },
    };
    if name.is_empty() {
        return HeaderKind::Other;
    }
    if has_body {
        return HeaderKind::Function(vec![name]);
    }
    let close = matching_close(header, paren).unwrap_or(header.len());
    if header[close..].iter().any(|token| token.is_punct('=')) {
        return HeaderKind::Function(vec![name]);
    }
    HeaderKind::Other
}

/// The `kotlin` adapter.
pub fn adapter() -> LexicalAdapter<Kotlin> {
    LexicalAdapter::new(Kotlin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover_text;

    const SOURCE: &str = r#"package demo

import kotlin.test.Test

/* outer /* nested */ still comment { */
class Calc(private val base: Int = 0) : Base() {
    // @vtest.src-id SRC-KT-ADD
    fun add(a: Int, b: Int): Int {
        return base + a + b
    }

    fun twice(a: Int) = a * 2

    abstract fun undefined(): Int

    companion object {
        fun create(): Calc = Calc()
    }

    private val handler = object : Runnable { override fun run() {} }
}

fun <T> List<T>.second(): T = this[1]

val template = """{ "not": "code" }"""

class CalcTest {
    // @vtest.id TEST-KT-ADD
    // @vtest.covers VO-KT-ADD
    // @vtest.target src/Calc.kt::Calc::add
    // @vtest.intent adding two numbers returns their sum
    @Test
    fun `adds two numbers`() {
        assertEquals(3, Calc().add(1, 2))
    }
}
"#;

    /// @vtest.id TEST-ADAPTER-KOTLIN-DISCOVERS-FUNS-AND-TESTS
    /// @vtest.covers VO-ADAPTER-KOTLIN-REGISTERS-WITHOUT-CORE-CHANGES, VO-ADAPTER-LEXICAL-OPAQUE-LOCATOR-SYNTAX
    /// @vtest.target crates/vtest-adapter-lexical/src/kotlin.rs::Kotlin::classify
    /// @vtest.intent verifies Kotlin block-bodied, expression-bodied, extension, companion, and backtick-named funs become class-qualified Source Targets while abstract funs and anonymous objects do not, and an annotated test fun is a Test construct including its annotation
    #[test]
    fn discovers_kotlin_funs_and_tests() {
        let outcome = discover_text(&Kotlin, "src/Calc.kt", SOURCE, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        let sources = outcome
            .sources
            .iter()
            .map(|source| source.locator.value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            sources,
            vec![
                "src/Calc.kt::Calc::add",
                "src/Calc.kt::Calc::twice",
                "src/Calc.kt::Calc::Companion::create",
                "src/Calc.kt::second",
                "src/Calc.kt::CalcTest::adds two numbers",
            ]
        );
        assert_eq!(
            outcome.sources[0].src_id.as_ref().map(|id| id.as_str()),
            Some("SRC-KT-ADD")
        );
        assert_eq!(
            outcome.sources[1].construct_text,
            "fun twice(a: Int) = a * 2"
        );
        assert_eq!(outcome.tests.len(), 1);
        let test = &outcome.tests[0];
        assert_eq!(test.id.as_str(), "TEST-KT-ADD");
        assert_eq!(test.location.locator, "CalcTest::adds two numbers");
        assert!(test
            .construct_text
            .starts_with("@Test\n    fun `adds two numbers`() {"));
        assert!(test.construct_text.ends_with("add(1, 2))\n    }"));
    }
}
