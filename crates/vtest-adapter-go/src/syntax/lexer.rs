//! The Go scanner: source text to tokens, with the automatic semicolons the
//! Go specification inserts at a line end.
//!
//! Responsibility: split one Go source text into tokens exactly as the `go`
//! command's scanner does (identifiers, keywords, numbers, runes, strings,
//! operators, comments dropped, semicolon insertion). Boundary: no grammar and
//! no meaning; a text this scanner cannot read is a [`ParseError`], never a
//! guess (DS-615: an unreadable file is never `PASS`). Realizes DES-619.

use super::ParseError;

/// The kind of a [`Token`]. An operator or a keyword is told apart by its text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Tok {
    Ident,
    Int,
    Float,
    Imag,
    Char,
    Str,
    /// An operator or punctuation other than `;`.
    Op,
    Kw,
    /// `;`, written or inserted at a line end (an inserted one has no text).
    Semi,
    Eof,
}

/// One token: its kind and its byte range in the scanned text.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Token {
    pub(crate) kind: Tok,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

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

/// The operators, longest first within each leading character.
const OPERATORS: &[&str] = &[
    "<<=", ">>=", "&^=", "...", "<<", ">>", "&^", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=",
    "&&", "||", "<-", "++", "--", "==", "!=", "<=", ">=", ":=", "+", "-", "*", "/", "%", "&", "|",
    "^", "<", ">", "=", "!", "(", ")", "[", "]", "{", "}", ",", ".", ":", "~",
];

/// The `'static` spelling of the operator `text`, or `""` for a text that is
/// not one.
pub(crate) fn static_operator(text: &str) -> &'static str {
    OPERATORS.iter().find(|op| **op == text).copied().unwrap_or("")
}

fn is_letter(c: char) -> bool {
    c == '_' || c.is_alphabetic()
}

fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}

fn error(pos: usize, message: &str) -> ParseError {
    ParseError {
        pos,
        message: message.to_owned(),
    }
}

/// Scans `src` into tokens ending with [`Tok::Eof`].
pub(crate) fn tokenize(src: &str) -> Result<Vec<Token>, ParseError> {
    let bytes = src.as_bytes();
    let mut tokens: Vec<Token> = Vec::new();
    let mut i = usize::from(src.starts_with('\u{feff}')) * 3;
    // Whether a newline here ends a statement (Go specification, Semicolons).
    let mut insert_semi = false;
    while i < bytes.len() {
        let c = src[i..].chars().next().unwrap_or('\0');
        if c == '\n' {
            if insert_semi {
                tokens.push(Token {
                    kind: Tok::Semi,
                    start: i,
                    end: i,
                });
                insert_semi = false;
            }
            i += 1;
            continue;
        }
        if c == ' ' || c == '\t' || c == '\r' {
            i += 1;
            continue;
        }
        if src[i..].starts_with("//") {
            // The comment runs to the line end; the newline is read next.
            i += src[i..].find('\n').unwrap_or(src.len() - i);
            continue;
        }
        if src[i..].starts_with("/*") {
            let Some(length) = src[i + 2..].find("*/") else {
                return Err(error(i, "unterminated comment"));
            };
            let comment = &src[i..i + 2 + length + 2];
            // A general comment holding a newline acts as a newline.
            if insert_semi && comment.contains('\n') {
                tokens.push(Token {
                    kind: Tok::Semi,
                    start: i,
                    end: i,
                });
                insert_semi = false;
            }
            i += comment.len();
            continue;
        }
        let start = i;
        if is_letter(c) {
            let mut end = i;
            for (offset, ch) in src[i..].char_indices() {
                if is_letter(ch) || ch.is_numeric() {
                    end = i + offset + ch.len_utf8();
                } else {
                    break;
                }
            }
            let text = &src[start..end];
            if KEYWORDS.contains(&text) {
                tokens.push(Token {
                    kind: Tok::Kw,
                    start,
                    end,
                });
                insert_semi = matches!(text, "break" | "continue" | "fallthrough" | "return");
            } else {
                tokens.push(Token {
                    kind: Tok::Ident,
                    start,
                    end,
                });
                insert_semi = true;
            }
            i = end;
            continue;
        }
        if is_digit(c) || (c == '.' && bytes.get(i + 1).is_some_and(u8::is_ascii_digit)) {
            let (kind, end) = scan_number(src, i)?;
            tokens.push(Token { kind, start, end });
            insert_semi = true;
            i = end;
            continue;
        }
        match c {
            '"' => {
                let end = scan_quoted(src, i, '"')?;
                tokens.push(Token {
                    kind: Tok::Str,
                    start,
                    end,
                });
                insert_semi = true;
                i = end;
                continue;
            }
            '\'' => {
                let end = scan_quoted(src, i, '\'')?;
                tokens.push(Token {
                    kind: Tok::Char,
                    start,
                    end,
                });
                insert_semi = true;
                i = end;
                continue;
            }
            '`' => {
                let Some(length) = src[i + 1..].find('`') else {
                    return Err(error(i, "unterminated raw string"));
                };
                let end = i + 1 + length + 1;
                tokens.push(Token {
                    kind: Tok::Str,
                    start,
                    end,
                });
                insert_semi = true;
                i = end;
                continue;
            }
            ';' => {
                tokens.push(Token {
                    kind: Tok::Semi,
                    start,
                    end: start + 1,
                });
                insert_semi = false;
                i += 1;
                continue;
            }
            _ => {}
        }
        let Some(operator) = OPERATORS.iter().find(|op| src[i..].starts_with(**op)) else {
            return Err(error(i, "unexpected character"));
        };
        let end = i + operator.len();
        tokens.push(Token {
            kind: Tok::Op,
            start,
            end,
        });
        insert_semi = matches!(*operator, "++" | "--" | ")" | "]" | "}");
        i = end;
    }
    if insert_semi {
        tokens.push(Token {
            kind: Tok::Semi,
            start: src.len(),
            end: src.len(),
        });
    }
    tokens.push(Token {
        kind: Tok::Eof,
        start: src.len(),
        end: src.len(),
    });
    Ok(tokens)
}

