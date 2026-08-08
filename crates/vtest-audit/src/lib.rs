//! Deterministic static audit rules (M3).

use std::{collections::BTreeSet, fs, path::Path};

use serde::Serialize;
use syn::{
    parse::Parser,
    spanned::Spanned,
    visit::{self, Visit},
    Attribute, Expr, ExprCall, ExprMethodCall, File, Item, ItemFn, Macro,
};
use thiserror::Error;
use vtest_model::{ContentHash, Diagnostic, SourceLocation, TargetRef, TestEntity};
use vtest_scan::ScanResult;
use vtest_store::{new_record_id, now_rfc3339, write_atomic, VerifyLayout};

#[derive(Debug, Error)]
pub enum AuditError {
    #[error("failed to read {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse {path}: {source}")]
    Parse {
        path: std::path::PathBuf,
        source: syn::Error,
    },
    #[error("test `{0}` was not found")]
    TestNotFound(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AuditVerdict {
    Pass,
    Fail,
    Unknown,
}

#[derive(Clone, Debug, Serialize)]
pub struct RuleResult {
    pub rule: String,
    pub verdict: AuditVerdict,
    pub reason: String,
    pub location: SourceLocation,
}

#[derive(Clone, Debug, Serialize)]
pub struct StaticAudit {
    pub id: String,
    pub test_id: String,
    pub subject_hash: ContentHash,
    pub verdict: AuditVerdict,
    pub rules: Vec<RuleResult>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, Serialize)]
pub struct StaticAuditSummary {
    pub audits: Vec<StaticAudit>,
}

#[derive(Clone, Debug)]
pub struct AuditOptions {
    pub test_id: Option<String>,
}

pub fn audit_static(
    root: &Path,
    scan: &ScanResult,
    options: &AuditOptions,
) -> Result<StaticAuditSummary, AuditError> {
    let tests = scan
        .tests
        .iter()
        .filter(|test| {
            options
                .test_id
                .as_deref()
                .is_none_or(|id| id == test.id.as_str())
        })
        .collect::<Vec<_>>();
    if let Some(test_id) = &options.test_id {
        if tests.is_empty() {
            return Err(AuditError::TestNotFound(test_id.clone()));
        }
    }
    let mut audits = Vec::new();
    for test in tests {
        audits.push(audit_one(root, scan, test)?);
    }
    Ok(StaticAuditSummary { audits })
}

pub fn persist_static_audits(
    layout: &VerifyLayout,
    summary: &StaticAuditSummary,
) -> Result<(), AuditError> {
    fs::create_dir_all(layout.audits_dir()).map_err(|source| AuditError::Io {
        path: layout.audits_dir(),
        source,
    })?;
    for audit in &summary.audits {
        let path = layout.audits_dir().join(format!("{}.yaml", audit.id));
        let mut output = format!(
            "id: {}\nkind: static\ntest_id: {}\ntest_hash: {}\nverdict: {}\nreasons:\n",
            yaml_scalar(&audit.id),
            yaml_scalar(&audit.test_id),
            yaml_scalar(audit.subject_hash.as_str()),
            yaml_scalar(&format_verdict(audit.verdict)),
        );
        for rule in &audit.rules {
            output.push_str(&format!(
                "  - rule: {}\n    verdict: {}\n    claim: {}\n    basis: {}:{}\n",
                yaml_scalar(&rule.rule),
                yaml_scalar(&format_verdict(rule.verdict)),
                yaml_scalar(&rule.reason),
                yaml_scalar(&rule.location.file),
                rule.location.start_line,
            ));
        }
        output.push_str("auditor:\n  kind: deterministic\n  id: vtest\naudited_at: ");
        output.push_str(&yaml_scalar(&now_rfc3339()));
        output.push('\n');
        write_atomic(&path, &output).map_err(|error| AuditError::Io {
            path,
            source: std::io::Error::other(error.to_string()),
        })?;
    }
    Ok(())
}

fn audit_one(root: &Path, scan: &ScanResult, test: &TestEntity) -> Result<StaticAudit, AuditError> {
    let path = root.join(&test.location.file);
    let source = fs::read_to_string(&path).map_err(|source| AuditError::Io {
        path: path.clone(),
        source,
    })?;
    let syntax = syn::parse_file(&source).map_err(|source| AuditError::Parse {
        path: path.clone(),
        source,
    })?;
    let item = find_function(&syntax, &test.location.function)
        .ok_or_else(|| AuditError::TestNotFound(test.id.to_string()))?;
    let target_name = target_name(&test.target);
    let target_file = target_file(scan, &test.target);
    let mut rules = Vec::new();
    let has_assert = has_assert_like(item, &[]);
    let ignored = has_attribute(&item.attrs, "ignore");
    rules.push(rule_da001(item, has_assert));
    rules.push(rule_da002(
        item,
        target_name.as_deref(),
        target_file.as_deref(),
        &test.location.file,
    ));
    rules.push(rule_da003(item, target_name.as_deref()));
    rules.push(rule_da004(item));
    rules.push(RuleResult {
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
    });
    rules.push(RuleResult {
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
    });
    if ignored {
        rules.push(RuleResult {
            rule: "W-DA-101".to_owned(),
            verdict: AuditVerdict::Pass,
            reason: "test is marked #[ignore]; execution may be NOT_EXECUTED".to_owned(),
            location: source_location(&test.location, item.span()),
        });
    }
    for rule in &mut rules {
        if rule.location.file.is_empty() {
            rule.location = source_location(&test.location, item.span());
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
    Ok(StaticAudit {
        id: new_record_id(),
        test_id: test.id.to_string(),
        subject_hash: test.content_hash.clone(),
        verdict,
        rules,
        diagnostics: Vec::new(),
    })
}

fn rule_da001(item: &ItemFn, has_assert: bool) -> RuleResult {
    let mut assertions = Vec::new();
    collect_assertions(item, &mut assertions);
    let all_constant = has_assert
        && assertions.iter().all(|(name, args)| {
            !matches!(name.as_str(), "panic") && args.iter().all(is_constant_expr)
        });
    RuleResult {
        rule: "DA-001".to_owned(),
        verdict: if all_constant {
            AuditVerdict::Fail
        } else {
            AuditVerdict::Pass
        },
        reason: if all_constant {
            "all assert-like arguments are literal or constant expressions".to_owned()
        } else {
            "assert-like arguments are not all constant".to_owned()
        },
        location: source_location_from_item(item),
    }
}

fn rule_da002(
    item: &ItemFn,
    target_name: Option<&str>,
    target_file: Option<&str>,
    test_file: &str,
) -> RuleResult {
    let mut calls = CallVisitor::default();
    calls.visit_block(&item.block);
    let called = target_name.is_some_and(|name| {
        calls.names.iter().any(|value| value == name) || macro_contains_target(item, name)
    });
    let has_other_calls = calls
        .names
        .iter()
        .any(|value| target_name != Some(value.as_str()));
    let verdict = if called {
        AuditVerdict::Pass
    } else if target_file.is_some_and(|file| file != test_file) && has_other_calls {
        AuditVerdict::Unknown
    } else {
        AuditVerdict::Fail
    };
    RuleResult {
        rule: "DA-002".to_owned(),
        verdict,
        reason: match verdict {
            AuditVerdict::Pass => "declared target is called directly".to_owned(),
            AuditVerdict::Fail => {
                "declared target call is absent and no indirect call is visible".to_owned()
            }
            AuditVerdict::Unknown => {
                "other-file call prevents ruling out an indirect target call".to_owned()
            }
        },
        location: source_location_from_item(item),
    }
}

fn rule_da003(item: &ItemFn, target_name: Option<&str>) -> RuleResult {
    let mut calls = CallVisitor::default();
    calls.visit_block(&item.block);
    let called = target_name.is_some_and(|name| {
        calls.names.iter().any(|value| value == name) || macro_contains_target(item, name)
    });
    let should_panic = has_attribute(&item.attrs, "should_panic");
    let verified = target_name.is_some_and(|name| assertion_verifies_target(item, name));
    let verdict = if !called || should_panic || verified {
        AuditVerdict::Pass
    } else {
        AuditVerdict::Fail
    };
    RuleResult {
        rule: "DA-003".to_owned(),
        verdict,
        reason: if !called {
            "target is not called, so result-flow rule is not applicable".to_owned()
        } else if should_panic {
            "#[should_panic] verifies the failure behavior".to_owned()
        } else if verified {
            "assert-like syntax uses the target result".to_owned()
        } else {
            "target is called but no assert-like syntax verifies its result".to_owned()
        },
        location: source_location_from_item(item),
    }
}

fn assertion_verifies_target(item: &ItemFn, target_name: &str) -> bool {
    let mut visitor = TargetAssertionVisitor {
        target_name,
        bindings: BTreeSet::new(),
        verified: false,
    };
    visitor.visit_item_fn(item);
    visitor.verified
}

fn macro_contains_target(item: &ItemFn, target_name: &str) -> bool {
    let mut visitor = MacroTargetVisitor {
        target_name,
        found: false,
    };
    visitor.visit_item_fn(item);
    visitor.found
}

fn rule_da004(item: &ItemFn) -> RuleResult {
    let mut comparisons = Vec::new();
    let mut visitor = MacroVisitor {
        comparisons: &mut comparisons,
    };
    visitor.visit_item_fn(item);
    let self_compare = comparisons.iter().any(|(left, right)| left == right);
    RuleResult {
        rule: "DA-004".to_owned(),
        verdict: if self_compare {
            AuditVerdict::Fail
        } else {
            AuditVerdict::Pass
        },
        reason: if self_compare {
            "assert_eq! compares identical token sequences".to_owned()
        } else {
            "no identical assert_eq! operands found".to_owned()
        },
        location: source_location_from_item(item),
    }
}

fn has_assert_like(item: &ItemFn, assertion_macros: &[String]) -> bool {
    let mut assertions = Vec::new();
    collect_assertions(item, &mut assertions);
    if !assertions.is_empty() || has_attribute(&item.attrs, "should_panic") {
        return true;
    }
    let mut visitor = TryVisitor { found: false };
    visitor.visit_item_fn(item);
    if visitor.found {
        return true;
    }
    assertion_macros.iter().any(|name| {
        let mut visitor = ConfiguredMacroVisitor { name, found: false };
        visitor.visit_item_fn(item);
        visitor.found
    })
}

fn collect_assertions(item: &ItemFn, output: &mut Vec<(String, Vec<Expr>)>) {
    let mut visitor = AssertionVisitor { output };
    visitor.visit_item_fn(item);
}

fn is_constant_expr(expr: &Expr) -> bool {
    matches!(expr, Expr::Lit(_) | Expr::Path(_))
}

struct AssertionVisitor<'a> {
    output: &'a mut Vec<(String, Vec<Expr>)>,
}

impl<'ast> Visit<'ast> for AssertionVisitor<'_> {
    fn visit_macro(&mut self, node: &'ast Macro) {
        let Some(name) = node
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
        else {
            return;
        };
        if matches!(
            name.as_str(),
            "assert" | "assert_eq" | "assert_ne" | "panic"
        ) {
            let args = parse_macro_exprs(node);
            self.output.push((name, args));
        }
        visit::visit_macro(self, node);
    }
}

struct MacroVisitor<'a> {
    comparisons: &'a mut Vec<(String, String)>,
}

