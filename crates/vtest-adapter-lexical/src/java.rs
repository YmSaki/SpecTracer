//! `java`: Java sources (`.java`).
//!
//! A file is read in three steps, all specific to Java:
//!
//! 1. [`lex`] splits it into comments, string and character literals (text
//!    blocks included), words, brackets, `;`, and other punctuation.
//! 2. [`member_end`] finds where a member declaration ends: after the `{}`
//!    body of a type, method, or constructor, or at its `;`. Brackets inside
//!    annotations and parameter lists (`@Tag(values = {"a"})`) and the
//!    value of a field initializer (`= {1, 2}`, `= new Runnable() { ... }`)
//!    are skipped whole, because they are not a body.
//! 3. [`Reader::members`] walks the members of the file: `class`,
//!    `interface`, `@interface`, `enum`, and `record` bodies qualify the
//!    methods and constructors inside them. A method without a body
//!    (abstract or in an interface) is not a Source Target.
//!
//! A comment block attached to a member makes that member a Test (or
//! declares its SRC ID), whatever framework annotations (`@Test`,
//! `@ParameterizedTest`, ...) it carries.

use crate::{
    shared::{attached_comments, pair_brackets, Attached, Span},
    Annotated, ExtractError, FileItems, Function, LanguageProfile, LexicalAdapter,
};

pub const ADAPTER_ID: &str = "java";

#[derive(Clone, Copy, Debug, Default)]
pub struct Java;

impl LanguageProfile for Java {
    fn adapter_id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn languages(&self) -> &'static [&'static str] {
        &["java"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["java"]
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
        reader.members(0, tokens.len(), &mut Vec::new(), &mut functions)?;
        Ok(FileItems {
            functions,
            annotated: reader.annotated(),
        })
    }
}

/// The `java` adapter.
pub fn adapter() -> LexicalAdapter<Java> {
    LexicalAdapter::new(Java)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    Comment,
    Literal,
    Word,
    Open(char),
    Close(char),
    Semi,
    Punct(char),
}

#[derive(Clone, Copy, Debug)]
struct Tok {
    kind: Kind,
    start: usize,
    end: usize,
}

/// Splits Java source into tokens. An unterminated comment or literal is an
/// error.
fn lex(source: &str) -> Result<Vec<Tok>, ExtractError> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        let rest = &source[i..];
        let kind = if bytes[i].is_ascii_whitespace() {
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
        } else if rest.starts_with("\"\"\"") {
            i = literal_end(source, i + 3, "\"\"\"", true)
                .ok_or_else(|| ExtractError::new(start, "unterminated text block"))?;
            Kind::Literal
        } else if bytes[i] == b'"' || bytes[i] == b'\'' {
            let quote = if bytes[i] == b'"' { "\"" } else { "'" };
            i = literal_end(source, i + 1, quote, false)
                .ok_or_else(|| ExtractError::new(start, "unterminated literal"))?;
            Kind::Literal
        } else if is_word_byte(bytes[i]) || !bytes[i].is_ascii() {
            let len = rest
                .char_indices()
                .find(|(_, c)| !(c.is_alphanumeric() || *c == '_' || *c == '$'))
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

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$'
}

/// The offset just past the `close` that ends a literal whose content starts
/// at `from`, honoring `\` escapes. A single-line literal may not contain a
/// newline.
fn literal_end(source: &str, from: usize, close: &str, multiline: bool) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut i = from;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == b'\n' && !multiline {
            return None;
        }
        if source[i..].starts_with(close) {
            return Some(i + close.len());
        }
        i += 1;
    }
    None
}

/// A member declaration: one past its last token, and the `{` of its body.
struct Member {
    end: usize,
    body: Option<usize>,
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

