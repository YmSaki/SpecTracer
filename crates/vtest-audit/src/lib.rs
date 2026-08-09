//! Static-audit orchestration facade.
//!
//! Rust AST rules are implemented by `vtest-adapter-rust`; this crate selects
//! the adapter based on the neutral execution descriptor and keeps unsupported
//! capabilities fail-closed.

use std::path::Path;

use vtest_adapter_rust::{audit_static as rust_audit_static, StaticAudit, StaticAuditSummary};
pub use vtest_adapter_rust::{AuditError, AuditOptions, AuditVerdict};
use vtest_model::{AdapterId, Diagnostic, TestEntity};
use vtest_scan::ScanResult;
use vtest_store::{new_record_id, AuditSubjectRecord};

pub use vtest_adapter_rust::RuleResult;

pub fn audit_static(
    root: &Path,
    scan: &ScanResult,
    options: &AuditOptions,
) -> Result<StaticAuditSummary, AuditError> {
    let rust_tests = scan
        .tests
        .iter()
        .filter(|test| test.execution.adapter == AdapterId::from("rust-cargo"));
    let unsupported_tests = scan
        .tests
        .iter()
        .filter(|test| test.execution.adapter != AdapterId::from("rust-cargo"));
    let selected_rust = rust_tests
        .filter(|test| is_selected(test, options))
        .map(|test| test.id.as_str().to_owned())
        .collect::<Vec<_>>();
    let has_requested_unsupported = unsupported_tests
        .clone()
        .any(|test| is_selected(test, options));
    if options.test_id.is_some() && selected_rust.is_empty() && !has_requested_unsupported {
        return Err(AuditError::TestNotFound(
            options.test_id.clone().unwrap_or_default(),
        ));
    }
    let mut audits = if selected_rust.is_empty() {
        Vec::new()
    } else {
        rust_audit_static(
            root,
            scan,
            &AuditOptions {
                test_id: options.test_id.clone(),
            },
        )?
        .audits
    };
    for test in unsupported_tests.filter(|test| is_selected(test, options)) {
        audits.push(unsupported_audit(test));
    }
    audits.sort_by(|left, right| left.test_id.cmp(&right.test_id));
    Ok(StaticAuditSummary { audits })
}

fn is_selected(test: &TestEntity, options: &AuditOptions) -> bool {
    options
        .test_id
        .as_deref()
        .is_none_or(|id| id == test.id.as_str())
}

fn unsupported_audit(test: &TestEntity) -> StaticAudit {
    StaticAudit {
        id: new_record_id(),
        test_id: test.id.to_string(),
        subject_hash: test.content_hash.clone(),
        subjects: vec![AuditSubjectRecord {
            id: Some(test.id.to_string()),
            locator: None,
            hash: test.content_hash.clone(),
        }],
        verdict: AuditVerdict::Unknown,
        rules: Vec::new(),
        diagnostics: vec![Diagnostic::warning(
            "W-ADAPTER-101",
            format!(
                "adapter `{}` does not provide static-audit capability",
                test.execution.adapter
            ),
        )],
    }
}

/// Persist only concrete Rust audit records.  A missing capability is
/// represented by the absence of a current audit record and remains
/// non-PASS in verification; it is never fabricated as a passing record.
pub fn persist_static_audits(
    layout: &vtest_store::VerifyLayout,
    summary: &StaticAuditSummary,
) -> Result<(), AuditError> {
    let supported = StaticAuditSummary {
        audits: summary
            .audits
            .iter()
            .filter(|audit| {
                !audit
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == "W-ADAPTER-101")
            })
            .cloned()
            .collect(),
    };
    vtest_adapter_rust::persist_static_audits(layout, &supported)
}
