//! Rust-specific static-audit rules (DA-001..006 and their AST analysis).
//! These move to vtest-adapter-rust with the static audit; isolating them
//! into a scan/store-free module keeps the cross-crate move mechanical.

use std::collections::{BTreeMap, BTreeSet};

use quote::ToTokens;
use syn::parse::Parser;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{
    Attribute, Expr, ExprCall, ExprClosure, ExprMethodCall, File, Item, ItemFn, ItemUse, Macro,
    Pat, ReturnType, Stmt, Type, UseTree,
};
use vtest_model::{AdapterId, ProjectPath, SourceLocation, SourceRange};

/// Deterministic per-rule verdict. Local to the adapter; the core maps this
/// onto `CheckValue` when it reshapes the observation into an `AuditRecord`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AuditVerdict {
    Pass,
    Fail,
    Unknown,
}

/// One rule outcome with its concrete source location. Converted to
/// `RuleObservationDraft` at the `StaticAuditAdapter::audit` boundary.
#[derive(Clone, Debug)]
pub(crate) struct RuleResult {
    pub rule: String,
    pub verdict: AuditVerdict,
    pub reason: String,
    pub location: SourceLocation,
}

pub(crate) fn has_attribute(attrs: &[Attribute], name: &str) -> bool {
    attrs.iter().any(|attribute| {
        attribute
            .path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == name)
    })
}

pub(crate) fn source_location_from_item(item: &ItemFn) -> SourceLocation {
    source_location_from_span(item, item.span())
}

