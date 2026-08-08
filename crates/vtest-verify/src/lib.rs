//! Fail-closed aggregation and report boundary (M6).

use std::{collections::BTreeMap, fs, path::Path};

use serde::Serialize;
use vtest_model::{CheckValue, ContentHash, Diagnostic, EvidenceRecord, Locator};
use vtest_scan::ScanResult;
use vtest_store::{
    read_evidence, read_record_ids, read_req, read_text, yaml_scalar_value, ProjectConfig,
    VerifyLayout,
};

pub const ALL_ITEMS: [&str; 11] = [
    "spec_coverage",
    "vo_decomposition",
    "vo_coverage",
    "test_existence",
    "static_audit",
    "semantic_audit",
    "impl_consistency",
    "test_execution",
    "runtime_result",
    "target_execution",
    "evidence_validity",
];

#[derive(Clone, Debug, Serialize)]
pub struct ReportItem {
    pub item: String,
    pub value: CheckValue,
    pub basis: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct VerificationReport {
    pub requested_scope: Vec<String>,
    pub scope_outside_not_checked: bool,
    pub items: Vec<ReportItem>,
    pub result: CheckValue,
}

#[derive(Clone, Debug, Serialize)]
pub struct VerificationResult {
    pub report: VerificationReport,
    pub diagnostics: Vec<Diagnostic>,
}

impl VerificationResult {
    pub fn is_ok(&self) -> bool {
        self.report.result == CheckValue::Pass
    }
}

pub fn verify_project(
    root: &Path,
    scan: &ScanResult,
    config: &ProjectConfig,
    requested_scope: Option<Vec<String>>,
) -> VerificationResult {
    let requested_scope = requested_scope.unwrap_or_else(|| config.verify.full_scope.clone());
    let layout = VerifyLayout::new(root);
    let diagnostics = scan.diagnostics.clone();
    let evidence = read_evidence_records(&layout);
    let mut items = Vec::new();
    for item in ALL_ITEMS {
        if !requested_scope.iter().any(|requested| requested == item) {
            items.push(ReportItem {
                item: item.to_owned(),
                value: CheckValue::NotChecked,
                basis: vec!["outside requested scope".to_owned()],
            });
            continue;
        }
        let (value, basis) = evaluate_item(root, &layout, scan, item, &evidence);
        items.push(ReportItem {
            item: item.to_owned(),
            value,
            basis,
        });
    }
    let result = requested_scope
        .iter()
        .filter_map(|requested| items.iter().find(|item| &item.item == requested))
        .map(|item| item.value)
        .fold(CheckValue::Pass, combine_values);
    VerificationResult {
        report: VerificationReport {
            scope_outside_not_checked: requested_scope.len() < ALL_ITEMS.len(),
            requested_scope,
            items,
            result,
        },
        diagnostics,
    }
}

fn evaluate_item(
    root: &Path,
    layout: &VerifyLayout,
    scan: &ScanResult,
    item: &str,
    evidence: &BTreeMap<String, EvidenceRecord>,
) -> (CheckValue, Vec<String>) {
    match item {
        "spec_coverage" => {
            let ids = read_record_ids(&layout.req_dir()).unwrap_or_default();
            let covered = ids
                .iter()
                .filter_map(|id| read_req(layout, id).ok())
                .any(|req| !req.spec_refs.is_empty());
            if covered {
                (
                    CheckValue::Pass,
                    vec!["REQ records reference SPEC records".to_owned()],
                )
            } else {
                (
                    CheckValue::Missing,
                    vec!["no REQ with a SPEC reference".to_owned()],
                )
            }
        }
        "vo_decomposition" => {
            if scan.diagnostics.iter().any(Diagnostic::is_error) {
                (
                    CheckValue::Fail,
                    vec!["scan emitted an error diagnostic".to_owned()],
                )
            } else if read_record_ids(&layout.vo_dir())
                .unwrap_or_default()
                .is_empty()
            {
                (CheckValue::Missing, vec!["no VO records exist".to_owned()])
            } else {
                (
                    CheckValue::Pass,
                    vec!["scan and canonical VO records are structurally readable".to_owned()],
                )
            }
        }
        "vo_coverage" => evaluate_external_audit(root, layout, scan, "vo-coverage"),
        "test_existence" => {
            let vo_ids = read_record_ids(&layout.vo_dir()).unwrap_or_default();
            let covered = vo_ids.iter().any(|id| {
                scan.tests
                    .iter()
                    .any(|test| test.covers.iter().any(|covered| covered.as_str() == id))
            });
            if covered {
                (
                    CheckValue::Pass,
                    vec!["at least one VO has a covering Test".to_owned()],
                )
            } else {
                (
                    CheckValue::Missing,
                    vec!["no covering Test was found for any VO".to_owned()],
                )
            }
        }
        "static_audit" => evaluate_static_audit(layout),
        "semantic_audit" => evaluate_external_audit(root, layout, scan, "test-semantic"),
        "impl_consistency" => evaluate_external_audit(root, layout, scan, "impl-consistency"),
        "test_execution" => {
            if evidence.is_empty() {
                (
                    CheckValue::NotExecuted,
                    vec!["no Evidence records exist".to_owned()],
                )
            } else {
                (
                    CheckValue::Pass,
                    vec![format!("{} Evidence record(s) exist", evidence.len())],
                )
            }
        }
        "runtime_result" => evaluate_runtime(evidence, scan),
        "target_execution" => evaluate_target_execution(evidence, scan),
        "evidence_validity" => evaluate_evidence_validity(evidence, scan),
        _ => (CheckValue::Unknown, vec!["unknown check item".to_owned()]),
    }
}

fn evaluate_static_audit(layout: &VerifyLayout) -> (CheckValue, Vec<String>) {
    let ids = read_record_ids(&layout.audits_dir()).unwrap_or_default();
    let mut static_ids = Vec::new();
    for id in ids {
        let path = layout.audits_dir().join(format!("{id}.yaml"));
        if read_text(&path)
            .ok()
            .and_then(|text| yaml_scalar_value(&text, "kind"))
            .as_deref()
            == Some("static")
        {
            static_ids.push(id);
        }
    }
    if static_ids.is_empty() {
        return (
            CheckValue::NotChecked,
            vec!["no static audit records exist".to_owned()],
        );
    }
    let mut value = CheckValue::Pass;
    for id in static_ids {
        if let Ok(text) = read_text(&layout.audits_dir().join(format!("{id}.yaml"))) {
            value = combine_values(
                value,
                match yaml_scalar_value(&text, "verdict").as_deref() {
                    Some("PASS") => CheckValue::Pass,
                    Some("FAIL") => CheckValue::Fail,
                    Some("UNKNOWN") => CheckValue::Unknown,
                    _ => CheckValue::Unknown,
                },
            );
        }
    }
    (
        value,
        vec!["static audit verdicts are combined fail-closed".to_owned()],
    )
}

#[derive(Clone, Debug)]
struct AuditSubject {
    kind: String,
    id: Option<String>,
    locator: Option<String>,
    hash: Option<ContentHash>,
}

fn evaluate_external_audit(
    root: &Path,
    layout: &VerifyLayout,
    scan: &ScanResult,
    kind: &str,
) -> (CheckValue, Vec<String>) {
    let ids = read_record_ids(&layout.audits_dir()).unwrap_or_default();
    let mut record_count = 0usize;
    let mut valid_count = 0usize;
    let mut stale_count = 0usize;
    let mut verdict = CheckValue::Pass;
    for id in ids {
        let path = layout.audits_dir().join(format!("{id}.yaml"));
        let Ok(text) = read_text(&path) else { continue };
        if yaml_scalar_value(&text, "kind").as_deref() != Some(kind) {
            continue;
        }
        record_count += 1;
        let subjects = parse_audit_subjects(&text);
        let valid = !subjects.is_empty()
            && subjects
                .iter()
                .all(|subject| subject_is_current(root, layout, scan, subject));
        if !valid {
            stale_count += 1;
            continue;
        }
        let current = match yaml_scalar_value(&text, "verdict").as_deref() {
            Some("PASS") => CheckValue::Pass,
            Some("FAIL") => CheckValue::Fail,
            Some("UNKNOWN") => CheckValue::Unknown,
            _ => CheckValue::Unknown,
        };
        valid_count += 1;
        verdict = combine_values(verdict, current);
    }
    if record_count == 0 {
        return (
            CheckValue::NotChecked,
            vec![format!("no {kind} audit records exist")],
        );
    }
    if valid_count == 0 {
        return (
            CheckValue::Stale,
            vec![format!("all {kind} audit records are stale")],
        );
    }
    let mut basis = vec![format!("{valid_count} valid {kind} audit record(s)")];
    if stale_count > 0 {
        basis.push(format!(
            "{stale_count} stale {kind} audit record(s) ignored"
        ));
    }
    (verdict, basis)
}

fn parse_audit_subjects(text: &str) -> Vec<AuditSubject> {
    let mut subjects = Vec::new();
    let mut current: Option<AuditSubject> = None;
    for raw in text.lines() {
        if raw.starts_with("  - kind:") {
            if let Some(subject) = current.take() {
                subjects.push(subject);
            }
            let kind_line = raw
                .trim_start()
                .strip_prefix("- ")
                .unwrap_or(raw.trim_start());
            current = Some(AuditSubject {
                kind: yaml_line_value(kind_line, "kind").unwrap_or_default(),
                id: None,
                locator: None,
                hash: None,
            });
            continue;
        }
        let Some(subject) = current.as_mut() else {
            continue;
        };
        if raw.starts_with("    id:") {
            subject.id = yaml_line_value(raw, "id");
        } else if raw.starts_with("    locator:") {
            subject.locator = yaml_line_value(raw, "locator");
        } else if raw.starts_with("    hash:") {
            subject.hash = yaml_line_value(raw, "hash").and_then(|value| value.parse().ok());
        }
    }
    if let Some(subject) = current {
        subjects.push(subject);
    }
    subjects
}

fn yaml_line_value(line: &str, key: &str) -> Option<String> {
    let value = line.trim().strip_prefix(&format!("{key}:"))?.trim();
    if value == "null" {
        return None;
    }
    if value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2 {
        return Some(value[1..value.len() - 1].replace("''", "'"));
    }
    Some(value.to_owned())
}

fn subject_is_current(
    root: &Path,
    layout: &VerifyLayout,
    scan: &ScanResult,
    subject: &AuditSubject,
) -> bool {
    let Some(expected) = &subject.hash else {
        return false;
    };
    let actual = match subject.kind.as_str() {
        "test" => subject.id.as_deref().and_then(|id| {
            scan.tests
                .iter()
                .find(|test| test.id.as_str() == id)
                .map(|test| test.content_hash.clone())
        }),
        "target" => subject
            .locator
            .as_deref()
            .and_then(Locator::parse)
            .and_then(|locator| {
                scan.sources
                    .iter()
                    .find(|source| source.locator == locator)
                    .map(|source| source.content_hash.clone())
            }),
        "vo" => record_hash(&layout.vo_dir(), subject.id.as_deref()),
        "req" => record_hash(&layout.req_dir(), subject.id.as_deref()),
        "spec" => subject.id.as_deref().and_then(|id| {
            let text = read_text(&layout.spec_dir().join(format!("{id}.yaml"))).ok()?;
            let path = yaml_scalar_value(&text, "path")?;
            fs::read_to_string(root.join(path))
                .ok()
                .map(|source| ContentHash::from_text(&source))
        }),
        _ => None,
    };
    actual.as_ref() == Some(expected)
}

fn record_hash(directory: &Path, id: Option<&str>) -> Option<ContentHash> {
    let id = id?;
    if id.is_empty()
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return None;
    }
    fs::read_to_string(directory.join(format!("{id}.yaml")))
        .ok()
        .map(|text| ContentHash::from_text(&text))
}

