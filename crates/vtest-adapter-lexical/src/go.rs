//! `go`: Go sources (`.go`).
//!
//! A file is read in three steps, all specific to Go:
//!
//! 1. [`lex`] splits it into comments, literals (interpreted and raw
//!    strings, runes, numbers), words, brackets, `;`, and operators.
//! 2. [`Reader::statement_end`] finds where a declaration or statement
//!    ends, using Go's own rule: at a `;`, or at a newline that follows an
//!    identifier, a literal, `return`, `break`, `continue`, `fallthrough`,
//!    `++`, `--`, `)`, `]`, or `}` (the places where Go inserts a
//!    semicolon), counted at the statement's own bracket depth. Brackets are
//!    skipped whole, so a newline inside a body, a parameter list, or a
//!    `struct { ... }` type does not end anything.
//! 3. [`Reader::functions`] reads the top-level `func` declarations. A
//!    function's body is its last `{}` group, unless that group belongs to
//!    a `struct` or `interface` result type (`func f() struct{}` has no
//!    body). A method is qualified by its receiver's type name.
//!
//! A comment block attached to a declaration or statement makes it a Test
//! (or declares its SRC ID).

use crate::{
    shared::{attached_comments, pair_brackets, Attached, Span},
    Annotated, ExtractError, FileItems, Function, LanguageProfile, LexicalAdapter,
};

pub const ADAPTER_ID: &str = "go";

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

    fn extract(&self, source: &str) -> Result<FileItems, ExtractError> {
        let tokens = lex(source)?;
        let partners = pair_brackets(tokens.iter().map(|token| match token.kind {
            Kind::Open(c) | Kind::Close(c) => (token.start, Some(c)),
            _ => (token.start, None),
        }))?;
        let reader = Reader {
            source,
            tokens: &tokens,
            partners: &partners,
        };
        Ok(FileItems {
            functions: reader.functions(),
            annotated: reader.annotated(),
        })
    }
}

/// The `go` adapter.
pub fn adapter() -> LexicalAdapter<Go> {
    LexicalAdapter::new(Go)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    Comment,
    Literal,
    Word,
    Open(char),
    Close(char),
    Semi,
    /// `++` or `--`, after which Go inserts a semicolon at a newline.
    Step,
    Punct(char),
}

#[derive(Clone, Copy, Debug)]
struct Tok {
    kind: Kind,
    start: usize,
    end: usize,
}

/// Splits Go source into tokens. An unterminated comment or literal is an
/// error.
fn lex(source: &str) -> Result<Vec<Tok>, ExtractError> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        let rest = &source[i..];
        let byte = bytes[i];
        let kind = if byte.is_ascii_whitespace() {
            i += 1;
            continue;
        } else if rest.starts_with("//") {
            i += rest.find('\n').unwrap_or(rest.len());
            Kind::Comment
        } else if let Some(body) = rest.strip_prefix("/*") {
            let close = body
                .find("*/")
                .ok_or_else(|| ExtractError::new(start, "unterminated comment"))?;
            i += close + 4;
            Kind::Comment
        } else if byte == b'`' {
            let close = rest[1..]
                .find('`')
                .ok_or_else(|| ExtractError::new(start, "unterminated raw string"))?;
            i += close + 2;
            Kind::Literal
        } else if byte == b'"' || byte == b'\'' {
            i = quoted_end(bytes, i + 1, byte)
                .ok_or_else(|| ExtractError::new(start, "unterminated literal"))?;
            Kind::Literal
        } else if byte.is_ascii_digit()
            || (byte == b'.' && bytes.get(i + 1).is_some_and(u8::is_ascii_digit))
        {
            i += 1;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'_' | b'.'))
            {
                i += 1;
            }
            Kind::Literal
        } else if byte == b'_' || byte.is_ascii_alphabetic() || !byte.is_ascii() {
            while i < bytes.len()
                && (bytes[i] == b'_' || bytes[i].is_ascii_alphanumeric() || !bytes[i].is_ascii())
            {
                i += 1;
            }
            Kind::Word
        } else if rest.starts_with("++") || rest.starts_with("--") {
            i += 2;
            Kind::Step
        } else {
            i += 1;
            match byte {
                b'(' | b'[' | b'{' => Kind::Open(byte as char),
                b')' | b']' | b'}' => Kind::Close(byte as char),
                b';' => Kind::Semi,
                _ => Kind::Punct(byte as char),
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

/// One past the closing `quote` of a string or rune whose contents start at
/// `i`. Interpreted strings and runes cannot span lines.
fn quoted_end(bytes: &[u8], mut i: usize, quote: u8) -> Option<usize> {
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'\n' => return None,
            byte if byte == quote => return Some(i + 1),
            _ => i += 1,
        }
    }
    None
}