pub(crate) fn source_location_from_span(item: &ItemFn, span: proc_macro2::Span) -> SourceLocation {
    SourceLocation {
        adapter: AdapterId::new("rust-cargo"),
        path: ProjectPath::new(""),
        locator: item.sig.ident.to_string(),
        byte_range: SourceRange {
            start: 0,
            end: 0,
            start_line: span.start().line,
            end_line: span.end().line,
        },
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Constness {
    Constant,
    Runtime,
    Unknown,
}

#[derive(Clone)]
pub(crate) struct CollectedAssertion {
    name: String,
    args: Vec<Expr>,
    span: proc_macro2::Span,
}

pub(crate) fn rule_da001(
    item: &ItemFn,
    file: &File,
    assertion_macros: &[String],
    target_resolution: Option<&TargetResolution<'_>>,
) -> RuleResult {
    let constants = ConstantContext::new(file, item);
    let call_facts = call_facts(item, assertion_macros);
    let bare_shadowed = target_resolution
        .is_some_and(|resolution| call_facts.bare_call_is_shadowed(resolution.target_name()));
    let mut assertions = Vec::new();
    collect_assertions(item, assertion_macros, &mut assertions);
    let assertions = assertions
        .into_iter()
        .filter(|assertion| assertion.name != "panic")
        .collect::<Vec<_>>();
    let mut saw_runtime = false;
    let mut saw_unknown = false;
    for assertion in &assertions {
        if assertion.args.is_empty() {
            saw_unknown = true;
            continue;
        }
        let values = assertion.args.iter().map(|expr| {
            constants.classify(expr, target_resolution, bare_shadowed, &mut BTreeSet::new())
        });
        match combine_constness(values) {
            Constness::Constant => {}
            Constness::Runtime => saw_runtime = true,
            Constness::Unknown => saw_unknown = true,
        }
    }
    let verdict = if assertions.is_empty() || saw_runtime {
        AuditVerdict::Pass
    } else if saw_unknown {
        AuditVerdict::Unknown
    } else {
        AuditVerdict::Fail
    };
    RuleResult {
        rule: "DA-001".to_owned(),
        verdict,
        reason: match verdict {
            AuditVerdict::Fail => {
                "all assertion arguments are provably literal or constant expressions".to_owned()
            }
            AuditVerdict::Unknown => {
                "at least one assertion expression has unresolved constness".to_owned()
            }
            AuditVerdict::Pass => {
                "at least one assertion depends on runtime data, or the rule is not applicable"
                    .to_owned()
            }
        },
        location: assertions
            .first()
            .map(|assertion| source_location_from_span(item, assertion.span))
            .unwrap_or_else(|| source_location_from_item(item)),
    }
}

pub(crate) struct ConstantContext<'a> {
    expressions: BTreeMap<String, &'a Expr>,
}

impl<'a> ConstantContext<'a> {
    pub(crate) fn new(file: &'a File, item: &'a ItemFn) -> Self {
        fn collect<'a>(
            items: &'a [Item],
            prefix: &str,
            output: &mut Vec<(String, String, &'a Expr)>,
        ) {
            for item in items {
                match item {
                    Item::Const(item_const) => {
                        let short = item_const.ident.to_string();
                        let full = if prefix.is_empty() {
                            short.clone()
                        } else {
                            format!("{prefix}::{short}")
                        };
                        output.push((full, short, &item_const.expr));
                    }
                    Item::Mod(item_mod) => {
                        if let Some((_, nested)) = &item_mod.content {
                            let name = item_mod.ident.to_string();
                            let nested_prefix = if prefix.is_empty() {
                                name
                            } else {
                                format!("{prefix}::{name}")
                            };
                            collect(nested, &nested_prefix, output);
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut entries = Vec::new();
        collect(&file.items, "", &mut entries);
        let mut short_counts = BTreeMap::<String, usize>::new();
        for (_, short, _) in &entries {
            *short_counts.entry(short.clone()).or_default() += 1;
        }
        let mut expressions = BTreeMap::new();
        for (full, short, expr) in entries {
            expressions.insert(full, expr);
            if short_counts.get(&short) == Some(&1) {
                expressions.insert(short, expr);
            }
        }
        for statement in &item.block.stmts {
            if let Stmt::Item(Item::Const(item_const)) = statement {
                expressions.insert(item_const.ident.to_string(), &item_const.expr);
            }
        }
        Self { expressions }
    }

    fn classify(
        &self,
        expr: &Expr,
        target_resolution: Option<&TargetResolution<'_>>,
        bare_shadowed: bool,
        visiting: &mut BTreeSet<String>,
    ) -> Constness {
        match expr {
            Expr::Lit(_) => Constness::Constant,
            Expr::Paren(value) => {
                self.classify(&value.expr, target_resolution, bare_shadowed, visiting)
            }
            Expr::Group(value) => {
                self.classify(&value.expr, target_resolution, bare_shadowed, visiting)
            }
            Expr::Unary(value) => {
                self.classify(&value.expr, target_resolution, bare_shadowed, visiting)
            }
            Expr::Binary(value) => combine_constness([
                self.classify(&value.left, target_resolution, bare_shadowed, visiting),
                self.classify(&value.right, target_resolution, bare_shadowed, visiting),
            ]),
            Expr::Tuple(value) => combine_constness(
                value
                    .elems
                    .iter()
                    .map(|expr| self.classify(expr, target_resolution, bare_shadowed, visiting)),
            ),
            Expr::Array(value) => combine_constness(
                value
                    .elems
                    .iter()
                    .map(|expr| self.classify(expr, target_resolution, bare_shadowed, visiting)),
            ),
            Expr::Repeat(value) => combine_constness([
                self.classify(&value.expr, target_resolution, bare_shadowed, visiting),
                self.classify(&value.len, target_resolution, bare_shadowed, visiting),
            ]),
            Expr::Call(call) => {
                let call_path = match call.func.as_ref() {
                    Expr::Path(path) => Some(
                        path.path
                            .segments
                            .iter()
                            .map(|segment| segment.ident.to_string())
                            .collect::<Vec<_>>()
                            .join("::"),
                    ),
                    _ => None,
                };
                if call_path.as_deref().zip(target_resolution).is_some_and(
                    |(call_path, resolution)| {
                        resolution.classify(call_path, bare_shadowed) == TargetCallMatch::Proven
                    },
                ) {
                    Constness::Runtime
                } else {
                    Constness::Unknown
                }
            }
            Expr::MethodCall(_) => Constness::Unknown,
            Expr::Await(value) => {
                self.classify(&value.base, target_resolution, bare_shadowed, visiting)
            }
            Expr::Try(value) => {
                self.classify(&value.expr, target_resolution, bare_shadowed, visiting)
            }
            Expr::Path(path) => {
                let path = path
                    .path
                    .segments
                    .iter()
                    .map(|segment| segment.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                let lookup = path.strip_prefix("crate::").unwrap_or(&path);
                let Some(value) = self.expressions.get(lookup) else {
                    return Constness::Unknown;
                };
                if !visiting.insert(lookup.to_owned()) {
                    return Constness::Unknown;
                }
                let result = self.classify(value, target_resolution, bare_shadowed, visiting);
                visiting.remove(lookup);
                result
            }
            _ => Constness::Unknown,
        }
    }
}

pub(crate) fn combine_constness(values: impl IntoIterator<Item = Constness>) -> Constness {
    let mut saw_unknown = false;
    for value in values {
        match value {
            Constness::Runtime => return Constness::Runtime,
            Constness::Unknown => saw_unknown = true,
            Constness::Constant => {}
        }
    }
    if saw_unknown {
        Constness::Unknown
    } else {
        Constness::Constant
    }
}

pub(crate) fn rule_da002(
    item: &ItemFn,
    file: &File,
    target_resolution: Option<&TargetResolution<'_>>,
    assertion_macros: &[String],
) -> RuleResult {
    let functions = functions_by_name(file);
    let facts = call_facts(item, assertion_macros);
    let Some(target_resolution) = target_resolution else {
        return RuleResult {
            rule: "DA-002".to_owned(),
            verdict: AuditVerdict::Unknown,
            reason: "declared target symbol could not be resolved".to_owned(),
            location: source_location_from_item(item),
        };
    };
    let direct_match = facts_target_match(&facts, target_resolution);
    if direct_match == TargetCallMatch::Proven {
        return RuleResult {
            rule: "DA-002".to_owned(),
            verdict: AuditVerdict::Pass,
            reason: "declared target is called directly".to_owned(),
            location: source_location_from_item(item),
        };
    }

    let mut uncertain = facts.uncertain || direct_match == TargetCallMatch::Ambiguous;
    for name in &facts.names {
        if matches!(name.as_str(), "unwrap" | "expect") {
            continue;
        }
        let Some(candidates) = functions.get(name) else {
            uncertain = true;
            continue;
        };
        if candidates.len() != 1 || std::ptr::eq(candidates[0], item) {
            uncertain = true;
            continue;
        }
        let helper_facts = call_facts(candidates[0], assertion_macros);
        let Some(helper_path) = function_item_path(file, candidates[0]) else {
            uncertain = true;
            continue;
        };
        let helper_resolution = target_resolution.for_caller(&helper_path);
        let helper_match = facts_target_match(&helper_facts, &helper_resolution);
        if helper_match == TargetCallMatch::Proven {
            return RuleResult {
                rule: "DA-002".to_owned(),
                verdict: AuditVerdict::Pass,
                reason: format!("same-file helper `{name}` calls the declared target"),
                location: source_location_from_item(candidates[0]),
            };
        }
        // Only one helper hop is in the deterministic analysis boundary.
        if helper_facts.uncertain
            || helper_match == TargetCallMatch::Ambiguous
            || !helper_facts.names.is_empty()
        {
            uncertain = true;
        }
    }
    let verdict = if uncertain {
        AuditVerdict::Unknown
    } else {
        AuditVerdict::Fail
    };
    RuleResult {
        rule: "DA-002".to_owned(),
        verdict,
        reason: if verdict == AuditVerdict::Fail {
            "declared target is absent after direct and one-hop same-file analysis".to_owned()
        } else {
            "an external, ambiguous, macro, or closure call may reach the target".to_owned()
        },
        location: source_location_from_item(item),
    }
}

#[derive(Default)]
pub(crate) struct CallFacts {
    pub(crate) names: BTreeSet<String>,
    paths: BTreeSet<String>,
    methods: BTreeSet<String>,
    shadowed_names: BTreeSet<String>,
    has_glob_import: bool,
    uncertain: bool,
}

impl CallFacts {
    fn bare_call_is_shadowed(&self, target_name: &str) -> bool {
        self.has_glob_import || self.shadowed_names.contains(target_name)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TargetCallMatch {
    Proven,
    Ambiguous,
    Absent,
}

#[derive(Clone, Debug)]
pub(crate) struct TargetResolution<'a> {
    target_item_path: &'a str,
    caller_item_path: String,
    same_file: bool,
}

impl<'a> TargetResolution<'a> {
    pub(crate) fn new(target_item_path: &'a str, caller_item_path: &str, same_file: bool) -> Self {
        Self {
            target_item_path,
            caller_item_path: caller_item_path.to_owned(),
            same_file,
        }
    }

    fn for_caller(&self, caller_item_path: &str) -> Self {
        Self::new(self.target_item_path, caller_item_path, self.same_file)
    }

    fn target_name(&self) -> &str {
        self.target_item_path.split("::").last().unwrap_or_default()
    }

    fn classify(&self, call_path: &str, bare_shadowed: bool) -> TargetCallMatch {
        let call_name = call_path.split("::").last().unwrap_or_default();
        if call_name != self.target_name() {
            return TargetCallMatch::Absent;
        }
        if !self.same_file {
            return TargetCallMatch::Ambiguous;
        }
        if !call_path.contains("::") {
            if bare_shadowed {
                return TargetCallMatch::Ambiguous;
            }
            return if module_path(&self.caller_item_path) == module_path(self.target_item_path) {
                TargetCallMatch::Proven
            } else {
                TargetCallMatch::Ambiguous
            };
        }
        match resolve_call_path(call_path, module_path(&self.caller_item_path)) {
            Some(resolved) if resolved == self.target_item_path => TargetCallMatch::Proven,
            _ => TargetCallMatch::Ambiguous,
        }
    }
}

pub(crate) fn classify_target_call(
    call_path: &str,
    resolution: &TargetResolution<'_>,
    bare_shadowed: bool,
) -> TargetCallMatch {
    resolution.classify(call_path, bare_shadowed)
}

pub(crate) fn module_path(item_path: &str) -> &str {
    item_path.rsplit_once("::").map_or("", |(module, _)| module)
}

pub(crate) fn resolve_call_path(call_path: &str, caller_module: &str) -> Option<String> {
    if let Some(path) = call_path.strip_prefix("crate::") {
        return Some(path.to_owned());
    }
    let mut modules = caller_module
        .split("::")
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let mut remaining = call_path;
    if let Some(path) = remaining.strip_prefix("self::") {
        remaining = path;
    } else {
        while let Some(path) = remaining.strip_prefix("super::") {
            modules.pop()?;
            remaining = path;
        }
    }
    modules.extend(
        remaining
            .split("::")
            .filter(|part| !part.is_empty())
            .map(str::to_owned),
    );
    Some(modules.join("::"))
}

pub(crate) fn facts_target_match(
    facts: &CallFacts,
    resolution: &TargetResolution<'_>,
) -> TargetCallMatch {
    let mut result = TargetCallMatch::Absent;
    for path in &facts.paths {
        match classify_target_call(
            path,
            resolution,
            facts.bare_call_is_shadowed(resolution.target_name()),
        ) {
            TargetCallMatch::Proven => return TargetCallMatch::Proven,
            TargetCallMatch::Ambiguous => result = TargetCallMatch::Ambiguous,
            TargetCallMatch::Absent => {}
        }
    }
    let target_name = resolution.target_name();
    if facts.methods.contains(target_name) {
        result = TargetCallMatch::Ambiguous;
    }
    result
}

pub(crate) struct CallVisitor<'a> {
    assertion_macros: &'a [String],
    facts: CallFacts,
}

pub(crate) fn call_facts(item: &ItemFn, assertion_macros: &[String]) -> CallFacts {
    let mut visitor = CallVisitor {
        assertion_macros,
        facts: CallFacts::default(),
    };
    visitor.visit_block(&item.block);
    visitor.facts
}

impl<'ast> Visit<'ast> for CallVisitor<'_> {
    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Expr::Path(path) = node.func.as_ref() {
            if let Some(name) = path.path.segments.last() {
                self.facts.names.insert(name.ident.to_string());
                self.facts.paths.insert(
                    path.path
                        .segments
                        .iter()
                        .map(|segment| segment.ident.to_string())
                        .collect::<Vec<_>>()
                        .join("::"),
                );
            }
        } else {
            self.facts.uncertain = true;
        }
        visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        self.facts.names.insert(node.method.to_string());
        self.facts.methods.insert(node.method.to_string());
        visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_closure(&mut self, _node: &'ast ExprClosure) {
        self.facts.uncertain = true;
    }

    fn visit_pat_ident(&mut self, node: &'ast syn::PatIdent) {
        self.facts.shadowed_names.insert(node.ident.to_string());
        visit::visit_pat_ident(self, node);
    }

    fn visit_item_use(&mut self, node: &'ast ItemUse) {
        collect_use_bindings(
            &node.tree,
            &mut self.facts.shadowed_names,
            &mut self.facts.has_glob_import,
        );
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        // A local function declaration occupies the value namespace.  Do not
        // walk its body: calls inside a nested item are not direct calls from
        // the Test being audited.
        self.facts.shadowed_names.insert(node.sig.ident.to_string());
    }

    fn visit_item_const(&mut self, node: &'ast syn::ItemConst) {
        self.facts.shadowed_names.insert(node.ident.to_string());
    }

    fn visit_item_static(&mut self, node: &'ast syn::ItemStatic) {
        self.facts.shadowed_names.insert(node.ident.to_string());
    }

    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        self.facts.shadowed_names.insert(node.ident.to_string());
    }

    fn visit_macro(&mut self, node: &'ast Macro) {
        if is_assertion_macro(node, self.assertion_macros) {
            match try_parse_macro_exprs(node) {
                Some(args) => {
                    for expr in &args {
                        self.visit_expr(expr);
                    }
                }
                None => self.facts.uncertain = true,
            }
        } else {
            self.facts.uncertain = true;
        }
    }
}

pub(crate) fn collect_use_bindings(
    tree: &UseTree,
    names: &mut BTreeSet<String>,
    has_glob: &mut bool,
) {
    match tree {
        UseTree::Path(path) => collect_use_bindings(&path.tree, names, has_glob),
        UseTree::Name(name) => {
            names.insert(name.ident.to_string());
        }
        UseTree::Rename(rename) => {
            names.insert(rename.rename.to_string());
        }
        UseTree::Glob(_) => *has_glob = true,
        UseTree::Group(group) => {
            for item in &group.items {
                collect_use_bindings(item, names, has_glob);
            }
        }
    }
}

pub(crate) fn functions_by_name(file: &File) -> BTreeMap<String, Vec<&ItemFn>> {
    fn collect<'a>(items: &'a [Item], output: &mut BTreeMap<String, Vec<&'a ItemFn>>) {
        for item in items {
            match item {
                Item::Fn(item_fn) => output
                    .entry(item_fn.sig.ident.to_string())
                    .or_default()
                    .push(item_fn),
                Item::Mod(item_mod) => {
                    if let Some((_, items)) = &item_mod.content {
                        collect(items, output);
                    }
                }
                _ => {}
            }
        }
    }
    let mut functions = BTreeMap::new();
    collect(&file.items, &mut functions);
    functions
}

pub(crate) fn rule_da003(
    item: &ItemFn,
    file: &File,
    target_resolution: Option<&TargetResolution<'_>>,
    assertion_macros: &[String],
) -> RuleResult {
    let Some(target_resolution) = target_resolution else {
        return RuleResult {
            rule: "DA-003".to_owned(),
            verdict: AuditVerdict::Unknown,
            reason: "declared target symbol could not be resolved".to_owned(),
            location: source_location_from_item(item),
        };
    };
    let result_returning = returns_result_and_can_err(item);
    let facts = call_facts(item, assertion_macros);
    let mut analyzer = FlowAnalyzer::new(
        target_resolution.clone(),
        assertion_macros,
        result_returning,
        facts.bare_call_is_shadowed(target_resolution.target_name()),
    );
    analyzer.analyze_block(&item.block);
    let called = !analyzer.all_origins.is_empty();
    let helper_boundary = if called {
        false
    } else {
        let functions = functions_by_name(file);
        facts.names.iter().any(|name| {
            functions.get(name).is_some_and(|candidates| {
                if candidates.len() != 1 || std::ptr::eq(candidates[0], item) {
                    return false;
                }
                let Some(helper_path) = function_item_path(file, candidates[0]) else {
                    return true;
                };
                facts_target_match(
                    &call_facts(candidates[0], assertion_macros),
                    &target_resolution.for_caller(&helper_path),
                ) != TargetCallMatch::Absent
            })
        })
    };
    let should_panic = has_attribute(&item.attrs, "should_panic");
    let all_verified = called && analyzer.all_origins.is_subset(&analyzer.verified_origins);
    let verdict = if analyzer.uncertain || helper_boundary {
        AuditVerdict::Unknown
    } else if !called || should_panic || all_verified {
        AuditVerdict::Pass
    } else {
        AuditVerdict::Fail
    };
    RuleResult {
        rule: "DA-003".to_owned(),
        verdict,
        reason: match verdict {
            AuditVerdict::Unknown => {
                if helper_boundary {
                    "target result crosses a same-file helper boundary".to_owned()
                } else {
                    "target result crosses a macro, closure, mutation, or control-flow boundary"
                        .to_owned()
                }
            }
            AuditVerdict::Fail => {
                "at least one target result does not reach assert-like syntax".to_owned()
            }
            AuditVerdict::Pass if !called => {
                "target is not called, so result-flow is not applicable".to_owned()
            }
            AuditVerdict::Pass if should_panic => {
                "#[should_panic] verifies the target failure behavior".to_owned()
            }
            AuditVerdict::Pass => "every target result reaches assert-like syntax".to_owned(),
        },
        location: source_location_from_item(item),
    }
}

pub(crate) struct FlowAnalyzer<'a> {
    target_resolution: TargetResolution<'a>,
    target_name: String,
    assertion_macros: &'a [String],
    next_origin: usize,
    all_origins: BTreeSet<usize>,
    verified_origins: BTreeSet<usize>,
    bindings: BTreeMap<String, BTreeSet<usize>>,
    uncertain: bool,
    result_returning: bool,
    bare_shadowed: bool,
}

impl<'a> FlowAnalyzer<'a> {
    pub(crate) fn new(
        target_resolution: TargetResolution<'a>,
        assertion_macros: &'a [String],
        result_returning: bool,
        bare_shadowed: bool,
    ) -> Self {
        let target_name = target_resolution.target_name().to_owned();
        Self {
            target_resolution,
            target_name,
            assertion_macros,
            next_origin: 0,
            all_origins: BTreeSet::new(),
            verified_origins: BTreeSet::new(),
            bindings: BTreeMap::new(),
            uncertain: false,
            result_returning,
            bare_shadowed,
        }
    }

    fn analyze_block(&mut self, block: &syn::Block) {
        for (index, statement) in block.stmts.iter().enumerate() {
            match statement {
                Stmt::Local(local) => {
                    if let Some(init) = &local.init {
                        let origins = self.eval_expr(&init.expr, false);
                        self.bind_pattern(&local.pat, &origins);
                        if let Some((_, diverge)) = &init.diverge {
                            self.eval_expr(diverge, false);
                        }
                    }
                }
                Stmt::Expr(expr, _) => {
                    let is_tail =
                        index + 1 == block.stmts.len() && matches!(statement, Stmt::Expr(_, None));
                    self.eval_expr(expr, self.result_returning && is_tail);
                }
                Stmt::Macro(statement) => {
                    self.eval_macro(&statement.mac, false);
                }
                Stmt::Item(_) => {}
            }
        }
    }

    fn bind_pattern(&mut self, pattern: &Pat, origins: &BTreeSet<usize>) {
        let mut names = Vec::new();
        collect_pattern_names(pattern, &mut names);
        for (name, mutable_or_ref) in names {
            if mutable_or_ref && !origins.is_empty() {
                self.uncertain = true;
            }
            self.bindings.insert(name, origins.clone());
        }
    }

    fn eval_expr(&mut self, expr: &Expr, verifying: bool) -> BTreeSet<usize> {
        let origins = match expr {
            Expr::Path(path) => path
                .path
                .segments
                .last()
                .and_then(|segment| self.bindings.get(&segment.ident.to_string()))
                .cloned()
                .unwrap_or_default(),
            Expr::Call(call) => {
                let mut nested = BTreeSet::new();
                for argument in &call.args {
                    nested.extend(self.eval_expr(argument, verifying));
                }
                let target_match = match call.func.as_ref() {
                    Expr::Path(path) => classify_target_call(
                        &path
                            .path
                            .segments
                            .iter()
                            .map(|segment| segment.ident.to_string())
                            .collect::<Vec<_>>()
                            .join("::"),
                        &self.target_resolution,
                        self.bare_shadowed,
                    ),
                    _ => TargetCallMatch::Absent,
                };
                if target_match == TargetCallMatch::Proven {
                    let origin = self.next_origin;
                    self.next_origin += 1;
                    self.all_origins.insert(origin);
                    nested.insert(origin);
                } else {
                    if target_match == TargetCallMatch::Ambiguous {
                        self.uncertain = true;
                    }
                    let function_origins = self.eval_expr(&call.func, false);
                    if !function_origins.is_empty() || !nested.is_empty() {
                        // Function-call propagation is beyond the supported
                        // local binding/method-chain boundary.
                        self.uncertain = true;
                    }
                    nested.extend(function_origins);
                }
                nested
            }
            Expr::MethodCall(call) => {
                let mut values = self.eval_expr(&call.receiver, false);
                for argument in &call.args {
                    values.extend(self.eval_expr(argument, false));
                }
                if call.method == self.target_name {
                    self.uncertain = true;
                }
                if matches!(call.method.to_string().as_str(), "unwrap" | "expect") {
                    self.verified_origins.extend(values.iter().copied());
                }
                values
            }
            Expr::Try(value) => {
                let values = self.eval_expr(&value.expr, false);
                self.verified_origins.extend(values.iter().copied());
                values
            }
            Expr::Macro(value) => self.eval_macro(&value.mac, verifying),
            Expr::Paren(value) => self.eval_expr(&value.expr, verifying),
            Expr::Group(value) => self.eval_expr(&value.expr, verifying),
            Expr::Reference(value) => {
                let values = self.eval_expr(&value.expr, verifying);
                if value.mutability.is_some() && !values.is_empty() {
                    self.uncertain = true;
                }
                values
            }
            Expr::RawAddr(value) => {
                let values = self.eval_expr(&value.expr, verifying);
                if matches!(value.mutability, syn::PointerMutability::Mut(_)) && !values.is_empty()
                {
                    self.uncertain = true;
                }
                values
            }
            Expr::Unary(value) => self.eval_expr(&value.expr, verifying),
            Expr::Await(value) => self.eval_expr(&value.base, verifying),
            Expr::Cast(value) => self.eval_expr(&value.expr, verifying),
            Expr::Field(value) => self.eval_expr(&value.base, verifying),
            Expr::Binary(value) => {
                let mut values = self.eval_expr(&value.left, verifying);
                values.extend(self.eval_expr(&value.right, verifying));
                values
            }
            Expr::Index(value) => {
                let mut values = self.eval_expr(&value.expr, verifying);
                values.extend(self.eval_expr(&value.index, verifying));
                values
            }
            Expr::Tuple(value) => self.eval_many(value.elems.iter(), verifying),
            Expr::Array(value) => self.eval_many(value.elems.iter(), verifying),
            Expr::Repeat(value) => {
                let mut values = self.eval_expr(&value.expr, verifying);
                values.extend(self.eval_expr(&value.len, verifying));
                values
            }
            Expr::Assign(value) => {
                let values = self.eval_expr(&value.right, verifying);
                if let Expr::Path(path) = value.left.as_ref() {
                    if let Some(name) = path.path.segments.last() {
                        self.bindings.insert(name.ident.to_string(), values.clone());
                    }
                } else if !values.is_empty() {
                    self.uncertain = true;
                }
                values
            }
            Expr::Struct(value) => {
                let mut values = BTreeSet::new();
                for field in &value.fields {
                    values.extend(self.eval_expr(&field.expr, verifying));
                }
                if let Some(rest) = &value.rest {
                    values.extend(self.eval_expr(rest, verifying));
                }
                values
            }
            Expr::Range(value) => {
                let mut values = BTreeSet::new();
                if let Some(start) = &value.start {
                    values.extend(self.eval_expr(start, verifying));
                }
                if let Some(end) = &value.end {
                    values.extend(self.eval_expr(end, verifying));
                }
                values
            }
            Expr::Return(value) => value
                .expr
                .as_deref()
                .map(|expr| self.eval_expr(expr, verifying || self.result_returning))
                .unwrap_or_default(),
            Expr::Break(value) => value
                .expr
                .as_deref()
                .map(|expr| self.eval_expr(expr, verifying))
                .unwrap_or_default(),
            Expr::Yield(value) => value
                .expr
                .as_deref()
                .map(|expr| self.eval_expr(expr, verifying))
                .unwrap_or_default(),
            Expr::Block(_)
            | Expr::If(_)
            | Expr::Match(_)
            | Expr::Closure(_)
            | Expr::Async(_)
            | Expr::ForLoop(_)
            | Expr::Loop(_)
            | Expr::While(_)
            | Expr::TryBlock(_)
            | Expr::Unsafe(_)
            | Expr::Const(_) => {
                if expression_may_depend(
                    expr,
                    &self.target_resolution,
                    &self.target_name,
                    &self.bindings,
                    self.bare_shadowed,
                ) {
                    self.uncertain = true;
                }
                BTreeSet::new()
            }
            _ => BTreeSet::new(),
        };
        if verifying {
            self.verified_origins.extend(origins.iter().copied());
        }
        origins
    }

    fn eval_many<'b>(
        &mut self,
        expressions: impl IntoIterator<Item = &'b Expr>,
        verifying: bool,
    ) -> BTreeSet<usize> {
        let mut values = BTreeSet::new();
        for expression in expressions {
            values.extend(self.eval_expr(expression, verifying));
        }
        values
    }

    fn eval_macro(&mut self, mac: &Macro, verifying: bool) -> BTreeSet<usize> {
        let assertion = is_assertion_macro(mac, self.assertion_macros);
        if !assertion {
            if macro_mentions_dependency(mac, &self.target_name, &self.bindings) {
                self.uncertain = true;
            }
            return BTreeSet::new();
        }
        let Some(arguments) = try_parse_macro_exprs(mac) else {
            if macro_mentions_dependency(mac, &self.target_name, &self.bindings) {
                self.uncertain = true;
            }
            return BTreeSet::new();
        };
        let is_panic = macro_short_name(mac).as_deref() == Some("panic");
        self.eval_many(arguments.iter(), verifying || !is_panic)
    }
}

pub(crate) fn collect_pattern_names(pattern: &Pat, output: &mut Vec<(String, bool)>) {
    match pattern {
        Pat::Ident(value) => output.push((
            value.ident.to_string(),
            value.mutability.is_some() || value.by_ref.is_some(),
        )),
        Pat::Type(value) => collect_pattern_names(&value.pat, output),
        Pat::Reference(value) => collect_pattern_names(&value.pat, output),
        Pat::Paren(value) => collect_pattern_names(&value.pat, output),
        Pat::Tuple(value) => {
            for element in &value.elems {
                collect_pattern_names(element, output);
            }
        }
        Pat::TupleStruct(value) => {
            for element in &value.elems {
                collect_pattern_names(element, output);
            }
        }
        Pat::Slice(value) => {
            for element in &value.elems {
                collect_pattern_names(element, output);
            }
        }
        Pat::Struct(value) => {
            for field in &value.fields {
                collect_pattern_names(&field.pat, output);
            }
        }
        Pat::Or(value) => {
            for case in &value.cases {
                collect_pattern_names(case, output);
            }
        }
        _ => {}
    }
}

pub(crate) fn expression_may_depend(
    expr: &Expr,
    target_resolution: &TargetResolution<'_>,
    target_name: &str,
    bindings: &BTreeMap<String, BTreeSet<usize>>,
    bare_shadowed: bool,
) -> bool {
    let mut visitor = DependencyProbe {
        target_resolution,
        target_name,
        bindings,
        bare_shadowed,
        found: false,
    };
    visitor.visit_expr(expr);
    visitor.found
}

pub(crate) struct DependencyProbe<'a> {
    target_resolution: &'a TargetResolution<'a>,
    target_name: &'a str,
    bindings: &'a BTreeMap<String, BTreeSet<usize>>,
    bare_shadowed: bool,
    found: bool,
}

impl<'ast> Visit<'ast> for DependencyProbe<'_> {
    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Expr::Path(path) = node.func.as_ref() {
            let path = path
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");
            if classify_target_call(&path, self.target_resolution, self.bare_shadowed)
                != TargetCallMatch::Absent
            {
                self.found = true;
            }
        }
        visit::visit_expr_call(self, node);
    }

    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        if node.path.segments.last().is_some_and(|segment| {
            self.bindings
                .get(&segment.ident.to_string())
                .is_some_and(|origins| !origins.is_empty())
        }) {
            self.found = true;
        }
        visit::visit_expr_path(self, node);
    }

    fn visit_macro(&mut self, node: &'ast Macro) {
        if macro_mentions_dependency(node, self.target_name, self.bindings) {
            self.found = true;
        }
    }
}

