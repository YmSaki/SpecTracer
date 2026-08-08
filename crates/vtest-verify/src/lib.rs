//! Fail-closed aggregation and report boundary (M6).

use std::{collections::BTreeMap, fs, path::Path};

use serde::Serialize;
use vtest_model::{CheckValue, ContentHash, Diagnostic, EvidenceRecord, Locator};
use vtest_scan::ScanResult;
use vtest_store::{
    read_audit, read_evidence, read_record_ids, read_req, read_text, yaml_scalar_value,
    AuditRecord, AuditSubjectRecord, ProjectConfig, VerifyLayout,
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
        "static_audit" => evaluate_static_audit(layout, scan),
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

/// Evaluate static audits per currently scanned Test, rather than treating a
/// historical audit record as a project-wide evergreen result.  A static
/// audit is current only when it binds the Test under evaluation and every
/// recorded subject still has its captured hash.
fn evaluate_static_audit(layout: &VerifyLayout, scan: &ScanResult) -> (CheckValue, Vec<String>) {
    if scan.tests.is_empty() {
        return (
            CheckValue::NotChecked,
            vec!["no registered Tests are available for static-audit coverage".to_owned()],
        );
    }

    let mut records = Vec::new();
    let mut malformed = Vec::new();
    for id in read_record_ids(&layout.audits_dir()).unwrap_or_default() {
        let path = layout.audits_dir().join(format!("{id}.yaml"));
        match read_audit(&path) {
            Ok(record) if record.kind == "static" => records.push(record),
            Ok(_) => {}
            Err(error) => {
                // A malformed non-static record belongs to a different audit
                // boundary.  If it declares itself static, however, it must
                // not be silently discarded as a successful static audit.
                if read_text(&path)
                    .ok()
                    .and_then(|text| yaml_scalar_value(&text, "kind"))
                    .as_deref()
                    == Some("static")
                {
                    malformed.push((id, error.to_string(), read_text(&path).unwrap_or_default()));
                }
            }
        }
    }

    let mut project_value = CheckValue::Pass;
    let mut basis = Vec::new();
    for test in &scan.tests {
        let test_id = test.id.as_str();
        let test_records = records
            .iter()
            .filter(|record| audit_mentions_test(record, test_id))
            .collect::<Vec<_>>();
        let malformed_for_test = malformed
            .iter()
            .filter(|(_, _, text)| audit_text_mentions_test(text, test_id))
            .collect::<Vec<_>>();

        let mut valid = Vec::new();
        let mut stale = 0usize;
        for record in &test_records {
            if static_audit_binds_test_subjects(record, test, scan)
                && record
                    .subjects
                    .iter()
                    .all(|subject| audit_subject_is_current(layout, scan, subject))
            {
                valid.push(*record);
            } else {
                stale += 1;
            }
        }

        let value = if !malformed_for_test.is_empty() {
            CheckValue::Unknown
        } else if valid.is_empty() {
            if test_records.is_empty() {
                CheckValue::NotChecked
            } else {
                CheckValue::Stale
            }
        } else if valid.iter().any(|record| record.verdict == "FAIL") {
            CheckValue::Fail
        } else {
            valid
                .iter()
                .max_by(|left, right| compare_audit_recency(left, right))
                .map_or(CheckValue::Unknown, |record| audit_verdict_value(record))
        };

        project_value = combine_values(project_value, value);
        let mut detail = format!("Test {test_id}: {value:?}");
        if !valid.is_empty() {
            detail.push_str(&format!(
                " ({} current static audit record(s))",
                valid.len()
            ));
        }
        if stale > 0 {
            detail.push_str(&format!("; {stale} stale record(s) ignored"));
        }
        if !malformed_for_test.is_empty() {
            detail.push_str(&format!(
                "; {} malformed static audit record(s)",
                malformed_for_test.len()
            ));
        }
        basis.push(detail);
    }

    // A malformed static record that cannot be associated with a registered
    // Test is still an unknown input to this item.  It may not silently leave
    // an otherwise-PASS aggregate untouched.
    let unassociated_malformed = malformed
        .iter()
        .filter(|(_, _, text)| {
            !scan
                .tests
                .iter()
                .any(|test| audit_text_mentions_test(text, test.id.as_str()))
        })
        .collect::<Vec<_>>();
    if !unassociated_malformed.is_empty() {
        // A deterministic FAIL remains stronger than an unreadable record,
        // but every other aggregate must expose the malformed static input as
        // UNKNOWN rather than letting the normal NotChecked/Stale ordering
        // obscure it.
        if project_value != CheckValue::Fail {
            project_value = CheckValue::Unknown;
        }
        basis.push(format!(
            "{} malformed static audit record(s) could not be assigned to a registered Test: {}",
            unassociated_malformed.len(),
            unassociated_malformed
                .iter()
                .map(|(id, error, _)| format!("{id} ({error})"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    (project_value, basis)
}

fn audit_mentions_test(record: &AuditRecord, test_id: &str) -> bool {
    record
        .subjects
        .iter()
        .any(|subject| subject.id.as_deref() == Some(test_id))
}

fn audit_mentions_config(record: &AuditRecord) -> bool {
    record
        .subjects
        .iter()
        .any(|subject| subject.id.as_deref() == Some("CONFIG"))
}

/// Static audits are per-Test records.  A current record must bind the Test,
/// the raw configuration, and the exact source function resolved from that
/// Test's declared target.  It may additionally bind direct helpers, but it
/// cannot substitute a helper for the declared target or combine tests.
fn static_audit_binds_test_subjects(
    record: &AuditRecord,
    test: &vtest_model::TestEntity,
    scan: &ScanResult,
) -> bool {
    let current_test_subjects = record
        .subjects
        .iter()
        .filter_map(|subject| subject.id.as_deref())
        .filter(|id| {
            scan.tests
                .iter()
                .any(|candidate| candidate.id.as_str() == *id)
        })
        .collect::<Vec<_>>();
    if current_test_subjects.len() != 1 || current_test_subjects[0] != test.id.as_str() {
        return false;
    }

    let Some(target) = source_for_target(scan, &test.target) else {
        return false;
    };
    let test_locator = format!("{}::{}", test.location.file, test.location.function);
    let binds_test_code = record.subjects.iter().any(|subject| {
        subject.locator.as_deref() == Some(&test_locator) && subject.hash == test.content_hash
    });
    let binds_target = record.subjects.iter().any(|subject| {
        subject.locator.as_deref() == Some(&target.locator.as_string())
            && subject.hash == target.content_hash
    });
    binds_test_code && binds_target && audit_mentions_config(record)
}

fn source_for_target<'a>(
    scan: &'a ScanResult,
    target: &vtest_model::TargetRef,
) -> Option<&'a vtest_model::SourceFunction> {
    scan.sources.iter().find(|source| match target {
        vtest_model::TargetRef::Locator(locator) => source.locator == *locator,
        vtest_model::TargetRef::SrcId(src_id) => source.src_id.as_ref() == Some(src_id),
    })
}

/// Audit records use RFC 3339 timestamps, whose lexical order differs from
/// chronological order when offsets differ.  `read_audit` validates the
/// format, but retain an invalid-last fallback so an unexpected malformed
/// in-memory record cannot be promoted to the latest result.
fn compare_audit_recency(left: &AuditRecord, right: &AuditRecord) -> std::cmp::Ordering {
    match (
        rfc3339_instant(&left.audited_at),
        rfc3339_instant(&right.audited_at),
    ) {
        (Some(left_time), Some(right_time)) => left_time
            .cmp(&right_time)
            .then_with(|| left.id.cmp(&right.id)),
        (Some(_), None) => std::cmp::Ordering::Greater,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (None, None) => left.id.cmp(&right.id),
    }
}

/// Return a sortable UTC second count plus the fractional second digits.
/// Trimming trailing zeroes makes lexicographic comparison equivalent to a
/// numerical comparison without limiting valid RFC 3339 fractional precision.
fn rfc3339_instant(value: &str) -> Option<(i64, String)> {
    let (date, time_and_zone) = value.split_once('T')?;
    let mut date = date.split('-').map(str::parse::<i64>);
    let (year, month, day) = (date.next()?.ok()?, date.next()?.ok()?, date.next()?.ok()?);
    if date.next().is_some() {
        return None;
    }
    let (time, offset_seconds) = if let Some(time) = time_and_zone.strip_suffix('Z') {
        (time, 0_i64)
    } else {
        let index = time_and_zone
            .char_indices()
            .skip(1)
            .find_map(|(index, ch)| matches!(ch, '+' | '-').then_some(index))?;
        let (time, offset) = time_and_zone.split_at(index);
        let sign = if offset.starts_with('+') {
            1_i64
        } else {
            -1_i64
        };
        let hour = offset.get(1..3)?.parse::<i64>().ok()?;
        let minute = offset.get(4..6)?.parse::<i64>().ok()?;
        (time, sign * (hour * 3_600 + minute * 60))
    };
    let mut parts = time.split(':');
    let hour = parts.next()?.parse::<i64>().ok()?;
    let minute = parts.next()?.parse::<i64>().ok()?;
    let seconds_and_fraction = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    let (second, fraction) = seconds_and_fraction
        .split_once('.')
        .map_or((seconds_and_fraction, ""), |(second, fraction)| {
            (second, fraction)
        });
    let second = second.parse::<i64>().ok()?;
    let fraction = fraction.trim_end_matches('0').to_owned();
    Some((
        days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60 + second
            - offset_seconds,
        fraction,
    ))
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month_from_march = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_from_march + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn audit_text_mentions_test(text: &str, test_id: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim();
        line == format!("id: {test_id}")
            || line == format!("id: '{test_id}'")
            || line == format!("- id: {test_id}")
            || line == format!("- id: '{test_id}'")
    })
}

fn audit_verdict_value(record: &AuditRecord) -> CheckValue {
    match record.verdict.as_str() {
        "PASS" => CheckValue::Pass,
        "FAIL" => CheckValue::Fail,
        "UNKNOWN" => CheckValue::Unknown,
        _ => CheckValue::Unknown,
    }
}

fn audit_subject_is_current(
    layout: &VerifyLayout,
    scan: &ScanResult,
    subject: &AuditSubjectRecord,
) -> bool {
    let actual = match (&subject.id, &subject.locator) {
        (Some(id), None) if id == "CONFIG" => read_text(&layout.config())
            .ok()
            .map(|text| ContentHash::from_text(&text)),
        (Some(id), None) => scan
            .tests
            .iter()
            .find(|test| test.id.as_str() == id)
            .map(|test| test.content_hash.clone()),
        (None, Some(locator)) => Locator::parse(locator).and_then(|locator| {
            scan.sources
                .iter()
                .find(|source| source.locator == locator)
                .map(|source| source.content_hash.clone())
        }),
        _ => None,
    };
    actual.as_ref() == Some(&subject.hash)
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
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };
    use vtest_model::{
        Locator, Revision, ScanSummary, SourceFunction, SourceLocation, TargetRef, TestEntity,
        TestId, TestTarget, VoId,
    };
    use vtest_store::{
        init_project, new_record_id, AuditBasisRecord, AuditReasonRecord, AuditorRecord,
    };

    #[test]
    fn multi_target_evidence_cannot_pass_with_a_single_target_hash() {
        assert!(v01_evidence_binds_all_targets(0));
        assert!(!v01_evidence_binds_all_targets(1));
    }

    fn static_fixture(test_ids: &[&str]) -> (VerifyLayout, ScanResult) {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("vtest-verify-static-{suffix}"));
        let layout = init_project(&root, "fixture").expect("initialise fixture");
        let mut sources = vec![SourceFunction {
            locator: Locator::parse("src/lib.rs::target").expect("valid locator"),
            src_id: None,
            location: location("src/lib.rs", "target"),
            content_hash: ContentHash::from_text("pub fn target() {}"),
        }];
        let tests = test_ids
            .iter()
            .map(|id| TestEntity {
                id: TestId::new(*id),
                covers: vec![VoId::new("VO-ONE")],
                target: TargetRef::Locator(
                    Locator::parse("src/lib.rs::target").expect("valid locator"),
                ),
                additional_targets: Vec::new(),
                intent: "fixture".to_owned(),
                input: None,
                expect: None,
                kind: None,
                cases: Vec::new(),
                related: Vec::new(),
                location: location("tests/static.rs", id),
                content_hash: ContentHash::from_text(&format!("#[test] fn {id}() {{}}")),
                filter: (*id).to_owned(),
                package: "fixture".to_owned(),
                test_target: TestTarget::IntegrationTest("static".to_owned()),
            })
            .collect::<Vec<_>>();
        sources.extend(tests.iter().map(|test| {
            SourceFunction {
                locator: Locator::parse(&format!(
                    "{}::{}",
                    test.location.file, test.location.function
                ))
                .expect("valid test locator"),
                src_id: None,
                location: test.location.clone(),
                content_hash: test.content_hash.clone(),
            }
        }));
        let scan = ScanResult {
            summary: ScanSummary {
                files: 2,
                tests: tests.len(),
                sources: sources.len(),
            },
            tests,
            sources,
            diagnostics: Vec::new(),
        };
        (layout, scan)
    }

    fn location(file: &str, function: &str) -> SourceLocation {
        SourceLocation {
            file: file.to_owned(),
            function: function.to_owned(),
            start_line: 1,
            end_line: 1,
            start_byte: 0,
            end_byte: 1,
        }
    }

    fn write_static_audit(
        layout: &VerifyLayout,
        test: &TestEntity,
        verdict: &str,
        hash: ContentHash,
    ) {
        write_static_audit_with_subjects(layout, verdict, static_audit_subjects(test, hash));
    }

    fn static_audit_subjects(test: &TestEntity, hash: ContentHash) -> Vec<AuditSubjectRecord> {
        let target = match &test.target {
            TargetRef::Locator(locator) => locator,
            TargetRef::SrcId(_) => panic!("fixture uses a locator target"),
        };
        vec![
            AuditSubjectRecord {
                id: Some(test.id.as_str().to_owned()),
                locator: None,
                hash,
            },
            AuditSubjectRecord {
                id: None,
                locator: Some(format!(
                    "{}::{}",
                    test.location.file, test.location.function
                )),
                hash: test.content_hash.clone(),
            },
            AuditSubjectRecord {
                id: None,
                locator: Some(target.as_string()),
                hash: ContentHash::from_text("pub fn target() {}"),
            },
        ]
    }

    fn write_static_audit_with_subjects(
        layout: &VerifyLayout,
        verdict: &str,
        subjects: Vec<AuditSubjectRecord>,
    ) -> String {
        write_static_audit_at(layout, verdict, subjects, "2026-08-08T00:00:00Z")
    }

    fn write_static_audit_at(
        layout: &VerifyLayout,
        verdict: &str,
        mut subjects: Vec<AuditSubjectRecord>,
        audited_at: &str,
    ) -> String {
        let config = read_text(&layout.config()).expect("read fixture config");
        subjects.push(AuditSubjectRecord {
            id: Some("CONFIG".to_owned()),
            locator: None,
            hash: ContentHash::from_text(&config),
        });
        let id = new_record_id();
        let record = AuditRecord {
            id: id.clone(),
            kind: "static".to_owned(),
            bundle_id: None,
            subjects,
            verdict: verdict.to_owned(),
            reasons: ["DA-001", "DA-002", "DA-003", "DA-004", "DA-005", "DA-006"]
                .into_iter()
                .map(|rule| AuditReasonRecord {
                    rule: Some(rule.to_owned()),
                    verdict: Some(if rule == "DA-001" { verdict } else { "PASS" }.to_owned()),
                    claim: format!("fixture result for {rule}"),
                    basis: vec![AuditBasisRecord {
                        kind: "test-code".to_owned(),
                        reference: "tests/static.rs:1".to_owned(),
                    }],
                })
                .collect(),
            exclusions: Vec::new(),
            auditor: AuditorRecord {
                kind: "deterministic".to_owned(),
                id: "vtest-audit".to_owned(),
                model: None,
            },
            confidence: None,
            audited_at: audited_at.to_owned(),
            revision: Revision {
                commit: None,
                dirty: true,
            },
        };
        fs::write(
            layout.audits_dir().join(format!("{id}.yaml")),
            record.to_yaml().expect("serialise static audit"),
        )
        .expect("write static audit");
        id
    }

    #[test]
    fn static_audit_uses_only_current_per_test_records_and_fail_wins() {
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let test = &scan.tests[0];
        write_static_audit(&layout, test, "UNKNOWN", test.content_hash.clone());
        write_static_audit(&layout, test, "PASS", test.content_hash.clone());
        assert_eq!(evaluate_static_audit(&layout, &scan).0, CheckValue::Pass);

        write_static_audit(&layout, test, "FAIL", test.content_hash.clone());
        assert_eq!(evaluate_static_audit(&layout, &scan).0, CheckValue::Fail);
    }

    #[test]
    fn static_audit_becomes_stale_when_assertion_configuration_changes() {
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let test = &scan.tests[0];
        write_static_audit(&layout, test, "PASS", test.content_hash.clone());
        let mut config = read_text(&layout.config()).expect("read fixture config");
        config = config.replace(
            "  assertion_macros: []",
            "  assertion_macros:\n    - assert_valid",
        );
        fs::write(layout.config(), config).expect("change assertion macro configuration");
        assert_eq!(evaluate_static_audit(&layout, &scan).0, CheckValue::Stale);
    }

    #[test]
    fn static_audit_becomes_stale_when_the_test_locator_moves() {
        let (layout, mut scan) = static_fixture(&["TEST-ONE"]);
        write_static_audit(
            &layout,
            &scan.tests[0],
            "PASS",
            scan.tests[0].content_hash.clone(),
        );
        scan.tests[0].location.function = "moved::TEST-ONE".to_owned();
        assert_eq!(evaluate_static_audit(&layout, &scan).0, CheckValue::Stale);
    }

    #[test]
    fn static_audit_marks_hash_mismatch_stale() {
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        write_static_audit(
            &layout,
            &scan.tests[0],
            "PASS",
            ContentHash::from_text("historic test body"),
        );
        assert_eq!(evaluate_static_audit(&layout, &scan).0, CheckValue::Stale);
    }

    #[test]
    fn static_audit_requires_every_bound_subject_to_be_current() {
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        write_static_audit_with_subjects(
            &layout,
            "PASS",
            vec![
                AuditSubjectRecord {
                    id: Some("TEST-ONE".to_owned()),
                    locator: None,
                    hash: scan.tests[0].content_hash.clone(),
                },
                AuditSubjectRecord {
                    id: None,
                    locator: Some("src/lib.rs::target".to_owned()),
                    hash: ContentHash::from_text("historic target body"),
                },
            ],
        );
        assert_eq!(evaluate_static_audit(&layout, &scan).0, CheckValue::Stale);
    }

    #[test]
    fn static_audit_requires_the_exact_declared_target_subject() {
        let (layout, mut scan) = static_fixture(&["TEST-ONE"]);
        let test = &scan.tests[0];
        write_static_audit_with_subjects(
            &layout,
            "PASS",
            vec![AuditSubjectRecord {
                id: Some(test.id.as_str().to_owned()),
                locator: None,
                hash: test.content_hash.clone(),
            }],
        );
        assert_eq!(evaluate_static_audit(&layout, &scan).0, CheckValue::Stale);

        let helper = SourceFunction {
            locator: Locator::parse("src/lib.rs::helper").expect("valid helper locator"),
            src_id: None,
            location: location("src/lib.rs", "helper"),
            content_hash: ContentHash::from_text("pub fn helper() {}"),
        };
        scan.sources.push(helper.clone());
        write_static_audit_with_subjects(
            &layout,
            "PASS",
            vec![
                AuditSubjectRecord {
                    id: Some(test.id.as_str().to_owned()),
                    locator: None,
                    hash: test.content_hash.clone(),
                },
                AuditSubjectRecord {
                    id: None,
                    locator: Some(helper.locator.as_string()),
                    hash: helper.content_hash,
                },
            ],
        );
        assert_eq!(evaluate_static_audit(&layout, &scan).0, CheckValue::Stale);
    }

    #[test]
    fn static_audit_rejects_a_record_that_binds_multiple_tests() {
        let (layout, scan) = static_fixture(&["TEST-ONE", "TEST-TWO"]);
        let id = write_static_audit_with_subjects(
            &layout,
            "PASS",
            static_audit_subjects(&scan.tests[0], scan.tests[0].content_hash.clone()),
        );
        let path = layout.audits_dir().join(format!("{id}.yaml"));
        let yaml = fs::read_to_string(&path).expect("read static audit");
        let second_subject = format!(
            "  - id: '{}'\n    hash: '{}'\n",
            scan.tests[1].id, scan.tests[1].content_hash
        );
        fs::write(
            &path,
            yaml.replace("verdict:", &format!("{second_subject}verdict:")),
        )
        .expect("write multi-test static audit");
        assert_eq!(evaluate_static_audit(&layout, &scan).0, CheckValue::Unknown);
    }

    #[test]
    fn static_audit_orders_offsets_by_the_actual_rfc3339_instant() {
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let test = &scan.tests[0];
        // 09:00+09:00 is 00:00Z.  It is lexically later than 00:30Z but
        // chronologically earlier, so the later PASS must be selected.
        write_static_audit_at(
            &layout,
            "UNKNOWN",
            static_audit_subjects(test, test.content_hash.clone()),
            "2026-08-08T09:00:00+09:00",
        );
        write_static_audit_at(
            &layout,
            "PASS",
            static_audit_subjects(test, test.content_hash.clone()),
            "2026-08-08T00:30:00Z",
        );
        assert_eq!(evaluate_static_audit(&layout, &scan).0, CheckValue::Pass);
    }

    #[test]
    fn static_audit_requires_an_audit_for_every_registered_test() {
        let (layout, scan) = static_fixture(&["TEST-ONE", "TEST-TWO"]);
        write_static_audit(
            &layout,
            &scan.tests[0],
            "PASS",
            scan.tests[0].content_hash.clone(),
        );
        assert_eq!(
            evaluate_static_audit(&layout, &scan).0,
            CheckValue::NotChecked
        );
    }

    #[test]
    fn malformed_static_audit_is_unknown_not_silently_ignored() {
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let id = new_record_id();
        fs::write(
            layout.audits_dir().join(format!("{id}.yaml")),
            format!(
                "id: {id}\nkind: static\nbundle_id: null\nsubjects:\n  - id: TEST-ONE\n    hash: invalid\nverdict: PASS\n"
            ),
        )
        .expect("write malformed static audit");
        let result = evaluate_static_audit(&layout, &scan);
        assert_eq!(result.0, CheckValue::Unknown, "basis: {:?}", result.1);
    }
}
