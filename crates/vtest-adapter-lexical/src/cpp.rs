//! `cpp`: C++ sources and headers (`.cc`, `.cpp`, `.cxx`, `.c++`, `.h`,
//! `.hh`, `.hpp`, `.hxx`, `.h++`, `.ipp`, `.inl`, `.tpp`).
//!
//! Functions are definitions with a body, including member functions defined
//! out of line (`Calc::add`), constructors with initializer lists,
//! operators, and templates. `namespace`, `class`, `struct`, `union`, and
//! `enum` bodies qualify them. Test-framework macros with a body
//! (GoogleTest's `TEST(Suite, Name) { ... }`, Catch2's `TEST_CASE("...")
//! { ... }`, doctest, Boost.Test) are functions named after the macro, and a
//! `@vtest.` comment above one makes it a Test construct.

use crate::{
    header::{classify_c_like, skip_angle_brackets, AttributeForms, CLikeRules, HTok, HeaderKind},
    lexer::{LexicalSyntax, StringSyntax, Structure},
    LanguageProfile, LexicalAdapter,
};

pub const ADAPTER_ID: &str = "cpp";

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
    cpp_raw_strings: true,
    word_suffix_chars: &[],
    newline_terminates: false,
    structure: Structure::Braces,
};

const RULES: CLikeRules = CLikeRules {
    attributes: AttributeForms {
        at_sign: false,
        hash_bracket: false,
        bracket: true,
    },
    scope_keywords: &["namespace", "class", "struct", "union", "enum"],
    scope_rest_keywords: &[],
    arrow_bodies: false,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct Cpp;

impl LanguageProfile for Cpp {
    fn adapter_id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn languages(&self) -> &'static [&'static str] {
        &["cpp"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &[
            "cc", "cpp", "cxx", "c++", "h", "hh", "hpp", "hxx", "h++", "ipp", "inl", "tpp",
        ]
    }

    fn excluded_dirs(&self) -> &'static [&'static str] {
        &["build", "cmake-build-debug", "cmake-build-release"]
    }

    fn syntax(&self) -> &'static LexicalSyntax {
        &SYNTAX
    }

    /// A `template <...>` prefix (possibly repeated) is skipped first, so
    /// that `template <class T> T max(T, T)` is not mistaken for a class.
    fn classify(&self, header: &[HTok<'_>], has_body: bool) -> HeaderKind {
        let mut start = 0;
        while header
            .get(start)
            .is_some_and(|token| token.is_word("template"))
        {
            let after = skip_angle_brackets(header, start + 1);
            if after == start + 1 {
                break;
            }
            start = after;
        }
        classify_c_like(&header[start..], has_body, &RULES)
    }
}

/// The `cpp` adapter.
pub fn adapter() -> LexicalAdapter<Cpp> {
    LexicalAdapter::new(Cpp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover_text;

    const SOURCE: &str = r#"#include <gtest/gtest.h>

namespace demo::math {

class Calc {
public:
    Calc() : base_(0) {}
    // @vtest.src-id SRC-CPP-ADD
    int add(int a, int b) const { return a + b; }
    bool operator==(const Calc& other) const { return base_ == other.base_; }
    int declared_only(int a);
private:
    int base_;
};

int Calc::declared_only(int a) { return a; }

template <class T>
T max_of(T a, T b) {
    auto pick = [](T x, T y) { return x > y ? x : y; };
    return pick(a, b);
}

const char* kRaw = R"json({"not": "code"})json";

}  // namespace demo::math

extern "C" {
int c_entry(void) { return 0; }
}

// @vtest.id TEST-CPP-ADD
// @vtest.covers VO-CPP-ADD
// @vtest.target src/calc.cpp::demo::math::Calc::add
// @vtest.intent adding two numbers returns their sum
TEST(CalcTest, Adds) {
    EXPECT_EQ(demo::math::Calc().add(1, 2), 3);
}

TEST(CalcTest, Other) {}
"#;

    /// @vtest.id TEST-ADAPTER-CPP-DISCOVERS-FUNCTIONS-AND-TESTS
    /// @vtest.covers VO-ADAPTER-CPP-REGISTERS-WITHOUT-CORE-CHANGES, VO-ADAPTER-LEXICAL-OPAQUE-LOCATOR-SYNTAX
    /// @vtest.target crates/vtest-adapter-lexical/src/cpp.rs::Cpp::classify
    /// @vtest.intent verifies C++ member, out-of-line, operator, template, and extern "C" function definitions become namespace- and class-qualified Source Targets while declarations, lambdas, and raw strings do not, and an annotated GoogleTest macro block is a Test construct
    #[test]
    fn discovers_cpp_functions_and_macro_tests() {
        let outcome = discover_text(&Cpp, "src/calc.cpp", SOURCE, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        let sources = outcome
            .sources
            .iter()
            .map(|source| source.locator.value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            sources,
            vec![
                "src/calc.cpp::demo::math::Calc::Calc",
                "src/calc.cpp::demo::math::Calc::add",
                "src/calc.cpp::demo::math::Calc::operator==",
                "src/calc.cpp::demo::math::Calc::declared_only",
                "src/calc.cpp::demo::math::max_of",
                "src/calc.cpp::c_entry",
                "src/calc.cpp::TEST",
                "src/calc.cpp::TEST",
            ]
        );
        assert_eq!(
            outcome.sources[1].src_id.as_ref().map(|id| id.as_str()),
            Some("SRC-CPP-ADD")
        );
        assert_eq!(outcome.tests.len(), 1);
        let test = &outcome.tests[0];
        assert_eq!(test.id.as_str(), "TEST-CPP-ADD");
        assert_eq!(test.location.locator, "TEST#1");
        assert!(test.construct_text.starts_with("TEST(CalcTest, Adds) {"));
        assert!(test.construct_text.ends_with("3);\n}"));
    }
}