/// A quoted rune or string literal starting at `start`; returns the end.
fn scan_quoted(src: &str, start: usize, quote: char) -> Result<usize, ParseError> {
    let mut chars = src[start + 1..].char_indices();
    while let Some((offset, ch)) = chars.next() {
        if ch == '\\' {
            chars.next();
        } else if ch == quote {
            return Ok(start + 1 + offset + 1);
        } else if ch == '\n' {
            break;
        }
    }
    Err(error(start, "unterminated literal"))
}

/// A number literal starting at `start`; returns its kind and end.
fn scan_number(src: &str, start: usize) -> Result<(Tok, usize), ParseError> {
    let bytes = src.as_bytes();
    let at = |i: usize| bytes.get(i).copied().unwrap_or(0);
    let mut i = start;
    let mut kind = Tok::Int;
    let mut hex = false;
    let mut prefixed = false;
    if at(i) == b'0' {
        match at(i + 1) {
            b'x' | b'X' => {
                hex = true;
                prefixed = true;
                i += 2;
            }
            b'b' | b'B' | b'o' | b'O' => {
                prefixed = true;
                i += 2;
            }
            _ => {}
        }
    }
    let is_digit_here = |b: u8| b == b'_' || if hex { b.is_ascii_hexdigit() } else { b.is_ascii_digit() };
    while is_digit_here(at(i)) {
        i += 1;
    }
    // A fraction and an exponent: not for `0b` and `0o` numbers.
    if !prefixed || hex {
        if at(i) == b'.' {
            kind = Tok::Float;
            i += 1;
            while is_digit_here(at(i)) {
                i += 1;
            }
        }
        let exponent = if hex {
            matches!(at(i), b'p' | b'P')
        } else {
            matches!(at(i), b'e' | b'E')
        };
        if exponent {
            kind = Tok::Float;
            i += 1;
            if matches!(at(i), b'+' | b'-') {
                i += 1;
            }
            let digits_start = i;
            while at(i).is_ascii_digit() || at(i) == b'_' {
                i += 1;
            }
            if i == digits_start {
                return Err(error(start, "malformed exponent"));
            }
        }
    }
    if at(i) == b'i' {
        kind = Tok::Imag;
        i += 1;
    }
    Ok((kind, i))
}
