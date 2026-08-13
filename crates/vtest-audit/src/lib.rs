//! Deterministic static audit rules (M3).

use std::{fs, path::Path, process::Command};

use serde::Serialize;
use syn::{spanned::Spanned, Attribute, ItemFn};
use thiserror::Error;
use vtest_model::{ContentHash, Diagnostic, Revision, SourceLocation, TargetRef, TestEntity};
use vtest_scan::ScanResult;
use vtest_store::{
    load_config, new_record_id, now_rfc3339, write_new_record, AuditBasisRecord, AuditReasonRecord,
    AuditRecord, AuditSubjectRecord, AuditorRecord, StoreError, VerifyLayout,
};

mod audit_rules;
use audit_rules::*;

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
    #[error("store error: {0}")]
    Store(#[from] StoreError),
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
    pub subjects: Vec<AuditSubjectRecord>,
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
    let config = load_config(root)?;
    let config_path = VerifyLayout::new(root).config();
    let config_text = fs::read_to_string(&config_path).map_err(|source| AuditError::Io {
        path: config_path,
        source,
    })?;
    let config_hash = ContentHash::from_text(&config_text);
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
        audits.push(audit_one(
            root,
            scan,
            test,
            &config.rust_cargo().scan.assertion_macros,
            &config_hash,
        )?);
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
        let record = AuditRecord {
            id: audit.id.clone(),
            kind: "static".to_owned(),
            bundle_id: None,
            subjects: audit.subjects.clone(),
            verdict: format_verdict(audit.verdict),
            reasons: audit
                .rules
                .iter()
                .map(|rule| AuditReasonRecord {
                    rule: Some(rule.rule.clone()),
                    verdict: Some(format_verdict(rule.verdict)),
                    claim: rule.reason.clone(),
                    basis: vec![AuditBasisRecord {
                        kind: "test-code".to_owned(),
                        reference: format!(
                            "{}::{}:{}",
                            rule.location.path,
                            rule.location.locator,
                            rule.location.byte_range.start_line
                        ),
                    }],
                })
                .collect(),
            exclusions: Vec::new(),
            auditor: AuditorRecord {
                kind: "deterministic".to_owned(),
                id: "vtest".to_owned(),
                model: None,
            },
            confidence: None,
            audited_at: now_rfc3339(),
            revision: git_revision(&layout.root),
        };
        write_new_record(&path, &record.to_yaml()?)?;
    }
    Ok(())
}

fn audit_one(
    root: &Path,
    scan: &ScanResult,
    test: &TestEntity,
    assertion_macros: &[String],
    config_hash: &ContentHash,
) -> Result<StaticAudit, AuditError> {
    let path = root.join(test.location.path.as_str());
    let source = fs::read_to_string(&path).map_err(|source| AuditError::Io {
        path: path.clone(),
        source,
    })?;
    let syntax = syn::parse_file(&source).map_err(|source| AuditError::Parse {
        path: path.clone(),
        source,
    })?;
    let item = find_function(&syntax, &test.location.locator)
        .ok_or_else(|| AuditError::TestNotFound(test.id.to_string()))?;
    let target = test
        .targets
        .first()
        .and_then(|target| target_source(scan, target));
    let target_locator = target.and_then(source_locator_value);
    let target_item_path = target_locator.and_then(rust_item_path);
    let target_file = target.map(|source| source.location.path.as_str());
    let target_resolution = target_item_path.map(|target_item_path| {
        TargetResolution::new(
            target_item_path,
            &test.location.locator,
            target_file == Some(test.location.path.as_str()),
        )
    });
    let mut rules = Vec::new();
    let has_assert = has_assert_like(item, assertion_macros);
    let ignored = has_attribute(&item.attrs, "ignore");
    rules.push(rule_da001(
        item,
        &syntax,
        assertion_macros,
        target_resolution.as_ref(),
    ));
    rules.push(rule_da002(
        item,
        &syntax,
        target_resolution.as_ref(),
        assertion_macros,
    ));
    rules.push(rule_da003(
        item,
        &syntax,
        target_resolution.as_ref(),
        assertion_macros,
    ));
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
    let mut diagnostics = Vec::new();
    if ignored {
        let location = source_location(&test.location, item.span());
        rules.push(RuleResult {
            rule: "W-DA-101".to_owned(),
            verdict: AuditVerdict::Pass,
            reason: "test is marked #[ignore]; execution may be NOT_EXECUTED".to_owned(),
            location: location.clone(),
        });
        diagnostics.push(
            Diagnostic::warning("W-DA-101", format!("test {} is marked #[ignore]", test.id))
                .with_location(location),
        );
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
    let mut subjects = vec![
        AuditSubjectRecord {
            id: Some(test.id.to_string()),
            locator: None,
            hash: test.content_hash.clone(),
        },
        AuditSubjectRecord {
            id: Some("CONFIG".to_owned()),
            locator: None,
            hash: config_hash.clone(),
        },
    ];
    if let Some(test_source) = scan.sources.iter().find(|source| {
        source.location.path == test.location.path
            && source.location.locator == test.location.locator
    }) {
        subjects.push(AuditSubjectRecord {
            id: None,
            locator: Some(test_source.target.normalized()),
            hash: test_source.content_hash.clone(),
        });
    }
    if let Some(target) = target {
        subjects.push(AuditSubjectRecord {
            id: None,
            locator: Some(target.target.normalized()),
            hash: target.content_hash.clone(),
        });
    }
    // DA-002 may inspect one-hop helpers in the test file. Bind precisely the
    // directly called same-file functions so a helper edit invalidates the
    // conclusion without making unrelated tests stale together.
    let helper_names = call_facts(item, assertion_macros).names;
    for source in scan.sources.iter().filter(|source| {
        source.location.path == test.location.path
            && source_locator_value(source)
                .and_then(rust_item_path)
                .and_then(|item_path| item_path.rsplit("::").next())
                .is_some_and(|name| helper_names.contains(name))
            && source.location.locator != test.location.locator
    }) {
        let locator = source.target.normalized();
        if subjects
            .iter()
            .any(|subject| subject.locator.as_deref() == Some(&locator))
        {
            continue;
        }
        subjects.push(AuditSubjectRecord {
            id: None,
            locator: Some(locator),
            hash: source.content_hash.clone(),
        });
    }
    Ok(StaticAudit {
        id: new_record_id(),
        test_id: test.id.to_string(),
        subject_hash: test.content_hash.clone(),
        subjects,
        verdict,
        rules,
        diagnostics,
    })
}

fn target_source<'a>(
    scan: &'a ScanResult,
    target: &TargetRef,
) -> Option<&'a vtest_model::SourceFunction> {
    scan.sources.iter().find(|source| match target {
        TargetRef::Locator { .. } => source.target == *target,
        TargetRef::SrcId(src_id) => source.src_id.as_ref() == Some(src_id),
    })
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