    /// Where the member declaration starting at `start` ends.
    ///
    /// The first `{` at the declaration's top level is its body, unless the
    /// declaration has already assigned a value (`int[] a = {1, 2};`,
    /// `Runnable r = new Runnable() { ... };`), in which case the braces are
    /// part of the value and the declaration ends at its `;`. Brackets
    /// inside `(...)` and `[...]` are skipped whole. A declaration that
    /// reaches the end of its enclosing block without a body or `;` cannot
    /// be read.
    fn member_end(&self, start: usize) -> Result<Member, String> {
        let mut assigned = false;
        let mut index = start;
        while index < self.tokens.len() {
            match self.tokens[index].kind {
                Kind::Open('{') if !assigned => {
                    let close = self.partner(index);
                    let mut end = close + 1;
                    if self.tokens.get(end).is_some_and(|t| t.kind == Kind::Semi) {
                        end += 1;
                    }
                    return Ok(Member {
                        end,
                        body: Some(index),
                    });
                }
                Kind::Open(_) => {
                    index = self.partner(index) + 1;
                    continue;
                }
                Kind::Close(_) => break,
                Kind::Semi => {
                    return Ok(Member {
                        end: index + 1,
                        body: None,
                    })
                }
                Kind::Punct('=') if !self.is_comparison(index) => assigned = true,
                _ => {}
            }
            index += 1;
        }
        Err("the declaration has neither a body nor a `;`".to_owned())
    }

    /// Whether the `=` at `index` is part of `==`, `!=`, `<=`, or `>=`.
    fn is_comparison(&self, index: usize) -> bool {
        let touching = |other: usize| {
            let (a, b) = (self.tokens[index], self.tokens[other]);
            a.end == b.start || b.end == a.start
        };
        let before = index
            .checked_sub(1)
            .filter(|&p| touching(p) && matches!(self.text(p), "=" | "!" | "<" | ">"));
        let after = (index + 1 < self.tokens.len())
            .then_some(index + 1)
            .filter(|&n| touching(n) && self.text(n) == "=");
        before.is_some() || after.is_some()
    }

    /// Collects the methods and constructors among the members in
    /// `tokens[low..high]`, qualified by `scope`.
    fn members(
        &self,
        low: usize,
        high: usize,
        scope: &mut Vec<String>,
        output: &mut Vec<Function>,
    ) -> Result<(), ExtractError> {
        let mut index = low;
        while index < high {
            if matches!(self.tokens[index].kind, Kind::Comment | Kind::Semi) {
                index += 1;
                continue;
            }
            let member = self
                .member_end(index)
                .map_err(|message| ExtractError::new(self.tokens[index].start, &message))?;
            let header_end = member.body.unwrap_or(member.end);
            match self.declares(index, header_end) {
                Declares::Type(name) => {
                    if let Some(open) = member.body {
                        scope.push(name);
                        self.members(open + 1, self.partner(open), scope, output)?;
                        scope.pop();
                    }
                }
                Declares::Method(name) if member.body.is_some() => {
                    let mut qualified = scope.clone();
                    qualified.push(name);
                    output.push(Function {
                        name: qualified,
                        start: self.tokens[index].start,
                        end: self.tokens[member.end - 1].end,
                    });
                }
                _ => {}
            }
            index = member.end;
        }
        Ok(())
    }

    /// What the member header `tokens[start..end]` (the declaration before
    /// its body) declares.
    fn declares(&self, start: usize, end: usize) -> Declares {
        let mut index = self.skip_annotations(start, end);
        let mut paren = None;
        while index < end {
            match self.tokens[index].kind {
                Kind::Word
                    if matches!(self.text(index), "class" | "interface" | "enum" | "record") =>
                {
                    return self
                        .word_after(index, end)
                        .map_or(Declares::Other, Declares::Type);
                }
                Kind::Punct('@') if index + 1 < end && self.text(index + 1) == "interface" => {
                    return self
                        .word_after(index + 1, end)
                        .map_or(Declares::Other, Declares::Type);
                }
                Kind::Punct('=') => return Declares::Other,
                Kind::Open('(') => {
                    paren = Some(index);
                    break;
                }
                Kind::Open(_) => index = self.partner(index),
                _ => {}
            }
            index += 1;
        }
        let Some(paren) = paren else {
            return Declares::Other;
        };
        let Some(name) = paren.checked_sub(1).filter(|&n| n >= start) else {
            return Declares::Other;
        };
        if self.tokens[name].kind != Kind::Word
            || (name > start && self.text(name - 1) == "new")
            || self.text(name) == "new"
        {
            return Declares::Other;
        }
        Declares::Method(self.text(name).to_owned())
    }

