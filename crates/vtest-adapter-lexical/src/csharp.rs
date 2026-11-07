//! `csharp`: C# sources (`.cs`).
//!
//! A file is read in three steps, all specific to C#:
//!
//! 1. [`lex`] splits it into comments, preprocessor lines (`#if`,
//!    `#region`), literals (regular, verbatim `@"..."`, raw `"""..."""`,
//!    and interpolated `$"{x}"` strings, and characters), words, brackets,
//!    `;`, and other punctuation.
//! 2. [`Reader::member_end`] finds where a member declaration ends. The
//!    first top-level `{` is its body unless a value has started (`=` or
//!    `=>`), in which case braces belong to the value (`= new() { X = 1 }`,
//!    `=> new[] { 1 }`) and the member ends at its `;`. Attribute lists
//!    (`[Values(new int[] { 1, 2 })]`) and parameter lists are skipped
//!    whole. A property body may be followed by an initializer
//!    (`{ get; set; } = 5;`).
//! 3. [`Reader::members`] walks the members: block and file-scoped
//!    `namespace` and `class`, `struct`, `interface`, `record`, and `enum`
//!    bodies qualify the methods, constructors, and operators inside them.
//!    A method is a Source Target when it has a block body or an
//!    expression body (`=> expr;`); properties and positional records are
//!    not.

use crate::{
    shared::{attached_comments, pair_brackets, Attached, Span},
    Annotated, ExtractError, FileItems, Function, LanguageProfile, LexicalAdapter,
};

pub const ADAPTER_ID: &str = "csharp";

#[derive(Clone, Copy, Debug, Default)]
pub struct CSharp;

