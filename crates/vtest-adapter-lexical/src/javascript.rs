//! `javascript`: JavaScript and TypeScript sources (`.js`, `.jsx`, `.mjs`,
//! `.cjs`, `.ts`, `.tsx`, `.mts`, `.cts`).
//!
//! A file is read in three steps, all specific to JavaScript and
//! TypeScript:
//!
//! 1. [`Lexer`] splits it into comments, literals, words (`#private`
//!    names included), brackets, `;`, and operators. Template literals
//!    (`${ ... }` read as code), regular expressions (a `/` where an
//!    expression can start, as JavaScript decides it), and JSX elements
//!    (`<Name ...>` where an expression can start) are one literal each, so
//!    the braces and quotes inside them are not code.
//! 2. [`Reader::statement_end`] and [`Reader::member_end`] find where a
//!    statement or class member ends. A `function`, `class`, `namespace`,
//!    `interface`, `enum`, method, or `if`/`for`/`while`/`switch`/`try`
//!    statement ends after its `{}` body (an `else`, `catch`, or `finally`
//!    continues it). Its body is the first `{` that can start one: a `{`
//!    after `:`, `|`, `&`, `=>`, `<`, `,`, `=`, or `extends` is a
//!    TypeScript object type (`function f(): { x: number } {`). Anything
//!    else ends at a `;` or where automatic semicolon insertion ends it:
//!    at a newline after a token that can end an expression, unless the
//!    next line starts with a token that continues it (`.`, `(`, `[`, a
//!    template, or a binary operator). A newline never ends a decorator
//!    (`@memoize()`). Brackets are skipped whole.
//! 3. [`Reader::statements`] and [`Reader::members`] walk the file:
//!    `class` bodies and TypeScript `namespace`/`module` bodies qualify the
//!    `function` declarations, methods (getters and setters included), and
//!    variables or class fields initialized with an arrow function or a
//!    function expression (`const add = (a, b) => a + b`) inside them.
//!    Function bodies, object literals, and calls are not walked.
//!
//! A comment block attached to a statement or member makes it a Test (or
//! declares its SRC ID); its decorators are part of the construct. Jest,
//! Vitest, Mocha, Jasmine, `node:test`, or Playwright calls such as
//! `test("adds", () => { ... })` are Test constructs this way.

use std::collections::HashSet;

use crate::{
    shared::{attached_comments, pair_brackets, Attached, Span},
    Annotated, ExtractError, FileItems, Function, LanguageProfile, LexicalAdapter,
};

pub const ADAPTER_ID: &str = "javascript";

#[derive(Clone, Copy, Debug, Default)]
pub struct JavaScript;

impl LanguageProfile for JavaScript {
    fn adapter_id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn languages(&self) -> &'static [&'static str] {
        &["javascript", "typescript"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["js", "jsx", "mjs", "cjs", "ts", "tsx", "mts", "cts"]
    }

    fn excluded_dirs(&self) -> &'static [&'static str] {
        &["node_modules", "dist", "build", "coverage", ".next"]
    }

    fn extract(&self, source: &str) -> Result<FileItems, ExtractError> {
        let lexer = Lexer {
            source,
            bytes: source.as_bytes(),
        };
        let (tokens, _) = lexer.run(0, false)?;
        let partners = pair_brackets(tokens.iter().map(|token| match token.kind {
            Kind::Open(c) | Kind::Close(c) => (token.start, Some(c)),
            _ => (token.start, None),
        }))?;
        let enclosing = enclosing(&tokens, &partners);
        let reader = Reader {
            source,
            tokens: &tokens,
            partners: &partners,
            enclosing: &enclosing,
        };
        if let Some(last) = reader.dangling_prefix() {
            return Err(ExtractError::new(
                tokens[last].start,
                "the file ends before the declaration this prefix belongs to",
            ));
        }
        let mut functions = Vec::new();
        let mut class_bodies = HashSet::new();
        reader.statements(
            0,
            tokens.len(),
            &mut Vec::new(),
            &mut functions,
            &mut class_bodies,
        );
        Ok(FileItems {
            functions,
            annotated: reader.annotated(&class_bodies),
        })
    }
}

/// The `javascript` adapter.
pub fn adapter() -> LexicalAdapter<JavaScript> {
    LexicalAdapter::new(JavaScript)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    Comment,
    Literal,
    Word,
    Open(char),
    Close(char),
    Semi,
    /// An operator or punctuation (`=>`, `?.`, `,`, `:`, `@`); its text
    /// tells which.
    Op,
}

#[derive(Clone, Copy, Debug)]
struct Tok {
    kind: Kind,
    start: usize,
    end: usize,
}

/// Operators read as one token, longest first. `>>` is read as two `>`
/// so that nested type arguments (`Array<Array<T>>`) close one by one.
const OPERATORS: &[&str] = &[
    "===", "!==", "**=", "<<=", "&&=", "||=", "??=", "...", "=>", "==", "!=", "<=", ">=", "&&",
    "||", "??", "?.", "++", "--", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "**", "<<",
];

/// Words after which a `/` starts a regular expression and a `<` may
/// start a JSX element.
const EXPRESSION_KEYWORDS: &[&str] = &[
    "return",
    "typeof",
    "instanceof",
    "in",
    "of",
    "new",
    "delete",
    "void",
    "throw",
    "case",
    "do",
    "else",
    "yield",
    "await",
    "extends",
];

fn is_word_start(byte: u8) -> bool {
    byte == b'_' || byte == b'$' || byte.is_ascii_alphabetic() || !byte.is_ascii()
}

fn is_word_char(byte: u8) -> bool {
    is_word_start(byte) || byte.is_ascii_digit()
}

