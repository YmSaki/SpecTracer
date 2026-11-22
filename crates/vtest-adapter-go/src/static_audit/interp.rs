//! The abstract reading of one Go Test function for DA-001 to DA-006.
//!
//! Responsibility: walk the function once in execution order, with the scopes
//! of Go, and collect (a) the assert-equivalent calls with the positions that
//! decide whether they run (DS-1884 to DS-1888, DS-1983), and (b) for one
//! declared target, how a value derived from the target's call result flows
//! (DS-1968, DS-1970, DS-1975, DS-1976, DS-1984, DS-1986) and where it reaches
//! an assert-equivalent (DS-1935, DS-1947). Branches join their states, a loop
//! is iterated to a fixed point, and a path that ends in a failure report does
//! not take part in a join (DS-1975, DS-1973).
//!
//! Boundary: no verdict is made here (`static_audit` composes the rules from
//! the facts), and nothing outside the function is read: a function that is
//! not the target is a delegation target (DS-641, DS-1936) and the audit
//! never confirms that one terminates (DS-643), so a derived value that goes
//! through one only ever makes a rule `UNKNOWN`. Where the walk meets a form
//! it does not follow, it records that (`taint`, `uncertain`) and the rules
//! answer `UNKNOWN`; it never reads a form as safe (DS-615, DS-1680).

use std::collections::{BTreeSet, HashMap};

use super::constant::{absorbing_literal, classify, const_bool, is_bool_literal, Constness, Names};
use super::target::{FileInfo, TargetMatch, TargetSpec};
use super::visit::{for_each_expr, for_each_expr_in_stmts, for_each_stmt};
use crate::syntax::{
    tokenize, Block, BranchKind, CaseClause, DeclSpec, Expr, ExprKind, FuncDecl, FuncType, Ident,
    Span, Stmt, StmtKind, Tok,
};

/// DS-1884: the methods of a `*testing.T` / `testing.TB` that report a failure.
const FAILURE_METHODS: &[&str] = &["Error", "Errorf", "Fatal", "Fatalf", "Fail", "FailNow"];
/// The failure reports that end the goroutine (`FailNow`, which `Fatal` calls).
const TERMINATING_FAILURES: &[&str] = &["Fatal", "Fatalf", "FailNow"];
/// DS-1973: not failures, and ways out of the function.
const SKIP_METHODS: &[&str] = &["Skip", "Skipf", "SkipNow"];
/// DS-1986: the names of the built-in types, whose conversions are not
/// delegations.
const BUILTIN_TYPES: &[&str] = &[
    "any",
    "bool",
    "byte",
    "complex64",
    "complex128",
    "error",
    "float32",
    "float64",
    "int",
    "int8",
    "int16",
    "int32",
    "int64",
    "rune",
    "string",
    "uint",
    "uint8",
    "uint16",
    "uint32",
    "uint64",
    "uintptr",
];
/// The built-in functions other than `len`, `cap`, `panic` and `recover`.
const OTHER_BUILTINS: &[&str] = &[
    "append", "clear", "close", "complex", "copy", "delete", "imag", "make", "max", "min", "new",
    "print", "println", "real",
];
/// DS-1986: the comparison functions, by their spelling.
const COMPARISONS: &[&str] = &["reflect.DeepEqual", "errors.Is", "bytes.Equal"];
/// The passes a loop is iterated for; the state lattice is finite, so this is
/// a bound that is never reached by a function that was read correctly.
const MAX_PASSES: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum D {
    /// Not derived from the target's result.
    No,
    /// Derived (DS-1968).
    Yes,
    /// May or may not be (DS-629).
    Unk,
}

/// What the analysis knows of a value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Val {
    d: D,
    /// A composite literal that holds a derived element (DS-1984): read only
    /// as a whole.
    comp: bool,
    /// The value or a binding it comes from holds a call that is a delegation
    /// target (DS-641, DS-1986).
    deleg: bool,
}

impl Val {
    const NO: Val = Val {
        d: D::No,
        comp: false,
        deleg: false,
    };
    const YES: Val = Val {
        d: D::Yes,
        comp: false,
        deleg: false,
    };
    const UNK: Val = Val {
        d: D::Unk,
        comp: false,
        deleg: false,
    };

    fn derived(self) -> bool {
        self.d != D::No
    }

    /// The value after two paths meet (DS-1975): derived on both is derived,
    /// derived on neither is not, anything else is unknown.
    fn join(self, other: Val) -> Val {
        let deleg = self.deleg || other.deleg;
        if self.d == other.d && self.comp == other.comp {
            Val { deleg, ..self }
        } else {
            Val {
                d: D::Unk,
                comp: false,
                deleg,
            }
        }
    }

    /// The value of an expression made of two operands.
    fn merge(self, other: Val) -> Val {
        let d = if self.d == D::Yes || other.d == D::Yes {
            D::Yes
        } else if self.d == D::Unk || other.d == D::Unk {
            D::Unk
        } else {
            D::No
        };
        Val {
            d,
            comp: false,
            deleg: self.deleg || other.deleg,
        }
    }
}

type Env = Vec<Val>;

fn env_get(env: &Env, id: usize) -> Val {
    env.get(id).copied().unwrap_or(Val::NO)
}

fn join_env(a: &Env, b: &Env) -> Env {
    (0..a.len().max(b.len()))
        .map(|id| env_get(a, id).join(env_get(b, id)))
        .collect()
}

fn join_opt(a: Option<Env>, b: Option<Env>) -> Option<Env> {
    match (a, b) {
        (Some(a), Some(b)) => Some(join_env(&a, &b)),
        (a, None) => a,
        (None, b) => b,
    }
}

/// What the flow of an expression is read for.
#[derive(Clone, Copy)]
struct Ctx {
    /// The expression is a deciding position, so a composite value compared
    /// whole by `==` / `!=` or by a comparison function counts (DS-1984).
    decider: bool,
}

const PLAIN: Ctx = Ctx { decider: false };
const DECIDER: Ctx = Ctx { decider: true };

struct VarInfo {
    name: String,
    /// Declared as a `*testing.T` / `testing.TB` (DS-1884).
    t_typed: bool,
}

/// A position that decides whether a later statement runs (DS-1887).
#[derive(Clone)]
struct DEntry {
    val: Val,
    constness: Constness,
    /// A constant `false` guards this position: what it decides never runs.
    never_run: bool,
    /// A `==` / `!=` in it has the same operand on both sides (DS-1889).
    self_cmp: bool,
    /// Whether the position is reached is not decided by the syntax (DS-1983).
    undecided: bool,
}

/// A deciding position added by an exit that comes before (DS-1983).
struct Extra {
    entry: DEntry,
    /// The index of the break frame whose end ends its effect (`break` out of
    /// a `switch`); `None` for an exit from the function.
    until: Option<usize>,
}

/// The function body being read (the Test, a `t.Run` literal, any other
/// literal: DS-1982 reads each as a function body of its own).
struct Frame {
    /// The depth of the deciding stack when the body began.
    base: usize,
    extras: Vec<Extra>,
    /// An exit whose condition the syntax does not fix came before (DS-1983).
    undecided_after: bool,
    /// A `defer` of this body may recover a `panic` (DS-1982).
    recover_defer: bool,
    loop_depth: usize,
    returns: Vec<Env>,
}

struct BreakFrame {
    is_loop: bool,
    breaks: Vec<Env>,
    continues: Vec<Env>,
}

/// How a statement that is a call ends the path (DS-1973).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Term {
    None,
    /// `Error`, `Errorf`, `Fail`: a failure, and the path goes on.
    Failure,
    /// `Fatal`, `Fatalf`, `FailNow`.
    Fatal,
    Panic,
    /// `os.Exit`, `runtime.Goexit`.
    Exit,
    /// `Skip`, `Skipf`, `SkipNow`.
    Skip,
}

/// One assert-equivalent call of the function (DS-1883).
#[derive(Clone, Debug)]
pub(crate) struct Site {
    /// A `panic(...)` that a `defer` of its body may recover (DS-1982).
    pub(crate) neutralized: bool,
    pub(crate) constness: Constness,
    /// A `==` / `!=` in a position enclosing it compares a value with itself.
    pub(crate) self_cmp: bool,
    /// Whether it runs is not decided by the syntax (DS-1983), or it never
    /// runs: the rules do not read its absence of a deciding derived value as
    /// a proof (DS-1983).
    pub(crate) undecided: bool,
}

