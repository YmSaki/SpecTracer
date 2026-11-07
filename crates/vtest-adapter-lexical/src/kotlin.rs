//! `kotlin`: Kotlin sources and scripts (`.kt`, `.kts`).
//!
//! A file is read in three steps, all specific to Kotlin:
//!
//! 1. [`lex`] splits it into comments (nested `/* */` included), string
//!    literals (raw `"""` strings and `${ ... }` templates included),
//!    character literals, words (backtick names included), brackets, `;`,
//!    and operators.
//! 2. [`Reader::declaration_end`] finds where a declaration or statement
//!    ends: at a `;`, or at a newline after a token that can end an
//!    expression when the next line cannot continue it. A next line
//!    starting with `.`, `?.`, `?:`, `&&`, `||`, `as`, `else`, `catch`, or
//!    `finally` continues it, so `= if (x) { 1 } else { 2 }` is one
//!    expression body. A newline never ends an annotation (`@Test`), an
//!    `if (...)`/`while (...)`/`for (...)`/`when (...)` condition, or a
//!    `fun`/`class` header still waiting for its `{`, `=`, `:`, or
//!    `where`. Brackets are skipped whole.
//! 3. [`Reader::members`] walks the members of the file: `class`,
//!    `interface`, and `object` bodies (`companion object` is `Companion`
//!    when unnamed) qualify the `fun` declarations inside them. A `fun`
//!    without a `{}` or `=` body (abstract) is not a Source Target, and
//!    an extension function is named without its receiver.
//!
//! A comment block attached to a declaration or statement makes it a Test
//! (or declares its SRC ID); its annotations are part of the construct.

use crate::{
    shared::{attached_comments, pair_brackets, Attached, Span},
    Annotated, ExtractError, FileItems, Function, LanguageProfile, LexicalAdapter,
};

pub const ADAPTER_ID: &str = "kotlin";

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
        let mut functions = Vec::new();
        reader.members(0, tokens.len(), &mut Vec::new(), &mut functions);
        Ok(FileItems {
            functions,
            annotated: reader.annotated(),
        })
    }
}

/// The `kotlin` adapter.
pub fn adapter() -> LexicalAdapter<Kotlin> {
    LexicalAdapter::new(Kotlin)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    Comment,
    Literal,
    Word,
    Open(char),
    Close(char),
    Semi,
    /// An operator; its text tells which.
    Op,
}

#[derive(Clone, Copy, Debug)]
struct Tok {
    kind: Kind,
    start: usize,
    end: usize,
}

/// The operators read as one token.
const OPERATORS: &[&str] = &["?.", "?:", "!!", "++", "--", "&&", "||", "->", "::"];

