//! Construct extents and function discovery for brace-structured languages.

use crate::{
    header::{HTok, HeaderKind},
    lexer::{LexError, LexicalSyntax, Tok, TokKind},
};

/// A function definition found in a file: its first and one-past-last token
/// and its qualified name (enclosing scopes first).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionSite {
    pub start_tok: usize,
    pub end_tok: usize,
    pub name: Vec<String>,
}

/// For each bracket token, the index of its partner. Mismatched or
/// unbalanced brackets are an error: a construct extent computed from them
/// would be a guess.
pub fn matching(tokens: &[Tok]) -> Result<Vec<Option<usize>>, LexError> {
    let mut partners = vec![None; tokens.len()];
    let mut stack: Vec<(usize, char)> = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            TokKind::Open(open) => stack.push((index, open)),
            TokKind::Close(close) => {
                let Some((open_index, open)) = stack.pop() else {
                    return Err(LexError {
                        offset: token.start,
                        message: format!("unmatched `{close}`"),
                    });
                };
                let expected = match open {
                    '(' => ')',
                    '[' => ']',
                    _ => '}',
                };
                if close != expected {
                    return Err(LexError {
                        offset: token.start,
                        message: format!("`{open}` closed by `{close}`"),
                    });
                }
                partners[open_index] = Some(index);
                partners[index] = Some(open_index);
            }
            _ => {}
        }
    }
    if let Some((index, open)) = stack.pop() {
        return Err(LexError {
            offset: tokens[index].start,
            message: format!("unclosed `{open}`"),
        });
    }
    Ok(partners)
}

/// The one-past-last token of the construct starting at `start`, never
/// beyond `limit`.
///
/// The construct is the statement that starts at `start`: it ends
///
/// - after the bracket that closes a `{` body and returns to the starting
///   depth (`void f() { ... }`, `test("x", () => { ... })`, `TEST(A, B) {
///   ... }`), together with a directly following `;`;
/// - after a `;` at the starting depth (`int f(int);`);
/// - before a bracket that closes an enclosing block; or
/// - for languages where a newline can end a statement, at such a newline
///   (see [`newline_ends_statement`]).
pub fn construct_end(tokens: &[Tok], syntax: &LexicalSyntax, start: usize, limit: usize) -> usize {
    let mut depth = 0usize;
    let mut seen_brace = false;
    let mut index = start;
    while index < limit {
        match tokens[index].kind {
            TokKind::Open(open) => {
                depth += 1;
                seen_brace |= open == '{';
            }
            TokKind::Close(_) => {
                if depth == 0 {
                    return index;
                }
                depth -= 1;
                if depth == 0 && seen_brace {
                    let mut end = index + 1;
                    if end < limit && tokens[end].kind == TokKind::Semi {
                        end += 1;
                    }
                    return end;
                }
            }
            TokKind::Semi if depth == 0 => return index + 1,
            TokKind::Newline
                if depth == 0
                    && syntax.newline_terminates
                    && newline_ends_statement(tokens, start, index, limit) =>
            {
                return index;
            }
            _ => {}
        }
        index += 1;
    }
    limit
}

/// Whether the newline at `newline` ends the statement that began at
/// `start`, for languages without mandatory `;`. It does not when the
/// statement so far is only attributes (`@Test` on its own line), when the
/// line ends in an operator or separator that needs a right operand, or
/// when the next line starts with `{` (a body on its own line), `.`, `?`,
/// `:`, or `=` (a continued expression or declaration).
pub fn newline_ends_statement(tokens: &[Tok], start: usize, newline: usize, limit: usize) -> bool {
    let Some(previous) = tokens[start..newline]
        .iter()
        .rev()
        .find(|token| !token.is_trivia())
    else {
        return false;
    };
    if let TokKind::Punct(character) = previous.kind {
        if "=,.:+-*/%|&?<>!^~@\\".contains(character) {
            return false;
        }
    }
    if only_at_attributes(&tokens[start..newline]) {
        return false;
    }
    let next = tokens[newline..limit]
        .iter()
        .find(|token| !token.is_trivia());
    match next.map(|token| token.kind) {
        Some(TokKind::Open('{')) => false,
        Some(TokKind::Punct(character)) => !".?:=".contains(character),
        _ => true,
    }
}

