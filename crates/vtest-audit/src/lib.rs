//! Language-neutral static-audit orchestration.
//!
//! Rust syntax and rule evaluation live in `vtest-adapter-rust`.  This crate
//! selects the adapter, preserves the public CLI-facing result shape, and
//! delegates canonical record persistence without interpreting language
//! constructs itself.

use std::{fs, path::Path, process::Command};

pub use vtest_adapter_rust::{
    AuditError, AuditOptions, AuditVerdict, RuleResult, StaticAudit, StaticAuditSummary,
};
use vtest_scan::ScanResult;
use vtest_store::{
    load_config, new_record_id, write_new_record, AuditBasisRecord, AuditReasonRecord, AuditRecord,
    AuditorRecord, VerifyLayout,
};

pub fn audit_static(
    root: &Path,
    scan: &ScanResult,
    options: &AuditOptions,
) -> Result<StaticAuditSummary, AuditError> {
    let registry =
        vtest_adapter_rust::registry().map_err(|error| AuditError::Adapter(error.to_string()))?;
    audit_static_with_registry(root, scan, options, &registry)
}

/// Run static audit through an explicitly composed registry.  Product callers
/// use the built-in Rust registry; synthetic and third-party adapters can be
/// exercised without changing this language-neutral orchestration service.
pub fn audit_static_with_registry(
    root: &Path,
    scan: &ScanResult,
    options: &AuditOptions,
    registry: &vtest_adapter_api::AdapterRegistry,
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
    if tests.is_empty() {
        if let Some(test_id) = &options.test_id {
            return Err(AuditError::TestNotFound(test_id.clone()));
        }
    }
    let mut audits = Vec::new();
    for test in tests {
        let adapter_config = adapter_config_for(root, &test.execution.adapter)?;
        let registration = registry
            .require(
                &test.execution.adapter,
                vtest_adapter_api::Capability::StaticAudit,
            )
            .map_err(|error| AuditError::Adapter(error.to_string()))?;
        let adapter = registration.static_audit.as_ref().ok_or_else(|| {
            AuditError::Adapter("static audit capability is unavailable".to_owned())
        })?;
        let observation = adapter
            .audit(root, test, &adapter_config)
            .map_err(|error| AuditError::Adapter(error.to_string()))?;
        let config_hash = static_audit_config_hash(&test.execution.adapter, &observation.config);
        let mut subjects = vec![
            vtest_store::AuditSubjectRecord {
                id: Some(test.id.to_string()),
                locator: None,
                hash: test.content_hash.clone(),
            },
            vtest_store::AuditSubjectRecord {
                id: Some(format!("CONFIG::{}", test.execution.adapter)),
                locator: None,
                hash: config_hash.clone(),
            },
        ];
        for target in &test.targets {
            if let Some(source) = scan.sources.iter().find(|source| {
                source.target.adapter == target.adapter
                    && (source.target.value == target.value
                        || source
                            .src_id
                            .as_ref()
                            .is_some_and(|id| id.as_str() == target.value))
            }) {
                subjects.push(vtest_store::AuditSubjectRecord {
                    id: None,
                    locator: Some(source.target.value.clone()),
                    hash: source.content_hash.clone(),
                });
            }
        }
        subjects.push(vtest_store::AuditSubjectRecord {
            id: None,
            locator: Some(format!(
                "test-source::{}::{}",
                test.location.file, test.location.function
            )),
            hash: test.content_hash.clone(),
        });
        for source in &observation.closure.sources {
            let locator = format!(
                "{}::{}",
                source.project_relative_path, source.opaque_locator
            );
            if subjects
                .iter()
                .any(|subject| subject.locator.as_deref() == Some(&locator))
            {
                continue;
            }
            let hash = scan
                .sources
                .iter()
                .find(|candidate| {
                    candidate.target.adapter == test.execution.adapter
                        && candidate.target.value == locator
                })
                .map(|candidate| candidate.content_hash.clone())
                .unwrap_or_else(|| vtest_model::ContentHash::from_bytes(&source.bytes));
            subjects.push(vtest_store::AuditSubjectRecord {
                id: None,
                locator: Some(locator),
                hash,
            });
        }
        let rules = observation
            .rules
            .into_iter()
            .map(|rule| RuleResult {
                rule: rule.rule,
                verdict: check_to_verdict(rule.verdict),
                reason: rule.reason,
                location: rule.location,
            })
            .collect::<Vec<_>>();
        audits.push(StaticAudit {
            id: new_record_id(),
            test_id: test.id.to_string(),
            subject_hash: test.content_hash.clone(),
            subjects,
            verdict: if observation.closure.complete {
                check_to_verdict(observation.verdict)
            } else {
                AuditVerdict::Unknown
            },
            rules,
            diagnostics: observation.diagnostics,
        });
    }
    Ok(StaticAuditSummary { audits })
}

