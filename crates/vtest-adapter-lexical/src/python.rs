//! `python`: Python sources (`.py`, `.pyi`).
//!
//! A file is read in three steps, all specific to Python:
//!
//! 1. [`lex`] splits it into comments, string literals (prefixed,
//!    triple-quoted, and f-strings whose `{...}` fields may hold quotes of
//!    their own), words, brackets, and operators.
//! 2. [`logical_lines`] joins them into Python's logical lines: a newline
//!    inside brackets or after a `\` does not end one. Each logical line
//!    has the indentation of its first physical line. A statement's block
//!    is every following logical line indented deeper than it (comment
//!    lines and text inside strings do not count), and `elif`, `else`,
//!    `except`, `finally`, and `case` lines at its own indentation
//!    continue it. A decorated definition starts at its first decorator.
//! 3. [`Reader::members`] walks the logical lines: `class` bodies qualify
//!    the `def` and `async def` definitions directly inside them. A `def`
//!    nested in a function body is not a Source Target.
//!
//! pytest functions and classes, `unittest.TestCase` methods, or any other
//! style are discovered the same way, because a Test is whatever follows
//! its `@vtest.` comment. Declarations are read from `#` comments, not from
//! docstrings.

use crate::{
    shared::{attached_comments, Attached, Span},
    Annotated, ExtractError, FileItems, Function, LanguageProfile, LexicalAdapter,
};

pub const ADAPTER_ID: &str = "python";

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
            ".mypy_cache",
            ".pytest_cache",
            "build",
            "dist",
        ]
    }

    fn extract(&self, source: &str) -> Result<FileItems, ExtractError> {
        let tokens = lex(source)?;
        let lines = logical_lines(source, &tokens)?;
        let reader = Reader {
            source,
            tokens: &tokens,
            lines: &lines,
        };
        let mut functions = Vec::new();
        reader.members(0, lines.len(), &mut Vec::new(), &mut functions);
        Ok(FileItems {
            functions,
            annotated: reader.annotated(),
        })
    }
}

/// The `python` adapter.
pub fn adapter() -> LexicalAdapter<Python> {
    LexicalAdapter::new(Python)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    Comment,
    Literal,
    Word,
    Open(char),
    Close(char),
    Op,
}

#[derive(Clone, Copy, Debug)]
struct Tok {
    kind: Kind,
    start: usize,
    end: usize,
}

fn is_word_char(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric() || !byte.is_ascii()
}

/// The length of a string prefix (`r`, `b`, `f`, `rb`, `Rf`, ...) at `i`
/// that is followed by a quote, and whether it makes an f-string.
fn string_prefix(bytes: &[u8], i: usize) -> Option<(usize, bool)> {
    let mut length = 0;
    while length < 2
        && bytes
            .get(i + length)
            .is_some_and(|b| b"rRbBuUfFtT".contains(b))
    {
        length += 1;
    }
    (1..=length).rev().chain([0]).find_map(|length| {
        matches!(bytes.get(i + length), Some(b'"' | b'\''))
            .then(|| {
                let prefix = &bytes[i..i + length];
                (
                    length,
                    prefix
                        .iter()
                        .any(|b| matches!(b, b'f' | b'F' | b't' | b'T')),
                )
            })
            .filter(|_| {
                length == 0
                    || !bytes
                        .get(i.wrapping_sub(1))
                        .copied()
                        .is_some_and(is_word_char)
            })
    })
}

