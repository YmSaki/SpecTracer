//! The Go parser: tokens to the syntax tree of [`super::ast`].
//!
//! Responsibility: read Go source with the grammar of the Go specification,
//! including the places where the grammar needs the context (a composite
//! literal against a block in a statement header, a type parameter list against
//! an array length, a parameter name against a type). The structure follows the
//! `go/parser` package, so a tree here has the shape `go/ast` gives the same
//! text. Boundary: no meaning (`static_audit` owns that); a text it does not
//! read is a [`ParseError`], never a partial tree. Realizes DES-619.

use super::ast::{
    BranchKind, CaseClause, CommClause, DeclSpec, Expr, ExprKind, Field, FileHeader, FuncDecl,
    FuncLit, FuncType, Ident, Import, LitKind, Span, Stmt, StmtKind,
};
use super::ast::Block;
use super::lexer::{static_operator, tokenize, Tok, Token};
use super::ParseError;

type PResult<T> = Result<T, ParseError>;

/// A parameter entry before it is known whether it names a parameter.
enum Entry {
    /// An expression: a type, or an identifier that is a name or a type.
    Lone(Expr),
    Named(Ident, Expr),
}

/// What a simple statement parsed to.
enum Simple {
    Stmt(Stmt),
    Range {
        key: Option<Expr>,
        value: Option<Expr>,
        define: bool,
        x: Expr,
    },
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Mode {
    Basic,
    LabelOk,
    RangeOk,
}

struct Parser<'a> {
    src: &'a str,
    toks: Vec<Token>,
    p: usize,
    /// The expression level of `go/parser`: below 0 in a statement header,
    /// where `T {` is a block and not a composite literal.
    expr_lev: i32,
    /// The end of the last token read that is not a semicolon.
    last_end: usize,
}

fn binary_prec(op: &str) -> i32 {
    match op {
        "||" => 1,
        "&&" => 2,
        "==" | "!=" | "<" | "<=" | ">" | ">=" => 3,
        "+" | "-" | "|" | "^" => 4,
        "*" | "/" | "%" | "<<" | ">>" | "&" | "&^" => 5,
        _ => 0,
    }
}

const ASSIGN_OPS: &[&str] = &[
    "=", ":=", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>=", "&^=",
];