fn check_to_verdict(value: vtest_model::CheckValue) -> AuditVerdict {
    match value {
        vtest_model::CheckValue::Pass => AuditVerdict::Pass,
        vtest_model::CheckValue::Fail => AuditVerdict::Fail,
        _ => AuditVerdict::Unknown,
    }
}

fn static_audit_config_hash(
    adapter: &vtest_model::AdapterId,
    config: &vtest_adapter_api::StaticAuditConfigDraft,
) -> vtest_model::ContentHash {
    let effective_config = serde_json::to_vec(&config.effective_config).unwrap_or_default();
    vtest_model::ContentHash::from_domain_fields(
        "vtest:static-audit-config:v1",
        &[
            ("adapter", adapter.as_str().as_bytes()),
            ("rule_set_id", config.rule_set_id.as_bytes()),
            ("rule_set_version", config.rule_set_version.as_bytes()),
            ("effective_config", &effective_config),
        ],
    )
}

fn adapter_config_for(
    root: &Path,
    adapter_id: &vtest_model::AdapterId,
) -> Result<vtest_adapter_api::AdapterConfig, AuditError> {
    let project = load_config(root)?;
    let (roots, include, assertion_macros, coverage) = project
        .adapters
        .iter()
        .find(|entry| entry.id == adapter_id.as_str())
        .map(|entry| {
            (
                entry.roots.clone(),
                entry.scan.include.clone(),
                entry.scan.assertion_macros.clone(),
                entry.run.coverage.clone(),
            )
        })
        .unwrap_or_else(|| {
            (
                vec![".".to_owned()],
                project.scan.include.clone(),
                project.scan.assertion_macros.clone(),
                project.run.coverage.clone(),
            )
        });
    let mut config = vtest_adapter_api::AdapterConfig::default();
    config.insert("roots", roots.join(","));
    config.insert("include", include.join(","));
    config.insert("assertion_macros", assertion_macros.join(","));
    config.insert("coverage", coverage);
    Ok(config)
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
                            rule.location.file, rule.location.function, rule.location.start_line
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
            audited_at: vtest_store::now_rfc3339(),
            revision: git_revision(&layout.root),
        };
        write_new_record(&path, &record.to_yaml()?)?;
    }
    Ok(())
}

fn format_verdict(verdict: AuditVerdict) -> String {
    match verdict {
        AuditVerdict::Pass => "PASS".to_owned(),
        AuditVerdict::Fail => "FAIL".to_owned(),
        AuditVerdict::Unknown => "UNKNOWN".to_owned(),
    }
}

fn git_revision(root: &Path) -> vtest_model::Revision {
    let commit = Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|value| !value.is_empty());
    let dirty = Command::new("git")
        .current_dir(root)
        .args(["status", "--porcelain"])
        .output()
        .is_ok_and(|output| output.status.success() && !output.stdout.is_empty());
    vtest_model::Revision { commit, dirty }
}
