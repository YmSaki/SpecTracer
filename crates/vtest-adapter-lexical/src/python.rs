//! `python`: Python sources (`.py`, `.pyi`).
//!
//! Functions are `def` and `async def` definitions; a decorated definition
//! starts at its first decorator. `class` bodies qualify the methods inside
//! them. pytest functions and classes, `unittest.TestCase` methods, doctest
//! wrappers, or any other style are discovered the same way, because a Test
//! is whatever follows its `@vtest.` comment. Declarations are read from `#`
//! comments, not from docstrings.

use crate::{
    header::{HTok, HeaderKind},
    lexer::{LexicalSyntax, StringSyntax, Structure, TokKind},
    LanguageProfile, LexicalAdapter,
};

pub const ADAPTER_ID: &str = "python";

const SYNTAX: LexicalSyntax = LexicalSyntax {
    line_comments: &["#"],
    line_comment_exclusions: &[],
    block_comments: &[],
    nested_block_comments: false,
    strings: &[
        StringSyntax::multi("\"\"\"", "\"\"\"", true),
        StringSyntax::multi("'''", "'''", true),
        StringSyntax::single("\"", "\""),
        StringSyntax::single("'", "'"),
    ],
    preprocessor: false,
    cpp_raw_strings: false,
    word_suffix_chars: &[],
    newline_terminates: true,
    structure: Structure::Indentation,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct Python;

impl LanguageProfile for Python {
    fn adapter_id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn languages(&self) -> &'static [&'static str] {
        &["python"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["py", "pyi"]
    }

    fn excluded_dirs(&self) -> &'static [&'static str] {
        &[
            "__pycache__",
            ".venv",
            "venv",
            ".tox",
            ".nox",
            "site-packages",
            "build",
            "dist",
            ".eggs",
            ".mypy_cache",
            ".pytest_cache",
        ]
    }

    fn syntax(&self) -> &'static LexicalSyntax {
        &SYNTAX
    }

    /// `header` is one logical line (decorators already skipped).
    fn classify(&self, header: &[HTok<'_>], _has_body: bool) -> HeaderKind {
        let mut index = 0;
        if header.first().is_some_and(|token| token.is_word("async")) {
            index = 1;
        }
        let keyword = header.get(index);
        let name = header.get(index + 1);
        match (keyword, name) {
            (Some(keyword), Some(name)) if name.kind == TokKind::Word => {
                if keyword.is_word("def") {
                    HeaderKind::Function(vec![name.text.to_owned()])
                } else if keyword.is_word("class") && index == 0 {
                    HeaderKind::Scope(vec![name.text.to_owned()])
                } else {
                    HeaderKind::Other
                }
            }
            _ => HeaderKind::Other,
        }
    }
}

/// The `python` adapter.
pub fn adapter() -> LexicalAdapter<Python> {
    LexicalAdapter::new(Python)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover_text;

    const SOURCE: &str = r#""""Module docstring with a stray { brace."""
import pytest


class Calc:
    LABEL = "{ not code"

    # @vtest.src-id SRC-PY-ADD
    def add(self, a: int, b: int) -> int:
        return a + b

    @staticmethod
    def parse(text: str) -> "Calc":
        values = [
            int(part)
            for part in text.split(",")
        ]
        return Calc()

    class Inner:
        async def fetch(self):
            def helper():
                return 1
            return helper()


def free(x): return x * 2


# @vtest.id TEST-PY-ADD
# @vtest.covers VO-PY-ADD
# @vtest.target src/calc.py::Calc::add
# @vtest.intent adding two numbers returns their sum
@pytest.mark.parametrize("a,b,expected", [(1, 2, 3)])
def test_add(a, b, expected):
    # a comment inside the body
    assert Calc().add(a, b) == expected

    assert True


x = 1
"#;

    /// @vtest.id TEST-ADAPTER-PYTHON-DISCOVERS-DEFS-AND-TESTS
    /// @vtest.covers VO-ADAPTER-PYTHON-REGISTERS-WITHOUT-CORE-CHANGES, VO-ADAPTER-LEXICAL-OPAQUE-LOCATOR-SYNTAX
    /// @vtest.target crates/vtest-adapter-lexical/src/python.rs::Python::classify
    /// @vtest.intent verifies Python defs, async defs, decorated and one-line defs become class-qualified Source Targets while nested helper defs do not, and a decorated pytest function is a Test construct spanning its decorators and indented body
    #[test]
    fn discovers_python_defs_and_tests() {
        let outcome = discover_text(&Python, "src/calc.py", SOURCE, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        let sources = outcome
            .sources
            .iter()
            .map(|source| source.locator.value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            sources,
            vec![
                "src/calc.py::Calc::add",
                "src/calc.py::Calc::parse",
                "src/calc.py::Calc::Inner::fetch",
                "src/calc.py::free",
                "src/calc.py::test_add",
            ]
        );
        assert_eq!(
            outcome.sources[0].src_id.as_ref().map(|id| id.as_str()),
            Some("SRC-PY-ADD")
        );
        assert!(outcome.sources[1]
            .construct_text
            .starts_with("@staticmethod\n    def parse"));
        assert!(outcome.sources[1].construct_text.ends_with("return Calc()"));
        assert_eq!(
            outcome.sources[3].construct_text,
            "def free(x): return x * 2"
        );
        assert_eq!(outcome.tests.len(), 1);
        let test = &outcome.tests[0];
        assert_eq!(test.id.as_str(), "TEST-PY-ADD");
        assert_eq!(test.location.locator, "test_add");
        assert!(test.construct_text.starts_with("@pytest.mark.parametrize"));
        assert!(test.construct_text.ends_with("assert True"));
    }
}