struct Lexer<'a> {
    source: &'a str,
    bytes: &'a [u8],
}

impl Lexer<'_> {
    /// Tokens from `i` to the end of the file, or (when `stop`) to the `}`
    /// closing a `${` or JSX `{` expression, returning one past it.
    fn run(&self, mut i: usize, stop: bool) -> Result<(Vec<Tok>, usize), ExtractError> {
        let bytes = self.bytes;
        let mut tokens: Vec<Tok> = Vec::new();
        let mut depth = 0usize;
        while i < bytes.len() {
            let start = i;
            let rest = &self.source[i..];
            let byte = bytes[i];
            let kind = if byte.is_ascii_whitespace() {
                i += 1;
                continue;
            } else if rest.starts_with("//") || (i == 0 && rest.starts_with("#!")) {
                i += rest.find('\n').unwrap_or(rest.len());
                Kind::Comment
            } else if let Some(body) = rest.strip_prefix("/*") {
                let close = body
                    .find("*/")
                    .ok_or_else(|| ExtractError::new(start, "unterminated comment"))?;
                i += close + 4;
                Kind::Comment
            } else if byte == b'"' || byte == b'\'' {
                i = self
                    .quoted_end(i + 1, byte)
                    .ok_or_else(|| ExtractError::new(start, "unterminated string"))?;
                Kind::Literal
            } else if byte == b'`' {
                i = self.template_end(i + 1)?;
                Kind::Literal
            } else if byte.is_ascii_digit()
                || (byte == b'.' && bytes.get(i + 1).is_some_and(u8::is_ascii_digit))
            {
                i += 1;
                while i < bytes.len()
                    && (is_word_char(bytes[i])
                        || (bytes[i] == b'.' && bytes.get(i + 1).is_some_and(u8::is_ascii_digit)))
                {
                    i += 1;
                }
                Kind::Literal
            } else if is_word_start(byte)
                || (byte == b'#' && bytes.get(i + 1).copied().is_some_and(is_word_start))
            {
                i += 1;
                while i < bytes.len() && is_word_char(bytes[i]) {
                    i += 1;
                }
                Kind::Word
            } else if let Some(end) = (byte == b'/' && self.expression_position(&tokens))
                .then(|| self.regex_end(i))
                .flatten()
            {
                i = end;
                Kind::Literal
            } else if let Some(end) = (byte == b'<' && self.expression_position(&tokens))
                .then(|| self.jsx_end(i))
                .flatten()
            {
                i = end;
                Kind::Literal
            } else {
                match byte {
                    b'{' => {
                        depth += 1;
                        i += 1;
                        Kind::Open('{')
                    }
                    b'}' if stop && depth == 0 => return Ok((tokens, i + 1)),
                    b'}' => {
                        depth = depth.saturating_sub(1);
                        i += 1;
                        Kind::Close('}')
                    }
                    b'(' | b'[' => {
                        i += 1;
                        Kind::Open(byte as char)
                    }
                    b')' | b']' => {
                        i += 1;
                        Kind::Close(byte as char)
                    }
                    b';' => {
                        i += 1;
                        Kind::Semi
                    }
                    _ => {
                        let operator = OPERATORS.iter().find(|op| {
                            rest.starts_with(**op)
                                && !(**op == "?."
                                    && bytes.get(i + 2).is_some_and(u8::is_ascii_digit))
                        });
                        i += operator.map_or(1, |op| op.len());
                        Kind::Op
                    }
                }
            };
            tokens.push(Tok {
                kind,
                start,
                end: i,
            });
        }
        if stop {
            return Err(ExtractError::new(
                i,
                "unterminated template or JSX expression",
            ));
        }
        Ok((tokens, i))
    }

    fn text(&self, token: &Tok) -> &str {
        &self.source[token.start..token.end]
    }

    /// Whether an expression can start after the tokens read so far, so
    /// that a `/` starts a regular expression and a `<` a JSX element.
    fn expression_position(&self, tokens: &[Tok]) -> bool {
        let Some(previous) = tokens.iter().rev().find(|t| t.kind != Kind::Comment) else {
            return true;
        };
        match previous.kind {
            Kind::Op => !matches!(self.text(previous), "++" | "--"),
            Kind::Open(_) | Kind::Semi | Kind::Close('}') => true,
            Kind::Word => EXPRESSION_KEYWORDS.contains(&self.text(previous)),
            _ => false,
        }
    }

    /// One past the closing quote of a string whose contents start at `i`.
    fn quoted_end(&self, mut i: usize, quote: u8) -> Option<usize> {
        while i < self.bytes.len() {
            match self.bytes[i] {
                b'\\' => i += 2,
                b'\n' => return None,
                byte if byte == quote => return Some(i + 1),
                _ => i += 1,
            }
        }
        None
    }

    /// One past the closing `` ` `` of a template whose contents start at
    /// `i`; `${ ... }` is read as code.
    fn template_end(&self, mut i: usize) -> Result<usize, ExtractError> {
        let start = i - 1;
        while i < self.bytes.len() {
            match self.bytes[i] {
                b'\\' => i += 2,
                b'`' => return Ok(i + 1),
                b'$' if self.bytes.get(i + 1) == Some(&b'{') => i = self.run(i + 2, true)?.1,
                _ => i += 1,
            }
        }
        Err(ExtractError::new(start, "unterminated template literal"))
    }

    /// One past a regular expression literal at `i`, or `None` when the
    /// `/` cannot start one on this line.
    fn regex_end(&self, i: usize) -> Option<usize> {
        let mut j = i + 1;
        if matches!(self.bytes.get(j), Some(b'/' | b'*')) {
            return None;
        }
        let mut class = false;
        while j < self.bytes.len() {
            match self.bytes[j] {
                b'\\' => j += 2,
                b'\n' => return None,
                b'[' => {
                    class = true;
                    j += 1;
                }
                b']' => {
                    class = false;
                    j += 1;
                }
                b'/' if !class => {
                    j += 1;
                    while j < self.bytes.len() && self.bytes[j].is_ascii_alphanumeric() {
                        j += 1;
                    }
                    return Some(j);
                }
                _ => j += 1,
            }
        }
        None
    }

    /// One past the JSX element starting at the `<` at `i`, or `None` when
    /// it is not one (a comparison, or TypeScript type parameters such as
    /// `<T,>(x: T) => x`).
    fn jsx_end(&self, i: usize) -> Option<usize> {
        let after = *self.bytes.get(i + 1)?;
        if !(after == b'>' || is_word_start(after)) {
            return None;
        }
        if is_word_start(after) {
            let mut j = i + 1;
            while j < self.bytes.len() && is_word_char(self.bytes[j]) {
                j += 1;
            }
            let rest = self.source[j..].trim_start();
            let generic = rest.starts_with(',')
                || rest.starts_with("extends ")
                || (rest.starts_with('>') && rest[1..].trim_start().starts_with('('));
            if generic {
                return None;
            }
        }
        let mut depth = 0usize;
        let mut j = i;
        loop {
            // At a tag.
            let (end, opens, closes) = self.jsx_tag(j)?;
            if opens {
                depth += 1;
            }
            if closes {
                depth = depth.checked_sub(1)?;
            }
            j = end;
            if depth == 0 {
                return Some(j);
            }
            // Children: text and `{ ... }` expressions up to the next tag.
            loop {
                match self.bytes.get(j)? {
                    b'<' => break,
                    b'{' => j = self.run(j + 1, true).ok()?.1,
                    _ => j += 1,
                }
            }
        }
    }

    /// The JSX tag at the `<` at `i`: one past its `>`, whether it opens an
    /// element, and whether it closes one (`</a>` closes, `<a/>` does
    /// neither).
    fn jsx_tag(&self, i: usize) -> Option<(usize, bool, bool)> {
        let closing = self.bytes.get(i + 1) == Some(&b'/');
        let mut j = i + 1;
        while j < self.bytes.len() {
            match self.bytes[j] {
                b'>' => return Some((j + 1, !closing, closing)),
                b'/' if self.bytes.get(j + 1) == Some(&b'>') && !closing => {
                    return Some((j + 2, false, false))
                }
                b'"' | b'\'' => {
                    let quote = self.bytes[j];
                    j += 1 + self.source[j + 1..].find(quote as char)? + 1;
                }
                b'{' => j = self.run(j + 1, true).ok()?.1,
                b'<' => return None,
                _ => j += 1,
            }
        }
        None
    }
}

