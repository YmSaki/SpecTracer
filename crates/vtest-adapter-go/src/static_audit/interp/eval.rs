//! The expressions of the walk: the value an expression has, calls and the
//! assert-equivalent calls they may be (DS-1884 to DS-1888, DS-1936, DS-1968,
//! DS-1970, DS-1984, DS-1986).
//!
//! Responsibility: read what a value derived from the target's result becomes
//! through an expression, tell what a callee is, and record an assert-equivalent
//! call with what decides it. Boundary: statements are executed by `exec`. Realizes
//! DES-619.

use super::*;

impl Interp<'_> {
    // -- expressions ---------------------------------------------------

    /// A composite value that is read as a part of it, or converted, is
    /// unknown (DS-1984).
    pub(super) fn uncomp(&mut self, val: Val) -> Val {
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

    pub(super) fn eval(&mut self, x: &Expr, c: Ctx) -> Val {
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

    pub(super) fn eval_ident(&mut self, name: &str) -> Val {
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

    pub(super) fn eval_selector(&mut self, base: &Expr, sel: &str) -> Val {
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
    pub(super) fn read_part(&mut self, base: Val, index: Val) -> Val {
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

    pub(super) fn eval_composite(&mut self, elts: &[Expr]) -> Val {
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

    pub(super) fn eval_unary(&mut self, op: &str, inner: &Expr, c: Ctx) -> Val {
        match op {
            "&" => {
                let target = strip(inner);
                if matches!(target.kind, ExprKind::CompositeLit { .. }) {
                    let v = self.eval(target, PLAIN);
                    // A pointer to a composite value is a new pointer: comparing
                    // it checks none of its elements.
                    return self.uncomp(v);
                }
                let v = self.eval(target, PLAIN);
                if let Some(id) = root_ident(target).and_then(|name| self.lookup(name)) {
                    // What is assigned to the variable later may be changed
                    // through the pointer.
                    self.addr_taken.insert(id);
                }
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

    pub(super) fn eval_binary(&mut self, op: &str, a: &Expr, b: &Expr, c: Ctx) -> Val {
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
            // A composite literal is never `nil`: a comparison with it checks
            // nothing of its elements.
            let mentions_nil = |x: &Expr| {
                let mut found = false;
                for_each_expr(x, true, &mut |e| {
                    found |= matches!(&e.kind, ExprKind::Ident(name) if name == "nil");
                });
                found && !self.is_local("nil")
            };
            if matches!(op, "==" | "!=") && c.decider && !mentions_nil(a) && !mentions_nil(b) {
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

    pub(super) fn classify_call<'e>(&mut self, fun: &'e Expr) -> Callee<'e> {
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

    pub(super) fn other_target_bare(&self, name: &str) -> bool {
        self.cfg.all_targets.iter().any(|t| {
            !t.is_method
                && t.symbol == name
                && self.cfg.file.bare_call_matches(t) == TargetMatch::Definite
                && Some(t) != self.target
        })
    }

    pub(super) fn classify_ident_callee<'e>(&mut self, name: &str) -> Callee<'e> {
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

    pub(super) fn classify_selector_callee<'e>(
        &mut self,
        base: &'e Expr,
        method: &str,
    ) -> Callee<'e> {
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
                        Callee::TMethod {
                            harmless: HARMLESS_T_METHODS.contains(&method),
                        }
                    };
                }
                return self.value_method(base, method);
            }
            // A package-qualified function, or a method of a package-level value.
            let spelled = format!("{root}.{method}");
            if COMPARISONS.contains(&spelled.as_str()) {
                return Callee::Compare;
            }
            if spelled == "os.Exit" {
                return Callee::Exit;
            }
            if spelled == "runtime.Goexit" {
                return Callee::Goexit;
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
    pub(super) fn value_method<'e>(&mut self, base: &'e Expr, method: &str) -> Callee<'e> {
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

    pub(super) fn eval_args(&mut self, args: &[Expr]) -> Vec<Val> {
        args.iter().map(|a| self.eval(a, PLAIN)).collect()
    }

    /// The values `args` evaluate to, and whether any is derived; a derived
    /// argument handed to a call is no longer known (DS-1976).
    pub(super) fn eval_call_args(&mut self, args: &[Expr]) -> (Vec<Val>, bool) {
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

    pub(super) fn candidate(&mut self) {
        if self.recording && self.target.is_none() {
            self.structure.candidates += 1;
        }
    }

    pub(super) fn eval_call(&mut self, fun: &Expr, args: &[Expr], _ellipsis: bool, c: Ctx) -> Val {
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
                        let t_names: BTreeSet<&str> = lit
                            .sig
                            .params
                            .iter()
                            .filter(|field| self.type_is_t(&field.ty))
                            .flat_map(|field| field.names.iter().map(|n| n.name.as_str()))
                            .collect();
                        if body_may_pause(&lit.body.stmts, &t_names) {
                            // Its body may run after the parent returns.
                            self.eval_func_lit(&lit.sig, &lit.body);
                        } else {
                            self.run_continuation(&lit.sig, &lit.body);
                        }
                    }
                    _ => {
                        // A function value is run as the subtest: a delegation.
                        self.eval_call_args(args.get(1..).unwrap_or(&[]));
                        self.candidate();
                    }
                }
                Val::NO
            }
            Callee::TMethod { harmless } => {
                self.eval_call_args(args);
                if !harmless {
                    self.candidate();
                }
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
            Callee::Goexit => {
                self.eval_call_args(args);
                term = Term::Goexit;
                Val::NO
            }
            Callee::Macro => self.eval_macro(args),
            Callee::Target(kind) => self.eval_target_call(fun, args, kind),
            Callee::OtherTarget => {
                // Another declared target: Go has no exception for it among the
                // delegation targets (DS-1986), so a value that meets its call
                // in one deciding position is not clean.
                let (_, any) = self.eval_call_args(args);
                Val {
                    d: if any { D::Unk } else { D::No },
                    comp: false,
                    deleg: true,
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

    pub(super) fn eval_builtin(&mut self, name: &str, args: &[Expr]) -> Val {
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
    pub(super) fn eval_macro(&mut self, args: &[Expr]) -> Val {
        let mut merged = Val::NO;
        let mut constness = Constness::Const;
        let mut ext = false;
        for arg in args {
            if self.is_t_ident(arg) {
                continue;
            }
            ext |= self.reads_external(arg);
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
            ext,
        });
        Val::NO
    }

    pub(super) fn eval_target_call(&mut self, fun: &Expr, args: &[Expr], kind: TargetMatch) -> Val {
        // A method target is called on a receiver: it is read, and changed
        // afterwards (DS-1976).
        if let ExprKind::Selector(base, _) = &strip(fun).kind {
            let is_value = match &strip(base).kind {
                ExprKind::Ident(root) => self.lookup(root).is_some(),
                _ => true,
            };
            if is_value {
                if self.recording {
                    // What the method changes is on its receiver.
                    self.facts.out_channel = true;
                }
                let rv = self.eval(base, PLAIN);
                if rv.derived() {
                    self.escape(base);
                    self.uncertain();
                }
            }
        }
        self.eval_call_args(args);
        if self.recording && args.iter().any(|a| classify(a, self) != Constness::Const) {
            // An argument that is not a literal may be changed by the call.
            self.facts.out_channel = true;
        }
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
    pub(super) fn eval_method_call(&mut self, base: &Expr, args: &[Expr]) -> Val {
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

    pub(super) fn record_site(&mut self, kind: SiteKind) {
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
        let (merged, constness, self_cmp, undecided_flag, ext) = match &kind {
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
                let ext = entries.iter().any(|e| e.ext);
                (merged, constness, self_cmp, undecided, ext)
            }
            SiteKind::Macro {
                val,
                constness,
                ext,
            } => {
                let constness = if self.dead {
                    Constness::Undecided
                } else {
                    *constness
                };
                (*val, constness, false, false, *ext)
            }
        };
        if ext {
            self.facts.ext_observed = true;
        }
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
