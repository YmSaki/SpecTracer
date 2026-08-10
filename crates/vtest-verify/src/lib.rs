//! Fail-closed aggregation and report boundary (M6).

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use serde::Serialize;
use vtest_model::{
    CheckValue, ContentHash, Diagnostic, EvidenceRecord, ScanSummary, TargetRef, TestEntity,
};
use vtest_scan::ScanResult;
use vtest_store::{
    read_approval, read_audit, read_evidence, read_record_ids, read_req, read_text, read_vo,
    yaml_scalar_value, AuditRecord, AuditSubjectRecord, ProjectConfig, ReqRecord, VerifyLayout,
    VoRecord,
};

pub const ALL_ITEMS: [&str; 12] = [
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
    "test_traceability",
];

/// Entity-axis scope for verification.  The item-axis scope remains the
/// `requested_scope` argument; keeping the axes separate prevents a selected
/// Test/VO/REQ from accidentally broadening a request to the whole project.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "lowercase")]
pub enum EntityScope {
    Req(String),
    Vo(String),
    Test(String),
}

#[derive(Clone, Debug, Serialize)]
pub struct ReportItem {
    pub item: String,
    pub value: CheckValue,
    pub basis: Vec<String>,
}

/// Deterministic REQ → VO → Test detail tree used by both `verify` and
/// `report`.  The item list remains the machine-friendly aggregate; this tree
/// gives a human or agent a stable path to each selected entity and preserves
/// the same fail-closed values at every node.
#[derive(Clone, Debug, Serialize)]
pub struct VerificationNode {
    pub kind: String,
    pub id: String,
    pub value: CheckValue,
    pub items: Vec<ReportItem>,
    pub children: Vec<VerificationNode>,
}

#[derive(Clone, Debug, Serialize)]
pub struct VerificationReport {
    pub requested_scope: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_scope: Option<EntityScope>,
    pub scope_outside_not_checked: bool,
    pub items: Vec<ReportItem>,
    pub tree: Vec<VerificationNode>,
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
    verify_project_scoped(root, scan, config, requested_scope, None)
}

/// Verify the requested item scope and, optionally, a REQ/VO/Test subtree.
///
/// This is intentionally the single aggregation entry point used by the CLI;
/// the legacy `verify_project` wrapper retains the project-wide API for callers
/// that do not need an entity selector.
pub fn verify_project_scoped(
    root: &Path,
    scan: &ScanResult,
    config: &ProjectConfig,
    requested_scope: Option<Vec<String>>,
    entity_scope: Option<EntityScope>,
) -> VerificationResult {
    let requested_scope = requested_scope.unwrap_or_else(|| config.verify.full_scope.clone());
    let layout = VerifyLayout::new(root);
    let selection = ScopeSelection::new(&layout, scan, entity_scope.clone());
    let scoped_scan = selection.scope_scan(scan);
    let diagnostics = scan.diagnostics.clone();
    let evidence = selection.scope_evidence(read_evidence_records(&layout));
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
        let (value, basis) =
            evaluate_item(root, &layout, &scoped_scan, item, &evidence, &selection);
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
    let tree = build_tree(&layout, &selection, &scoped_scan, &items);
    let scope_outside_not_checked =
        requested_scope.len() < ALL_ITEMS.len() || entity_scope.is_some();
    VerificationResult {
        report: VerificationReport {
            requested_scope,
            scope_outside_not_checked,
            entity_scope,
            items,
            tree,
            result,
        },
        diagnostics,
    }
}

#[derive(Clone, Debug)]
struct ScopeSelection {
    entity_scope: Option<EntityScope>,
    test_ids: BTreeSet<String>,
    vo_ids: BTreeSet<String>,
    req_ids: BTreeSet<String>,
}