fn evaluate_runtime(
    evidence: &BTreeMap<String, EvidenceRecord>,
    scan: &ScanResult,
) -> (CheckValue, Vec<String>) {
    let validity = evaluate_evidence_validity(evidence, scan).0;
    if evidence.is_empty() {
        return (
            CheckValue::NotExecuted,
            vec!["no Evidence exists".to_owned()],
        );
    }
    if validity != CheckValue::Pass {
        return (
            CheckValue::Stale,
            vec!["Evidence is not currently valid".to_owned()],
        );
    }
    let value = evidence
        .values()
        .map(|record| match record.result {
            vtest_model::TestResult::Pass => CheckValue::Pass,
            vtest_model::TestResult::Fail => CheckValue::Fail,
        })
        .fold(CheckValue::Pass, combine_values);
    (value, vec!["valid Evidence runtime results".to_owned()])
}

fn evaluate_target_execution(
    evidence: &BTreeMap<String, EvidenceRecord>,
    scan: &ScanResult,
) -> (CheckValue, Vec<String>) {
    let validity = evaluate_evidence_validity(evidence, scan).0;
    if evidence.is_empty() {
        return (
            CheckValue::NotExecuted,
            vec!["no Evidence exists".to_owned()],
        );
    }
    if validity != CheckValue::Pass {
        return (
            CheckValue::Stale,
            vec!["Evidence is not currently valid".to_owned()],
        );
    }
    let value = evidence
        .values()
        .map(|record| {
            if record.target_execution.checked {
                record.target_execution.result
            } else {
                CheckValue::NotChecked
            }
        })
        .fold(CheckValue::Pass, combine_values);
    (
        value,
        vec!["target execution is PASS only when a checked Evidence says so".to_owned()],
    )
}