fn unquote(text: &str) -> String {
    let inner = text.get(1..text.len().saturating_sub(1)).unwrap_or("");
    inner.to_owned()
}

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> PResult<Self> {
        Ok(Self {
            src,
            toks: tokenize(src)?,
            p: 0,
            expr_lev: 0,
            last_end: 0,
        })
    }

    fn token(&self) -> Token {
        self.toks[self.p]
    }

    fn kind(&self) -> Tok {
        self.toks[self.p].kind
    }

    fn text(&self) -> &'a str {
        let token = self.toks[self.p];
        &self.src[token.start..token.end]
    }

    fn ahead(&self, n: usize) -> Token {
        self.toks[(self.p + n).min(self.toks.len() - 1)]
    }

    fn at_op(&self, op: &str) -> bool {
        self.kind() == Tok::Op && self.text() == op
    }

    fn at_kw(&self, kw: &str) -> bool {
        self.kind() == Tok::Kw && self.text() == kw
    }

    fn advance(&mut self) {
        if self.kind() != Tok::Eof {
            if self.kind() != Tok::Semi {
                self.last_end = self.token().end;
            }
            self.p += 1;
        }
    }

    fn fail<T>(&self, message: &str) -> PResult<T> {
        Err(ParseError {
            pos: self.token().start,
            message: message.to_owned(),
        })
    }

    fn expect_op(&mut self, op: &str) -> PResult<()> {
        if self.at_op(op) {
            self.advance();
            Ok(())
        } else {
            self.fail(&format!("expected `{op}`"))
        }
    }

    fn expect_kw(&mut self, kw: &str) -> PResult<()> {
        if self.at_kw(kw) {
            self.advance();
            Ok(())
        } else {
            self.fail(&format!("expected `{kw}`"))
        }
    }

    fn got_op(&mut self, op: &str) -> bool {
        let at = self.at_op(op);
        if at {
            self.advance();
        }
        at
    }

    /// A statement ends at `;`, or before a closing `)` or `}`.
    fn expect_semi(&mut self) -> PResult<()> {
        if self.at_op(")") || self.at_op("}") || self.kind() == Tok::Eof {
            return Ok(());
        }
        if self.kind() == Tok::Semi {
            self.advance();
            return Ok(());
        }
        self.fail("expected `;`")
    }

    fn span_from(&self, start: usize) -> Span {
        Span {
            start,
            end: self.last_end.max(start),
        }
    }

    fn ident(&mut self) -> PResult<Ident> {
        if self.kind() != Tok::Ident {
            return self.fail("expected an identifier");
        }
        let token = self.token();
        let name = self.text().to_owned();
        self.advance();
        Ok(Ident {
            name,
            span: Span {
                start: token.start,
                end: token.end,
            },
        })
    }

    fn make(&self, start: usize, kind: ExprKind) -> Expr {
        Expr {
            span: self.span_from(start),
            kind,
        }
    }

    // -- types ----------------------------------------------------------

    fn starts_type(&self) -> bool {
        match self.kind() {
            Tok::Ident => true,
            Tok::Op => matches!(self.text(), "*" | "[" | "(" | "<-"),
            Tok::Kw => matches!(
                self.text(),
                "func" | "struct" | "interface" | "map" | "chan"
            ),
            _ => false,
        }
    }

    fn parse_type(&mut self) -> PResult<Expr> {
        let start = self.token().start;
        match self.kind() {
            Tok::Ident => {
                let id = self.ident()?;
                let mut x = self.make(start, ExprKind::Ident(id.name));
                if self.at_op(".") && self.ahead(1).kind == Tok::Ident {
                    self.advance();
                    let sel = self.ident()?;
                    x = self.make(start, ExprKind::Selector(Box::new(x), sel));
                }
                if self.at_op("[") {
                    x = self.parse_type_args(x, start)?;
                }
                Ok(x)
            }
            Tok::Op => match self.text() {
                "*" => {
                    self.advance();
                    let t = self.parse_type()?;
                    Ok(self.make(start, ExprKind::Star(Box::new(t))))
                }
                "[" => self.parse_array_type(),
                "(" => {
                    self.advance();
                    let t = self.parse_type()?;
                    self.expect_op(")")?;
                    Ok(self.make(start, ExprKind::Paren(Box::new(t))))
                }
                "<-" => {
                    self.advance();
                    self.expect_kw("chan")?;
                    let t = self.parse_type()?;
                    Ok(self.make(start, ExprKind::ChanType(Box::new(t))))
                }
                _ => self.fail("expected a type"),
            },
            Tok::Kw => match self.text() {
                "func" => {
                    self.advance();
                    let sig = self.parse_signature()?;
                    Ok(self.make(start, ExprKind::FuncType(sig)))
                }
                "struct" => self.parse_struct_type(),
                "interface" => self.parse_interface_type(),
                "map" => {
                    self.advance();
                    self.expect_op("[")?;
                    let key = self.parse_type()?;
                    self.expect_op("]")?;
                    let value = self.parse_type()?;
                    Ok(self.make(start, ExprKind::MapType(Box::new(key), Box::new(value))))
                }
                "chan" => {
                    self.advance();
                    self.got_op("<-");
                    let t = self.parse_type()?;
                    Ok(self.make(start, ExprKind::ChanType(Box::new(t))))
                }
                _ => self.fail("expected a type"),
            },
            _ => self.fail("expected a type"),
        }
    }

    /// `T[A, B]` after the type name `x`.
    fn parse_type_args(&mut self, x: Expr, start: usize) -> PResult<Expr> {
        self.expect_op("[")?;
        let mut args = Vec::new();
        while !self.at_op("]") && self.kind() != Tok::Eof {
            args.push(self.parse_type()?);
            if !self.got_op(",") {
                break;
            }
        }
        self.expect_op("]")?;
        Ok(self.make(start, ExprKind::Index(Box::new(x), args)))
    }

    fn parse_array_type(&mut self) -> PResult<Expr> {
        let start = self.token().start;
        self.expect_op("[")?;
        let len = if self.at_op("]") {
            None
        } else if self.at_op("...") {
            let at = self.token().start;
            self.advance();
            Some(Box::new(self.make(at, ExprKind::Ellipsis(None))))
        } else {
            self.expr_lev += 1;
            let len = self.parse_expr();
            self.expr_lev -= 1;
            Some(Box::new(len?))
        };
        self.expect_op("]")?;
        let elt = self.parse_type()?;
        Ok(self.make(
            start,
            ExprKind::ArrayType {
                len,
                elt: Box::new(elt),
            },
        ))
    }

    fn parse_struct_type(&mut self) -> PResult<Expr> {
        let start = self.token().start;
        self.expect_kw("struct")?;
        self.expect_op("{")?;
        while !self.at_op("}") && self.kind() != Tok::Eof {
            self.skip_field_decl()?;
            self.expect_semi()?;
        }
        self.expect_op("}")?;
        Ok(self.make(start, ExprKind::StructType))
    }

    /// One field declaration of a struct type, read and dropped.
    fn skip_field_decl(&mut self) -> PResult<()> {
        if self.at_op("*") {
            self.advance();
            self.parse_type()?;
        } else if self.kind() == Tok::Ident {
            self.ident()?;
            if self.at_op(".") {
                // An embedded `pkg.T`, perhaps with type arguments.
                self.advance();
                self.ident()?;
                if self.at_op("[") {
                    let start = self.token().start;
                    let dummy = self.make(start, ExprKind::StructType);
                    self.parse_type_args(dummy, start)?;
                }
            } else if self.kind() == Tok::Semi || self.at_op("}") || self.kind() == Tok::Str {
                // An embedded type name.
            } else if self.at_op("[") {
                // `name [N]T`, `name []T`, or an embedded `T[A]`.
                self.advance();
                if self.at_op("]") {
                    self.advance();
                    self.parse_type()?;
                } else {
                    self.expr_lev += 1;
                    let first = self.parse_expr();
                    self.expr_lev -= 1;
                    first?;
                    while self.got_op(",") {
                        self.expr_lev += 1;
                        let more = self.parse_expr();
                        self.expr_lev -= 1;
                        more?;
                    }
                    self.expect_op("]")?;
                    if !(self.kind() == Tok::Semi || self.at_op("}") || self.kind() == Tok::Str) {
                        self.parse_type()?;
                    }
                }
            } else {
                while self.got_op(",") {
                    self.ident()?;
                }
                self.parse_type()?;
            }
        } else {
            return self.fail("expected a struct field");
        }
        if self.kind() == Tok::Str {
            self.advance();
        }
        Ok(())
    }

    fn parse_interface_type(&mut self) -> PResult<Expr> {
        let start = self.token().start;
        self.expect_kw("interface")?;
        self.expect_op("{")?;
        while !self.at_op("}") && self.kind() != Tok::Eof {
            let next = self.ahead(1);
            let is_method = self.kind() == Tok::Ident
                && next.kind == Tok::Op
                && &self.src[next.start..next.end] == "(";
            if is_method {
                self.ident()?;
                self.parse_signature()?;
            } else {
                self.parse_constraint()?;
            }
            self.expect_semi()?;
        }
        self.expect_op("}")?;
        Ok(self.make(start, ExprKind::InterfaceType))
    }

    /// A type constraint: a union of terms, each optionally `~T`.
    fn parse_constraint(&mut self) -> PResult<Expr> {
        let start = self.token().start;
        let mut x = self.parse_constraint_term()?;
        while self.at_op("|") {
            self.advance();
            let y = self.parse_constraint_term()?;
            x = self.make(start, ExprKind::Binary("|", Box::new(x), Box::new(y)));
        }
        Ok(x)
    }

    fn parse_constraint_term(&mut self) -> PResult<Expr> {
        let start = self.token().start;
        if self.got_op("~") {
            let t = self.parse_type()?;
            return Ok(self.make(start, ExprKind::Unary("~", Box::new(t))));
        }
        self.parse_type()
    }

    /// `[T any, K comparable]`, read and dropped.
    fn skip_type_params(&mut self) -> PResult<()> {
        self.expect_op("[")?;
        while !self.at_op("]") && self.kind() != Tok::Eof {
            self.ident()?;
            while self.got_op(",") {
                self.ident()?;
            }
            self.parse_constraint()?;
            if !self.got_op(",") {
                break;
            }
        }
        self.expect_op("]")
    }

    fn parse_signature(&mut self) -> PResult<FuncType> {
        let params = self.parse_params()?;
        let results = if self.at_op("(") {
            self.parse_params()?
        } else if self.starts_type() {
            vec![Field {
                names: Vec::new(),
                ty: self.parse_type()?,
            }]
        } else {
            Vec::new()
        };
        Ok(FuncType { params, results })
    }

    fn parse_params(&mut self) -> PResult<Vec<Field>> {
        self.expect_op("(")?;
        let mut entries = Vec::new();
        while !self.at_op(")") && self.kind() != Tok::Eof {
            entries.push(self.parse_param_entry()?);
            if !self.got_op(",") {
                break;
            }
        }
        self.expect_op(")")?;
        let named = entries.iter().any(|entry| matches!(entry, Entry::Named(..)));
        let mut fields = Vec::new();
        if !named {
            for entry in entries {
                if let Entry::Lone(ty) = entry {
                    fields.push(Field {
                        names: Vec::new(),
                        ty,
                    });
                }
            }
            return Ok(fields);
        }
        let mut pending: Vec<Ident> = Vec::new();
        for entry in entries {
            match entry {
                Entry::Named(name, ty) => {
                    let mut names = std::mem::take(&mut pending);
                    names.push(name);
                    fields.push(Field { names, ty });
                }
                Entry::Lone(Expr {
                    kind: ExprKind::Ident(name),
                    span,
                }) => pending.push(Ident { name, span }),
                Entry::Lone(_) => return self.fail("mixed named and unnamed parameters"),
            }
        }
        if !pending.is_empty() {
            return self.fail("mixed named and unnamed parameters");
        }
        Ok(fields)
    }

    fn parse_param_entry(&mut self) -> PResult<Entry> {
        let start = self.token().start;
        if self.at_op("...") {
            self.advance();
            let t = self.parse_type()?;
            return Ok(Entry::Lone(self.make(start, ExprKind::Ellipsis(Some(Box::new(t))))));
        }
        if self.kind() != Tok::Ident {
            return Ok(Entry::Lone(self.parse_type()?));
        }
        let id = self.ident()?;
        let name_expr = self.make(start, ExprKind::Ident(id.name.clone()));
        if self.at_op(",") || self.at_op(")") {
            return Ok(Entry::Lone(name_expr));
        }
        if self.at_op(".") {
            self.advance();
            let sel = self.ident()?;
            let mut x = self.make(start, ExprKind::Selector(Box::new(name_expr), sel));
            if self.at_op("[") {
                x = self.parse_type_args(x, start)?;
            }
            return Ok(Entry::Lone(x));
        }
        if self.at_op("...") {
            let at = self.token().start;
            self.advance();
            let t = self.parse_type()?;
            let ty = self.make(at, ExprKind::Ellipsis(Some(Box::new(t))));
            return Ok(Entry::Named(id, ty));
        }
        if self.at_op("[") {
            // `name []T`, `name [N]T`, or a lone `T[A]`.
            let bracket = self.token().start;
            self.advance();
            if self.at_op("]") {
                self.advance();
                let elt = self.parse_type()?;
                let ty = self.make(
                    bracket,
                    ExprKind::ArrayType {
                        len: None,
                        elt: Box::new(elt),
                    },
                );
                return Ok(Entry::Named(id, ty));
            }
            let mut args = Vec::new();
            self.expr_lev += 1;
            let first = self.parse_expr();
            self.expr_lev -= 1;
            args.push(first?);
            while self.got_op(",") {
                self.expr_lev += 1;
                let more = self.parse_expr();
                self.expr_lev -= 1;
                args.push(more?);
            }
            self.expect_op("]")?;
            if self.at_op(",") || self.at_op(")") {
                return Ok(Entry::Lone(
                    self.make(start, ExprKind::Index(Box::new(name_expr), args)),
                ));
            }
            if args.len() != 1 {
                return self.fail("expected a parameter type");
            }
            let elt = self.parse_type()?;
            let len = args.pop().map(Box::new);
            let ty = self.make(bracket, ExprKind::ArrayType { len, elt: Box::new(elt) });
            return Ok(Entry::Named(id, ty));
        }
        let ty = self.parse_type()?;
        Ok(Entry::Named(id, ty))
    }

    // -- expressions ----------------------------------------------------

    fn parse_expr(&mut self) -> PResult<Expr> {
        self.parse_binary(1)
    }

    fn parse_expr_list(&mut self) -> PResult<Vec<Expr>> {
        let mut list = vec![self.parse_expr()?];
        while self.got_op(",") {
            list.push(self.parse_expr()?);
        }
        Ok(list)
    }

    fn parse_binary(&mut self, min_prec: i32) -> PResult<Expr> {
        let start = self.token().start;
        let mut x = self.parse_unary()?;
        loop {
            if self.kind() != Tok::Op {
                return Ok(x);
            }
            let op = self.text();
            let prec = binary_prec(op);
            if prec < min_prec || prec == 0 {
                return Ok(x);
            }
            self.advance();
            let y = self.parse_binary(prec + 1)?;
            x = self.make(
                start,
                ExprKind::Binary(static_operator(op), Box::new(x), Box::new(y)),
            );
        }
    }

    fn parse_unary(&mut self) -> PResult<Expr> {
        let start = self.token().start;
        if self.kind() == Tok::Op {
            match self.text() {
                op @ ("+" | "-" | "!" | "^" | "&") => {
                    self.advance();
                    let x = self.parse_unary()?;
                    return Ok(self.make(start, ExprKind::Unary(static_operator(op), Box::new(x))));
                }
                "<-" => {
                    self.advance();
                    if self.at_kw("chan") {
                        // `<-chan T`, a type.
                        self.advance();
                        let t = self.parse_type()?;
                        return Ok(self.make(start, ExprKind::ChanType(Box::new(t))));
                    }
                    let x = self.parse_unary()?;
                    return Ok(self.make(start, ExprKind::Unary("<-", Box::new(x))));
                }
                "*" => {
                    self.advance();
                    let x = self.parse_unary()?;
                    return Ok(self.make(start, ExprKind::Star(Box::new(x))));
                }
                _ => {}
            }
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> PResult<Expr> {
        let start = self.token().start;
        let mut x = self.parse_operand()?;
        loop {
            if self.kind() != Tok::Op {
                return Ok(x);
            }
            match self.text() {
                "." => {
                    self.advance();
                    if self.kind() == Tok::Ident {
                        let sel = self.ident()?;
                        x = self.make(start, ExprKind::Selector(Box::new(x), sel));
                    } else if self.at_op("(") {
                        self.advance();
                        if self.at_kw("type") {
                            self.advance();
                            self.expect_op(")")?;
                            x = self.make(start, ExprKind::TypeAssert(Box::new(x), None));
                        } else {
                            let t = self.parse_type()?;
                            self.expect_op(")")?;
                            x = self.make(
                                start,
                                ExprKind::TypeAssert(Box::new(x), Some(Box::new(t))),
                            );
                        }
                    } else {
                        return self.fail("expected a selector or a type assertion");
                    }
                }
                "[" => x = self.parse_index_or_slice(x, start)?,
                "(" => x = self.parse_call(x, start)?,
                "{" => {
                    // go/parser: a type name, selector or instance before `{`
                    // is a composite literal type only outside a statement
                    // header; an array, struct or map type always is.
                    let literal = match x.kind {
                        ExprKind::Ident(_) | ExprKind::Selector(..) | ExprKind::Index(..) => {
                            self.expr_lev >= 0
                        }
                        ExprKind::ArrayType { .. }
                        | ExprKind::StructType
                        | ExprKind::MapType(..) => true,
                        _ => false,
                    };
                    if !literal {
                        return Ok(x);
                    }
                    x = self.parse_literal_value(Some(x), start)?;
                }
                _ => return Ok(x),
            }
        }
    }

    fn parse_operand(&mut self) -> PResult<Expr> {
        let start = self.token().start;
        match self.kind() {
            Tok::Ident => {
                let id = self.ident()?;
                Ok(self.make(start, ExprKind::Ident(id.name)))
            }
            Tok::Int | Tok::Float | Tok::Imag | Tok::Char | Tok::Str => {
                let kind = match self.kind() {
                    Tok::Int => LitKind::Int,
                    Tok::Float => LitKind::Float,
                    Tok::Imag => LitKind::Imag,
                    Tok::Char => LitKind::Char,
                    _ => LitKind::Str,
                };
                let text = self.text().to_owned();
                self.advance();
                Ok(self.make(start, ExprKind::BasicLit { kind, text }))
            }
            Tok::Op => match self.text() {
                "(" => {
                    self.advance();
                    self.expr_lev += 1;
                    let x = self.parse_expr();
                    self.expr_lev -= 1;
                    let x = x?;
                    self.expect_op(")")?;
                    Ok(self.make(start, ExprKind::Paren(Box::new(x))))
                }
                "[" => self.parse_type(),
                _ => self.fail("expected an expression"),
            },
            Tok::Kw => match self.text() {
                "func" => {
                    self.advance();
                    let sig = self.parse_signature()?;
                    if self.at_op("{") {
                        self.expr_lev += 1;
                        let body = self.parse_block();
                        self.expr_lev -= 1;
                        let body = body?;
                        Ok(self.make(start, ExprKind::FuncLit(Box::new(FuncLit { sig, body }))))
                    } else {
                        Ok(self.make(start, ExprKind::FuncType(sig)))
                    }
                }
                "struct" | "map" | "chan" | "interface" => self.parse_type(),
                _ => self.fail("expected an expression"),
            },
            _ => self.fail("expected an expression"),
        }
    }

    fn parse_index_or_slice(&mut self, x: Expr, start: usize) -> PResult<Expr> {
        self.expect_op("[")?;
        self.expr_lev += 1;
        let result = self.parse_index_rest(x, start);
        self.expr_lev -= 1;
        result
    }

    fn parse_index_rest(&mut self, x: Expr, start: usize) -> PResult<Expr> {
        let mut first = None;
        if !self.at_op(":") {
            first = Some(self.parse_expr()?);
        }
        if self.at_op(",") {
            let mut list: Vec<Expr> = first.into_iter().collect();
            while self.got_op(",") {
                if self.at_op("]") {
                    break;
                }
                list.push(self.parse_expr()?);
            }
            self.expect_op("]")?;
            return Ok(self.make(start, ExprKind::Index(Box::new(x), list)));
        }
        let mut parts: Vec<Option<Expr>> = vec![first.clone(), None, None];
        let mut colons = 0;
        while self.at_op(":") && colons < 2 {
            colons += 1;
            self.advance();
            if !self.at_op(":") && !self.at_op("]") {
                parts[colons] = Some(self.parse_expr()?);
            }
        }
        self.expect_op("]")?;
        if colons == 0 {
            let Some(index) = first else {
                return self.fail("expected an index");
            };
            return Ok(self.make(start, ExprKind::Index(Box::new(x), vec![index])));
        }
        Ok(self.make(start, ExprKind::Slice(Box::new(x), parts)))
    }

    fn parse_call(&mut self, fun: Expr, start: usize) -> PResult<Expr> {
        self.expect_op("(")?;
        self.expr_lev += 1;
        let result = self.parse_call_rest(fun, start);
        self.expr_lev -= 1;
        result
    }

    fn parse_call_rest(&mut self, fun: Expr, start: usize) -> PResult<Expr> {
        let mut args = Vec::new();
        let mut ellipsis = false;
        while !self.at_op(")") && self.kind() != Tok::Eof {
            args.push(self.parse_expr()?);
            if self.at_op("...") {
                self.advance();
                ellipsis = true;
            }
            if !self.got_op(",") {
                break;
            }
        }
        self.expect_op(")")?;
        Ok(self.make(
            start,
            ExprKind::Call {
                fun: Box::new(fun),
                args,
                ellipsis,
            },
        ))
    }

    fn parse_literal_value(&mut self, ty: Option<Expr>, start: usize) -> PResult<Expr> {
        self.expect_op("{")?;
        self.expr_lev += 1;
        let elts = self.parse_elements();
        self.expr_lev -= 1;
        let elts = elts?;
        self.expect_op("}")?;
        Ok(self.make(
            start,
            ExprKind::CompositeLit {
                ty: ty.map(Box::new),
                elts,
            },
        ))
    }

    fn parse_elements(&mut self) -> PResult<Vec<Expr>> {
        let mut elts = Vec::new();
        while !self.at_op("}") && self.kind() != Tok::Eof {
            let start = self.token().start;
            let x = self.parse_element_value()?;
            if self.got_op(":") {
                let value = self.parse_element_value()?;
                elts.push(self.make(start, ExprKind::KeyValue(Box::new(x), Box::new(value))));
            } else {
                elts.push(x);
            }
            if !self.got_op(",") {
                break;
            }
        }
        Ok(elts)
    }

    fn parse_element_value(&mut self) -> PResult<Expr> {
        if self.at_op("{") {
            let start = self.token().start;
            return self.parse_literal_value(None, start);
        }
        self.parse_expr()
    }

    // -- statements -----------------------------------------------------

    fn parse_block(&mut self) -> PResult<Block> {
        let start = self.token().start;
        self.expect_op("{")?;
        let stmts = self.parse_stmt_list()?;
        self.expect_op("}")?;
        Ok(Block {
            span: self.span_from(start),
            stmts,
        })
    }

    fn parse_stmt_list(&mut self) -> PResult<Vec<Stmt>> {
        let mut stmts = Vec::new();
        while !self.at_op("}")
            && !self.at_kw("case")
            && !self.at_kw("default")
            && self.kind() != Tok::Eof
        {
            stmts.push(self.parse_stmt()?);
        }
        Ok(stmts)
    }

    fn stmt(&self, start: usize, kind: StmtKind) -> Stmt {
        Stmt {
            span: self.span_from(start),
            kind,
        }
    }

    fn parse_stmt(&mut self) -> PResult<Stmt> {
        let start = self.token().start;
        match self.kind() {
            Tok::Semi => {
                self.advance();
                Ok(self.stmt(start, StmtKind::Empty))
            }
            Tok::Kw => match self.text() {
                "const" | "var" | "type" => {
                    let kw = self.text();
                    self.advance();
                    let specs = self.parse_specs(kw)?;
                    self.expect_semi()?;
                    Ok(self.stmt(start, StmtKind::Decl(specs)))
                }
                "go" | "defer" => {
                    let is_go = self.text() == "go";
                    self.advance();
                    let call = self.parse_expr()?;
                    self.expect_semi()?;
                    let kind = if is_go {
                        StmtKind::Go(call)
                    } else {
                        StmtKind::Defer(call)
                    };
                    Ok(self.stmt(start, kind))
                }
                "return" => {
                    self.advance();
                    let mut results = Vec::new();
                    if self.kind() != Tok::Semi && !self.at_op("}") {
                        results = self.parse_expr_list()?;
                    }
                    self.expect_semi()?;
                    Ok(self.stmt(start, StmtKind::Return(results)))
                }
                kw @ ("break" | "continue" | "goto" | "fallthrough") => {
                    let kind = match kw {
                        "break" => BranchKind::Break,
                        "continue" => BranchKind::Continue,
                        "goto" => BranchKind::Goto,
                        _ => BranchKind::Fallthrough,
                    };
                    self.advance();
                    let label = if kind != BranchKind::Fallthrough && self.kind() == Tok::Ident {
                        Some(self.ident()?)
                    } else {
                        None
                    };
                    self.expect_semi()?;
                    Ok(self.stmt(start, StmtKind::Branch { kind, label }))
                }
                "if" => {
                    let stmt = self.parse_if()?;
                    self.expect_semi()?;
                    Ok(stmt)
                }
                "switch" => {
                    let stmt = self.parse_switch()?;
                    self.expect_semi()?;
                    Ok(stmt)
                }
                "select" => {
                    let stmt = self.parse_select()?;
                    self.expect_semi()?;
                    Ok(stmt)
                }
                "for" => {
                    let stmt = self.parse_for()?;
                    self.expect_semi()?;
                    Ok(stmt)
                }
                "func" | "struct" | "map" | "chan" | "interface" => self.parse_simple_stmt_semi(),
                _ => self.fail("expected a statement"),
            },
            Tok::Op if self.at_op("{") => {
                let block = self.parse_block()?;
                self.expect_semi()?;
                Ok(self.stmt(start, StmtKind::Block(block)))
            }
            _ => self.parse_simple_stmt_semi(),
        }
    }

    /// A simple statement with its closing semicolon (a labeled one has its
    /// own).
    fn parse_simple_stmt_semi(&mut self) -> PResult<Stmt> {
        let Simple::Stmt(stmt) = self.parse_simple_stmt(Mode::LabelOk)? else {
            return self.fail("unexpected `range`");
        };
        if !matches!(stmt.kind, StmtKind::Labeled(..)) {
            self.expect_semi()?;
        }
        Ok(stmt)
    }

    fn parse_simple_stmt(&mut self, mode: Mode) -> PResult<Simple> {
        let start = self.token().start;
        let lhs = self.parse_expr_list()?;
        if self.kind() == Tok::Op {
            let op = self.text();
            if ASSIGN_OPS.contains(&op) {
                self.advance();
                if mode == Mode::RangeOk && self.at_kw("range") && (op == ":=" || op == "=") {
                    self.advance();
                    let x = self.parse_expr()?;
                    let mut keys = lhs.into_iter();
                    let key = keys.next();
                    let value = keys.next();
                    if keys.next().is_some() {
                        return self.fail("range with more than two variables");
                    }
                    return Ok(Simple::Range {
                        key,
                        value,
                        define: op == ":=",
                        x,
                    });
                }
                let rhs = self.parse_expr_list()?;
                return Ok(Simple::Stmt(self.stmt(
                    start,
                    StmtKind::Assign {
                        lhs,
                        op: static_operator(op),
                        rhs,
                    },
                )));
            }
        }
        let mut lhs = lhs;
        if lhs.len() > 1 {
            return self.fail("expected an assignment");
        }
        let Some(x) = lhs.pop() else {
            return self.fail("expected a statement");
        };
        if self.at_op(":") && mode == Mode::LabelOk {
            if let ExprKind::Ident(name) = &x.kind {
                let label = Ident {
                    name: name.clone(),
                    span: x.span,
                };
                self.advance();
                let inner = if self.at_op("}") {
                    self.stmt(self.token().start, StmtKind::Empty)
                } else {
                    self.parse_stmt()?
                };
                return Ok(Simple::Stmt(
                    self.stmt(start, StmtKind::Labeled(label, Box::new(inner))),
                ));
            }
        }
        if self.at_op("<-") {
            self.advance();
            let value = self.parse_expr()?;
            return Ok(Simple::Stmt(self.stmt(start, StmtKind::Send(x, value))));
        }
        if self.at_op("++") || self.at_op("--") {
            self.advance();
            return Ok(Simple::Stmt(self.stmt(start, StmtKind::IncDec(x))));
        }
        Ok(Simple::Stmt(self.stmt(start, StmtKind::Expr(x))))
    }

    fn simple_stmt(&mut self) -> PResult<Stmt> {
        match self.parse_simple_stmt(Mode::Basic)? {
            Simple::Stmt(stmt) => Ok(stmt),
            Simple::Range { .. } => self.fail("unexpected `range`"),
        }
    }

    fn cond_of(&self, stmt: Stmt) -> PResult<Expr> {
        match stmt.kind {
            StmtKind::Expr(x) => Ok(x),
            _ => self.fail("expected a condition"),
        }
    }

    fn parse_if(&mut self) -> PResult<Stmt> {
        let start = self.token().start;
        self.expect_kw("if")?;
        let saved = self.expr_lev;
        self.expr_lev = -1;
        let header = self.parse_if_header();
        self.expr_lev = saved;
        let (init, cond) = header?;
        let then = self.parse_block()?;
        let mut els = None;
        if self.at_kw("else") {
            self.advance();
            if self.at_kw("if") {
                els = Some(Box::new(self.parse_if()?));
            } else if self.at_op("{") {
                let at = self.token().start;
                let block = self.parse_block()?;
                els = Some(Box::new(self.stmt(at, StmtKind::Block(block))));
            } else {
                return self.fail("expected `if` or a block after `else`");
            }
        }
        Ok(self.stmt(
            start,
            StmtKind::If {
                init,
                cond,
                then,
                els,
            },
        ))
    }

    fn parse_if_header(&mut self) -> PResult<(Option<Box<Stmt>>, Expr)> {
        if self.at_op("{") {
            return self.fail("missing condition in `if`");
        }
        let mut init = None;
        let first = if self.kind() == Tok::Semi {
            None
        } else {
            Some(self.simple_stmt()?)
        };
        let cond = if self.kind() == Tok::Semi {
            self.advance();
            init = first.map(Box::new);
            if self.at_op("{") {
                return self.fail("missing condition in `if`");
            }
            self.simple_stmt()?
        } else {
            match first {
                Some(stmt) => stmt,
                None => return self.fail("missing condition in `if`"),
            }
        };
        Ok((init, self.cond_of(cond)?))
    }

    fn parse_switch(&mut self) -> PResult<Stmt> {
        let start = self.token().start;
        self.expect_kw("switch")?;
        let saved = self.expr_lev;
        self.expr_lev = -1;
        let header = self.parse_switch_header();
        self.expr_lev = saved;
        let (init, tag) = header?;
        self.expect_op("{")?;
        let mut clauses = Vec::new();
        while self.at_kw("case") || self.at_kw("default") {
            clauses.push(self.parse_case_clause()?);
        }
        self.expect_op("}")?;
        // A type switch: `switch x.(type)` or `switch v := x.(type)`.
        if let Some(tag_stmt) = &tag {
            let (bind, subject) = match &tag_stmt.kind {
                StmtKind::Expr(Expr {
                    kind: ExprKind::TypeAssert(x, None),
                    ..
                }) => (None, Some((**x).clone())),
                StmtKind::Assign { lhs, op: ":=", rhs } if lhs.len() == 1 && rhs.len() == 1 => {
                    match (&lhs[0].kind, &rhs[0].kind) {
                        (ExprKind::Ident(name), ExprKind::TypeAssert(x, None)) => (
                            Some(Ident {
                                name: name.clone(),
                                span: lhs[0].span,
                            }),
                            Some((**x).clone()),
                        ),
                        _ => (None, None),
                    }
                }
                _ => (None, None),
            };
            if let Some(x) = subject {
                return Ok(self.stmt(
                    start,
                    StmtKind::TypeSwitch {
                        init,
                        bind,
                        x,
                        clauses,
                    },
                ));
            }
        }
        let tag = match tag {
            Some(stmt) => Some(self.cond_of(stmt)?),
            None => None,
        };
        Ok(self.stmt(start, StmtKind::Switch { init, tag, clauses }))
    }

    fn parse_switch_header(&mut self) -> PResult<(Option<Box<Stmt>>, Option<Stmt>)> {
        let mut init = None;
        let mut tag = None;
        if !self.at_op("{") {
            if self.kind() != Tok::Semi {
                tag = Some(self.simple_stmt()?);
            }
            if self.kind() == Tok::Semi {
                self.advance();
                init = tag.take().map(Box::new);
                if !self.at_op("{") {
                    tag = Some(self.simple_stmt()?);
                }
            }
        }
        Ok((init, tag))
    }

    fn parse_case_clause(&mut self) -> PResult<CaseClause> {
        let start = self.token().start;
        let default = self.at_kw("default");
        self.advance();
        let exprs = if default {
            Vec::new()
        } else {
            self.parse_expr_list()?
        };
        self.expect_op(":")?;
        let body = self.parse_stmt_list()?;
        Ok(CaseClause {
            span: self.span_from(start),
            default,
            exprs,
            body,
        })
    }

    fn parse_select(&mut self) -> PResult<Stmt> {
        let start = self.token().start;
        self.expect_kw("select")?;
        self.expect_op("{")?;
        let mut clauses = Vec::new();
        while self.at_kw("case") || self.at_kw("default") {
            let clause_start = self.token().start;
            let comm = if self.at_kw("case") {
                self.advance();
                Some(Box::new(self.simple_stmt()?))
            } else {
                self.advance();
                None
            };
            self.expect_op(":")?;
            let body = self.parse_stmt_list()?;
            clauses.push(CommClause {
                span: self.span_from(clause_start),
                comm,
                body,
            });
        }
        self.expect_op("}")?;
        Ok(self.stmt(start, StmtKind::Select { clauses }))
    }

    fn parse_for(&mut self) -> PResult<Stmt> {
        let start = self.token().start;
        self.expect_kw("for")?;
        let saved = self.expr_lev;
        self.expr_lev = -1;
        let header = self.parse_for_header();
        self.expr_lev = saved;
        let header = header?;
        let body = self.parse_block()?;
        let kind = match header {
            ForHeader::Range {
                key,
                value,
                define,
                x,
            } => StmtKind::Range {
                key,
                value,
                define,
                x,
                body,
            },
            ForHeader::Clauses { init, cond, post } => StmtKind::For {
                init,
                cond,
                post,
                body,
            },
        };
        Ok(self.stmt(start, kind))
    }

    fn parse_for_header(&mut self) -> PResult<ForHeader> {
        let mut init = None;
        let mut cond = None;
        let mut post = None;
        if self.at_op("{") {
            return Ok(ForHeader::Clauses { init, cond, post });
        }
        let mut first = None;
        if self.kind() != Tok::Semi {
            if self.at_kw("range") {
                self.advance();
                let x = self.parse_expr()?;
                return Ok(ForHeader::Range {
                    key: None,
                    value: None,
                    define: false,
                    x,
                });
            }
            match self.parse_simple_stmt(Mode::RangeOk)? {
                Simple::Range {
                    key,
                    value,
                    define,
                    x,
                } => {
                    return Ok(ForHeader::Range {
                        key,
                        value,
                        define,
                        x,
                    })
                }
                Simple::Stmt(stmt) => first = Some(stmt),
            }
        }
        if self.kind() == Tok::Semi {
            self.advance();
            init = first.map(Box::new);
            if self.kind() != Tok::Semi {
                let stmt = self.simple_stmt()?;
                cond = Some(self.cond_of(stmt)?);
            }
            if self.kind() != Tok::Semi {
                return self.fail("expected `;` in the `for` header");
            }
            self.advance();
            if !self.at_op("{") {
                post = Some(Box::new(self.simple_stmt()?));
            }
        } else if let Some(stmt) = first {
            cond = Some(self.cond_of(stmt)?);
        }
        Ok(ForHeader::Clauses { init, cond, post })
    }

    // -- declarations ---------------------------------------------------

    /// The specs of a `var`, `const` or `type` declaration, after its keyword.
    fn parse_specs(&mut self, kw: &str) -> PResult<Vec<DeclSpec>> {
        let mut specs = Vec::new();
        if self.got_op("(") {
            while !self.at_op(")") && self.kind() != Tok::Eof {
                specs.push(self.parse_spec(kw)?);
                self.expect_semi()?;
            }
            self.expect_op(")")?;
        } else {
            specs.push(self.parse_spec(kw)?);
        }
        Ok(specs)
    }

    fn parse_spec(&mut self, kw: &str) -> PResult<DeclSpec> {
        if kw == "type" {
            let name = self.ident()?;
            if self.at_op("[") && self.is_type_param_list() {
                self.skip_type_params()?;
            }
            self.got_op("=");
            self.parse_type()?;
            return Ok(DeclSpec::Type { name });
        }
        let mut names = vec![self.ident()?];
        while self.got_op(",") {
            names.push(self.ident()?);
        }
        let mut ty = None;
        if !self.at_op("=") && self.kind() != Tok::Semi && !self.at_op(")") {
            ty = Some(self.parse_type()?);
        }
        let mut values = Vec::new();
        if self.got_op("=") {
            values = self.parse_expr_list()?;
        }
        if kw == "var" {
            Ok(DeclSpec::Var { names, ty, values })
        } else {
            Ok(DeclSpec::Const { names, values })
        }
    }

    /// After `type Name`, whether `[` opens a type parameter list and not an
    /// array length (`type T[P any] ..` against `type T [N]int`).
    fn is_type_param_list(&self) -> bool {
        let first = self.ahead(1);
        if first.kind != Tok::Ident {
            return false;
        }
        let second = self.ahead(2);
        let text = &self.src[second.start..second.end];
        match second.kind {
            Tok::Ident => true,
            Tok::Kw => matches!(text, "interface" | "func" | "map" | "chan" | "struct"),
            Tok::Op if text == "*" => self.bracket_holds_top_level_comma(),
            Tok::Op => matches!(text, "~" | "," | "[" | "("),
            _ => false,
        }
    }

    /// Whether the bracket at the current token holds a comma outside nested
    /// brackets: an array length is one expression, so a comma makes the
    /// bracket a type parameter list (`type T[P *C | D, Q any] ..`).
    fn bracket_holds_top_level_comma(&self) -> bool {
        let mut depth = 0usize;
        for token in &self.toks[self.p..] {
            if token.kind == Tok::Eof {
                return false;
            }
            if token.kind != Tok::Op {
                continue;
            }
            match &self.src[token.start..token.end] {
                "[" | "(" | "{" => depth += 1,
                "]" | ")" | "}" => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return false;
                    }
                }
                "," if depth == 1 => return true,
                _ => {}
            }
        }
        false
    }

    fn parse_import_specs(&mut self) -> PResult<Vec<Import>> {
        let mut imports = Vec::new();
        if self.got_op("(") {
            while !self.at_op(")") && self.kind() != Tok::Eof {
                imports.push(self.parse_import_spec()?);
                self.expect_semi()?;
            }
            self.expect_op(")")?;
        } else {
            imports.push(self.parse_import_spec()?);
        }
        Ok(imports)
    }

    fn parse_import_spec(&mut self) -> PResult<Import> {
        let name = if self.kind() == Tok::Ident {
            Some(self.ident()?.name)
        } else if self.at_op(".") {
            self.advance();
            Some(".".to_owned())
        } else {
            None
        };
        if self.kind() != Tok::Str {
            return self.fail("expected an import path");
        }
        let path = unquote(self.text());
        self.advance();
        Ok(Import { name, path })
    }

    fn parse_func_decl(&mut self) -> PResult<FuncDecl> {
        self.expect_kw("func")?;
        let has_receiver = self.at_op("(");
        if has_receiver {
            self.parse_params()?;
        }
        let name = self.ident()?;
        if self.at_op("[") {
            self.skip_type_params()?;
        }
        let sig = self.parse_signature()?;
        let body = if self.at_op("{") {
            Some(self.parse_block()?)
        } else {
            None
        };
        Ok(FuncDecl {
            name,
            has_receiver,
            sig,
            body,
        })
    }

    fn parse_package(&mut self) -> PResult<String> {
        self.expect_kw("package")?;
        let name = self.ident()?.name;
        self.expect_semi()?;
        Ok(name)
    }

    fn parse_imports(&mut self) -> PResult<Vec<Import>> {
        let mut imports = Vec::new();
        while self.at_kw("import") {
            self.advance();
            imports.extend(self.parse_import_specs()?);
            self.expect_semi()?;
        }
        Ok(imports)
    }
}