pub(crate) fn source_location(
    location: &SourceLocation,
    span: proc_macro2::Span,
) -> SourceLocation {
    let mut location = location.clone();
    location.byte_range.start_line = span.start().line;
    location.byte_range.end_line = span.end().line;
    location
}

pub(crate) fn source_location_from_item(item: &ItemFn) -> SourceLocation {
    source_location_from_span(item, item.span())
}

pub(crate) fn source_location_from_span(item: &ItemFn, span: proc_macro2::Span) -> SourceLocation {
    SourceLocation {
        adapter: vtest_model::AdapterId::new("rust-cargo"),
        path: vtest_model::ProjectPath::new(""),
        locator: item.sig.ident.to_string(),
        byte_range: vtest_model::SourceRange {
            start: 0,
            end: 0,
            start_line: span.start().line,
            end_line: span.end().line,
        },
    }
}

fn source_locator_value(source: &vtest_model::SourceFunction) -> Option<&str> {
    match &source.target {
        TargetRef::Locator { adapter, value } if adapter.as_str() == "rust-cargo" => Some(value),
        TargetRef::Locator { .. } | TargetRef::SrcId(_) => None,
    }
}

fn rust_item_path(locator: &str) -> Option<&str> {
    let separator = locator.find("::")?;
    locator.get(separator + 2..)
}

fn git_revision(root: &Path) -> Revision {
    let root_text = root.to_string_lossy();
    let git_root = root_text
        .strip_prefix(r"\\?\")
        .unwrap_or(&root_text)
        .replace('\\', "/");
    let safe_directory = format!("safe.directory={git_root}");
    let commit = Command::new("git")
        .args(["-c", &safe_directory, "-C", &git_root, "rev-parse", "HEAD"])
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|value| !value.is_empty())
        .or_else(|| read_head_commit(root));
    let dirty = Command::new("git")
        .args([
            "-c",
            &safe_directory,
            "-C",
            &git_root,
            "status",
            "--porcelain",
        ])
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .map_or(true, |output| {
            !output.status.success() || !output.stdout.is_empty()
        });
    Revision { commit, dirty }
}

fn read_head_commit(root: &Path) -> Option<String> {
    let dot_git = root.join(".git");
    let git_dir = if dot_git.is_dir() {
        dot_git
    } else {
        let pointer = fs::read_to_string(&dot_git).ok()?;
        let path = pointer.trim().strip_prefix("gitdir:")?.trim();
        let path = Path::new(path);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            root.join(path)
        }
    };
    let head = fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let head = head.trim();
    if is_git_object_id(head) {
        return Some(head.to_owned());
    }
    let reference = head.strip_prefix("ref:")?.trim();
    if let Ok(value) = fs::read_to_string(git_dir.join(reference)) {
        let value = value.trim();
        if is_git_object_id(value) {
            return Some(value.to_owned());
        }
    }
    let packed = fs::read_to_string(git_dir.join("packed-refs")).ok()?;
    packed.lines().find_map(|line| {
        let (object_id, name) = line.split_once(' ')?;
        (name == reference && is_git_object_id(object_id)).then(|| object_id.to_owned())
    })
}

fn is_git_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn format_verdict(verdict: AuditVerdict) -> String {
    match verdict {
        AuditVerdict::Pass => "PASS",
        AuditVerdict::Fail => "FAIL",
        AuditVerdict::Unknown => "UNKNOWN",
    }
    .to_owned()
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
            crate::classify_target_call("crate::module::target", &resolution, false),
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