fn evaluate_evidence_validity(
    evidence: &BTreeMap<String, EvidenceRecord>,
    scan: &ScanResult,
) -> (CheckValue, Vec<String>) {
    if evidence.is_empty() {
        return (
            CheckValue::NotExecuted,
            vec!["no Evidence exists".to_owned()],
        );
    }
    let value = evidence
        .values()
        .map(|record| {
            let Some(test) = scan.tests.iter().find(|test| test.id == record.test_id) else {
                return CheckValue::Stale;
            };
            // Evidence v0.1 stores one target_fn hash. Integration tests may
            // declare additional targets, so their full target set cannot yet
            // be proven current from this record alone.
            if !v01_evidence_binds_all_targets(test.additional_targets.len()) {
                return CheckValue::Unknown;
            }
            let Some(target) = scan.sources.iter().find(|source| match &test.target {
                vtest_model::TargetRef::Locator(locator) => source.locator == *locator,
                vtest_model::TargetRef::SrcId(src_id) => source.src_id.as_ref() == Some(src_id),
            }) else {
                return CheckValue::Stale;
            };
            if record.hashes.test_fn != test.content_hash
                || record.hashes.target_fn != target.content_hash
            {
                CheckValue::Stale
            } else if record.revision.commit.is_none() {
                CheckValue::Fail
            } else {
                CheckValue::Pass
            }
        })
        .fold(CheckValue::Pass, combine_values);
    let mut basis =
        vec!["Evidence hashes and revision are checked against current scan".to_owned()];
    for record in evidence.values() {
        if scan
            .tests
            .iter()
            .find(|test| test.id == record.test_id)
            .is_some_and(|test| !v01_evidence_binds_all_targets(test.additional_targets.len()))
        {
            basis.push(format!(
                "Test {} has multiple targets but Evidence v0.1 binds one target_fn hash",
                record.test_id
            ));
        }
    }
    (value, basis)
}