pub(crate) fn macro_mentions_dependency(
    mac: &Macro,
    target_name: &str,
    bindings: &BTreeMap<String, BTreeSet<usize>>,
) -> bool {
    token_stream_contains_ident(&mac.tokens, target_name)
        || bindings.iter().any(|(name, origins)| {
            !origins.is_empty() && token_stream_contains_ident(&mac.tokens, name)
        })
}

pub(crate) fn token_stream_contains_ident(
    stream: &proc_macro2::TokenStream,
    expected: &str,
) -> bool {
    stream.clone().into_iter().any(|token| match token {
        proc_macro2::TokenTree::Ident(ident) => ident == expected,
        proc_macro2::TokenTree::Group(group) => {
            token_stream_contains_ident(&group.stream(), expected)
        }
        _ => false,
    })
}

pub(crate) fn rule_da004(item: &ItemFn) -> RuleResult {
    let mut visitor = ComparisonVisitor {
        matches: Vec::new(),
    };
    visitor.visit_item_fn(item);
    let matched = visitor.matches.first().copied();
    RuleResult {
        rule: "DA-004".to_owned(),
        verdict: if matched.is_some() {
            AuditVerdict::Fail
        } else {
            AuditVerdict::Pass
        },
        reason: if matched.is_some() {
            "assert_eq! compares syntactically identical operands".to_owned()
        } else {
            "no identical assert_eq! operands found".to_owned()
        },
        location: matched
            .map(|span| source_location_from_span(item, span))
            .unwrap_or_else(|| source_location_from_item(item)),
    }
}

