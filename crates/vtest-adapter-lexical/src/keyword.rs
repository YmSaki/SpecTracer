//! Construct extents and function discovery for keyword-structured
//! languages (Ruby), where `def`, `class`, `module`, `do`, `begin`, `case`,
//! and statement-leading `if`/`unless`/`while`/`until`/`for` open a body
//! that `end` closes.
//!
//! Modifier forms (`return x if y`) do not open a body, a `do` after a
//! `while`/`until`/`for` on the same line belongs to that loop, an endless
//! method (`def name(args) = expr`) has no `end`, and a keyword used as a
//! method name or symbol (`range.end`, `x.class`, `:end`, `class:`) is not a
//! keyword.

use crate::{
    braces::FunctionSite,
    header::{HTok, HeaderKind},
    lexer::{LexError, Tok, TokKind},
};

const OPENERS: &[&str] = &["def", "class", "module", "case", "begin", "do", "for"];
const STATEMENT_OPENERS: &[&str] = &["if", "unless", "while", "until"];
const LOOPS: &[&str] = &["while", "until", "for"];
const STATEMENT_LEADING_WORDS: &[&str] = &[
    "then", "do", "else", "elsif", "and", "or", "not", "return", "begin", "when", "in",
];

fn previous_significant(tokens: &[Tok], index: usize) -> Option<usize> {
    (0..index).rev().find(|&candidate| {
        !matches!(
            tokens[candidate].kind,
            TokKind::Comment | TokKind::Directive
        )
    })
}

fn next_significant(tokens: &[Tok], index: usize) -> Option<usize> {
    (index + 1..tokens.len()).find(|&candidate| {
        !matches!(
            tokens[candidate].kind,
            TokKind::Comment | TokKind::Directive
        )
    })
}

/// Whether the word at `index` is used as a keyword, not as a method name,
/// symbol, or hash key.
fn is_keyword_use(tokens: &[Tok], index: usize) -> bool {
    let token = tokens[index];
    if let Some(previous) = previous_significant(tokens, index) {
        let adjacent = tokens[previous].end == token.start;
        if matches!(tokens[previous].kind, TokKind::Punct('.'))
            || (adjacent && tokens[previous].kind == TokKind::Punct(':'))
        {
            return false;
        }
    }
    if let Some(next) = next_significant(tokens, index) {
        let adjacent = tokens[next].start == token.end;
        let double_colon = tokens
            .get(next + 1)
            .is_some_and(|after| after.kind == TokKind::Punct(':'));
        if adjacent && tokens[next].kind == TokKind::Punct(':') && !double_colon {
            return false;
        }
    }
    true
}

/// Whether the token at `index` begins a statement.
fn at_statement_start(tokens: &[Tok], source: &str, index: usize) -> bool {
    let Some(previous) = previous_significant(tokens, index) else {
        return true;
    };
    match tokens[previous].kind {
        TokKind::Newline | TokKind::Semi | TokKind::Open(_) => true,
        TokKind::Punct(character) => "=,|&!(".contains(character),
        TokKind::Word => STATEMENT_LEADING_WORDS.contains(&tokens[previous].text(source)),
        _ => false,
    }
}

