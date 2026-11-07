//! A small, language-configurable lexer.
//!
//! It recognizes only what the discovery engines need: comments (whose text
//! carries `@vtest.` declarations), preprocessor directives, string literals
//! (so that brackets and comment markers inside them are not mistaken for
//! code), words, brackets, `;`, and newlines. Everything else is a single
//! punctuation character.
//!
//! The lexer is deliberately lenient about single-line string delimiters:
//! a quote that does not close on its own line is treated as punctuation
//! rather than swallowing the rest of the file (for example a C++14 digit
//! separator `1'000`). An unterminated block comment or multi-line string,
//! which cannot be recovered locally, is a [`LexError`]; the adapter reports
//! it as `E-SCAN-001` for that file and discovers nothing from it.

/// How one string or character literal is delimited.
#[derive(Clone, Copy, Debug)]
pub struct StringSyntax {
    pub open: &'static str,
    pub close: &'static str,
    /// Whether `\` escapes the next character.
    pub escape: bool,
    /// Whether the literal may span lines. A single-line literal whose
    /// opening delimiter does not close on the same line is lexed as
    /// punctuation instead (see the module documentation).
    pub multiline: bool,
}

impl StringSyntax {
    pub const fn single(open: &'static str, close: &'static str) -> Self {
        Self {
            open,
            close,
            escape: true,
            multiline: false,
        }
    }

    pub const fn multi(open: &'static str, close: &'static str, escape: bool) -> Self {
        Self {
            open,
            close,
            escape,
            multiline: true,
        }
    }
}

/// How source text is split into blocks of code.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Structure {
    /// `{ ... }` delimits bodies (C, C++, Java, C#, Go, JavaScript, ...).
    Braces,
}