enum ForHeader {
    Range {
        key: Option<Expr>,
        value: Option<Expr>,
        define: bool,
        x: Expr,
    },
    Clauses {
        init: Option<Box<Stmt>>,
        cond: Option<Expr>,
        post: Option<Box<Stmt>>,
    },
}

/// The one function declaration `src` holds (a Test construct), with nothing
/// after it.
pub(crate) fn parse_func_decl(src: &str) -> PResult<FuncDecl> {
    let mut parser = Parser::new(src)?;
    let decl = parser.parse_func_decl()?;
    while parser.kind() == Tok::Semi {
        parser.advance();
    }
    if parser.kind() != Tok::Eof {
        return parser.fail("text after the function declaration");
    }
    Ok(decl)
}

/// The package clause and the imports of the Go source file `src`; nothing
/// after the imports is read.
pub(crate) fn parse_file_header(src: &str) -> PResult<FileHeader> {
    let mut parser = Parser::new(src)?;
    let package = parser.parse_package()?;
    let imports = parser.parse_imports()?;
    Ok(FileHeader { package, imports })
}

/// A whole Go source file: its header and its function declarations. Other
/// top-level declarations are read (so the file is known to be valid) and
/// dropped.
pub(crate) fn parse_file(src: &str) -> PResult<(FileHeader, Vec<FuncDecl>)> {
    let mut parser = Parser::new(src)?;
    let package = parser.parse_package()?;
    let imports = parser.parse_imports()?;
    let mut funcs = Vec::new();
    while parser.kind() != Tok::Eof {
        if parser.kind() == Tok::Kw {
            match parser.text() {
                "func" => funcs.push(parser.parse_func_decl()?),
                kw @ ("var" | "const" | "type") => {
                    parser.advance();
                    parser.parse_specs(kw)?;
                }
                _ => return parser.fail("expected a declaration"),
            }
        } else if parser.kind() == Tok::Semi {
            parser.advance();
            continue;
        } else {
            return parser.fail("expected a declaration");
        }
        parser.expect_semi()?;
    }
    Ok((FileHeader { package, imports }, funcs))
}
