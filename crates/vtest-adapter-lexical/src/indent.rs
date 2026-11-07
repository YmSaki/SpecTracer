//! Construct extents and function discovery for indentation-structured
//! languages (Python).
//!
//! The source is split into logical lines: a newline inside brackets or after
//! a `\` continuation does not end a line, and comment-only lines do not
//! count. A construct that starts a logical line extends over any decorator
//! lines that start it and then over every following logical line indented
//! deeper than its first line.

use crate::{
    braces::FunctionSite,
    header::{HTok, HeaderKind},
    lexer::{Tok, TokKind},
};

/// One logical line: its first and last significant token and the column of
/// its first token.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Line {
    pub first: usize,
    pub last: usize,
    pub indent: usize,
}

pub fn logical_lines(tokens: &[Tok], source: &str) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut depth = 0usize;
    let mut current: Option<(usize, usize)> = None;
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            TokKind::Comment | TokKind::Directive => continue,
            TokKind::Newline => {
                let continued =
                    current.is_some_and(|(_, last)| tokens[last].kind == TokKind::Punct('\\'));
                if depth == 0 && !continued {
                    if let Some((first, last)) = current.take() {
                        lines.push(line(tokens, source, first, last));
                    }
                }
                continue;
            }
            TokKind::Open(_) => depth += 1,
            TokKind::Close(_) => depth = depth.saturating_sub(1),
            _ => {}
        }
        current = Some(match current {
            Some((first, _)) => (first, index),
            None => (index, index),
        });
    }
    if let Some((first, last)) = current {
        lines.push(line(tokens, source, first, last));
    }
    lines
}

fn line(tokens: &[Tok], source: &str, first: usize, last: usize) -> Line {
    let start = tokens[first].start;
    let line_start = source[..start].rfind('\n').map_or(0, |offset| offset + 1);
    Line {
        first,
        last,
        indent: source[line_start..start].chars().count(),
    }
}

fn starts_with_decorator(tokens: &[Tok], line: &Line) -> bool {
    tokens[line.first].kind == TokKind::Punct('@')
}

/// The index (into `lines`) of the last line of the construct whose first
/// line is `lines[at]`.
fn last_line(tokens: &[Tok], lines: &[Line], at: usize) -> usize {
    let indent = lines[at].indent;
    let mut index = at;
    while starts_with_decorator(tokens, &lines[index])
        && lines
            .get(index + 1)
            .is_some_and(|next| next.indent == indent)
    {
        index += 1;
    }
    while lines
        .get(index + 1)
        .is_some_and(|next| next.indent > indent)
    {
        index += 1;
    }
    index
}

/// The one-past-last token of the construct starting at token `start`, or
/// `start` when `start` does not begin a logical line.
pub fn construct_end(tokens: &[Tok], lines: &[Line], start: usize) -> usize {
    match lines.binary_search_by_key(&start, |line| line.first) {
        Ok(at) => lines[last_line(tokens, lines, at)].last + 1,
        Err(_) => start,
    }
}

/// Every function definition the profile's `classify` recognizes, qualified
/// by the enclosing named scopes. A decorated definition starts at its first
/// decorator. Function bodies are not searched for nested functions.
pub fn functions(
    tokens: &[Tok],
    source: &str,
    lines: &[Line],
    classify: &dyn Fn(&[HTok<'_>], bool) -> HeaderKind,
) -> Vec<FunctionSite> {
    let mut output = Vec::new();
    let mut scopes: Vec<(usize, Vec<String>)> = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let start_line = lines[index];
        scopes.retain(|(indent, _)| *indent < start_line.indent);
        // Decorators belong to the definition that follows them.
        let mut definition = index;
        while starts_with_decorator(tokens, &lines[definition])
            && lines
                .get(definition + 1)
                .is_some_and(|next| next.indent == start_line.indent)
        {
            definition += 1;
        }
        let header = tokens[lines[definition].first..=lines[definition].last]
            .iter()
            .filter(|token| !token.is_trivia())
            .map(|token| HTok {
                kind: token.kind,
                text: token.text(source),
            })
            .collect::<Vec<_>>();
        let end = last_line(tokens, lines, index);
        match classify(&header, true) {
            HeaderKind::Function(name) => {
                let mut qualified = scopes
                    .iter()
                    .flat_map(|(_, segments)| segments.clone())
                    .collect::<Vec<_>>();
                qualified.extend(name);
                output.push(FunctionSite {
                    start_tok: start_line.first,
                    end_tok: lines[end].last + 1,
                    name: qualified,
                });
                index = end + 1;
            }
            HeaderKind::Scope(name) | HeaderKind::ScopeRest(name) => {
                scopes.push((start_line.indent, name));
                index = definition + 1;
            }
            HeaderKind::Other => index = definition + 1,
        }
    }
    output
}