/// For each token, the index of the innermost bracket enclosing it (a
/// class body, for its members).
fn enclosing(tokens: &[Tok], partners: &[Option<usize>]) -> Vec<Option<usize>> {
    let mut enclosing = vec![None; tokens.len()];
    let mut stack: Vec<usize> = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if let Kind::Close(_) = token.kind {
            stack.pop();
        }
        enclosing[index] = stack.last().copied();
        if let (Kind::Open(_), Some(_)) = (token.kind, partners[index]) {
            stack.push(index);
        }
    }
    enclosing
}

/// Words after which an expression cannot end.
const OPEN_KEYWORDS: &[&str] = &[
    "function",
    "class",
    "const",
    "let",
    "var",
    "new",
    "typeof",
    "delete",
    "await",
    "in",
    "instanceof",
    "if",
    "else",
    "for",
    "while",
    "do",
    "try",
    "catch",
    "finally",
    "switch",
    "case",
    "throw",
    "extends",
    "implements",
    "export",
    "import",
];

/// Words after which a `{` is a TypeScript type, not a body.
const TYPE_OPERATORS: &[&str] = &[
    "extends",
    "implements",
    "keyof",
    "typeof",
    "infer",
    "is",
    "asserts",
    "readonly",
    "unique",
    "new",
    "as",
    "satisfies",
];

/// Class member modifiers (when another name follows).
const MODIFIERS: &[&str] = &[
    "public",
    "private",
    "protected",
    "static",
    "readonly",
    "abstract",
    "override",
    "declare",
    "accessor",
    "async",
    "get",
    "set",
];

/// What a statement or member declares.
enum Declares {
    Class(String, usize),
    Namespace(Vec<String>, usize),
    Function(String),
    /// The declarators of a `const`/`let`/`var` statement whose values are
    /// functions: each name, with its first and one-past-last token.
    Bindings(Vec<(String, usize, usize)>),
    Other,
}

struct Reader<'a> {
    source: &'a str,
    tokens: &'a [Tok],
    partners: &'a [Option<usize>],
    enclosing: &'a [Option<usize>],
}