    /// The first token after the annotations (`@Test`,
    /// `@Timeout(value = 5)`, `@org.junit.Test`) that begin a header.
    fn skip_annotations(&self, start: usize, end: usize) -> usize {
        let mut index = start;
        loop {
            while index < end && self.tokens[index].kind == Kind::Comment {
                index += 1;
            }
            let is_annotation = index + 1 < end
                && self.tokens[index].kind == Kind::Punct('@')
                && self.tokens[index + 1].kind == Kind::Word
                && self.text(index + 1) != "interface";
            if !is_annotation {
                return index;
            }
            index += 2;
            while index + 1 < end
                && self.tokens[index].kind == Kind::Punct('.')
                && self.tokens[index + 1].kind == Kind::Word
            {
                index += 2;
            }
            if index < end && self.tokens[index].kind == Kind::Open('(') {
                index = self.partner(index) + 1;
            }
        }
    }

    fn word_after(&self, index: usize, end: usize) -> Option<String> {
        (index + 1..end)
            .find(|&i| self.tokens[i].kind != Kind::Comment)
            .filter(|&i| self.tokens[i].kind == Kind::Word)
            .map(|i| self.text(i).to_owned())
    }

    /// Every comment block that starts its own line and is followed, with
    /// no blank line between, by a member or statement, together with that
    /// construct's extent. Consecutive comments separated by at most one
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
            if matches!(self.tokens[next].kind, Kind::Close(_) | Kind::Semi) {
                continue;
            }
            let construct = self
                .member_end(next)
                .map(|member| (self.tokens[next].start, self.tokens[member.end - 1].end));
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

enum Declares {
    Type(String),
    Method(String),
    Other,
}

/// The lines of a `//` or `/* */` comment with the comment markers removed
/// (`/**` and the leading `*` of Javadoc lines included).
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

    const SOURCE: &str = r#"package demo;

import org.junit.jupiter.api.Test;

public class Calculator {
    private final Runnable hook = new Runnable() { public void run() {} };
    private static final int[] PRIMES = {2, 3, 5};
    private static final String BRACES = """
        { not code }
        """;

    public Calculator() {}

    // @vtest.src-id SRC-JAVA-ADD
    public int add(int a, int b) { return a + b; }

    public <T extends Comparable<T>> T max(java.util.List<T> items) {
        if (items.isEmpty()) { throw new IllegalArgumentException("}"); }
        return items.get(0);
    }

    abstract static class Shape { abstract double area(); }

    @interface Marker { String value() default "x"; }

    record Point(int x, int y) {
        Point { if (x < 0) throw new IllegalArgumentException(); }
        int sum() { return x + y; }
    }
}

class CalculatorTest {
    /**
     * @vtest.id TEST-JAVA-ADD
     * @vtest.covers VO-JAVA-ADD
     * @vtest.target src/Calculator.java::Calculator::add
     * @vtest.intent adding two numbers returns their sum
     */
    @Test
    @Timeout(value = 5, unit = TimeUnit.SECONDS)
    void addsNumbers() {
        assertEquals(3, new Calculator().add(1, 2));
    }
}
"#;