/// `def name(args) = expr` (Ruby 3 endless method) has no `end`.
fn is_endless_def(tokens: &[Tok], def: usize) -> bool {
    let mut index = def + 1;
    // `self.` / `Const.` receiver.
    if tokens
        .get(index + 1)
        .is_some_and(|t| t.kind == TokKind::Punct('.'))
        && tokens.get(index).is_some_and(|t| t.kind == TokKind::Word)
    {
        index += 2;
    }
    // The name: a word (`name`, `name=`, `name?`) or operator characters.
    match tokens.get(index).map(|t| t.kind) {
        Some(TokKind::Word) => {
            index += 1;
            // A setter name `value=(v)` is `value` `=` `(`: not endless.
            if tokens
                .get(index)
                .is_some_and(|t| t.kind == TokKind::Punct('='))
                && tokens
                    .get(index + 1)
                    .is_some_and(|t| t.kind == TokKind::Open('('))
                && tokens[index].start == tokens[index - 1].end
            {
                index += 1;
            }
        }
        Some(TokKind::Punct(_)) | Some(TokKind::Open('[')) => {
            while tokens.get(index).is_some_and(|t| {
                matches!(
                    t.kind,
                    TokKind::Punct(_) | TokKind::Open('[') | TokKind::Close(']')
                )
            }) {
                index += 1;
            }
        }
        _ => return false,
    }
    if tokens
        .get(index)
        .is_some_and(|t| t.kind == TokKind::Open('('))
    {
        let mut depth = 0usize;
        while let Some(token) = tokens.get(index) {
            match token.kind {
                TokKind::Open(_) => depth += 1,
                TokKind::Close(_) => {
                    depth -= 1;
                    if depth == 0 {
                        index += 1;
                        break;
                    }
                }
                _ => {}
            }
            index += 1;
        }
    }
    tokens
        .get(index)
        .is_some_and(|t| t.kind == TokKind::Punct('='))
        && tokens
            .get(index + 1)
            .is_none_or(|t| !matches!(t.kind, TokKind::Punct('=') | TokKind::Punct('~')))
}

/// For each opener (keyword or bracket) its closer, and for each closer its
/// opener. Unbalanced bodies are an error.
pub fn pairs(tokens: &[Tok], source: &str) -> Result<Vec<Option<usize>>, LexError> {
    let mut partners = vec![None; tokens.len()];
    let mut stack: Vec<usize> = Vec::new();
    let mut loop_awaiting_do: Option<usize> = None;
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            TokKind::Newline | TokKind::Semi => loop_awaiting_do = None,
            TokKind::Open(_) => stack.push(index),
            TokKind::Close(close) => {
                let open = stack
                    .pop()
                    .filter(|&open| tokens[open].kind != TokKind::Word);
                let Some(open) = open else {
                    return Err(LexError {
                        offset: token.start,
                        message: format!("unmatched `{close}`"),
                    });
                };
                partners[open] = Some(index);
                partners[index] = Some(open);
            }
            TokKind::Word => {
                let word = token.text(source);
                let opens = if word == "end" {
                    if !is_keyword_use(tokens, index) {
                        continue;
                    }
                    let open = stack
                        .pop()
                        .filter(|&open| tokens[open].kind == TokKind::Word);
                    let Some(open) = open else {
                        return Err(LexError {
                            offset: token.start,
                            message: "unmatched `end`".to_owned(),
                        });
                    };
                    partners[open] = Some(index);
                    partners[index] = Some(open);
                    continue;
                } else if word == "do" {
                    if loop_awaiting_do.take().is_some() {
                        false
                    } else {
                        is_keyword_use(tokens, index)
                    }
                } else if word == "def" {
                    is_keyword_use(tokens, index) && !is_endless_def(tokens, index)
                } else if OPENERS.contains(&word) {
                    is_keyword_use(tokens, index)
                        && (word != "for" || at_statement_start(tokens, source, index))
                } else if STATEMENT_OPENERS.contains(&word) {
                    is_keyword_use(tokens, index) && at_statement_start(tokens, source, index)
                } else {
                    false
                };
                if opens {
                    if LOOPS.contains(&word) {
                        loop_awaiting_do = Some(index);
                    }
                    stack.push(index);
                }
            }
            _ => {}
        }
    }
    if let Some(open) = stack.pop() {
        return Err(LexError {
            offset: tokens[open].start,
            message: format!("`{}` is never closed", tokens[open].text(source)),
        });
    }
    Ok(partners)
}

fn is_closer(partners: &[Option<usize>], index: usize) -> bool {
    partners[index].is_some_and(|open| open < index)
}

