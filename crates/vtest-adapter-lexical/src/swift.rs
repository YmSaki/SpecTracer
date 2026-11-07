//! `swift`: Swift sources (`.swift`).
//!
//! A file is read in three steps, all specific to Swift:
//!
//! 1. [`lex`] splits it into comments (nested `/* */` included), string
//!    literals (multi-line `"""`, raw `#"..."#`, and `\( ... )`
//!    interpolations included), words (`#if`, `$0`, and backtick names
//!    included), brackets, `;`, and operators (read by maximal munch, as
//!    Swift does).
//! 2. [`Reader::declaration_end`] finds where a declaration or statement
//!    ends. The `{}` body of a `func`, `init`, `deinit`, or type, and the
//!    accessor block of a property (`var x: Int { ... }`), end it.
//!    Otherwise it ends at a `;`, or at a newline unless the line is
//!    incomplete or the next line continues it. Swift tells this from
//!    whitespace: an operator with no space before it is postfix (`x!`,
//!    `Int?`), so a line ending with it is complete, while a line ending
//!    with a spaced operator, `,`, `:`, or an attribute (`@Test("...")`)
//!    is not. A next line starting with a spaced binary operator, a
//!    leading `.member`, `else`, or `catch` continues it, and so does a
//!    `{`, `->`, `where`, `throws`, or `async` while a `func` or type
//!    header still waits for its body. Brackets are skipped whole, so a
//!    default argument closure (`func f(x: () -> Int = { 1 })`) is never
//!    taken for the body: a function's body is the first `{` after its
//!    parameter list.
//! 3. [`Reader::members`] walks the members of the file: `class`,
//!    `struct`, `enum`, `actor`, `protocol`, and `extension` (named after
//!    the extended type) bodies qualify the `func` (operators included),
//!    `init`, and `deinit` declarations inside them. A requirement without
//!    a body is not a Source Target. `#if`/`#else`/`#endif` lines are
//!    read as lines of their own, so the members between them are read
//!    normally.
//!
//! A comment block attached to a declaration or statement makes it a Test
//! (or declares its SRC ID); its attributes are part of the construct.

use crate::{
    shared::{attached_comments, pair_brackets, Attached, Span},
    Annotated, ExtractError, FileItems, Function, LanguageProfile, LexicalAdapter,
};

pub const ADAPTER_ID: &str = "swift";

#[derive(Clone, Copy, Debug, Default)]
pub struct Swift;