impl ScopeSelection {
    fn new(layout: &VerifyLayout, scan: &ScanResult, entity_scope: Option<EntityScope>) -> Self {
        let all_test_ids = scan
            .tests
            .iter()
            .map(|test| test.id.as_str().to_owned())
            .collect::<BTreeSet<_>>();
        let vo_records = read_record_ids(&layout.vo_dir())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|id| read_vo(layout, &id).ok().map(|record| (id, record)))
            .collect::<BTreeMap<_, _>>();
        let req_records = read_record_ids(&layout.req_dir())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|id| read_req(layout, &id).ok().map(|record| (id, record)))
            .collect::<BTreeMap<_, _>>();
        if entity_scope.is_none() {
            return Self {
                entity_scope,
                test_ids: all_test_ids,
                vo_ids: vo_records.keys().cloned().collect(),
                req_ids: req_records.keys().cloned().collect(),
            };
        }

        let mut test_ids = BTreeSet::new();
        let mut vo_ids = BTreeSet::new();
        let mut req_ids = BTreeSet::new();
        match entity_scope.as_ref().expect("checked above") {
            EntityScope::Test(id) => {
                if all_test_ids.contains(id) {
                    test_ids.insert(id.clone());
                    if let Some(test) = scan.tests.iter().find(|test| test.id.as_str() == id) {
                        vo_ids.extend(test.covers.iter().map(|vo| vo.as_str().to_owned()));
                    }
                }
            }
            EntityScope::Vo(id) => {
                vo_ids.insert(id.clone());
                include_vo_descendants(&vo_records, &mut vo_ids);
                test_ids.extend(
                    scan.tests
                        .iter()
                        .filter(|test| test.covers.iter().any(|vo| vo_ids.contains(vo.as_str())))
                        .map(|test| test.id.as_str().to_owned()),
                );
            }
            EntityScope::Req(id) => {
                req_ids.insert(id.clone());
                include_req_descendants(&req_records, &mut req_ids);
                vo_ids.extend(
                    vo_records
                        .iter()
                        .filter(|(_, vo)| {
                            vo.requirements
                                .iter()
                                .any(|req| req_ids.contains(req.as_str()))
                        })
                        .map(|(vo_id, _)| vo_id.clone()),
                );
                include_vo_descendants(&vo_records, &mut vo_ids);
                test_ids.extend(
                    scan.tests
                        .iter()
                        .filter(|test| test.covers.iter().any(|vo| vo_ids.contains(vo.as_str())))
                        .map(|test| test.id.as_str().to_owned()),
                );
            }
        }
        for vo_id in &vo_ids {
            if let Some(vo) = vo_records.get(vo_id) {
                req_ids.extend(vo.requirements.iter().map(|req| req.as_str().to_owned()));
            }
        }
        Self {
            entity_scope,
            test_ids,
            vo_ids,
            req_ids,
        }
    }

    fn scope_scan(&self, scan: &ScanResult) -> ScanResult {
        if self.entity_scope.is_none() {
            return scan.clone();
        }
        let tests = scan
            .tests
            .iter()
            .filter(|test| self.test_ids.contains(test.id.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        ScanResult {
            summary: ScanSummary {
                files: scan.summary.files,
                tests: tests.len(),
                sources: scan.sources.len(),
            },
            tests,
            sources: scan.sources.clone(),
            diagnostics: scan.diagnostics.clone(),
        }
    }

    fn scope_evidence(
        &self,
        evidence: BTreeMap<String, EvidenceRecord>,
    ) -> BTreeMap<String, EvidenceRecord> {
        if self.entity_scope.is_none() {
            return evidence;
        }
        evidence
            .into_iter()
            .filter(|(test_id, _)| self.test_ids.contains(test_id))
            .collect()
    }
}

fn include_vo_descendants(records: &BTreeMap<String, VoRecord>, selected: &mut BTreeSet<String>) {
    loop {
        let before = selected.len();
        let additions = records
            .iter()
            .filter(|(_, vo)| {
                vo.parent
                    .as_ref()
                    .is_some_and(|parent| selected.contains(parent.as_str()))
            })
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        selected.extend(additions);
        if selected.len() == before {
            break;
        }
    }
}

fn include_req_descendants(records: &BTreeMap<String, ReqRecord>, selected: &mut BTreeSet<String>) {
    loop {
        let before = selected.len();
        let additions = records
            .iter()
            .filter(|(_, req)| {
                req.parent
                    .as_ref()
                    .is_some_and(|parent| selected.contains(parent.as_str()))
            })
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        selected.extend(additions);
        if selected.len() == before {
            break;
        }
    }
}

fn build_tree(
    layout: &VerifyLayout,
    selection: &ScopeSelection,
    scan: &ScanResult,
    items: &[ReportItem],
) -> Vec<VerificationNode> {
    let values = items
        .iter()
        .map(|item| (item.item.as_str(), item))
        .collect::<BTreeMap<_, _>>();
    let reqs = selection
        .req_ids
        .iter()
        .filter_map(|id| read_req(layout, id).ok().map(|record| (id.clone(), record)))
        .collect::<BTreeMap<_, _>>();
    let vos = selection
        .vo_ids
        .iter()
        .filter_map(|id| read_vo(layout, id).ok().map(|record| (id.clone(), record)))
        .collect::<BTreeMap<_, _>>();

    let mut roots = Vec::new();
    let mut attached_vos = BTreeSet::new();
    let mut attached_reqs = BTreeSet::new();
    for (id, req) in &reqs {
        if req
            .parent
            .as_ref()
            .is_some_and(|parent| reqs.contains_key(parent.as_str()))
        {
            continue;
        }
        attached_reqs.insert(id.clone());
        roots.push(build_req_node(
            id,
            &reqs,
            &vos,
            scan,
            &values,
            &mut attached_vos,
            &mut BTreeSet::new(),
        ));
    }
    // A VO whose requirement is not selected (or which has no REQ) remains a
    // top-level graph node rather than disappearing from a limited report.
    for (id, vo) in &vos {
        if vo.parent.is_none() && !attached_vos.contains(id) {
            roots.push(build_vo_node(id, &vos, scan, &values, &mut attached_vos));
        }
    }
    for test in &scan.tests {
        if !test
            .covers
            .iter()
            .any(|vo| attached_vos.contains(vo.as_str()))
        {
            roots.push(test_node(test, &values));
        }
    }
    if roots.is_empty() {
        roots.push(VerificationNode {
            kind: "scope".to_owned(),
            id: selection
                .entity_scope
                .as_ref()
                .map(entity_scope_id)
                .unwrap_or_else(|| "PROJECT".to_owned()),
            value: CheckValue::NotChecked,
            items: items.to_vec(),
            children: Vec::new(),
        });
    }
    // Keep this binding explicit: it documents that REQ records were loaded
    // for the tree even when the selected graph contains only orphan VOs.
    let _ = attached_reqs;
    roots
}

fn build_req_node(
    id: &str,
    reqs: &BTreeMap<String, ReqRecord>,
    vos: &BTreeMap<String, VoRecord>,
    scan: &ScanResult,
    values: &BTreeMap<&str, &ReportItem>,
    attached_vos: &mut BTreeSet<String>,
    visiting: &mut BTreeSet<String>,
) -> VerificationNode {
    if !visiting.insert(id.to_owned()) {
        return VerificationNode {
            kind: "req".to_owned(),
            id: id.to_owned(),
            value: CheckValue::Unknown,
            items: item_copies(values, &["spec_coverage"]),
            children: Vec::new(),
        };
    }
    let mut children = Vec::new();
    for (child_id, child) in reqs {
        if child
            .parent
            .as_ref()
            .is_some_and(|parent| parent.as_str() == id)
        {
            children.push(build_req_node(
                child_id,
                reqs,
                vos,
                scan,
                values,
                attached_vos,
                visiting,
            ));
        }
    }
    for (vo_id, vo) in vos {
        if vo.parent.is_none() && vo.requirements.iter().any(|req| req.as_str() == id) {
            children.push(build_vo_node(vo_id, vos, scan, values, attached_vos));
        }
    }
    visiting.remove(id);
    let items = item_copies(values, &["spec_coverage"]);
    node_with_children("req", id, items, children)
}

fn build_vo_node(
    id: &str,
    vos: &BTreeMap<String, VoRecord>,
    scan: &ScanResult,
    values: &BTreeMap<&str, &ReportItem>,
    attached_vos: &mut BTreeSet<String>,
) -> VerificationNode {
    if !attached_vos.insert(id.to_owned()) {
        return VerificationNode {
            kind: "vo".to_owned(),
            id: id.to_owned(),
            value: CheckValue::Unknown,
            items: item_copies(
                values,
                &["vo_decomposition", "vo_coverage", "test_existence"],
            ),
            children: Vec::new(),
        };
    }
    let mut children = Vec::new();
    for (child_id, child) in vos {
        if child
            .parent
            .as_ref()
            .is_some_and(|parent| parent.as_str() == id)
        {
            children.push(build_vo_node(child_id, vos, scan, values, attached_vos));
        }
    }
    for test in &scan.tests {
        if test.covers.iter().any(|vo| vo.as_str() == id) {
            children.push(test_node(test, values));
        }
    }
    let items = item_copies(
        values,
        &["vo_decomposition", "vo_coverage", "test_existence"],
    );
    node_with_children("vo", id, items, children)
}

fn test_node(test: &TestEntity, values: &BTreeMap<&str, &ReportItem>) -> VerificationNode {
    let items = item_copies(
        values,
        &[
            "static_audit",
            "semantic_audit",
            "impl_consistency",
            "test_execution",
            "runtime_result",
            "target_execution",
            "evidence_validity",
        ],
    );
    node_with_children("test", test.id.as_str(), items, Vec::new())
}

fn item_copies<'a>(values: &BTreeMap<&'a str, &'a ReportItem>, keys: &[&str]) -> Vec<ReportItem> {
    keys.iter()
        .filter_map(|key| values.get(key).map(|item| (*item).clone()))
        .collect()
}

