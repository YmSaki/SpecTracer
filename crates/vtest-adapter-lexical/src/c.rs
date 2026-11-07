//! `c`: C sources and headers (`.c`, `.h`).
//!
//! Functions are definitions with a body, including K&R-style headers split
//! over several lines and function-like macro blocks such as Check's
//! `START_TEST(name) { ... }`. Preprocessor lines are directives, never
//! code. Unity, CMocka, Check, or a hand-written `main` are discovered the
//! same way, because a Test is whatever follows its `@vtest.` comment.

use crate::{
    header::{classify_c_like, AttributeForms, CLikeRules, HTok, HeaderKind},
    lexer::{LexicalSyntax, StringSyntax, Structure},
    LanguageProfile, LexicalAdapter,
};

pub const ADAPTER_ID: &str = "c";

const SYNTAX: LexicalSyntax = LexicalSyntax {
    line_comments: &["//"],
    line_comment_exclusions: &[],
    block_comments: &[("/*", "*/")],
    nested_block_comments: false,
    strings: &[
        StringSyntax::single("\"", "\""),
        StringSyntax::single("'", "'"),
    ],
    preprocessor: true,
    cpp_raw_strings: false,
    word_suffix_chars: &[],
    newline_terminates: false,
    structure: Structure::Braces,
};

const RULES: CLikeRules = CLikeRules {
    // C23 `[[nodiscard]]` attributes.
    attributes: AttributeForms {
        at_sign: false,
        hash_bracket: false,
        bracket: true,
    },
    scope_keywords: &[],
    scope_rest_keywords: &[],
    arrow_bodies: false,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct C;

impl LanguageProfile for C {
    fn adapter_id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn languages(&self) -> &'static [&'static str] {
        &["c"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["c", "h"]
    }

    fn excluded_dirs(&self) -> &'static [&'static str] {
        &["build"]
    }

    fn syntax(&self) -> &'static LexicalSyntax {
        &SYNTAX
    }

    fn classify(&self, header: &[HTok<'_>], has_body: bool) -> HeaderKind {
        classify_c_like(header, has_body, &RULES)
    }
}

/// The `c` adapter.
pub fn adapter() -> LexicalAdapter<C> {
    LexicalAdapter::new(C)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover_text;

    const SOURCE: &str = r#"#include <check.h>
#define BLOCK(x) { if (x) { return; } }

struct point { int x; int y; };
static const int table[] = { 1, 2, 3 };
int prototype(int a);

/* @vtest.src-id SRC-C-ADD */
int add(int a, int b) { return a + b; }

static char *
copy_label(const char *label)
{
    if (label == NULL) { return "}"; }
    return strdup(label);
}

// @vtest.id TEST-C-ADD
// @vtest.covers VO-C-ADD
// @vtest.target src/calc.c::add
// @vtest.intent adding two numbers returns their sum
START_TEST(test_add)
{
    ck_assert_int_eq(add(1, 2), 3);
}
END_TEST
"#;

    /// @vtest.id TEST-ADAPTER-C-DISCOVERS-FUNCTIONS-AND-TESTS
    /// @vtest.covers VO-ADAPTER-C-REGISTERS-WITHOUT-CORE-CHANGES, VO-ADAPTER-LEXICAL-OPAQUE-LOCATOR-SYNTAX
    /// @vtest.target crates/vtest-adapter-lexical/src/c.rs::C::classify
    /// @vtest.intent verifies C function definitions (including multi-line headers) become Source Targets while prototypes, struct definitions, initializers, and macro definitions do not, and an annotated test-macro block is a Test construct
    #[test]
    fn discovers_c_functions_and_macro_tests() {
        let outcome = discover_text(&C, "src/calc.c", SOURCE, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        let sources = outcome
            .sources
            .iter()
            .map(|source| source.locator.value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            sources,
            vec![
                "src/calc.c::add",
                "src/calc.c::copy_label",
                "src/calc.c::START_TEST"
            ]
        );
        assert_eq!(
            outcome.sources[0].src_id.as_ref().map(|id| id.as_str()),
            Some("SRC-C-ADD")
        );
        assert_eq!(outcome.tests.len(), 1);
        let test = &outcome.tests[0];
        assert_eq!(test.id.as_str(), "TEST-C-ADD");
        assert_eq!(test.location.locator, "START_TEST");
        assert!(test.construct_text.starts_with("START_TEST(test_add)"));
        assert!(test
            .construct_text
            .ends_with("ck_assert_int_eq(add(1, 2), 3);\n}"));
    }
}
