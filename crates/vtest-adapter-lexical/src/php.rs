//! `php`: PHP sources (`.php`).
//!
//! A file is read in three steps, all specific to PHP:
//!
//! 1. [`lex`] reads the code between `<?php` (or `<?=`) and `?>` (which
//!    ends a statement like `;`); the inline HTML around it is not code.
//!    It splits the code into comments (`//`, `#`, `/* */`), string
//!    literals (`"{$a['}']}"` interpolations, heredocs, and nowdocs
//!    included), words (`$variables` and `App\Math` names included),
//!    brackets (`#[` opens an attribute), `;`, and operators.
//! 2. [`Reader::statement_end`] finds where a statement or class member
//!    ends. PHP needs no newline rule: a statement ends at its `;`, or
//!    after the `{}` body of a `function`, `class`, `interface`, `trait`,
//!    `enum`, `namespace`, or control statement (an `else`, `elseif`,
//!    `catch`, or `finally` continues it; a control statement's body can
//!    also be one statement or alternative syntax). A file that ends
//!    before a statement does is unreadable. Brackets are skipped whole, so
//!    closures, `match` arms, and anonymous classes inside an expression
//!    statement do not end it.
//! 3. [`Reader::statements`] walks the file: `namespace` (braced or
//!    statement form, `A\B` names) and `class`, `interface`, `trait`, and
//!    `enum` bodies qualify the named `function` declarations inside them.
//!    Named functions inside control statements and function bodies are
//!    declared in the namespace.
//!    A method without a body (abstract or in an interface) and closures
//!    are not Source Targets.
//!
//! A comment block (a docblock included) attached to a statement or member
//! makes it a Test (or declares its SRC ID); its `#[...]` attributes are
//! part of the construct, so PHPUnit methods and Pest's
//! `test('adds', function () { ... });` are discovered the same way.

use crate::{
    shared::{attached_comments, pair_brackets, Attached, Span},
    Annotated, ExtractError, FileItems, Function, LanguageProfile, LexicalAdapter,
};

pub const ADAPTER_ID: &str = "php";

#[derive(Clone, Copy, Debug, Default)]
pub struct Php;

impl LanguageProfile for Php {
    fn adapter_id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn languages(&self) -> &'static [&'static str] {
        &["php"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["php"]
    }

    fn excluded_dirs(&self) -> &'static [&'static str] {
        &["vendor"]
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
        if let Some(index) = reader.unclosed_alternative() {
            return Err(ExtractError::new(
                tokens[index].start,
                "alternative syntax without its end keyword",
            ));
        }
        reader
            .statements(0, tokens.len(), &mut Vec::new(), 0, &mut functions)
            .map_err(|index| {
                ExtractError::new(
                    tokens[index].start,
                    "the file or its enclosing block ends before this statement is complete",
                )
            })?;
        Ok(FileItems {
            functions,
            annotated: reader.annotated(),
        })
    }
}

/// The `php` adapter.
pub fn adapter() -> LexicalAdapter<Php> {
    LexicalAdapter::new(Php)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    Comment,
    Literal,
    Word,
    /// `(`, `[`, `{`, or `#[` (read as `[`).
    Open(char),
    Close(char),
    /// `;` or `?>`.
    Semi,
    Op,
}

#[derive(Clone, Copy, Debug)]
struct Tok {
    kind: Kind,
    start: usize,
    end: usize,
}

fn is_word_char(byte: u8) -> bool {
    byte == b'_' || byte == b'\\' || byte.is_ascii_alphanumeric() || !byte.is_ascii()
}

/// Where the next `<?php`, `<?=`, or `<? ` opening tag after `i` ends.
fn code_start(source: &str, i: usize) -> Option<usize> {
    let mut from = i;
    while let Some(found) = source[from..].find("<?") {
        let at = from + found;
        let rest = &source[at + 2..];
        if rest
            .as_bytes()
            .get(..3)
            .is_some_and(|tag| tag.eq_ignore_ascii_case(b"php"))
        {
            return Some(at + 5);
        }
        if rest.starts_with('=') {
            return Some(at + 3);
        }
        if rest.starts_with(|c: char| c.is_ascii_whitespace()) {
            return Some(at + 2);
        }
        from = at + 2;
    }
    None
}

