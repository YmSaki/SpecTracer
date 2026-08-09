//! Fail-closed aggregation and report boundary (M6).

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    process::Command,
};

use serde::Serialize;
use vtest_model::{CheckValue, ContentHash, Diagnostic, EvidenceRecord, ScanSummary, TestEntity};
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
            discovered_tests: scan.discovered_tests.clone(),
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
            let specs = read_record_ids(&layout.spec_dir())
                .unwrap_or_default()
                .into_iter()
                .filter_map(|id| vtest_store::read_spec(layout, &id).ok())
                .collect::<Vec<_>>();
            let referenced_specs = reqs
                .iter()
                .flat_map(|req| req.spec_refs.iter().map(|spec_ref| spec_ref.spec.clone()))
                .collect::<BTreeSet<_>>();
            let unreferenced_specs = specs
                .iter()
                .filter(|spec| !referenced_specs.contains(&spec.id))
                .map(|spec| spec.id.as_str().to_owned())
                .collect::<Vec<_>>();
            let missing = reqs
                .iter()
                .filter(|req| {
                    !vos.iter()
                        .any(|vo| vo.requirements.iter().any(|candidate| candidate == &req.id))
                        || req.spec_refs.is_empty()
                        || req.spec_refs.iter().any(|spec_ref| {
                            let Ok(spec) = vtest_store::read_spec(layout, spec_ref.spec.as_str())
                            else {
                                return true;
                            };
                            let Ok(source) = fs::read_to_string(root.join(&spec.path)) else {
                                return true;
                            };
                            ContentHash::from_text(&source) != spec.sha256
                        })
                })
                .map(|req| req.id.as_str().to_owned())
                .collect::<Vec<_>>();
            if !reqs.is_empty() && missing.is_empty() && unreferenced_specs.is_empty() {
                (
                    CheckValue::Pass,
                    vec!["every selected REQ and discovered SPEC is represented".to_owned()],
                )
            } else {
                (
                    CheckValue::Missing,
                    if !unreferenced_specs.is_empty() {
                        vec![format!(
                            "SPEC record(s) are not referenced by an active REQ: {}",
                            unreferenced_specs.join(", ")
                        )]
                    } else if reqs.is_empty() {
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
            let source_structure_error = scan.diagnostics.iter().any(|diagnostic| {
                diagnostic.is_error()
                    && matches!(diagnostic.code.as_str(), "E-SCAN-001" | "E-SCAN-002")
            });
            let vos = selection
                .vo_ids
                .iter()
                .filter_map(|id| read_vo(layout, id).ok())
                .collect::<Vec<_>>();
            let missing_parent = vos.iter().any(|vo| {
                vo.parent
                    .as_ref()
                    .is_some_and(|parent| !vos.iter().any(|candidate| candidate.id == *parent))
            });
            if source_structure_error || missing_parent {
                (
                    CheckValue::Fail,
                    vec![
                        "VO decomposition contains an unreadable source or parent reference"
                            .to_owned(),
                    ],
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
        "test_execution" => evaluate_test_execution(root, evidence, scan),
        "runtime_result" => evaluate_runtime(root, evidence, scan),
        "target_execution" => evaluate_target_execution(root, evidence, scan),
        "evidence_validity" => evaluate_evidence_validity(root, evidence, scan),
        "test_traceability" => evaluate_test_traceability(scan),
        _ => (CheckValue::Unknown, vec!["unknown check item".to_owned()]),
    }
}

fn evaluate_test_traceability(scan: &ScanResult) -> (CheckValue, Vec<String>) {
    let missing = scan
        .discovered_tests
        .iter()
        .filter(|draft| {
            matches!(
                draft.managed,
                vtest_adapter_api::ManagedTestDraftLink::Missing
            )
        })
        .count();
    let multiple = scan
        .discovered_tests
        .iter()
        .filter(|draft| {
            matches!(
                draft.managed,
                vtest_adapter_api::ManagedTestDraftLink::Multiple(_)
            )
        })
        .count();
    let unresolved = scan
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "E-SCAN-003")
        .count();
    let unregistered = scan
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "W-SCAN-101")
        .count();
    if missing > 0 || unregistered > 0 {
        return (
            CheckValue::Missing,
            vec![format!(
                "{} discovered Test(s) have no managed entity",
                missing.max(unregistered)
            )],
        );
    }
    let empty_covers = scan
        .tests
        .iter()
        .filter(|test| test.covers.is_empty())
        .count();
    if multiple > 0 || unresolved > 0 || empty_covers > 0 {
        return (
            CheckValue::Mismatch,
            vec![format!(
                "managed Test traceability mismatch: multiple={multiple}, unresolved_vo={unresolved}, empty_covers={empty_covers}"
            )],
        );
    }
    (
        CheckValue::Pass,
        vec![format!(
            "all {} discovered Test(s) are managed",
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
    record.subjects.iter().any(|subject| {
        subject
            .id
            .as_deref()
            .is_some_and(|id| id == "CONFIG" || id.starts_with("CONFIG::"))
    })
}

/// Static audits are per-Test records. A current record must bind the Test,
/// the raw configuration, and the exact source function resolved from that
/// Test's declared target. The disambiguated `test-source::` locator preserves
/// the managed construct identity in the v0.1 opaque-locator schema, without
/// confusing it with a target/helper locator. It may additionally bind direct
/// helpers, but it cannot substitute a helper for the declared target or
/// combine tests. Configuration subjects are adapter-qualified when produced
/// by the current writer; the unqualified `CONFIG` form remains a v0.1 reader
/// compatibility path.
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

    let Some(target) = test
        .targets
        .first()
        .and_then(|target| source_for_target(scan, target))
    else {
        return false;
    };
    let test_locator = format!(
        "test-source::{}::{}",
        test.location.file, test.location.function
    );
    let binds_test_code = record.subjects.iter().any(|subject| {
        subject.locator.as_deref() == Some(&test_locator) && subject.hash == test.content_hash
    });
    let binds_target = record.subjects.iter().any(|subject| {
        subject.locator.as_deref() == Some(&target.target.value)
            && subject.hash == target.content_hash
    });
    binds_test_code && binds_target && audit_mentions_config(record)
}

fn source_for_target<'a>(
    scan: &'a ScanResult,
    target: &vtest_model::NeutralTargetRef,
) -> Option<&'a vtest_model::SourceFunction> {
    scan.sources.iter().find(|source| {
        source.target.adapter == target.adapter
            && (source.target.value == target.value
                || source
                    .src_id
                    .as_ref()
                    .is_some_and(|src_id| src_id.as_str() == target.value))
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
    if let Some(locator) = subject.locator.as_deref() {
        if let Some(test_locator) = locator.strip_prefix("test-source::") {
            let actual = scan.tests.iter().find(|test| {
                format!("{}::{}", test.location.file, test.location.function) == test_locator
            });
            return actual.is_some_and(|test| test.content_hash == subject.hash);
        }
    }
    let actual = match (&subject.id, &subject.locator) {
        (Some(id), None) if id == "CONFIG" => static_audit_config_hash(layout, None),
        (Some(id), None) if id.starts_with("CONFIG::") => {
            static_audit_config_hash(layout, id.strip_prefix("CONFIG::"))
        }
        (Some(id), None) => {
            if let Ok(spec) = vtest_store::read_spec(layout, id) {
                fs::read_to_string(layout.root.join(&spec.path))
                    .ok()
                    .map(|text| ContentHash::from_text(&text))
            } else {
                scan.tests
                    .iter()
                    .find(|test| test.id.as_str() == id)
                    .map(|test| test.content_hash.clone())
            }
        }
        (None, Some(locator)) => scan
            .sources
            .iter()
            .find(|source| source.target.value == *locator)
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

fn normalize_audit_verdict(kind: &str, value: Option<&str>) -> CheckValue {
    match (kind, value) {
        (_, Some("PASS")) => CheckValue::Pass,
        ("impl-consistency", Some("FAIL")) => CheckValue::Mismatch,
        (_, Some("FAIL")) => CheckValue::Fail,
        (_, Some("UNKNOWN")) => CheckValue::Unknown,
        _ => CheckValue::Unknown,
    }
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
        let verdict = normalize_audit_verdict(kind, yaml_scalar_value(&text, "verdict").as_deref());
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
    if let Some(locator) = subject.locator.as_deref() {
        if let Some(locator) = locator.strip_prefix("static-analysis-source::") {
            subject.kind = "static_analysis_source".to_owned();
            subject.locator = Some(locator.to_owned());
            return subject;
        }
    }
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
        } else if subject
            .id
            .as_deref()
            .is_some_and(|id| id == "CONFIG" || id.starts_with("CONFIG::"))
        {
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
                .find(|source| source.target.value == locator)
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
        "config" => static_audit_config_hash(
            layout,
            subject
                .id
                .as_deref()
                .and_then(|id| id.strip_prefix("CONFIG::")),
        ),
        "static_analysis_source" => subject
            .locator
            .as_deref()
            .and_then(|locator| {
                scan.tests.iter().find(|test| {
                    format!("{}::{}", test.location.file, test.location.function) == locator
                })
            })
            .and_then(|test| {
                let source = fs::read_to_string(root.join(&test.location.file)).ok()?;
                let source = source.get(test.location.start_byte..test.location.end_byte)?;
                Some(ContentHash::from_domain_fields(
                    "vtest:static-analysis-source:v1",
                    &[
                        ("test", test.content_hash.as_str().as_bytes()),
                        ("source", source.as_bytes()),
                    ],
                ))
            }),
        _ => None,
    };
    actual.as_ref() == Some(expected)
}

/// Hash only the adapter rule-configuration projection that can affect a
/// static-audit verdict. Run/coverage settings are deliberately excluded so
/// changing an unrelated execution option does not invalidate a static audit,
/// while assertion-macro changes cannot leave an old PASS fresh.
fn static_audit_config_hash(
    layout: &VerifyLayout,
    adapter_id: Option<&str>,
) -> Option<ContentHash> {
    let config = vtest_store::load_config(&layout.root).ok()?;
    let adapter_id = adapter_id.unwrap_or("rust-cargo");
    let assertion_macros = config
        .adapters
        .iter()
        .find(|adapter| adapter.id == adapter_id)
        .map(|adapter| adapter.scan.assertion_macros.join(","))
        .unwrap_or_else(|| config.scan.assertion_macros.join(","));
    let effective_config = serde_json::json!({
        "assertion_macros": assertion_macros,
    });
    let effective_config = serde_json::to_vec(&effective_config).ok()?;
    Some(ContentHash::from_domain_fields(
        "vtest:static-audit-config:v1",
        &[
            ("adapter", adapter_id.as_bytes()),
            ("rule_set_id", b"rust-da"),
            ("rule_set_version", b"m3-v1"),
            ("effective_config", &effective_config),
        ],
    ))
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
    root: &Path,
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
                |record| match evidence_record_validity(root, record, test, scan) {
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
    root: &Path,
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
                if evidence_record_validity(root, record, test, scan) != CheckValue::Pass {
                    CheckValue::Stale
                } else {
                    evaluate_record_target_execution(record, test)
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
    root: &Path,
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
                evidence_record_validity(root, record, test, scan)
            });
        value = combine_values(value, current);
        basis.push(evidence_basis(
            test,
            evidence.get(test.id.as_str()),
            current,
        ));
        if let Some(record) = evidence.get(test.id.as_str()) {
            if test.targets.len() > 1 && record.hashes.target_fns.is_empty() {
                basis.push(format!(
                    "Test {} has multiple targets but Evidence has no target_fns list",
                    record.test_id
                ));
            }
        }
    }
    (value, basis)
}

fn evaluate_test_execution(
    root: &Path,
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
                evidence_record_validity(root, record, test, scan)
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
    root: &Path,
    record: &EvidenceRecord,
    test: &TestEntity,
    scan: &ScanResult,
) -> CheckValue {
    // A legacy v0.1 record has no target_fns list.  It remains sufficient for
    // a single-target Test, but cannot prove that every target of a
    // multi-target Test was captured.
    if test.targets.len() > 1 && record.hashes.target_fns.is_empty() {
        return CheckValue::Unknown;
    }
    let Some(targets) = test
        .targets
        .iter()
        .map(|target_ref| source_for_target(scan, target_ref))
        .collect::<Option<Vec<_>>>()
    else {
        return CheckValue::Stale;
    };
    let target_hashes = if record.hashes.target_fns.is_empty() {
        vec![&record.hashes.target_fn]
    } else {
        record.hashes.target_fns.iter().collect::<Vec<_>>()
    };
    let target_subjects_valid = if record.hashes.targets.is_empty() {
        // The v0.1 reader has no neutral target-subject list.  It remains
        // compatible for a single-target Test, but a multi-target record must
        // carry one subject per declared target.
        test.targets.len() <= 1
    } else {
        record.hashes.targets.len() == test.targets.len()
            && record
                .hashes
                .targets
                .iter()
                .zip(targets.iter())
                .all(|(actual, current)| {
                    actual.target == current.target
                        && actual.target_construct == current.content_hash
                })
    };
    if record.hashes.test_fn != test.content_hash
        || target_hashes.len() != targets.len()
        || target_hashes
            .iter()
            .zip(targets.iter())
            .any(|(actual, target)| **actual != target.content_hash)
        || !target_subjects_valid
        || !revision_matches_current(root, record)
    {
        CheckValue::Stale
    } else {
        match record.execution_state.as_ref() {
            None => CheckValue::Stale,
            Some(state) if !state.complete || state.hash.is_none() => CheckValue::Unknown,
            Some(state) => {
                let target_hashes = targets
                    .iter()
                    .map(|target| target.content_hash.clone())
                    .collect::<Vec<_>>();
                let expected = current_execution_state_hash(
                    root,
                    test,
                    &record.runner.kind,
                    &record.runner.command,
                    &target_hashes,
                );
                if expected.as_ref() == state.hash.as_ref() {
                    CheckValue::Pass
                } else {
                    CheckValue::Stale
                }
            }
        }
    }
}

fn evaluate_record_target_execution(record: &EvidenceRecord, test: &TestEntity) -> CheckValue {
    if !record.target_execution.checked {
        return CheckValue::NotChecked;
    }
    let entries = &record.target_execution.targets;
    if entries.is_empty() {
        // Single-target v0.1 records only have the aggregate fields.  A
        // checked multi-target record without per-target entries is not
        // sufficient evidence for any target.
        return if test.targets.len() == 1 {
            record.target_execution.result
        } else {
            CheckValue::Unknown
        };
    }
    if entries.len() != test.targets.len()
        || entries
            .iter()
            .zip(test.targets.iter())
            .any(|(entry, target)| entry.target != *target)
        || has_duplicate_targets(entries)
    {
        return CheckValue::Unknown;
    }
    let aggregate = entries.iter().fold(CheckValue::Pass, |current, entry| {
        combine_values(current, entry.result)
    });
    let count = entries.iter().filter_map(|entry| entry.count).sum::<u64>();
    let aggregate_count = entries
        .iter()
        .all(|entry| entry.count.is_some())
        .then_some(count);
    if aggregate != record.target_execution.result
        || aggregate_count != record.target_execution.count
    {
        return CheckValue::Unknown;
    }
    aggregate
}

fn has_duplicate_targets(entries: &[vtest_model::TargetExecutionEntry]) -> bool {
    entries.iter().enumerate().any(|(index, entry)| {
        entries
            .iter()
            .skip(index + 1)
            .any(|other| other.target == entry.target)
    })
}

fn revision_matches_current(root: &Path, record: &EvidenceRecord) -> bool {
    let Some(expected) = record.revision.commit.as_deref() else {
        return false;
    };
    Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .is_some_and(|actual| actual == expected)
}

fn current_execution_state_hash(
    root: &Path,
    test: &TestEntity,
    runner: &str,
    command: &str,
    target_hashes: &[ContentHash],
) -> Option<ContentHash> {
    let manifest = execution_manifest(root)?;
    let target_material = target_hashes
        .iter()
        .map(ContentHash::as_str)
        .collect::<Vec<_>>()
        .join("\n");
    let execution = serde_json::to_vec(&test.execution).ok()?;
    let invocation = serde_json::to_vec(&serde_json::json!({
        "command": command,
    }))
    .ok()?;
    let effective_config =
        serde_json::to_vec(&execution_effective_config(root, &test.execution.adapter)).ok()?;
    let config_hash = execution_config_material(root);
    Some(ContentHash::from_domain_fields(
        "vtest:execution-state:v1",
        &[
            ("adapter", test.execution.adapter.as_str().as_bytes()),
            ("schema_id", b"vtest-execution-state"),
            ("schema_version", b"v1"),
            ("runner", runner.as_bytes()),
            ("command", command.as_bytes()),
            ("invocation", &invocation),
            ("toolchain", runner.as_bytes()),
            ("effective_config", &effective_config),
            ("test_subject", test.content_hash.as_str().as_bytes()),
            ("target_subjects", target_material.as_bytes()),
            ("execution", &execution),
            ("config", config_hash.as_bytes()),
            ("manifest", manifest.as_bytes()),
        ],
    ))
}

fn execution_config_material(root: &Path) -> String {
    fs::read_to_string(root.join(".verify/config.yaml")).unwrap_or_default()
}

fn execution_effective_config(
    root: &Path,
    adapter_id: &vtest_model::AdapterId,
) -> serde_json::Value {
    let Ok(project) = vtest_store::load_config(root) else {
        return serde_json::json!({});
    };
    let adapter = project
        .adapters
        .iter()
        .find(|entry| entry.id == adapter_id.as_str());
    let (roots, include, assertion_macros, coverage) = adapter
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
    serde_json::json!({
        "assertion_macros": assertion_macros.join(","),
        "coverage": coverage,
        "include": include.join(","),
        "roots": roots.join(","),
    })
}

fn execution_manifest(root: &Path) -> Option<String> {
    let mut files = BTreeMap::new();
    collect_execution_files(root, root, &mut files).ok()?;
    Some(
        files
            .into_iter()
            .map(|(path, bytes)| {
                format!(
                    "workspace\t{path}\tworkspace-file\t{}\n",
                    ContentHash::from_bytes(&bytes).as_str()
                )
            })
            .collect(),
    )
}

fn collect_execution_files(
    root: &Path,
    directory: &Path,
    files: &mut BTreeMap<String, Vec<u8>>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap_or(&path);
        if relative.components().any(|component| {
            component
                .as_os_str()
                .to_str()
                .is_some_and(|name| matches!(name, ".verify" | ".git" | "target"))
        }) {
            continue;
        }
        if path.is_dir() {
            collect_execution_files(root, &path, files)?;
        } else if path.is_file() {
            files.insert(
                relative.to_string_lossy().replace('\\', "/"),
                fs::read(path)?,
            );
        }
    }
    Ok(())
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
        EvidenceHashes, EvidenceRecord, Revision, RunnerInfo, ScanSummary, SourceFunction,
        SourceLocation, TargetExecution, TestEntity, TestId, TestResult, TestSuite, VoId,
    };
    use vtest_store::{
        init_project, new_record_id, AuditBasisRecord, AuditReasonRecord, AuditorRecord,
    };

    #[test]
    fn evidence_recency_uses_the_actual_rfc3339_instant() {
        let make = |id: &str, executed_at: &str| EvidenceRecord {
            id: id.to_owned(),
            test_id: TestId::new("TEST-ONE"),
            result: TestResult::Pass,
            executed_at: executed_at.to_owned(),
            revision: Revision {
                commit: Some("abc".to_owned()),
                dirty: false,
            },
            hashes: EvidenceHashes {
                test_fn: ContentHash::from_text("test"),
                target_fn: ContentHash::from_text("target"),
                target_fns: vec![ContentHash::from_text("target")],
                test_subject: None,
                targets: Vec::new(),
            },
            runner: RunnerInfo {
                kind: "cargo-test".to_owned(),
                command: "cargo test".to_owned(),
                exit_code: 0,
            },
            target_execution: TargetExecution {
                checked: false,
                method: None,
                result: CheckValue::NotChecked,
                count: None,
                targets: Vec::new(),
            },
            log_ref: "cache/logs/test.log".to_owned(),
            adapter: None,
            execution_state: None,
        };
        let earlier = make("01", "2026-08-08T09:00:00+09:00");
        let later = make("02", "2026-08-08T00:30:00Z");
        assert!(compare_evidence_recency(&earlier, &later).is_lt());
    }

    #[test]
    fn impl_consistency_failure_maps_to_mismatch() {
        assert_eq!(
            normalize_audit_verdict("impl-consistency", Some("FAIL")),
            CheckValue::Mismatch
        );
        assert_eq!(
            normalize_audit_verdict("test-semantic", Some("FAIL")),
            CheckValue::Fail
        );
    }

    fn static_fixture(test_ids: &[&str]) -> (VerifyLayout, ScanResult) {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("vtest-verify-static-{suffix}"));
        let layout = init_project(&root, "fixture").expect("initialise fixture");
        let mut sources = vec![SourceFunction {
            target: vtest_model::NeutralTargetRef::new("rust-cargo", "src/lib.rs::target"),
            src_id: None,
            location: location("src/lib.rs", "target"),
            content_hash: ContentHash::from_text("pub fn target() {}"),
        }];
        let tests = test_ids
            .iter()
            .map(|id| TestEntity {
                id: TestId::new(*id),
                covers: vec![VoId::new("VO-ONE")],
                targets: vec![vtest_model::NeutralTargetRef::new(
                    "rust-cargo",
                    "src/lib.rs::target",
                )],
                intent: "fixture".to_owned(),
                input: None,
                expect: None,
                kind: None,
                cases: Vec::new(),
                related: Vec::new(),
                location: location("tests/static.rs", id),
                content_hash: ContentHash::from_text(&format!("#[test] fn {id}() {{}}")),
                execution: vtest_model::ExecutionDescriptor {
                    adapter: vtest_model::AdapterId::from("rust-cargo"),
                    project: Some("fixture".to_owned()),
                    suite: Some(TestSuite {
                        kind: "integration".to_owned(),
                        name: Some("static".to_owned()),
                    }),
                    selector: (*id).to_owned(),
                    runner: Some("cargo-test".to_owned()),
                    working_root: Some(".".to_owned()),
                },
            })
            .collect::<Vec<_>>();
        sources.extend(tests.iter().map(|test| SourceFunction {
            target: vtest_model::NeutralTargetRef::new(
                "rust-cargo",
                format!("{}::{}", test.location.file, test.location.function),
            ),
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
            discovered_tests: Vec::new(),
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
            ..SourceLocation::default()
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
        let target = test.targets.first().expect("fixture uses a target");
        vec![
            AuditSubjectRecord {
                id: Some(test.id.as_str().to_owned()),
                locator: None,
                hash,
            },
            AuditSubjectRecord {
                id: None,
                locator: Some(format!(
                    "test-source::{}::{}",
                    test.location.file, test.location.function
                )),
                hash: test.content_hash.clone(),
            },
            AuditSubjectRecord {
                id: None,
                locator: Some(target.value.clone()),
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
        subjects.push(AuditSubjectRecord {
            id: Some("CONFIG".to_owned()),
            locator: None,
            hash: static_audit_config_hash(layout, None).expect("hash fixture static-audit config"),
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
            target: vtest_model::NeutralTargetRef::new("rust-cargo", "src/lib.rs::helper"),
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
                    locator: Some(helper.target.value.clone()),
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