pub(crate) struct ComparisonVisitor {
    matches: Vec<proc_macro2::Span>,
}

impl<'ast> Visit<'ast> for ComparisonVisitor {
    fn visit_macro(&mut self, node: &'ast Macro) {
        if macro_short_name(node).as_deref() == Some("assert_eq") {
            if let Some(arguments) = try_parse_macro_exprs(node) {
                if arguments.len() >= 2
                    && arguments[0].to_token_stream().to_string()
                        == arguments[1].to_token_stream().to_string()
                {
                    self.matches.push(node.span());
                }
            }
        }
        visit::visit_macro(self, node);
    }
}

pub(crate) fn has_assert_like(item: &ItemFn, assertion_macros: &[String]) -> bool {
    let mut assertions = Vec::new();
    collect_assertions(item, assertion_macros, &mut assertions);
    if !assertions.is_empty() || has_attribute(&item.attrs, "should_panic") {
        return true;
    }
    let mut visitor = VerificationSyntaxVisitor { found: false };
    visitor.visit_item_fn(item);
    visitor.found || returns_result_and_can_err(item)
}

pub(crate) struct VerificationSyntaxVisitor {
    found: bool,
}

impl<'ast> Visit<'ast> for VerificationSyntaxVisitor {
    fn visit_expr_try(&mut self, _node: &'ast syn::ExprTry) {
        self.found = true;
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        if matches!(node.method.to_string().as_str(), "unwrap" | "expect") {
            self.found = true;
        }
        visit::visit_expr_method_call(self, node);
    }
}