fn node_with_children(
    kind: &str,
    id: &str,
    items: Vec<ReportItem>,
    children: Vec<VerificationNode>,
) -> VerificationNode {
    let value = items
        .iter()
        .map(|item| item.value)
        .chain(children.iter().map(|child| child.value))
        .fold(CheckValue::Pass, combine_values);
    VerificationNode {
        kind: kind.to_owned(),
        id: id.to_owned(),
        value,
        items,
        children,
    }
}

fn entity_scope_id(scope: &EntityScope) -> String {
    match scope {
        EntityScope::Req(id) | EntityScope::Vo(id) | EntityScope::Test(id) => id.clone(),
    }
}

fn evaluate_item(
    root: &Path,
    layout: &VerifyLayout,
    scan: &ScanResult,
    item: &str,
    evidence: &BTreeMap<String, EvidenceRecord>,
    selection: &ScopeSelection,
) -> (CheckValue, Vec<String>) {
    match item {
        "spec_coverage" => {
            let reqs = selection
                .req_ids
                .iter()
                .filter_map(|id| read_req(layout, id).ok())
                .collect::<Vec<_>>();
            let vos = selection
                .vo_ids
                .iter()
                .filter_map(|id| read_vo(layout, id).ok())
                .collect::<Vec<_>>();
            let missing = reqs
                .iter()
                .filter(|req| {
                    !vos.iter()
                        .any(|vo| vo.requirements.iter().any(|candidate| candidate == &req.id))
                })
                .map(|req| req.id.as_str().to_owned())
                .collect::<Vec<_>>();
            if !reqs.is_empty() && missing.is_empty() {
                (
                    CheckValue::Pass,
                    vec!["every selected REQ has at least one linked VO".to_owned()],
                )
            } else {
                (
                    CheckValue::Missing,
                    if reqs.is_empty() {
                        vec!["no REQ records exist in the selected scope".to_owned()]
                    } else {
                        vec![format!(
                            "selected REQ record(s) have no linked VO: {}",
                            missing.join(", ")
                        )]
                    },
                )
            }
        }
        "vo_decomposition" => {
            if scan.diagnostics.iter().any(Diagnostic::is_error) {
                (
                    CheckValue::Fail,
                    vec!["scan emitted an error diagnostic".to_owned()],
                )
            } else if selection.vo_ids.is_empty() {
                (CheckValue::Missing, vec!["no VO records exist".to_owned()])
            } else {
                (
                    CheckValue::Pass,
                    vec!["scan and canonical VO records are structurally readable".to_owned()],
                )
            }
        }
        "vo_coverage" => evaluate_vo_coverage(root, layout, scan, selection),
        "test_existence" => {
            let vos = selection
                .vo_ids
                .iter()
                .filter_map(|id| read_vo(layout, id).ok())
                .collect::<Vec<_>>();
            let child_ids = vos
                .iter()
                .filter_map(|vo| vo.parent.as_ref().map(|parent| parent.as_str().to_owned()))
                .collect::<BTreeSet<_>>();
            let leaves = vos
                .iter()
                .filter(|vo| !child_ids.contains(vo.id.as_str()))
                .collect::<Vec<_>>();
            let missing = leaves
                .iter()
                .filter(|vo| {
                    !scan.tests.iter().any(|test| {
                        test.covers
                            .iter()
                            .any(|covered| covered.as_str() == vo.id.as_str())
                    })
                })
                .map(|vo| vo.id.as_str().to_owned())
                .collect::<Vec<_>>();
            if !leaves.is_empty() && missing.is_empty() {
                (
                    CheckValue::Pass,
                    vec![format!(
                        "all {} leaf VO(s) have a covering Test",
                        leaves.len()
                    )],
                )
            } else {
                (
                    CheckValue::Missing,
                    if leaves.is_empty() {
                        vec!["no leaf VO records exist in the selected scope".to_owned()]
                    } else {
                        vec![format!(
                            "leaf VO(s) have no covering Test: {}",
                            missing.join(", ")
                        )]
                    },
                )
            }
        }
        "static_audit" => evaluate_static_audit(layout, scan),
        "semantic_audit" => evaluate_test_audit(root, layout, scan, "test-semantic"),
        "impl_consistency" => evaluate_test_audit(root, layout, scan, "impl-consistency"),
        "test_execution" => evaluate_test_execution(evidence, scan),
        "runtime_result" => evaluate_runtime(evidence, scan),
        "target_execution" => evaluate_target_execution(evidence, scan),
        "evidence_validity" => evaluate_evidence_validity(evidence, scan),
        "test_traceability" => evaluate_test_traceability(scan),
        _ => (CheckValue::Unknown, vec!["unknown check item".to_owned()]),
    }
}