impl LanguageProfile for CSharp {
    fn adapter_id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn languages(&self) -> &'static [&'static str] {
        &["csharp"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["cs"]
    }

    fn excluded_dirs(&self) -> &'static [&'static str] {
        &["bin", "obj"]
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

/// The `csharp` adapter.
pub fn adapter() -> LexicalAdapter<CSharp> {
    LexicalAdapter::new(CSharp)
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

/// Splits C# source into tokens. An unterminated comment or literal is an
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
            i += rest.find('\n').unwrap_or(rest.len());
            Kind::Directive
        } else if rest.starts_with("//") {
            i += rest.find('\n').unwrap_or(rest.len());
            Kind::Comment
        } else if let Some(body) = rest.strip_prefix("/*") {
            let close = body
                .find("*/")
                .ok_or_else(|| ExtractError::new(start, "unterminated comment"))?;
            i += close + 4;
            Kind::Comment
        } else if let Some(end) = string_end(source, i)? {
            i = end;
            Kind::Literal
        } else if bytes[i] == b'\'' {
            i = quoted_end(source, i + 1, b'\'')
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

/// The end of a string literal starting at `start`, or `None` when no
/// string starts there. Prefixes `$` (interpolated) and `@` (verbatim) may
/// be combined in either order; `"""` starts a raw string.
fn string_end(source: &str, start: usize) -> Result<Option<usize>, ExtractError> {
    let bytes = source.as_bytes();
    let mut i = start;
    let mut interpolated = false;
    let mut verbatim = false;
    while i < bytes.len() && matches!(bytes[i], b'$' | b'@') && i - start < 4 {
        interpolated |= bytes[i] == b'$';
        verbatim |= bytes[i] == b'@';
        i += 1;
    }
    if bytes.get(i) != Some(&b'"') {
        return Ok(None);
    }
    let unterminated = || ExtractError::new(start, "unterminated string");
    let quotes = bytes[i..].iter().take_while(|&&b| b == b'"').count();
    if quotes >= 3 {
        let fence = &source[i..i + quotes];
        let close = source[i + quotes..].find(fence).ok_or_else(unterminated)?;
        return Ok(Some(i + quotes + close + quotes));
    }
    i += 1;
    let mut depth = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if !verbatim && depth == 0 => i += 1,
            b'\n' if !verbatim && depth == 0 => return Err(unterminated()),
            b'"' if depth == 0 => {
                if verbatim && bytes.get(i + 1) == Some(&b'"') {
                    i += 1;
                } else {
                    return Ok(Some(i + 1));
                }
            }
            b'{' if interpolated => {
                if depth == 0 && bytes.get(i + 1) == Some(&b'{') {
                    i += 1;
                } else {
                    depth += 1;
                }
            }
            b'}' if interpolated && depth > 0 => depth -= 1,
            b'"' => {
                // A string inside an interpolation hole.
                i = string_end(source, i)?.ok_or_else(unterminated)? - 1;
            }
            _ => {}
        }
        i += 1;
    }
    Err(unterminated())
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

/// A member declaration: one past its last token, the `{` of its body, and
/// whether it has an expression body (`=>`).
struct Member {
    end: usize,
    body: Option<usize>,
    arrow: bool,
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

    /// Whether the tokens at `index` and `index + 1` touch (`=>`, `::`).
    fn touches_next(&self, index: usize) -> bool {
        index + 1 < self.tokens.len() && self.tokens[index].end == self.tokens[index + 1].start
    }

    /// Where the member declaration starting at `start` ends (see the
    /// module documentation).
    fn member_end(&self, start: usize) -> Result<Member, String> {
        let mut value = false;
        let mut arrow = false;
        let mut body = None;
        let mut index = start;
        while index < self.tokens.len() {
            match self.tokens[index].kind {
                Kind::Open('{') if !value => {
                    body.get_or_insert(index);
                    let close = self.partner(index);
                    let next = self.next_significant(close + 1);
                    if next.and_then(|n| self.kind(n)) == Some(Kind::Punct('='))
                        && !self.touches_next(next.unwrap_or(0))
                    {
                        // `{ get; set; } = 5;`
                        index = close + 1;
                        continue;
                    }
                    let mut end = close + 1;
                    if self.kind(end) == Some(Kind::Semi) {
                        end += 1;
                    }
                    return Ok(Member { end, body, arrow });
                }
                Kind::Open(_) => {
                    index = self.partner(index) + 1;
                    continue;
                }
                Kind::Close(_) => break,
                Kind::Semi => {
                    return Ok(Member {
                        end: index + 1,
                        body,
                        arrow,
                    })
                }
                Kind::Punct('=') => {
                    let previous = index.checked_sub(1).filter(|&p| {
                        self.tokens[p].end == self.tokens[index].start
                            && matches!(self.text(p), "=" | "!" | "<" | ">")
                    });
                    let next_is = |c: char| {
                        self.touches_next(index) && self.kind(index + 1) == Some(Kind::Punct(c))
                    };
                    if next_is('>') {
                        value = true;
                        arrow = body.is_none();
                    } else if previous.is_none() && !next_is('=') {
                        value = true;
                    }
                }
                _ => {}
            }
            index += 1;
        }
        Err("the declaration has neither a body nor a `;`".to_owned())
    }

    fn next_significant(&self, from: usize) -> Option<usize> {
        (from..self.tokens.len()).find(|&i| !self.tokens[i].kind.is_trivia())
    }

    /// Collects the methods among the members in `tokens[low..high]`,
    /// qualified by `scope`. A file-scoped `namespace X;` qualifies the
    /// rest of `low..high`.
    fn members(
        &self,
        low: usize,
        high: usize,
        scope: &mut Vec<String>,
        output: &mut Vec<Function>,
    ) -> Result<(), ExtractError> {
        let depth = scope.len();
        let mut index = low;
        while index < high {
            if self.tokens[index].kind.is_trivia() || self.tokens[index].kind == Kind::Semi {
                index += 1;
                continue;
            }
            let member = self
                .member_end(index)
                .map_err(|message| ExtractError::new(self.tokens[index].start, &message))?;
            let header_end = member.body.unwrap_or(member.end);
            match self.declares(index, header_end) {
                Declares::Scope(name) => match member.body {
                    Some(open) => {
                        let restore = scope.len();
                        scope.extend(name);
                        self.members(open + 1, self.partner(open), scope, output)?;
                        scope.truncate(restore);
                    }
                    None if self.text(self.first_keyword(index, header_end)) == "namespace" => {
                        scope.extend(name);
                    }
                    None => {}
                },
                Declares::Method(name) if member.body.is_some() || member.arrow => {
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
        scope.truncate(depth);
        Ok(())
    }

    fn first_keyword(&self, start: usize, end: usize) -> usize {
        (start..end)
            .find(|&i| {
                self.tokens[i].kind == Kind::Word
                    && matches!(
                        self.text(i),
                        "namespace" | "class" | "struct" | "interface" | "record" | "enum"
                    )
            })
            .unwrap_or(start)
    }

    /// What the member header `tokens[start..end]` declares.
    fn declares(&self, start: usize, end: usize) -> Declares {
        let mut index = self.skip_attributes(start, end);
        while index < end {
            match self.tokens[index].kind {
                Kind::Word
                    if matches!(
                        self.text(index),
                        "namespace" | "class" | "struct" | "interface" | "record" | "enum"
                    ) =>
                {
                    let name = self.dotted_name(index + 1, end);
                    return if name.is_empty() {
                        Declares::Other
                    } else {
                        Declares::Scope(name)
                    };
                }
                Kind::Punct('=') => return Declares::Other,
                Kind::Open('(') => return self.method_name(start, index),
                Kind::Open(_) => index = self.partner(index),
                _ => {}
            }
            index += 1;
        }
        Declares::Other
    }

    /// The name before the parameter list at `paren`: `Add`, `Max<T>`,
    /// `operator +`, or a constructor's type name.
    fn method_name(&self, start: usize, paren: usize) -> Declares {
        let mut name_end = paren;
        if name_end > start && self.tokens[name_end - 1].kind == Kind::Punct('>') {
            let mut depth = 0isize;
            while name_end > start {
                name_end -= 1;
                match self.tokens[name_end].kind {
                    Kind::Punct('>') => depth += 1,
                    Kind::Punct('<') => depth -= 1,
                    _ => {}
                }
                if depth == 0 {
                    break;
                }
            }
        }
        let mut symbols = name_end;
        while symbols > start && matches!(self.tokens[symbols - 1].kind, Kind::Punct(_)) {
            symbols -= 1;
        }
        if symbols > start && symbols < name_end && self.text(symbols - 1) == "operator" {
            let operator = (symbols..name_end)
                .map(|i| self.text(i))
                .collect::<String>();
            return Declares::Method(format!("operator{operator}"));
        }
        let Some(name) = name_end.checked_sub(1).filter(|&n| n >= start) else {
            return Declares::Other;
        };
        if self.tokens[name].kind != Kind::Word || self.text(name) == "new" {
            return Declares::Other;
        }
        Declares::Method(self.text(name).to_owned())
    }

    /// `A.B.C` starting at `index`, as segments.
    fn dotted_name(&self, mut index: usize, end: usize) -> Vec<String> {
        let mut segments = Vec::new();
        while index < end && self.tokens[index].kind == Kind::Word {
            segments.push(self.text(index).to_owned());
            if index + 2 < end && self.tokens[index + 1].kind == Kind::Punct('.') {
                index += 2;
            } else {
                break;
            }
        }
        segments
    }

    /// The first token after the attribute lists (`[Fact]`,
    /// `[Theory, InlineData(1, 2)]`) and trivia that begin a header.
    fn skip_attributes(&self, start: usize, end: usize) -> usize {
        let mut index = start;
        while index < end {
            match self.tokens[index].kind {
                Kind::Comment | Kind::Directive => index += 1,
                Kind::Open('[') => index = self.partner(index) + 1,
                _ => break,
            }
        }
        index
    }

    /// Every comment block that starts its own line and is followed, with
    /// no blank line between, by a member, together with that member's
    /// extent. Consecutive comments separated by at most one line break
    /// form one block.
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
    Scope(Vec<String>),
    Method(String),
    Other,
}

/// The lines of a `//`, `///`, or `/* */` comment with the comment markers
/// removed.
fn comment_lines(comment: &str) -> Vec<String> {
    if let Some(line) = comment.strip_prefix("//") {
        return vec![line.trim_start_matches('/').to_owned()];
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

    const SOURCE: &str = r#"using Xunit;

namespace Demo.Math;

#region helpers
public static class Calculator
{
    private const string Braces = @"{ ""not code"" }";
    private static readonly string Hole = $"{Braces} {{ }}";

    // @vtest.src-id SRC-CS-ADD
    public static int Add(int a, int b)
    {
        return a + b;
    }

    public static int Twice(int a) => a * 2;

    public static T Max<T>(T a, T b) where T : System.IComparable<T>
    {
        if (a.CompareTo(b) > 0) { return a; }
        return b;
    }

    public static int Count { get { return 1; } }

    public int Size { get; set; } = 5;

    public record Point(int X, int Y);
}
#endregion

public class CalculatorTests
{
    /// <summary>Adds.</summary>
    /// @vtest.id TEST-CS-ADD
    /// @vtest.covers VO-CS-ADD
    /// @vtest.target src/Calculator.cs::Demo::Math::Calculator::Add
    /// @vtest.intent adding two numbers returns their sum
    [Theory]
    [InlineData(1, 2, 3)]
    public void AddsNumbers(int a, int b, int expected)
    {
        Assert.Equal(expected, Calculator.Add(a, b));
    }
}
"#;

    /// @vtest.id TEST-ADAPTER-CSHARP-DISCOVERS-METHODS-AND-TESTS
    /// @vtest.covers VO-ADAPTER-CSHARP-REGISTERS-WITHOUT-CORE-CHANGES, VO-ADAPTER-LEXICAL-OPAQUE-LOCATOR-SYNTAX
    /// @vtest.target crates/vtest-adapter-lexical/src/csharp.rs::CSharp::extract
    /// @vtest.intent verifies C# block-bodied and expression-bodied methods become namespace- and class-qualified Source Targets while properties and positional records do not, and an attributed xUnit method is a Test construct including its attributes
    #[test]
    fn discovers_csharp_methods_and_attributed_tests() {
        let outcome = discover_text(&CSharp, "src/Calculator.cs", SOURCE, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        let sources = outcome
            .sources
            .iter()
            .map(|source| source.locator.value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            sources,
            vec![
                "src/Calculator.cs::Demo::Math::Calculator::Add",
                "src/Calculator.cs::Demo::Math::Calculator::Twice",
                "src/Calculator.cs::Demo::Math::Calculator::Max",
                "src/Calculator.cs::Demo::Math::CalculatorTests::AddsNumbers",
            ]
        );
        assert_eq!(
            outcome.sources[0].src_id.as_ref().map(|id| id.as_str()),
            Some("SRC-CS-ADD")
        );
        assert_eq!(outcome.tests.len(), 1);
        let test = &outcome.tests[0];
        assert_eq!(test.id.as_str(), "TEST-CS-ADD");
        assert!(test
            .construct_text
            .starts_with("[Theory]\n    [InlineData(1, 2, 3)]"));
        assert!(test
            .construct_text
            .ends_with("Calculator.Add(a, b));\n    }"));
    }

    fn only_test(source: &str) -> String {
        let outcome = discover_text(&CSharp, "src/T.cs", source, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(
            outcome.tests.len(),
            1,
            "{} missing",
            outcome.missing_tests.len()
        );
        outcome.tests[0].construct_text.clone()
    }

    /// @vtest.id TEST-ADAPTER-CSHARP-ATTRIBUTE-BRACES-ARE-NOT-THE-BODY
    /// @vtest.covers VO-ADAPTER-LEXICAL-RETURNS-SOURCE-RANGE-AND-BYTES
    /// @vtest.target crates/vtest-adapter-lexical/src/csharp.rs::Reader::member_end
    /// @vtest.intent verifies braces inside an attribute's arguments and inside an expression body's value do not end the Test construct, and a change to only the end of the body changes the construct text
    #[test]
    fn attribute_and_value_braces_are_not_the_body() {
        let declare = "class C\n{\n  // @vtest.id TEST-X\n  // @vtest.covers VO-X\n  // @vtest.intent check\n";
        let one = format!("{declare}  [Values(new int[]{{1,2}})] void Test(){{ Marker(); }}\n}}\n");
        let two = one.replace("Marker();", "Marker(); Other();");
        assert_eq!(
            only_test(&one),
            "[Values(new int[]{1,2})] void Test(){ Marker(); }"
        );
        assert_ne!(only_test(&one), only_test(&two));
        assert_eq!(
            only_test(&format!(
                "{declare}  int[] Make() => new[] {{ 1, 2 }};\n}}\n"
            )),
            "int[] Make() => new[] { 1, 2 };"
        );
    }
}