pub(crate) fn returns_result_and_can_err(item: &ItemFn) -> bool {
    let returns_result = match &item.sig.output {
        ReturnType::Type(_, ty) => match ty.as_ref() {
            Type::Path(path) => path
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "Result"),
            _ => false,
        },
        ReturnType::Default => false,
    };
    if !returns_result {
        return false;
    }
    let mut err = ErrVisitor { found: false };
    err.visit_item_fn(item);
    let mut try_expr = TryExpressionVisitor { found: false };
    try_expr.visit_item_fn(item);
    let mut explicit_return = ResultReturnVisitor { may_err: false };
    explicit_return.visit_item_fn(item);
    let tail_may_err = item.block.stmts.last().is_some_and(
        |statement| matches!(statement, Stmt::Expr(expr, None) if !is_ok_constructor(expr)),
    );
    err.found || try_expr.found || explicit_return.may_err || tail_may_err
}

pub(crate) fn is_ok_constructor(expr: &Expr) -> bool {
    matches!(expr, Expr::Call(call) if matches!(call.func.as_ref(), Expr::Path(path) if path.path.segments.last().is_some_and(|segment| segment.ident == "Ok")))
}

pub(crate) struct ErrVisitor {
    found: bool,
}

impl<'ast> Visit<'ast> for ErrVisitor {
    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Expr::Path(path) = node.func.as_ref() {
            if path
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "Err")
            {
                self.found = true;
            }
        }
        visit::visit_expr_call(self, node);
    }
}

pub(crate) struct TryExpressionVisitor {
    found: bool,
}

impl<'ast> Visit<'ast> for TryExpressionVisitor {
    fn visit_expr_try(&mut self, _node: &'ast syn::ExprTry) {
        self.found = true;
    }
}

pub(crate) struct ResultReturnVisitor {
    may_err: bool,
}

impl<'ast> Visit<'ast> for ResultReturnVisitor {
    fn visit_expr_return(&mut self, node: &'ast syn::ExprReturn) {
        if node
            .expr
            .as_deref()
            .is_some_and(|expr| !is_ok_constructor(expr))
        {
            self.may_err = true;
        }
        visit::visit_expr_return(self, node);
    }
}

pub(crate) fn collect_assertions(
    item: &ItemFn,
    assertion_macros: &[String],
    output: &mut Vec<CollectedAssertion>,
) {
    let mut visitor = AssertionVisitor {
        assertion_macros,
        output,
    };
    visitor.visit_item_fn(item);
}

pub(crate) struct AssertionVisitor<'a> {
    assertion_macros: &'a [String],
    output: &'a mut Vec<CollectedAssertion>,
}

impl<'ast> Visit<'ast> for AssertionVisitor<'_> {
    fn visit_macro(&mut self, node: &'ast Macro) {
        if is_assertion_macro(node, self.assertion_macros) {
            self.output.push(CollectedAssertion {
                name: macro_short_name(node).unwrap_or_default(),
                args: try_parse_macro_exprs(node).unwrap_or_default(),
                span: node.span(),
            });
        }
        visit::visit_macro(self, node);
    }
}

pub(crate) fn is_assertion_macro(mac: &Macro, configured: &[String]) -> bool {
    let short = macro_short_name(mac);
    if short
        .as_deref()
        .is_some_and(|name| matches!(name, "assert" | "assert_eq" | "assert_ne" | "panic"))
    {
        return true;
    }
    let full = mac
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>()
        .join("::");
    configured.iter().any(|name| {
        name == &full
            || (!name.contains("::") && short.as_deref().is_some_and(|short| short == name))
    })
}

pub(crate) fn macro_short_name(mac: &Macro) -> Option<String> {
    mac.path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
}

