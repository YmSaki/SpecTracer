//! Deterministic static audit orchestration (M3).
//!
//! The Rust-specific rules live in `vtest-adapter-rust` behind
//! `StaticAuditAdapter`. This crate loads config, invokes the adapter per Test,
//! and reshapes each observation into a persisted `AuditRecord` — it performs no
//! Rust parsing of its own.

use std::{collections::BTreeSet, fs, path::Path, process::Command};

use serde::Serialize;
use thiserror::Error;
use vtest_adapter_api::{AdapterError, StaticAuditAdapter};
use vtest_model::{
    hash_static_audit_config_subject, AdapterId, CheckValue, ContentHash, Diagnostic, Revision,
    SourceLocation, TestEntity,
};
use vtest_scan::{
    find_target_source, rust_cargo_static_audit_projection, ScanResult, STATIC_AUDIT_RULE_SET_ID,
    STATIC_AUDIT_RULE_SET_VERSION,
};
use vtest_store::{
    load_config, new_record_id, now_rfc3339, pure_static_fold, write_new_record, AuditBasisRecord,
    AuditReasonRecord, AuditRecord, AuditSubjectRecord, AuditTargetVerdictRecord, AuditorRecord,
    StoreError, VerifyLayout,
};

#[derive(Debug, Error)]
pub enum AuditError {
    #[error("failed to read {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("test `{0}` was not found")]
    TestNotFound(String),
    #[error("store error: {0}")]
    Store(#[from] StoreError),
    #[error("adapter error: {0}")]
    Adapter(#[from] AdapterError),
    #[error("malformed static audit observation: {0}")]
    MalformedObservation(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AuditVerdict {
    Pass,
    Fail,
    Unknown,
}

/// One target-scoped verdict of a static rule (DA-002 / DA-003), keyed by the
/// resolved canonical Locator of the declared target (詳細設計 §3.6).
#[derive(Clone, Debug, Serialize)]
pub struct RuleTargetVerdict {
    pub target: String,
    pub verdict: AuditVerdict,
    pub basis: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct RuleResult {
    pub rule: String,
    pub verdict: AuditVerdict,
    pub reason: String,
    pub location: SourceLocation,
    /// Per-target verdicts for the target-scoped rules; empty for the rest. The
    /// `verdict` above is their pure-static fold (詳細設計 §3.6, §7.2).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<RuleTargetVerdict>,
}

fn basis_reference(location: &SourceLocation) -> String {
    format!(
        "{}::{}:{}",
        location.path, location.locator, location.byte_range.start_line
    )
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
    adapter: &dyn StaticAuditAdapter,
) -> Result<StaticAuditSummary, AuditError> {
    let config = load_config(root)?;
    // Core owns the config -> projection mapping; the adapter receives only the
    // rule-affecting subset. The same projection drives the CONFIG subject so a
    // run-only config change never stales a static Audit.
    let projection = rust_cargo_static_audit_projection(&config);
    let adapter_id = AdapterId::new("rust-cargo");
    let config_hash = hash_static_audit_config_subject(
        &adapter_id,
        STATIC_AUDIT_RULE_SET_ID,
        STATIC_AUDIT_RULE_SET_VERSION,
        &projection,
    );
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
        let observation = adapter.audit(root, &projection, test)?;
        audits.push(record_from_observation(
            scan,
            test,
            observation,
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
                        reference: basis_reference(&rule.location),
                    }],
                    targets: rule
                        .targets
                        .iter()
                        .map(|target| AuditTargetVerdictRecord {
                            target: target.target.clone(),
                            verdict: format_verdict(target.verdict),
                            basis: vec![AuditBasisRecord {
                                kind: "test-code".to_owned(),
                                reference: target.basis.clone(),
                            }],
                        })
                        .collect(),
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

fn check_to_verdict(value: CheckValue) -> AuditVerdict {
    match value {
        CheckValue::Pass => AuditVerdict::Pass,
        CheckValue::Fail => AuditVerdict::Fail,
        // The Rust auditor emits only Pass/Fail/Unknown; every other CheckValue
        // is defensively treated as Unknown rather than aggregated to PASS.
        _ => AuditVerdict::Unknown,
    }
}

/// Reshape one adapter `StaticAuditObservation` into a persisted `StaticAudit`.
///
/// Per-rule outcomes and the aggregate verdict come straight from the
/// observation. Source subjects are bound through the scan index so their
/// content hashes match what verify re-derives (the adapter's fragment bytes
/// are identity-only here); an analysis fragment with no scan match is skipped
/// rather than fallback-hashed. The CONFIG subject hash is owned by the core.
fn record_from_observation(
    scan: &ScanResult,
    test: &TestEntity,
    observation: vtest_adapter_api::StaticAuditObservation,
    config_hash: &ContentHash,
) -> Result<StaticAudit, AuditError> {
    // A PASS over an incomplete analysis input closure is clamped to UNKNOWN
    // (詳細設計 §5.2, §7.1). An incomplete closure also means some declared
    // target did not resolve, so the per-target lists are dropped below rather
    // than persisted as a malformed partial (1:1-violating) list (§7.2, §7.3).
    let complete = observation.analysis.complete;
    let mut rules: Vec<RuleResult> = Vec::new();
    for rule in observation.rules {
        let mut targets: Vec<RuleTargetVerdict> = Vec::new();
        let mut target_checks: Vec<CheckValue> = Vec::new();
        if complete {
            let mut seen = BTreeSet::new();
            for entry in &rule.targets {
                // The core owns target resolution (§6.1.1): the persisted target
                // identity is the resolved canonical Locator, never the declared
                // spelling (an SRC ID reference is not a locator).
                let Some(source) = find_target_source(scan, &entry.target) else {
                    continue;
                };
                let canonical = source.target.normalized();
                // The same canonical Source Target declared twice (locator + SRC
                // ID, §922) collapses to a single per-target entry.
                if !seen.insert(canonical.clone()) {
                    continue;
                }
                target_checks.push(entry.verdict);
                targets.push(RuleTargetVerdict {
                    target: canonical,
                    verdict: check_to_verdict(entry.verdict),
                    basis: basis_reference(&entry.location),
                });
            }
        }
        // The core owns the rule-level fold. With per-target verdicts present it
        // is their pure-static fold; the adapter's own folded verdict must agree
        // or the observation is a malformed adapter output (§7.1, §7.2).
        let verdict = if targets.is_empty() {
            check_to_verdict(rule.verdict)
        } else {
            let folded = pure_static_fold(&target_checks);
            if folded != rule.verdict {
                return Err(AuditError::MalformedObservation(format!(
                    "rule {} verdict {:?} disagrees with the fold {:?} of its per-target verdicts",
                    rule.rule, rule.verdict, folded
                )));
            }
            check_to_verdict(folded)
        };
        rules.push(RuleResult {
            rule: rule.rule,
            verdict,
            reason: rule.reason,
            location: rule.location,
            targets,
        });
    }
    // Aggregate the core-derived rule verdicts (§7.1 L962): FAIL dominates, then
    // UNKNOWN; a PASS over an incomplete closure is clamped to UNKNOWN.
    let aggregate = if rules.iter().any(|rule| rule.verdict == AuditVerdict::Fail) {
        AuditVerdict::Fail
    } else if rules
        .iter()
        .any(|rule| rule.verdict == AuditVerdict::Unknown)
    {
        AuditVerdict::Unknown
    } else {
        AuditVerdict::Pass
    };
    let verdict = match aggregate {
        AuditVerdict::Pass if !complete => AuditVerdict::Unknown,
        other => other,
    };
    let mut diagnostics = Vec::new();
    for rule in &rules {
        if rule.rule == "W-DA-101" {
            diagnostics.push(
                Diagnostic::warning("W-DA-101", format!("test {} is marked #[ignore]", test.id))
                    .with_location(rule.location.clone()),
            );
        }
    }
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
    for fragment in &observation.analysis.sources {
        let Some(source) = scan.sources.iter().find(|source| {
            source.location.path == fragment.location.path
                && source.location.locator == fragment.location.locator
        }) else {
            continue;
        };
        let locator = source.target.normalized();
        if subjects
            .iter()
            .any(|subject| subject.locator.as_deref() == Some(&locator))
        {
            continue;
        }
        // The Test's own construct binds the Test entity by its content hash so
        // verify can prove the Test code is unchanged at its locator; the target
        // and one-hop helpers bind their scan-derived source hashes.
        let is_test_code = fragment.location.path == test.location.path
            && fragment.location.locator == test.location.locator;
        let hash = if is_test_code {
            test.content_hash.clone()
        } else {
            source.content_hash.clone()
        };
        subjects.push(AuditSubjectRecord {
            id: None,
            locator: Some(locator),
            hash,
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
