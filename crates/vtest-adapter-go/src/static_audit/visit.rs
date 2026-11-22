//! Walks over the Go syntax tree for the static audit.
//!
//! Responsibility: visit every expression or statement of a tree in source
//! order, optionally entering the bodies of function literals. Boundary: no
//! meaning is attached to what is visited; the callers decide what a visit
//! counts for (DES-619).

use crate::syntax::{DeclSpec, Expr, ExprKind, Stmt, StmtKind};

/// A walk over expressions and statements. `enter_lits` says whether the body
/// of a function literal is walked too.
pub(crate) struct Walker<'f> {
    pub(crate) enter_lits: bool,
    pub(crate) on_expr: &'f mut dyn FnMut(&Expr),
    pub(crate) on_stmt: &'f mut dyn FnMut(&Stmt),
}

impl Walker<'_> {
    pub(crate) fn expr(&mut self, x: &Expr) {
        (self.on_expr)(x);
        match &x.kind {
            ExprKind::Ident(_)
            | ExprKind::BasicLit { .. }
            | ExprKind::StructType
            | ExprKind::InterfaceType
            | ExprKind::Ellipsis(None)
            | ExprKind::FuncType(_) => {}
            ExprKind::CompositeLit { ty, elts } => {
                if let Some(ty) = ty {
                    self.expr(ty);
                }
                for elt in elts {
                    self.expr(elt);
                }
            }
            ExprKind::FuncLit(lit) => {
                if self.enter_lits {
                    self.stmts(&lit.body.stmts);
                }
            }
            ExprKind::Paren(x)
            | ExprKind::Star(x)
            | ExprKind::Unary(_, x)
            | ExprKind::ChanType(x)
            | ExprKind::Ellipsis(Some(x)) => self.expr(x),
            ExprKind::Selector(x, _) => self.expr(x),
            ExprKind::Index(x, idx) => {
                self.expr(x);
                for i in idx {
                    self.expr(i);
                }
            }
            ExprKind::Slice(x, parts) => {
                self.expr(x);
                for part in parts.iter().flatten() {
                    self.expr(part);
                }
            }
            ExprKind::TypeAssert(x, t) => {
                self.expr(x);
                if let Some(t) = t {
                    self.expr(t);
                }
            }
            ExprKind::Call { fun, args, .. } => {
                self.expr(fun);
                for arg in args {
                    self.expr(arg);
                }
            }
            ExprKind::Binary(_, a, b) | ExprKind::KeyValue(a, b) | ExprKind::MapType(a, b) => {
                self.expr(a);
                self.expr(b);
            }
            ExprKind::ArrayType { len, elt } => {
                if let Some(len) = len {
                    self.expr(len);
                }
                self.expr(elt);
            }
        }
    }

    pub(crate) fn stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.stmt(stmt);
        }
    }

    fn opt_stmt(&mut self, stmt: Option<&Stmt>) {
        if let Some(stmt) = stmt {
            self.stmt(stmt);
        }
    }

    pub(crate) fn stmt(&mut self, s: &Stmt) {
        (self.on_stmt)(s);
        match &s.kind {
            StmtKind::Empty | StmtKind::Branch { .. } => {}
            StmtKind::Decl(specs) => {
                for spec in specs {
                    match spec {
                        DeclSpec::Var { values, .. } | DeclSpec::Const { values, .. } => {
                            for value in values {
                                self.expr(value);
                            }
                        }
                        DeclSpec::Type { .. } => {}
                    }
                }
            }
            StmtKind::Labeled(_, inner) => self.stmt(inner),
            StmtKind::Expr(x) | StmtKind::Go(x) | StmtKind::Defer(x) | StmtKind::IncDec(x) => {
                self.expr(x);
            }
            StmtKind::Send(a, b) => {
                self.expr(a);
                self.expr(b);
            }
            StmtKind::Assign { lhs, rhs, .. } => {
                for x in lhs.iter().chain(rhs) {
                    self.expr(x);
                }
            }
            StmtKind::Return(xs) => {
                for x in xs {
                    self.expr(x);
                }
            }
            StmtKind::Block(b) => self.stmts(&b.stmts),
            StmtKind::If {
                init,
                cond,
                then,
                els,
            } => {
                self.opt_stmt(init.as_deref());
                self.expr(cond);
                self.stmts(&then.stmts);
                self.opt_stmt(els.as_deref());
            }
            StmtKind::Switch { init, tag, clauses } => {
                self.opt_stmt(init.as_deref());
                if let Some(tag) = tag {
                    self.expr(tag);
                }
                for clause in clauses {
                    for x in &clause.exprs {
                        self.expr(x);
                    }
                    self.stmts(&clause.body);
                }
            }
            StmtKind::TypeSwitch {
                init, x, clauses, ..
            } => {
                self.opt_stmt(init.as_deref());
                self.expr(x);
                for clause in clauses {
                    self.stmts(&clause.body);
                }
            }
            StmtKind::Select { clauses } => {
                for clause in clauses {
                    self.opt_stmt(clause.comm.as_deref());
                    self.stmts(&clause.body);
                }
            }
            StmtKind::For {
                init,
                cond,
                post,
                body,
            } => {
                self.opt_stmt(init.as_deref());
                if let Some(cond) = cond {
                    self.expr(cond);
                }
                self.opt_stmt(post.as_deref());
                self.stmts(&body.stmts);
            }
            StmtKind::Range {
                key,
                value,
                x,
                body,
                ..
            } => {
                for x in key.iter().chain(value.iter()) {
                    self.expr(x);
                }
                self.expr(x);
                self.stmts(&body.stmts);
            }
        }
    }
}

/// Calls `f` on `x` and every expression inside it (function literal bodies
/// included when `enter_lits`).
pub(crate) fn for_each_expr(x: &Expr, enter_lits: bool, f: &mut dyn FnMut(&Expr)) {
    Walker {
        enter_lits,
        on_expr: f,
        on_stmt: &mut |_| {},
    }
    .expr(x);
}

/// Calls `f` on every expression of `stmts` (function literal bodies included
/// when `enter_lits`).
pub(crate) fn for_each_expr_in_stmts(stmts: &[Stmt], enter_lits: bool, f: &mut dyn FnMut(&Expr)) {
    Walker {
        enter_lits,
        on_expr: f,
        on_stmt: &mut |_| {},
    }
    .stmts(stmts);
}

/// Calls `f` on every statement of `stmts`, nested ones included (function
/// literal bodies when `enter_lits`).
pub(crate) fn for_each_stmt(stmts: &[Stmt], enter_lits: bool, f: &mut dyn FnMut(&Stmt)) {
    Walker {
        enter_lits,
        on_expr: &mut |_| {},
        on_stmt: f,
    }
    .stmts(stmts);
}