pub(crate) fn try_parse_macro_exprs(mac: &Macro) -> Option<Vec<Expr>> {
    syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated
        .parse2(mac.tokens.clone())
        .ok()
        .map(|values| values.into_iter().collect())
}

pub(crate) fn find_function<'a>(file: &'a File, item_path: &str) -> Option<&'a ItemFn> {
    fn collect<'a>(items: &'a [Item], prefix: &str, output: &mut Vec<(String, &'a ItemFn)>) {
        for item in items {
            match item {
                Item::Fn(item_fn) => {
                    let name = item_fn.sig.ident.to_string();
                    let path = if prefix.is_empty() {
                        name
                    } else {
                        format!("{prefix}::{name}")
                    };
                    output.push((path, item_fn));
                }
                Item::Mod(item_mod) => {
                    if let Some((_, nested)) = &item_mod.content {
                        let name = item_mod.ident.to_string();
                        let path = if prefix.is_empty() {
                            name
                        } else {
                            format!("{prefix}::{name}")
                        };
                        collect(nested, &path, output);
                    }
                }
                _ => {}
            }
        }
    }
    let mut functions = Vec::new();
    collect(&file.items, "", &mut functions);
    if let Some((_, item)) = functions.iter().find(|(path, _)| path == item_path) {
        return Some(*item);
    }
    // A file-backed `mod foo;` contributes an outer scanner prefix that does
    // not appear in the parsed file itself.  A unique suffix is safe; an
    // ambiguity must not silently select the first same-named function.
    let suffix = format!("::{item_path}");
    let matches = functions
        .iter()
        .filter(|(path, _)| item_path.ends_with(&format!("::{path}")) || path.ends_with(&suffix))
        .map(|(_, item)| *item)
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        Some(matches[0])
    } else {
        None
    }
}