/// What the walk found without regard to a target.
#[derive(Debug, Default)]
pub(crate) struct Structure {
    pub(crate) sites: Vec<Site>,
    /// A call named like a failure report on a receiver that is not declared a
    /// `testing.T` (DS-1979 reads only declared ones).
    pub(crate) maybe_asserts: usize,
    /// Calls that are delegation targets of a body with no assert-equivalent
    /// (DS-642).
    pub(crate) candidates: usize,
    pub(crate) empty_body: bool,
    /// The Test takes a `T` through a package name other than `testing`.
    pub(crate) unrecognized_t: bool,
}

/// What the walk found about one declared target.
#[derive(Debug, Default)]
pub(crate) struct Facts {
    /// Calls of the target at a counted position (DS-1970).
    pub(crate) counted_calls: usize,
    /// Calls of the target where DS-1970 does not count them.
    pub(crate) uncounted_calls: usize,
    /// Calls that may be of the target.
    pub(crate) ambiguous_calls: usize,
    /// A derived value decides an assert-equivalent directly.
    pub(crate) reach_clean: bool,
    /// A derived value reaches one only with a delegation target in the way.
    pub(crate) reach_deleg: bool,
    /// A value that may be derived reaches one, or a derived one reaches one
    /// that may not run.
    pub(crate) reach_unk: bool,
    /// A derived value left the part of the function the walk follows.
    pub(crate) uncertain: bool,
    /// A form the walk does not follow (`goto`, labeled `break`, `select`).
    pub(crate) taint: bool,
    /// A derived value is read where DS-1970 does not count a position.
    pub(crate) uncounted_flow: bool,
}

/// The input of one walk.
pub(crate) struct Config<'a> {
    /// The text the tree was read from.
    pub(crate) src: &'a str,
    pub(crate) file: &'a FileInfo,
    /// The `assertion_macros` of the config (DS-1886).
    pub(crate) macros: &'a [String],
    pub(crate) all_targets: &'a [TargetSpec],
}

/// What a callee is.
enum Callee<'e> {
    Panic,
    Recover,
    LenCap,
    Conversion,
    Builtin(&'static str),
    Compare,
    Failure {
        terminal: bool,
    },
    Skip,
    Run,
    /// Another method of a `testing.T`.
    TMethod,
    Exit,
    Macro,
    Target(TargetMatch),
    OtherTarget,
    Method(&'e Expr),
    Function,
}

enum SiteKind {
    Failure,
    Panic,
    Macro { val: Val, constness: Constness },
}

enum LoopHeader<'e> {
    For {
        cond: Option<&'e Expr>,
        post: Option<&'e Stmt>,
    },
    Range {
        xval: Val,
        key: Option<&'e Expr>,
        value: Option<&'e Expr>,
        define: bool,
        x: &'e Expr,
    },
}

struct PassOut {
    next_head: Env,
    exit: Option<Env>,
    breaks: Vec<Env>,
}

struct Interp<'a> {
    cfg: &'a Config<'a>,
    target: Option<&'a TargetSpec>,
    scopes: Vec<HashMap<String, usize>>,
    vars: Vec<VarInfo>,
    decl_ids: HashMap<usize, usize>,
    /// The names that a part the walk does not count refers to.
    captured: BTreeSet<String>,
    env: Option<Env>,
    counted: bool,
    dead: bool,
    recording: bool,
    dec: Vec<DEntry>,
    frames: Vec<Frame>,
    breaks: Vec<BreakFrame>,
    last_term: Term,
    last_stmt_failure: bool,
    facts: Facts,
    structure: Structure,
}

impl Names for Interp<'_> {
    fn is_local(&self, name: &str) -> bool {
        self.lookup(name).is_some()
    }
}

fn strip(x: &Expr) -> &Expr {
    match &x.kind {
        ExprKind::Paren(inner) => strip(inner),
        _ => x,
    }
}

/// The identifier a place expression is rooted at (`x` of `x.f[i]`).
fn root_ident(x: &Expr) -> Option<&str> {
    match &x.kind {
        ExprKind::Ident(name) => Some(name),
        ExprKind::Paren(inner)
        | ExprKind::Star(inner)
        | ExprKind::Selector(inner, _)
        | ExprKind::Index(inner, _)
        | ExprKind::Slice(inner, _)
        | ExprKind::TypeAssert(inner, _) => root_ident(inner),
        _ => None,
    }
}

/// `assert.Equal` for a chain of identifiers separated by dots (DS-1886).
fn dotted_name(x: &Expr) -> Option<String> {
    match &strip(x).kind {
        ExprKind::Ident(name) => Some(name.clone()),
        ExprKind::Selector(base, sel) => {
            dotted_name(base).map(|head| format!("{head}.{}", sel.name))
        }
        _ => None,
    }
}

fn names_in_expr(x: &Expr, out: &mut BTreeSet<String>) {
    for_each_expr(x, true, &mut |e| {
        if let ExprKind::Ident(name) = &e.kind {
            out.insert(name.clone());
        }
    });
}

fn names_in_stmts(stmts: &[Stmt], out: &mut BTreeSet<String>) {
    for_each_expr_in_stmts(stmts, true, &mut |e| {
        if let ExprKind::Ident(name) = &e.kind {
            out.insert(name.clone());
        }
    });
}

/// DS-1982: whether a `defer` of this body may recover a `panic`: one that
/// calls a function literal that calls `recover()`, or one that calls anything
/// but a function literal.
fn has_recover_defer(stmts: &[Stmt]) -> bool {
    let mut found = false;
    for_each_stmt(stmts, false, &mut |s| {
        let StmtKind::Defer(call) = &s.kind else {
            return;
        };
        let ExprKind::Call { fun, .. } = &call.kind else {
            found = true;
            return;
        };
        match &strip(fun).kind {
            ExprKind::FuncLit(lit) => {
                for_each_expr_in_stmts(&lit.body.stmts, true, &mut |e| {
                    if let ExprKind::Call { fun, .. } = &e.kind {
                        if matches!(&strip(fun).kind, ExprKind::Ident(name) if name == "recover") {
                            found = true;
                        }
                    }
                });
            }
            _ => found = true,
        }
    });
    found
}

/// DS-1983: whether the body assigns a name that the loop's condition or range
/// expression reads (the post statement of a `for` is not part of the body).
fn assigns_any(body: &[Stmt], names: &BTreeSet<String>) -> bool {
    let mut found = false;
    let mut place = |x: &Expr| {
        if root_ident(x).is_some_and(|name| names.contains(name)) {
            found = true;
        }
    };
    for_each_stmt(body, true, &mut |s| match &s.kind {
        StmtKind::Assign { lhs, op, .. } if *op != ":=" => lhs.iter().for_each(&mut place),
        StmtKind::IncDec(x) => place(x),
        StmtKind::Range {
            key,
            value,
            define: false,
            ..
        } => key.iter().chain(value.iter()).for_each(&mut place),
        _ => {}
    });
    found
}

impl<'a> Interp<'a> {
    fn new(cfg: &'a Config<'a>, target: Option<&'a TargetSpec>) -> Self {
        Self {
            cfg,
            target,
            scopes: vec![HashMap::new()],
            vars: Vec::new(),
            decl_ids: HashMap::new(),
            captured: BTreeSet::new(),
            env: Some(Vec::new()),
            counted: true,
            dead: false,
            recording: true,
            dec: Vec::new(),
            frames: Vec::new(),
            breaks: Vec::new(),
            last_term: Term::None,
            last_stmt_failure: false,
            facts: Facts::default(),
            structure: Structure::default(),
        }
    }

    // -- facts ---------------------------------------------------------

    fn uncertain(&mut self) {
        if self.recording {
            self.facts.uncertain = true;
        }
    }

    fn taint(&mut self) {
        if self.recording {
            self.facts.taint = true;
        }
    }