impl LanguageProfile for Swift {
    fn adapter_id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn languages(&self) -> &'static [&'static str] {
        &["swift"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["swift"]
    }

    fn excluded_dirs(&self) -> &'static [&'static str] {
        &[".build", "DerivedData", "Pods", "Carthage"]
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

/// The `swift` adapter.
pub fn adapter() -> LexicalAdapter<Swift> {
    LexicalAdapter::new(Swift)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    Comment,
    Literal,
    Word,
    Open(char),
    Close(char),
    Semi,
    /// An operator (`==`, `->`, `?`, `..<`).
    Operator,
    /// `.`, `,`, `:`, `@`, or another single punctuation character.
    Punct,
}

#[derive(Clone, Copy, Debug)]
struct Tok {
    kind: Kind,
    start: usize,
    end: usize,
}

fn is_operator_char(byte: u8) -> bool {
    matches!(
        byte,
        b'/' | b'='
            | b'-'
            | b'+'
            | b'!'
            | b'*'
            | b'%'
            | b'<'
            | b'>'
            | b'&'
            | b'|'
            | b'^'
            | b'~'
            | b'?'
    )
}

fn is_word_char(byte: u8) -> bool {
    byte == b'_' || byte == b'$' || byte.is_ascii_alphanumeric() || !byte.is_ascii()
}

/// Splits Swift source into tokens. An unterminated comment or literal is
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
        } else if byte == b'"' || (byte == b'#' && raw_string_quote(bytes, i).is_some()) {
            i = string_end(bytes, i)
                .ok_or_else(|| ExtractError::new(start, "unterminated string"))?;
            Kind::Literal
        } else if let Some(body) = rest.strip_prefix("#/") {
            let close = body
                .find("/#")
                .ok_or_else(|| ExtractError::new(start, "unterminated regex literal"))?;
            i += close + 4;
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
                && (is_word_char(bytes[i])
                    || (bytes[i] == b'.' && bytes.get(i + 1).is_some_and(u8::is_ascii_digit)))
            {
                i += 1;
            }
            Kind::Literal
        } else if is_word_char(byte)
            || (byte == b'#' && bytes.get(i + 1).is_some_and(|b| b.is_ascii_alphabetic()))
        {
            i += 1;
            while i < bytes.len() && is_word_char(bytes[i]) {
                i += 1;
            }
            Kind::Word
        } else if is_operator_char(byte) || (byte == b'.' && bytes.get(i + 1) == Some(&b'.')) {
            let dots = byte == b'.';
            i += 1;
            while i < bytes.len()
                && (is_operator_char(bytes[i]) || (dots && bytes[i] == b'.'))
                && !source[i..].starts_with("//")
                && !source[i..].starts_with("/*")
            {
                i += 1;
            }
            Kind::Operator
        } else {
            i += 1;
            match byte {
                b'(' | b'[' | b'{' => Kind::Open(byte as char),
                b')' | b']' | b'}' => Kind::Close(byte as char),
                b';' => Kind::Semi,
                _ => Kind::Punct,
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

/// For `#"`, `##"""`, ...: the index of the first `"` after the `#`s.
fn raw_string_quote(bytes: &[u8], i: usize) -> Option<usize> {
    let mut j = i;
    while bytes.get(j) == Some(&b'#') {
        j += 1;
    }
    (bytes.get(j) == Some(&b'"')).then_some(j)
}

/// One past the end of the string literal opening at `i` (its `#`
/// delimiters included). `\( ... )` interpolations are read as code.
fn string_end(bytes: &[u8], i: usize) -> Option<usize> {
    let quote = raw_string_quote(bytes, i)?;
    let hashes = quote - i;
    let multi = bytes[quote..].starts_with(b"\"\"\"");
    let delimiter = if multi { 3 } else { 1 };
    let mut j = quote + delimiter;
    let closes = |j: usize| {
        bytes[j..].len() >= delimiter + hashes
            && bytes[j..j + delimiter].iter().all(|&b| b == b'"')
            && bytes[j + delimiter..j + delimiter + hashes]
                .iter()
                .all(|&b| b == b'#')
    };
    while j < bytes.len() {
        if closes(j) {
            return Some(j + delimiter + hashes);
        }
        match bytes[j] {
            b'\n' if !multi => return None,
            b'\\'
                if bytes[j + 1..].iter().take(hashes).all(|&b| b == b'#')
                    && bytes.len() > j + 1 + hashes =>
            {
                let after = j + 1 + hashes;
                j = if bytes[after] == b'(' {
                    interpolation_end(bytes, after + 1)?
                } else {
                    after + 1
                };
            }
            _ => j += 1,
        }
    }
    None
}

/// One past the `)` closing an interpolation whose code starts at `i`.
fn interpolation_end(bytes: &[u8], mut i: usize) -> Option<usize> {
    let mut depth = 1usize;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => i = string_end(bytes, i)?,
            b'#' if raw_string_quote(bytes, i).is_some() => i = string_end(bytes, i)?,
            b'(' => {
                depth += 1;
                i += 1;
            }
            b')' => {
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

/// Keywords after which a declaration or statement cannot end.
const OPEN_KEYWORDS: &[&str] = &[
    "actor",
    "any",
    "as",
    "await",
    "case",
    "catch",
    "class",
    "do",
    "else",
    "enum",
    "extension",
    "for",
    "func",
    "guard",
    "if",
    "import",
    "in",
    "inout",
    "is",
    "let",
    "protocol",
    "repeat",
    "some",
    "struct",
    "switch",
    "try",
    "typealias",
    "var",
    "where",
    "while",
];

const TYPE_KEYWORDS: &[&str] = &["class", "struct", "enum", "actor", "protocol", "extension"];

/// The kind of declaration being read, as far as its end is concerned.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Shape {
    /// `func`, `init`, `deinit`, `subscript`, or a type: ends after its
    /// `{}` body.
    Bodied,
    /// `var` or `let` before any `=`: a `{` opens its accessor block.
    Property,
    /// Anything else: a `{` is a closure.
    Expression,
}

/// What a member declaration declares.
enum Declares {
    Scope(Vec<String>, usize),
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

    fn line_breaks(&self, before: usize, after: usize) -> usize {
        self.source[self.tokens[before].end..self.tokens[after].start]
            .matches('\n')
            .count()
    }

    /// Whether `tokens[index]` is a member name after `.` (`self.init`).
    fn after_dot(&self, index: usize) -> bool {
        index > 0 && self.is(index - 1, ".")
    }

    /// Whether a line ending with `tokens[last]` is complete.
    fn completes(&self, last: usize) -> bool {
        match self.tokens[last].kind {
            Kind::Literal | Kind::Close(_) => !self.ends_attribute(last),
            Kind::Word => {
                (!OPEN_KEYWORDS.contains(&self.text(last)) || self.after_dot(last))
                    && !self.ends_attribute(last)
            }
            // Postfix (`x!`, `Int?`, `Array<Int>`) when bound to the left.
            Kind::Operator => last > 0 && self.touching(last - 1, last),
            _ => false,
        }
    }

    /// Whether `tokens[last]` ends an attribute: `@Test`, `@MainActor`, or
    /// `@Test("adds")`.
    fn ends_attribute(&self, last: usize) -> bool {
        let mut name = last;
        if self.kind(last) == Some(Kind::Close(')')) {
            let open = self.partner(last);
            match open.checked_sub(1) {
                Some(before) if self.touching(before, open) => name = before,
                _ => return false,
            }
        }
        self.kind(name) == Some(Kind::Word)
            && name > 0
            && self.is(name - 1, "@")
            && self.touching(name - 1, name)
    }

    /// Whether a line starting with `tokens[next]` continues the previous
    /// one; `header` is whether a `func` or type header waits for its body.
    fn continues(&self, next: usize, header: bool) -> bool {
        match self.tokens[next].kind {
            Kind::Operator => {
                // A binary operator has whitespace on both sides; one bound
                // to the right is a prefix operator starting a new line.
                self.text(next) == "->"
                    || self
                        .next_significant(next + 1)
                        .is_none_or(|after| !self.touching(next, after))
            }
            Kind::Punct => self.text(next) == ".",
            Kind::Open('{') => header,
            Kind::Word => match self.text(next) {
                "else" | "catch" => true,
                "where" | "throws" | "rethrows" | "async" => header,
                _ => false,
            },
            _ => false,
        }
    }

    /// One past the last token of the declaration or statement starting at
    /// `start` (see the module documentation). A `}` or `)` closing the
    /// enclosing block also ends it.
    fn declaration_end(&self, start: usize) -> usize {
        if self.is_directive(start) {
            let mut index = start;
            while let Some(next) = self.next_significant(index + 1) {
                if self.line_breaks(index, next) > 0 {
                    break;
                }
                index = next;
            }
            return index + 1;
        }
        let mut shape = Shape::Expression;
        let mut index = start;
        while index < self.tokens.len() {
            let last = match self.tokens[index].kind {
                Kind::Comment => {
                    index += 1;
                    continue;
                }
                Kind::Close(_) => return index,
                Kind::Semi => return index + 1,
                Kind::Open('{') if shape != Shape::Expression => return self.partner(index) + 1,
                Kind::Open(_) => self.partner(index),
                Kind::Word if shape == Shape::Expression && !self.after_dot(index) => {
                    match self.text(index) {
                        "func" | "init" | "deinit" | "subscript" => shape = Shape::Bodied,
                        "var" | "let" => shape = Shape::Property,
                        word if self.is_type_keyword(index, word) => shape = Shape::Bodied,
                        _ => {}
                    }
                    index
                }
                Kind::Operator if self.text(index) == "=" && shape == Shape::Property => {
                    shape = Shape::Expression;
                    index
                }
                _ => index,
            };
            let Some(next) = self.next_significant(last + 1) else {
                return last + 1;
            };
            if self.line_breaks(last, next) > 0
                && self.completes(last)
                && !self.continues(next, shape == Shape::Bodied)
            {
                return last + 1;
            }
            index = next;
        }
        index
    }

    /// Whether `word` at `index` declares a type (`class Calc`, not the
    /// `class func` modifier).
    fn is_type_keyword(&self, index: usize, word: &str) -> bool {
        TYPE_KEYWORDS.contains(&word)
            && !(word == "class"
                && self.next_significant(index + 1).is_some_and(|next| {
                    matches!(
                        self.text(next),
                        "func" | "var" | "let" | "subscript" | "override" | "final" | "static"
                    )
                }))
    }

    /// Whether `tokens[index]` is a conditional compilation line.
    fn is_directive(&self, index: usize) -> bool {
        self.kind(index) == Some(Kind::Word)
            && matches!(self.text(index), "#if" | "#elseif" | "#else" | "#endif")
    }

    /// Collects the function definitions among the declarations in
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
                Declares::Scope(name, open) => {
                    let restore = scope.len();
                    scope.extend(name);
                    self.members(open + 1, self.partner(open), scope, output);
                    scope.truncate(restore);
                }
                Declares::Function(name) => {
                    let mut qualified = scope.clone();
                    qualified.push(name);
                    output.push(Function {
                        name: qualified,
                        start: self.tokens[start].start,
                        end: self.tokens[end - 1].end,
                    });
                }
                Declares::Other => {}
            }
            index = end;
        }
    }

    /// What `tokens[start..end]` declares.
    fn declares(&self, start: usize, end: usize) -> Declares {
        if self.is_directive(start) {
            return Declares::Other;
        }
        let mut index = start;
        while index < end {
            match self.tokens[index].kind {
                Kind::Open(_) => index = self.partner(index),
                Kind::Operator if self.text(index) == "=" => break,
                Kind::Word if !self.after_dot(index) => match self.text(index) {
                    "func" => {
                        let name = index + 1;
                        return match self.kind(name) {
                            Some(Kind::Word | Kind::Operator) => self.function(name, end),
                            _ => Declares::Other,
                        };
                    }
                    "init" | "deinit" => return self.function(index, end),
                    word if self.is_type_keyword(index, word) => return self.scope(index, end),
                    "var" | "let" | "case" | "typealias" | "import" | "subscript" => break,
                    _ => {}
                },
                _ => {}
            }
            index += 1;
        }
        Declares::Other
    }

    /// The function named by `tokens[name]` if it has a body: the first
    /// `{` after its parameter list (`deinit` has none).
    fn function(&self, name: usize, end: usize) -> Declares {
        let mut index = name + 1;
        let mut params_done = self.text(name) == "deinit";
        while index < end {
            match self.tokens[index].kind {
                Kind::Open('(') if !params_done => {
                    index = self.partner(index);
                    params_done = true;
                }
                Kind::Open('{') if params_done => {
                    return Declares::Function(self.text(name).trim_matches('`').to_owned());
                }
                Kind::Open(_) => index = self.partner(index),
                _ => {}
            }
            index += 1;
        }
        Declares::Other
    }

    /// The type declared by the keyword at `keyword`, with its body.
    fn scope(&self, keyword: usize, end: usize) -> Declares {
        let mut index = keyword + 1;
        let mut name = Vec::new();
        while self.kind(index) == Some(Kind::Word) {
            name.push(self.text(index).trim_matches('`').to_owned());
            if self.is(index + 1, ".") && self.kind(index + 2) == Some(Kind::Word) {
                index += 2;
            } else {
                break;
            }
        }
        if name.is_empty() {
            return Declares::Other;
        }
        while index < end {
            match self.tokens[index].kind {
                Kind::Open('{') => return Declares::Scope(name, index),
                Kind::Open(_) => index = self.partner(index),
                _ => {}
            }
            index += 1;
        }
        Declares::Other
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

/// The lines of a `//` (or `///`) or `/* */` comment with the comment
/// markers removed.
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

    const SOURCE: &str = r#"import Testing

/* outer /* nested */ still a comment { */
public struct Calc {
    let base: Int

    init(base: Int = 0) { self.base = base }

    // @vtest.src-id SRC-SWIFT-ADD
    public func add(_ a: Int, _ b: Int) -> Int {
        return base + a + b
    }

    func max<T: Comparable>(_ a: T, _ b: T) -> T {
        if a > b { return a }
        return b
    }

    var doubled: Int { base * 2 }

    static func == (lhs: Calc, rhs: Calc) -> Bool { lhs.base == rhs.base }
}

extension Calc: CustomStringConvertible {
    var description: String { "Calc(\(base))" }
    func reset() -> Calc { Calc() }
}

#if DEBUG
let label = """
{ not code
"""
#endif

let handler = { (x: Int) -> Int in x }

// @vtest.id TEST-SWIFT-ADD
// @vtest.covers VO-SWIFT-ADD
// @vtest.target Sources/Calc.swift::Calc::add
// @vtest.intent adding two numbers returns their sum
@Test("adds two numbers")
func adds() {
    #expect(Calc().add(1, 2) == 3)
}
"#;

    /// @vtest.id TEST-ADAPTER-SWIFT-DISCOVERS-FUNCS-AND-TESTS
    /// @vtest.covers VO-ADAPTER-SWIFT-REGISTERS-WITHOUT-CORE-CHANGES, VO-ADAPTER-LEXICAL-OPAQUE-LOCATOR-SYNTAX
    /// @vtest.target crates/vtest-adapter-lexical/src/swift.rs::Swift::extract
    /// @vtest.intent verifies Swift funcs, generic funcs, operators, and initializers become type- and extension-qualified Source Targets while computed properties and closures do not, and an annotated Swift Testing function is a Test construct including its attribute
    #[test]
    fn discovers_swift_funcs_and_tests() {
        let outcome = discover_text(&Swift, "Sources/Calc.swift", SOURCE, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        let sources = outcome
            .sources
            .iter()
            .map(|source| source.locator.value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            sources,
            vec![
                "Sources/Calc.swift::Calc::init",
                "Sources/Calc.swift::Calc::add",
                "Sources/Calc.swift::Calc::max",
                "Sources/Calc.swift::Calc::==",
                "Sources/Calc.swift::Calc::reset",
                "Sources/Calc.swift::adds",
            ]
        );
        assert_eq!(
            outcome.sources[1].src_id.as_ref().map(|id| id.as_str()),
            Some("SRC-SWIFT-ADD")
        );
        assert_eq!(outcome.tests.len(), 1);
        let test = &outcome.tests[0];
        assert_eq!(test.id.as_str(), "TEST-SWIFT-ADD");
        assert_eq!(test.location.locator, "adds");
        assert!(test
            .construct_text
            .starts_with("@Test(\"adds two numbers\")\nfunc adds() {"));
        assert!(test.construct_text.ends_with("== 3)\n}"));
    }

    /// @vtest.id TEST-ADAPTER-SWIFT-DEFAULT-CLOSURE-IS-NOT-THE-BODY
    /// @vtest.covers VO-ADAPTER-SWIFT-REGISTERS-WITHOUT-CORE-CHANGES
    /// @vtest.target crates/vtest-adapter-lexical/src/swift.rs::Reader::declaration_end
    /// @vtest.intent verifies that a closure in a default argument or an attribute argument is not taken for the function body, so a Test construct runs to the end of the body and an edit to only the body's tail changes its text, and that a protocol requirement without a body ends at its line
    #[test]
    fn default_argument_closure_is_not_the_body() {
        let source = |tail: &str| {
            format!(
                "protocol Maker {{\n    func make() -> Int\n    func other() throws\n}}\n\n// @vtest.id TEST-SWIFT-MAKE\n// @vtest.covers VO-SWIFT-ADD\n// @vtest.intent the whole body is the construct\n@Test(arguments: [1, 2].map {{ $0 * 2 }})\nfunc makes(value: Int, make: () -> Int = {{ 42 }}) async throws\n    -> Int\n{{\n    marker()\n    {tail}\n    return value\n        + make()\n}}\n\nfunc after() {{}}\n"
            )
        };
        let outcome = discover_text(&Swift, "Tests/MakeTests.swift", &source("one()"), "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        let sources = outcome
            .sources
            .iter()
            .map(|source| source.locator.value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            sources,
            vec![
                "Tests/MakeTests.swift::makes",
                "Tests/MakeTests.swift::after"
            ]
        );
        let test = &outcome.tests[0];
        assert!(test.construct_text.starts_with("@Test(arguments:"));
        assert!(test.construct_text.ends_with("+ make()\n}"));
        let edited = discover_text(&Swift, "Tests/MakeTests.swift", &source("two()"), "demo");
        assert_ne!(edited.tests[0].construct_text, test.construct_text);
    }

    /// @vtest.id TEST-ADAPTER-SWIFT-STRINGS-AND-UNREADABLE-FILES
    /// @vtest.covers VO-ADAPTER-SWIFT-REGISTERS-WITHOUT-CORE-CHANGES
    /// @vtest.target crates/vtest-adapter-lexical/src/swift.rs::string_end
    /// @vtest.intent verifies that brackets inside raw strings and interpolations are not code, that an annotated Quick-style statement is a Test construct running to the end of its trailing closure, and that an unterminated multi-line string makes the file unreadable
    #[test]
    fn strings_and_unreadable_files() {
        let source = "class CalcSpec: QuickSpec {\n    override class func spec() {\n        // @vtest.id TEST-SWIFT-SPEC\n        // @vtest.covers VO-SWIFT-ADD\n        // @vtest.intent a Quick example is a construct\n        it(#\"adds \"{\"#) {\n            expect(\"\\(f(\")\"))\").to(equal(1))\n        }\n    }\n}\n";
        let outcome = discover_text(&Swift, "Tests/CalcSpec.swift", source, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(
            outcome
                .sources
                .iter()
                .map(|source| source.locator.value.as_str())
                .collect::<Vec<_>>(),
            vec!["Tests/CalcSpec.swift::CalcSpec::spec"]
        );
        assert_eq!(outcome.tests.len(), 1);
        assert!(outcome.tests[0].construct_text.starts_with("it(#\"adds"));
        assert!(outcome.tests[0]
            .construct_text
            .ends_with("to(equal(1))\n        }"));

        let broken = discover_text(
            &Swift,
            "Sources/Broken.swift",
            "let x = \"\"\"\nopen\n",
            "demo",
        );
        assert_eq!(broken.diagnostics.len(), 1);
        assert_eq!(broken.diagnostics[0].code.as_str(), "E-SCAN-001");
    }
}