/// Splits Python source into tokens. An unterminated string is an error.
fn lex(source: &str) -> Result<Vec<Tok>, ExtractError> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        let byte = bytes[i];
        let kind = if byte.is_ascii_whitespace()
            || (byte == b'\\' && matches!(bytes.get(i + 1), Some(b'\n' | b'\r')))
        {
            i += 1;
            continue;
        } else if byte == b'#' {
            i += source[i..].find('\n').unwrap_or(source.len() - i);
            Kind::Comment
        } else if let Some((prefix, formatted)) =
            (is_word_char(byte) || byte == b'"' || byte == b'\'')
                .then(|| string_prefix(bytes, i))
                .flatten()
        {
            i = string_end(bytes, i + prefix, formatted)
                .ok_or_else(|| ExtractError::new(start, "unterminated string"))?;
            Kind::Literal
        } else if byte.is_ascii_digit()
            || (byte == b'.' && bytes.get(i + 1).is_some_and(u8::is_ascii_digit))
        {
            i += 1;
            while i < bytes.len() && (is_word_char(bytes[i]) || bytes[i] == b'.') {
                i += 1;
            }
            Kind::Literal
        } else if is_word_char(byte) {
            while i < bytes.len() && is_word_char(bytes[i]) {
                i += 1;
            }
            Kind::Word
        } else {
            i += 1;
            match byte {
                b'(' | b'[' | b'{' => Kind::Open(byte as char),
                b')' | b']' | b'}' => Kind::Close(byte as char),
                _ => Kind::Op,
            }
        };
        tokens.push(Tok {
            kind,
            start,
            end: i,
        });
    }
    Ok(tokens)
}

/// One past the end of the string whose opening quote is at `i`. In an
/// f-string, `{...}` fields are read as code (`{{` is a literal brace).
fn string_end(bytes: &[u8], i: usize, formatted: bool) -> Option<usize> {
    let quote = bytes[i];
    let triple = bytes[i..].starts_with(&[quote; 3]);
    let mut j = i + if triple { 3 } else { 1 };
    while j < bytes.len() {
        match bytes[j] {
            b'\\' => j += 2,
            b'\n' if !triple => return None,
            byte if byte == quote && (!triple || bytes[j..].starts_with(&[quote; 3])) => {
                return Some(j + if triple { 3 } else { 1 })
            }
            b'{' if formatted && bytes.get(j + 1) == Some(&b'{') => j += 2,
            b'{' if formatted => j = field_end(bytes, j + 1)?,
            _ => j += 1,
        }
    }
    None
}

/// One past the `}` closing an f-string field whose code starts at `i`.
fn field_end(bytes: &[u8], mut i: usize) -> Option<usize> {
    let mut depth = 1usize;
    while i < bytes.len() {
        match bytes[i] {
            b'"' | b'\'' => i = string_end(bytes, i, false)?,
            b'(' | b'[' | b'{' => {
                depth += 1;
                i += 1;
            }
            b')' | b']' | b'}' => {
                depth -= 1;
                i += 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => i += 1,
        }
    }
    None
}

/// A logical line: its tokens (comments excluded) and indentation.
#[derive(Clone, Debug)]
struct Line {
    /// Indices into the token list, all non-comment.
    first: usize,
    last: usize,
    indent: usize,
}

/// Joins the tokens into logical lines. Unbalanced brackets are an error.
fn logical_lines(source: &str, tokens: &[Tok]) -> Result<Vec<Line>, ExtractError> {
    let mut lines: Vec<Line> = Vec::new();
    let mut stack: Vec<(usize, char)> = Vec::new();
    let mut previous: Option<usize> = None;
    for (index, token) in tokens.iter().enumerate() {
        if token.kind == Kind::Comment {
            continue;
        }
        let breaks = previous.is_none_or(|p| {
            stack.is_empty()
                && source[tokens[p].end..token.start]
                    .replace("\\\r\n", "")
                    .replace("\\\n", "")
                    .contains('\n')
        });
        if breaks {
            let line_start = source[..token.start].rfind('\n').map_or(0, |n| n + 1);
            let indent = source[line_start..token.start]
                .bytes()
                .fold(0, |column, b| {
                    if b == b'\t' {
                        column / 8 * 8 + 8
                    } else {
                        column + 1
                    }
                });
            lines.push(Line {
                first: index,
                last: index,
                indent,
            });
        } else if let Some(line) = lines.last_mut() {
            line.last = index;
        }
        match token.kind {
            Kind::Open(open) => stack.push((index, open)),
            Kind::Close(close) => {
                let (_, open) = stack
                    .pop()
                    .ok_or_else(|| ExtractError::new(token.start, "unmatched closing bracket"))?;
                if !matches!((open, close), ('(', ')') | ('[', ']') | ('{', '}')) {
                    return Err(ExtractError::new(token.start, "mismatched brackets"));
                }
            }
            _ => {}
        }
        previous = Some(index);
    }
    match stack.pop() {
        Some((index, _)) => Err(ExtractError::new(tokens[index].start, "unclosed bracket")),
        None => Ok(lines),
    }
}

/// Lines that continue the compound statement before them at the same
/// indentation.
/// Compound statements whose suites stay in the enclosing scope.
const COMPOUND: &[&str] = &[
    "if", "elif", "else", "try", "except", "finally", "with", "for", "while", "match", "case",
];

const CLAUSES: &[&str] = &["elif", "else", "except", "finally", "case"];

struct Reader<'a> {
    source: &'a str,
    tokens: &'a [Tok],
    lines: &'a [Line],
}