pub(crate) fn function_item_path(file: &File, needle: &ItemFn) -> Option<String> {
    fn find(items: &[Item], prefix: &str, needle: &ItemFn) -> Option<String> {
        for item in items {
            match item {
                Item::Fn(item_fn) if std::ptr::eq(item_fn, needle) => {
                    return Some(if prefix.is_empty() {
                        item_fn.sig.ident.to_string()
                    } else {
                        format!("{prefix}::{}", item_fn.sig.ident)
                    });
                }
                Item::Mod(item_mod) => {
                    if let Some((_, nested)) = &item_mod.content {
                        let nested_prefix = if prefix.is_empty() {
                            item_mod.ident.to_string()
                        } else {
                            format!("{prefix}::{}", item_mod.ident)
                        };
                        if let Some(path) = find(nested, &nested_prefix, needle) {
                            return Some(path);
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    find(&file.items, "", needle)
}

// ---------------------------------------------------------------------------
// StaticAuditAdapter implementation
// ---------------------------------------------------------------------------

use std::fs;
use std::path::Path;

use vtest_adapter_api::AdapterError;
use vtest_adapter_api::{
    RuleObservationDraft, SourceFragment as ApiSourceFragment, StaticAnalysisClosureDraft,
    StaticAuditAdapter, StaticAuditConfigDraft, StaticAuditObservation,
};
use vtest_model::{CanonicalProjection, CheckValue, SrcId, TargetRef, TestEntity};

use crate::discovery::{
    collect_rs_files, join_module_path, line_offsets, make_location, parse_src_id, source_slice,
    Locator,
};
use crate::RUST_CARGO_ADAPTER_ID;

/// Identity of the deterministic Rust rule set. Feeds the config subject so
/// the audit stales when the rule set itself is versioned.
const RULE_SET_ID: &str = "rust-cargo-static-da";
const RULE_SET_VERSION: &str = "1";

/// The built-in `rust-cargo` deterministic static auditor.
#[derive(Default)]
pub struct RustCargoStaticAudit;

/// A filesystem-resolved source construct (target or helper) with the exact
/// bytes and neutral location the core needs to bind a subject.
struct ResolvedSource {
    path: String,
    item_path: String,
    location: SourceLocation,
    bytes: Vec<u8>,
}

fn source_location(location: &SourceLocation, span: proc_macro2::Span) -> SourceLocation {
    let mut location = location.clone();
    location.byte_range.start_line = span.start().line;
    location.byte_range.end_line = span.end().line;
    location
}

fn assertion_macros_from(config: &CanonicalProjection) -> Vec<String> {
    let CanonicalProjection::Map(map) = config else {
        return Vec::new();
    };
    match map.get("assertion_macros") {
        Some(CanonicalProjection::List(items)) => items
            .iter()
            .filter_map(|value| match value {
                CanonicalProjection::String(text) => Some(text.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn verdict_to_check(verdict: AuditVerdict) -> CheckValue {
    match verdict {
        AuditVerdict::Pass => CheckValue::Pass,
        AuditVerdict::Fail => CheckValue::Fail,
        AuditVerdict::Unknown => CheckValue::Unknown,
    }
}

fn relative_path(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Build a `ResolvedSource` from a parsed file and an item path.
fn resolve_item(path: &str, source: &str, item_path: &str) -> Option<ResolvedSource> {
    let syntax = syn::parse_file(source).ok()?;
    let item = find_function(&syntax, item_path)?;
    let offsets = line_offsets(source);
    let location = make_location(path, item_path, item.span(), source, &offsets);
    let bytes = source_slice(source, &location).as_bytes().to_vec();
    Some(ResolvedSource {
        path: path.to_owned(),
        item_path: item_path.to_owned(),
        location,
        bytes,
    })
}

/// Resolve a declared target reference to its source construct on disk. Locator
/// targets read `path::item` directly; permanent SRC IDs are found by walking
/// the source tree for the matching `@vtest.src-id` annotation.
fn resolve_target(root: &Path, target: Option<&TargetRef>) -> Option<ResolvedSource> {
    match target? {
        TargetRef::Locator { adapter, value } if adapter.as_str() == RUST_CARGO_ADAPTER_ID => {
            let locator = Locator::parse(value)?;
            let source = fs::read_to_string(root.join(&locator.path)).ok()?;
            resolve_item(&locator.path, &source, &locator.item_path)
        }
        TargetRef::Locator { .. } => None,
        TargetRef::SrcId(src_id) => resolve_src_id(root, src_id),
    }
}

fn resolve_src_id(root: &Path, src_id: &SrcId) -> Option<ResolvedSource> {
    let mut files = Vec::new();
    let _ = collect_rs_files(root, root, &mut files);
    for file in &files {
        let Ok(source) = fs::read_to_string(file) else {
            continue;
        };
        let Ok(syntax) = syn::parse_file(&source) else {
            continue;
        };
        if let Some(item_path) = find_src_id_item_path(&syntax.items, "", src_id) {
            let relative = relative_path(root, file);
            return resolve_item(&relative, &source, &item_path);
        }
    }
    None
}

fn find_src_id_item_path(items: &[Item], prefix: &str, src_id: &SrcId) -> Option<String> {
    for item in items {
        match item {
            Item::Fn(function) => {
                if parse_src_id(&function.attrs).as_ref() == Some(src_id) {
                    let name = function.sig.ident.to_string();
                    return Some(join_module_path(prefix, &name));
                }
            }
            Item::Mod(module) => {
                if let Some((_, inner)) = &module.content {
                    let child = join_module_path(prefix, &module.ident.to_string());
                    if let Some(found) = find_src_id_item_path(inner, &child, src_id) {
                        return Some(found);
                    }
                }
            }
            _ => {}
        }
    }
    None
}

/// Collect every function in a parsed file with its canonical item path so
/// same-file helpers reached by the Test can be bound as analysis subjects.
fn collect_file_functions<'a>(
    items: &'a [Item],
    prefix: &str,
    output: &mut Vec<(String, &'a ItemFn)>,
) {
    for item in items {
        match item {
            Item::Fn(function) => {
                let name = function.sig.ident.to_string();
                output.push((join_module_path(prefix, &name), function));
            }
            Item::Mod(module) => {
                if let Some((_, inner)) = &module.content {
                    let child = join_module_path(prefix, &module.ident.to_string());
                    collect_file_functions(inner, &child, output);
                }
            }
            _ => {}
        }
    }
}

impl StaticAuditAdapter for RustCargoStaticAudit {
    fn audit(
        &self,
        root: &Path,
        config: &CanonicalProjection,
        test: &TestEntity,
    ) -> Result<StaticAuditObservation, AdapterError> {
        let test_path = test.location.path.as_str().to_owned();
        let source = fs::read_to_string(root.join(&test_path))
            .map_err(|error| AdapterError::Operation(format!("read {test_path}: {error}")))?;
        let syntax = syn::parse_file(&source)
            .map_err(|error| AdapterError::Operation(format!("parse {test_path}: {error}")))?;
        let item = find_function(&syntax, test.location.locator.as_str()).ok_or_else(|| {
            AdapterError::Operation(format!(
                "test `{}` not found in {test_path}",
                test.location.locator
            ))
        })?;

        let assertion_macros = assertion_macros_from(config);
        let offsets = line_offsets(&source);

        let resolved_target = resolve_target(root, test.targets.first());
        let target_resolution = resolved_target.as_ref().map(|resolved| {
            TargetResolution::new(
                &resolved.item_path,
                test.location.locator.as_str(),
                resolved.path == test_path,
            )
        });

        let has_assert = has_assert_like(item, &assertion_macros);
        let ignored = has_attribute(&item.attrs, "ignore");

        let mut rules = vec![
            rule_da001(item, &syntax, &assertion_macros, target_resolution.as_ref()),
            rule_da002(item, &syntax, target_resolution.as_ref(), &assertion_macros),
            rule_da003(item, &syntax, target_resolution.as_ref(), &assertion_macros),
            rule_da004(item),
            RuleResult {
                rule: "DA-005".to_owned(),
                verdict: if item.block.stmts.is_empty() {
                    AuditVerdict::Fail
                } else {
                    AuditVerdict::Pass
                },
                reason: if item.block.stmts.is_empty() {
                    "test body contains no statements".to_owned()
                } else {
                    "test body contains statements".to_owned()
                },
                location: source_location(&test.location, item.span()),
            },
            RuleResult {
                rule: "DA-006".to_owned(),
                verdict: if has_assert {
                    AuditVerdict::Pass
                } else {
                    AuditVerdict::Fail
                },
                reason: if has_assert {
                    "assert-like syntax is present".to_owned()
                } else {
                    "no assert-like syntax is present".to_owned()
                },
                location: source_location(&test.location, item.span()),
            },
        ];
        if ignored {
            rules.push(RuleResult {
                rule: "W-DA-101".to_owned(),
                verdict: AuditVerdict::Pass,
                reason: "test is marked #[ignore]; execution may be NOT_EXECUTED".to_owned(),
                location: source_location(&test.location, item.span()),
            });
        }

        for rule in &mut rules {
            if rule.location.path.as_str().is_empty() {
                rule.location.path = test.location.path.clone();
            }
            if rule.location.locator.is_empty() || item.sig.ident == rule.location.locator {
                rule.location.locator = test.location.locator.clone();
            }
            if rule.location.byte_range.end == 0 {
                rule.location.byte_range.start = test.location.byte_range.start;
                rule.location.byte_range.end = test.location.byte_range.end;
            }
        }

        let verdict = if rules.iter().any(|rule| rule.verdict == AuditVerdict::Fail) {
            AuditVerdict::Fail
        } else if rules
            .iter()
            .any(|rule| rule.verdict == AuditVerdict::Unknown)
        {
            AuditVerdict::Unknown
        } else {
            AuditVerdict::Pass
        };

        // Analysis closure: the Test construct, its resolved target, and every
        // same-file helper the Test directly reaches. Editing any of these must
        // invalidate the conclusion, so each is returned as a subject fragment.
        let mut sources = Vec::new();
        let test_location = make_location(
            &test_path,
            test.location.locator.as_str(),
            item.span(),
            &source,
            &offsets,
        );
        sources.push(ApiSourceFragment {
            bytes: source_slice(&source, &test_location).as_bytes().to_vec(),
            location: test_location,
        });
        if let Some(resolved) = &resolved_target {
            sources.push(ApiSourceFragment {
                location: resolved.location.clone(),
                bytes: resolved.bytes.clone(),
            });
        }
        let helper_names = call_facts(item, &assertion_macros).names;
        let mut file_functions = Vec::new();
        collect_file_functions(&syntax.items, "", &mut file_functions);
        for (item_path, function) in &file_functions {
            let last = item_path.rsplit("::").next().unwrap_or(item_path);
            if !helper_names.contains(last) || item_path == test.location.locator.as_str() {
                continue;
            }
            let location = make_location(&test_path, item_path, function.span(), &source, &offsets);
            if sources
                .iter()
                .any(|fragment| fragment.location.locator == location.locator)
            {
                continue;
            }
            sources.push(ApiSourceFragment {
                bytes: source_slice(&source, &location).as_bytes().to_vec(),
                location,
            });
        }

        let reasons: Vec<String> = {
            let non_pass: Vec<String> = rules
                .iter()
                .filter(|rule| rule.verdict != AuditVerdict::Pass)
                .map(|rule| format!("{}: {}", rule.rule, rule.reason))
                .collect();
            if non_pass.is_empty() {
                vec!["all deterministic static rules pass".to_owned()]
            } else {
                non_pass
            }
        };

        let rule_observations = rules
            .into_iter()
            .map(|rule| RuleObservationDraft {
                rule: rule.rule,
                verdict: verdict_to_check(rule.verdict),
                reason: rule.reason,
                location: rule.location,
            })
            .collect();

        Ok(StaticAuditObservation {
            verdict: verdict_to_check(verdict),
            reasons,
            rules: rule_observations,
            config: StaticAuditConfigDraft {
                rule_set_id: RULE_SET_ID.to_owned(),
                rule_set_version: RULE_SET_VERSION.to_owned(),
                effective_config: config.clone(),
            },
            analysis: StaticAnalysisClosureDraft {
                // The analysis input closure is incomplete when a target is
                // declared but cannot be resolved on disk; the core forbids a
                // PASS over an incomplete closure.
                complete: test.targets.is_empty() || resolved_target.is_some(),
                sources,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::Item;

    fn item(source: &str) -> ItemFn {
        match syn::parse_file(source)
            .unwrap()
            .items
            .into_iter()
            .next()
            .unwrap()
        {
            Item::Fn(item) => item,
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn constant_assertion_is_a_deterministic_failure() {
        let file =
            syn::parse_file("const YES: bool = true; #[test] fn x() { assert!(YES); }").unwrap();
        let item = find_function(&file, "x").unwrap();
        assert_eq!(
            rule_da001(
                item,
                &file,
                &[],
                Some(&TargetResolution::new("target", "x", true)),
            )
            .verdict,
            AuditVerdict::Fail
        );
        assert!(has_assert_like(item, &[]));
    }

    #[test]
    fn target_call_in_assertion_passes_call_and_result_rules() {
        let file = syn::parse_file("#[test] fn x() { assert_eq!(add(1, 2), 3); }").unwrap();
        let item = find_function(&file, "x").unwrap();
        assert_eq!(
            rule_da002(
                item,
                &file,
                Some(&TargetResolution::new("add", "x", true)),
                &[],
            )
            .verdict,
            AuditVerdict::Pass
        );
        assert_eq!(
            rule_da001(
                item,
                &file,
                &[],
                Some(&TargetResolution::new("add", "x", true)),
            )
            .verdict,
            AuditVerdict::Pass
        );
        assert_eq!(
            rule_da003(
                item,
                &file,
                Some(&TargetResolution::new("add", "x", true)),
                &[],
            )
            .verdict,
            AuditVerdict::Pass
        );
    }

    /// @vtest.id TEST-DOGFOOD-M3-TARGET-RULES
    /// @vtest.covers VO-DOGFOOD-M3-STATIC-AUDIT
    /// @vtest.target crates/vtest-audit/src/lib.rs::classify_target_call
    /// @vtest.intent exact target paths are distinguished from ambiguous same-name calls
    #[test]
    fn dogfood_exact_target_path_is_classified() {
        let resolution = TargetResolution::new("module::target", "tests::dogfood", true);
        assert_eq!(
            classify_target_call("crate::module::target", &resolution, false),
            TargetCallMatch::Proven
        );
    }

    #[test]
    fn empty_and_self_comparing_tests_fail_their_rules() {
        let empty = item("#[test] fn x() {}");
        assert!(empty.block.stmts.is_empty());
        let self_compare = item("#[test] fn x() { let a = 1; assert_eq!(a, a); }");
        assert_eq!(rule_da004(&self_compare).verdict, AuditVerdict::Fail);
    }

    #[test]
    fn result_flow_never_uses_identifier_substrings_as_proof() {
        let file = syn::parse_file(
            "#[test] fn x() { let _ = target(); let target_called = true; assert!(target_called); }",
        )
        .unwrap();
        let item = find_function(&file, "x").unwrap();
        assert_eq!(
            rule_da003(
                item,
                &file,
                Some(&TargetResolution::new("target", "x", true)),
                &[],
            )
            .verdict,
            AuditVerdict::Fail
        );
    }

    #[test]
    fn same_file_helper_is_followed_once_and_external_call_is_unknown() {
        let helper_file = syn::parse_file(
            "fn helper() { target(); } #[test] fn x() { helper(); assert!(true); }",
        )
        .unwrap();
        let test = find_function(&helper_file, "x").unwrap();
        assert_eq!(
            rule_da002(
                test,
                &helper_file,
                Some(&TargetResolution::new("target", "x", true)),
                &[],
            )
            .verdict,
            AuditVerdict::Pass
        );

        let external_file = syn::parse_file("#[test] fn x() { other(); assert!(true); }").unwrap();
        let test = find_function(&external_file, "x").unwrap();
        assert_eq!(
            rule_da002(
                test,
                &external_file,
                Some(&TargetResolution::new("target", "x", false)),
                &[],
            )
            .verdict,
            AuditVerdict::Unknown
        );

        assert_eq!(
            rule_da003(
                test,
                &external_file,
                Some(&TargetResolution::new("target", "x", false)),
                &[],
            )
            .verdict,
            AuditVerdict::Pass,
            "DA-002 owns target reachability for an unrelated external call"
        );
    }

    #[test]
    fn qualified_homonym_never_proves_the_declared_target_call() {
        let file = syn::parse_file("#[test] fn x() { assert_eq!(Other::known(), 1); }").unwrap();
        let test = find_function(&file, "x").unwrap();
        assert_eq!(
            rule_da002(
                test,
                &file,
                Some(&TargetResolution::new("known", "x", true)),
                &[],
            )
            .verdict,
            AuditVerdict::Unknown
        );
        assert_eq!(
            rule_da003(
                test,
                &file,
                Some(&TargetResolution::new("known", "x", true)),
                &[],
            )
            .verdict,
            AuditVerdict::Unknown
        );
    }

    #[test]
    fn bare_import_alias_and_local_shadow_never_prove_the_target() {
        let imported =
            syn::parse_file("#[test] fn x() { use crate::other::known; assert_eq!(known(), 1); }")
                .unwrap();
        let test = find_function(&imported, "x").unwrap();
        let nested_target = TargetResolution::new("real::known", "x", true);
        assert_eq!(
            rule_da002(test, &imported, Some(&nested_target), &[]).verdict,
            AuditVerdict::Unknown
        );
        assert_eq!(
            rule_da003(test, &imported, Some(&nested_target), &[]).verdict,
            AuditVerdict::Unknown
        );

        let shadowed =
            syn::parse_file("#[test] fn x() { let known = || 1; assert_eq!(known(), 1); }")
                .unwrap();
        let test = find_function(&shadowed, "x").unwrap();
        let root_target = TargetResolution::new("known", "x", true);
        assert_eq!(
            rule_da002(test, &shadowed, Some(&root_target), &[]).verdict,
            AuditVerdict::Unknown
        );
        assert_eq!(
            rule_da003(test, &shadowed, Some(&root_target), &[]).verdict,
            AuditVerdict::Unknown
        );

        let nested_item =
            syn::parse_file("#[test] fn x() { fn known() {} assert_eq!(known(), ()); }").unwrap();
        let test = find_function(&nested_item, "x").unwrap();
        assert_eq!(
            rule_da002(test, &nested_item, Some(&root_target), &[]).verdict,
            AuditVerdict::Unknown
        );
        assert_eq!(
            rule_da003(test, &nested_item, Some(&root_target), &[]).verdict,
            AuditVerdict::Unknown
        );

        let callable_const = syn::parse_file(
            "#[test] fn x() { const known: fn() = other; assert_eq!(known(), ()); }",
        )
        .unwrap();
        let test = find_function(&callable_const, "x").unwrap();
        assert_eq!(
            rule_da002(test, &callable_const, Some(&root_target), &[]).verdict,
            AuditVerdict::Unknown
        );
    }

    #[test]
    fn helper_target_result_flow_is_unknown_not_pass() {
        let file = syn::parse_file(
            "fn helper() -> i32 { let _ = known(); 7 } #[test] fn x() { assert_eq!(helper(), 7); }",
        )
        .unwrap();
        let test = find_function(&file, "x").unwrap();
        assert_eq!(
            rule_da002(
                test,
                &file,
                Some(&TargetResolution::new("known", "x", true)),
                &[],
            )
            .verdict,
            AuditVerdict::Pass
        );
        assert_eq!(
            rule_da003(
                test,
                &file,
                Some(&TargetResolution::new("known", "x", true)),
                &[],
            )
            .verdict,
            AuditVerdict::Unknown
        );
    }

    #[test]
    fn nested_comma_self_comparison_and_configured_macro_are_recognized() {
        let file = syn::parse_file(
            "#[test] fn x() { check_it!(target()); assert_eq!(f(1, 2), f(1, 2), \"context\"); }",
        )
        .unwrap();
        let item = find_function(&file, "x").unwrap();
        assert_eq!(rule_da004(item).verdict, AuditVerdict::Fail);
        assert!(has_assert_like(item, &["check_it".to_owned()]));
        assert_eq!(
            rule_da003(
                item,
                &file,
                Some(&TargetResolution::new("target", "x", true)),
                &["check_it".to_owned()],
            )
            .verdict,
            AuditVerdict::Pass
        );
    }

    #[test]
    fn exact_module_path_prevents_same_name_test_mixup() {
        let file = syn::parse_file(
            "mod a { #[test] fn checks() { assert!(true); } } mod b { #[test] fn checks() { assert!(false); } }",
        )
        .unwrap();
        let a = find_function(&file, "a::checks").unwrap();
        let b = find_function(&file, "b::checks").unwrap();
        assert!(!std::ptr::eq(a, b));
        assert!(find_function(&file, "checks").is_none());
    }

    #[test]
    fn unwrap_try_and_result_err_are_assert_like() {
        assert!(has_assert_like(
            &item("#[test] fn x() { target().unwrap(); }"),
            &[]
        ));
        assert!(has_assert_like(
            &item("#[test] fn x() -> Result<(), E> { target()?; Ok(()) }"),
            &[]
        ));
        assert!(has_assert_like(
            &item("#[test] fn x() -> Result<(), E> { if bad() { return Err(E); } Ok(()) }"),
            &[]
        ));

        let returned = syn::parse_file("#[test] fn x() -> Result<(), E> { target() }").unwrap();
        let test = find_function(&returned, "x").unwrap();
        assert!(has_assert_like(test, &[]));
        assert_eq!(
            rule_da003(
                test,
                &returned,
                Some(&TargetResolution::new("target", "x", true)),
                &[],
            )
            .verdict,
            AuditVerdict::Pass
        );

        let discarded =
            syn::parse_file("#[test] fn x() -> Result<(), E> { target(); Ok(()) }").unwrap();
        let test = find_function(&discarded, "x").unwrap();
        assert_eq!(
            rule_da003(
                test,
                &discarded,
                Some(&TargetResolution::new("target", "x", true)),
                &[],
            )
            .verdict,
            AuditVerdict::Fail
        );
    }
}