impl<'ast> Visit<'ast> for MacroVisitor<'_> {
    fn visit_macro(&mut self, node: &'ast Macro) {
        let Some(name) = node
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
        else {
            return;
        };
        if name == "assert_eq" {
            let tokens = node.tokens.to_string();
            let mut pieces = tokens.splitn(2, ',').map(str::trim);
            if let (Some(left), Some(right)) = (pieces.next(), pieces.next()) {
                self.comparisons.push((left.to_owned(), right.to_owned()));
            }
        }
        visit::visit_macro(self, node);
    }
}

#[derive(Default)]
struct CallVisitor {
    names: Vec<String>,
}

struct TargetAssertionVisitor<'a> {
    target_name: &'a str,
    bindings: BTreeSet<String>,
    verified: bool,
}

struct MacroTargetVisitor<'a> {
    target_name: &'a str,
    found: bool,
}

impl<'ast> Visit<'ast> for MacroTargetVisitor<'_> {
    fn visit_macro(&mut self, node: &'ast Macro) {
        let tokens = node.tokens.to_string();
        let target = self.target_name;
        if tokens.contains(&format!("{target} (")) || tokens.contains(&format!("{target}(")) {
            self.found = true;
        }
        visit::visit_macro(self, node);
    }
}

impl<'ast> Visit<'ast> for TargetAssertionVisitor<'_> {
    fn visit_local(&mut self, node: &'ast syn::Local) {
        if let Some(init) = &node.init {
            if let Expr::Call(call) = init.expr.as_ref() {
                if let Expr::Path(path) = call.func.as_ref() {
                    let is_target = path
                        .path
                        .segments
                        .last()
                        .is_some_and(|segment| segment.ident == self.target_name);
                    if is_target {
                        if let syn::Pat::Ident(pattern) = &node.pat {
                            self.bindings.insert(pattern.ident.to_string());
                        }
                    }
                }
            }
        }
        visit::visit_local(self, node);
    }

    fn visit_macro(&mut self, node: &'ast Macro) {
        let Some(name) = node
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
        else {
            return;
        };
        if matches!(
            name.as_str(),
            "assert" | "assert_eq" | "assert_ne" | "panic"
        ) {
            let tokens = node.tokens.to_string();
            if tokens.contains(self.target_name)
                || self.bindings.iter().any(|binding| tokens.contains(binding))
            {
                self.verified = true;
            }
        }
        visit::visit_macro(self, node);
    }
}