fn evaluate_test_traceability(scan: &ScanResult) -> (CheckValue, Vec<String>) {
    let mismatch = scan
        .diagnostics
        .iter()
        .filter(|diagnostic| matches!(diagnostic.code.as_str(), "E-SCAN-002" | "E-SCAN-003"))
        .map(|diagnostic| diagnostic.message.clone())
        .collect::<Vec<_>>();
    if !mismatch.is_empty() {
        return (CheckValue::Mismatch, mismatch);
    }
    let missing = scan
        .diagnostics
        .iter()
        .filter(|diagnostic| matches!(diagnostic.code.as_str(), "W-SCAN-101" | "E-SCAN-007"))
        .map(|diagnostic| diagnostic.message.clone())
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return (CheckValue::Missing, missing);
    }
    (
        CheckValue::Pass,
        vec![format!(
            "all {} discovered managed Test(s) are structurally traceable",
            scan.tests.len()
        )],
    )
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
                " (current static audit record(s): {})",
                valid
                    .iter()
                    .map(|record| record.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
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

    let Some(target_ref) = test.targets.first() else {
        return false;
    };
    let Some(target) = source_for_target(scan, target_ref) else {
        return false;
    };
    let test_locator = TargetRef::Locator {
        adapter: test.location.adapter.clone(),
        value: format!("{}::{}", test.location.path, test.location.locator),
    }
    .normalized();
    let binds_test_code = record.subjects.iter().any(|subject| {
        subject.locator.as_deref() == Some(&test_locator) && subject.hash == test.content_hash
    });
    let binds_target = record.subjects.iter().any(|subject| {
        subject.locator.as_deref() == Some(&target.target.normalized())
            && subject.hash == target.content_hash
    });
    binds_test_code && binds_target && audit_mentions_config(record)
}