impl Reader<'_> {
    fn text(&self, index: usize) -> &str {
        &self.source[self.tokens[index].start..self.tokens[index].end]
    }

    /// The text of the `n`th token of logical line `line`, if any.
    fn word(&self, line: usize, n: usize) -> Option<&str> {
        let index = self.lines[line].first + n;
        (index <= self.lines[line].last).then(|| self.text(index))
    }

    fn starts_with(&self, line: usize, text: &str) -> bool {
        self.word(line, 0) == Some(text)
    }

    /// One past the last logical line of the statement at `line`: its
    /// decorators, its block, and the clauses that continue it.
    fn statement_end(&self, line: usize) -> usize {
        let indent = self.lines[line].indent;
        let mut index = line;
        while self.starts_with(index, "@")
            && index + 1 < self.lines.len()
            && self.lines[index + 1].indent == indent
        {
            index += 1;
        }
        loop {
            index += 1;
            while index < self.lines.len() && self.lines[index].indent > indent {
                index += 1;
            }
            let continues = index < self.lines.len()
                && self.lines[index].indent == indent
                && CLAUSES.iter().any(|clause| self.starts_with(index, clause));
            if !continues {
                return index;
            }
        }
    }

    /// The byte range of logical lines `first..end`.
    fn range(&self, first: usize, end: usize) -> (usize, usize) {
        (
            self.tokens[self.lines[first].first].start,
            self.tokens[self.lines[end - 1].last].end,
        )
    }

    /// The first logical line after the decorators of the statement at
    /// `line`.
    fn after_decorators(&self, line: usize, end: usize) -> usize {
        let mut index = line;
        while index + 1 < end && self.starts_with(index, "@") {
            index += 1;
        }
        index
    }

    /// Collects the `def` definitions among the statements in logical lines
    /// `low..high`, qualified by `scope`.
    fn members(
        &self,
        low: usize,
        high: usize,
        scope: &mut Vec<String>,
        output: &mut Vec<Function>,
    ) {
        let mut line = low;
        while line < high {
            let end = self.statement_end(line).min(high);
            let header = self.after_decorators(line, end);
            let offset = usize::from(self.starts_with(header, "async"));
            match self.word(header, offset) {
                Some("def") => {
                    if let Some(name) = self.word(header, offset + 1) {
                        let mut qualified = scope.clone();
                        qualified.push(name.to_owned());
                        let (start, end_byte) = self.range(line, end);
                        output.push(Function {
                            name: qualified,
                            start,
                            end: end_byte,
                        });
                    }
                }
                Some("class") if header + 1 < end => {
                    if let Some(name) = self.word(header, 1) {
                        scope.push(name.to_owned());
                        self.members(header + 1, end, scope, output);
                        scope.pop();
                    }
                }
                Some(word) if COMPOUND.contains(&word) => {
                    self.suites(header, end, scope, output);
                }
                _ => {}
            }
            line = end;
        }
    }

    /// Collects the definitions in the suites of the compound statement
    /// `lines[header..end]` (`if`, `try`, `with`, ...), in the same scope:
    /// a function defined under `if sys.platform == ...:` is still a
    /// module (or class) member. The conditions are not evaluated, so every
    /// branch counts. Clause headers (`else:`, `except E:`) at the
    /// statement's own indentation separate the suites.
    fn suites(
        &self,
        header: usize,
        end: usize,
        scope: &mut Vec<String>,
        output: &mut Vec<Function>,
    ) {
        let indent = self.lines[header].indent;
        let mut line = header + 1;
        while line < end {
            let stop = (line..end)
                .find(|&i| self.lines[i].indent <= indent)
                .unwrap_or(end);
            if stop > line {
                self.members(line, stop, scope, output);
            }
            line = stop + 1;
        }
    }

    /// Every block of comment lines that is followed, on the next line, by
    /// a statement.
    fn annotated(&self) -> Vec<Annotated> {
        let spans: Vec<Span> = self
            .tokens
            .iter()
            .map(|token| Span {
                start: token.start,
                end: token.end,
                comment: token.kind == Kind::Comment,
            })
            .collect();
        attached_comments(self.source, &spans)
            .into_iter()
            .map(|Attached { first, last, next }| {
                let construct = match self.lines.binary_search_by_key(&next, |line| line.first) {
                    Ok(line) => Ok(self.range(line, self.statement_end(line))),
                    Err(_) => {
                        Err("the comment is inside brackets, not before a statement".to_owned())
                    }
                };
                Annotated {
                    comment_start: self.tokens[first].start,
                    lines: (first..=last)
                        .map(|i| self.text(i)[1..].to_owned())
                        .collect(),
                    construct,
                }
            })
            .collect()
    }
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

    fn locators(outcome: &vtest_adapter_api::DiscoveryOutcome) -> Vec<&str> {
        outcome
            .sources
            .iter()
            .map(|source| source.locator.value.as_str())
            .collect()
    }

    /// @vtest.id TEST-ADAPTER-PYTHON-DISCOVERS-DEFS-AND-TESTS
    /// @vtest.covers VO-ADAPTER-PYTHON-REGISTERS-WITHOUT-CORE-CHANGES, VO-ADAPTER-LEXICAL-OPAQUE-LOCATOR-SYNTAX
    /// @vtest.target crates/vtest-adapter-lexical/src/python.rs::Python::extract
    /// @vtest.intent verifies Python defs, async defs, decorated and one-line defs become class-qualified Source Targets while nested helper defs do not, and a decorated pytest function is a Test construct spanning its decorators and indented body
    #[test]
    fn discovers_python_defs_and_tests() {
        let outcome = discover_text(&Python, "src/calc.py", SOURCE, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(
            locators(&outcome),
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

    /// @vtest.id TEST-ADAPTER-PYTHON-DEDENTED-TEXT-IS-NOT-THE-END
    /// @vtest.covers VO-ADAPTER-PYTHON-REGISTERS-WITHOUT-CORE-CHANGES
    /// @vtest.target crates/vtest-adapter-lexical/src/python.rs::logical_lines
    /// @vtest.intent verifies that dedented text inside a triple-quoted string, a bracketed expression, or a backslash continuation does not end a function, so a Test construct runs to the end of its body and an edit to only the body's tail changes its text, and that f-string fields may hold quotes and braces
    #[test]
    fn dedented_text_is_not_the_end() {
        let source = |tail: &str| {
            format!(
                "# @vtest.id TEST-PY-TEXT\n# @vtest.covers VO-PY-ADD\n# @vtest.intent the whole body is the construct\ndef test_text():\n    text = \"\"\"\nnot indented {{\n\"\"\"\n    values = [\n1,\n    ]\n    total = 1 + \\\n2\n    label = f\"{{values[\"}}\"]}} {{{{\"\n    {tail}\n\n\ndef after():\n    pass\n"
            )
        };
        let outcome = discover_text(&Python, "tests/test_text.py", &source("one()"), "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(
            locators(&outcome),
            vec!["tests/test_text.py::test_text", "tests/test_text.py::after"]
        );
        let test = &outcome.tests[0];
        assert!(test.construct_text.starts_with("def test_text():"));
        assert!(test.construct_text.ends_with("one()"));
        let edited = discover_text(&Python, "tests/test_text.py", &source("two()"), "demo");
        assert_ne!(edited.tests[0].construct_text, test.construct_text);
    }

    /// @vtest.id TEST-ADAPTER-PYTHON-CLAUSES-AND-UNREADABLE-FILES
    /// @vtest.covers VO-ADAPTER-PYTHON-REGISTERS-WITHOUT-CORE-CHANGES
    /// @vtest.target crates/vtest-adapter-lexical/src/python.rs::Reader::statement_end
    /// @vtest.intent verifies that an annotated compound statement includes its else and except clauses, that a comment inside brackets cannot be a Test construct, and that an unterminated string makes the file unreadable
    #[test]
    fn clauses_and_unreadable_files() {
        let source = "def test_all():\n    # @vtest.id TEST-PY-TRY\n    # @vtest.covers VO-PY-ADD\n    # @vtest.intent a try statement is a construct\n    try:\n        run()\n    except ValueError:\n        handle()\n    else:\n        done()\n    after()\n";
        let outcome = discover_text(&Python, "tests/test_all.py", source, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(outcome.tests.len(), 1);
        assert!(outcome.tests[0].construct_text.starts_with("try:"));
        assert!(outcome.tests[0].construct_text.ends_with("done()"));

        let inside = "values = [\n    # @vtest.id TEST-PY-INSIDE\n    # @vtest.covers VO-PY-ADD\n    # @vtest.intent not a statement\n    1,\n]\n";
        let outcome = discover_text(&Python, "tests/test_inside.py", inside, "demo");
        assert!(outcome.tests.is_empty());
        assert_eq!(outcome.missing_tests.len(), 1);

        let broken = discover_text(&Python, "src/broken.py", "x = \"\"\"open\n", "demo");
        assert_eq!(broken.diagnostics.len(), 1);
        assert_eq!(broken.diagnostics[0].code.as_str(), "E-SCAN-001");
    }
    /// @vtest.id TEST-ADAPTER-PYTHON-DEFINITIONS-IN-COMPOUND-STATEMENTS
    /// @vtest.covers VO-ADAPTER-LEXICAL-NORMALIZES-DECLARATION
    /// @vtest.target crates/vtest-adapter-lexical/src/python.rs::Reader::suites
    /// @vtest.intent verifies functions defined in every branch of module- or class-level if, try, with, and match statements are Source Targets in the enclosing scope with their SRC IDs, while functions defined inside a function body are not
    #[test]
    fn definitions_in_compound_statements() {
        let source = "import sys\nif sys.platform == \"win32\":\n    # @vtest.src-id SRC-PY-WIN\n    def f():\n        return 1\nelif True:\n    def f():\n        return 2\nelse:\n    def g():\n        def inner():\n            pass\n        return 3\ntry:\n    from fast import h\nexcept ImportError:\n    def h():\n        return 4\nclass C:\n    if True:\n        def m(self):\n            return 5\n    with open(\"x\") as handle:\n        def n(self):\n            return 6\nmatch sys.argv:\n    case [_]:\n        def k():\n            return 7\n";
        let outcome = discover_text(&Python, "src/m.py", source, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        let sources = outcome
            .sources
            .iter()
            .map(|source| source.locator.value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            sources,
            vec![
                "src/m.py::f",
                "src/m.py::f",
                "src/m.py::g",
                "src/m.py::h",
                "src/m.py::C::m",
                "src/m.py::C::n",
                "src/m.py::k",
            ]
        );
        assert_eq!(
            outcome.sources[0].src_id.as_ref().map(|id| id.as_str()),
            Some("SRC-PY-WIN")
        );
        assert_eq!(outcome.sources[1].src_id, None);
    }
}
