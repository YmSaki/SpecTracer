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
//! The state and the scopes are here, the statements are executed in `exec` and
//! the expressions and calls are read in `eval`.
//!
//! Boundary: no verdict is made here (`static_audit` composes the rules from
//! the facts), and nothing outside the function is read: a function that is
//! not the target is a delegation target (DS-641, DS-1936) and the audit
//! never confirms that one terminates (DS-643), so a derived value that goes
//! through one only ever makes a rule `UNKNOWN`. Where the walk meets a form
//! it does not follow, it records that (`taint`, `uncertain`) and the rules
//! answer `UNKNOWN`; it never reads a form as safe (DS-615, DS-1680).

mod eval;
mod exec;

use std::cell::RefCell;
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
/// The methods of a `testing.T` that take no function to run and decide nothing:
/// a call of another one (`Cleanup`, `Run` with a function value) may run a
/// check (DS-642).
const HARMLESS_T_METHODS: &[&str] = &[
    "Chdir", "Context", "Deadline", "Failed", "Helper", "Log", "Logf", "Name", "Parallel",
    "Setenv", "Skipped", "TempDir",
];
/// DS-1986: the comparison functions, by their spelling.
const COMPARISONS: &[&str] = &["reflect.DeepEqual", "errors.Is", "bytes.Equal"];
/// The statements a walk reads before it gives up: nested loops are iterated to
/// a fixed point, so the work can grow with their depth, and a walk that does
/// not finish is `UNKNOWN`, never a verdict from what it had read.
const WALK_BUDGET: usize = 2_000_000;
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
    /// The position reads something the Test does not hold in a local variable
    /// (a call, a package-level name), through which an effect of the target
    /// may be observed (DS-623).
    ext: bool,
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
    /// `os.Exit`.
    Exit,
    /// `runtime.Goexit`: ends the goroutine, which is the subtest of a `t.Run`.
    Goexit,
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
    /// The walk stopped at its budget, so nothing it found is complete.
    pub(crate) exhausted: bool,
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
    /// A call of the target takes an argument that is not a literal, or has a
    /// receiver: its effect may be on that argument or receiver, which a later
    /// check reads (DS-623).
    pub(crate) out_channel: bool,
    /// A deciding position of an assert-equivalent reads a call or a package-level
    /// name, through which the target's effect on state may be observed (DS-623).
    pub(crate) ext_observed: bool,
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
    /// Another method of a `testing.T`; `harmless` for the ones that take no
    /// function to run.
    TMethod {
        harmless: bool,
    },
    Exit,
    Goexit,
    Macro,
    Target(TargetMatch),
    OtherTarget,
    Method(&'e Expr),
    Function,
}

enum SiteKind {
    Failure,
    Panic,
    Macro {
        val: Val,
        constness: Constness,
        ext: bool,
    },
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
    /// The variables whose address was taken, or that a method value or a
    /// slice was made from: what is assigned to them may be changed through
    /// another name (DS-1976).
    addr_taken: BTreeSet<usize>,
    /// Whether a condition holds a `==` / `!=` of one value with itself, by the
    /// start of the condition.
    self_cmp_cache: RefCell<HashMap<usize, bool>>,
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
    /// The statements the walk may still read.
    fuel: usize,
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

/// DS-1933 does not say when a subtest that calls `Parallel` runs its body: it
/// is paused until the parent returns, so the body is not read as the rest of
/// the Test.
fn body_calls_parallel(stmts: &[Stmt]) -> bool {
    let mut found = false;
    for_each_expr_in_stmts(stmts, false, &mut |e| {
        if let ExprKind::Call { fun, .. } = &e.kind {
            if matches!(&strip(fun).kind, ExprKind::Selector(_, sel) if sel.name == "Parallel") {
                found = true;
            }
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
            addr_taken: BTreeSet::new(),
            self_cmp_cache: RefCell::new(HashMap::new()),
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
            fuel: WALK_BUDGET,
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
        let mut val = val;
        if val.derived() {
            let captured = self.captured.contains(&self.vars[id].name);
            if captured || self.addr_taken.contains(&id) {
                // A part that DS-1970 does not count reads this name, or another
                // name may change what it holds: it is no longer known (DS-1976).
                if self.recording {
                    self.facts.uncertain = true;
                    self.facts.uncounted_flow |= captured;
                }
                val = Val {
                    d: D::Unk,
                    comp: false,
                    deleg: val.deleg,
                };
            }
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
        for field in &sig.results {
            for name in &field.names {
                self.declare(name, false, Val::NO);
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
