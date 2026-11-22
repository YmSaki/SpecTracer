//! Go source reading for the static audit (DES-619): a scanner, a syntax tree
//! and a parser for one Go function and for the header of a Go file.
//!
//! Responsibility: turn Go source text into the tree `static_audit` reads, with
//! the shape `go/ast` gives the same text. The reader is in-process on purpose:
//! the audit runs inside `vtest verify`, where a judgment must not start a
//! process or depend on an installed toolchain (AGENTS.md 1.2, 2.3).
//! Boundary: a text this module does not read is a [`ParseError`] and the audit
//! then reports `UNKNOWN`, never a verdict from a partial tree (DS-615); this
//! module attaches no meaning to a name, a type or a value.

mod ast;
mod lexer;
mod parser;

pub(crate) use ast::{
    BranchKind, CaseClause, CommClause, DeclSpec, Expr, ExprKind, Field, FuncDecl, FuncType,
    Ident, LitKind, Span, Stmt, StmtKind,
};
pub(crate) use ast::{Block, FileHeader};
pub(crate) use lexer::{tokenize, Tok};
pub(crate) use parser::{parse_file, parse_file_header, parse_func_decl};

/// A Go text this module cannot read: where (a byte offset) and why. The
/// message is for the person reading a diagnostic and is never matched on.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ParseError {
    pub(crate) pos: usize,
    pub(crate) message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at byte {}", self.message, self.pos)
    }
}

#[cfg(test)]
mod tests;
