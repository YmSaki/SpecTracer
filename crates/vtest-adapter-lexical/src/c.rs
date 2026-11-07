//! `c`: C sources and headers (`.c`, `.h`).
//!
//! A file is read in three steps, all specific to C:
//!
//! 1. [`lex`] splits it into comments, preprocessor lines (with `\`
//!    continuations), string and character literals, words, brackets, `;`,
//!    and other punctuation. A `#define` body is part of its directive, so
//!    braces in a macro are not code.
//! 2. [`Reader::declaration_end`] finds where a top-level declaration ends.
//!    The first top-level `{` is a function body, unless an initializer has
//!    started (`int t[] = { 1, 2 };`), in which case it belongs to the value.
//!    The body of a `struct`, `union`, or `enum` (and a `typedef` of one) is
//!    followed by declarators and ends at its `;`. A line that is only a
//!    macro invocation without `;` (`END_TEST`, `DEFINE_X(a)`) ends at its
//!    line break.
//! 3. A top-level declaration with a body and a parameter list is a
//!    function definition, named by the word before its parameter list
//!    (`add`, a test macro such as `START_TEST`). C has no named scopes.

use crate::{
    shared::{attached_comments, pair_brackets, Attached, Span},
    Annotated, ExtractError, FileItems, Function, LanguageProfile, LexicalAdapter,
};

pub const ADAPTER_ID: &str = "c";

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
            functions: reader.functions()?,
            annotated: reader.annotated(),
        })
    }
}

/// The `c` adapter.
pub fn adapter() -> LexicalAdapter<C> {
    LexicalAdapter::new(C)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    Comment,
    Directive,
    Literal,
    Word,
    Open(char),
    Close(char),
    Semi,
    Punct(char),
}

impl Kind {
    fn is_trivia(self) -> bool {
        matches!(self, Kind::Comment | Kind::Directive)
    }
}

#[derive(Clone, Copy, Debug)]
struct Tok {
    kind: Kind,
    start: usize,
    end: usize,
}

/// Splits C source into tokens. An unterminated comment or literal is an
/// error.
fn lex(source: &str) -> Result<Vec<Tok>, ExtractError> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    let mut line_start = true;
    while i < bytes.len() {
        let start = i;
        let rest = &source[i..];
        if bytes[i].is_ascii_whitespace() {
            line_start |= bytes[i] == b'\n';
            i += 1;
            continue;
        }
        let at_line_start = std::mem::replace(&mut line_start, false);
        let kind = if at_line_start && bytes[i] == b'#' {
            i = line_end(source, i);
            Kind::Directive
        } else if rest.starts_with("//") {
            i = line_end(source, i);
            Kind::Comment
        } else if let Some(body) = rest.strip_prefix("/*") {
            let close = body
                .find("*/")
                .ok_or_else(|| ExtractError::new(start, "unterminated comment"))?;
            i += close + 4;
            Kind::Comment
        } else if bytes[i] == b'"' || bytes[i] == b'\'' {
            i = quoted_end(source, i + 1, bytes[i])
                .ok_or_else(|| ExtractError::new(start, "unterminated literal"))?;
            Kind::Literal
        } else if bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || !bytes[i].is_ascii() {
            let len = rest
                .char_indices()
                .find(|(_, c)| !(c.is_alphanumeric() || *c == '_'))
                .map_or(rest.len(), |(offset, _)| offset);
            i += len.max(rest.chars().next().map_or(1, char::len_utf8));
            Kind::Word
        } else {
            i += 1;
            match bytes[start] {
                b'(' | b'[' | b'{' => Kind::Open(bytes[start] as char),
                b')' | b']' | b'}' => Kind::Close(bytes[start] as char),
                b';' => Kind::Semi,
                other => Kind::Punct(other as char),
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

/// The end of a preprocessor line or `//` comment starting at `start`:
/// the first line break not preceded by `\`, because a backslash-newline
/// is removed before comments and directives are recognized.
fn line_end(source: &str, start: usize) -> usize {
    let bytes = source.as_bytes();
    let mut i = start;
    while i < bytes.len() {
        if bytes[i] == b'\n' && !source[..i].trim_end_matches('\r').ends_with('\\') {
            return i;
        }
        i += 1;
    }
    i
}

fn quoted_end(source: &str, from: usize, quote: u8) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut i = from;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 1,
            b'\n' => return None,
            byte if byte == quote => return Some(i + 1),
            _ => {}
        }
        i += 1;
    }
    None
}

/// A top-level declaration: one past its last token, the `{` of its body,
/// and the first top-level `(` before the body.
struct Declaration {
    end: usize,
    body: Option<usize>,
    paren: Option<usize>,
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

    fn partner(&self, index: usize) -> usize {
        self.partners[index].expect("brackets are balanced")
    }

    fn next_significant(&self, from: usize) -> Option<usize> {
        (from..self.tokens.len()).find(|&i| !self.tokens[i].kind.is_trivia())
    }

    fn line_breaks(&self, before: usize, after: usize) -> usize {
        self.source[self.tokens[before].end..self.tokens[after].start]
            .matches('\n')
            .count()
    }