impl<'ast> Visit<'ast> for CallVisitor {
    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Expr::Path(path) = node.func.as_ref() {
            if let Some(name) = path.path.segments.last() {
                self.names.push(name.ident.to_string());
            }
        }
        visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        self.names.push(node.method.to_string());
        visit::visit_expr_method_call(self, node);
    }
}

struct TryVisitor {
    found: bool,
}

impl<'ast> Visit<'ast> for TryVisitor {
    fn visit_expr_try(&mut self, _node: &'ast syn::ExprTry) {
        self.found = true;
    }
}

struct ConfiguredMacroVisitor<'a> {
    name: &'a str,
    found: bool,
}

impl<'ast> Visit<'ast> for ConfiguredMacroVisitor<'_> {
    fn visit_macro(&mut self, node: &'ast Macro) {
        if node
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == self.name)
        {
            self.found = true;
        }
        visit::visit_macro(self, node);
    }
}

fn parse_macro_exprs(mac: &Macro) -> Vec<Expr> {
    syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated
        .parse2(mac.tokens.clone())
        .map(|values| values.into_iter().collect())
        .unwrap_or_default()
}

fn find_function<'a>(file: &'a File, item_path: &str) -> Option<&'a ItemFn> {
    let name = item_path.split("::").last()?;
    find_function_in_items(&file.items, name)
}

