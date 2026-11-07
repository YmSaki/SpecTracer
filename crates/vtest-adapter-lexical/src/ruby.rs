//! `ruby`: Ruby sources (`.rb`, `.rake`).
//!
//! A file is read in three steps, all specific to Ruby:
//!
//! 1. [`Lexer`] splits it into comments (`#` and `=begin`/`=end`),
//!    literals (strings with `#{...}` interpolations, heredocs, `%w[]`-style
//!    percent literals, regular expressions, symbols such as `:end`, and
//!    hash labels such as `class:`), words, brackets, `;`, and operators.
//!    The file ends at `__END__`.
//! 2. [`Reader::walk`] pairs each `end` with the keyword it closes:
//!    `def` (unless endless, `def name = expr`), `class`, `module`,
//!    `case`, `begin`, `for`, `do`, and `if`/`unless`/`while`/`until` when
//!    they start an expression (after a value on the same line they are
//!    modifiers and close nothing). The `do` of `while x do` belongs to the
//!    loop. The word after `def` is always the method name, so
//!    `def end` and `range.end` close nothing. A missing or extra `end`
//!    makes the file unreadable.
//! 3. While pairing, it records the `def` methods that are not inside
//!    another method, qualified by the enclosing `class` and `module`
//!    names (`A::B` names split); blocks such as `RSpec.describe ... do`
//!    do not qualify them. A method starts at its line's first token
//!    (`private def secret`).
//!
//! A comment block attached to a statement makes it a Test (or declares
//! its SRC ID): the statement runs to the `end` of the keywords it opens,
//! then to the end of its line, unless the line ends with an operator or
//! the next line starts with `.method`; the bodies of heredocs opened on
//! the line are part of it. A file that ends while an expression is
//! awaited is unreadable. Minitest methods, RSpec examples
//! (`it "adds" do ... end`), and test-unit are discovered the same way.

use crate::{
    shared::{attached_comments, pair_brackets, Attached, Span},
    Annotated, ExtractError, FileItems, Function, LanguageProfile, LexicalAdapter,
};

pub const ADAPTER_ID: &str = "ruby";

#[derive(Clone, Copy, Debug, Default)]
pub struct Ruby;

impl LanguageProfile for Ruby {
    fn adapter_id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn languages(&self) -> &'static [&'static str] {
        &["ruby"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["rb", "rake"]
    }

    fn excluded_dirs(&self) -> &'static [&'static str] {
        &["vendor", ".bundle", "tmp", "log"]
    }

    fn extract(&self, source: &str) -> Result<FileItems, ExtractError> {
        let tokens = Lexer {
            source,
            bytes: source.as_bytes(),
        }
        .run()?;
        let partners = pair_brackets(tokens.iter().map(|token| match token.kind {
            Kind::Open(c) | Kind::Close(c) => (token.start, Some(c)),
            _ => (token.start, None),
        }))?;
        let mut reader = Reader {
            source,
            tokens: &tokens,
            partners: &partners,
            ends: vec![None; tokens.len()],
        };
        let functions = reader.walk()?;
        Ok(FileItems {
            functions,
            annotated: reader.annotated(),
        })
    }
}

/// The `ruby` adapter.
pub fn adapter() -> LexicalAdapter<Ruby> {
    LexicalAdapter::new(Ruby)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    Comment,
    Literal,
    /// The bodies of the heredocs opened on the line before, up to their
    /// terminators: part of the statement that opened them.
    HeredocBody,
    Word,
    Open(char),
    Close(char),
    Semi,
    Op,
}

#[derive(Clone, Copy, Debug)]
struct Tok {
    kind: Kind,
    start: usize,
    end: usize,
}

fn is_word_start(byte: u8) -> bool {
    byte == b'_' || byte == b'@' || byte == b'$' || byte.is_ascii_alphabetic() || !byte.is_ascii()
}

fn is_word_char(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric() || !byte.is_ascii()
}

/// Operators read as one token, longest first.
const OPERATORS: &[&str] = &[
    "**=", "<=>", "===", "...", "<<=", ">>=", "&&=", "||=", "==", "!=", ">=", "<=", "&&", "||",
    "<<", ">>", "**", "=~", "!~", "+=", "-=", "*=", "/=", "%=", "|=", "&=", "^=", "::", "&.", "..",
    "->", "=>",
];

/// Keywords after which an expression can start.
const EXPRESSION_KEYWORDS: &[&str] = &[
    "then", "else", "elsif", "do", "begin", "and", "or", "not", "return", "when", "in", "yield",
    "ensure", "rescue", "if", "unless", "while", "until", "puts", "case",
];

/// A heredoc whose body starts on the next line.
struct Pending {
    label: String,
    indented: bool,
}

struct Lexer<'a> {
    source: &'a str,
    bytes: &'a [u8],
}