    /// Where the declaration starting at `start` ends (see the module
    /// documentation).
    fn declaration_end(&self, start: usize) -> Result<Declaration, String> {
        let mut initializer = false;
        let mut aggregate = false;
        let mut body = None;
        let mut paren = None;
        let mut index = start;
        while index < self.tokens.len() {
            match self.tokens[index].kind {
                Kind::Open('{') if !initializer && body.is_none() => {
                    body = Some(index);
                    let close = self.partner(index);
                    if !aggregate || paren.is_some() {
                        return Ok(Declaration {
                            end: close + 1,
                            body,
                            paren,
                        });
                    }
                    index = close + 1;
                    continue;
                }
                Kind::Open(open) => {
                    if open == '(' && body.is_none() && !initializer && !self.is_attribute(index) {
                        paren.get_or_insert(index);
                    }
                    let close = self.partner(index);
                    if self.is_macro_line(start, close) {
                        return Ok(Declaration {
                            end: close + 1,
                            body: None,
                            paren: None,
                        });
                    }
                    index = close + 1;
                    continue;
                }
                Kind::Close(_) => break,
                Kind::Semi => {
                    return Ok(Declaration {
                        end: index + 1,
                        body,
                        paren,
                    })
                }
                Kind::Punct('=') if body.is_none() => {
                    let comparison = index
                        .checked_sub(1)
                        .is_some_and(|p| matches!(self.text(p), "=" | "!" | "<" | ">"))
                        || self.tokens.get(index + 1).is_some_and(|n| {
                            n.start == self.tokens[index].end && self.text(index + 1) == "="
                        });
                    initializer |= !comparison;
                }
                Kind::Word => {
                    if matches!(self.text(index), "struct" | "union" | "enum" | "typedef") {
                        aggregate = true;
                    }
                    if self.is_macro_line(start, index) {
                        return Ok(Declaration {
                            end: index + 1,
                            body: None,
                            paren: None,
                        });
                    }
                }
                _ => {}
            }
            index += 1;
        }
        Err("the declaration has neither a body nor a `;`".to_owned())
    }

    /// Whether the `(` at `paren` holds the arguments of a compiler
    /// attribute (`__attribute__((noinline))`, `__declspec(dllexport)`)
    /// rather than a declarator.
    fn is_attribute(&self, paren: usize) -> bool {
        paren
            .checked_sub(1)
            .is_some_and(|p| self.tokens[p].kind == Kind::Word && is_attribute_word(self.text(p)))
    }

    /// Whether `tokens[start..=last]` is a lone macro invocation (`NAME` or
    /// `NAME(...)`) that ends its line without `;` or a body.
    fn is_macro_line(&self, start: usize, last: usize) -> bool {
        if is_attribute_word(self.text(start)) {
            return false;
        }
        let shape = if last == start {
            self.tokens[start].kind == Kind::Word
        } else {
            self.tokens[start].kind == Kind::Word
                && last > start + 1
                && self.tokens[start + 1].kind == Kind::Open('(')
                && self.partner(start + 1) == last
        };
        if !shape {
            return false;
        }
        match self.next_significant(last + 1) {
            None => true,
            Some(next) => {
                self.line_breaks(last, next) >= 1
                    && !matches!(
                        self.tokens[next].kind,
                        Kind::Open(_) | Kind::Semi | Kind::Punct(_)
                    )
                    && !(last == start && self.tokens[next].kind == Kind::Word)
            }
        }
    }

    /// The name declared by the declarator whose first top-level `(` is
    /// at `paren`: the word before it, or, when that `(` groups a pointer
    /// declarator (`int (*factory(void))(int)`), the name inside the group.
    fn declarator_name(&self, start: usize, paren: usize) -> Option<usize> {
        let inner = self.next_significant(paren + 1)?;
        if self.tokens[inner].kind == Kind::Punct('*') {
            let close = self.partner(paren);
            let open = (inner..close).find(|&i| self.tokens[i].kind == Kind::Open('('))?;
            return self.declarator_name(inner, open);
        }
        paren
            .checked_sub(1)
            .filter(|&n| n >= start && self.tokens[n].kind == Kind::Word)
    }

    /// Every top-level function definition.
    fn functions(&self) -> Result<Vec<Function>, ExtractError> {
        let mut output = Vec::new();
        let mut index = 0;
        while index < self.tokens.len() {
            if self.tokens[index].kind.is_trivia() || self.tokens[index].kind == Kind::Semi {
                index += 1;
                continue;
            }
            let declaration = self
                .declaration_end(index)
                .map_err(|message| ExtractError::new(self.tokens[index].start, &message))?;
            if let (Some(_), Some(paren)) = (declaration.body, declaration.paren) {
                if let Some(name) = self.declarator_name(index, paren) {
                    output.push(Function {
                        name: vec![self.text(name).to_owned()],
                        start: self.tokens[index].start,
                        end: self.tokens[declaration.end - 1].end,
                    });
                }
            }
            index = declaration.end;
        }
        Ok(output)
    }