    /// @vtest.id TEST-ADAPTER-JAVA-DISCOVERS-METHODS-AND-TESTS
    /// @vtest.covers VO-ADAPTER-JAVA-REGISTERS-WITHOUT-CORE-CHANGES, VO-ADAPTER-LEXICAL-OPAQUE-LOCATOR-SYNTAX
    /// @vtest.target crates/vtest-adapter-lexical/src/java.rs::Java::extract
    /// @vtest.intent verifies Java methods and constructors become class-qualified Source Targets while initializers, abstract methods, and anonymous-class fields do not, and an annotated JUnit method is a Test construct including its annotations
    #[test]
    fn discovers_java_methods_and_annotated_tests() {
        let outcome = discover_text(&Java, "src/Calculator.java", SOURCE, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        let sources = outcome
            .sources
            .iter()
            .map(|source| source.locator.value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            sources,
            vec![
                "src/Calculator.java::Calculator::Calculator",
                "src/Calculator.java::Calculator::add",
                "src/Calculator.java::Calculator::max",
                "src/Calculator.java::Calculator::Point::sum",
                "src/Calculator.java::CalculatorTest::addsNumbers",
            ]
        );
        assert_eq!(
            outcome.sources[1].src_id.as_ref().map(|id| id.as_str()),
            Some("SRC-JAVA-ADD")
        );
        assert_eq!(outcome.tests.len(), 1);
        let test = &outcome.tests[0];
        assert_eq!(test.id.as_str(), "TEST-JAVA-ADD");
        assert_eq!(test.location.locator, "CalculatorTest::addsNumbers");
        assert!(test.construct_text.starts_with("@Test\n    @Timeout"));
        assert!(test.construct_text.ends_with("add(1, 2));\n    }"));
    }

    fn only_test(source: &str) -> String {
        let outcome = discover_text(&Java, "src/T.java", source, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(
            outcome.tests.len(),
            1,
            "{} missing",
            outcome.missing_tests.len()
        );
        outcome.tests[0].construct_text.clone()
    }

    /// @vtest.id TEST-ADAPTER-JAVA-ANNOTATION-BRACES-ARE-NOT-THE-BODY
    /// @vtest.covers VO-ADAPTER-LEXICAL-RETURNS-SOURCE-RANGE-AND-BYTES
    /// @vtest.target crates/vtest-adapter-lexical/src/java.rs::Reader::member_end
    /// @vtest.intent verifies braces inside an annotation's arguments do not end the Test construct, so it runs to the end of the method body, and a change to only the end of the body changes the construct text
    #[test]
    fn annotation_braces_are_not_the_body() {
        let declare =
            "class C {\n  // @vtest.id TEST-X\n  // @vtest.covers VO-X\n  // @vtest.intent check\n";
        let one = format!(
            "{declare}  @Tag(values = {{\"a\", \"b\"}})\n  void test() {{ marker(); }}\n}}\n"
        );
        let two = one.replace("marker();", "marker(); other();");
        assert_eq!(
            only_test(&one),
            "@Tag(values = {\"a\", \"b\"})\n  void test() { marker(); }"
        );
        assert_ne!(only_test(&one), only_test(&two));
    }

    /// @vtest.id TEST-ADAPTER-JAVA-FIELD-INITIALIZER-IS-ONE-CONSTRUCT
    /// @vtest.covers VO-ADAPTER-LEXICAL-RETURNS-SOURCE-RANGE-AND-BYTES
    /// @vtest.target crates/vtest-adapter-lexical/src/java.rs::Reader::member_end
    /// @vtest.intent verifies a field initialized with braces or an anonymous class runs to its semicolon, and a file with a declaration that has neither a body nor a semicolon is E-SCAN-001 with no Test taken from it
    #[test]
    fn initializers_end_at_their_semicolon_and_unreadable_members_fail_the_file() {
        let declare =
            "class C {\n  // @vtest.id TEST-X\n  // @vtest.covers VO-X\n  // @vtest.intent check\n";
        assert_eq!(
            only_test(&format!(
                "{declare}  Runnable r = new Runnable() {{ public void run() {{}} }};\n}}\n"
            )),
            "Runnable r = new Runnable() { public void run() {} };"
        );
        let outcome = discover_text(
            &Java,
            "src/T.java",
            &format!("{declare}  int broken\n}}\n"),
            "demo",
        );
        assert!(outcome.tests.is_empty());
        assert!(outcome.sources.is_empty());
        assert_eq!(outcome.diagnostics.len(), 1);
        assert_eq!(outcome.diagnostics[0].code, "E-SCAN-001");
    }
}