struct Reader<'a> {
    source: &'a str,
    tokens: &'a [Tok],
    partners: &'a [Option<usize>],
}

impl Reader<'_> {
    fn text(&self, index: usize) -> &str {
        &self.source[self.tokens[index].start..self.tokens[index].end]
    }

    fn kind(&self, index: usize) -> Option<Kind> {
        self.tokens.get(index).map(|token| token.kind)
    }

    fn partner(&self, index: usize) -> usize {
        self.partners[index].expect("brackets are balanced")
    }

    fn next_significant(&self, from: usize) -> Option<usize> {
        (from..self.tokens.len()).find(|&i| self.tokens[i].kind != Kind::Comment)
    }

    fn line_breaks(&self, before: usize, after: usize) -> usize {
        self.source[self.tokens[before].end..self.tokens[after].start]
            .matches('\n')
            .count()
    }

    /// Whether Go inserts a semicolon at a newline after `tokens[index]`.
    fn ends_line(&self, index: usize) -> bool {
        match self.tokens[index].kind {
            Kind::Literal | Kind::Step | Kind::Close(_) => true,
            Kind::Word => {
                !KEYWORDS.contains(&self.text(index))
                    || matches!(
                        self.text(index),
                        "break" | "continue" | "fallthrough" | "return"
                    )
            }
            _ => false,
        }
    }

    /// One past the last token of the declaration or statement starting at
    /// `start` (see the module documentation). A `}` or `)` closing the
    /// enclosing block also ends it.
    fn statement_end(&self, start: usize) -> usize {
        let mut index = start;
        while index < self.tokens.len() {
            let last = match self.tokens[index].kind {
                Kind::Comment => {
                    index += 1;
                    continue;
                }
                Kind::Close(_) => return index,
                Kind::Semi => return index + 1,
                Kind::Open(_) => self.partner(index),
                _ => index,
            };
            match self.next_significant(last + 1) {
                None => return last + 1,
                Some(next) if self.line_breaks(last, next) > 0 && self.ends_line(last) => {
                    return last + 1
                }
                Some(next) => index = next,
            }
        }
        index
    }

    /// The top-level `func` declarations that have a body.
    fn functions(&self) -> Vec<Function> {
        let mut output = Vec::new();
        let mut index = 0;
        while let Some(start) = self.next_significant(index) {
            let end = self.statement_end(start).max(start + 1);
            if self.kind(start) == Some(Kind::Word) && self.text(start) == "func" {
                // An explicit `;` after the body (`func f() {};`) is not part
                // of the function.
                let mut last = end - 1;
                while last > start && self.kind(last) == Some(Kind::Semi) {
                    last -= 1;
                }
                if let Some(name) = self.function_name(start, last + 1) {
                    output.push(Function {
                        name,
                        start: self.tokens[start].start,
                        end: self.tokens[last].end,
                    });
                }
            }
            index = end;
        }
        output
    }

    /// The name of the `func` declaration `tokens[start..end]`, qualified by
    /// its receiver type, if it has a body.
    fn function_name(&self, start: usize, end: usize) -> Option<Vec<String>> {
        let last = end - 1;
        if self.kind(last) != Some(Kind::Close('}')) {
            return None;
        }
        let body = self.partner(last);
        let before_body = body.checked_sub(1)?;
        if self.kind(before_body) == Some(Kind::Word)
            && matches!(self.text(before_body), "struct" | "interface")
        {
            return None;
        }
        let mut name = start + 1;
        let mut segments = Vec::new();
        if self.kind(name) == Some(Kind::Open('(')) {
            let close = self.partner(name);
            segments.push(self.receiver_type(name, close)?);
            name = close + 1;
        }
        if self.kind(name) != Some(Kind::Word) || name >= body {
            return None;
        }
        segments.push(self.text(name).to_owned());
        Some(segments)
    }

    /// The type name in the receiver `(c *Calc)`, `(l List[T])`, or
    /// `(Calc)`: its last word outside type arguments.
    fn receiver_type(&self, open: usize, close: usize) -> Option<String> {
        let mut index = open + 1;
        let mut found = None;
        while index < close {
            match self.tokens[index].kind {
                Kind::Word => found = Some(index),
                Kind::Open(_) => index = self.partner(index),
                _ => {}
            }
            index += 1;
        }
        found.map(|index| self.text(index).to_owned())
    }

    /// Every comment block that starts a line and is followed, on the next
    /// line, by a declaration or statement.
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
        let mut output = Vec::new();
        for Attached { first, last, next } in attached_comments(self.source, &spans) {
            if matches!(self.tokens[next].kind, Kind::Close(_) | Kind::Semi) {
                continue;
            }
            let end = self.statement_end(next);
            output.push(Annotated {
                comment_start: self.tokens[first].start,
                lines: (first..=last)
                    .flat_map(|i| comment_lines(self.text(i)))
                    .collect(),
                construct: Ok((self.tokens[next].start, self.tokens[end - 1].end)),
            });
        }
        output
    }
}