impl Reader<'_> {
    /// The text of `tokens[index]`, or `""` past the last token (a file
    /// may end in the middle of a declaration, such as a lone `export`).
    fn text(&self, index: usize) -> &str {
        self.tokens
            .get(index)
            .map_or("", |token| &self.source[token.start..token.end])
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

    /// A declaration prefix (`export`, `default`, `declare`, `abstract`,
    /// or a decorator) with nothing after it at the end of the file.
    fn dangling_prefix(&self) -> Option<usize> {
        let last = (0..self.tokens.len())
            .rev()
            .find(|&i| self.tokens[i].kind != Kind::Comment)?;
        if self.kind(last) == Some(Kind::Word)
            && matches!(
                self.text(last),
                "export" | "default" | "declare" | "abstract"
            )
            && !self.after_dot(last)
        {
            return Some(last);
        }
        (0..=last)
            .rev()
            .take_while(|&i| !matches!(self.kind(i), Some(Kind::Semi | Kind::Close('}'))))
            .find(|&i| self.is(i, "@") && self.skip_decorators(i) >= self.tokens.len())
    }

    fn next_significant(&self, from: usize) -> Option<usize> {
        (from..self.tokens.len()).find(|&i| self.tokens[i].kind != Kind::Comment)
    }

    fn previous_significant(&self, low: usize, index: usize) -> Option<usize> {
        (low..index)
            .rev()
            .find(|&i| self.tokens[i].kind != Kind::Comment)
    }

    fn line_breaks(&self, before: usize, after: usize) -> usize {
        self.source[self.tokens[before].end..self.tokens[after].start]
            .matches('\n')
            .count()
    }

    fn after_dot(&self, index: usize) -> bool {
        self.previous_significant(0, index)
            .is_some_and(|p| matches!(self.text(p), "." | "?."))
    }

    /// Whether an expression can end with `tokens[last]`.
    fn can_end(&self, last: usize) -> bool {
        match self.tokens[last].kind {
            Kind::Literal => true,
            Kind::Word => !OPEN_KEYWORDS.contains(&self.text(last)) || self.after_dot(last),
            Kind::Close(')') => {
                let open = self.partner(last);
                !self.previous_significant(0, open).is_some_and(|keyword| {
                    matches!(self.text(keyword), "if" | "for" | "while" | "with")
                        && !self.after_dot(keyword)
                        && !(self.text(keyword) == "while"
                            && self
                                .previous_significant(0, keyword)
                                .is_some_and(|p| self.is(p, "}")))
                })
            }
            Kind::Close(_) => true,
            Kind::Op => {
                matches!(self.text(last), "++" | "--" | "!" | ">")
                    && last > 0
                    && self.touching(last - 1, last)
            }
            _ => false,
        }
    }

    /// Whether a line starting with `tokens[next]` continues the previous
    /// one; `header` is whether a declaration still waits for its body.
    fn continues(&self, next: usize, header: bool) -> bool {
        match self.tokens[next].kind {
            Kind::Open('(' | '[') => true,
            Kind::Open(_) => header,
            Kind::Literal => self.text(next).starts_with('`'),
            Kind::Op => !matches!(self.text(next), "++" | "--" | "!" | "~" | "@"),
            Kind::Word => {
                matches!(
                    self.text(next),
                    "in" | "instanceof" | "as" | "satisfies" | "else" | "catch" | "finally"
                ) || (header && matches!(self.text(next), "extends" | "implements"))
            }
            _ => false,
        }
    }

    /// Whether `tokens[last]` ends a decorator: `@sealed`, `@a.b`, or
    /// `@memoize()`.
    fn ends_decorator(&self, last: usize) -> bool {
        let mut index = last;
        if self.kind(index) == Some(Kind::Close(')')) {
            let open = self.partner(index);
            match open.checked_sub(1) {
                Some(before) if self.touching(before, open) => index = before,
                _ => return false,
            }
        }
        while self.kind(index) == Some(Kind::Word) {
            let Some(before) = index.checked_sub(1) else {
                return false;
            };
            if self.is(before, "@") && self.touching(before, index) {
                return true;
            }
            if !(self.is(before, ".") && before > 0) {
                return false;
            }
            index = before - 1;
        }
        false
    }

    /// The first token after the decorators starting at `index`.
    fn skip_decorators(&self, mut index: usize) -> usize {
        while self.is(index, "@") {
            index += 1;
            while self.kind(index) == Some(Kind::Word) {
                index += 1;
                if self.is(index, ".") {
                    index += 1;
                } else {
                    break;
                }
            }
            if self.kind(index) == Some(Kind::Open('(')) && self.touching(index - 1, index) {
                index = self.partner(index) + 1;
            }
            index = self.next_significant(index).unwrap_or(self.tokens.len());
        }
        index
    }

    /// Whether the `{` at `index` can open the body of the declaration
    /// starting at `start` (not a TypeScript object type).
    fn opens_body(&self, start: usize, index: usize) -> bool {
        match self.previous_significant(start, index) {
            None => true,
            Some(previous) => match self.tokens[previous].kind {
                Kind::Word => !TYPE_OPERATORS.contains(&self.text(previous)),
                Kind::Close(_) | Kind::Literal => true,
                Kind::Op => self.text(previous) == ">",
                _ => false,
            },
        }
    }

    /// One past the last token from `start`: after the body if `bodied`
    /// (see the module documentation), otherwise where automatic semicolon
    /// insertion ends it. A `}` or `)` closing the enclosing block also
    /// ends it.
    fn scan(&self, start: usize, bodied: bool, keyword: &str) -> usize {
        let mut index = start;
        while index < self.tokens.len() {
            let last = match self.tokens[index].kind {
                Kind::Comment => {
                    index += 1;
                    continue;
                }
                Kind::Close(_) => return index,
                Kind::Semi => return index + 1,
                Kind::Open('{') if bodied && self.opens_body(start, index) => {
                    let close = self.partner(index);
                    match self.next_significant(close + 1) {
                        Some(next)
                            if matches!(self.text(next), "else" | "catch" | "finally")
                                || (keyword == "do" && self.is(next, "while")) =>
                        {
                            index = next;
                            continue;
                        }
                        _ => return close + 1,
                    }
                }
                Kind::Open(_) => self.partner(index),
                _ => index,
            };
            let Some(next) = self.next_significant(last + 1) else {
                return last + 1;
            };
            if self.line_breaks(last, next) > 0
                && self.can_end(last)
                && !self.ends_decorator(last)
                && !self.continues(next, bodied)
            {
                return last + 1;
            }
            index = next;
        }
        index
    }

    /// The first token of the statement at `start` after its decorators and
    /// `export`, `default`, `declare`, `abstract`, and `async` prefixes.
    fn statement_keyword(&self, start: usize) -> usize {
        let mut index = self.skip_decorators(start);
        loop {
            let prefix = match self.text(index) {
                "export" | "default" | "declare" | "abstract" => true,
                "async" => self.is(index + 1, "function"),
                _ => false,
            };
            if !prefix || self.kind(index) != Some(Kind::Word) {
                return index;
            }
            index = self
                .next_significant(index + 1)
                .unwrap_or(self.tokens.len());
        }
    }

    /// Whether the `namespace`/`module` at `index` declares one (not
    /// `module.exports`).
    fn is_namespace(&self, index: usize) -> bool {
        matches!(self.text(index), "namespace" | "module")
            && matches!(self.kind(index + 1), Some(Kind::Word | Kind::Literal))
            && self.line_breaks(index, index + 1) == 0
    }

    /// One past the last token of the statement starting at `start`.
    fn statement_end(&self, start: usize) -> usize {
        if self.kind(start) == Some(Kind::Open('{')) {
            return self.partner(start) + 1;
        }
        let keyword = self.statement_keyword(start);
        if keyword >= self.tokens.len() {
            return self.tokens.len();
        }
        let text = self.text(keyword);
        let bodied = matches!(
            text,
            "function"
                | "class"
                | "interface"
                | "enum"
                | "if"
                | "for"
                | "while"
                | "switch"
                | "try"
                | "do"
                | "with"
        ) || self.is_namespace(keyword);
        self.scan(start, bodied, text)
    }

    /// The name token of the class member at `start`, after its decorators
    /// and modifiers.
    fn member_name(&self, start: usize) -> usize {
        let mut index = self.skip_decorators(start);
        while self.kind(index) == Some(Kind::Word)
            && MODIFIERS.contains(&self.text(index))
            && self.next_significant(index + 1).is_some_and(|next| {
                self.line_breaks(index, next) == 0
                    && (matches!(
                        self.kind(next),
                        Some(Kind::Word | Kind::Literal | Kind::Open('['))
                    ) || self.is(next, "*"))
            })
        {
            index = self
                .next_significant(index + 1)
                .unwrap_or(self.tokens.len());
        }
        if self.is(index, "*") {
            index += 1;
        }
        index
    }

    /// The token after the member name at `name` (and its `?` or `!`).
    fn after_member_name(&self, name: usize) -> usize {
        let mut after = match self.kind(name) {
            Some(Kind::Open('[')) => self.partner(name) + 1,
            _ => name + 1,
        };
        if self.is(after, "?") || self.is(after, "!") {
            after += 1;
        }
        after
    }

    /// One past the last token of the class member starting at `start`.
    fn member_end(&self, start: usize) -> usize {
        let name = self.member_name(start);
        if self.is(name, "static") && self.kind(name + 1) == Some(Kind::Open('{')) {
            return self.partner(name + 1) + 1;
        }
        let after = self.after_member_name(name);
        let method = self.kind(after) == Some(Kind::Open('(')) || self.is(after, "<");
        self.scan(start, method, "")
    }

    /// The first `{` in `tokens[from..end]` that opens a body.
    fn body(&self, start: usize, from: usize, end: usize) -> Option<usize> {
        let mut index = from;
        while index < end {
            match self.tokens[index].kind {
                Kind::Open('{') if self.opens_body(start, index) => return Some(index),
                Kind::Open(_) => index = self.partner(index),
                _ => {}
            }
            index += 1;
        }
        None
    }

    /// Whether the value starting at `value` is an arrow function or a
    /// function expression.
    fn is_function_value(&self, value: usize, end: usize) -> bool {
        let mut index = value;
        if self.is(index, "async") && index + 1 < end {
            index += 1;
        }
        if self.is(index, "function") {
            return true;
        }
        if self.is(index, "<") {
            let mut depth = 0isize;
            while index < end {
                match self.text(index) {
                    "<" => depth += 1,
                    ">" => depth -= 1,
                    _ => {}
                }
                index += 1;
                if depth == 0 {
                    break;
                }
            }
        }
        let after = match self.kind(index) {
            Some(Kind::Open('(')) => self.partner(index) + 1,
            Some(Kind::Word) => index + 1,
            _ => return false,
        };
        if self.is(after, "=>") {
            return true;
        }
        self.is(after, ":")
            && self.kind(index) == Some(Kind::Open('('))
            && (after..end).any(|i| self.is(i, "=>"))
    }

    /// The first top-level `=` in `tokens[from..end]`.
    /// The declarators of a variable statement whose first declarator starts
    /// at `from`, split at top-level commas: `f = () => 1, g = () => 2`.
    /// Commas inside a type annotation's `<...>` (`Map<K, V>`) or a generic
    /// arrow's type parameters (`= <T, U>(x: T) => x`) do not split them.
    /// Each is a token range, without the trailing `,` or `;`.
    fn declarators(&self, from: usize, end: usize) -> Vec<(usize, usize)> {
        let mut output = Vec::new();
        let mut first = from;
        let mut angles = 0usize;
        let mut typed = true;
        let mut index = from;
        while index < end {
            match self.tokens[index].kind {
                Kind::Open(_) => index = self.partner(index),
                Kind::Semi => break,
                Kind::Op => match self.text(index) {
                    "<" if typed || self.is(index.wrapping_sub(1), "=") => angles += 1,
                    ">" | ">>" | ">>>" if angles > 0 => {
                        angles = angles.saturating_sub(self.text(index).len());
                    }
                    "=" if angles == 0 => typed = false,
                    "," if angles == 0 => {
                        output.push((first, index));
                        first = index + 1;
                        typed = true;
                    }
                    _ => {}
                },
                _ => {}
            }
            index += 1;
        }
        output.push((first, index));
        output
    }

    fn initializer(&self, from: usize, end: usize) -> Option<usize> {
        let mut index = from;
        while index < end {
            match self.tokens[index].kind {
                Kind::Open(_) => index = self.partner(index),
                Kind::Op if self.text(index) == "=" => return Some(index),
                _ => {}
            }
            index += 1;
        }
        None
    }

    /// What the statement `tokens[start..end]` declares.
    fn declares_statement(&self, start: usize, end: usize) -> Declares {
        let keyword = self.statement_keyword(start);
        if keyword >= end {
            return Declares::Other;
        }
        match self.text(keyword) {
            "function" => {
                let mut name = keyword + 1;
                if self.is(name, "*") {
                    name += 1;
                }
                if self.kind(name) == Some(Kind::Word) && self.body(start, name, end).is_some() {
                    Declares::Function(self.text(name).to_owned())
                } else {
                    Declares::Other
                }
            }
            "class" => {
                let name = keyword + 1;
                match (self.kind(name), self.body(start, name, end)) {
                    (Some(Kind::Word), Some(open))
                        if !matches!(self.text(name), "extends" | "implements") =>
                    {
                        Declares::Class(self.text(name).to_owned(), open)
                    }
                    _ => Declares::Other,
                }
            }
            "namespace" | "module" if self.is_namespace(keyword) => {
                let mut names = Vec::new();
                let mut index = keyword + 1;
                while matches!(self.kind(index), Some(Kind::Word | Kind::Literal)) {
                    names.push(self.text(index).trim_matches(['"', '\'']).to_owned());
                    if self.is(index + 1, ".") {
                        index += 2;
                    } else {
                        break;
                    }
                }
                match self.body(start, index, end) {
                    Some(open) => Declares::Namespace(names, open),
                    None => Declares::Other,
                }
            }
            "const" | "let" | "var" => {
                let bindings = self
                    .declarators(keyword + 1, end)
                    .into_iter()
                    .enumerate()
                    .filter_map(|(n, (first, last))| {
                        let equals = self.initializer(first, last)?;
                        (self.kind(first) == Some(Kind::Word)
                            && self.is_function_value(equals + 1, last))
                        .then(|| {
                            let from = if n == 0 { start } else { first };
                            (self.text(first).to_owned(), from, last)
                        })
                    })
                    .collect::<Vec<_>>();
                if bindings.is_empty() {
                    Declares::Other
                } else {
                    Declares::Bindings(bindings)
                }
            }
            _ => Declares::Other,
        }
    }

    /// What the class member `tokens[start..end]` declares.
    fn declares_member(&self, start: usize, end: usize) -> Declares {
        let name = self.member_name(start);
        let text = match self.kind(name) {
            Some(Kind::Word) => self.text(name).to_owned(),
            Some(Kind::Literal) if !self.text(name).starts_with('`') => {
                self.text(name).trim_matches(['"', '\'']).to_owned()
            }
            _ => return Declares::Other,
        };
        let after = self.after_member_name(name);
        let method = self.kind(after) == Some(Kind::Open('(')) || self.is(after, "<");
        let function = if method {
            self.body(start, after, end).is_some()
        } else {
            self.initializer(after, end)
                .is_some_and(|equals| self.is_function_value(equals + 1, end))
        };
        if function {
            Declares::Function(text)
        } else {
            Declares::Other
        }
    }

    /// Collects the functions among the statements in `tokens[low..high]`.
    fn statements(
        &self,
        low: usize,
        high: usize,
        scope: &mut Vec<String>,
        output: &mut Vec<Function>,
        class_bodies: &mut HashSet<usize>,
    ) {
        let mut index = low;
        while let Some(start) = self.next_significant(index).filter(|&s| s < high) {
            if matches!(self.tokens[start].kind, Kind::Semi | Kind::Close(_)) {
                index = start + 1;
                continue;
            }
            let end = self.statement_end(start).clamp(start + 1, high);
            self.record(
                self.declares_statement(start, end),
                start,
                end,
                scope,
                output,
                class_bodies,
            );
            index = end;
        }
    }

    /// Collects the functions among the class members in
    /// `tokens[low..high]`.
    fn members(
        &self,
        low: usize,
        high: usize,
        scope: &mut Vec<String>,
        output: &mut Vec<Function>,
        class_bodies: &mut HashSet<usize>,
    ) {
        let mut index = low;
        while let Some(start) = self.next_significant(index).filter(|&s| s < high) {
            if matches!(self.tokens[start].kind, Kind::Semi | Kind::Close(_)) {
                index = start + 1;
                continue;
            }
            let end = self.member_end(start).clamp(start + 1, high);
            self.record(
                self.declares_member(start, end),
                start,
                end,
                scope,
                output,
                class_bodies,
            );
            index = end;
        }
    }

    fn record(
        &self,
        declares: Declares,
        start: usize,
        end: usize,
        scope: &mut Vec<String>,
        output: &mut Vec<Function>,
        class_bodies: &mut HashSet<usize>,
    ) {
        match declares {
            Declares::Class(name, open) => {
                class_bodies.insert(open);
                scope.push(name);
                self.members(open + 1, self.partner(open), scope, output, class_bodies);
                scope.pop();
            }
            Declares::Namespace(names, open) => {
                let restore = scope.len();
                scope.extend(names);
                self.statements(open + 1, self.partner(open), scope, output, class_bodies);
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
            Declares::Bindings(bindings) => {
                for (name, first, last) in bindings {
                    let mut qualified = scope.clone();
                    qualified.push(name);
                    output.push(Function {
                        name: qualified,
                        start: self.tokens[first].start,
                        end: self.tokens[last - 1].end,
                    });
                }
            }
            Declares::Other => {}
        }
    }

    /// Every comment block that starts a line and is followed, on the next
    /// line, by a statement or class member.
    fn annotated(&self, class_bodies: &HashSet<usize>) -> Vec<Annotated> {
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
            let in_class = self.enclosing[next].is_some_and(|open| class_bodies.contains(&open));
            let end = if in_class {
                self.member_end(next)
            } else {
                self.statement_end(next)
            }
            .max(next + 1);
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
/// (`/**` and the leading `*` of JSDoc lines included).
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

    const SOURCE: &str = r#"import { describe, expect, test } from "vitest";

// @vtest.src-id SRC-JS-ADD
export function add(a: number, b: number): number {
  return a + b;
}

export const twice = (a: number) => a * 2;
const label = `{ ${"not"} code`;
export default async function* stream() { yield 1 }

export class Calc<T> {
  private base = 0;
  @memoize()
  sum(values: number[]): number {
    if (values.length === 0) { return this.base; }
    return values.reduce((x, y) => x + y, this.base);
  }
  get size() { return 1; }
  onClick = (event: Event) => { this.base += 1 };
}

namespace Shapes.Circles {
  export function area(r: number) { return Math.PI * r * r }
}

const api = {
  ping() { return "pong" },
};

describe("Calc", () => {
  // @vtest.id TEST-JS-ADD
  // @vtest.covers VO-JS-ADD
  // @vtest.target src/calc.ts::add
  // @vtest.intent adding two numbers returns their sum
  test("adds", () => {
    expect(add(1, 2)).toBe(3)
  })
})
"#;

    fn locators(outcome: &vtest_adapter_api::DiscoveryOutcome) -> Vec<&str> {
        outcome
            .sources
            .iter()
            .map(|source| source.locator.value.as_str())
            .collect()
    }

    /// @vtest.id TEST-ADAPTER-JAVASCRIPT-DISCOVERS-FUNCTIONS-AND-TESTS
    /// @vtest.covers VO-ADAPTER-JAVASCRIPT-REGISTERS-WITHOUT-CORE-CHANGES, VO-ADAPTER-LEXICAL-OPAQUE-LOCATOR-SYNTAX
    /// @vtest.target crates/vtest-adapter-lexical/src/javascript.rs::JavaScript::extract
    /// @vtest.intent verifies JS/TS function declarations, arrow-function variables and fields, class methods, and namespace functions become scope-qualified Source Targets while plain fields and calls do not, and an annotated test call nested in describe is a Test construct
    #[test]
    fn discovers_functions_and_nested_test_calls() {
        let outcome = discover_text(&JavaScript, "src/calc.ts", SOURCE, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(
            locators(&outcome),
            vec![
                "src/calc.ts::add",
                "src/calc.ts::twice",
                "src/calc.ts::stream",
                "src/calc.ts::Calc::sum",
                "src/calc.ts::Calc::size",
                "src/calc.ts::Calc::onClick",
                "src/calc.ts::Shapes::Circles::area",
            ]
        );
        assert_eq!(
            outcome.sources[0].src_id.as_ref().map(|id| id.as_str()),
            Some("SRC-JS-ADD")
        );
        assert_eq!(outcome.tests.len(), 1);
        let test = &outcome.tests[0];
        assert_eq!(test.id.as_str(), "TEST-JS-ADD");
        assert_eq!(test.location.locator, "test#1");
        assert!(test.construct_text.starts_with("test(\"adds\", () => {"));
        assert!(test.construct_text.ends_with("toBe(3)\n  })"));
    }

    /// @vtest.id TEST-ADAPTER-JAVASCRIPT-OBJECT-TYPE-IS-NOT-THE-BODY
    /// @vtest.covers VO-ADAPTER-JAVASCRIPT-REGISTERS-WITHOUT-CORE-CHANGES
    /// @vtest.target crates/vtest-adapter-lexical/src/javascript.rs::Reader::opens_body
    /// @vtest.intent verifies that a TypeScript object return type is not taken for the function body, so a Test construct runs to the end of the body and an edit to only the body's tail changes its text, and that statements without semicolons end where automatic semicolon insertion ends them
    #[test]
    fn object_return_type_is_not_the_body() {
        let source = |tail: &str| {
            format!(
                "// @vtest.id TEST-JS-TOTAL\n// @vtest.covers VO-JS-ADD\n// @vtest.intent the whole body is the construct\nexport function totals(items: Item[]): {{ total: number }} | {{ none: true }} {{\n  marker()\n  {tail}\n  return {{ total: 1 }}\n}}\n\nconst make = (): {{ a: number }} => ({{ a: 1 }})\nconst value = compute()\n  .then(done)\nfunction overload(a: string): void\nfunction overload(a: unknown) {{}}\n"
            )
        };
        let outcome = discover_text(&JavaScript, "src/total.ts", &source("one()"), "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(
            locators(&outcome),
            vec![
                "src/total.ts::totals",
                "src/total.ts::make",
                "src/total.ts::overload",
            ]
        );
        assert_eq!(
            outcome.sources[1].construct_text,
            "const make = (): { a: number } => ({ a: 1 })"
        );
        assert_eq!(
            outcome.sources[2].construct_text,
            "function overload(a: unknown) {}"
        );
        let test = &outcome.tests[0];
        assert!(test.construct_text.starts_with("export function totals("));
        assert!(test
            .construct_text
            .ends_with("one()\n  return { total: 1 }\n}"));
        let edited = discover_text(&JavaScript, "src/total.ts", &source("two()"), "demo");
        assert_ne!(edited.tests[0].construct_text, test.construct_text);
    }

    /// @vtest.id TEST-ADAPTER-JAVASCRIPT-JSX-REGEX-AND-UNREADABLE-FILES
    /// @vtest.covers VO-ADAPTER-JAVASCRIPT-REGISTERS-WITHOUT-CORE-CHANGES
    /// @vtest.target crates/vtest-adapter-lexical/src/javascript.rs::Lexer::jsx_end
    /// @vtest.intent verifies that quotes and braces in JSX text and regular expressions are not code, that a decorated class method annotated as a Test runs to the end of its body, and that an unterminated template literal makes the file unreadable
    #[test]
    fn jsx_regex_and_unreadable_files() {
        let source = "// @vtest.id TEST-JS-RENDER\n// @vtest.covers VO-JS-ADD\n// @vtest.intent a rendering test is a construct\nit(\"renders\", () => {\n  render(<p className=\"a\">don't {name} }</p>)\n  expect(/[)}]/.test(text)).toBe(true)\n})\n\nclass Suite {\n  // @vtest.id TEST-JS-METHOD\n  // @vtest.covers VO-JS-ADD\n  // @vtest.intent a decorated method is a construct\n  @Test()\n  adds() {\n    check()\n  }\n}\n";
        let outcome = discover_text(&JavaScript, "src/render.test.tsx", source, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(outcome.tests.len(), 2);
        assert!(outcome.tests[0].construct_text.ends_with("toBe(true)\n})"));
        assert_eq!(
            outcome.tests[1].construct_text,
            "@Test()\n  adds() {\n    check()\n  }"
        );
        assert_eq!(outcome.tests[1].location.locator, "Suite::adds");

        let broken = discover_text(&JavaScript, "src/broken.js", "const x = `open\n", "demo");
        assert_eq!(broken.diagnostics.len(), 1);
        assert_eq!(broken.diagnostics[0].code.as_str(), "E-SCAN-001");
    }

    /// @vtest.id TEST-ADAPTER-JAVASCRIPT-DEFAULT-PARAMETER-METHOD
    /// @vtest.covers VO-ADAPTER-JAVASCRIPT-REGISTERS-WITHOUT-CORE-CHANGES
    /// @vtest.target crates/vtest-adapter-lexical/src/javascript.rs::Reader::declares_member
    /// @vtest.intent verifies that a method whose parameters have default values is a Source Target, because a `=` inside the parameter list is not a field initializer
    #[test]
    fn default_parameter_method_is_a_source() {
        let outcome = discover_text(
            &JavaScript,
            "fixture.js",
            "class C { f(x = 1) { return x; } }\n",
            "demo",
        );
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(locators(&outcome), vec!["fixture.js::C::f"]);
    }
    /// @vtest.id TEST-ADAPTER-JAVASCRIPT-DECLARATORS-AND-TRUNCATED-FILES
    /// @vtest.covers VO-ADAPTER-LEXICAL-NORMALIZES-DECLARATION
    /// @vtest.target crates/vtest-adapter-lexical/src/javascript.rs::Reader::declarators
    /// @vtest.intent verifies each function-valued declarator of one variable statement is its own Source with its own range, and a file that ends inside a declaration is read without a panic
    #[test]
    fn declarators_and_truncated_files() {
        let source = "// @vtest.src-id SRC-JS-F\nconst f = () => 1, n = 2, g = function () { return 2; };\nexport let h = <T, U>(x: T, y: U) => x, m: Map<string, number> = new Map(), k = async (a: number) => a;\n";
        let outcome = discover_text(&JavaScript, "src/a.ts", source, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        let ranges = outcome
            .sources
            .iter()
            .map(|found| {
                let range = &found.location.byte_range;
                (
                    found.locator.value.as_str(),
                    &source[range.start as usize..range.end as usize],
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            ranges,
            vec![
                (
                    "src/a.ts::f",
                    "// @vtest.src-id SRC-JS-F\nconst f = () => 1"
                ),
                ("src/a.ts::g", "g = function () { return 2; }"),
                ("src/a.ts::h", "export let h = <T, U>(x: T, y: U) => x"),
                ("src/a.ts::k", "k = async (a: number) => a"),
            ]
        );
        assert_eq!(
            outcome.sources[0].src_id.as_ref().map(|id| id.as_str()),
            Some("SRC-JS-F")
        );
        for (source, unreadable) in [
            ("export", true),
            ("export default", true),
            ("declare", true),
            ("export abstract", true),
            ("@decorator", true),
            ("@a.b(1)", true),
            ("function f() {}\nexport", true),
            ("", false),
            ("// only a comment", false),
            ("x.export", false),
        ] {
            let outcome = discover_text(&JavaScript, "src/t.ts", source, "demo");
            assert_eq!(
                outcome.diagnostics.len(),
                usize::from(unreadable),
                "{source:?}"
            );
        }
        let after = discover_text(&JavaScript, "src/u.ts", "function u() {}\n", "demo");
        assert_eq!(after.sources.len(), 1);
    }
}