/// The lexical syntax of one language.
#[derive(Clone, Copy, Debug)]
pub struct LexicalSyntax {
    pub line_comments: &'static [&'static str],
    /// Prefixes that look like a line comment but are not one (PHP's `#[`
    /// attribute starts with the `#` line-comment marker).
    pub line_comment_exclusions: &'static [&'static str],
    pub block_comments: &'static [(&'static str, &'static str)],
    pub nested_block_comments: bool,
    /// Checked in order, so a longer delimiter (`"""`) must precede a
    /// shorter one with the same prefix (`"`).
    pub strings: &'static [StringSyntax],
    /// `#` at the start of a line begins a directive running to the end of
    /// the line, with `\` line continuation (C and C++).
    pub preprocessor: bool,
    /// C++ raw string literals `R"delim(...)delim"` (with optional
    /// `u8`/`u`/`U`/`L` prefix).
    pub cpp_raw_strings: bool,
    /// Characters a word may end with (Ruby's `empty?` / `save!`).
    pub word_suffix_chars: &'static [char],
    /// Whether a newline can end a statement (Go, Kotlin, Swift,
    /// JavaScript/TypeScript). See `braces::construct_end`.
    pub newline_terminates: bool,
    pub structure: Structure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokKind {
    Comment,
    Directive,
    Str,
    Word,
    Open(char),
    Close(char),
    Semi,
    Newline,
    Punct(char),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Tok {
    pub kind: TokKind,
    pub start: usize,
    pub end: usize,
}

impl Tok {
    pub fn text<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }

    /// Comments, directives, and newlines carry no code.
    pub fn is_trivia(&self) -> bool {
        matches!(
            self.kind,
            TokKind::Comment | TokKind::Directive | TokKind::Newline
        )
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct LexError {
    pub offset: usize,
    pub message: String,
}

pub fn lex(source: &str, syntax: &LexicalSyntax) -> Result<Vec<Tok>, LexError> {
    let mut tokens = Vec::new();
    let mut index = 0usize;
    let mut line_start = true;
    while index < source.len() {
        let rest = &source[index..];
        let Some(character) = rest.chars().next() else {
            break;
        };
        if character == '\n' {
            tokens.push(tok(TokKind::Newline, index, index + 1));
            index += 1;
            line_start = true;
            continue;
        }
        if character.is_whitespace() {
            index += character.len_utf8();
            continue;
        }
        let at_line_start = line_start;
        line_start = false;

        if syntax.preprocessor && at_line_start && character == '#' {
            let end = directive_end(source, index);
            tokens.push(tok(TokKind::Directive, index, end));
            index = end;
            continue;
        }
        if let Some((open, close)) = syntax
            .block_comments
            .iter()
            .find(|(open, _)| rest.starts_with(open))
        {
            let end = block_comment_end(source, index, open, close, syntax)?;
            tokens.push(tok(TokKind::Comment, index, end));
            index = end;
            continue;
        }
        if syntax
            .line_comments
            .iter()
            .any(|prefix| rest.starts_with(prefix))
            && !syntax
                .line_comment_exclusions
                .iter()
                .any(|prefix| rest.starts_with(prefix))
        {
            let end = rest
                .find('\n')
                .map_or(source.len(), |offset| index + offset);
            tokens.push(tok(TokKind::Comment, index, end));
            index = end;
            continue;
        }
        if let Some(string) = syntax
            .strings
            .iter()
            .find(|string| rest.starts_with(string.open))
        {
            match string_end(source, index, string) {
                Some(end) => {
                    tokens.push(tok(TokKind::Str, index, end));
                    index = end;
                }
                None if string.multiline => {
                    return Err(LexError {
                        offset: index,
                        message: format!("unterminated string literal `{}`", string.open),
                    });
                }
                None => {
                    // Lenient single-line fallback (module documentation).
                    let first = string.open.chars().next().unwrap_or(character);
                    tokens.push(tok(TokKind::Punct(first), index, index + first.len_utf8()));
                    index += first.len_utf8();
                }
            }
            continue;
        }
        if is_word_char(character) {
            let mut end = index;
            for (offset, next) in rest.char_indices() {
                if is_word_char(next) {
                    end = index + offset + next.len_utf8();
                } else {
                    break;
                }
            }
            if let Some(suffix) = source[end..].chars().next() {
                if syntax.word_suffix_chars.contains(&suffix) {
                    end += suffix.len_utf8();
                }
            }
            let word = &source[index..end];
            if syntax.cpp_raw_strings
                && matches!(word, "R" | "u8R" | "uR" | "UR" | "LR")
                && source[end..].starts_with('"')
            {
                let raw_end = cpp_raw_string_end(source, end).ok_or(LexError {
                    offset: index,
                    message: "unterminated raw string literal".to_owned(),
                })?;
                tokens.push(tok(TokKind::Str, index, raw_end));
                index = raw_end;
                continue;
            }
            tokens.push(tok(TokKind::Word, index, end));
            index = end;
            continue;
        }
        let kind = match character {
            '(' | '[' | '{' => TokKind::Open(character),
            ')' | ']' | '}' => TokKind::Close(character),
            ';' => TokKind::Semi,
            other => TokKind::Punct(other),
        };
        tokens.push(tok(kind, index, index + character.len_utf8()));
        index += character.len_utf8();
    }
    Ok(tokens)
}

fn tok(kind: TokKind, start: usize, end: usize) -> Tok {
    Tok { kind, start, end }
}

pub fn is_word_char(character: char) -> bool {
    character == '_' || character == '$' || character.is_alphanumeric()
}

fn directive_end(source: &str, start: usize) -> usize {
    let bytes = source.as_bytes();
    let mut index = start;
    while index < bytes.len() {
        if bytes[index] == b'\n' {
            let mut back = index;
            while back > start && (bytes[back - 1] == b'\r' || bytes[back - 1] == b' ') {
                back -= 1;
            }
            if back > start && bytes[back - 1] == b'\\' {
                index += 1;
                continue;
            }
            return index;
        }
        index += 1;
    }
    bytes.len()
}

fn block_comment_end(
    source: &str,
    start: usize,
    open: &str,
    close: &str,
    syntax: &LexicalSyntax,
) -> Result<usize, LexError> {
    let mut depth = 1usize;
    let mut index = start + open.len();
    while index < source.len() {
        let rest = &source[index..];
        if rest.starts_with(close) {
            depth -= 1;
            index += close.len();
            if depth == 0 {
                return Ok(index);
            }
            continue;
        }
        if syntax.nested_block_comments && rest.starts_with(open) {
            depth += 1;
            index += open.len();
            continue;
        }
        index += rest.chars().next().map_or(1, char::len_utf8);
    }
    Err(LexError {
        offset: start,
        message: format!("unterminated block comment `{open}`"),
    })
}

fn string_end(source: &str, start: usize, string: &StringSyntax) -> Option<usize> {
    let mut index = start + string.open.len();
    while index < source.len() {
        let rest = &source[index..];
        if rest.starts_with(string.close) {
            return Some(index + string.close.len());
        }
        let character = rest.chars().next()?;
        if character == '\n' && !string.multiline {
            return None;
        }
        if string.escape && character == '\\' {
            index += 1;
            if let Some(next) = source[index..].chars().next() {
                if next == '\n' && !string.multiline {
                    // A line continuation inside a single-line literal.
                    index += 1;
                    continue;
                }
                index += next.len_utf8();
            }
            continue;
        }
        index += character.len_utf8();
    }
    None
}

fn cpp_raw_string_end(source: &str, quote: usize) -> Option<usize> {
    let after_quote = quote + 1;
    let paren = source[after_quote..].find('(')? + after_quote;
    let delimiter = &source[after_quote..paren];
    if delimiter.len() > 16 || delimiter.contains(char::is_whitespace) {
        return None;
    }
    let terminator = format!("){delimiter}\"");
    source[paren..]
        .find(&terminator)
        .map(|offset| paren + offset + terminator.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    const C_LIKE: LexicalSyntax = LexicalSyntax {
        line_comments: &["//"],
        line_comment_exclusions: &[],
        block_comments: &[("/*", "*/")],
        nested_block_comments: false,
        strings: &[
            StringSyntax::single("\"", "\""),
            StringSyntax::single("'", "'"),
        ],
        preprocessor: true,
        cpp_raw_strings: true,
        word_suffix_chars: &[],
        newline_terminates: false,
        structure: Structure::Braces,
    };

    fn kinds(source: &str) -> Vec<TokKind> {
        lex(source, &C_LIKE)
            .expect("lexes")
            .into_iter()
            .map(|token| token.kind)
            .collect()
    }

    /// @vtest.id TEST-ADAPTER-LEXICAL-LEXER-HIDES-BRACKETS-IN-LITERALS
    /// @vtest.covers VO-ADAPTER-LEXICAL-RETURNS-SOURCE-RANGE-AND-BYTES
    /// @vtest.target crates/vtest-adapter-lexical/src/lexer.rs::lex
    /// @vtest.intent verifies brackets inside strings, comments, raw strings, and directives are not lexed as code brackets
    #[test]
    fn brackets_inside_literals_comments_and_directives_are_not_code() {
        let source = "#define X { \\\n }\nf(\"{\", '}', R\"x(})x\") /* { */ // }\n";
        let kinds = kinds(source);
        assert_eq!(
            kinds,
            vec![
                TokKind::Directive,
                TokKind::Newline,
                TokKind::Word,
                TokKind::Open('('),
                TokKind::Str,
                TokKind::Punct(','),
                TokKind::Str,
                TokKind::Punct(','),
                TokKind::Str,
                TokKind::Close(')'),
                TokKind::Comment,
                TokKind::Comment,
                TokKind::Newline,
            ]
        );
    }

    /// @vtest.id TEST-ADAPTER-LEXICAL-LEXER-UNTERMINATED-COMMENT-ERROR
    /// @vtest.covers VO-ADAPTER-LEXICAL-RETURNS-SOURCE-RANGE-AND-BYTES
    /// @vtest.target crates/vtest-adapter-lexical/src/lexer.rs::lex
    /// @vtest.intent verifies an unterminated block comment is a lex error while an unclosed single-line quote degrades to punctuation
    #[test]
    fn unterminated_block_comment_is_an_error_but_a_stray_quote_is_not() {
        assert!(lex("int x; /* never closed", &C_LIKE).is_err());
        let kinds = kinds("int x = 1'000;\n");
        assert!(kinds.contains(&TokKind::Semi));
    }
}