/// Go's keywords. Only `break`, `continue`, `fallthrough`, and `return` end
/// a line.
const KEYWORDS: &[&str] = &[
    "break",
    "case",
    "chan",
    "const",
    "continue",
    "default",
    "defer",
    "else",
    "fallthrough",
    "for",
    "func",
    "go",
    "goto",
    "if",
    "import",
    "interface",
    "map",
    "package",
    "range",
    "return",
    "select",
    "struct",
    "switch",
    "type",
    "var",
];

/// The lines of a `//` or `/* */` comment with the comment markers removed.
fn comment_lines(comment: &str) -> Vec<String> {
    if let Some(line) = comment.strip_prefix("//") {
        return vec![line.to_owned()];
    }
    let inner = comment
        .strip_prefix("/*")
        .and_then(|rest| rest.strip_suffix("*/"))
        .unwrap_or(comment);
    inner
        .lines()
        .map(|line| {
            let line = line.trim_start();
            line.strip_prefix('*').unwrap_or(line).to_owned()
        })
        .collect()
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

func assembly(x int) int

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
    /// @vtest.target crates/vtest-adapter-lexical/src/go.rs::Go::extract
    /// @vtest.intent verifies Go functions, generic functions, and methods qualified by their receiver type become Source Targets while function literals, bodyless declarations, and types do not, and an annotated go test function is a Test construct
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

    /// @vtest.id TEST-ADAPTER-GO-STRUCT-RESULT-IS-NOT-THE-BODY
    /// @vtest.covers VO-ADAPTER-GO-REGISTERS-WITHOUT-CORE-CHANGES
    /// @vtest.target crates/vtest-adapter-lexical/src/go.rs::Reader::statement_end
    /// @vtest.intent verifies that the braces of a struct or interface result type are not taken for the function body, so a Test construct runs to the end of the body and an edit to only the body's tail changes its text, and that a function whose last braces are a struct result type has no body
    #[test]
    fn struct_result_braces_are_not_the_body() {
        let source = |tail: &str| {
            format!(
                "package calc\n\n// @vtest.id TEST-GO-STRUCT\n// @vtest.covers VO-GO-ADD\n// @vtest.intent the whole body is the construct\nfunc TestPair(t *testing.T) struct {{\n\tX int\n}} {{\n\tmarker()\n\t{tail}\n\treturn struct{{ X int }}{{1}}\n}}\n\nfunc declared() struct{{}}\n"
            )
        };
        let outcome = discover_text(&Go, "calc/pair_test.go", &source("one()"), "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert!(outcome.missing_tests.is_empty());
        let locators = outcome
            .sources
            .iter()
            .map(|source| source.locator.value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(locators, vec!["calc/pair_test.go::TestPair"]);
        let test = &outcome.tests[0];
        assert!(test.construct_text.starts_with("func TestPair("));
        assert!(test
            .construct_text
            .ends_with("one()\n\treturn struct{ X int }{1}\n}"));
        let edited = discover_text(&Go, "calc/pair_test.go", &source("two()"), "demo");
        assert_ne!(edited.tests[0].construct_text, test.construct_text);
    }

    /// @vtest.id TEST-ADAPTER-GO-STATEMENT-CONSTRUCTS
    /// @vtest.covers VO-ADAPTER-GO-REGISTERS-WITHOUT-CORE-CHANGES
    /// @vtest.target crates/vtest-adapter-lexical/src/go.rs::Reader::statement_end
    /// @vtest.intent verifies that an annotated subtest statement inside a function is a Test construct running to the end of its call across lines, and that an unterminated raw string makes the file unreadable
    #[test]
    fn annotated_statements_and_unreadable_files() {
        let source = "package calc\n\nfunc TestAll(t *testing.T) {\n\t// @vtest.id TEST-GO-SUB\n\t// @vtest.covers VO-GO-ADD\n\t// @vtest.intent a subtest is a construct\n\tt.Run(\"adds\", func(t *testing.T) {\n\t\tcheck(t)\n\t})\n\tdone()\n}\n";
        let outcome = discover_text(&Go, "calc/all_test.go", source, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(outcome.tests.len(), 1);
        assert_eq!(
            outcome.tests[0].construct_text,
            "t.Run(\"adds\", func(t *testing.T) {\n\t\tcheck(t)\n\t})"
        );
        assert_eq!(outcome.tests[0].location.locator, "t#1");

        let broken = discover_text(
            &Go,
            "calc/broken.go",
            "package calc\n\nconst raw = `open\n",
            "demo",
        );
        assert_eq!(broken.diagnostics.len(), 1);
        assert_eq!(broken.diagnostics[0].code.as_str(), "E-SCAN-001");
    }
    /// @vtest.id TEST-ADAPTER-GO-EXPLICIT-SEMICOLON
    /// @vtest.covers VO-ADAPTER-LEXICAL-NORMALIZES-DECLARATION
    /// @vtest.target crates/vtest-adapter-lexical/src/go.rs::Reader::function_name
    /// @vtest.intent verifies a function followed by an explicit semicolon is found with the same name and SRC ID as one ended by a newline
    #[test]
    fn explicit_semicolon_after_a_body() {
        for source in [
            "package p\n// @vtest.src-id SRC-GO-F\nfunc f() {}\n",
            "package p;\n// @vtest.src-id SRC-GO-F\nfunc f() {};\n",
            "package p\n// @vtest.src-id SRC-GO-F\nfunc f() {}; func g() {}\n",
        ] {
            let outcome = discover_text(&Go, "p/p.go", source, "demo");
            assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
            assert_eq!(outcome.sources[0].locator.value, "p/p.go::f", "{source}");
            let range = &outcome.sources[0].location.byte_range;
            assert!(
                source[range.start as usize..range.end as usize].ends_with("{}"),
                "{source}"
            );
            assert_eq!(
                outcome.sources[0].src_id.as_ref().map(|id| id.as_str()),
                Some("SRC-GO-F"),
                "{source}"
            );
        }
    }
}