fn source_for_target<'a>(
    scan: &'a ScanResult,
    target: &vtest_model::TargetRef,
) -> Option<&'a vtest_model::SourceFunction> {
    scan.sources.iter().find(|source| match target {
        vtest_model::TargetRef::Locator { .. } => source.target == *target,
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
        (None, Some(locator)) => scan
            .sources
            .iter()
            .find(|source| source.target.normalized() == *locator)
            .map(|source| source.content_hash.clone()),
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

fn evaluate_vo_coverage(
    root: &Path,
    layout: &VerifyLayout,
    scan: &ScanResult,
    selection: &ScopeSelection,
) -> (CheckValue, Vec<String>) {
    if selection.vo_ids.is_empty() {
        return (
            CheckValue::NotChecked,
            vec!["no VO records are available in the selected scope".to_owned()],
        );
    }
    let mut per_vo: BTreeMap<String, Vec<(String, bool, CheckValue)>> = BTreeMap::new();
    for id in read_record_ids(&layout.audits_dir()).unwrap_or_default() {
        let path = layout.audits_dir().join(format!("{id}.yaml"));
        let Ok(text) = read_text(&path) else { continue };
        if yaml_scalar_value(&text, "kind").as_deref() != Some("vo-coverage") {
            continue;
        }
        let subjects = parse_audit_subjects(&text);
        let vo_ids = subjects
            .iter()
            .filter(|subject| subject.kind == "vo")
            .filter_map(|subject| subject.id.as_deref())
            .filter(|id| selection.vo_ids.contains(*id))
            .map(str::to_owned)
            .collect::<BTreeSet<_>>();
        if vo_ids.is_empty() {
            continue;
        }
        let valid = !subjects.is_empty()
            && subjects
                .iter()
                .all(|subject| subject_is_current(root, layout, scan, subject));
        let verdict = match yaml_scalar_value(&text, "verdict").as_deref() {
            Some("PASS") => CheckValue::Pass,
            Some("FAIL") => CheckValue::Fail,
            Some("UNKNOWN") => CheckValue::Unknown,
            _ => CheckValue::Unknown,
        };
        for vo_id in vo_ids {
            per_vo
                .entry(vo_id)
                .or_default()
                .push((id.clone(), valid, verdict));
        }
    }

    let mut overall = CheckValue::Pass;
    let mut basis = Vec::new();
    for vo_id in &selection.vo_ids {
        let records = per_vo.get(vo_id).cloned().unwrap_or_default();
        let current = if records.is_empty() {
            CheckValue::NotChecked
        } else {
            let valid = records
                .iter()
                .filter(|(_, current, _)| *current)
                .collect::<Vec<_>>();
            if valid.is_empty() {
                CheckValue::Stale
            } else {
                let audit_value = valid
                    .iter()
                    .map(|(_, _, verdict)| *verdict)
                    .fold(CheckValue::Pass, combine_values);
                if audit_value == CheckValue::Pass && !vo_is_approved(layout, vo_id) {
                    CheckValue::Missing
                } else {
                    audit_value
                }
            }
        };
        overall = combine_values(overall, current);
        let record_basis = records
            .iter()
            .map(|(id, valid, _)| format!("{id}{}", if *valid { "" } else { " (stale)" }))
            .collect::<Vec<_>>();
        if records.is_empty() {
            basis.push(format!("VO {vo_id}: {current:?} (coverage audit missing)"));
        } else {
            basis.push(format!(
                "VO {vo_id}: {current:?} (coverage audits: {})",
                record_basis.join(", ")
            ));
        }
    }
    (overall, basis)
}

fn evaluate_test_audit(
    root: &Path,
    layout: &VerifyLayout,
    scan: &ScanResult,
    kind: &str,
) -> (CheckValue, Vec<String>) {
    if scan.tests.is_empty() {
        return (
            CheckValue::NotChecked,
            vec![format!("no registered Tests are available for {kind}")],
        );
    }
    let mut per_test: BTreeMap<String, Vec<(String, bool, CheckValue)>> = BTreeMap::new();
    for id in read_record_ids(&layout.audits_dir()).unwrap_or_default() {
        let path = layout.audits_dir().join(format!("{id}.yaml"));
        let Ok(text) = read_text(&path) else { continue };
        if yaml_scalar_value(&text, "kind").as_deref() != Some(kind) {
            continue;
        }
        let subjects = parse_audit_subjects(&text);
        let test_ids = subjects
            .iter()
            .filter(|subject| subject.kind == "test")
            .filter_map(|subject| subject.id.as_deref())
            .filter(|id| scan.tests.iter().any(|test| test.id.as_str() == *id))
            .map(str::to_owned)
            .collect::<BTreeSet<_>>();
        if test_ids.is_empty() {
            continue;
        }
        let valid = !subjects.is_empty()
            && subjects
                .iter()
                .all(|subject| subject_is_current(root, layout, scan, subject));
        let verdict = match yaml_scalar_value(&text, "verdict").as_deref() {
            Some("PASS") => CheckValue::Pass,
            Some("FAIL") => CheckValue::Fail,
            Some("UNKNOWN") => CheckValue::Unknown,
            _ => CheckValue::Unknown,
        };
        for test_id in test_ids {
            per_test
                .entry(test_id)
                .or_default()
                .push((id.clone(), valid, verdict));
        }
    }

    let mut overall = CheckValue::Pass;
    let mut basis = Vec::new();
    for test in &scan.tests {
        let test_id = test.id.as_str();
        let records = per_test.get(test_id).cloned().unwrap_or_default();
        let current = if records.is_empty() {
            CheckValue::NotChecked
        } else {
            let valid = records
                .iter()
                .filter(|(_, current, _)| *current)
                .collect::<Vec<_>>();
            if valid.is_empty() {
                CheckValue::Stale
            } else {
                valid
                    .iter()
                    .map(|(_, _, verdict)| *verdict)
                    .fold(CheckValue::Pass, combine_values)
            }
        };
        overall = combine_values(overall, current);
        if records.is_empty() {
            basis.push(format!("Test {test_id}: {current:?} (audit missing)"));
        } else {
            let record_basis = records
                .iter()
                .map(|(id, valid, _)| format!("{id}{}", if *valid { "" } else { " (stale)" }))
                .collect::<Vec<_>>();
            basis.push(format!(
                "Test {test_id}: {current:?} (audits: {})",
                record_basis.join(", ")
            ));
        }
    }
    (overall, basis)
}

fn vo_is_approved(layout: &VerifyLayout, id: &str) -> bool {
    let Ok(text) = read_text(&layout.vo_dir().join(format!("{id}.yaml"))) else {
        return false;
    };
    let current_hash = ContentHash::from_text(&text);
    let Ok(entries) = fs::read_dir(layout.approvals_dir()) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let path = entry.path();
        path.extension().and_then(|value| value.to_str()) == Some("yaml")
            && read_approval(&path).is_ok_and(|approval| {
                approval.subject.as_str() == id && approval.subject_hash == current_hash
            })
    })
}