/// Splits Kotlin source into tokens. An unterminated comment or literal is
/// an error.
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
        } else if rest.starts_with("/*") {
            i = block_comment_end(bytes, i)
                .ok_or_else(|| ExtractError::new(start, "unterminated comment"))?;
            Kind::Comment
        } else if byte == b'"' || byte == b'\'' {
            i = literal_end(bytes, i)
                .ok_or_else(|| ExtractError::new(start, "unterminated literal"))?;
            Kind::Literal
        } else if byte == b'`' {
            let close = rest[1..]
                .find(['`', '\n'])
                .filter(|&close| rest.as_bytes()[close + 1] == b'`')
                .ok_or_else(|| ExtractError::new(start, "unterminated backtick name"))?;
            i += close + 2;
            Kind::Word
        } else if byte.is_ascii_digit() {
            i += 1;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric()
                    || bytes[i] == b'_'
                    || (bytes[i] == b'.' && bytes.get(i + 1).is_some_and(u8::is_ascii_digit)))
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
        } else if let Some(operator) = OPERATORS.iter().find(|op| rest.starts_with(**op)) {
            i += operator.len();
            Kind::Op
        } else {
            i += 1;
            match byte {
                b'(' | b'[' | b'{' => Kind::Open(byte as char),
                b')' | b']' | b'}' => Kind::Close(byte as char),
                b';' => Kind::Semi,
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

/// One past the `*/` closing the (possibly nested) comment opening at `i`.
fn block_comment_end(bytes: &[u8], mut i: usize) -> Option<usize> {
    let mut depth = 0usize;
    while i + 1 < bytes.len() {
        if bytes[i] == b'/' && bytes[i + 1] == b'*' {
            depth += 1;
            i += 2;
        } else if bytes[i] == b'*' && bytes[i + 1] == b'/' {
            depth -= 1;
            i += 2;
            if depth == 0 {
                return Some(i);
            }
        } else {
            i += 1;
        }
    }
    None
}

/// One past the end of the string or character literal opening at `i`.
/// `${ ... }` templates are read as code, so their braces and strings do
/// not end the literal.
fn literal_end(bytes: &[u8], i: usize) -> Option<usize> {
    if bytes[i] == b'\'' {
        let mut j = i + 1;
        while j < bytes.len() {
            match bytes[j] {
                b'\\' => j += 2,
                b'\'' => return Some(j + 1),
                b'\n' => return None,
                _ => j += 1,
            }
        }
        return None;
    }
    let raw = bytes[i..].starts_with(b"\"\"\"");
    let mut j = i + if raw { 3 } else { 1 };
    while j < bytes.len() {
        if raw && bytes[j..].starts_with(b"\"\"\"") {
            while bytes.get(j + 3) == Some(&b'"') {
                j += 1;
            }
            return Some(j + 3);
        }
        match bytes[j] {
            b'\\' if !raw => j += 2,
            b'"' if !raw => return Some(j + 1),
            b'\n' if !raw => return None,
            b'$' if bytes.get(j + 1) == Some(&b'{') => j = template_end(bytes, j + 2)?,
            _ => j += 1,
        }
    }
    None
}

/// One past the `}` closing a `${` template whose code starts at `i`.
fn template_end(bytes: &[u8], mut i: usize) -> Option<usize> {
    let mut depth = 1usize;
    while i < bytes.len() {
        match bytes[i] {
            b'"' | b'\'' => i = literal_end(bytes, i)?,
            b'/' if bytes.get(i + 1) == Some(&b'*') => i = block_comment_end(bytes, i)?,
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'{' => {
                depth += 1;
                i += 1;
            }
            b'}' => {
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

/// Keywords after which an expression cannot end.
const OPEN_KEYWORDS: &[&str] = &[
    "as",
    "class",
    "do",
    "else",
    "finally",
    "fun",
    "if",
    "in",
    "interface",
    "is",
    "object",
    "package",
    "throw",
    "try",
    "typealias",
    "val",
    "var",
    "when",
    "where",
    "while",
];

/// Tokens that, at the start of a line, continue the previous line.
const CONTINUATIONS: &[&str] = &[
    ".", "?.", "?:", "&&", "||", "as", "else", "catch", "finally",
];

/// What a member declaration declares.
enum Declares {
    Scope(String),
    Function(String),
    Other,
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

    fn is(&self, index: usize, text: &str) -> bool {
        index < self.tokens.len()
            && self.tokens[index].kind != Kind::Comment
            && self.text(index) == text
    }

    fn partner(&self, index: usize) -> usize {
        self.partners[index].expect("brackets are balanced")
    }

    fn touching(&self, a: usize, b: usize) -> bool {
        self.tokens[a].end == self.tokens[b].start
    }

    fn next_significant(&self, from: usize) -> Option<usize> {
        (from..self.tokens.len()).find(|&i| self.tokens[i].kind != Kind::Comment)
    }

    fn previous_significant(&self, index: usize) -> Option<usize> {
        (0..index)
            .rev()
            .find(|&i| self.tokens[i].kind != Kind::Comment)
    }

    fn line_breaks(&self, before: usize, after: usize) -> usize {
        self.source[self.tokens[before].end..self.tokens[after].start]
            .matches('\n')
            .count()
    }

    /// Whether `tokens[index]` is a member name after `.`, `?.`, or `::`
    /// (`Foo::class`), where a keyword is just a name.
    fn after_access(&self, index: usize) -> bool {
        self.previous_significant(index)
            .is_some_and(|p| matches!(self.text(p), "." | "?." | "::"))
    }

    /// Whether an expression can end with `tokens[last]`.
    fn can_end(&self, last: usize) -> bool {
        match self.tokens[last].kind {
            Kind::Literal => true,
            Kind::Word => !OPEN_KEYWORDS.contains(&self.text(last)) || self.after_access(last),
            Kind::Close(')') => {
                let open = self.partner(last);
                !self.previous_significant(open).is_some_and(|p| {
                    self.kind(p) == Some(Kind::Word)
                        && matches!(self.text(p), "if" | "while" | "for" | "when" | "catch")
                        && !self.after_access(p)
                })
            }
            Kind::Close(_) => true,
            Kind::Op => match self.text(last) {
                "!!" | "++" | "--" | "?" | ">" => true,
                "*" => self
                    .previous_significant(last)
                    .is_some_and(|p| self.is(p, ".")),
                _ => false,
            },
            _ => false,
        }
    }

    /// Whether `tokens[last]` ends an annotation: `@Test`, `@a.B`,
    /// `@field:Ann`, `@Tag("x")`, or `@[A B]`.
    fn ends_annotation(&self, last: usize) -> bool {
        let mut index = last;
        if matches!(self.kind(index), Some(Kind::Close(')' | ']'))) {
            let open = self.partner(index);
            if self.kind(index) == Some(Kind::Close(']')) {
                return open > 0 && self.is_annotation_at(open - 1);
            }
            match open.checked_sub(1) {
                Some(before) if self.touching(before, open) => index = before,
                _ => return false,
            }
        }
        while self.kind(index) == Some(Kind::Word) {
            let Some(before) = index.checked_sub(1) else {
                return false;
            };
            if self.is_annotation_at(before) {
                return true;
            }
            if !(matches!(self.text(before), "." | ":") && before > 0) {
                return false;
            }
            index = before - 1;
        }
        false
    }

    /// Whether the `@` at `index` starts an annotation (not a label use
    /// such as `this@Outer`).
    fn is_annotation_at(&self, index: usize) -> bool {
        self.is(index, "@")
            && !(index > 0
                && self.kind(index - 1) == Some(Kind::Word)
                && self.touching(index - 1, index))
    }

    /// One past the last token of the declaration or statement starting at
    /// `start` (see the module documentation). A `}` or `)` closing the
    /// enclosing block also ends it.
    fn declaration_end(&self, start: usize) -> usize {
        let mut header = false;
        let mut property = false;
        let mut valued = false;
        let mut index = start;
        while index < self.tokens.len() {
            let last = match self.tokens[index].kind {
                Kind::Comment => {
                    index += 1;
                    continue;
                }
                Kind::Close(_) => return index,
                Kind::Semi => return index + 1,
                Kind::Open(open) => {
                    if open == '{' {
                        header = false;
                    }
                    self.partner(index)
                }
                Kind::Word if !valued && !self.after_access(index) => {
                    match self.text(index) {
                        "fun" | "class" | "interface" | "object" => header = true,
                        "val" | "var" => property = true,
                        _ => {}
                    }
                    index
                }
                Kind::Op if self.text(index) == "=" => {
                    valued = true;
                    header = false;
                    index
                }
                _ => index,
            };
            let Some(next) = self.next_significant(last + 1) else {
                return last + 1;
            };
            if self.line_breaks(last, next) > 0
                && self.can_end(last)
                && !self.ends_annotation(last)
                && !CONTINUATIONS.contains(&self.text(next))
                && !(header && matches!(self.text(next), "{" | "=" | ":" | "where"))
                && !(property && self.starts_accessor(next))
            {
                return last + 1;
            }
            index = next;
        }
        index
    }

    /// Whether a property accessor (`get() = ...`, `private set`) starts at
    /// `index`.
    fn starts_accessor(&self, index: usize) -> bool {
        let accessor = if matches!(
            self.text(index),
            "private" | "protected" | "internal" | "public"
        ) {
            index + 1
        } else {
            index
        };
        self.kind(accessor) == Some(Kind::Word) && matches!(self.text(accessor), "get" | "set")
    }

    /// Collects the `fun` definitions among the declarations in
    /// `tokens[low..high]`, qualified by `scope`.
    fn members(
        &self,
        low: usize,
        high: usize,
        scope: &mut Vec<String>,
        output: &mut Vec<Function>,
    ) {
        let mut index = low;
        while let Some(start) = self.next_significant(index).filter(|&s| s < high) {
            if matches!(self.tokens[start].kind, Kind::Semi | Kind::Close(_)) {
                index = start + 1;
                continue;
            }
            let end = self.declaration_end(start).clamp(start + 1, high);
            match self.declares(start, end) {
                (Declares::Scope(name), Some(open)) => {
                    scope.push(name);
                    self.members(open + 1, self.partner(open), scope, output);
                    scope.pop();
                }
                (Declares::Function(name), _) => {
                    let mut qualified = scope.clone();
                    qualified.push(name);
                    output.push(Function {
                        name: qualified,
                        start: self.tokens[start].start,
                        end: self.tokens[end - 1].end,
                    });
                }
                _ => {}
            }
            index = end;
        }
    }

    /// What `tokens[start..end]` declares, and the `{` of its body.
    fn declares(&self, start: usize, end: usize) -> (Declares, Option<usize>) {
        let mut index = start;
        while index < end {
            match self.tokens[index].kind {
                Kind::Open(_) => index = self.partner(index),
                Kind::Op if self.text(index) == "=" => break,
                Kind::Word => match self.text(index) {
                    "fun" if !self.is(index + 1, "interface") => {
                        return (self.function(index, end), None)
                    }
                    "class" | "interface" | "object" => return self.scope(index, end),
                    "val" | "var" | "typealias" | "constructor" | "init" => break,
                    _ => {}
                },
                _ => {}
            }
            index += 1;
        }
        (Declares::Other, None)
    }

    /// The `fun` at `keyword`: named by the word before its parameter list,
    /// if it has a `{}` or `=` body.
    fn function(&self, keyword: usize, end: usize) -> Declares {
        let Some(paren) = (keyword..end).find(|&i| self.kind(i) == Some(Kind::Open('('))) else {
            return Declares::Other;
        };
        let has_body = (self.partner(paren) + 1..end)
            .any(|i| self.kind(i) == Some(Kind::Open('{')) || self.is(i, "="));
        match paren.checked_sub(1) {
            Some(name) if has_body && name > keyword && self.kind(name) == Some(Kind::Word) => {
                Declares::Function(self.text(name).trim_matches('`').to_owned())
            }
            _ => Declares::Other,
        }
    }

    /// The `class`, `interface`, or `object` at `keyword` and its body.
    fn scope(&self, keyword: usize, end: usize) -> (Declares, Option<usize>) {
        let companion = keyword > 0 && self.is(keyword - 1, "companion");
        let name = match self.kind(keyword + 1) {
            Some(Kind::Word) => self.text(keyword + 1).trim_matches('`').to_owned(),
            _ if companion => "Companion".to_owned(),
            _ => return (Declares::Other, None),
        };
        let mut index = keyword + 1;
        while index < end {
            match self.tokens[index].kind {
                Kind::Open('{') => return (Declares::Scope(name), Some(index)),
                Kind::Open(_) => index = self.partner(index),
                _ => {}
            }
            index += 1;
        }
        (Declares::Other, None)
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
            let end = self.declaration_end(next).max(next + 1);
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

/// The lines of a `//` or `/* */` comment with the comment markers removed
/// (`/**` and the leading `*` of KDoc lines included).
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
    /// @vtest.target crates/vtest-adapter-lexical/src/kotlin.rs::Kotlin::extract
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

    /// @vtest.id TEST-ADAPTER-KOTLIN-EXPRESSION-BODY-RUNS-TO-ITS-END
    /// @vtest.covers VO-ADAPTER-KOTLIN-REGISTERS-WITHOUT-CORE-CHANGES
    /// @vtest.target crates/vtest-adapter-lexical/src/kotlin.rs::Reader::declaration_end
    /// @vtest.intent verifies that an expression body continued by else, catch, a leading dot, or a trailing operator runs to the end of its last branch, so a Test construct includes the body's tail and an edit to only that tail changes its text, while the next member stays separate
    #[test]
    fn expression_body_runs_to_its_end() {
        let source = |tail: &str| {
            format!(
                "class PickTest {{\n    // @vtest.id TEST-KT-PICK\n    // @vtest.covers VO-KT-ADD\n    // @vtest.intent the whole expression body is the construct\n    @Test\n    fun picks() = if (flag) {{\n        check(1)\n    }} else {{\n        check(2)\n        {tail}\n    }}\n\n    fun guarded(): Int =\n        try {{ load() }}\n        catch (e: Exception) {{ 0 }}\n\n    fun chained() = listOf(1, 2)\n        .map {{ it * 2 }}\n        .sum()\n\n    fun after() {{}}\n}}\n"
            )
        };
        let outcome = discover_text(&Kotlin, "src/Pick.kt", &source("one()"), "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        let sources = outcome
            .sources
            .iter()
            .map(|source| {
                (
                    source.locator.value.as_str(),
                    source.construct_text.as_str(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(sources[0].0, "src/Pick.kt::PickTest::picks");
        assert!(sources[0].1.ends_with("one()\n    }"));
        assert_eq!(
            sources[1],
            (
                "src/Pick.kt::PickTest::guarded",
                "fun guarded(): Int =\n        try { load() }\n        catch (e: Exception) { 0 }"
            )
        );
        assert!(sources[2].1.ends_with(".sum()"));
        assert_eq!(sources[3].0, "src/Pick.kt::PickTest::after");
        let test = &outcome.tests[0];
        assert!(test.construct_text.starts_with("@Test\n    fun picks()"));
        assert!(test.construct_text.ends_with("one()\n    }"));
        let edited = discover_text(&Kotlin, "src/Pick.kt", &source("two()"), "demo");
        assert_ne!(edited.tests[0].construct_text, test.construct_text);
    }

    /// @vtest.id TEST-ADAPTER-KOTLIN-TEMPLATES-AND-UNREADABLE-FILES
    /// @vtest.covers VO-ADAPTER-KOTLIN-REGISTERS-WITHOUT-CORE-CHANGES
    /// @vtest.target crates/vtest-adapter-lexical/src/kotlin.rs::literal_end
    /// @vtest.intent verifies that braces and quotes inside string templates are not code, that a Kotest-style annotated string block is a Test construct, and that an unterminated raw string makes the file unreadable
    #[test]
    fn templates_and_unreadable_files() {
        let source = "class CalcSpec : StringSpec({\n    // @vtest.id TEST-KT-SPEC\n    // @vtest.covers VO-KT-ADD\n    // @vtest.intent a Kotest block is a construct\n    \"adds ${listOf(\"}\").map { it }}\" {\n        check()\n    }\n})\n";
        let outcome = discover_text(&Kotlin, "src/CalcSpec.kt", source, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(outcome.tests.len(), 1);
        assert!(outcome.tests[0].construct_text.ends_with("check()\n    }"));

        let broken = discover_text(&Kotlin, "src/Broken.kt", "val x = \"\"\"open\n", "demo");
        assert_eq!(broken.diagnostics.len(), 1);
        assert_eq!(broken.diagnostics[0].code.as_str(), "E-SCAN-001");
    }
}