/// Splits PHP source into tokens. An unterminated comment or literal is an
/// error.
fn lex(source: &str) -> Result<Vec<Tok>, ExtractError> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let Some(mut i) = code_start(source, 0) else {
        return Ok(tokens);
    };
    while i < bytes.len() {
        let start = i;
        let rest = &source[i..];
        let byte = bytes[i];
        let kind = if byte.is_ascii_whitespace() {
            i += 1;
            continue;
        } else if rest.starts_with("?>") {
            i += 2;
            tokens.push(Tok {
                kind: Kind::Semi,
                start,
                end: i,
            });
            match code_start(source, i) {
                Some(next) => i = next,
                None => break,
            }
            continue;
        } else if rest.starts_with("#[") {
            i += 2;
            Kind::Open('[')
        } else if rest.starts_with("//") || byte == b'#' {
            let line = rest.find('\n').unwrap_or(rest.len());
            i += rest[..line].find("?>").unwrap_or(line);
            Kind::Comment
        } else if let Some(body) = rest.strip_prefix("/*") {
            let close = body
                .find("*/")
                .ok_or_else(|| ExtractError::new(start, "unterminated comment"))?;
            i += close + 4;
            Kind::Comment
        } else if rest.starts_with("<<<") {
            i = heredoc_end(source, i)
                .ok_or_else(|| ExtractError::new(start, "unterminated heredoc"))?;
            Kind::Literal
        } else if matches!(byte, b'\'' | b'"' | b'`') {
            i = string_end(bytes, i)
                .ok_or_else(|| ExtractError::new(start, "unterminated string"))?;
            Kind::Literal
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
        } else if is_word_char(byte)
            || (byte == b'$' && bytes.get(i + 1).copied().is_some_and(is_word_char))
        {
            i += 1;
            while i < bytes.len() && is_word_char(bytes[i]) {
                i += 1;
            }
            Kind::Word
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

/// One past the end of the `'`, `"`, or `` ` `` string at `i`.
/// Interpolations (`{$a['}']}`, `${a}`) are read as code.
fn string_end(bytes: &[u8], i: usize) -> Option<usize> {
    let quote = bytes[i];
    let mut j = i + 1;
    while j < bytes.len() {
        match bytes[j] {
            b'\\' => j += 2,
            byte if byte == quote => return Some(j + 1),
            b'{' if quote != b'\'' && bytes.get(j + 1) == Some(&b'$') => {
                j = interpolation_end(bytes, j + 1)?
            }
            b'$' if quote != b'\'' && bytes.get(j + 1) == Some(&b'{') => {
                j = interpolation_end(bytes, j + 2)?
            }
            _ => j += 1,
        }
    }
    None
}

/// One past the `}` closing an interpolation whose code starts at `i`.
fn interpolation_end(bytes: &[u8], mut i: usize) -> Option<usize> {
    let mut depth = 1usize;
    while i < bytes.len() {
        match bytes[i] {
            b'\'' | b'"' => i = string_end(bytes, i)?,
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

/// One past the closing identifier of the heredoc or nowdoc at `i`
/// (`<<<ID`, `<<<"ID"`, `<<<'ID'`), which may be indented.
fn heredoc_end(source: &str, i: usize) -> Option<usize> {
    let header_end = i + source[i..].find('\n')?;
    let label = source[i + 3..header_end]
        .trim()
        .trim_matches(|c| c == '"' || c == '\'');
    if label.is_empty() {
        return None;
    }
    let mut line_start = header_end + 1;
    while line_start <= source.len() {
        let line_end = source[line_start..]
            .find('\n')
            .map_or(source.len(), |n| line_start + n);
        let line = &source[line_start..line_end];
        let indent = line.len() - line.trim_start().len();
        if let Some(after) = line.trim_start().strip_prefix(label) {
            if !after.starts_with(|c: char| c == '_' || c.is_ascii_alphanumeric()) {
                return Some(line_start + indent + label.len());
            }
        }
        if line_end == source.len() {
            break;
        }
        line_start = line_end + 1;
    }
    None
}

const MODIFIERS: &[&str] = &[
    "abstract",
    "final",
    "public",
    "protected",
    "private",
    "static",
    "readonly",
    "var",
];

/// Statements that end after a `{}` body.
const BODIED: &[&str] = &[
    "function",
    "class",
    "interface",
    "trait",
    "enum",
    "namespace",
    "if",
    "for",
    "foreach",
    "while",
    "do",
    "switch",
    "try",
    "declare",
    "use",
];

/// Control statements whose blocks can declare named functions.
const CONTROL: &[&str] = &[
    "if", "else", "elseif", "while", "do", "for", "foreach", "switch", "declare", "try",
];

/// What a statement declares.
enum Declares {
    /// A class-like or braced namespace body (`true` for a namespace).
    Scope(Vec<String>, usize, bool),
    /// `namespace A\B;`: the statements after it belong to `A\B`.
    Namespace(Vec<String>),
    /// A named function and the `{` of its body.
    Function(String, usize),
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
            && self.text(index).eq_ignore_ascii_case(text)
    }

    fn partner(&self, index: usize) -> usize {
        self.partners[index].expect("brackets are balanced")
    }

    fn next_significant(&self, from: usize) -> Option<usize> {
        (from..self.tokens.len()).find(|&i| self.tokens[i].kind != Kind::Comment)
    }

    /// The first token of the statement at `start` after its `#[...]`
    /// attributes and modifiers.
    fn keyword(&self, start: usize) -> usize {
        let mut index = start;
        loop {
            let next = match self.kind(index) {
                Some(Kind::Open('[')) if self.text(index) == "#[" => self.partner(index) + 1,
                Some(Kind::Word) if MODIFIERS.iter().any(|m| self.is(index, m)) => index + 1,
                Some(Kind::Comment) => index + 1,
                _ => return index,
            };
            index = next;
        }
    }

    /// One past the last token of the statement or member starting at
    /// `start` (see the module documentation), or `None` when the file
    /// or the enclosing block ends first (PHP needs a `;`, `?>`, or body to
    /// end every statement). A `)` or `]` closing an enclosing argument
    /// list ends it.
    fn statement_end(&self, start: usize) -> Option<usize> {
        if self.kind(start) == Some(Kind::Open('{')) {
            return Some(self.partner(start) + 1);
        }
        let keyword_index = self.keyword(start);
        let keyword = match self.kind(keyword_index) {
            Some(Kind::Word) => self.text(keyword_index).to_ascii_lowercase(),
            _ => String::new(),
        };
        match keyword.as_str() {
            "if" | "elseif" | "while" | "for" | "foreach" | "switch" | "declare" => {
                if let Some(end) = self.control_end(keyword_index, &keyword) {
                    return end;
                }
            }
            "else" => return self.statement_end(self.next_significant(keyword_index + 1)?),
            "do" => {
                let body = self.statement_end(self.next_significant(keyword_index + 1)?)?;
                let word = self.next_significant(body)?;
                let open = self.next_significant(word + 1)?;
                if !(self.is(word, "while") && self.kind(open) == Some(Kind::Open('('))) {
                    return None;
                }
                let semi = self.next_significant(self.partner(open) + 1)?;
                return (self.kind(semi) == Some(Kind::Semi)).then_some(semi + 1);
            }
            _ => {}
        }
        let bodied = BODIED.contains(&keyword.as_str());
        let mut index = start;
        while index < self.tokens.len() {
            match self.tokens[index].kind {
                // A statement needs its `;` or body before the `}` closing
                // its block: `function f()` or `test(...)` there, or `if ($a)`
                // with no statement after it, is incomplete.
                Kind::Close('}') => return None,
                Kind::Close(_) => return Some(index),
                Kind::Semi => return Some(index + 1),
                Kind::Open('{') if bodied => {
                    let close = self.partner(index);
                    match self.next_significant(close + 1) {
                        Some(next) if ["catch", "finally"].iter().any(|k| self.is(next, k)) => {
                            index = next;
                        }
                        Some(next) if keyword == "use" && self.kind(next) == Some(Kind::Semi) => {
                            return Some(next + 1)
                        }
                        _ => return Some(close + 1),
                    }
                    continue;
                }
                Kind::Open(_) => index = self.partner(index),
                _ => {}
            }
            index += 1;
        }
        None
    }

    /// The end of the control statement (`if`, `while`, ...) at `keyword`
    /// whose body follows its parenthesized head: a block, a single
    /// statement (`if ($a) one(); else two();`), or alternative syntax
    /// (`if ($a): ... endif;`); `if` and `elseif` continue with `else` or
    /// `elseif`. The outer `None` means the keyword has no head (it is not
    /// a control statement); the inner one that the file ends first.
    #[allow(clippy::option_option)]
    fn control_end(&self, keyword: usize, word: &str) -> Option<Option<usize>> {
        let open = self.next_significant(keyword + 1)?;
        if self.kind(open) != Some(Kind::Open('(')) {
            return None;
        }
        let end = (|| {
            let after = self.next_significant(self.partner(open) + 1)?;
            if self.is(after, ":") {
                let end = self.alternative_end(after, word)?;
                let semi = self.next_significant(end)?;
                return (self.kind(semi) == Some(Kind::Semi)).then_some(semi + 1);
            }
            let body = self.statement_end(after)?;
            if matches!(word, "if" | "elseif") {
                if let Some(next) = self.next_significant(body) {
                    if self.is(next, "else") || self.is(next, "elseif") {
                        return self.statement_end(next);
                    }
                }
            }
            Some(body)
        })();
        Some(end)
    }

    /// Whether the `:` at `index` opens the alternative syntax of a
    /// control statement (`if ($x):` ... `endif;`).
    fn opens_alternative(&self, index: usize) -> bool {
        (0..index)
            .rev()
            .find(|&i| self.tokens[i].kind != Kind::Comment)
            .is_some_and(|previous| {
                self.kind(previous) == Some(Kind::Close(')')) || self.is(previous, "else")
            })
    }

    /// One past the `end...` word closing the alternative syntax opened at
    /// `colon` by `keyword`, or `None` when it is never closed. The same
    /// keyword opening the alternative syntax again inside it
    /// (`if (a): if (b): ... endif; ... endif;`) needs its own `end...`.
    fn alternative_end(&self, colon: usize, keyword: &str) -> Option<usize> {
        let Some(closing) = alternative_closing(keyword) else {
            return Some(colon + 1);
        };
        let mut depth = 0usize;
        let mut index = colon + 1;
        while index < self.tokens.len() {
            match self.tokens[index].kind {
                Kind::Open(_) => index = self.partner(index),
                Kind::Word
                    if self.is(index, keyword) && self.alternative_colon(index).is_some() =>
                {
                    depth += 1;
                }
                Kind::Word if self.is(index, closing) => {
                    if depth == 0 {
                        return Some(index + 1);
                    }
                    depth -= 1;
                }
                _ => {}
            }
            index += 1;
        }
        None
    }

    /// The `:` that opens the alternative syntax of the `if`, `for`, ...
    /// at `keyword`: right after its parenthesized head.
    fn alternative_colon(&self, keyword: usize) -> Option<usize> {
        let open = self.next_significant(keyword + 1)?;
        if self.kind(open) != Some(Kind::Open('(')) {
            return None;
        }
        let colon = self.next_significant(self.partner(open) + 1)?;
        self.is(colon, ":").then_some(colon)
    }

    /// The first alternative syntax (`if (...):`) in the file that is never
    /// closed by its `end...` word.
    fn unclosed_alternative(&self) -> Option<usize> {
        (0..self.tokens.len()).find(|&index| {
            self.kind(index) == Some(Kind::Word)
                && alternative_closing(&self.text(index).to_ascii_lowercase()).is_some()
                && self.alternative_colon(index).is_some_and(|colon| {
                    self.alternative_end(colon, &self.text(index).to_ascii_lowercase())
                        .is_none()
                })
        })
    }

    /// Collects the function definitions among the statements in
    /// `tokens[low..high]`, qualified by `scope`, whose first `namespace`
    /// names are the namespace. Named functions inside control statements
    /// and function bodies are declared in the namespace, so they are
    /// collected too. `Err` is the statement the file ends inside.
    fn statements(
        &self,
        low: usize,
        high: usize,
        scope: &mut Vec<String>,
        mut namespace: usize,
        output: &mut Vec<Function>,
    ) -> Result<(), usize> {
        let restore = scope.len();
        let mut index = low;
        while let Some(start) = self.next_significant(index).filter(|&s| s < high) {
            if matches!(self.tokens[start].kind, Kind::Semi | Kind::Close(_)) {
                index = start + 1;
                continue;
            }
            if let Some(colon) = self.label_colon(start) {
                index = colon + 1;
                continue;
            }
            let end = self
                .statement_end(start)
                .ok_or(start)?
                .clamp(start + 1, high);
            match self.declares(start, end) {
                Declares::Scope(name, open, is_namespace) => {
                    let inner = scope.len();
                    scope.extend(name);
                    let inner_namespace = if is_namespace { scope.len() } else { namespace };
                    self.statements(open + 1, self.partner(open), scope, inner_namespace, output)?;
                    scope.truncate(inner);
                }
                Declares::Namespace(name) => {
                    scope.truncate(restore);
                    scope.extend(name);
                    namespace = scope.len();
                }
                Declares::Function(name, open) => {
                    let mut qualified = scope.clone();
                    qualified.push(name);
                    output.push(Function {
                        name: qualified,
                        start: self.tokens[start].start,
                        end: self.tokens[end - 1].end,
                    });
                    let mut inner = scope[..namespace].to_vec();
                    self.statements(open + 1, self.partner(open), &mut inner, namespace, output)?;
                }
                Declares::Other => {
                    if CONTROL.iter().any(|k| self.is(self.keyword(start), k)) {
                        self.control_bodies(start, end, scope, namespace, output)?;
                    }
                }
            }
            index = end;
        }
        scope.truncate(restore);
        Ok(())
    }

    /// Collects the functions in the blocks and alternative-syntax bodies
    /// of the control statement `tokens[start..end]`.
    fn control_bodies(
        &self,
        start: usize,
        end: usize,
        scope: &mut Vec<String>,
        namespace: usize,
        output: &mut Vec<Function>,
    ) -> Result<(), usize> {
        let mut index = start;
        while index < end {
            match self.tokens[index].kind {
                Kind::Open('{') => {
                    let close = self.partner(index);
                    self.statements(index + 1, close, scope, namespace, output)?;
                    index = close;
                }
                Kind::Open(_) => index = self.partner(index),
                Kind::Op if self.text(index) == ":" && self.opens_alternative(index) => {
                    let keyword = (start..index)
                        .rev()
                        .find(|&i| CONTROL.iter().any(|k| self.is(i, k)) && !self.is(i, "else"))
                        .map(|i| self.text(i).to_ascii_lowercase())
                        .unwrap_or_default();
                    if let Some(after) = self.alternative_end(index, &keyword) {
                        self.statements(index + 1, after - 1, scope, namespace, output)?;
                        index = after - 1;
                    }
                }
                _ => {}
            }
            index += 1;
        }
        Ok(())
    }

    /// The `:` ending the label at `start` that only separates statements
    /// inside a body: `case ...:`, `default:`, or, in alternative syntax,
    /// `else:` and `elseif (...):`.
    fn label_colon(&self, start: usize) -> Option<usize> {
        if self.is(start, "else") {
            let colon = self.next_significant(start + 1)?;
            return self.is(colon, ":").then_some(colon);
        }
        if self.is(start, "elseif") {
            return self.alternative_colon(start);
        }
        if !(self.is(start, "case") || self.is(start, "default")) {
            return None;
        }
        // The first `:` that is not part of `::`, `?:`, or a ternary.
        let mut ternaries = 0usize;
        let mut index = start + 1;
        while index < self.tokens.len() {
            match self.tokens[index].kind {
                Kind::Open(_) => index = self.partner(index),
                Kind::Semi => return Some(index),
                Kind::Close(_) => return None,
                Kind::Op if self.text(index) == "?" => {
                    if self.touching(index, &["?", ":", "-", ">"]) {
                        index += 1;
                    } else {
                        ternaries += 1;
                    }
                }
                Kind::Op if self.text(index) == ":" => {
                    if self.touching(index, &[":"]) {
                        index += 1;
                    } else if ternaries > 0 {
                        ternaries -= 1;
                    } else {
                        return Some(index);
                    }
                }
                _ => {}
            }
            index += 1;
        }
        None
    }

    /// Whether the token after `index` follows it without a gap and is
    /// one of `texts` (`??`, `?:`, `?->`, `::`).
    fn touching(&self, index: usize, texts: &[&str]) -> bool {
        self.tokens.get(index + 1).is_some_and(|next| {
            next.start == self.tokens[index].end && texts.contains(&self.text(index + 1))
        })
    }

    /// The first top-level `{` in `tokens[from..end]`.
    fn body(&self, from: usize, end: usize) -> Option<usize> {
        let mut index = from;
        while index < end {
            match self.tokens[index].kind {
                Kind::Open('{') => return Some(index),
                Kind::Open(_) => index = self.partner(index),
                _ => {}
            }
            index += 1;
        }
        None
    }

    /// What the statement `tokens[start..end]` declares.
    fn declares(&self, start: usize, end: usize) -> Declares {
        let keyword = self.keyword(start);
        if keyword >= end || self.kind(keyword) != Some(Kind::Word) {
            return Declares::Other;
        }
        let word = self.text(keyword).to_ascii_lowercase();
        match word.as_str() {
            "function" => {
                let mut name = keyword + 1;
                if self.is(name, "&") {
                    name += 1;
                }
                match self.body(name, end) {
                    Some(open)
                        if self.kind(name) == Some(Kind::Word)
                            && self.kind(name + 1) == Some(Kind::Open('(')) =>
                    {
                        Declares::Function(self.text(name).to_owned(), open)
                    }
                    _ => Declares::Other,
                }
            }
            "class" | "interface" | "trait" | "enum" => {
                let name = keyword + 1;
                match (self.kind(name), self.body(name, end)) {
                    (Some(Kind::Word), Some(open)) => {
                        Declares::Scope(vec![self.text(name).to_owned()], open, false)
                    }
                    _ => Declares::Other,
                }
            }
            "namespace" => {
                let name = keyword + 1;
                let segments = match self.kind(name) {
                    Some(Kind::Word) => self
                        .text(name)
                        .split('\\')
                        .filter(|segment| !segment.is_empty())
                        .map(str::to_owned)
                        .collect(),
                    _ => Vec::new(),
                };
                match self.body(keyword, end) {
                    Some(open) => Declares::Scope(segments, open, true),
                    None => Declares::Namespace(segments),
                }
            }
            _ => Declares::Other,
        }
    }

    /// Every comment block that starts a line and is followed, on the next
    /// line, by a statement or member.
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
            let construct = match self.statement_end(next) {
                Some(end) => Ok((
                    self.tokens[next].start,
                    self.tokens[end.max(next + 1) - 1].end,
                )),
                None => Err(
                    "the file or its enclosing block ends before this statement is complete"
                        .to_owned(),
                ),
            };
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

/// The word that closes the alternative syntax of `keyword`.
fn alternative_closing(keyword: &str) -> Option<&'static str> {
    Some(match keyword {
        "if" => "endif",
        "for" => "endfor",
        "foreach" => "endforeach",
        "while" => "endwhile",
        "switch" => "endswitch",
        "declare" => "enddeclare",
        _ => return None,
    })
}

/// The lines of a `//`, `#`, or `/* */` comment with the comment markers
/// removed (`/**` and the leading `*` of docblock lines included).
fn comment_lines(comment: &str) -> Vec<String> {
    if let Some(line) = comment
        .strip_prefix("//")
        .or_else(|| comment.strip_prefix('#'))
    {
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

    const SOURCE: &str = r#"<?php namespace App\Math;

declare(strict_types=1);

use PHPUnit\Framework\TestCase;

# a hash comment with a brace {
final class Calc
{
    private string $label = "{ not
code }";

    // @vtest.src-id SRC-PHP-ADD
    public function add(int $a, int $b): int
    {
        return $a + $b;
    }

    abstract protected function undefined(): int;

    public static function &shared(): array { static $x = []; return $x; }
}

function helper(callable $f = null) {
    $g = function () use ($f) { return $f; };
    return fn($x) => $x;
}

class CalcTest extends TestCase
{
    /**
     * @vtest.id TEST-PHP-ADD
     * @vtest.covers VO-PHP-ADD
     * @vtest.target src/Calc.php::App::Math::Calc::add
     * @vtest.intent adding two numbers returns their sum
     */
    #[Test]
    #[DataProvider('numbers')]
    public function testAdd(): void
    {
        $this->assertSame(3, (new Calc())->add(1, 2));
    }
}
"#;

    fn locators(outcome: &vtest_adapter_api::DiscoveryOutcome) -> Vec<&str> {
        outcome
            .sources
            .iter()
            .map(|source| source.locator.value.as_str())
            .collect()
    }

    /// @vtest.id TEST-ADAPTER-PHP-DISCOVERS-FUNCTIONS-AND-TESTS
    /// @vtest.covers VO-ADAPTER-PHP-REGISTERS-WITHOUT-CORE-CHANGES, VO-ADAPTER-LEXICAL-OPAQUE-LOCATOR-SYNTAX
    /// @vtest.target crates/vtest-adapter-lexical/src/php.rs::Php::extract
    /// @vtest.intent verifies PHP functions and methods become namespace- and class-qualified Source Targets while abstract methods and closures do not, hash comments are not attributes, and an attributed PHPUnit method is a Test construct including its attributes
    #[test]
    fn discovers_php_functions_and_tests() {
        let outcome = discover_text(&Php, "src/Calc.php", SOURCE, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(
            locators(&outcome),
            vec![
                "src/Calc.php::App::Math::Calc::add",
                "src/Calc.php::App::Math::Calc::shared",
                "src/Calc.php::App::Math::helper",
                "src/Calc.php::App::Math::CalcTest::testAdd",
            ]
        );
        assert_eq!(
            outcome.sources[0].src_id.as_ref().map(|id| id.as_str()),
            Some("SRC-PHP-ADD")
        );
        assert_eq!(outcome.tests.len(), 1);
        let test = &outcome.tests[0];
        assert_eq!(test.id.as_str(), "TEST-PHP-ADD");
        assert!(test
            .construct_text
            .starts_with("#[Test]\n    #[DataProvider('numbers')]"));
        assert!(test.construct_text.ends_with("add(1, 2));\n    }"));
    }

    /// @vtest.id TEST-ADAPTER-PHP-ATTRIBUTE-BRACES-ARE-NOT-THE-BODY
    /// @vtest.covers VO-ADAPTER-PHP-REGISTERS-WITHOUT-CORE-CHANGES
    /// @vtest.target crates/vtest-adapter-lexical/src/php.rs::Reader::statement_end
    /// @vtest.intent verifies that closures, match arms, and interpolations inside attributes, default values, and strings are not taken for the body, so a Test construct runs to the end of the method body and an edit to only the body's tail changes its text, and that braced namespaces qualify their functions
    #[test]
    fn attribute_braces_are_not_the_body() {
        let source = |tail: &str| {
            format!(
                "<?php\nnamespace Demo {{\nclass PickTest\n{{\n    // @vtest.id TEST-PHP-PICK\n    // @vtest.covers VO-PHP-ADD\n    // @vtest.intent the whole body is the construct\n    #[TestWith([['a' => '}}']])]\n    public function testPick(array $o = [1, 2], ?\\Closure $f = null): void\n    {{\n        $x = match ($o) {{ default => \"{{$o['}}']}}\" }};\n        {tail}\n    }}\n\n    public function after() {{}}\n}}\n}}\n"
            )
        };
        let outcome = discover_text(&Php, "tests/PickTest.php", &source("one();"), "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(
            locators(&outcome),
            vec![
                "tests/PickTest.php::Demo::PickTest::testPick",
                "tests/PickTest.php::Demo::PickTest::after",
            ]
        );
        let test = &outcome.tests[0];
        assert!(test.construct_text.starts_with("#[TestWith("));
        assert!(test.construct_text.ends_with("one();\n    }"));
        let edited = discover_text(&Php, "tests/PickTest.php", &source("two();"), "demo");
        assert_ne!(edited.tests[0].construct_text, test.construct_text);
    }

    /// @vtest.id TEST-ADAPTER-PHP-PEST-HEREDOC-AND-UNREADABLE-FILES
    /// @vtest.covers VO-ADAPTER-PHP-REGISTERS-WITHOUT-CORE-CHANGES
    /// @vtest.target crates/vtest-adapter-lexical/src/php.rs::heredoc_end
    /// @vtest.intent verifies that an annotated Pest test call is a Test construct ending at its semicolon, that heredoc text and inline HTML are not code, and that an unterminated string makes the file unreadable
    #[test]
    fn pest_heredoc_and_unreadable_files() {
        let source = "<html>{ <?php\n// @vtest.id TEST-PHP-PEST\n// @vtest.covers VO-PHP-ADD\n// @vtest.intent a Pest test is a construct\ntest('adds', function () {\n    $text = <<<EOT\n    } not code {\n    EOT;\n    expect($text)->toBeString();\n});\n?>\n</html>}\n";
        let outcome = discover_text(&Php, "tests/AddTest.php", source, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(outcome.tests.len(), 1);
        assert!(outcome.tests[0].construct_text.starts_with("test('adds'"));
        assert!(outcome.tests[0]
            .construct_text
            .ends_with("toBeString();\n});"));

        let broken = discover_text(&Php, "src/Broken.php", "<?php\n$x = 'open;\n", "demo");
        assert_eq!(broken.diagnostics.len(), 1);
        assert_eq!(broken.diagnostics[0].code.as_str(), "E-SCAN-001");
    }

    /// @vtest.id TEST-ADAPTER-PHP-OPEN-TAG-AND-STATEMENT-NAMESPACES
    /// @vtest.covers VO-ADAPTER-PHP-REGISTERS-WITHOUT-CORE-CHANGES
    /// @vtest.target crates/vtest-adapter-lexical/src/php.rs::Reader::statements
    /// @vtest.intent verifies that the opening tag is not part of the first function, so the SRC ID comment above it attaches, and that a statement-form namespace replaces the previous one instead of nesting in it
    #[test]
    fn open_tag_and_statement_namespaces() {
        let outcome = discover_text(
            &Php,
            "fixture.php",
            "<?php\n// @vtest.src-id SRC-X\nfunction f(){return 1;}\n",
            "demo",
        );
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(
            outcome.sources[0].src_id.as_ref().map(|id| id.as_str()),
            Some("SRC-X")
        );
        let outcome = discover_text(
            &Php,
            "fixture.php",
            "<?php\nnamespace A;\nfunction a() {}\nnamespace B;\nfunction b() {}\n",
            "demo",
        );
        assert_eq!(
            locators(&outcome),
            vec!["fixture.php::A::a", "fixture.php::B::b"]
        );
    }
    /// @vtest.id TEST-ADAPTER-PHP-NESTED-ALTERNATIVE-SYNTAX-AND-TAGS
    /// @vtest.covers VO-ADAPTER-LEXICAL-RETURNS-SOURCE-RANGE-AND-BYTES
    /// @vtest.target crates/vtest-adapter-lexical/src/php.rs::Reader::alternative_end
    /// @vtest.intent verifies nested alternative syntax runs to the outer end keyword, so a change at the end of the outer block changes the Test construct, an unclosed one makes the file unreadable, and a non-ASCII character after <? does not panic
    #[test]
    fn nested_alternative_syntax_and_tags() {
        let declare = "<?php\n// @vtest.id TEST-X\n// @vtest.covers VO-X\n// @vtest.intent check\n";
        for body in [
            "if (true):\n    if (true): echo 0; endif;\n    echo 1;\nendif;\n",
            "foreach ($a as $x):\n    foreach ($x as $y): echo 0; endforeach;\n    echo 1;\nendforeach;\n",
            "while (true):\n    if (true): echo 0; else: echo 0; endif;\n    echo 1;\nENDWHILE;\n",
        ] {
            let one = format!("{declare}{body}");
            let two = one.replace("echo 1", "echo 2");
            let text = |source: &str| {
                let outcome = discover_text(&Php, "src/t.php", source, "demo");
                assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
                assert_eq!(outcome.tests.len(), 1, "{body:?}");
                outcome.tests[0].construct_text.clone()
            };
            assert!(text(&one).contains("echo 1;"), "{body:?}");
            assert_ne!(text(&one), text(&two), "{body:?}");
        }
        for source in [
            "<?php if (true): if (true): endif;\n",
            "<?php foreach ($a as $b): echo 1;\n",
        ] {
            let outcome = discover_text(&Php, "src/u.php", source, "demo");
            assert_eq!(outcome.diagnostics.len(), 1, "{source:?}");
        }
        for source in ["<?éé", "<?é", "<html><?xml é ?>", "<?PHP function f() {}\n"] {
            let _ = discover_text(&Php, "src/v.php", source, "demo");
        }
        let upper = discover_text(&Php, "src/w.php", "<?PHP function f() {}\n", "demo");
        assert_eq!(upper.sources[0].locator.value, "src/w.php::f");
    }

    /// @vtest.id TEST-ADAPTER-PHP-UNBRACED-BODIES-TRUNCATED-FILES-AND-NESTED-FUNCTIONS
    /// @vtest.covers VO-ADAPTER-LEXICAL-RETURNS-SOURCE-RANGE-AND-BYTES
    /// @vtest.target crates/vtest-adapter-lexical/src/php.rs::Reader::statement_end
    /// @vtest.intent verifies that a single-statement if/else or do/while body is part of the Test construct so an edit to its tail changes the text, that a file ending inside a statement is unreadable instead of yielding a Test, and that named functions inside control statements, switch cases, alternative syntax, and function bodies are Source Targets in their namespace
    #[test]
    fn unbraced_bodies_truncated_files_and_nested_functions() {
        let declare = "<?php\n// @vtest.id TEST-X\n// @vtest.covers VO-X\n// @vtest.intent check\n";
        for body in [
            "if ($flag) one(); else tail();\n",
            "if ($flag) one(); elseif ($other) two(); else if ($third) three(); else tail();\n",
            "do one(); while (tail());\n",
            "while ($flag) tail();\n",
            "if ($flag) { one(); } else { tail(); }\n",
        ] {
            let text = |source: &str| {
                let outcome = discover_text(&Php, "src/t.php", source, "demo");
                assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
                assert_eq!(outcome.tests.len(), 1, "{body:?}");
                outcome.tests[0].construct_text.clone()
            };
            let one = format!("{declare}{body}");
            assert!(text(&one).ends_with(body.trim_end()), "{body:?}");
            assert_ne!(
                text(&one),
                text(&one.replace("tail()", "tail_changed()")),
                "{body:?}"
            );
        }
        for tail in [
            "function broken()\n",
            "if ($flag)\n",
            "if ($flag) { one(); } else\n",
            "test('tail', function () { tail(); })\n",
            "do one();\n",
        ] {
            let outcome = discover_text(&Php, "src/u.php", &format!("{declare}{tail}"), "demo");
            assert_eq!(outcome.diagnostics.len(), 1, "{tail:?}");
            assert_eq!(outcome.diagnostics[0].code.as_str(), "E-SCAN-001");
            assert!(outcome.tests.is_empty(), "{tail:?}");
        }
        let outcome = discover_text(
            &Php,
            "fixture.php",
            "<?php\nnamespace N;\nif (!function_exists('f')) { function f() { return 1; } }\nfunction outer() { function inner() { return 1; } }\nclass C { function m() { function g() {} } }\nswitch ($x) { case $a ? 1 : 2: function s() {} break; case A::B: default: function t() {} }\nif ($a): function u() {} elseif ($b): function v() {} else: function w() {} endif;\ntry { function x() {} } catch (E $e) { function y() {} } finally { function z() {} }\n",
            "demo",
        );
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(
            locators(&outcome),
            vec![
                "fixture.php::N::f",
                "fixture.php::N::outer",
                "fixture.php::N::inner",
                "fixture.php::N::C::m",
                "fixture.php::N::g",
                "fixture.php::N::s",
                "fixture.php::N::t",
                "fixture.php::N::u",
                "fixture.php::N::v",
                "fixture.php::N::w",
                "fixture.php::N::x",
                "fixture.php::N::y",
                "fixture.php::N::z",
            ]
        );
    }

    /// @vtest.id TEST-ADAPTER-PHP-ENCLOSING-BLOCK-ENDS
    /// @vtest.covers VO-ADAPTER-LEXICAL-RETURNS-SOURCE-RANGE-AND-BYTES
    /// @vtest.target crates/vtest-adapter-lexical/src/php.rs::Reader::statement_end
    /// @vtest.intent verifies that a statement cut off by the `}` of its enclosing block (a function without its body, a call without its `;`, an `if` without its statement) is not read as a whole Test, while a `?>` before that `}` still ends a statement
    #[test]
    fn enclosing_block_ends() {
        let declare = "// @vtest.id TEST-X\n// @vtest.covers VO-X\n// @vtest.intent check\n";
        for tail in [
            "function broken()\n",
            "test('tail', function () { tail(); })\n",
            "if ($flag)\n",
            "if ($flag) { one(); } else\n",
        ] {
            for source in [
                format!("<?php\nfunction outer() {{\n{declare}{tail}}}\n"),
                format!("<?php\nclass C {{\n    function m() {{\n{declare}{tail}}}\n}}\n"),
            ] {
                let outcome = discover_text(&Php, "src/u.php", &source, "demo");
                assert!(outcome.tests.is_empty(), "{source:?}");
                assert!(!outcome.diagnostics.is_empty(), "{source:?}");
            }
        }
        let source = format!("<?php\nif ($flag) {{\n{declare}one() ?>\n<p>html</p>\n<?php }}\n");
        let outcome = discover_text(&Php, "src/v.php", &source, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(outcome.tests.len(), 1);
        assert_eq!(outcome.tests[0].construct_text, "one() ?>");
    }
}