fn read_evidence_records(layout: &VerifyLayout) -> BTreeMap<String, EvidenceRecord> {
    let mut records = BTreeMap::new();
    for id in read_record_ids(&layout.evidence_dir()).unwrap_or_default() {
        let path = layout.evidence_dir().join(format!("{id}.yaml"));
        if let Ok(record) = read_evidence(&path) {
            records
                .entry(record.test_id.as_str().to_owned())
                .and_modify(|current: &mut EvidenceRecord| {
                    if current.executed_at < record.executed_at {
                        *current = record.clone();
                    }
                })
                .or_insert(record);
        }
    }
    records
}

fn combine_values(left: CheckValue, right: CheckValue) -> CheckValue {
    if left == CheckValue::Pass {
        return right;
    }
    if right == CheckValue::Pass {
        return left;
    }
    let rank = |value| match value {
        CheckValue::Fail => 8,
        CheckValue::Mismatch => 7,
        CheckValue::Missing => 6,
        CheckValue::Stale => 5,
        CheckValue::NotExecuted => 4,
        CheckValue::NotChecked => 3,
        CheckValue::Unknown => 2,
        CheckValue::Pass => 1,
    };
    if rank(right) > rank(left) {
        right
    } else {
        left
    }
}

fn v01_evidence_binds_all_targets(additional_target_count: usize) -> bool {
    additional_target_count == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multi_target_evidence_cannot_pass_with_a_single_target_hash() {
        assert!(v01_evidence_binds_all_targets(0));
        assert!(!v01_evidence_binds_all_targets(1));
    }
}
