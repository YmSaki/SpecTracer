//! The statements of the walk: sequencing, branches, switches, loops and
//! assignments (DS-1887, DS-1973, DS-1975, DS-1976, DS-1983).
//!
//! Responsibility: execute each statement over the state of `Interp`, joining the
//! states of the paths that meet, iterating a loop to a fixed point, and recording
//! the exits that decide what follows. Boundary: the value of an expression is
//! read by `eval`, and what an assert-equivalent call is by `eval`. Realizes
//! DES-619.

use super::*;

impl<'a> Interp<'a> {
    // -- statements ----------------------------------------------------

    pub(super) fn exec_stmts(&mut self, stmts: &[Stmt]) {
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
    pub(super) fn exec_arm(&mut self, stmts: &[Stmt]) {
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

    pub(super) fn exec_stmt(&mut self, s: &Stmt) {
        if self.fuel == 0 {
            self.structure.exhausted = true;
            self.facts.taint = true;
            return;
        }
        self.fuel -= 1;
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
                if self.reads_external(x) {
                    let places: Vec<&Expr> = key.iter().chain(value.iter()).collect();
                    self.note_external_binding(&places, std::slice::from_ref(x));
                }
                let xval = self.eval(x, PLAIN);
                if xval.derived() {
                    // The variables of the loop hold its elements, which they
                    // may change.
                    self.alias_source(x);
                }
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

    pub(super) fn exec_decl(&mut self, specs: &[DeclSpec]) {
        for spec in specs {
            match spec {
                DeclSpec::Var { names, ty, values } => {
                    if values.iter().any(|v| self.reads_external(v)) {
                        self.ext_names
                            .extend(names.iter().map(|name| name.name.clone()));
                    }
                    let vals: Vec<Val> = values.iter().map(|v| self.eval(v, PLAIN)).collect();
                    let t_typed = ty.as_ref().is_some_and(|ty| self.type_is_t(ty));
                    for (i, name) in names.iter().enumerate() {
                        let val = if vals.len() == 1 && names.len() > 1 {
                            vals[0]
                        } else {
                            vals.get(i).copied().unwrap_or(Val::NO)
                        };
                        let ty = match ty {
                            Some(ty) => self.type_ident(ty),
                            None if vals.len() == 1 && names.len() > 1 => None,
                            None => values.get(i).and_then(|v| self.static_type_of(v)),
                        };
                        if let Some(id) = self.declare(name, t_typed, val) {
                            self.vars[id].ty = ty;
                        }
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
                        if let Some(id) = self.declare(name, false, Val::NO) {
                            self.vars[id].is_const = true;
                        }
                    }
                }
                DeclSpec::Type { name } => {
                    // A local type hides what it is named like.
                    self.declare(name, false, Val::NO);
                }
            }
        }
    }

    pub(super) fn exec_expr_stmt(&mut self, x: &Expr) {
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
            Term::Fatal => {
                self.last_stmt_failure = true;
                self.env = None;
            }
            Term::Panic => {
                self.last_stmt_failure = true;
                // A panic that a `defer` recovers ends the function as a return
                // does (DS-1982): the code after the `t.Run` of a subtest goes
                // on with what it assigned.
                if self.frames.last().is_some_and(|frame| frame.recover_defer) {
                    self.record_exit(false);
                    if let (Some(env), Some(frame)) = (self.env.clone(), self.frames.last_mut()) {
                        frame.returns.push(env);
                    }
                }
                self.env = None;
            }
            Term::Exit => {
                self.record_exit(false);
                self.env = None;
            }
            Term::Skip | Term::Goexit => {
                // The subtest ends and the code after its `t.Run` goes on, with
                // what the subtest assigned.
                self.record_exit(false);
                if let (Some(env), Some(frame)) = (self.env.clone(), self.frames.last_mut()) {
                    frame.returns.push(env);
                }
                self.env = None;
            }
        }
    }

    pub(super) fn exec_branch(&mut self, kind: BranchKind, labeled: bool) {
        match kind {
            BranchKind::Fallthrough => {
                // Whether the next clause is entered by its own case or by this
                // clause is not decided by the syntax (DS-1983).
                self.taint();
                self.record_undecided();
            }
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
                if self.dead || self.dec.iter().any(|e| e.never_run) {
                    // Never taken: it leaves nothing, and what follows the loop
                    // is reached only by a `break` that is.
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
    pub(super) fn record_exit(&mut self, unfixed: bool) {
        if self.dead || self.dec.iter().any(|e| e.never_run) {
            // An exit that is never taken decides nothing.
            return;
        }
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
    pub(super) fn record_break(&mut self, index: usize, loop_frame: bool) {
        if self.dead || self.dec.iter().any(|e| e.never_run) {
            return;
        }
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

    pub(super) fn record_undecided(&mut self) {
        if let Some(frame) = self.frames.last_mut() {
            frame.undecided_after = true;
        }
    }

    pub(super) fn make_entry(&self, cond: &Expr, val: Val) -> DEntry {
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
            ext: self.reads_external(cond),
        }
    }

    /// Whether `x` reads a call or a name that is not a local variable: state
    /// that the target may have changed (DS-623).
    pub(super) fn reads_external(&self, x: &Expr) -> bool {
        if const_bool(x, self).is_some() {
            // Fixed by literals (`X || true`): nothing is read.
            return false;
        }
        let mut skipped: BTreeSet<usize> = BTreeSet::new();
        let mut found = false;
        for_each_expr(x, true, &mut |e| match &e.kind {
            ExprKind::Call { fun, .. } => {
                let callee = strip(fun);
                // The call of the target being audited, a conversion and `len` /
                // `cap` read no state of their own.
                let reads_nothing = match (&callee.kind, self.target) {
                    (ExprKind::Ident(n), target) => {
                        self.lookup(n).is_none()
                            && (BUILTIN_TYPES.contains(&n.as_str())
                                || matches!(n.as_str(), "len" | "cap")
                                || target.is_some_and(|t| !t.is_method && t.symbol == *n))
                    }
                    (ExprKind::Selector(base, sel), Some(t)) if t.symbol == sel.name.as_str() => {
                        // A method of a value is the target's when it is a method
                        // target; `pkg.Name` is a function of a package.
                        t.is_method
                            || matches!(&strip(base).kind, ExprKind::Ident(q) if self.lookup(q).is_none())
                    }
                    _ => false,
                };
                if reads_nothing {
                    // The name of the callee and its qualifier read no state.
                    let mut root = callee;
                    while let ExprKind::Selector(base, _) = &root.kind {
                        root = strip(base);
                    }
                    skipped.insert(root.span.start);
                } else {
                    found = true;
                }
            }
            ExprKind::Ident(name)
                if !skipped.contains(&e.span.start)
                    && (self.ext_names.contains(name)
                        || (self.lookup(name).is_none()
                            && !matches!(name.as_str(), "true" | "false" | "nil" | "iota")
                            && !BUILTIN_TYPES.contains(&name.as_str()))) =>
            {
                found = true;
            }
            _ => {}
        });
        found
    }

    /// The names `places` are bound to what `values` read (DS-623).
    pub(super) fn note_external_binding(&mut self, places: &[&Expr], values: &[Expr]) {
        if values.iter().any(|v| self.reads_external(v)) {
            for place in places {
                if let Some(name) = root_ident(place) {
                    self.ext_names.insert(name.to_owned());
                }
            }
        }
    }

    fn extras_len(&self) -> usize {
        self.frames.last().map_or(0, |frame| frame.extras.len())
    }

    /// The exits recorded since `mark`, taken out: an exit in one branch does
    /// not decide the other branch (DS-1983).
    fn take_extras(&mut self, mark: usize) -> Vec<Extra> {
        self.frames
            .last_mut()
            .map(|frame| frame.extras.split_off(mark.min(frame.extras.len())))
            .unwrap_or_default()
    }

    fn restore_extras(&mut self, extras: Vec<Extra>) {
        if let Some(frame) = self.frames.last_mut() {
            frame.extras.extend(extras);
        }
    }

    pub(super) fn tokens_of(&self, span: Span) -> Option<Vec<(Tok, &'a str)>> {
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
    pub(super) fn has_self_comparison(&self, cond: &Expr) -> bool {
        if let Some(known) = self.self_cmp_cache.borrow().get(&cond.span.start) {
            return *known;
        }
        let found = self.find_self_comparison(cond);
        self.self_cmp_cache
            .borrow_mut()
            .insert(cond.span.start, found);
        found
    }

    /// Whether two expressions are the same tokens, through the parentheses
    /// around either (DS-1889).
    pub(super) fn same_tokens(&self, a: &Expr, b: &Expr) -> bool {
        let (ta, tb) = (self.tokens_of(strip(a).span), self.tokens_of(strip(b).span));
        ta.is_some() && ta == tb
    }

    /// A condition built from literals that is not evaluated here (`'a' == 'b'`):
    /// it may be false, so what it guards may never run (DS-621).
    pub(super) fn unevaluated_literal(&self, cond: &Expr) -> bool {
        classify(cond, self) == Constness::Const && const_bool(cond, self).is_none()
    }

    /// The same for the tag and a case of a `switch`, which compare as `==`.
    fn unevaluated_case(&self, tag: &Expr, case: &Expr) -> bool {
        let equal = Expr {
            span: case.span,
            kind: ExprKind::Binary("==", Box::new(tag.clone()), Box::new(case.clone())),
        };
        classify(tag, self) == Constness::Const
            && classify(case, self) == Constness::Const
            && const_bool(&equal, self).is_none()
    }

    fn find_self_comparison(&self, cond: &Expr) -> bool {
        let mut found = false;
        for_each_expr(cond, false, &mut |e| {
            if let ExprKind::Binary("==" | "!=", a, b) = &e.kind {
                found |= self.same_tokens(a, b);
            }
        });
        found
    }

    pub(super) fn exec_if(
        &mut self,
        init: Option<&Stmt>,
        cond: &Expr,
        then: &Block,
        els: Option<&Stmt>,
    ) {
        self.push_scope();
        if let Some(init) = init {
            self.exec_stmt(init);
        }
        let val = self.eval(cond, DECIDER);
        let mut entry = self.make_entry(cond, val);
        entry.undecided = self.unevaluated_literal(cond);
        let fixed = const_bool(cond, self);
        let base = self.env.clone();
        let mark = self.extras_len();

        let mut then_entry = entry.clone();
        then_entry.never_run = fixed == Some(false);
        self.dec.push(then_entry);
        self.exec_arm(&then.stmts);
        self.dec.pop();
        let then_env = self.env.take();
        let then_extras = self.take_extras(mark);

        self.env = base;
        if let Some(els) = els {
            let mut else_entry = entry;
            else_entry.never_run = fixed == Some(true);
            self.dec.push(else_entry);
            self.exec_stmt(els);
            self.dec.pop();
        }
        let else_env = self.env.take();
        let else_extras = self.take_extras(mark);
        // What an exit in one branch adds decides what comes after the `if`,
        // not the other branch (DS-1983).
        self.restore_extras(then_extras);
        self.restore_extras(else_extras);
        // A constant condition takes one way only: the other path does not
        // reach what follows.
        let then_env = if fixed == Some(false) { None } else { then_env };
        let else_env = if fixed == Some(true) { None } else { else_env };
        self.env = join_opt(then_env, else_env);
        self.pop_scope();
    }

    pub(super) fn exec_switch(
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
        let x_val = tag.map(|t| self.eval(t, PLAIN));
        // The guard of a type switch is not a tag expression (DS-1887): a check
        // of the type of a derived value is not shown to decide by the value.
        let tag_val = x_val.map(|v| {
            if bind.is_some() && v.derived() {
                Val {
                    d: D::Unk,
                    comp: false,
                    deleg: v.deleg,
                }
            } else {
                v
            }
        });
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
                let mut entry = self.make_entry(e, v);
                // `switch x { case x: }` compares the tag with itself (DS-1889).
                entry.self_cmp |= tag.is_some_and(|t| self.same_tokens(t, e));
                // Literals compared without being evaluated: the clause may
                // never run (DS-621).
                entry.undecided |= match tag {
                    Some(t) => self.unevaluated_case(t, e),
                    None => self.unevaluated_literal(e),
                };
                entries.push(entry);
            }
            case_entries.push(entries);
        }
        // A clause that no value reaches: one whose cases are all false (`switch 1
        // { case 2: .. }`, `case false:`), and every clause after a case that is
        // true, a `default` wherever it stands (it is chosen only when no case
        // matches).
        let case_holds = |e: &Expr| -> Option<bool> {
            match tag {
                Some(t) => {
                    let equal = Expr {
                        span: e.span,
                        kind: ExprKind::Binary("==", Box::new(t.clone()), Box::new(e.clone())),
                    };
                    const_bool(&equal, self)
                }
                None => const_bool(e, self),
            }
        };
        let first_true = if bind.is_some() {
            None
        } else {
            clauses
                .iter()
                .position(|c| c.exprs.iter().any(|e| case_holds(e) == Some(true)))
        };
        let clause_dead: Vec<bool> = clauses
            .iter()
            .enumerate()
            .map(|(k, clause)| {
                if bind.is_some() {
                    return false;
                }
                if first_true.is_some_and(|j| k > j || clause.default) {
                    return true;
                }
                if clause.default || clause.exprs.is_empty() {
                    return false;
                }
                clause.exprs.iter().all(|e| case_holds(e) == Some(false))
            })
            .collect();
        let base = self.env.clone();
        let has_default = clauses.iter().any(|c| c.default);
        let mut clause_extras: Vec<Extra> = Vec::new();
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
            for (j, entries) in case_entries[range].iter().enumerate() {
                for entry in entries {
                    let mut entry = entry.clone();
                    if j > k && !clause.default && entry.val.derived() {
                        // The cases are tried in order: a clause is not decided by
                        // the case expressions after it, whatever the place of
                        // `default` (DS-1887 reads them as deciding).
                        entry.val = Val {
                            d: D::Unk,
                            comp: false,
                            deleg: entry.val.deleg,
                        };
                    }
                    self.dec.push(entry);
                    pushed += 1;
                }
            }
            if clause_dead[k] {
                // No value reaches the clause: what it holds never runs.
                self.dec.push(DEntry {
                    val: Val::NO,
                    constness: Constness::Const,
                    never_run: true,
                    self_cmp: false,
                    undecided: false,
                    ext: false,
                });
                pushed += 1;
            }
            let mark = self.extras_len();
            self.env = join_opt(base.clone(), carry.take());
            self.push_scope();
            if let Some(Some(ident)) = bind {
                let span = Span {
                    start: clause.span.start,
                    end: clause.span.start,
                };
                let xv = x_val.unwrap_or(Val::NO);
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
            // An exit in one clause does not decide the next (DS-1983).
            clause_extras.extend(self.take_extras(mark));
            if clause_dead[k] {
                // No value reaches it: it ends no path.
                self.env = None;
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
        self.restore_extras(clause_extras);
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
    pub(super) fn exec_arm_inline(&mut self, stmts: &[Stmt]) {
        self.last_stmt_failure = false;
        self.exec_stmts(stmts);
        let ends_in_failure = self.last_stmt_failure && self.env.is_some();
        if ends_in_failure {
            self.env = None;
        }
        self.last_stmt_failure = false;
    }

    // -- loops ---------------------------------------------------------

    pub(super) fn exec_loop(&mut self, header: LoopHeader<'_>, body: &Block) {
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

    pub(super) fn loop_pass(
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
        let fixed_cond = match header {
            LoopHeader::For { cond: Some(c), .. } => const_bool(c, self),
            _ => None,
        };
        if let Some(entry) = &mut entry {
            let emptiness = match header {
                LoopHeader::Range { x, .. } => range_emptiness(x),
                LoopHeader::For { .. } => Emptiness::NotLiteral,
            };
            entry.undecided |= undecided
                || emptiness == Emptiness::Unknown
                || matches!(header, LoopHeader::For { cond: Some(c), .. } if self.unevaluated_literal(c));
            // `for false {}` and a range over an empty literal run no body.
            entry.never_run = fixed_cond == Some(false) || emptiness == Emptiness::Empty;
        }
        // `for {}` and `for true {}` end only by a `break`.
        let endless =
            matches!(header, LoopHeader::For { cond: None, .. }) || fixed_cond == Some(true);
        let exit = if endless { None } else { self.env.clone() };
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
        if fixed_cond == Some(false) {
            // The body never runs.
            end = None;
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

    pub(super) fn bind_range(
        &mut self,
        xval: Val,
        key: Option<&Expr>,
        value: Option<&Expr>,
        define: bool,
    ) {
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
    pub(super) fn alias_source(&mut self, x: &Expr) {
        let x = strip(x);
        let aliasing = matches!(
            x.kind,
            ExprKind::Selector(..)
                | ExprKind::Index(..)
                | ExprKind::Slice(..)
                | ExprKind::Star(_)
                | ExprKind::TypeAssert(..)
        );
        if !(aliasing || matches!(x.kind, ExprKind::Ident(_))) {
            return;
        }
        // A bare name, or a part of a variable (a field, a slice of it, a method
        // value, which holds its address): the variable may be changed through
        // what it was assigned to.
        let Some(id) = root_ident(x).and_then(|name| self.lookup(name)) else {
            return;
        };
        if aliasing {
            self.addr_taken.insert(id);
        }
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

    /// DS-1976: every local variable the expression refers to that holds a
    /// derived value is unknown afterwards (the call may keep or change it).
    pub(super) fn escape(&mut self, x: &Expr) {
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

    pub(super) fn exec_assign(&mut self, lhs: &[Expr], op: &str, rhs: &[Expr]) {
        let places: Vec<&Expr> = lhs.iter().collect();
        self.note_external_binding(&places, rhs);
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
                            let ty = if single {
                                None
                            } else {
                                rhs.get(i).and_then(|r| self.static_type_of(r))
                            };
                            let declared = self.declare(
                                &Ident {
                                    name: name.clone(),
                                    span: l.span,
                                },
                                t_typed,
                                val,
                            );
                            if let Some(id) = declared {
                                self.vars[id].ty = ty;
                            }
                        }
                    }
                }
            }
            "=" => {
                for (i, l) in lhs.iter().enumerate() {
                    let val = val_for(i);
                    if let ExprKind::Ident(name) = &strip(l).kind {
                        let holds_t = self.lookup(name).is_some_and(|id| self.vars[id].t_typed);
                        let from_t = !single && rhs.get(i).is_some_and(|r| self.is_t_ident(r));
                        if holds_t && !from_t {
                            self.structure.t_replaced = true;
                        }
                    }
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
    pub(super) fn assign_place(&mut self, place: &Expr, val: Val) {
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
    pub(super) fn compound_assign(&mut self, place: &Expr, val: Val) {
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
}
