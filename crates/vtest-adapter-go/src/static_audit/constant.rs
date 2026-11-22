//! Literal and constant conditions, for DA-001 and for the code that never
//! runs (DS-621, DS-1928, DS-1949, DS-1950, DS-1996).
//!
//! Responsibility: tell from the syntax of an expression whether it is built
//! from literals only, and what the `true` / `false` literals make of a logical
//! operator. Boundary: no name is resolved to a constant: a named constant, a
//! call and a selector are not constant here, which can only make DA-001 report
//! no violation where a deeper analysis might report `FAIL`, never fabricate
//! one (DS-1950). An operand built from literals whose value is not evaluated
//! here (`1 == 1`) may absorb like `true`, so it makes the result undecided
//! (DS-621's escape).

use crate::syntax::{Expr, ExprKind, LitKind};

/// Whether a name is declared in the function being read (so that `true` and
/// `false` are not the predeclared ones).
pub(crate) trait Names {
    fn is_local(&self, name: &str) -> bool;
}

/// What DA-001 makes of a deciding position.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Constness {
    /// Built from literals, or fixed by a literal `true` / `false` (DS-1949).
    Const,
    NonConst,
    /// May be constant; not decided from the syntax (DS-621).
    Undecided,
}

fn strip_parens(x: &Expr) -> &Expr {
    match &x.kind {
        ExprKind::Paren(inner) => strip_parens(inner),
        _ => x,
    }
}

/// `x` is the literal `value` (`true` or `false`), through parentheses.
pub(crate) fn is_bool_literal(x: &Expr, value: bool, names: &dyn Names) -> bool {
    match &strip_parens(x).kind {
        ExprKind::Ident(name) => {
            !names.is_local(name) && name == if value { "true" } else { "false" }
        }
        _ => false,
    }
}

fn is_any_bool_literal(x: &Expr, names: &dyn Names) -> bool {
    is_bool_literal(x, true, names) || is_bool_literal(x, false, names)
}

/// The absorbing literal of a logical operator: `true` for `||`, `false` for
/// `&&` (DS-1948, DS-1996).
pub(crate) fn absorbing_literal(op: &str) -> Option<bool> {
    match op {
        "||" => Some(true),
        "&&" => Some(false),
        _ => None,
    }
}

/// DA-001's reading of a deciding position (DS-621, DS-1949, DS-1950).
pub(crate) fn classify(x: &Expr, names: &dyn Names) -> Constness {
    match &x.kind {
        ExprKind::BasicLit { .. } => Constness::Const,
        ExprKind::Ident(name) => {
            if !names.is_local(name) && matches!(name.as_str(), "true" | "false" | "nil") {
                Constness::Const
            } else {
                Constness::NonConst
            }
        }
        ExprKind::Paren(inner) => classify(inner, names),
        ExprKind::Unary(op, inner) if matches!(*op, "!" | "-" | "+" | "^") => {
            classify(inner, names)
        }
        ExprKind::Binary(op, a, b) => {
            if let Some(absorbing) = absorbing_literal(op) {
                if is_bool_literal(a, absorbing, names) || is_bool_literal(b, absorbing, names) {
                    return Constness::Const;
                }
            }
            let (ca, cb) = (classify(a, names), classify(b, names));
            match (ca, cb) {
                (Constness::NonConst, Constness::Const)
                | (Constness::Const, Constness::NonConst)
                    if absorbing_literal(op).is_some() =>
                {
                    // A constant operand that is not a `true` / `false`
                    // literal may still be an absorbing constant expression.
                    let constant = if ca == Constness::Const { a } else { b };
                    if is_any_bool_literal(constant, names) {
                        Constness::NonConst
                    } else {
                        Constness::Undecided
                    }
                }
                (Constness::NonConst, _) | (_, Constness::NonConst) => Constness::NonConst,
                (Constness::Undecided, _) | (_, Constness::Undecided) => Constness::Undecided,
                (Constness::Const, Constness::Const) => Constness::Const,
            }
        }
        _ => Constness::NonConst,
    }
}

fn parse_int(text: &str) -> Option<i128> {
    let digits: String = text.chars().filter(|c| *c != '_').collect();
    let lower = digits.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("0x") {
        i128::from_str_radix(rest, 16).ok()
    } else if let Some(rest) = lower.strip_prefix("0b") {
        i128::from_str_radix(rest, 2).ok()
    } else if let Some(rest) = lower.strip_prefix("0o") {
        i128::from_str_radix(rest, 8).ok()
    } else if lower.len() > 1 && lower.starts_with('0') {
        i128::from_str_radix(&lower[1..], 8).ok()
    } else {
        lower.parse().ok()
    }
}

/// An interpreted string literal without an escape, or a raw string.
fn plain_string(text: &str) -> Option<&str> {
    let inner = text.get(1..text.len().checked_sub(1)?)?;
    if text.starts_with('`') || !inner.contains('\\') {
        Some(inner)
    } else {
        None
    }
}

enum Literal<'a> {
    Int(i128),
    Str(&'a str),
    Bool(bool),
}

fn literal<'a>(x: &'a Expr, names: &dyn Names) -> Option<Literal<'a>> {
    let x = strip_parens(x);
    match &x.kind {
        ExprKind::BasicLit {
            kind: LitKind::Int,
            text,
        } => parse_int(text).map(Literal::Int),
        ExprKind::BasicLit {
            kind: LitKind::Str,
            text,
        } => plain_string(text).map(Literal::Str),
        _ if is_bool_literal(x, true, names) => Some(Literal::Bool(true)),
        _ if is_bool_literal(x, false, names) => Some(Literal::Bool(false)),
        _ => None,
    }
}

/// The value of a condition that the syntax fixes: `true` and `false`, `!`,
/// `&&` and `||` (a literal absorbs the other operand), and the comparison of
/// two integer literals, two strings without escapes or two booleans.
pub(crate) fn const_bool(x: &Expr, names: &dyn Names) -> Option<bool> {
    let x = strip_parens(x);
    match &x.kind {
        ExprKind::Ident(_) => match literal(x, names) {
            Some(Literal::Bool(value)) => Some(value),
            _ => None,
        },
        ExprKind::Unary("!", inner) => const_bool(inner, names).map(|value| !value),
        ExprKind::Binary("&&", a, b) => match (const_bool(a, names), const_bool(b, names)) {
            (Some(false), _) | (_, Some(false)) => Some(false),
            (Some(true), Some(true)) => Some(true),
            _ => None,
        },
        ExprKind::Binary("||", a, b) => match (const_bool(a, names), const_bool(b, names)) {
            (Some(true), _) | (_, Some(true)) => Some(true),
            (Some(false), Some(false)) => Some(false),
            _ => None,
        },
        ExprKind::Binary(op @ ("==" | "!=" | "<" | "<=" | ">" | ">="), a, b) => {
            let ordering = match (literal(a, names)?, literal(b, names)?) {
                (Literal::Int(x), Literal::Int(y)) => x.cmp(&y),
                (Literal::Str(x), Literal::Str(y)) => x.cmp(y),
                (Literal::Bool(x), Literal::Bool(y)) if matches!(*op, "==" | "!=") => x.cmp(&y),
                _ => return None,
            };
            Some(match *op {
                "==" => ordering.is_eq(),
                "!=" => ordering.is_ne(),
                "<" => ordering.is_lt(),
                "<=" => ordering.is_le(),
                ">" => ordering.is_gt(),
                _ => ordering.is_ge(),
            })
        }
        _ => None,
    }
}