fn parse_audit_subjects(text: &str) -> Vec<AuditSubject> {
    let mut subjects = Vec::new();
    let mut current: Option<AuditSubject> = None;
    let mut in_subjects = false;
    for raw in text.lines() {
        if !raw.starts_with([' ', '\t']) {
            if raw.trim() == "subjects:" {
                in_subjects = true;
                continue;
            }
            if in_subjects {
                break;
            }
            continue;
        }
        if !in_subjects {
            continue;
        }
        if raw.starts_with("  - ") {
            if let Some(subject) = current.take() {
                subjects.push(normalize_audit_subject(subject));
            }
            let mut subject = AuditSubject {
                kind: String::new(),
                id: None,
                locator: None,
                hash: None,
            };
            let line = raw.trim_start().strip_prefix("- ").unwrap_or_default();
            if let Some(value) = line.strip_prefix("kind:") {
                subject.kind =
                    yaml_line_value(&format!("kind:{value}"), "kind").unwrap_or_default();
            } else if let Some(value) = line.strip_prefix("id:") {
                subject.id = yaml_line_value(&format!("id:{value}"), "id");
            } else if let Some(value) = line.strip_prefix("locator:") {
                subject.locator = yaml_line_value(&format!("locator:{value}"), "locator");
            }
            current = Some(subject);
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
        subjects.push(normalize_audit_subject(subject));
    }
    subjects
}

fn normalize_audit_subject(mut subject: AuditSubject) -> AuditSubject {
    if subject.kind.is_empty() {
        subject.kind = if subject.locator.is_some() {
            "target".to_owned()
        } else if subject
            .id
            .as_deref()
            .is_some_and(|id| id.starts_with("TEST-"))
        {
            "test".to_owned()
        } else if subject
            .id
            .as_deref()
            .is_some_and(|id| id.starts_with("VO-"))
        {
            "vo".to_owned()
        } else if subject
            .id
            .as_deref()
            .is_some_and(|id| id.starts_with("REQ-"))
        {
            "req".to_owned()
        } else if subject
            .id
            .as_deref()
            .is_some_and(|id| id.starts_with("SPEC-"))
        {
            "spec".to_owned()
        } else if subject.id.as_deref() == Some("CONFIG") {
            "config".to_owned()
        } else {
            String::new()
        };
    }
    subject
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
        "target" => subject.locator.as_deref().and_then(|locator| {
            scan.sources
                .iter()
                .find(|source| source.target.normalized() == locator)
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
        "config" => read_text(&layout.config())
            .ok()
            .map(|text| ContentHash::from_text(&text)),
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
    if scan.tests.is_empty() {
        return (
            CheckValue::NotChecked,
            vec!["no registered Tests are available in the selected scope".to_owned()],
        );
    }
    let mut value = CheckValue::Pass;
    let mut basis = Vec::new();
    for test in &scan.tests {
        let current = evidence
            .get(test.id.as_str())
            .map_or(
                CheckValue::NotExecuted,
                |record| match evidence_record_validity(record, test, scan) {
                    CheckValue::Pass => match record.result {
                        vtest_model::TestResult::Pass => CheckValue::Pass,
                        vtest_model::TestResult::Fail => CheckValue::Fail,
                    },
                    _ => CheckValue::Stale,
                },
            );
        value = combine_values(value, current);
        basis.push(evidence_basis(
            test,
            evidence.get(test.id.as_str()),
            current,
        ));
    }
    (value, basis)
}

fn evaluate_target_execution(
    evidence: &BTreeMap<String, EvidenceRecord>,
    scan: &ScanResult,
) -> (CheckValue, Vec<String>) {
    if scan.tests.is_empty() {
        return (
            CheckValue::NotChecked,
            vec!["no registered Tests are available in the selected scope".to_owned()],
        );
    }
    let mut value = CheckValue::Pass;
    let mut basis = Vec::new();
    for test in &scan.tests {
        let current = evidence
            .get(test.id.as_str())
            .map_or(CheckValue::NotExecuted, |record| {
                if evidence_record_validity(record, test, scan) != CheckValue::Pass {
                    CheckValue::Stale
                } else if record.target_execution.checked {
                    record
                        .target_execution
                        .result
                        .unwrap_or(CheckValue::Unknown)
                } else {
                    CheckValue::NotChecked
                }
            });
        value = combine_values(value, current);
        basis.push(evidence_basis(
            test,
            evidence.get(test.id.as_str()),
            current,
        ));
    }
    (value, basis)
}

fn evaluate_evidence_validity(
    evidence: &BTreeMap<String, EvidenceRecord>,
    scan: &ScanResult,
) -> (CheckValue, Vec<String>) {
    if scan.tests.is_empty() {
        return (
            CheckValue::NotChecked,
            vec!["no registered Tests are available in the selected scope".to_owned()],
        );
    }
    let mut value = CheckValue::Pass;
    let mut basis =
        vec!["Evidence hashes and revision are checked against current scan".to_owned()];
    for test in &scan.tests {
        let current = evidence
            .get(test.id.as_str())
            .map_or(CheckValue::NotExecuted, |record| {
                evidence_record_validity(record, test, scan)
            });
        value = combine_values(value, current);
        basis.push(evidence_basis(
            test,
            evidence.get(test.id.as_str()),
            current,
        ));
        if let Some(record) = evidence.get(test.id.as_str()) {
            if test.targets.len() > 1 && record.hashes.targets.is_empty() {
                basis.push(format!(
                    "Test {} has multiple targets but Evidence has no neutral targets list",
                    record.test_id
                ));
            }
        }
    }
    (value, basis)
}

fn evaluate_test_execution(
    evidence: &BTreeMap<String, EvidenceRecord>,
    scan: &ScanResult,
) -> (CheckValue, Vec<String>) {
    if scan.tests.is_empty() {
        return (
            CheckValue::NotChecked,
            vec!["no registered Tests are available in the selected scope".to_owned()],
        );
    }
    let mut value = CheckValue::Pass;
    let mut basis = Vec::new();
    for test in &scan.tests {
        let current = evidence
            .get(test.id.as_str())
            .map_or(CheckValue::NotExecuted, |record| {
                evidence_record_validity(record, test, scan)
            });
        value = combine_values(value, current);
        basis.push(evidence_basis(
            test,
            evidence.get(test.id.as_str()),
            current,
        ));
    }
    (value, basis)
}

fn evidence_basis(test: &TestEntity, record: Option<&EvidenceRecord>, value: CheckValue) -> String {
    record.map_or_else(
        || format!("Test {}: {value:?} (Evidence missing)", test.id),
        |record| {
            format!(
                "Test {}: {value:?} (Evidence {}, log_ref {})",
                test.id, record.id, record.log_ref
            )
        },
    )
}

fn evidence_record_validity(
    record: &EvidenceRecord,
    test: &TestEntity,
    scan: &ScanResult,
) -> CheckValue {
    let Some(adapter) = record.adapter.as_ref() else {
        return CheckValue::Stale;
    };
    if adapter != &test.execution.adapter {
        return CheckValue::Mismatch;
    }
    let Some(execution_state) = record.execution_state.as_ref() else {
        return CheckValue::Stale;
    };
    if !execution_state.complete || execution_state.hash.is_none() {
        return CheckValue::Unknown;
    }
    let Some(test_subject) = record.hashes.test_subject.as_ref() else {
        return CheckValue::Stale;
    };
    let Some(targets) = test
        .targets
        .iter()
        .map(|target_ref| source_for_target(scan, target_ref))
        .collect::<Option<Vec<_>>>()
    else {
        return CheckValue::Stale;
    };
    if test_subject != &test.content_hash
        || record.hashes.targets.len() != targets.len()
        || record.revision.commit.is_none()
        || record
            .hashes
            .targets
            .iter()
            .zip(test.targets.iter().zip(targets.iter()))
            .any(|(actual, (target_ref, target))| {
                actual.target != target_ref.normalized()
                    || actual.target_construct != target.content_hash
            })
    {
        CheckValue::Stale
    } else {
        CheckValue::Pass
    }
}

fn read_evidence_records(layout: &VerifyLayout) -> BTreeMap<String, EvidenceRecord> {
    let mut records = BTreeMap::new();
    for id in read_record_ids(&layout.evidence_dir()).unwrap_or_default() {
        let path = layout.evidence_dir().join(format!("{id}.yaml"));
        if let Ok(record) = read_evidence(&path) {
            records
                .entry(record.test_id.as_str().to_owned())
                .and_modify(|current: &mut EvidenceRecord| {
                    if compare_evidence_recency(current, &record).is_lt() {
                        *current = record.clone();
                    }
                })
                .or_insert(record);
        }
    }
    records
}

fn compare_evidence_recency(left: &EvidenceRecord, right: &EvidenceRecord) -> std::cmp::Ordering {
    match (
        rfc3339_instant(&left.executed_at),
        rfc3339_instant(&right.executed_at),
    ) {
        (Some(left_time), Some(right_time)) => left_time
            .cmp(&right_time)
            .then_with(|| left.id.cmp(&right.id)),
        (Some(_), None) => std::cmp::Ordering::Greater,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (None, None) => left.id.cmp(&right.id),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };
    use vtest_model::{
        AdapterId, EvidenceHashes, EvidenceRecord, EvidenceTargetHash, ExecutionDescriptor,
        ExecutionStateSubject, ProjectPath, Revision, RunnerInfo, ScanSummary, SourceFunction,
        SourceLocation, SourceRange, TargetExecution, TargetRef, TestEntity, TestId, TestResult,
        TestSuite, VoId,
    };
    use vtest_store::{
        init_project, new_record_id, AuditBasisRecord, AuditReasonRecord, AuditorRecord,
    };

    #[test]
    fn evidence_recency_uses_the_actual_rfc3339_instant() {
        let make = |id: &str, executed_at: &str| EvidenceRecord {
            id: id.to_owned(),
            test_id: TestId::new("TEST-ONE"),
            adapter: Some(AdapterId::new("rust-cargo")),
            result: TestResult::Pass,
            executed_at: executed_at.to_owned(),
            revision: Revision {
                commit: Some("abc".to_owned()),
                dirty: false,
            },
            execution_state: Some(ExecutionStateSubject {
                schema: "fixture-v1".to_owned(),
                complete: true,
                hash: Some(ContentHash::from_text("state")),
            }),
            hashes: EvidenceHashes {
                test_subject: Some(ContentHash::from_text("test")),
                targets: vec![EvidenceTargetHash {
                    target: "rust-cargo::src/lib.rs::target".to_owned(),
                    target_construct: ContentHash::from_text("target"),
                }],
                compatibility: None,
            },
            runner: RunnerInfo {
                kind: "cargo-test".to_owned(),
                command: "cargo test".to_owned(),
                exit_code: 0,
            },
            target_execution: TargetExecution {
                checked: false,
                method: None,
                result: None,
                targets: Vec::new(),
                compatibility_count: None,
            },
            log_ref: "cache/logs/test.log".to_owned(),
        };
        let earlier = make("01", "2026-08-08T09:00:00+09:00");
        let later = make("02", "2026-08-08T00:30:00Z");
        assert!(compare_evidence_recency(&earlier, &later).is_lt());
    }

    fn static_fixture(test_ids: &[&str]) -> (VerifyLayout, ScanResult) {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("vtest-verify-static-{suffix}"));
        let layout = init_project(&root, "fixture").expect("initialise fixture");
        let mut sources = vec![SourceFunction {
            target: rust_target("src/lib.rs::target"),
            src_id: None,
            location: location("src/lib.rs", "target"),
            content_hash: ContentHash::from_text("pub fn target() {}"),
        }];
        let tests = test_ids
            .iter()
            .map(|id| TestEntity {
                id: TestId::new(*id),
                covers: vec![VoId::new("VO-ONE")],
                targets: vec![rust_target("src/lib.rs::target")],
                intent: "fixture".to_owned(),
                input: None,
                expect: None,
                kind: None,
                cases: Vec::new(),
                related: Vec::new(),
                location: location("tests/static.rs", id),
                content_hash: ContentHash::from_text(&format!("#[test] fn {id}() {{}}")),
                execution: ExecutionDescriptor {
                    adapter: AdapterId::new("rust-cargo"),
                    project: Some("fixture".to_owned()),
                    suite: Some(TestSuite {
                        kind: "integration".to_owned(),
                        name: Some("static".to_owned()),
                    }),
                    selector: (*id).to_owned(),
                },
            })
            .collect::<Vec<_>>();
        sources.extend(tests.iter().map(|test| SourceFunction {
            target: rust_target(&format!(
                "{}::{}",
                test.location.path, test.location.locator
            )),
            src_id: None,
            location: test.location.clone(),
            content_hash: test.content_hash.clone(),
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
            adapter: AdapterId::new("rust-cargo"),
            path: ProjectPath::new(file),
            locator: function.to_owned(),
            byte_range: SourceRange {
                start: 0,
                end: 1,
                start_line: 1,
                end_line: 1,
            },
        }
    }

    fn rust_target(value: &str) -> TargetRef {
        TargetRef::Locator {
            adapter: AdapterId::new("rust-cargo"),
            value: value.to_owned(),
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
        let target = match &test.targets[0] {
            TargetRef::Locator { .. } => &test.targets[0],
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
                locator: Some(
                    TargetRef::Locator {
                        adapter: test.location.adapter.clone(),
                        value: format!("{}::{}", test.location.path, test.location.locator),
                    }
                    .normalized(),
                ),
                hash: test.content_hash.clone(),
            },
            AuditSubjectRecord {
                id: None,
                locator: Some(target.normalized()),
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
        scan.tests[0].location.locator = "moved::TEST-ONE".to_owned();
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
            target: rust_target("src/lib.rs::helper"),
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
                    locator: Some(helper.target.normalized()),
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