fn only_at_attributes(tokens: &[Tok]) -> bool {
    let significant = tokens
        .iter()
        .filter(|token| !token.is_trivia())
        .collect::<Vec<_>>();
    if significant.is_empty() {
        return false;
    }
    let mut index = 0;
    while index < significant.len() {
        if significant[index].kind != TokKind::Punct('@') {
            return false;
        }
        index += 1;
        if significant.get(index).map(|t| t.kind) != Some(TokKind::Word) {
            return false;
        }
        index += 1;
        while significant.get(index).map(|t| t.kind) == Some(TokKind::Punct('.'))
            && significant.get(index + 1).map(|t| t.kind) == Some(TokKind::Word)
        {
            index += 2;
        }
        if significant.get(index).map(|t| t.kind) == Some(TokKind::Open('(')) {
            let mut depth = 0usize;
            loop {
                match significant.get(index).map(|t| t.kind) {
                    Some(TokKind::Open(_)) => depth += 1,
                    Some(TokKind::Close(_)) => {
                        depth -= 1;
                        if depth == 0 {
                            index += 1;
                            break;
                        }
                    }
                    Some(_) => {}
                    None => return false,
                }
                index += 1;
            }
        }
    }
    true
}

/// Walks the declarations of a file and returns every function definition
/// the profile's `classify` recognizes, qualified by the named scopes that
/// enclose it. Function bodies are not searched for nested functions.
pub fn functions(
    tokens: &[Tok],
    source: &str,
    syntax: &LexicalSyntax,
    partners: &[Option<usize>],
    classify: &dyn Fn(&[HTok<'_>], bool) -> HeaderKind,
) -> Vec<FunctionSite> {
    let mut output = Vec::new();
    let mut scope = Vec::new();
    walk(
        &Walk {
            tokens,
            source,
            syntax,
            partners,
            classify,
        },
        0,
        tokens.len(),
        &mut scope,
        &mut output,
    );
    output
}

struct Walk<'a> {
    tokens: &'a [Tok],
    source: &'a str,
    syntax: &'a LexicalSyntax,
    partners: &'a [Option<usize>],
    classify: &'a dyn Fn(&[HTok<'_>], bool) -> HeaderKind,
}

fn walk(
    context: &Walk<'_>,
    low: usize,
    high: usize,
    scope: &mut Vec<String>,
    output: &mut Vec<FunctionSite>,
) {
    let tokens = context.tokens;
    let scope_depth = scope.len();
    let mut index = low;
    while index < high {
        let token = tokens[index];
        if token.is_trivia() || matches!(token.kind, TokKind::Semi | TokKind::Close(_)) {
            index += 1;
            continue;
        }
        let start = index;
        let end = construct_end(tokens, context.syntax, start, high);
        if end <= start {
            index += 1;
            continue;
        }
        let body = body_brace(tokens, start, end);
        let header_end = body.unwrap_or(end);
        let header = tokens[start..header_end]
            .iter()
            .filter(|token| !token.is_trivia())
            .map(|token| HTok {
                kind: token.kind,
                text: token.text(context.source),
            })
            .collect::<Vec<_>>();
        match (context.classify)(&header, body.is_some()) {
            HeaderKind::Function(name) => {
                let mut qualified = scope.clone();
                qualified.extend(name);
                output.push(FunctionSite {
                    start_tok: start,
                    end_tok: end,
                    name: qualified,
                });
            }
            HeaderKind::Scope(name) => {
                if let Some((open, close)) =
                    body.and_then(|open| Some((open, context.partners[open]?)))
                {
                    let restore = scope.len();
                    scope.extend(name);
                    walk(context, open + 1, close, scope, output);
                    scope.truncate(restore);
                }
            }
            HeaderKind::ScopeRest(name) => scope.extend(name),
            HeaderKind::Other => {
                if let Some((open, close)) =
                    body.and_then(|open| Some((open, context.partners[open]?)))
                {
                    walk(context, open + 1, close, scope, output);
                }
            }
        }
        index = end;
    }
    scope.truncate(scope_depth);
}

/// The `{` that opens the statement's body: the first `{` in
/// `tokens[start..end]` that is not nested in `(` or `[`.
fn body_brace(tokens: &[Tok], start: usize, end: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().take(end).skip(start) {
        match token.kind {
            TokKind::Open('{') if depth == 0 => return Some(index),
            TokKind::Open(_) => depth += 1,
            TokKind::Close(_) => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    None
}