    /// Every comment block that starts its own line and is followed, with
    /// no blank line between, by a declaration, together with that
    /// declaration's extent. Consecutive comments separated by at most one
    /// line break form one block.
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
            if matches!(
                self.tokens[next].kind,
                Kind::Close(_) | Kind::Semi | Kind::Directive
            ) {
                continue;
            }
            let construct = self.declaration_end(next).map(|declaration| {
                (
                    self.tokens[next].start,
                    self.tokens[declaration.end - 1].end,
                )
            });
            output.push(Annotated {
                comment_start: self.tokens[first].start,
                lines: (first..=last)
                    .flat_map(|i| comment_lines(self.text(i)))
                    .collect(),
                construct,
            });
        }
        output
    }
}

fn is_attribute_word(word: &str) -> bool {
    matches!(word, "__attribute__" | "__attribute" | "__declspec")
}

/// The lines of a `//` or `/* */` comment with the comment markers removed.
/// A `//` comment continued with `\` is one line.
fn comment_lines(comment: &str) -> Vec<String> {
    if let Some(line) = comment.strip_prefix("//") {
        return vec![line.replace("\\\r\n", "").replace("\\\n", "")];
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

    const SOURCE: &str = r#"#include <check.h>
#define BLOCK(x) { if (x) { return; } }

struct point { int x; int y; };
typedef struct { int w; } size_t2;
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
    /// @vtest.target crates/vtest-adapter-lexical/src/c.rs::C::extract
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

    /// @vtest.id TEST-ADAPTER-C-INITIALIZER-AND-STRUCT-EXTENTS
    /// @vtest.covers VO-ADAPTER-LEXICAL-RETURNS-SOURCE-RANGE-AND-BYTES
    /// @vtest.target crates/vtest-adapter-lexical/src/c.rs::Reader::declaration_end
    /// @vtest.intent verifies an annotated initializer and an annotated struct definition run to their semicolons, and a change to only the end of a function body changes the Test construct text
    #[test]
    fn initializers_and_structs_end_at_their_semicolon() {
        let declare = "// @vtest.id TEST-X\n// @vtest.covers VO-X\n// @vtest.intent check\n";
        let test = |body: &str| {
            let outcome = discover_text(&C, "t.c", &format!("{declare}{body}\n"), "demo");
            assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
            outcome.tests[0].construct_text.clone()
        };
        assert_eq!(test("int t[] = { 1, 2 };"), "int t[] = { 1, 2 };");
        assert_eq!(
            test("struct s { int x; } v = { 1 };"),
            "struct s { int x; } v = { 1 };"
        );
        assert_eq!(
            test("struct point *make(void) { return 0; }\nint later;"),
            "struct point *make(void) { return 0; }"
        );
        assert_ne!(
            test("void f(void) { a(); b(); }"),
            test("void f(void) { a(); c(); }")
        );
    }

    /// @vtest.id TEST-ADAPTER-C-POINTER-RETURNING-FUNCTION-NAME
    /// @vtest.covers VO-ADAPTER-C-REGISTERS-WITHOUT-CORE-CHANGES
    /// @vtest.target crates/vtest-adapter-lexical/src/c.rs::Reader::declarator_name
    /// @vtest.intent verifies that a function returning a function pointer is named by the identifier inside its grouped declarator, not by its return type
    #[test]
    fn pointer_returning_function_is_named_by_its_declarator() {
        let source = "int f(int x) { return x; }\nint (*factory(void))(int) { return &f; }\n";
        let outcome = discover_text(&C, "fixture.c", source, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        let locators = outcome
            .sources
            .iter()
            .map(|source| source.locator.value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(locators, vec!["fixture.c::f", "fixture.c::factory"]);
    }
    /// @vtest.id TEST-ADAPTER-C-LINE-SPLICES-AND-ATTRIBUTES
    /// @vtest.covers VO-ADAPTER-LEXICAL-NORMALIZES-DECLARATION
    /// @vtest.target crates/vtest-adapter-lexical/src/c.rs::Reader::declaration_end
    /// @vtest.intent verifies a line comment continued with a backslash hides the code on its next line, and compiler attributes before or after the return type are not taken for the function's declarator
    #[test]
    fn line_splices_and_attributes() {
        let source = "// disabled \\\nint phantom(void) { return 1; }\nint real(void) { return 2; }\n__attribute__((noinline)) int f(void) { return 1; }\n__attribute__((noinline)) __attribute__((cold))\nstatic int *g(void) { return 0; }\n// @vtest.src-id SRC-C-H\n__declspec(dllexport) int (*h(void))(int) { return 0; }\n";
        let outcome = discover_text(&C, "src/a.c", source, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        let sources = outcome
            .sources
            .iter()
            .map(|source| source.locator.value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            sources,
            vec!["src/a.c::real", "src/a.c::f", "src/a.c::g", "src/a.c::h"]
        );
        assert_eq!(
            outcome.sources[3].src_id.as_ref().map(|id| id.as_str()),
            Some("SRC-C-H")
        );
    }
}