    // -- scopes and state ----------------------------------------------

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn lookup(&self, name: &str) -> Option<usize> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }

    fn get(&self, id: usize) -> Val {
        self.env.as_ref().map_or(Val::NO, |env| env_get(env, id))
    }

    fn set(&mut self, id: usize, val: Val) {
        if val.derived() && self.recording && self.captured.contains(&self.vars[id].name) {
            // A part that DS-1970 does not count reads this name.
            self.facts.uncounted_flow = true;
            self.facts.uncertain = true;
        }
        if let Some(env) = &mut self.env {
            if env.len() <= id {
                env.resize(id + 1, Val::NO);
            }
            env[id] = val;
        }
    }

    fn declare(&mut self, ident: &Ident, t_typed: bool, val: Val) -> Option<usize> {
        if ident.name == "_" {
            return None;
        }
        let id = match self.decl_ids.get(&ident.span.start) {
            Some(id) => *id,
            None => {
                self.vars.push(VarInfo {
                    name: ident.name.clone(),
                    t_typed,
                });
                self.decl_ids.insert(ident.span.start, self.vars.len() - 1);
                self.vars.len() - 1
            }
        };
        self.vars[id].t_typed = t_typed;
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(ident.name.clone(), id);
        }
        self.set(id, val);
        Some(id)
    }

    fn is_testing(&self, x: &Expr, name: &str) -> bool {
        match &x.kind {
            ExprKind::Selector(base, sel) => {
                sel.name == name
                    && matches!(&base.kind, ExprKind::Ident(pkg) if pkg == "testing")
                    && self.lookup("testing").is_none()
            }
            _ => false,
        }
    }

    /// DS-1884: `*testing.T` or `testing.TB`.
    fn type_is_t(&self, ty: &Expr) -> bool {
        match &ty.kind {
            ExprKind::Star(inner) => self.is_testing(inner, "T"),
            ExprKind::Paren(inner) => self.type_is_t(inner),
            _ => self.is_testing(ty, "TB"),
        }
    }

    /// A `T` taken through another package name (`*tt.T`).
    fn type_is_foreign_t(&self, ty: &Expr) -> bool {
        let selector = match &ty.kind {
            ExprKind::Star(inner) => inner,
            _ => ty,
        };
        matches!(
            &selector.kind,
            ExprKind::Selector(base, sel)
                if matches!(sel.name.as_str(), "T" | "TB")
                    && matches!(&base.kind, ExprKind::Ident(pkg) if pkg != "testing")
        )
    }

    fn declare_params(&mut self, sig: &FuncType, top: bool) {
        for field in &sig.params {
            let t_typed = self.type_is_t(&field.ty);
            if top && self.type_is_foreign_t(&field.ty) {
                self.structure.unrecognized_t = true;
            }
            for name in &field.names {
                self.declare(name, t_typed, Val::NO);
            }
        }
    }

    fn is_t_ident(&self, x: &Expr) -> bool {
        match &strip(x).kind {
            ExprKind::Ident(name) => self.lookup(name).is_some_and(|id| self.vars[id].t_typed),
            _ => false,
        }
    }

    // -- functions and regions -----------------------------------------

    /// Reads the body of a function (the Test, or a function literal).
    fn exec_func_body(&mut self, sig: &FuncType, body: &Block, top: bool) {
        self.push_scope();
        self.declare_params(sig, top);
        let saved_breaks = std::mem::take(&mut self.breaks);
        self.frames.push(Frame {
            base: self.dec.len(),
            extras: Vec::new(),
            undecided_after: false,
            recover_defer: has_recover_defer(&body.stmts),
            loop_depth: 0,
            returns: Vec::new(),
        });
        self.exec_stmts(&body.stmts);
        let frame = self.frames.pop();
        let mut end = self.env.take();
        for state in frame.into_iter().flat_map(|frame| frame.returns) {
            end = join_opt(end, Some(state));
        }
        self.env = end;
        self.breaks = saved_breaks;
        self.pop_scope();
    }

    /// Reads `f` in a part that DS-1970 does not count: no value is derived
    /// there, and nothing assigned there reaches the counted code.
    fn in_uncounted(&mut self, f: impl FnOnce(&mut Self)) {
        let saved_env = self.env.take();
        let saved_counted = self.counted;
        self.env = Some(Vec::new());
        self.counted = false;
        f(self);
        self.env = saved_env;
        self.counted = saved_counted;
    }

    /// A derived value that an uncounted part refers to is no longer known
    /// (DS-1976), and a derived value assigned to such a name later is not
    /// read where it flows (DS-1970).
    fn mark_captured(&mut self, names: BTreeSet<String>) {
        for name in &names {
            if let Some(id) = self.lookup(name) {
                let val = self.get(id);
                if val.derived() {
                    self.set(
                        id,
                        Val {
                            d: D::Unk,
                            comp: false,
                            deleg: val.deleg,
                        },
                    );
                    if self.recording {
                        self.facts.uncounted_flow = true;
                        self.facts.uncertain = true;
                    }
                }
            }
        }
        self.captured.extend(names);
    }

    /// A function literal that is not read as a continuation of the Test body.
    fn eval_func_lit(&mut self, sig: &FuncType, body: &Block) {
        let mut names = BTreeSet::new();
        names_in_stmts(&body.stmts, &mut names);
        self.mark_captured(names);
        self.in_uncounted(|this| this.exec_func_body(sig, body, false));
    }

    /// A call started by `go` or `defer` (DS-1970).
    fn eval_uncounted_call(&mut self, call: &Expr) {
        let mut names = BTreeSet::new();
        names_in_expr(call, &mut names);
        self.mark_captured(names);
        self.in_uncounted(|this| {
            this.eval(call, PLAIN);
        });
    }

    /// DS-1933: the literal passed to `t.Run` runs as the rest of the body.
    fn run_continuation(&mut self, sig: &FuncType, body: &Block) {
        let entry = self.env.clone();
        self.exec_func_body(sig, body, false);
        if self.env.is_none() {
            self.env = entry;
        }
    }

    fn walk_dead(&mut self, s: &Stmt) {
        let saved_env = self.env.take();
        let (counted, dead) = (self.counted, self.dead);
        self.env = Some(Vec::new());
        self.counted = false;
        self.dead = true;
        self.exec_stmt(s);
        self.counted = counted;
        self.dead = dead;
        self.env = saved_env;
    }

    // -- statements ----------------------------------------------------

    fn exec_stmts(&mut self, stmts: &[Stmt]) {
        for s in stmts {
            if self.env.is_none() {
                self.walk_dead(s);
            } else {
                self.exec_stmt(s);
            }
        }
    }

    /// A branch's statements: the path that ends in a failure report does not
    /// reach what follows (DS-1975, DS-1973).
    fn exec_arm(&mut self, stmts: &[Stmt]) {
        self.push_scope();
        self.last_stmt_failure = false;
        self.exec_stmts(stmts);
        let ends_in_failure = self.last_stmt_failure && self.env.is_some();
        self.pop_scope();
        if ends_in_failure {
            self.env = None;
        }
        self.last_stmt_failure = false;
    }

    fn exec_stmt(&mut self, s: &Stmt) {
        self.last_stmt_failure = false;
        match &s.kind {
            StmtKind::Empty => {}
            StmtKind::Decl(specs) => self.exec_decl(specs),
            StmtKind::Labeled(_, inner) => self.exec_stmt(inner),
            StmtKind::Expr(x) => self.exec_expr_stmt(x),
            StmtKind::Send(ch, value) => {
                self.eval(ch, PLAIN);
                let v = self.eval(value, PLAIN);
                if v.derived() {
                    // The value is in the channel; what a receive reads is not
                    // followed (DS-629).
                    self.escape(value);
                    if let Some(id) = root_ident(ch).and_then(|name| self.lookup(name)) {
                        let current = self.get(id);
                        self.set(
                            id,
                            Val {
                                d: D::Unk,
                                comp: false,
                                deleg: current.deleg || v.deleg,
                            },
                        );
                    }
                    self.uncertain();
                }
            }
            StmtKind::IncDec(x) => {
                if !matches!(&strip(x).kind, ExprKind::Ident(_)) {
                    self.assign_place(x, Val::NO);
                }
            }
            StmtKind::Assign { lhs, op, rhs } => self.exec_assign(lhs, op, rhs),
            StmtKind::Go(call) | StmtKind::Defer(call) => self.eval_uncounted_call(call),
            StmtKind::Return(results) => {
                for result in results {
                    self.eval(result, PLAIN);
                }
                self.record_exit(false);
                if let (Some(env), Some(frame)) = (self.env.clone(), self.frames.last_mut()) {
                    frame.returns.push(env);
                }
                self.env = None;
            }
            StmtKind::Branch { kind, label } => self.exec_branch(*kind, label.is_some()),
            StmtKind::Block(block) => self.exec_arm(&block.stmts),
            StmtKind::If {
                init,
                cond,
                then,
                els,
            } => self.exec_if(init.as_deref(), cond, then, els.as_deref()),
            StmtKind::Switch { init, tag, clauses } => {
                self.exec_switch(init.as_deref(), tag.as_ref(), None, clauses);
            }
            StmtKind::TypeSwitch {
                init,
                bind,
                x,
                clauses,
            } => self.exec_switch(init.as_deref(), Some(x), Some(bind.as_ref()), clauses),
            StmtKind::Select { clauses } => {
                self.taint();
                self.record_undecided();
                let base = self.env.clone();
                let mut ends: Option<Env> = None;
                let mut any = false;
                for clause in clauses {
                    self.env = base.clone();
                    self.push_scope();
                    if let Some(comm) = &clause.comm {
                        self.exec_stmt(comm);
                    }
                    self.exec_arm(&clause.body);
                    self.pop_scope();
                    ends = join_opt(ends, self.env.take());
                    any = true;
                }
                self.env = if any { ends } else { base };
            }
            StmtKind::For {
                init,
                cond,
                post,
                body,
            } => {
                self.push_scope();
                if let Some(init) = init {
                    self.exec_stmt(init);
                }
                self.exec_loop(
                    LoopHeader::For {
                        cond: cond.as_ref(),
                        post: post.as_deref(),
                    },
                    body,
                );
                self.pop_scope();
            }
            StmtKind::Range {
                key,
                value,
                define,
                x,
                body,
            } => {
                let xval = self.eval(x, PLAIN);
                self.push_scope();
                self.exec_loop(
                    LoopHeader::Range {
                        xval,
                        key: key.as_ref(),
                        value: value.as_ref(),
                        define: *define,
                        x,
                    },
                    body,
                );
                self.pop_scope();
            }
        }
        if !matches!(s.kind, StmtKind::Expr(_)) {
            self.last_stmt_failure = false;
        }
    }

    fn exec_decl(&mut self, specs: &[DeclSpec]) {
        for spec in specs {
            match spec {
                DeclSpec::Var { names, ty, values } => {
                    let vals: Vec<Val> = values.iter().map(|v| self.eval(v, PLAIN)).collect();
                    let t_typed = ty.as_ref().is_some_and(|ty| self.type_is_t(ty));
                    for (i, name) in names.iter().enumerate() {
                        let val = if vals.len() == 1 && names.len() > 1 {
                            vals[0]
                        } else {
                            vals.get(i).copied().unwrap_or(Val::NO)
                        };
                        self.declare(name, t_typed, val);
                    }
                    if !names.iter().all(|n| n.name == "_") {
                        for value in values {
                            self.alias_source(value);
                        }
                    }
                }
                DeclSpec::Const { names, values } => {
                    for value in values {
                        self.eval(value, PLAIN);
                    }
                    for name in names {
                        self.declare(name, false, Val::NO);
                    }
                }
                DeclSpec::Type { .. } => {}
            }
        }
    }

    fn exec_expr_stmt(&mut self, x: &Expr) {
        self.last_term = Term::None;
        self.eval(x, PLAIN);
        let term = if matches!(strip(x).kind, ExprKind::Call { .. }) {
            self.last_term
        } else {
            Term::None
        };
        match term {
            Term::None => {}
            Term::Failure => self.last_stmt_failure = true,
            Term::Fatal | Term::Panic => {
                self.last_stmt_failure = true;
                self.env = None;
            }
            Term::Exit | Term::Skip => {
                self.record_exit(false);
                self.env = None;
            }
        }
    }

    fn exec_branch(&mut self, kind: BranchKind, labeled: bool) {
        match kind {
            BranchKind::Fallthrough => {}
            BranchKind::Goto => {
                self.taint();
                self.record_exit(true);
                self.env = None;
            }
            BranchKind::Break | BranchKind::Continue => {
                if labeled {
                    self.taint();
                    self.record_exit(true);
                    self.env = None;
                    return;
                }
                let want_loop = kind == BranchKind::Continue;
                let index = self
                    .breaks
                    .iter()
                    .rposition(|frame| frame.is_loop || !want_loop);
                match (index, self.env.clone()) {
                    (Some(index), Some(env)) => {
                        let loop_frame = self.breaks[index].is_loop;
                        if want_loop {
                            self.breaks[index].continues.push(env);
                        } else {
                            self.breaks[index].breaks.push(env);
                        }
                        self.record_break(index, loop_frame);
                    }
                    _ => self.record_exit(true),
                }
                self.env = None;
            }
        }
    }

    /// An exit that leaves the function or a statement (DS-1983): what
    /// decides it also decides what comes after, in the same body.
    fn record_exit(&mut self, unfixed: bool) {
        let depth = self.dec.len();
        let Some(frame) = self.frames.last_mut() else {
            return;
        };
        if unfixed || frame.loop_depth > 0 {
            frame.undecided_after = true;
        } else if depth > frame.base {
            for entry in &self.dec[frame.base..] {
                frame.extras.push(Extra {
                    entry: entry.clone(),
                    until: None,
                });
            }
        }
    }

    /// `break` or `continue` that leaves the statement at `index`.
    fn record_break(&mut self, index: usize, loop_frame: bool) {
        let depth = self.dec.len();
        let Some(frame) = self.frames.last_mut() else {
            return;
        };
        if loop_frame || frame.loop_depth > 0 {
            frame.undecided_after = true;
        } else if depth > frame.base {
            for entry in &self.dec[frame.base..] {
                frame.extras.push(Extra {
                    entry: entry.clone(),
                    until: Some(index),
                });
            }
        }
    }

    fn record_undecided(&mut self) {
        if let Some(frame) = self.frames.last_mut() {
            frame.undecided_after = true;
        }
    }

    fn make_entry(&self, cond: &Expr, val: Val) -> DEntry {
        let val = if val.comp {
            Val {
                d: D::Unk,
                comp: false,
                deleg: val.deleg,
            }
        } else {
            val
        };
        DEntry {
            val,
            constness: classify(cond, self),
            never_run: false,
            self_cmp: self.has_self_comparison(cond),
            undecided: false,
        }
    }

    fn tokens_of(&self, span: Span) -> Option<Vec<(Tok, &'a str)>> {
        let src: &'a str = self.cfg.src;
        let text = src.get(span.start..span.end)?;
        let tokens = tokenize(text).ok()?;
        Some(
            tokens
                .iter()
                .filter(|t| !matches!(t.kind, Tok::Semi | Tok::Eof))
                .map(|t| (t.kind, &text[t.start..t.end]))
                .collect(),
        )
    }

    /// DS-1889: a `==` / `!=` whose two sides are the same tokens.
    fn has_self_comparison(&self, cond: &Expr) -> bool {
        let mut found = false;
        for_each_expr(cond, false, &mut |e| {
            if let ExprKind::Binary("==" | "!=", a, b) = &e.kind {
                let (ta, tb) = (self.tokens_of(a.span), self.tokens_of(b.span));
                if ta.is_some() && ta == tb {
                    found = true;
                }
            }
        });
        found
    }

    fn exec_if(&mut self, init: Option<&Stmt>, cond: &Expr, then: &Block, els: Option<&Stmt>) {
        self.push_scope();
        if let Some(init) = init {
            self.exec_stmt(init);
        }
        let val = self.eval(cond, DECIDER);
        let entry = self.make_entry(cond, val);
        let fixed = const_bool(cond, self);
        let base = self.env.clone();

        let mut then_entry = entry.clone();
        then_entry.never_run = fixed == Some(false);
        self.dec.push(then_entry);
        self.exec_arm(&then.stmts);
        self.dec.pop();
        let then_env = self.env.take();

        self.env = base;
        if let Some(els) = els {
            let mut else_entry = entry;
            else_entry.never_run = fixed == Some(true);
            self.dec.push(else_entry);
            self.exec_stmt(els);
            self.dec.pop();
        }
        let else_env = self.env.take();
        self.env = join_opt(then_env, else_env);
        self.pop_scope();
    }

    fn exec_switch(
        &mut self,
        init: Option<&Stmt>,
        tag: Option<&Expr>,
        bind: Option<Option<&Ident>>,
        clauses: &[CaseClause],
    ) {
        self.push_scope();
        if let Some(init) = init {
            self.exec_stmt(init);
        }
        let tag_val = tag.map(|t| self.eval(t, PLAIN));
        let tag_entry = tag.zip(tag_val).map(|(t, v)| self.make_entry(t, v));
        let mut case_entries: Vec<Vec<DEntry>> = Vec::new();
        for clause in clauses {
            let mut entries = Vec::new();
            for e in &clause.exprs {
                if bind.is_some() {
                    // A type switch: the clause lists types.
                    continue;
                }
                let v = self.eval(e, PLAIN);
                entries.push(self.make_entry(e, v));
            }
            case_entries.push(entries);
        }
        let base = self.env.clone();
        let has_default = clauses.iter().any(|c| c.default);
        self.breaks.push(BreakFrame {
            is_loop: false,
            breaks: Vec::new(),
            continues: Vec::new(),
        });
        let frame_index = self.breaks.len() - 1;
        let mut ends: Option<Env> = None;
        let mut carry: Option<Env> = None;
        for (k, clause) in clauses.iter().enumerate() {
            let default_in_scope = clauses[..=k].iter().any(|c| c.default);
            let range = if default_in_scope {
                0..clauses.len()
            } else {
                0..k + 1
            };
            let mut pushed = 0;
            if let Some(entry) = &tag_entry {
                self.dec.push(entry.clone());
                pushed += 1;
            }
            for entries in &case_entries[range] {
                for entry in entries {
                    self.dec.push(entry.clone());
                    pushed += 1;
                }
            }
            self.env = join_opt(base.clone(), carry.take());
            self.push_scope();
            if let Some(Some(ident)) = bind {
                let span = Span {
                    start: clause.span.start,
                    end: clause.span.start,
                };
                let xv = tag_val.unwrap_or(Val::NO);
                let val = if xv.comp {
                    Val {
                        d: D::Unk,
                        comp: false,
                        deleg: xv.deleg,
                    }
                } else {
                    xv
                };
                self.declare(
                    &Ident {
                        name: ident.name.clone(),
                        span,
                    },
                    false,
                    val,
                );
            }
            self.exec_arm_inline(&clause.body);
            self.pop_scope();
            for _ in 0..pushed {
                self.dec.pop();
            }
            let falls = matches!(
                clause.body.last().map(|s| &s.kind),
                Some(StmtKind::Branch {
                    kind: BranchKind::Fallthrough,
                    ..
                })
            );
            if falls {
                carry = self.env.take();
            } else {
                ends = join_opt(ends, self.env.take());
            }
        }
        let frame = self.breaks.pop();
        if let Some(frame) = frame {
            for state in frame.breaks {
                ends = join_opt(ends, Some(state));
            }
        }
        if let Some(frame) = self.frames.last_mut() {
            frame
                .extras
                .retain(|extra| extra.until != Some(frame_index));
        }
        let mut exit = ends;
        if !has_default {
            exit = join_opt(exit, base);
        }
        self.env = exit;
        self.pop_scope();
    }

    /// A clause body: like an arm, but the scope belongs to the caller.
    fn exec_arm_inline(&mut self, stmts: &[Stmt]) {
        self.last_stmt_failure = false;
        self.exec_stmts(stmts);
        let ends_in_failure = self.last_stmt_failure && self.env.is_some();
        if ends_in_failure {
            self.env = None;
        }
        self.last_stmt_failure = false;
    }

    // -- loops ---------------------------------------------------------

    fn exec_loop(&mut self, header: LoopHeader<'_>, body: &Block) {
        let Some(entry_env) = self.env.clone() else {
            return;
        };
        let original_recording = self.recording;
        // DS-1983: the body assigns a name its condition reads.
        let mut read = BTreeSet::new();
        match &header {
            LoopHeader::For {
                cond: Some(cond), ..
            } => names_in_expr(cond, &mut read),
            LoopHeader::Range { x, .. } => names_in_expr(x, &mut read),
            LoopHeader::For { cond: None, .. } => {}
        }
        let undecided = assigns_any(&body.stmts, &read);
        let mut head = entry_env;
        let mut converged = false;
        for _ in 0..MAX_PASSES {
            self.recording = false;
            let out = self.loop_pass(&header, body, head.clone(), undecided);
            self.recording = original_recording;
            if out.next_head == head {
                converged = true;
                break;
            }
            head = out.next_head;
        }
        if !converged {
            self.taint();
        }
        let out = self.loop_pass(&header, body, head, undecided);
        let mut exit = out.exit;
        for state in out.breaks {
            exit = join_opt(exit, Some(state));
        }
        self.env = exit;
    }

    fn loop_pass(
        &mut self,
        header: &LoopHeader<'_>,
        body: &Block,
        head: Env,
        undecided: bool,
    ) -> PassOut {
        self.env = Some(head.clone());
        let mut entry = None;
        match header {
            LoopHeader::For { cond, .. } => {
                if let Some(cond) = cond {
                    let val = self.eval(cond, DECIDER);
                    entry = Some(self.make_entry(cond, val));
                }
            }
            LoopHeader::Range { xval, x, .. } => {
                entry = Some(self.make_entry(x, *xval));
            }
        }
        if let Some(entry) = &mut entry {
            entry.undecided = undecided;
        }
        let exit = self.env.clone();
        self.push_scope();
        if let LoopHeader::Range {
            xval,
            key,
            value,
            define,
            ..
        } = header
        {
            self.bind_range(*xval, *key, *value, *define);
        }
        let pushed = entry.is_some();
        if let Some(entry) = entry {
            self.dec.push(entry);
        }
        if let Some(frame) = self.frames.last_mut() {
            frame.loop_depth += 1;
        }
        self.breaks.push(BreakFrame {
            is_loop: true,
            breaks: Vec::new(),
            continues: Vec::new(),
        });
        self.exec_arm(&body.stmts);
        let frame = self.breaks.pop();
        if let Some(frame) = self.frames.last_mut() {
            frame.loop_depth = frame.loop_depth.saturating_sub(1);
        }
        if pushed {
            self.dec.pop();
        }
        self.pop_scope();
        let (breaks, continues) =
            frame.map_or((Vec::new(), Vec::new()), |f| (f.breaks, f.continues));
        let mut end = self.env.take();
        for state in continues {
            end = join_opt(end, Some(state));
        }
        if let LoopHeader::For {
            post: Some(post), ..
        } = header
        {
            self.env = end;
            if self.env.is_some() {
                self.exec_stmt(post);
            }
            end = self.env.take();
        }
        let next_head = join_opt(Some(head), end).unwrap_or_default();
        PassOut {
            next_head,
            exit,
            breaks,
        }
    }

    fn bind_range(&mut self, xval: Val, key: Option<&Expr>, value: Option<&Expr>, define: bool) {
        let key_val = if xval.derived() { Val::UNK } else { Val::NO };
        let elem_val = if xval.comp {
            Val {
                d: D::Unk,
                comp: false,
                deleg: xval.deleg,
            }
        } else {
            xval
        };
        for (x, val) in [(key, key_val), (value, elem_val)] {
            let Some(x) = x else { continue };
            if define {
                if let ExprKind::Ident(name) = &x.kind {
                    self.declare(
                        &Ident {
                            name: name.clone(),
                            span: x.span,
                        },
                        false,
                        val,
                    );
                }
            } else {
                self.assign_place(x, val);
            }
        }
        if xval.derived() {
            self.uncertain();
        }
    }

    // -- assignment ----------------------------------------------------

    /// DS-1976: a derived `x` assigned to another variable or passed on is no
    /// longer known afterwards.
    fn alias_source(&mut self, x: &Expr) {
        if let ExprKind::Ident(name) = &strip(x).kind {
            if let Some(id) = self.lookup(name) {
                let val = self.get(id);
                if val.derived() {
                    self.set(
                        id,
                        Val {
                            d: D::Unk,
                            comp: false,
                            deleg: val.deleg,
                        },
                    );
                    self.uncertain();
                }
            }
        }
    }

    /// DS-1976: every local variable the expression refers to that holds a
    /// derived value is unknown afterwards (the call may keep or change it).
    fn escape(&mut self, x: &Expr) {
        let mut names = BTreeSet::new();
        for_each_expr(x, false, &mut |e| {
            if let ExprKind::Ident(name) = &e.kind {
                names.insert(name.clone());
            }
        });
        for name in names {
            if let Some(id) = self.lookup(&name) {
                let val = self.get(id);
                if val.derived() {
                    self.set(
                        id,
                        Val {
                            d: D::Unk,
                            comp: false,
                            deleg: val.deleg,
                        },
                    );
                    self.uncertain();
                }
            }
        }
    }

    fn exec_assign(&mut self, lhs: &[Expr], op: &str, rhs: &[Expr]) {
        let vals: Vec<Val> = rhs.iter().map(|r| self.eval(r, PLAIN)).collect();
        let single = rhs.len() == 1 && lhs.len() > 1;
        let val_for = |i: usize| -> Val {
            if single {
                vals[0]
            } else {
                vals.get(i).copied().unwrap_or(Val::NO)
            }
        };
        match op {
            ":=" => {
                for (i, l) in lhs.iter().enumerate() {
                    let ExprKind::Ident(name) = &l.kind else {
                        continue;
                    };
                    let val = val_for(i);
                    let existing = self
                        .scopes
                        .last()
                        .and_then(|scope| scope.get(name).copied());
                    match existing {
                        Some(id) if name != "_" => self.set(id, val),
                        _ => {
                            let t_typed = !single && rhs.get(i).is_some_and(|r| self.is_t_ident(r));
                            self.declare(
                                &Ident {
                                    name: name.clone(),
                                    span: l.span,
                                },
                                t_typed,
                                val,
                            );
                        }
                    }
                }
            }
            "=" => {
                for (i, l) in lhs.iter().enumerate() {
                    let val = val_for(i);
                    self.assign_place(l, val);
                }
            }
            _ => {
                for (i, l) in lhs.iter().enumerate() {
                    let val = val_for(i);
                    self.compound_assign(l, val);
                }
            }
        }
        let blank = lhs
            .iter()
            .all(|l| matches!(&strip(l).kind, ExprKind::Ident(name) if name == "_"));
        if !blank {
            for r in rhs {
                self.alias_source(r);
            }
        }
    }

    /// `x = v` where `x` is a place (DS-1975).
    fn assign_place(&mut self, place: &Expr, val: Val) {
        let place = strip(place);
        match &place.kind {
            ExprKind::Ident(name) if name == "_" => {}
            ExprKind::Ident(name) => match self.lookup(name) {
                Some(id) => self.set(id, val),
                None => {
                    if val.derived() {
                        // A package-level variable: read elsewhere than here.
                        self.uncertain();
                    }
                }
            },
            _ => {
                self.eval(place, PLAIN);
                if matches!(place.kind, ExprKind::Star(_)) && val.derived() {
                    self.uncertain();
                }
                match root_ident(place).and_then(|name| self.lookup(name)) {
                    Some(id) => {
                        let current = self.get(id);
                        if current.derived() || val.derived() {
                            self.set(
                                id,
                                Val {
                                    d: D::Unk,
                                    comp: false,
                                    deleg: current.deleg || val.deleg,
                                },
                            );
                            self.uncertain();
                        }
                    }
                    None => {
                        if val.derived() {
                            self.uncertain();
                        }
                    }
                }
            }
        }
    }

    /// `x op= v` (DS-1975).
    fn compound_assign(&mut self, place: &Expr, val: Val) {
        let place = strip(place);
        match &place.kind {
            ExprKind::Ident(name) if name != "_" => match self.lookup(name) {
                Some(id) => {
                    let current = self.get(id);
                    if current.comp || val.comp {
                        self.set(id, Val::UNK);
                        self.uncertain();
                    } else {
                        self.set(id, current.merge(val));
                    }
                }
                None => {
                    if val.derived() {
                        self.uncertain();
                    }
                }
            },
            _ => self.assign_place(place, val),
        }
    }

    // -- expressions ---------------------------------------------------

    /// A composite value that is read as a part of it, or converted, is
    /// unknown (DS-1984).
    fn uncomp(&mut self, val: Val) -> Val {
        if val.comp {
            self.uncertain();
            Val {
                d: D::Unk,
                comp: false,
                deleg: val.deleg,
            }
        } else {
            val
        }
    }

    fn eval(&mut self, x: &Expr, c: Ctx) -> Val {
        match &x.kind {
            ExprKind::BasicLit { .. }
            | ExprKind::ArrayType { .. }
            | ExprKind::Ellipsis(_)
            | ExprKind::StructType
            | ExprKind::FuncType(_)
            | ExprKind::InterfaceType
            | ExprKind::MapType(..)
            | ExprKind::ChanType(_) => Val::NO,
            ExprKind::Ident(name) => self.eval_ident(name),
            ExprKind::Paren(inner) => self.eval(inner, c),
            ExprKind::CompositeLit { elts, .. } => self.eval_composite(elts),
            ExprKind::FuncLit(lit) => {
                self.eval_func_lit(&lit.sig, &lit.body);
                Val::NO
            }
            ExprKind::Selector(base, sel) => self.eval_selector(base, &sel.name),
            ExprKind::Index(base, idx) => {
                let vb = self.eval(base, PLAIN);
                let mut vi = Val::NO;
                for i in idx {
                    let v = self.eval(i, PLAIN);
                    vi = vi.merge(v);
                }
                self.read_part(vb, vi)
            }
            ExprKind::Slice(base, parts) => {
                let vb = self.eval(base, PLAIN);
                let mut vi = Val::NO;
                for part in parts.iter().flatten() {
                    let v = self.eval(part, PLAIN);
                    vi = vi.merge(v);
                }
                self.read_part(vb, vi)
            }
            ExprKind::TypeAssert(base, _) => {
                let v = self.eval(base, PLAIN);
                self.uncomp(v)
            }
            ExprKind::Call {
                fun,
                args,
                ellipsis,
                ..
            } => self.eval_call(fun, args, *ellipsis, c),
            ExprKind::Star(inner) => {
                let v = self.eval(inner, PLAIN);
                self.uncomp(v)
            }
            ExprKind::Unary(op, inner) => self.eval_unary(op, inner, c),
            ExprKind::Binary(op, a, b) => self.eval_binary(op, a, b, c),
            ExprKind::KeyValue(k, v) => {
                let (vk, vv) = (self.eval(k, PLAIN), self.eval(v, PLAIN));
                vk.merge(vv)
            }
        }
    }

    fn eval_ident(&mut self, name: &str) -> Val {
        if let Some(id) = self.lookup(name) {
            return self.get(id);
        }
        if let Some(target) = self.target {
            if !target.is_method && name == target.symbol {
                // The target used as a value: it may be called through it.
                self.uncertain();
            }
        }
        Val::NO
    }

    fn eval_selector(&mut self, base: &Expr, sel: &str) -> Val {
        if let Some(target) = self.target {
            if sel == target.symbol {
                self.uncertain();
            }
        }
        if let ExprKind::Ident(root) = &strip(base).kind {
            if self.lookup(root).is_none() {
                return Val::NO;
            }
        }
        let v = self.eval(base, PLAIN);
        self.uncomp(v)
    }

    /// The value read from `base` at an index or slice with `index` values.
    fn read_part(&mut self, base: Val, index: Val) -> Val {
        let part = self.uncomp(base);
        if part.derived() {
            Val {
                deleg: part.deleg || index.deleg,
                ..part
            }
        } else if index.derived() {
            self.uncertain();
            Val {
                d: D::Unk,
                comp: false,
                deleg: part.deleg || index.deleg,
            }
        } else {
            Val {
                d: D::No,
                comp: false,
                deleg: part.deleg || index.deleg,
            }
        }
    }

    fn eval_composite(&mut self, elts: &[Expr]) -> Val {
        let mut any_yes = false;
        let mut any_unk = false;
        let mut deleg = false;
        for elt in elts {
            let v = match &elt.kind {
                ExprKind::KeyValue(k, v) => {
                    let (vk, vv) = (self.eval(k, PLAIN), self.eval(v, PLAIN));
                    // A composite element keeps its own composite-ness.
                    if vv.comp {
                        vv
                    } else {
                        vk.merge(vv)
                    }
                }
                _ => self.eval(elt, PLAIN),
            };
            deleg |= v.deleg;
            if v.comp || v.d == D::Yes {
                any_yes = true;
            } else if v.d == D::Unk {
                any_unk = true;
            }
        }
        if any_yes {
            Val {
                d: D::Yes,
                comp: true,
                deleg,
            }
        } else if any_unk {
            Val {
                d: D::Unk,
                comp: false,
                deleg,
            }
        } else {
            Val { deleg, ..Val::NO }
        }
    }

    fn eval_unary(&mut self, op: &str, inner: &Expr, c: Ctx) -> Val {
        match op {
            "&" => {
                let target = strip(inner);
                if matches!(target.kind, ExprKind::CompositeLit { .. }) {
                    return self.eval(target, PLAIN);
                }
                let v = self.eval(target, PLAIN);
                if v.derived() {
                    // A pointer to a derived value: it may be changed through it.
                    self.escape(target);
                    self.uncertain();
                    return Val {
                        d: D::Unk,
                        comp: false,
                        deleg: v.deleg,
                    };
                }
                Val {
                    deleg: v.deleg,
                    ..Val::NO
                }
            }
            "!" => self.eval(inner, c),
            _ => {
                let v = self.eval(inner, PLAIN);
                self.uncomp(v)
            }
        }
    }

    fn eval_binary(&mut self, op: &str, a: &Expr, b: &Expr, c: Ctx) -> Val {
        if let Some(absorbing) = absorbing_literal(op) {
            // `X || true`, `false && X`: X is not a position the value flows
            // from (DS-1948, DS-1996).
            let (va, vb) = (self.eval(a, c), self.eval(b, c));
            if is_bool_literal(a, absorbing, self) || is_bool_literal(b, absorbing, self) {
                return Val::NO;
            }
            let (ca, cb) = (classify(a, self), classify(b, self));
            let other = if ca == Constness::Const && cb != Constness::Const {
                Some(vb)
            } else if cb == Constness::Const && ca != Constness::Const {
                Some(va)
            } else {
                None
            };
            if let Some(other) = other {
                let constant = if ca == Constness::Const { a } else { b };
                let plain_bool =
                    is_bool_literal(constant, true, self) || is_bool_literal(constant, false, self);
                if !plain_bool {
                    // A constant that is not the literal may absorb (DS-621).
                    self.uncertain();
                    return if other.derived() {
                        Val {
                            d: D::Unk,
                            comp: false,
                            deleg: other.deleg,
                        }
                    } else {
                        Val {
                            deleg: other.deleg,
                            ..Val::NO
                        }
                    };
                }
            }
            return va.merge(vb);
        }
        let (va, vb) = (self.eval(a, PLAIN), self.eval(b, PLAIN));
        if va.comp || vb.comp {
            if matches!(op, "==" | "!=") && c.decider {
                // DS-1984: a composite value compared whole.
                return Val {
                    comp: false,
                    ..Val {
                        d: D::Yes,
                        comp: false,
                        deleg: va.deleg || vb.deleg,
                    }
                };
            }
            self.uncertain();
            return Val {
                d: D::Unk,
                comp: false,
                deleg: va.deleg || vb.deleg,
            };
        }
        va.merge(vb)
    }

    // -- calls ---------------------------------------------------------

    fn classify_call<'e>(&mut self, fun: &'e Expr) -> Callee<'e> {
        let callee = strip(fun);
        // `f[int](x)`: the instance is a call of `f`.
        let callee = match &callee.kind {
            ExprKind::Index(base, _)
                if matches!(
                    &strip(base).kind,
                    ExprKind::Ident(_) | ExprKind::Selector(..)
                ) =>
            {
                strip(base)
            }
            _ => callee,
        };
        if let Some(spelled) = dotted_name(callee) {
            if self.cfg.macros.contains(&spelled) {
                return Callee::Macro;
            }
        }
        match &callee.kind {
            ExprKind::Ident(name) => self.classify_ident_callee(name),
            ExprKind::Selector(base, sel) => self.classify_selector_callee(base, &sel.name),
            _ => Callee::Function,
        }
    }

    fn other_target_bare(&self, name: &str) -> bool {
        self.cfg.all_targets.iter().any(|t| {
            !t.is_method
                && t.symbol == name
                && self.cfg.file.bare_call_matches(t) == TargetMatch::Definite
                && Some(t) != self.target
        })
    }

    fn classify_ident_callee<'e>(&mut self, name: &str) -> Callee<'e> {
        if self.lookup(name).is_some() {
            return Callee::Function;
        }
        match name {
            "panic" => return Callee::Panic,
            "recover" => return Callee::Recover,
            "len" | "cap" => return Callee::LenCap,
            _ => {}
        }
        if BUILTIN_TYPES.contains(&name) {
            return Callee::Conversion;
        }
        if let Some(builtin) = OTHER_BUILTINS.iter().find(|b| **b == name) {
            return Callee::Builtin(builtin);
        }
        if let Some(target) = self.target {
            if !target.is_method && name == target.symbol {
                match self.cfg.file.bare_call_matches(target) {
                    TargetMatch::No => {}
                    other => return Callee::Target(other),
                }
            }
        }
        if self.other_target_bare(name) {
            return Callee::OtherTarget;
        }
        Callee::Function
    }

    fn classify_selector_callee<'e>(&mut self, base: &'e Expr, method: &str) -> Callee<'e> {
        let root = match &strip(base).kind {
            ExprKind::Ident(root) => Some(root.as_str()),
            _ => None,
        };
        if let Some(root) = root {
            if let Some(id) = self.lookup(root) {
                if self.vars[id].t_typed {
                    return if FAILURE_METHODS.contains(&method) {
                        Callee::Failure {
                            terminal: TERMINATING_FAILURES.contains(&method),
                        }
                    } else if SKIP_METHODS.contains(&method) {
                        Callee::Skip
                    } else if method == "Run" {
                        Callee::Run
                    } else {
                        Callee::TMethod
                    };
                }
                return self.value_method(base, method);
            }
            // A package-qualified function, or a method of a package-level value.
            let spelled = format!("{root}.{method}");
            if COMPARISONS.contains(&spelled.as_str()) {
                return Callee::Compare;
            }
            if spelled == "os.Exit" || spelled == "runtime.Goexit" {
                return Callee::Exit;
            }
            if let Some(target) = self.target {
                if method == target.symbol {
                    if target.is_method {
                        return Callee::Target(TargetMatch::Ambiguous);
                    }
                    match self.cfg.file.qualified_call_matches(root, target) {
                        TargetMatch::No => {}
                        other => return Callee::Target(other),
                    }
                }
            }
            let other_qualified = self.cfg.all_targets.iter().any(|t| {
                !t.is_method
                    && t.symbol == method
                    && Some(t) != self.target
                    && self.cfg.file.qualified_call_matches(root, t) == TargetMatch::Definite
            });
            if other_qualified {
                return Callee::OtherTarget;
            }
            return Callee::Function;
        }
        self.value_method(base, method)
    }

    /// A method called on a value (a variable that is not a `testing.T`, or
    /// any other expression).
    fn value_method<'e>(&mut self, base: &'e Expr, method: &str) -> Callee<'e> {
        if FAILURE_METHODS.contains(&method) && self.recording {
            self.structure.maybe_asserts += 1;
        }
        if let Some(target) = self.target {
            if target.is_method && method == target.symbol {
                return Callee::Target(TargetMatch::Definite);
            }
        }
        let other_method = self
            .cfg
            .all_targets
            .iter()
            .any(|t| t.is_method && t.symbol == method && Some(t) != self.target);
        if other_method {
            return Callee::OtherTarget;
        }
        Callee::Method(base)
    }

    fn eval_args(&mut self, args: &[Expr]) -> Vec<Val> {
        args.iter().map(|a| self.eval(a, PLAIN)).collect()
    }

    /// The values `args` evaluate to, and whether any is derived; a derived
    /// argument handed to a call is no longer known (DS-1976).
    fn eval_call_args(&mut self, args: &[Expr]) -> (Vec<Val>, bool) {
        let vals = self.eval_args(args);
        let any = vals.iter().any(|v| v.derived());
        if any {
            for arg in args {
                self.escape(arg);
            }
            self.uncertain();
        }
        (vals, any)
    }

    fn candidate(&mut self) {
        if self.recording && self.target.is_none() {
            self.structure.candidates += 1;
        }
    }

    fn eval_call(&mut self, fun: &Expr, args: &[Expr], _ellipsis: bool, c: Ctx) -> Val {
        let callee = self.classify_call(fun);
        let mut term = Term::None;
        let val = match callee {
            Callee::Panic => {
                self.eval_call_args(args);
                self.record_site(SiteKind::Panic);
                term = Term::Panic;
                Val::NO
            }
            Callee::Recover => Val::NO,
            Callee::LenCap | Callee::Conversion => {
                let vals = self.eval_args(args);
                let first = vals.first().copied().unwrap_or(Val::NO);
                let rest = vals.iter().skip(1).fold(Val::NO, |m, v| m.merge(*v));
                let first = self.uncomp(first);
                Val {
                    deleg: first.deleg || rest.deleg,
                    ..first
                }
            }
            Callee::Builtin(name) => self.eval_builtin(name, args),
            Callee::Compare => {
                let vals = self.eval_args(args);
                let mut merged = Val::NO;
                for v in vals {
                    let v = if v.comp && c.decider {
                        Val { comp: false, ..v }
                    } else {
                        self.uncomp(v)
                    };
                    merged = merged.merge(v);
                }
                merged
            }
            Callee::Failure { terminal } => {
                self.eval_call_args(args);
                self.record_site(SiteKind::Failure);
                term = if terminal { Term::Fatal } else { Term::Failure };
                Val::NO
            }
            Callee::Skip => {
                self.eval_call_args(args);
                term = Term::Skip;
                Val::NO
            }
            Callee::Run => {
                if let Some(first) = args.first() {
                    self.eval(first, PLAIN);
                }
                match args.get(1).map(|a| &a.kind) {
                    Some(ExprKind::FuncLit(lit)) if args.len() == 2 => {
                        self.run_continuation(&lit.sig, &lit.body);
                    }
                    _ => {
                        // A function value is run as the subtest: a delegation.
                        self.eval_call_args(args.get(1..).unwrap_or(&[]));
                        self.candidate();
                    }
                }
                Val::NO
            }
            Callee::TMethod => {
                self.eval_call_args(args);
                Val {
                    deleg: true,
                    ..Val::NO
                }
            }
            Callee::Exit => {
                self.eval_call_args(args);
                term = Term::Exit;
                Val::NO
            }
            Callee::Macro => self.eval_macro(args),
            Callee::Target(kind) => self.eval_target_call(fun, args, kind),
            Callee::OtherTarget => {
                let (_, any) = self.eval_call_args(args);
                if any {
                    Val::UNK
                } else {
                    Val::NO
                }
            }
            Callee::Method(base) => self.eval_method_call(base, args),
            Callee::Function => {
                // The function value may itself hold a derived value (a method
                // value bound to the result): it is an input of the call.
                let callee_val = self.eval(fun, PLAIN);
                if callee_val.derived() {
                    self.escape(fun);
                    self.uncertain();
                }
                let (_, any) = self.eval_call_args(args);
                self.candidate();
                Val {
                    d: if any || callee_val.derived() {
                        D::Unk
                    } else {
                        D::No
                    },
                    comp: false,
                    deleg: true,
                }
            }
        };
        self.last_term = term;
        val
    }

    fn eval_builtin(&mut self, name: &str, args: &[Expr]) -> Val {
        match name {
            "append" => {
                let vals = self.eval_args(args);
                let mut any_yes = false;
                let mut any_unk = false;
                let mut deleg = false;
                for v in &vals {
                    deleg |= v.deleg;
                    if v.comp || v.d == D::Yes {
                        any_yes = true;
                    } else if v.d == D::Unk {
                        any_unk = true;
                    }
                }
                if any_yes {
                    // The derived element now sits in the slice.
                    for arg in args.iter().skip(1) {
                        self.alias_source(arg);
                    }
                    Val {
                        d: D::Yes,
                        comp: true,
                        deleg,
                    }
                } else if any_unk {
                    Val { deleg, ..Val::UNK }
                } else {
                    Val { deleg, ..Val::NO }
                }
            }
            "min" | "max" => {
                let vals = self.eval_args(args);
                let mut merged = Val::NO;
                for v in vals {
                    let v = self.uncomp(v);
                    merged = merged.merge(v);
                }
                merged
            }
            _ => {
                self.eval_call_args(args);
                Val::NO
            }
        }
    }

    /// DS-1886, DS-1888: a call listed in `assertion_macros`; the values of its
    /// arguments other than a `testing.T` are what it checks.
    fn eval_macro(&mut self, args: &[Expr]) -> Val {
        let mut merged = Val::NO;
        let mut constness = Constness::Const;
        for arg in args {
            if self.is_t_ident(arg) {
                continue;
            }
            let v = self.eval(arg, PLAIN);
            // DS-1984: a composite value reaches a macro only as unknown.
            let v = self.uncomp(v);
            merged = merged.merge(v);
            constness = match (constness, classify(arg, self)) {
                (Constness::NonConst, _) | (_, Constness::NonConst) => Constness::NonConst,
                (Constness::Undecided, _) | (_, Constness::Undecided) => Constness::Undecided,
                (Constness::Const, Constness::Const) => Constness::Const,
            };
        }
        self.record_site(SiteKind::Macro {
            val: merged,
            constness,
        });
        Val::NO
    }

    fn eval_target_call(&mut self, fun: &Expr, args: &[Expr], kind: TargetMatch) -> Val {
        // A method target is called on a receiver: it is read, and changed
        // afterwards (DS-1976).
        if let ExprKind::Selector(base, _) = &strip(fun).kind {
            let is_value = match &strip(base).kind {
                ExprKind::Ident(root) => self.lookup(root).is_some(),
                _ => true,
            };
            if is_value {
                let rv = self.eval(base, PLAIN);
                if rv.derived() {
                    self.escape(base);
                    self.uncertain();
                }
            }
        }
        self.eval_call_args(args);
        match kind {
            TargetMatch::Definite => {
                if self.recording {
                    if self.counted && !self.dead {
                        self.facts.counted_calls += 1;
                    } else {
                        self.facts.uncounted_calls += 1;
                    }
                }
                Val::YES
            }
            TargetMatch::Ambiguous | TargetMatch::No => {
                if self.recording {
                    self.facts.ambiguous_calls += 1;
                }
                self.uncertain();
                Val::UNK
            }
        }
    }

    /// DS-1968: the value of the receiver of a method flows into the result,
    /// DS-1936: a derived argument is handed to a delegation target.
    fn eval_method_call(&mut self, base: &Expr, args: &[Expr]) -> Val {
        let rv = self.eval(base, PLAIN);
        let (_, any_arg) = self.eval_call_args(args);
        self.candidate();
        let result = if any_arg {
            Val {
                d: D::Unk,
                comp: false,
                deleg: true,
            }
        } else if rv.derived() {
            let part = self.uncomp(rv);
            Val {
                deleg: part.deleg,
                ..part
            }
        } else {
            Val {
                d: D::No,
                comp: false,
                deleg: true,
            }
        };
        if rv.derived() {
            // The method may change what it is called on (DS-1976).
            self.escape(base);
            self.uncertain();
        }
        result
    }

    // -- assert-equivalent calls ---------------------------------------

    fn record_site(&mut self, kind: SiteKind) {
        if !self.recording {
            return;
        }
        let live = self.counted && !self.dead;
        let Some(frame) = self.frames.last() else {
            return;
        };
        let neutral = matches!(kind, SiteKind::Panic) && frame.recover_defer;
        let never_run =
            self.dec.iter().any(|e| e.never_run) || frame.extras.iter().any(|e| e.entry.never_run);
        let (merged, constness, self_cmp, undecided_flag) = match &kind {
            SiteKind::Failure | SiteKind::Panic => {
                let entries: Vec<&DEntry> = self
                    .dec
                    .iter()
                    .chain(frame.extras.iter().map(|e| &e.entry))
                    .collect();
                let undecided = frame.undecided_after || entries.iter().any(|e| e.undecided);
                let constness = if self.dead || undecided {
                    Constness::Undecided
                } else if entries.iter().any(|e| e.constness == Constness::NonConst) {
                    Constness::NonConst
                } else if entries.iter().any(|e| e.constness == Constness::Undecided) {
                    Constness::Undecided
                } else {
                    Constness::Const
                };
                let merged = entries.iter().fold(Val::NO, |m, e| m.merge(e.val));
                let self_cmp = self.dec.iter().any(|e| e.self_cmp);
                (merged, constness, self_cmp, undecided)
            }
            SiteKind::Macro { val, constness } => {
                let constness = if self.dead {
                    Constness::Undecided
                } else {
                    *constness
                };
                (*val, constness, false, false)
            }
        };
        if live && self.target.is_some() {
            if merged.d == D::Yes {
                if merged.deleg {
                    self.facts.reach_deleg = true;
                } else if undecided_flag || never_run || neutral {
                    self.facts.reach_unk = true;
                } else {
                    self.facts.reach_clean = true;
                }
            } else if merged.d == D::Unk {
                self.facts.reach_unk = true;
            }
        }
        self.structure.sites.push(Site {
            neutralized: neutral,
            constness,
            self_cmp: self_cmp && !neutral,
            undecided: undecided_flag || self.dead || never_run,
        });
    }
}

/// Reads `decl` for the declared target `target` (or for none).
pub(crate) fn run(
    cfg: &Config<'_>,
    decl: &FuncDecl,
    target: Option<&TargetSpec>,
) -> (Structure, Facts) {
    let mut interp = Interp::new(cfg, target);
    match &decl.body {
        Some(body) => {
            interp.structure.empty_body =
                body.stmts.iter().all(|s| matches!(s.kind, StmtKind::Empty));
            interp.exec_func_body(&decl.sig, body, true);
        }
        None => interp.structure.empty_body = true,
    }
    (interp.structure, interp.facts)
}
