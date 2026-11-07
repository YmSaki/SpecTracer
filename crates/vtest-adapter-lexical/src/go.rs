//! `go`: Go sources (`.go`).
//!
//! Functions are `func` declarations with a body. A method is qualified by
//! its receiver's type (`func (c *Calc) Add(...)` is `Calc::Add`); generic
//! type parameters are not part of the name. `go test`'s `func TestX(t
//! *testing.T)`, testify suites, Ginkgo specs (`It("...", func() { ... })`),
//! or any other style are discovered the same way, because a Test is
//! whatever follows its `@vtest.` comment.

use crate::{
    header::{matching_close, HTok, HeaderKind},
    lexer::{LexicalSyntax, StringSyntax, Structure, TokKind},
    LanguageProfile, LexicalAdapter,
};

pub const ADAPTER_ID: &str = "go";

const SYNTAX: LexicalSyntax = LexicalSyntax {
    line_comments: &["//"],
    line_comment_exclusions: &[],
    block_comments: &[("/*", "*/")],
    nested_block_comments: false,
    strings: &[
        StringSyntax::multi("`", "`", false),
        StringSyntax::single("\"", "\""),
        StringSyntax::single("'", "'"),
    ],
    preprocessor: false,
    cpp_raw_strings: false,
    word_suffix_chars: &[],
    newline_terminates: true,
    structure: Structure::Braces,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct Go;

impl LanguageProfile for Go {
    fn adapter_id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn languages(&self) -> &'static [&'static str] {
        &["go"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["go"]
    }

    fn excluded_dirs(&self) -> &'static [&'static str] {
        &["vendor", "testdata"]
    }

    fn syntax(&self) -> &'static LexicalSyntax {
        &SYNTAX
    }

    fn classify(&self, header: &[HTok<'_>], has_body: bool) -> HeaderKind {
        if !has_body || !header.first().is_some_and(|token| token.is_word("func")) {
            return HeaderKind::Other;
        }
        let mut index = 1;
        let mut receiver = None;
        if header.get(index).is_some_and(|token| token.is_open('(')) {
            let Some(close) = matching_close(header, index) else {
                return HeaderKind::Other;
            };
            receiver = receiver_type(&header[index + 1..close]);
            index = close + 1;
        }
        match header.get(index) {
            Some(token) if token.kind == TokKind::Word => {
                let mut name = receiver.into_iter().collect::<Vec<_>>();
                name.push(token.text.to_owned());
                HeaderKind::Function(name)
            }
            _ => HeaderKind::Other,
        }
    }
}

/// The receiver's type name: the last word outside its type-parameter
/// brackets (`c *Calc` and `l *List[T]` give `Calc` and `List`).
fn receiver_type(receiver: &[HTok<'_>]) -> Option<String> {
    let mut depth = 0usize;
    let mut last = None;
    for token in receiver {
        match token.kind {
            TokKind::Open(_) => depth += 1,
            TokKind::Close(_) => depth = depth.saturating_sub(1),
            TokKind::Word if depth == 0 => last = Some(token.text.to_owned()),
            _ => {}
        }
    }
    last
}

/// The `go` adapter.
pub fn adapter() -> LexicalAdapter<Go> {
    LexicalAdapter::new(Go)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover_text;

    const SOURCE: &str = r#"package calc

import "testing"

const raw = `{ not code`

type Calc struct{ base int }

// @vtest.src-id SRC-GO-ADD
func (c *Calc) Add(a, b int) int { return c.base + a + b }

func Max[T int | float64](a, b T) T {
	if a > b {
		return a
	}
	return b
}

func (l List[T]) Len() int { return len(l.items) }

var handler = func() {}

// @vtest.id TEST-GO-ADD
// @vtest.covers VO-GO-ADD
// @vtest.target calc/calc_test.go::Calc::Add
// @vtest.intent adding two numbers returns their sum
func TestAdd(t *testing.T) {
	if got := (&Calc{}).Add(1, 2); got != 3 {
		t.Fatalf("got %d", got)
	}
}
"#;

    /// @vtest.id TEST-ADAPTER-GO-DISCOVERS-FUNCS-AND-TESTS
    /// @vtest.covers VO-ADAPTER-GO-REGISTERS-WITHOUT-CORE-CHANGES, VO-ADAPTER-LEXICAL-OPAQUE-LOCATOR-SYNTAX
    /// @vtest.target crates/vtest-adapter-lexical/src/go.rs::Go::classify
    /// @vtest.intent verifies Go functions, generic functions, and methods qualified by their receiver type become Source Targets while function literals and types do not, and an annotated go test function is a Test construct
    #[test]
    fn discovers_go_funcs_and_tests() {
        let outcome = discover_text(&Go, "calc/calc_test.go", SOURCE, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        let sources = outcome
            .sources
            .iter()
            .map(|source| source.locator.value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            sources,
            vec![
                "calc/calc_test.go::Calc::Add",
                "calc/calc_test.go::Max",
                "calc/calc_test.go::List::Len",
                "calc/calc_test.go::TestAdd",
            ]
        );
        assert_eq!(
            outcome.sources[0].src_id.as_ref().map(|id| id.as_str()),
            Some("SRC-GO-ADD")
        );
        assert_eq!(outcome.tests.len(), 1);
        let test = &outcome.tests[0];
        assert_eq!(test.id.as_str(), "TEST-GO-ADD");
        assert_eq!(test.location.locator, "TestAdd");
        assert!(test
            .construct_text
            .starts_with("func TestAdd(t *testing.T) {"));
        assert!(test.construct_text.ends_with("got)\n\t}\n}"));
    }
}