impl Lexer<'_> {
    fn run(&self) -> Result<Vec<Tok>, ExtractError> {
        let bytes = self.bytes;
        let mut tokens: Vec<Tok> = Vec::new();
        let mut pending: Vec<Pending> = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            let start = i;
            let rest = &self.source[i..];
            let byte = bytes[i];
            let line_start = i == 0 || bytes[i - 1] == b'\n';
            let kind = if byte == b'\n' && !pending.is_empty() {
                i = self.heredoc_bodies(i + 1, &mut pending)?;
                tokens.push(Tok {
                    kind: Kind::HeredocBody,
                    start: start + 1,
                    end: i,
                });
                continue;
            } else if byte.is_ascii_whitespace()
                || (byte == b'\\' && matches!(bytes.get(i + 1), Some(b'\n' | b'\r')))
            {
                i += 1;
                continue;
            } else if line_start && rest.lines().next().map(str::trim_end) == Some("__END__") {
                break;
            } else if line_start && rest.starts_with("=begin") {
                let close = rest
                    .find("\n=end")
                    .ok_or_else(|| ExtractError::new(start, "unterminated =begin comment"))?;
                i += close + 5;
                i += self.source[i..].find('\n').unwrap_or(self.source.len() - i);
                Kind::Comment
            } else if byte == b'#' {
                i += rest.find('\n').unwrap_or(rest.len());
                Kind::Comment
            } else if matches!(byte, b'"' | b'\'' | b'`') {
                i = self
                    .delimited_end(i + 1, byte, byte, byte != b'\'')
                    .ok_or_else(|| ExtractError::new(start, "unterminated string"))?;
                Kind::Literal
            } else if let Some((end, label)) = self.heredoc_start(&tokens, i) {
                pending.push(label);
                i = end;
                Kind::Literal
            } else if byte == b':' && bytes.get(i + 1) == Some(&b'"') {
                i = self
                    .delimited_end(i + 2, b'"', b'"', true)
                    .ok_or_else(|| ExtractError::new(start, "unterminated symbol"))?;
                Kind::Literal
            } else if byte == b':'
                && bytes.get(i + 1).copied().is_some_and(is_word_start)
                && !(i > 0 && bytes[i - 1] == b':')
            {
                i = self.word_end(i + 1);
                Kind::Literal
            } else if byte.is_ascii_digit() {
                i += 1;
                while i < bytes.len()
                    && (is_word_char(bytes[i])
                        || (bytes[i] == b'.' && bytes.get(i + 1).is_some_and(u8::is_ascii_digit)))
                {
                    i += 1;
                }
                Kind::Literal
            } else if is_word_start(byte) {
                i = self.word_end(i);
                // `class:` in `{class: "x"}` or `{if:1}` is a hash label, not
                // a keyword. Without a space after the `:`, the word may
                // instead be a ternary's middle operand (`c ? a:b`).
                let ternary = tokens
                    .iter()
                    .rev()
                    .find(|t: &&Tok| t.kind != Kind::Comment)
                    .is_some_and(|t| self.text(t) == "?");
                if bytes.get(i) == Some(&b':')
                    && bytes.get(i + 1) != Some(&b':')
                    && (bytes.get(i + 1).is_none_or(u8::is_ascii_whitespace) || !ternary)
                {
                    i += 1;
                    Kind::Literal
                } else {
                    Kind::Word
                }
            } else if let Some(end) =
                (byte == b'/' && self.expression_position(&tokens, i)).then(|| {
                    Self::literal_or_error(
                        self.delimited_end(i + 1, b'/', b'/', true),
                        i,
                        "unterminated regular expression",
                    )
                })
            {
                i = end?;
                while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                    i += 1;
                }
                Kind::Literal
            } else if let Some(end) = (byte == b'%' && self.expression_position(&tokens, i))
                .then(|| {
                    self.percent_end(i)
                        .map(|end| Self::literal_or_error(end, i, "unterminated percent literal"))
                })
                .flatten()
            {
                i = end?;
                Kind::Literal
            } else {
                i += 1;
                match byte {
                    b'(' | b'[' | b'{' => Kind::Open(byte as char),
                    b')' | b']' | b'}' => Kind::Close(byte as char),
                    b';' => Kind::Semi,
                    _ => {
                        let operator = OPERATORS.iter().find(|op| rest.starts_with(**op));
                        i += operator.map_or(0, |op| op.len() - 1);
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
        if let Some(open) = pending.first() {
            return Err(ExtractError::new(
                self.source.len(),
                format!("unterminated heredoc `{}`", open.label),
            ));
        }
        Ok(tokens)
    }

    /// One past a word at `i`, with a `?` or `!` suffix (`empty?`) when it
    /// is not the start of `!=` or `?:`.
    fn word_end(&self, mut i: usize) -> usize {
        let bytes = self.bytes;
        i += 1;
        while i < bytes.len() && is_word_char(bytes[i]) {
            i += 1;
        }
        if matches!(bytes.get(i), Some(b'?' | b'!')) && bytes.get(i + 1) != Some(&b'=') {
            i += 1;
        }
        i
    }

    fn text<'t>(&'t self, token: &Tok) -> &'t str {
        &self.source[token.start..token.end]
    }

    /// Whether a `/`, `%`, or `<<` at `i` can start a literal: after an
    /// operator, a bracket, a keyword, or a line break, or after a method
    /// name when spaced from it but not from what follows (`puts /x/`).
    fn expression_position(&self, tokens: &[Tok], i: usize) -> bool {
        let Some(previous) = tokens.iter().rev().find(|t| t.kind != Kind::Comment) else {
            return true;
        };
        if self.source[previous.end..i].contains('\n') {
            return true;
        }
        match previous.kind {
            Kind::Op | Kind::Open(_) | Kind::Semi => true,
            Kind::Word if EXPRESSION_KEYWORDS.contains(&self.text(previous)) => true,
            Kind::Word => {
                previous.end < i
                    && self
                        .bytes
                        .get(i + 1)
                        .is_some_and(|b| !b.is_ascii_whitespace() && *b != b'=')
            }
            _ => false,
        }
    }

    /// The end of a literal that starts at `i` (a `/` or `%` where a
    /// literal can start), or an error when it has no closing delimiter.
    /// After a method name (`assert_match /x`) the `/` or `%` could also be
    /// an operator if the name were a local variable, which a lexical
    /// reader cannot tell, so an unclosed one is rejected rather than read
    /// as code.
    fn literal_or_error(
        end: Option<usize>,
        i: usize,
        message: &str,
    ) -> Result<usize, ExtractError> {
        end.ok_or_else(|| ExtractError::new(i, message))
    }

    /// One past the closing delimiter of a literal whose contents start at
    /// `i`; `open`/`close` nest, and `#{...}` is read as code when
    /// `interpolates`.
    fn delimited_end(
        &self,
        mut i: usize,
        open: u8,
        close: u8,
        interpolates: bool,
    ) -> Option<usize> {
        let mut depth = 1usize;
        while i < self.bytes.len() {
            let byte = self.bytes[i];
            if byte == b'\\' {
                i += 2;
            } else if byte == close {
                depth -= 1;
                i += 1;
                if depth == 0 {
                    return Some(i);
                }
            } else if byte == open {
                depth += 1;
                i += 1;
            } else if interpolates && byte == b'#' && self.bytes.get(i + 1) == Some(&b'{') {
                i = self.interpolation_end(i + 2)?;
            } else {
                i += 1;
            }
        }
        None
    }

    /// One past the `}` closing a `#{` interpolation whose code starts at
    /// `i`.
    fn interpolation_end(&self, mut i: usize) -> Option<usize> {
        let mut depth = 1usize;
        while i < self.bytes.len() {
            match self.bytes[i] {
                quote @ (b'"' | b'\'' | b'`') => {
                    i = self.delimited_end(i + 1, quote, quote, quote != b'\'')?
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

    /// One past a percent literal (`%w[a b]`, `%q{...}`, `%(...)`) at `i`:
    /// `None` when the `%` does not start one, `Some(None)` when it is not
    /// closed.
    fn percent_end(&self, i: usize) -> Option<Option<usize>> {
        let mut j = i + 1;
        let kind = *self.bytes.get(j)?;
        if b"qQwWiIrsx".contains(&kind) {
            j += 1;
        }
        let open = *self.bytes.get(j)?;
        if open.is_ascii_alphanumeric() || open.is_ascii_whitespace() {
            return None;
        }
        let close = match open {
            b'(' => b')',
            b'[' => b']',
            b'{' => b'}',
            b'<' => b'>',
            other => other,
        };
        let interpolates = !matches!(kind, b'q' | b'w' | b'i' | b's');
        let Some(mut end) = self.delimited_end(j + 1, open, close, interpolates) else {
            return Some(None);
        };
        if kind == b'r' {
            while end < self.bytes.len() && self.bytes[end].is_ascii_alphabetic() {
                end += 1;
            }
        }
        Some(Some(end))
    }

    /// A heredoc opener (`<<~EOS`, `<<-EOS`, `<<"EOS"`, `<<EOS`) at `i`:
    /// one past it and its label.
    fn heredoc_start(&self, tokens: &[Tok], i: usize) -> Option<(usize, Pending)> {
        let rest = &self.source[i..];
        let after = rest.strip_prefix("<<")?;
        let (indented, after) = match after.strip_prefix(['~', '-']) {
            Some(stripped) => (true, stripped),
            None => (false, after),
        };
        let (label, length) = match after.as_bytes().first()? {
            quote @ (b'"' | b'\'' | b'`') => {
                let close = after[1..].find(*quote as char)?;
                (after[1..=close].to_owned(), close + 2)
            }
            byte if is_word_char(*byte) => {
                let length = after.bytes().take_while(|b| is_word_char(*b)).count();
                let word = &after[..length];
                // After a name, `<<word` is a shift if the name is a local
                // variable (`a = []` before `a <<b`) and a heredoc otherwise
                // (`assert_equal <<text, actual`), as Ruby's parser decides
                // it; a heredoc then needs its terminator line.
                let previous = tokens
                    .iter()
                    .rev()
                    .find(|t| t.kind != Kind::Comment)
                    .filter(|t| {
                        t.kind == Kind::Word
                            && !EXPRESSION_KEYWORDS.contains(&self.text(t))
                            && !self.source[t.end..i].contains('\n')
                    });
                if previous.is_some_and(|name| self.is_local(tokens, self.text(name))) {
                    return None;
                }
                let explicit = indented || self.expression_position(tokens, i);
                if !explicit {
                    return None;
                }
                (word.to_owned(), length)
            }
            _ => return None,
        };
        let end = i + (rest.len() - after.len()) + length;
        Some((end, Pending { label, indented }))
    }

    /// Whether `name` is a local variable where the last of `tokens` is
    /// read: an earlier token assigns it (`a = []`, `a ||= []`), or names
    /// it as a block parameter (`|a|`, `|x, a|`), a method parameter
    /// (`def m(a)`), or a `for` variable. Like Ruby's parser, this looks at
    /// what comes before, but over the whole file rather than one scope.
    fn is_local(&self, tokens: &[Tok], name: &str) -> bool {
        let code: Vec<&Tok> = tokens.iter().filter(|t| t.kind != Kind::Comment).collect();
        (0..code.len().saturating_sub(1)).any(|j| {
            if code[j].kind != Kind::Word || self.text(code[j]) != name {
                return false;
            }
            let next = code.get(j + 1).map(|t| (t.kind, self.text(t)));
            let previous = j.checked_sub(1).map(|p| (code[p].kind, self.text(code[p])));
            let assigned = matches!(next, Some((Kind::Op, op))
                if op.ends_with('=') && !matches!(op, "==" | "===" | "!=" | ">=" | "<="));
            let block_parameter = matches!(previous, Some((_, "|")))
                || (matches!(previous, Some((_, ","))) && matches!(next, Some((_, "|" | ","))));
            let line_start = self.source[..code[j].start]
                .rfind('\n')
                .map_or(0, |n| n + 1);
            let method_parameter = matches!(previous, Some((_, "(" | ",")))
                && self.source[line_start..code[j].start]
                    .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .any(|word| word == "def");
            let for_variable = matches!(previous, Some((Kind::Word, "for")));
            assigned || block_parameter || method_parameter || for_variable
        })
    }

    /// Skips the bodies of the pending heredocs, which start at `i`,
    /// returning one past the last terminator line's label.
    fn heredoc_bodies(
        &self,
        mut i: usize,
        pending: &mut Vec<Pending>,
    ) -> Result<usize, ExtractError> {
        for heredoc in pending.drain(..) {
            loop {
                if i >= self.source.len() {
                    return Err(ExtractError::new(
                        i,
                        format!("unterminated heredoc `{}`", heredoc.label),
                    ));
                }
                let line_end = self.source[i..]
                    .find('\n')
                    .map_or(self.source.len(), |n| i + n);
                let line = self.source[i..line_end].trim_end_matches('\r');
                let candidate = if heredoc.indented {
                    line.trim_start()
                } else {
                    line
                };
                let next = (line_end + 1).min(self.source.len());
                if candidate == heredoc.label {
                    i = line_end;
                    break;
                }
                i = next;
            }
        }
        Ok(i)
    }
}

/// A keyword waiting for its `end`.
enum Opened {
    Def { name: String, start: usize },
    Scope(Vec<String>),
    Block,
}

struct Reader<'a> {
    source: &'a str,
    tokens: &'a [Tok],
    partners: &'a [Option<usize>],
    /// For each keyword that opens a construct, the index of its `end`.
    ends: Vec<Option<usize>>,
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

    /// Whether a line break separates `tokens[before]` and `tokens[after]`
    /// (a `\` continuation does not count).
    fn breaks(&self, before: usize, after: usize) -> bool {
        self.source[self.tokens[before].end..self.tokens[after].start]
            .replace("\\\r\n", "")
            .replace("\\\n", "")
            .contains('\n')
    }

    /// Whether `tokens[index]` is a method name after `.`, `&.`, or `::`.
    fn after_dot(&self, index: usize) -> bool {
        self.previous_significant(index)
            .is_some_and(|p| matches!(self.text(p), "." | "&." | "::"))
    }

    /// Whether an expression starts at `tokens[index]` (so `if` there opens
    /// a construct instead of modifying the statement before it).
    fn starts_expression(&self, index: usize) -> bool {
        match self.previous_significant(index) {
            None => true,
            Some(previous) => {
                self.breaks(previous, index)
                    || match self.tokens[previous].kind {
                        Kind::Semi | Kind::Open(_) | Kind::Op => true,
                        Kind::Word => EXPRESSION_KEYWORDS.contains(&self.text(previous)),
                        _ => false,
                    }
            }
        }
    }

    /// The first token of the line holding `tokens[index]` (`private` in
    /// `private def secret`).
    fn statement_start(&self, index: usize) -> usize {
        let mut start = index;
        while let Some(previous) = self.previous_significant(start) {
            if self.breaks(previous, start)
                || matches!(self.tokens[previous].kind, Kind::Semi | Kind::Open(_))
            {
                break;
            }
            start = previous;
        }
        start
    }

    /// The method name after the `def` at `keyword` and the index after
    /// it: `add`, `self.create`, `==`, `[]=`, `value=`, `empty?`, `end`.
    fn method_name(&self, keyword: usize) -> Option<(String, usize)> {
        let mut index = self.next_significant(keyword + 1)?;
        let mut name = String::new();
        if self.kind(index) == Some(Kind::Word) && self.is(index + 1, ".") {
            name.push_str(self.text(index));
            name.push('.');
            index += 2;
        }
        match self.kind(index)? {
            Kind::Open('[') if self.partner(index) == index + 1 => {
                name.push_str("[]");
                index += 2;
            }
            Kind::Word | Kind::Op | Kind::Literal => {
                name.push_str(self.text(index).trim_end_matches(':'));
                index += 1;
            }
            _ => return None,
        }
        // A unary operator: `def +@`, `def -@`.
        if self.kind(index - 1) == Some(Kind::Op)
            && matches!(self.text(index - 1), "+" | "-" | "!" | "~")
            && self.is(index, "@")
            && self.touching(index - 1, index)
        {
            name.push('@');
            index += 1;
        }
        // A setter: `def value=(v)`, `def value= v`, `def []=(k, v)`. An
        // endless method cannot be a setter, so a `=` bound to the name is
        // part of it.
        if self.is(index, "=")
            && matches!(self.kind(index - 1), Some(Kind::Word | Kind::Close(']')))
            && self.touching(index - 1, index)
        {
            name.push('=');
            index += 1;
        }
        Some((name, index))
    }

    /// Whether the `def` whose name ends before `after` is endless
    /// (`def name(args) = expr`).
    fn is_endless(&self, after: usize) -> bool {
        let mut index = after;
        if self.kind(index) == Some(Kind::Open('(')) {
            index = self.partner(index) + 1;
        }
        self.next_significant(index)
            .is_some_and(|next| self.is(next, "=") && !self.breaks(index - 1, next))
    }

    /// Pairs every `end` with the keyword it closes and collects the
    /// methods that are not inside another method.
    fn walk(&mut self) -> Result<Vec<Function>, ExtractError> {
        let mut functions = Vec::new();
        // Endless defs, whose end is found once every keyword is paired
        // with its `end` (`def f = if x then 1 else 2 end`).
        let mut endless: Vec<(Vec<String>, String, usize)> = Vec::new();
        let mut stack: Vec<(usize, Opened)> = Vec::new();
        let mut loop_do = false;
        let mut index = 0;
        while index < self.tokens.len() {
            if self.tokens[index].kind == Kind::Comment {
                index += 1;
                continue;
            }
            if self
                .previous_significant(index)
                .is_none_or(|p| self.breaks(p, index) || self.kind(p) == Some(Kind::Semi))
            {
                loop_do = false;
            }
            if self.tokens[index].kind != Kind::Word || self.after_dot(index) {
                index += 1;
                continue;
            }
            let in_def = stack
                .iter()
                .any(|(_, opened)| matches!(opened, Opened::Def { .. }));
            match self.text(index) {
                "def" => {
                    let Some((name, after)) = self.method_name(index) else {
                        index += 1;
                        continue;
                    };
                    let start = self.statement_start(index);
                    if self.is_endless(after) {
                        if !in_def {
                            endless.push((scopes(&stack), name, start));
                        }
                    } else {
                        stack.push((index, Opened::Def { name, start }));
                    }
                    index = after;
                    continue;
                }
                "class" | "module" => {
                    let mut names = Vec::new();
                    let mut name = index + 1;
                    while self.kind(name) == Some(Kind::Word) {
                        names.push(self.text(name).to_owned());
                        if self.is(name + 1, "::") {
                            name += 2;
                        } else {
                            break;
                        }
                    }
                    stack.push((index, Opened::Scope(names)));
                }
                "if" | "unless" | "while" | "until" if self.starts_expression(index) => {
                    loop_do = matches!(self.text(index), "while" | "until");
                    stack.push((index, Opened::Block));
                }
                "for" => {
                    loop_do = true;
                    stack.push((index, Opened::Block));
                }
                "case" | "begin" => stack.push((index, Opened::Block)),
                "do" if loop_do => loop_do = false,
                "do" => stack.push((index, Opened::Block)),
                "end" => {
                    let (opener, opened) = stack.pop().ok_or_else(|| {
                        ExtractError::new(
                            self.tokens[index].start,
                            "`end` without an opening keyword",
                        )
                    })?;
                    self.ends[opener] = Some(index);
                    if let Opened::Def { name, start } = opened {
                        let nested = stack
                            .iter()
                            .any(|(_, opened)| matches!(opened, Opened::Def { .. }));
                        if !nested {
                            functions.push(self.function(&stack, name, start, index));
                        }
                    }
                }
                _ => {}
            }
            index += 1;
        }
        if let Some((opener, _)) = stack.last() {
            return Err(ExtractError::new(
                self.tokens[*opener].start,
                format!("`{}` without `end`", self.text(*opener)),
            ));
        }
        for (mut qualified, name, start) in endless {
            let last = self.statement_end(start).ok_or_else(|| {
                ExtractError::new(
                    self.tokens[start].start,
                    "the file or its enclosing construct ends before this method's expression",
                )
            })? - 1;
            qualified.push(name);
            functions.push(Function {
                name: qualified,
                start: self.tokens[start].start,
                end: self.tokens[last].end,
            });
        }
        functions.sort_by_key(|function| function.start);
        Ok(functions)
    }

    fn function(
        &self,
        stack: &[(usize, Opened)],
        name: String,
        start: usize,
        last: usize,
    ) -> Function {
        let mut qualified = scopes(stack);
        qualified.push(name);
        Function {
            name: qualified,
            start: self.tokens[start].start,
            end: self.tokens[last].end,
        }
    }

    /// One past the last token of the statement starting at `start` (see
    /// the module documentation), or `None` when the file, or the `end`,
    /// clause, or bracket closing the enclosing construct, comes while it
    /// still waits for an expression (`def f =`, `a +`). A `}` or `)`
    /// closing the enclosing bracket also ends it.
    fn statement_end(&self, start: usize) -> Option<usize> {
        let mut index = start;
        while index < self.tokens.len() {
            let last = match self.tokens[index].kind {
                Kind::Comment => {
                    index += 1;
                    continue;
                }
                Kind::Close(_) => return Some(index),
                Kind::Semi => return Some(index + 1),
                Kind::Open(_) => self.partner(index),
                _ => self.ends[index].unwrap_or(index),
            };
            // An endless range (`1..`) may end the statement, but it
            // continues onto a next line like any other operator.
            let range =
                self.kind(last) == Some(Kind::Op) && matches!(self.text(last), ".." | "...");
            let waits = self.kind(last) == Some(Kind::Op)
                || (self.kind(last) == Some(Kind::Word)
                    && matches!(self.text(last), "and" | "or" | "not"));
            let Some(next) = self.next_significant(last + 1) else {
                return (!waits || range).then_some(last + 1);
            };
            if waits && self.closes(next) {
                // The enclosing `end`, clause, or bracket comes while an
                // expression is still awaited (`def f =` then `end`).
                return range.then_some(last + 1);
            }
            let continued = waits
                || matches!(self.text(next), "." | "&.")
                || self.kind(next) == Some(Kind::HeredocBody);
            if self.breaks(last, next) && !continued {
                return Some(last + 1);
            }
            index = next;
        }
        None
    }

    /// Whether `tokens[index]` closes the enclosing construct: a `)`, `]`,
    /// or `}`, an `end`, or a clause keyword (`else`, `when`, ...).
    fn closes(&self, index: usize) -> bool {
        match self.tokens[index].kind {
            Kind::Close(_) => true,
            Kind::Word => {
                matches!(
                    self.text(index),
                    "end" | "else" | "elsif" | "when" | "in" | "rescue" | "ensure" | "then"
                ) && !self.after_dot(index)
            }
            _ => false,
        }
    }

    /// Every comment block that starts a line and is followed, on the next
    /// line, by a statement.
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
            .filter(|block| !matches!(self.tokens[block.next].kind, Kind::Close(_) | Kind::Semi))
            .map(|Attached { first, last, next }| Annotated {
                comment_start: self.tokens[first].start,
                lines: (first..=last)
                    .flat_map(|i| comment_lines(self.text(i)))
                    .collect(),
                construct: match self.statement_end(next) {
                    Some(end) => Ok((
                        self.tokens[next].start,
                        self.tokens[end.max(next + 1) - 1].end,
                    )),
                    None => Err("the file or its enclosing construct ends before this statement is complete".to_owned()),
                },
            })
            .collect()
    }
}

/// The lines of a `#` or `=begin`/`=end` comment with the comment markers
/// removed.
fn comment_lines(comment: &str) -> Vec<String> {
    if let Some(line) = comment.strip_prefix('#') {
        return vec![line.to_owned()];
    }
    let lines: Vec<&str> = comment.lines().collect();
    lines
        .iter()
        .skip(1)
        .take(lines.len().saturating_sub(2))
        .map(|line| (*line).to_owned())
        .collect()
}

/// The `class`/`module` names on `stack`, outermost first.
fn scopes(stack: &[(usize, Opened)]) -> Vec<String> {
    stack
        .iter()
        .filter_map(|(_, opened)| match opened {
            Opened::Scope(names) => Some(names.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover_text;

    const SOURCE: &str = r#"# frozen_string_literal: true

module Demo
  module Math
    class Calc < Base
      LABEL = "{ not
        code }"

      # @vtest.src-id SRC-RB-ADD
      def add(a, b)
        return 0 if a.nil?
        while a > 100 do a -= 1 end
        result = if b.zero? then a else a + b end
        result
      end

      def self.create = new

      def ==(other)
        other.is_a?(Calc) && other.range.end == range.end
      end

      def empty?
        [1, 2].each { |x| puts x unless x.zero? }
        {class: "x"}.empty?
      end

      private def secret
        :end
      end

=begin
def commented_out
=end
    end
  end
end

RSpec.describe Demo::Math::Calc do
  def helper(value)
    value
  end

  # @vtest.id TEST-RB-ADD
  # @vtest.covers VO-RB-ADD
  # @vtest.target lib/calc.rb::Demo::Math::Calc::add
  # @vtest.intent adding two numbers returns their sum
  it "adds two numbers" do
    expect(described_class.new.add(1, 2)).to eq(3)
  end
end
"#;

    fn locators(outcome: &vtest_adapter_api::DiscoveryOutcome) -> Vec<&str> {
        outcome
            .sources
            .iter()
            .map(|source| source.locator.value.as_str())
            .collect()
    }

    /// @vtest.id TEST-ADAPTER-RUBY-DISCOVERS-METHODS-AND-TESTS
    /// @vtest.covers VO-ADAPTER-RUBY-REGISTERS-WITHOUT-CORE-CHANGES, VO-ADAPTER-LEXICAL-OPAQUE-LOCATOR-SYNTAX
    /// @vtest.target crates/vtest-adapter-lexical/src/ruby.rs::Ruby::extract
    /// @vtest.intent verifies Ruby methods (predicate, singleton, endless, operator, private) become module- and class-qualified Source Targets while modifier ifs, loop dos, keyword-named methods and symbols, and =begin blocks do not unbalance bodies, and an annotated RSpec example is a Test construct ending at its end
    #[test]
    fn discovers_ruby_methods_and_examples() {
        let outcome = discover_text(&Ruby, "lib/calc.rb", SOURCE, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(
            locators(&outcome),
            vec![
                "lib/calc.rb::Demo::Math::Calc::add",
                "lib/calc.rb::Demo::Math::Calc::self.create",
                "lib/calc.rb::Demo::Math::Calc::==",
                "lib/calc.rb::Demo::Math::Calc::empty?",
                "lib/calc.rb::Demo::Math::Calc::secret",
                "lib/calc.rb::helper",
            ]
        );
        assert_eq!(
            outcome.sources[0].src_id.as_ref().map(|id| id.as_str()),
            Some("SRC-RB-ADD")
        );
        assert!(outcome.sources[0]
            .construct_text
            .ends_with("result\n      end"));
        assert_eq!(outcome.sources[1].construct_text, "def self.create = new");
        assert_eq!(outcome.tests.len(), 1);
        let test = &outcome.tests[0];
        assert_eq!(test.id.as_str(), "TEST-RB-ADD");
        assert_eq!(test.location.locator, "it#1");
        assert!(test
            .construct_text
            .starts_with("it \"adds two numbers\" do"));
        assert!(test.construct_text.ends_with("eq(3)\n  end"));
    }

    /// @vtest.id TEST-ADAPTER-RUBY-END-IN-TEXT-IS-NOT-THE-END
    /// @vtest.covers VO-ADAPTER-RUBY-REGISTERS-WITHOUT-CORE-CHANGES
    /// @vtest.target crates/vtest-adapter-lexical/src/ruby.rs::Reader::walk
    /// @vtest.intent verifies that end, do, and if inside heredocs, percent literals, interpolations, and regular expressions are not keywords, so a Test construct runs to the end that closes it and an edit to only the body's tail changes its text, and that a method chain continued on the next line belongs to the statement
    #[test]
    fn end_in_text_is_not_the_end() {
        let source = |tail: &str| {
            format!(
                "class PickTest < Minitest::Test\n  # @vtest.id TEST-RB-PICK\n  # @vtest.covers VO-RB-ADD\n  # @vtest.intent the whole method is the construct\n  def test_pick\n    text = <<~EOS\n      end if do\n    EOS\n    words = %w[end do if]\n    label = \"#{{words.map do |w| w end}}\"\n    assert_match(/end|do/, label)\n    {tail}\n  end\n\n  def after; end\nend\n\n# @vtest.id TEST-RB-CHAIN\n# @vtest.covers VO-RB-ADD\n# @vtest.intent a chained statement is a construct\nexpect(value)\n  .to eq(1)\n"
            )
        };
        let outcome = discover_text(&Ruby, "test/pick_test.rb", &source("one"), "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(
            locators(&outcome),
            vec![
                "test/pick_test.rb::PickTest::test_pick",
                "test/pick_test.rb::PickTest::after",
            ]
        );
        let test = &outcome.tests[0];
        assert!(test.construct_text.starts_with("def test_pick"));
        assert!(test.construct_text.ends_with("one\n  end"));
        assert_eq!(
            outcome.tests[1].construct_text,
            "expect(value)\n  .to eq(1)"
        );
        let edited = discover_text(&Ruby, "test/pick_test.rb", &source("two"), "demo");
        assert_ne!(edited.tests[0].construct_text, test.construct_text);
    }

    /// @vtest.id TEST-ADAPTER-RUBY-UNBALANCED-FILES-ARE-UNREADABLE
    /// @vtest.covers VO-ADAPTER-RUBY-REGISTERS-WITHOUT-CORE-CHANGES
    /// @vtest.target crates/vtest-adapter-lexical/src/ruby.rs::Reader::walk
    /// @vtest.intent verifies that a missing end, an extra end, and an unterminated heredoc each make the file unreadable instead of producing wrong ranges
    #[test]
    fn unbalanced_files_are_unreadable() {
        for source in [
            "def open\n  1\n",
            "def closed\nend\nend\n",
            "x = <<~EOS\n  text\n",
        ] {
            let outcome = discover_text(&Ruby, "lib/broken.rb", source, "demo");
            assert_eq!(outcome.diagnostics.len(), 1, "{source:?}");
            assert_eq!(outcome.diagnostics[0].code.as_str(), "E-SCAN-001");
        }
    }

    /// @vtest.id TEST-ADAPTER-RUBY-BEGIN-EXPRESSION-AND-ENDLESS-DEFS
    /// @vtest.covers VO-ADAPTER-RUBY-REGISTERS-WITHOUT-CORE-CHANGES
    /// @vtest.target crates/vtest-adapter-lexical/src/ruby.rs::Reader::method_name
    /// @vtest.intent verifies that `x=begin` inside a line is an expression, not a block comment, and that an endless method `def f = (1)` is named `f` while a setter `def g=(x)` is named `g=`
    #[test]
    fn begin_expression_and_endless_defs() {
        let outcome = discover_text(
            &Ruby,
            "fixture.rb",
            "def f\n  x=begin\n    1\n  end\n  x\nend\ndef g = (1)\ndef h=(x)\n  x\nend\n",
            "demo",
        );
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(
            locators(&outcome),
            vec!["fixture.rb::f", "fixture.rb::g", "fixture.rb::h="]
        );
        assert!(outcome.sources[0].construct_text.ends_with("  x\nend"));
    }
    /// @vtest.id TEST-ADAPTER-RUBY-ENDLESS-EXPRESSIONS-AND-LABELS
    /// @vtest.covers VO-ADAPTER-LEXICAL-RETURNS-SOURCE-RANGE-AND-BYTES
    /// @vtest.target crates/vtest-adapter-lexical/src/ruby.rs::Reader::walk
    /// @vtest.intent verifies an endless method whose body is an if, case, or begin expression runs to that expression's end so a change at its end changes the Source, and a hash label written without a space is not a keyword while a ternary without spaces still parses
    #[test]
    fn endless_expressions_and_labels() {
        let range = |source: &str| {
            let outcome = discover_text(&Ruby, "lib/a.rb", source, "demo");
            assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
            let found = &outcome.sources[0];
            assert_eq!(found.locator.value, "lib/a.rb::f");
            let range = &found.location.byte_range;
            source[range.start as usize..range.end as usize].to_owned()
        };
        for source in [
            "def f = if true\n  1\nelse\n  2\nend\n",
            "def f = case x\nwhen 1 then 1\nelse 2\nend\n",
            "def f = begin\n  1\nrescue\n  2\nend\n",
        ] {
            assert!(range(source).ends_with("end"), "{source:?}");
            assert_ne!(
                range(source),
                range(&source.replace('2', "3")),
                "{source:?}"
            );
        }
        for source in [
            "def f\n  {if:1}\nend\n",
            "def f\n  {if: 1}\nend\n",
            "def f\n  {class:\"x\", while:2}\nend\n",
            "def f\n  x ? a:b\n  if y then 1 end\nend\n",
            "def f\n  [:if, :class]\nend\n",
        ] {
            assert!(range(source).ends_with("end"), "{source:?}");
        }
    }

    /// @vtest.id TEST-ADAPTER-RUBY-HEREDOCS-OPERATOR-NAMES-AND-TRUNCATED-FILES
    /// @vtest.covers VO-ADAPTER-LEXICAL-RETURNS-SOURCE-RANGE-AND-BYTES
    /// @vtest.target crates/vtest-adapter-lexical/src/ruby.rs::Reader::statement_end
    /// @vtest.intent verifies that a heredoc body is part of the construct that opens it so an edit to it changes the Test text, that a lower-case heredoc label hides its body from the code, that unterminated percent literals and regular expressions and a file ending while an expression is awaited are unreadable instead of yielding a Test, and that spaced setters and unary operator methods keep their names
    #[test]
    fn heredocs_operator_names_and_truncated_files() {
        let declare = "# @vtest.id TEST-X\n# @vtest.covers VO-X\n# @vtest.intent review boundary\n";
        for body in [
            "def f = <<~TEXT\n  tail\nTEXT\n",
            "assert_equal <<~TEXT, actual\n  tail\nTEXT\n",
            "it \"a\" do\n  x = <<-EOS\n    tail\n  EOS\nend\n",
        ] {
            let text = |source: &str| {
                let outcome = discover_text(&Ruby, "test/a_test.rb", source, "demo");
                assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
                assert_eq!(outcome.tests.len(), 1, "{body:?}");
                outcome.tests[0].construct_text.clone()
            };
            let one = format!("{declare}{body}");
            assert_eq!(text(&one), body.trim_end(), "{body:?}");
            assert_ne!(
                text(&one),
                text(&one.replace("tail", "tail_changed")),
                "{body:?}"
            );
        }

        let hidden = format!("text = <<text\n{declare}def fake\n  1\nend\ntext\n");
        let outcome = discover_text(&Ruby, "fixture.rb", &hidden, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert!(outcome.tests.is_empty());
        assert!(outcome.sources.is_empty());
        let shift = discover_text(
            &Ruby,
            "fixture.rb",
            "def f\n  a = []\n  a <<b\nend\n",
            "demo",
        );
        assert_eq!(locators(&shift), vec!["fixture.rb::f"]);

        for tail in [
            "def f\n  x = %q!unterminated\n  tail\nend\n",
            "def f\n  x = /unterminated\n  tail\nend\n",
            "def f =\n",
            "expect(value).to eq(1) +\n",
        ] {
            let outcome = discover_text(&Ruby, "fixture.rb", &format!("{declare}{tail}"), "demo");
            assert_eq!(outcome.diagnostics.len(), 1, "{tail:?}");
            assert_eq!(outcome.diagnostics[0].code.as_str(), "E-SCAN-001");
            assert!(outcome.tests.is_empty(), "{tail:?}");
            assert!(outcome.sources.is_empty(), "{tail:?}");
        }

        for (source, expected) in [
            (
                "class Box\n  def value= value\n    @value = value\n  end\nend\n",
                "fixture.rb::Box::value=",
            ),
            (
                "class Box\n  def value= (value)\n    @value = value\n  end\nend\n",
                "fixture.rb::Box::value=",
            ),
            (
                "class Box\n  def value=(value)\n    @value = value\n  end\nend\n",
                "fixture.rb::Box::value=",
            ),
            (
                "class Box\n  def +@\n    self\n  end\nend\n",
                "fixture.rb::Box::+@",
            ),
            (
                "class Box\n  def -@\n    self\n  end\nend\n",
                "fixture.rb::Box::-@",
            ),
            (
                "class Box\n  def +(other)\n    self\n  end\nend\n",
                "fixture.rb::Box::+",
            ),
        ] {
            let outcome = discover_text(&Ruby, "fixture.rb", source, "demo");
            assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
            assert_eq!(locators(&outcome), vec![expected], "{source:?}");
        }
    }

    /// @vtest.id TEST-ADAPTER-RUBY-ARGUMENT-LITERALS-ENCLOSING-ENDS-AND-OPERATOR-NAMES
    /// @vtest.covers VO-ADAPTER-LEXICAL-RETURNS-SOURCE-RANGE-AND-BYTES
    /// @vtest.target crates/vtest-adapter-lexical/src/ruby.rs::Reader::statement_end
    /// @vtest.intent verifies that a lower-case heredoc passed as a method argument hides its body from the code while `<<b` after a local variable is a shift that keeps the Tests after it, even with a later `b` line, that an unterminated literal passed as a method argument and an expression cut off by the enclosing `end` or `}` are not read as a whole Test, that an endless range may end an endless method, and that `[]=` and singleton unary operator methods keep their names
    #[test]
    fn argument_literals_enclosing_ends_and_operator_names() {
        let declare = "# @vtest.id TEST-X\n# @vtest.covers VO-X\n# @vtest.intent review\n";
        let hidden = format!("assert_equal <<text, actual\n{declare}def fake\n  1\nend\ntext\n");
        let outcome = discover_text(&Ruby, "fixture.rb", &hidden, "demo");
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert!(outcome.tests.is_empty());
        assert!(outcome.sources.is_empty());

        // A local variable makes `<<b` a shift even with a later `b` line.
        for (open, close) in [
            ("a = []", ""),
            ("a ||= []", ""),
            ("items.each do |a|", "end"),
            ("def m(x, a)", "end"),
            ("for a in list", "end"),
        ] {
            let source = format!(
                "{open}\n  x = a <<b\n{close}\n\n{declare}def test_f\n  assert true\nend\n\nb\n"
            );
            let outcome = discover_text(&Ruby, "fixture.rb", &source, "demo");
            assert_eq!(
                outcome.tests.len(),
                1,
                "{open:?}: {:?}",
                outcome.diagnostics
            );
            assert!(outcome.tests[0].construct_text.starts_with("def test_f"));
        }
        // Without one it is a heredoc, which needs its terminator line.
        let unterminated = discover_text(
            &Ruby,
            "fixture.rb",
            &format!("x = a <<b\n{declare}def test_f\n  assert true\nend\n"),
            "demo",
        );
        assert_eq!(unterminated.diagnostics.len(), 1);
        assert_eq!(unterminated.diagnostics[0].code.as_str(), "E-SCAN-001");
        assert!(unterminated.tests.is_empty());

        for source in [
            format!("{declare}def f\n  assert_equal %q!unterminated\n  tail\nend\n"),
            format!("{declare}def f\n  assert_match /unterminated\n  tail\nend\n"),
            format!("class C\n{declare}def f =\nend\n"),
            format!("describe do\n{declare}expect(value).to eq(1) +\nend\n"),
            format!("describe {{\n{declare}expect(value).to eq(1) +\n}}\n"),
            format!("if x\n{declare}y = 1 +\nelse\n  z\nend\n"),
        ] {
            let outcome = discover_text(&Ruby, "fixture.rb", &source, "demo");
            assert!(outcome.tests.is_empty(), "{source:?}");
            assert!(!outcome.diagnostics.is_empty(), "{source:?}");
        }

        let range = discover_text(
            &Ruby,
            "fixture.rb",
            &format!("{declare}def f = 1..\n"),
            "demo",
        );
        assert!(range.diagnostics.is_empty(), "{:?}", range.diagnostics);
        assert_eq!(range.tests[0].construct_text, "def f = 1..");
        let inner = discover_text(
            &Ruby,
            "fixture.rb",
            &format!("class C\n{declare}def f = 1..\nend\n"),
            "demo",
        );
        assert!(inner.diagnostics.is_empty(), "{:?}", inner.diagnostics);
        assert_eq!(inner.tests[0].construct_text, "def f = 1..");

        for (source, expected) in [
            (
                "class Box\n  def []=(key, value)\n    @items[key] = value\n  end\nend\n",
                "fixture.rb::Box::[]=",
            ),
            (
                "class Box\n  def self.+@\n    self\n  end\nend\n",
                "fixture.rb::Box::self.+@",
            ),
        ] {
            let outcome = discover_text(&Ruby, "fixture.rb", source, "demo");
            assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
            assert_eq!(locators(&outcome), vec![expected], "{source:?}");
        }
    }
}