fn find_function_in_items<'a>(items: &'a [Item], name: &str) -> Option<&'a ItemFn> {
    for item in items {
        match item {
            Item::Fn(item_fn) if item_fn.sig.ident == name => return Some(item_fn),
            Item::Mod(item_mod) => {
                if let Some((_, items)) = &item_mod.content {
                    if let Some(item_fn) = find_function_in_items(items, name) {
                        return Some(item_fn);
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn target_name(target: &TargetRef) -> Option<String> {
    match target {
        TargetRef::Locator(locator) => locator.item_path.split("::").last().map(str::to_owned),
        TargetRef::SrcId(_) => None,
    }
}

fn target_file(scan: &ScanResult, target: &TargetRef) -> Option<String> {
    scan.sources.iter().find_map(|source| match target {
        TargetRef::Locator(locator) if source.locator == *locator => {
            Some(source.location.file.clone())
        }
        TargetRef::SrcId(src_id) if source.src_id.as_ref() == Some(src_id) => {
            Some(source.location.file.clone())
        }
        _ => None,
    })
}

fn has_attribute(attrs: &[Attribute], name: &str) -> bool {
    attrs.iter().any(|attribute| {
        attribute
            .path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == name)
    })
}

fn source_location(location: &SourceLocation, span: proc_macro2::Span) -> SourceLocation {
    let mut location = location.clone();
    location.start_line = span.start().line;
    location.end_line = span.end().line;
    location
}

fn source_location_from_item(item: &ItemFn) -> SourceLocation {
    let span = item.span();
    SourceLocation {
        file: String::new(),
        function: item.sig.ident.to_string(),
        start_line: span.start().line,
        end_line: span.end().line,
        start_byte: 0,
        end_byte: 0,
    }
}

fn format_verdict(verdict: AuditVerdict) -> String {
    match verdict {
        AuditVerdict::Pass => "PASS",
        AuditVerdict::Fail => "FAIL",
        AuditVerdict::Unknown => "UNKNOWN",
    }
    .to_owned()
}

fn yaml_scalar(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let item = item("#[test] fn x() { assert!(true); }");
        assert_eq!(rule_da001(&item, true).verdict, AuditVerdict::Fail);
        assert!(has_assert_like(&item, &[]));
    }

    #[test]
    fn target_call_in_assertion_passes_call_and_result_rules() {
        let item = item("#[test] fn x() { assert_eq!(add(1, 2), 3); }");
        assert_eq!(
            rule_da002(&item, Some("add"), Some("src/lib.rs"), "tests/t.rs").verdict,
            AuditVerdict::Pass
        );
        assert_eq!(rule_da003(&item, Some("add")).verdict, AuditVerdict::Pass);
    }

    #[test]
    fn empty_and_self_comparing_tests_fail_their_rules() {
        let empty = item("#[test] fn x() {}");
        assert!(empty.block.stmts.is_empty());
        let self_compare = item("#[test] fn x() { let a = 1; assert_eq!(a, a); }");
        assert_eq!(rule_da004(&self_compare).verdict, AuditVerdict::Fail);
    }
}