/// The one-past-last token of the statement starting at `start`.
pub fn construct_end(
    tokens: &[Tok],
    partners: &[Option<usize>],
    start: usize,
    limit: usize,
) -> usize {
    let mut index = start;
    let mut last = start;
    while index < limit {
        let token = tokens[index];
        match token.kind {
            TokKind::Comment | TokKind::Directive => {}
            TokKind::Semi => return index + 1,
            TokKind::Newline => {
                let continued = matches!(
                    tokens[last].kind,
                    TokKind::Punct(c) if ".,\\=+-*/|&<>?:".contains(c)
                );
                let next_dot =
                    next_significant_skipping_newlines(tokens, index).is_some_and(|next| {
                        matches!(tokens[next].kind, TokKind::Punct('.') | TokKind::Punct('&'))
                    });
                if !continued && !next_dot {
                    return last + 1;
                }
            }
            _ if is_closer(partners, index) => return last + 1,
            _ => {
                if let Some(close) = partners[index].filter(|&close| close > index) {
                    index = close;
                }
                last = index;
            }
        }
        index += 1;
    }
    last + 1
}

fn next_significant_skipping_newlines(tokens: &[Tok], index: usize) -> Option<usize> {
    (index + 1..tokens.len()).find(|&candidate| !tokens[candidate].is_trivia())
}

/// Every method definition the profile's `classify` recognizes, qualified by
/// the enclosing `class`/`module` names. Method bodies are not searched;
/// other blocks (`describe ... do`, `class << self`) are.
pub fn functions(
    tokens: &[Tok],
    source: &str,
    partners: &[Option<usize>],
    classify: &dyn Fn(&[HTok<'_>], bool) -> HeaderKind,
) -> Vec<FunctionSite> {
    let mut output = Vec::new();
    let mut scope = Vec::new();
    walk(
        tokens,
        source,
        partners,
        classify,
        0,
        tokens.len(),
        &mut scope,
        &mut output,
    );
    output
}

#[allow(clippy::too_many_arguments)]
fn walk(
    tokens: &[Tok],
    source: &str,
    partners: &[Option<usize>],
    classify: &dyn Fn(&[HTok<'_>], bool) -> HeaderKind,
    low: usize,
    high: usize,
    scope: &mut Vec<String>,
    output: &mut Vec<FunctionSite>,
) {
    let mut index = low;
    while index < high {
        if tokens[index].is_trivia() || tokens[index].kind == TokKind::Semi {
            index += 1;
            continue;
        }
        let start = index;
        let end = construct_end(tokens, partners, start, high).max(start + 1);
        let header_end = (start..end)
            .find(|&candidate| tokens[candidate].kind == TokKind::Newline)
            .unwrap_or(end);
        let header = tokens[start..header_end]
            .iter()
            .filter(|token| !token.is_trivia())
            .map(|token| HTok {
                kind: token.kind,
                text: token.text(source),
            })
            .collect::<Vec<_>>();
        let opener = (start..end).find(|&candidate| {
            partners[candidate].is_some_and(|close| close > candidate)
                && tokens[candidate].kind == TokKind::Word
        });
        match classify(&header, opener.is_some()) {
            HeaderKind::Function(name) => {
                let mut qualified = scope.clone();
                qualified.extend(name);
                output.push(FunctionSite {
                    start_tok: start,
                    end_tok: end,
                    name: qualified,
                });
            }
            kind => {
                if let Some(open) = opener {
                    let close = partners[open].unwrap_or(open);
                    let restore = scope.len();
                    if let HeaderKind::Scope(name) = kind {
                        scope.extend(name);
                    }
                    walk(
                        tokens,
                        source,
                        partners,
                        classify,
                        open + 1,
                        close,
                        scope,
                        output,
                    );
                    scope.truncate(restore);
                }
            }
        }
        index = end;
    }
}
