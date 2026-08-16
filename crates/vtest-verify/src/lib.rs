//! Fail-closed aggregation and report boundary (M6).

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    sync::OnceLock,
};

use serde::Serialize;
use vtest_adapter_api::{hash_execution_state_draft, AdapterRegistry};
use vtest_adapter_rust::{rust_cargo_registration, RUST_CARGO_ADAPTER_ID};
use vtest_model::{
    hash_spec_audit_subject, hash_specification_source, hash_static_audit_config_subject,
    AdapterId, CheckValue, ContentHash, Diagnostic, EvidenceRecord, ScanSummary, TargetRef,
    TestEntity,
};
use vtest_scan::{
    classify_target_resolution, find_target_source, required_subject_keys,
    rust_cargo_static_audit_projection, ScanResult, TargetResolution, STATIC_AUDIT_RULE_SET_ID,
    STATIC_AUDIT_RULE_SET_VERSION,
};
use vtest_store::{
    current_approval_subject, derive_vo_status, load_config, read_audit, read_evidence,
    read_record_ids, read_req, read_spec, read_text, read_vo, required_spec_coverage_subject_keys,
    spec_coverage_req_sets, static_record_target_defect, yaml_scalar_value, AuditRecord,
    AuditSubjectRecord, ProjectConfig, ReqRecord, VerifyLayout, VoRecord,
};

/// The registry of adapters `evidence_record_validity` can resolve a
/// record's own `AdapterId` through, so freshness re-derivation always uses
/// the adapter's own schema (詳細設計 line 810 / 1390) instead of a single
/// hardcoded rust-cargo path. Built once; adding a future adapter here (the
/// only vtest-verify touch it needs) is registration, not new control flow.
fn built_in_test_runners() -> &'static AdapterRegistry {
    static REGISTRY: OnceLock<AdapterRegistry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        AdapterRegistry::from_registrations([rust_cargo_registration()])
            .expect("built-in rust-cargo adapter registration is well-formed")
    })
}

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
    /// The SPEC ids `spec_coverage` evaluates. Full scope (no entity_scope)
    /// is every registered SPEC record -- 基本仕様 §7.4/L419 pins MISSING to
    /// the count of *registered* Specifications, not to which ones some REQ
    /// happens to reference, so an unreferenced SPEC still counts toward the
    /// "0 registered" degenerate case. A REQ/VO/Test entity_scope narrows to
    /// the SPECs the selected REQs reference (§11.1 L1353: the entity axis
    /// must not narrow which REQs count toward one of those SPECs' own
    /// coverage -- that full-closure re-derivation happens inside
    /// `evaluate_one_spec_coverage`, not here).
    spec_ids: BTreeSet<String>,
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
            let spec_ids = read_record_ids(&layout.spec_dir())
                .unwrap_or_default()
                .into_iter()
                .collect::<BTreeSet<_>>();
            return Self {
                entity_scope,
                test_ids: all_test_ids,
                vo_ids: vo_records.keys().cloned().collect(),
                req_ids: req_records.keys().cloned().collect(),
                spec_ids,
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
        // §11.1 L1353: the REQ/VO/Test entity axis implicates a SPEC (so its
        // spec_coverage still shows) without narrowing which REQs count
        // toward it -- `evaluate_one_spec_coverage` re-derives that SPEC's
        // full active-REQ closure independently of `req_ids` above.
        let spec_ids = req_ids
            .iter()
            .filter_map(|id| req_records.get(id))
            .flat_map(|req| {
                req.spec_refs
                    .iter()
                    .map(|spec_ref| spec_ref.spec.to_string())
            })
            .collect::<BTreeSet<_>>();
        Self {
            entity_scope,
            test_ids,
            vo_ids,
            req_ids,
            spec_ids,
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
    // §11.1 L1425/step 9: "SPECはspec_coverageとREQ部分木の合成" -- a SPEC
    // node's children are every top-level REQ subtree whose own `spec_refs`
    // names it. A REQ can reference more than one SPEC (or none); this
    // mirrors how a Test already appears under every VO it `covers` rather
    // than being deduplicated to one parent.
    for spec_id in &selection.spec_ids {
        let mut children = Vec::new();
        for (id, req) in &reqs {
            if req
                .parent
                .as_ref()
                .is_some_and(|parent| reqs.contains_key(parent.as_str()))
            {
                continue;
            }
            if !req
                .spec_refs
                .iter()
                .any(|spec_ref| spec_ref.spec.as_str() == spec_id)
            {
                continue;
            }
            attached_reqs.insert(id.clone());
            children.push(build_req_node(
                id,
                &reqs,
                &vos,
                scan,
                &values,
                &mut attached_vos,
                &mut BTreeSet::new(),
            ));
        }
        roots.push(spec_node(spec_id, &values, children));
    }
    // A top-level REQ with no spec_refs (or none selected) stays an orphan
    // root, the same treatment orphan VOs below already get.
    for (id, req) in &reqs {
        if req
            .parent
            .as_ref()
            .is_some_and(|parent| reqs.contains_key(parent.as_str()))
        {
            continue;
        }
        if attached_reqs.contains(id) {
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
    roots
}

/// A SPEC node: `spec_coverage`'s own value plus the fail-closed fold of its
/// attached REQ subtrees (§11.1 step 9). Like every other multi-instance
/// item in this tree (`test_node`, `build_vo_node`), the node shows the same
/// project-wide aggregate `spec_coverage` `ReportItem` rather than a
/// per-SPEC slice -- `evaluate_spec_coverage`'s basis strings carry the
/// per-SPEC detail.
fn spec_node(
    id: &str,
    values: &BTreeMap<&str, &ReportItem>,
    children: Vec<VerificationNode>,
) -> VerificationNode {
    let items = item_copies(values, &["spec_coverage"]);
    node_with_children("spec", id, items, children)
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
            // §11.1: spec_coverage is now the SPEC node's own item; a REQ
            // node carries none of the fixed 12 items directly (cf. C-2's
            // sibling cluster, vo_decomposition, which is not this fix's
            // scope either).
            items: Vec::new(),
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
    node_with_children("req", id, Vec::new(), children)
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
        // §7.4/§11.1: spec_coverage is a SPEC-rooted Spec→REQ intake item,
        // never a REQ→VO linkage check (that axis is vo_coverage's own).
        "spec_coverage" => evaluate_spec_coverage(root, layout, scan, selection),
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
        "static_audit" => evaluate_static_audit(root, layout, evidence, scan),
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
fn evaluate_static_audit(
    root: &Path,
    layout: &VerifyLayout,
    evidence: &BTreeMap<String, EvidenceRecord>,
    scan: &ScanResult,
) -> (CheckValue, Vec<String>) {
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

        // Resolve the declared targets to their canonical Locators. A target
        // that cannot be resolved leaves reachability unprovable, so the item is
        // non-PASS with a diagnostic and the malformed classifier is not run
        // over a partial declared set (詳細設計 §6.1, §7.3).
        let declared = match declared_canonical_targets(scan, test) {
            Ok(declared) => declared,
            Err(unresolved) => {
                project_value = combine_values(project_value, CheckValue::Unknown);
                basis.push(format!(
                    "Test {test_id}: Unknown (declared target(s) do not resolve: {})",
                    unresolved.join(", ")
                ));
                continue;
            }
        };

        let test_records = records
            .iter()
            .filter(|record| audit_mentions_test(record, test_id))
            .collect::<Vec<_>>();
        let mut malformed_for_test = malformed
            .iter()
            .filter(|(_, _, text)| audit_text_mentions_test(text, test_id))
            .map(|(id, error, _)| format!("{id} ({error})"))
            .collect::<Vec<_>>();

        let mut valid = Vec::new();
        let mut stale = 0usize;
        for record in &test_records {
            // Stale is decided before the classifier: a record whose subjects no
            // longer match current bytes is out of date, not malformed.
            let subjects_current = static_audit_binds_test_subjects(record, test, scan)
                && record
                    .subjects
                    .iter()
                    .all(|subject| audit_subject_is_current(layout, scan, subject));
            if !subjects_current {
                stale += 1;
                continue;
            }
            // A subject-current record with an inconsistent per-target list is
            // malformed (E-SCAN-010); it is excluded and its per-target FAILs are
            // not extracted (詳細設計 §3.6).
            if let Some(defect) = static_record_target_defect(record, &declared) {
                malformed_for_test.push(format!("{} ({defect})", record.id));
                continue;
            }
            // A record without per-target DA-002/DA-003 verdicts is not a valid
            // v2 record for a target-declaring Test; treat it as STALE, never a
            // source of a current PASS (詳細設計 §7.3 L1019).
            if !record_carries_per_target(record) {
                stale += 1;
                continue;
            }
            valid.push(*record);
        }

        // 詳細設計 §3.6: a malformed record is E-SCAN-010 -- excluded from the
        // valid set, its content untrusted, never a distinct per-test UNKNOWN
        // in its own right. If a valid record still exists among this Test's
        // records, its value stands unmodified by the sibling malformed
        // record; if none does, the Test falls through to the same "no valid
        // record" rule STALE-only records already use (STALE when some
        // record -- valid-content-mismatched or malformed -- was attempted,
        // NOT_CHECKED only when this Test was never audited at all).
        let value = if valid.is_empty() {
            if test_records.is_empty() && malformed_for_test.is_empty() {
                CheckValue::NotChecked
            } else {
                CheckValue::Stale
            }
        } else {
            static_audit_item_value(root, evidence, test, scan, &valid, &declared)
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
        // §4.3: the malformed record is an Unknown-flavoured ground on the
        // parent representative, folded through the one aggregation rule --
        // never an ad-hoc overwrite of the fold's already-correct result.
        // Unknown is the order's minimum, so this only ever demotes an
        // otherwise-PASS aggregate; any coexisting higher-priority value
        // (FAIL, STALE, NOT_CHECKED, ...) still wins.
        project_value = combine_values(project_value, CheckValue::Unknown);
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

/// Resolve every declared target of a Test to its canonical Locator, the one
/// resolution path (§6.1). Returns the deduplicated canonical set, or the list
/// of declared spellings that do not resolve (`Err`) so the caller can surface a
/// diagnostic without running the classifier over a partial set.
fn declared_canonical_targets(
    scan: &ScanResult,
    test: &TestEntity,
) -> Result<BTreeSet<String>, Vec<String>> {
    let mut resolved = BTreeSet::new();
    let mut unresolved = Vec::new();
    for target in &test.targets {
        match find_target_source(scan, target) {
            Some(source) => {
                resolved.insert(source.target.normalized());
            }
            None => unresolved.push(target.normalized()),
        }
    }
    if unresolved.is_empty() {
        Ok(resolved)
    } else {
        Err(unresolved)
    }
}

/// A valid v2 static record carries per-target verdicts for both target-scoped
/// rules; every registered Test declares at least one target (§7.3 L1019).
fn record_carries_per_target(record: &AuditRecord) -> bool {
    let carries = |rule: &str| {
        record
            .reasons
            .iter()
            .any(|reason| reason.rule.as_deref() == Some(rule) && !reason.targets.is_empty())
    };
    carries("DA-002") && carries("DA-003")
}

fn parse_static_verdict(value: &str) -> CheckValue {
    match value {
        "PASS" => CheckValue::Pass,
        "FAIL" => CheckValue::Fail,
        _ => CheckValue::Unknown,
    }
}

fn per_target_static_verdict(
    record: &AuditRecord,
    rule: &str,
    canonical: &str,
) -> Option<CheckValue> {
    record
        .reasons
        .iter()
        .find(|reason| reason.rule.as_deref() == Some(rule))
        .and_then(|reason| {
            reason
                .targets
                .iter()
                .find(|target| target.target == canonical)
        })
        .map(|target| parse_static_verdict(&target.verdict))
}

/// The effective per-target verdict of a target-scoped rule across a Test's
/// valid records, per §8.5 applied per target: a FAIL in any valid record
/// dominates; otherwise the latest valid record's verdict for that target — the
/// old record's PASS never overrides a newer UNKNOWN (詳細設計 §7.3 L1009,
/// L1016). Every valid record shares the same target set (classifier) and
/// carries all six rules (store validation), so the lookup is total.
fn effective_target_verdict(valid: &[&AuditRecord], rule: &str, canonical: &str) -> CheckValue {
    if valid
        .iter()
        .any(|record| per_target_static_verdict(record, rule, canonical) == Some(CheckValue::Fail))
    {
        return CheckValue::Fail;
    }
    valid
        .iter()
        .max_by(|left, right| compare_audit_recency(left, right))
        .and_then(|record| per_target_static_verdict(record, rule, canonical))
        .unwrap_or(CheckValue::Unknown)
}

/// The effective verdict of a non-target-scoped rule across a Test's valid
/// records, per §8.5 at record granularity: FAIL dominates, otherwise the latest
/// valid record's rule verdict (詳細設計 §7.3 L1027).
fn effective_rule_verdict(valid: &[&AuditRecord], rule: &str) -> CheckValue {
    let rule_verdict = |record: &AuditRecord| -> CheckValue {
        record
            .reasons
            .iter()
            .find(|reason| reason.rule.as_deref() == Some(rule))
            .and_then(|reason| reason.verdict.as_deref())
            .map(parse_static_verdict)
            .unwrap_or(CheckValue::Unknown)
    };
    if valid
        .iter()
        .any(|record| rule_verdict(record) == CheckValue::Fail)
    {
        return CheckValue::Fail;
    }
    valid
        .iter()
        .max_by(|left, right| compare_audit_recency(left, right))
        .map(|record| rule_verdict(record))
        .unwrap_or(CheckValue::Unknown)
}

/// Runtime proof of target reachability (詳細設計 §7.3 step 2, §10.2): the
/// §11.2-selected latest Evidence (the one this test maps to) is valid, coverage
/// was measured, and that target's per-target result is PASS with count > 0. The
/// evidence map holds one record per Test, so there is no fallback to an older
/// valid Evidence (L1017).
fn runtime_target_reached(
    root: &Path,
    evidence: &BTreeMap<String, EvidenceRecord>,
    test: &TestEntity,
    scan: &ScanResult,
    canonical: &str,
) -> bool {
    let Some(record) = evidence.get(test.id.as_str()) else {
        return false;
    };
    if evidence_record_validity(root, record, test, scan, built_in_test_runners())
        != CheckValue::Pass
    {
        return false;
    }
    if !record.target_execution.checked {
        return false;
    }
    record.target_execution.targets.iter().any(|observation| {
        observation.target == canonical
            && observation.result == CheckValue::Pass
            && observation.count.is_some_and(|count| count > 0)
    })
}

/// Compute the static_audit item value for one Test at evaluation time
/// (詳細設計 §7.3): the stored fold is not used here. Each declared target's
/// DA-002 reachability is satisfied statically (effective PASS) or by runtime
/// proof (effective UNKNOWN + runtime target_execution PASS); a per-target FAIL
/// is never overturned by runtime. DA-003 and the non-target-scoped rules
/// contribute their effective verdicts with no runtime rescue. The item is PASS
/// only when every declared target's reachability is satisfied and every rule is
/// PASS. Per §7.1 L963 this preserves the invariant "a satisfied reachability
/// does not yield an UNKNOWN at computation time" — it is not an UNKNOWN→PASS
/// promotion.
fn static_audit_item_value(
    root: &Path,
    evidence: &BTreeMap<String, EvidenceRecord>,
    test: &TestEntity,
    scan: &ScanResult,
    valid: &[&AuditRecord],
    declared: &BTreeSet<String>,
) -> CheckValue {
    let mut contributions = Vec::new();
    for canonical in declared {
        let reachability = match effective_target_verdict(valid, "DA-002", canonical) {
            CheckValue::Pass => CheckValue::Pass,
            // A DA-002 FAIL statically denies reachability and is never rescued.
            CheckValue::Fail => CheckValue::Fail,
            // Statically unproven: reachable only if runtime proves it.
            _ => {
                if runtime_target_reached(root, evidence, test, scan, canonical) {
                    CheckValue::Pass
                } else {
                    CheckValue::Unknown
                }
            }
        };
        contributions.push(reachability);
    }
    // DA-003 is target-scoped but has no runtime rescue: coverage proves
    // execution, not result verification (§7.3 L1019, 別紙C §18.3.6).
    for canonical in declared {
        contributions.push(effective_target_verdict(valid, "DA-003", canonical));
    }
    for rule in ["DA-001", "DA-004", "DA-005", "DA-006"] {
        contributions.push(effective_rule_verdict(valid, rule));
    }
    if contributions.contains(&CheckValue::Fail) {
        CheckValue::Fail
    } else if contributions.iter().all(|value| *value == CheckValue::Pass) {
        CheckValue::Pass
    } else {
        CheckValue::Unknown
    }
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
    let test_locator = test_source_locator(test);
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
    compare_recency(&left.audited_at, &left.id, &right.audited_at, &right.id)
}

/// Compare two audit records' recency by the actual RFC 3339 instant of
/// `audited_at` (whose lexical order differs from chronological order when
/// offsets differ), falling back to `id` when a timestamp is missing or
/// unparsable so ordering stays total and deterministic. Shared by every
/// audit-record fold that must pick "the latest valid record" (§8.5).
fn compare_recency(
    left_at: &str,
    left_id: &str,
    right_at: &str,
    right_id: &str,
) -> std::cmp::Ordering {
    match (rfc3339_instant(left_at), rfc3339_instant(right_at)) {
        (Some(left_time), Some(right_time)) => left_time
            .cmp(&right_time)
            .then_with(|| left_id.cmp(right_id)),
        (Some(_), None) => std::cmp::Ordering::Greater,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (None, None) => left_id.cmp(right_id),
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

/// The static-audit CONFIG subject hash. Binds the rule set identity and the
/// rule-affecting config projection only, so a run- or coverage-only config
/// change never stales a static Audit. This must match the persist-side
/// computation in `vtest-audit::audit_static`.
///
/// W6 note (詳細設計 §5.2 line 781): re-evaluation should re-derive the closure
/// via the adapter; W4 keeps the scan-based mechanism and only corrects the
/// subject content. When W6 moves persist to the adapter, move this too.
fn static_audit_config_subject_hash(config: &ProjectConfig) -> ContentHash {
    hash_static_audit_config_subject(
        &AdapterId::new("rust-cargo"),
        STATIC_AUDIT_RULE_SET_ID,
        STATIC_AUDIT_RULE_SET_VERSION,
        &rust_cargo_static_audit_projection(config),
    )
}

fn audit_subject_is_current(
    layout: &VerifyLayout,
    scan: &ScanResult,
    subject: &AuditSubjectRecord,
) -> bool {
    let actual = match (&subject.id, &subject.locator) {
        (Some(id), None) if id == "CONFIG" => load_config(&layout.root)
            .ok()
            .map(|config| static_audit_config_subject_hash(&config)),
        (Some(id), None) => scan
            .tests
            .iter()
            .find(|test| test.id.as_str() == id)
            .map(|test| test.content_hash.clone()),
        // A Test-code subject binds the Test entity at its own source locator,
        // so its currency is the Test entity hash — check Tests before sources.
        // Target and helper subjects fall through to the scan source hash.
        (None, Some(locator)) => scan
            .tests
            .iter()
            .find(|test| test_source_locator(test) == *locator)
            .map(|test| test.content_hash.clone())
            .or_else(|| {
                scan.sources
                    .iter()
                    .find(|source| source.target.normalized() == *locator)
                    .map(|source| source.content_hash.clone())
            }),
        _ => None,
    };
    actual.as_ref() == Some(&subject.hash)
}

/// The normalized locator a static audit uses to bind a Test's own construct.
/// Must match the persist side and `static_audit_binds_test_subjects`.
fn test_source_locator(test: &TestEntity) -> String {
    TargetRef::Locator {
        adapter: test.location.adapter.clone(),
        value: format!("{}::{}", test.location.path, test.location.locator),
    }
    .normalized()
}

#[derive(Clone, Debug)]
struct AuditSubject {
    kind: String,
    id: Option<String>,
    locator: Option<String>,
    hash: Option<ContentHash>,
}

/// The identity half of a subject -- kind plus id-or-locator -- with no
/// hash. Two subjects with the same key name the same required object;
/// comparing sets of these keys is §8.5's set-equality validity conjunct.
fn audit_subject_key(subject: &AuditSubject) -> (String, String) {
    (
        subject.kind.clone(),
        subject
            .id
            .clone()
            .or_else(|| subject.locator.clone())
            .unwrap_or_default(),
    )
}

/// §7.4/§11.1 L1346-1354: `spec_coverage` folds every SPEC in the selected
/// scope with §4.3's fail-closed priority (`combine_values`, shared with
/// every other multi-subject item). Zero registered SPECs in scope is the
/// degenerate empty set 別紙C explicitly forbids promoting to PASS.
fn evaluate_spec_coverage(
    root: &Path,
    layout: &VerifyLayout,
    scan: &ScanResult,
    selection: &ScopeSelection,
) -> (CheckValue, Vec<String>) {
    if selection.spec_ids.is_empty() {
        return (
            CheckValue::Missing,
            vec!["no registered Specification exists in the selected scope".to_owned()],
        );
    }
    let mut overall = CheckValue::Pass;
    let mut basis = Vec::new();
    for spec_id in &selection.spec_ids {
        let (value, reason) = evaluate_one_spec_coverage(root, layout, scan, spec_id);
        overall = combine_values(overall, value);
        basis.push(format!("SPEC {spec_id}: {value:?} ({reason})"));
    }
    (overall, basis)
}

/// One SPEC's `spec_coverage` verdict (詳細設計 §11.1 L1346-1352, in the
/// document's own order):
/// 1. SPEC record or its Specification source unreadable -> MISSING.
/// 2. The record's registered `sha256` no longer matches the current source
///    -> STALE. §1.3 L87's SPEC-subject currency rule, applied here only --
///    generalizing it to every kind that carries a "spec" subject (e.g.
///    impl-consistency's SPEC closure) is wave-4's VO-INTAKE-04, not this fix.
/// 3. Zero active REQ reference it -> MISSING (submission or not, per L1144).
///    An active REQ whose reference has an empty `section` -> MISMATCH.
/// 4. Otherwise, the latest valid spec-coverage Audit Record bound to the
///    current SPEC subject and the complete active-REQ set: COMPLETE/
///    INCOMPLETE/UNKNOWN map to PASS/FAIL/UNKNOWN (mapped to that at submit
///    time, so the stored verdict is already PASS/FAIL/UNKNOWN here).
/// 5. No spec-coverage audit at all -> NOT_CHECKED; only invalid ones exist
///    -> STALE.
fn evaluate_one_spec_coverage(
    root: &Path,
    layout: &VerifyLayout,
    scan: &ScanResult,
    spec_id: &str,
) -> (CheckValue, String) {
    let Ok(record) = read_spec(layout, spec_id) else {
        return (
            CheckValue::Missing,
            "SPEC record could not be read".to_owned(),
        );
    };
    let Ok(source) = read_text(&root.join(&record.path)) else {
        return (
            CheckValue::Missing,
            "Specification source could not be read".to_owned(),
        );
    };
    if hash_specification_source(&source) != record.sha256 {
        return (
            CheckValue::Stale,
            "SPEC record's registered sha256 does not match the current Specification source"
                .to_owned(),
        );
    }
    let sets = match spec_coverage_req_sets(layout, spec_id) {
        Ok(sets) => sets,
        Err(error) => {
            return (
                CheckValue::Unknown,
                format!("active REQ set referencing {spec_id} could not be resolved: {error}"),
            )
        }
    };
    if !sets.malformed.is_empty() {
        return (
            CheckValue::Mismatch,
            format!(
                "active REQ(s) reference {spec_id} with an empty section: {}",
                sets.malformed.join(", ")
            ),
        );
    }
    if sets.active.is_empty() {
        return (
            CheckValue::Missing,
            format!("no active REQ references {spec_id}"),
        );
    }
    let required = match required_spec_coverage_subject_keys(layout, spec_id) {
        Ok(required) => required,
        Err(error) => {
            return (
                CheckValue::Unknown,
                format!("required subject set for {spec_id} could not be resolved: {error}"),
            )
        }
    };
    let mut records = Vec::new();
    for id in read_record_ids(&layout.audits_dir()).unwrap_or_default() {
        let path = layout.audits_dir().join(format!("{id}.yaml"));
        let Ok(text) = read_text(&path) else { continue };
        if yaml_scalar_value(&text, "kind").as_deref() != Some("spec-coverage") {
            continue;
        }
        let subjects = parse_audit_subjects(&text);
        let matches_spec = subjects
            .iter()
            .any(|subject| subject.kind == "spec" && subject.id.as_deref() == Some(spec_id));
        if !matches_spec {
            continue;
        }
        let recorded_keys = subjects
            .iter()
            .map(audit_subject_key)
            .collect::<BTreeSet<_>>();
        // §8.5: valid = the recorded subject SET exactly equals the current
        // required set (spec + complete active-REQ set) AND every recorded
        // subject's hash is still current -- the same two-conjunct rule
        // `evaluate_test_audit` applies via `required_subject_keys`.
        let valid = recorded_keys == required
            && subjects
                .iter()
                .all(|subject| subject_is_current(root, layout, scan, subject));
        let verdict = match yaml_scalar_value(&text, "verdict").as_deref() {
            Some("PASS") => CheckValue::Pass,
            Some("FAIL") => CheckValue::Fail,
            Some("UNKNOWN") => CheckValue::Unknown,
            _ => CheckValue::Unknown,
        };
        let audited_at = yaml_scalar_value(&text, "audited_at").unwrap_or_default();
        records.push((id, valid, verdict, audited_at));
    }
    if records.is_empty() {
        return (
            CheckValue::NotChecked,
            "no spec-coverage audit exists".to_owned(),
        );
    }
    let valid = records
        .iter()
        .filter(|(_, current, _, _)| *current)
        .collect::<Vec<_>>();
    let record_basis = records
        .iter()
        .map(|(id, current, _, _)| format!("{id}{}", if *current { "" } else { " (stale)" }))
        .collect::<Vec<_>>()
        .join(", ");
    if valid.is_empty() {
        return (
            CheckValue::Stale,
            format!("only invalid spec-coverage audit(s) exist: {record_basis}"),
        );
    }
    let audit_value = if valid
        .iter()
        .any(|(_, _, verdict, _)| *verdict == CheckValue::Fail)
    {
        CheckValue::Fail
    } else {
        valid
            .iter()
            .max_by(|left, right| compare_recency(&left.3, &left.0, &right.3, &right.0))
            .map_or(CheckValue::Unknown, |(_, _, verdict, _)| *verdict)
    };
    (
        audit_value,
        format!("spec-coverage audit(s): {record_basis}"),
    )
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
    let mut per_vo: BTreeMap<String, Vec<(String, bool, CheckValue, String)>> = BTreeMap::new();
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
        let audited_at = yaml_scalar_value(&text, "audited_at").unwrap_or_default();
        for vo_id in vo_ids {
            per_vo
                .entry(vo_id)
                .or_default()
                .push((id.clone(), valid, verdict, audited_at.clone()));
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
                .filter(|(_, current, _, _)| *current)
                .collect::<Vec<_>>();
            if valid.is_empty() {
                CheckValue::Stale
            } else {
                // §8.5, same two-stage rule as evaluate_test_audit
                // (VO-SEMAUDIT-05): among the valid records bound to this
                // VO, any FAIL dominates; otherwise the chronologically
                // latest valid record's verdict is authoritative --
                // audited_at is consulted, not a severity-max fold that
                // ignores recency. The approval gate below is unchanged.
                let audit_value = if valid
                    .iter()
                    .any(|(_, _, verdict, _)| *verdict == CheckValue::Fail)
                {
                    CheckValue::Fail
                } else {
                    valid
                        .iter()
                        .max_by(|left, right| compare_recency(&left.3, &left.0, &right.3, &right.0))
                        .map_or(CheckValue::Unknown, |(_, _, verdict, _)| *verdict)
                };
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
            .map(|(id, valid, _, _)| format!("{id}{}", if *valid { "" } else { " (stale)" }))
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

/// §7.5 (基本仕様): a Test's declared target that fails to resolve reports
/// `impl_consistency` as `MISSING` (absent, zero candidates) or `MISMATCH`
/// (ambiguous, 2+ candidates -- including an `E-SCAN-011` permanent SRC ID
/// collision). Both conditions share the pooled `E-SCAN-004` diagnostic code
/// (詳細設計 §6.2), so the split is made on the resolution condition itself
/// via `classify_target_resolution`, not on which code was emitted. `None`
/// means every declared target resolved to exactly one Source Target.
fn target_resolution_check_value(
    scan: &ScanResult,
    test: &TestEntity,
) -> Option<(CheckValue, String)> {
    let mut worst: Option<(CheckValue, Vec<String>)> = None;
    for target in &test.targets {
        let (value, label) = match classify_target_resolution(&scan.sources, target) {
            TargetResolution::Resolved(_) => continue,
            TargetResolution::Absent => (CheckValue::Missing, "absent"),
            TargetResolution::Ambiguous => (CheckValue::Mismatch, "ambiguous"),
        };
        let entry = worst.get_or_insert_with(|| (value, Vec::new()));
        entry.0 = combine_values(entry.0, value);
        entry.1.push(format!("{} ({label})", target.normalized()));
    }
    worst.map(|(value, targets)| {
        (
            value,
            format!("declared target(s) do not resolve: {}", targets.join(", ")),
        )
    })
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
    let mut per_test: BTreeMap<String, Vec<(String, bool, CheckValue, String)>> = BTreeMap::new();
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
        let verdict = match yaml_scalar_value(&text, "verdict").as_deref() {
            Some("PASS") => CheckValue::Pass,
            // impl-consistency contrasts implementation against specification, so
            // a FAIL is a MISMATCH; test-semantic keeps FAIL as a plain failure.
            Some("FAIL") if kind == "impl-consistency" => CheckValue::Mismatch,
            Some("FAIL") => CheckValue::Fail,
            Some("UNKNOWN") => CheckValue::Unknown,
            _ => CheckValue::Unknown,
        };
        let audited_at = yaml_scalar_value(&text, "audited_at").unwrap_or_default();
        let recorded_keys = subjects
            .iter()
            .map(audit_subject_key)
            .collect::<BTreeSet<_>>();
        for test_id in test_ids {
            // §8.5: valid = the recorded subject SET exactly equals the
            // currently required subject set for this Test/kind, AND every
            // recorded subject's hash still matches current. Set-equality
            // catches both a record that never bound everything required
            // (an omitted target, an unwalked upstream SPEC) and a record
            // whose requirement grew or shrank since acceptance; hash
            // currency alone (the pre-fix predicate) caught neither. An
            // unresolvable required closure (e.g. a dangling SPEC ref) fails
            // the record closed rather than counting it valid.
            let Some(test) = scan.tests.iter().find(|test| test.id.as_str() == test_id) else {
                continue;
            };
            let valid = required_subject_keys(layout, scan, test, kind).is_ok_and(|required| {
                recorded_keys == required
                    && subjects
                        .iter()
                        .all(|subject| subject_is_current(root, layout, scan, subject))
            });
            per_test
                .entry(test_id)
                .or_default()
                .push((id.clone(), valid, verdict, audited_at.clone()));
        }
    }

    // §8.5: among the valid records bound to one subject set, this kind's
    // FAIL marker (MISMATCH for impl-consistency, FAIL otherwise) dominates;
    // otherwise the chronologically latest valid record's verdict is
    // authoritative -- audited_at is consulted, not just severity-max.
    let fail_marker = if kind == "impl-consistency" {
        CheckValue::Mismatch
    } else {
        CheckValue::Fail
    };

    let mut overall = CheckValue::Pass;
    let mut basis = Vec::new();
    for test in &scan.tests {
        let test_id = test.id.as_str();
        // §7.5/§8.1/§8.3: for impl-consistency, an unresolved declared target
        // (absent -> MISSING, ambiguous -> MISMATCH; condition-keyed, not
        // code-keyed, since §6.2 pools both under E-SCAN-004) is a
        // resolution-failure state the audit-verdict mapping must not
        // overwrite. It is checked -- and reported -- independently of
        // whether any audit record exists, never combined with one.
        if kind == "impl-consistency" {
            if let Some((value, reason)) = target_resolution_check_value(scan, test) {
                overall = combine_values(overall, value);
                basis.push(format!("Test {test_id}: {value:?} ({reason})"));
                continue;
            }
        }
        let records = per_test.get(test_id).cloned().unwrap_or_default();
        let current = if records.is_empty() {
            CheckValue::NotChecked
        } else {
            let valid = records
                .iter()
                .filter(|(_, current, _, _)| *current)
                .collect::<Vec<_>>();
            if valid.is_empty() {
                CheckValue::Stale
            } else if valid
                .iter()
                .any(|(_, _, verdict, _)| *verdict == fail_marker)
            {
                fail_marker
            } else {
                valid
                    .iter()
                    .max_by(|left, right| compare_recency(&left.3, &left.0, &right.3, &right.0))
                    .map_or(CheckValue::Unknown, |(_, _, verdict, _)| *verdict)
            }
        };
        overall = combine_values(overall, current);
        if records.is_empty() {
            basis.push(format!("Test {test_id}: {current:?} (audit missing)"));
        } else {
            let record_basis = records
                .iter()
                .map(|(id, valid, _, _)| format!("{id}{}", if *valid { "" } else { " (stale)" }))
                .collect::<Vec<_>>();
            basis.push(format!(
                "Test {test_id}: {current:?} (audits: {})",
                record_basis.join(", ")
            ));
        }
    }
    (overall, basis)
}

/// Approval validity has exactly one derivation. Comparing an Approval to the
/// VO file alone would ignore the upstream closure its subject aggregates.
fn vo_is_approved(layout: &VerifyLayout, id: &str) -> bool {
    let Ok(vo) = read_vo(layout, id) else {
        return false;
    };
    let subject = current_approval_subject(layout, &vo);
    derive_vo_status(layout, &vo, subject.as_ref()).approved
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
        // Bound to both the SPEC record's own text and its referenced
        // Specification source (基本仕様 §7.3/§7.5, 詳細設計 §8.5): either
        // one changing alone must invalidate the subject.
        "spec" => subject.id.as_deref().and_then(|id| {
            let record_text = read_text(&layout.spec_dir().join(format!("{id}.yaml"))).ok()?;
            let path = yaml_scalar_value(&record_text, "path")?;
            let source_text = fs::read_to_string(root.join(path)).ok()?;
            Some(hash_spec_audit_subject(&record_text, &source_text))
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
                |record| match evidence_record_validity(
                    root,
                    record,
                    test,
                    scan,
                    built_in_test_runners(),
                ) {
                    CheckValue::Pass => match record.result {
                        vtest_model::TestResult::Pass => CheckValue::Pass,
                        vtest_model::TestResult::Fail => CheckValue::Fail,
                    },
                    // §11.2: an invalid Evidence is never mined for a runtime result --
                    // hold the SAME freshness verdict the ladder produced (STALE,
                    // MISMATCH, UNKNOWN, ...), not a fixed STALE for every case.
                    non_pass => non_pass,
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
                let validity =
                    evidence_record_validity(root, record, test, scan, built_in_test_runners());
                // §11.2: propagate the SAME freshness verdict for an invalid
                // Evidence (mirrors evaluate_test_execution); the checked:false
                // NOT_CHECKED escape is spec-gated to a VALID Evidence only.
                if validity != CheckValue::Pass {
                    validity
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
                evidence_record_validity(root, record, test, scan, built_in_test_runners())
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
                evidence_record_validity(root, record, test, scan, built_in_test_runners())
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
    registry: &AdapterRegistry,
) -> CheckValue {
    let adapter = match record.adapter.as_ref() {
        Some(adapter) => {
            if adapter != &test.execution.adapter {
                return CheckValue::Mismatch;
            }
            adapter.clone()
        }
        None => {
            // 基本仕様 §7.8 line 488 / 詳細設計 §3.7: an Evidence reader
            // accepts a record lacking an adapter ID. It is evaluated as
            // compat Evidence ONLY when the current Test is rust-cargo and
            // the record's runner kind uniquely confirms Rust execution (no
            // other adapter's runner emits these exact kinds) -- everything
            // else is UNKNOWN, never a guessed STALE. The writer always
            // stamps `adapter` (別紙C §18.3.3); this branch exists solely
            // for legacy/hand-written records. Once confirmed, the record is
            // treated exactly as an explicit rust-cargo record from here on
            // -- content-hash or Execution State drift discovered below is
            // still STALE (VO-EXEC-12: STALE is reserved for a failure of
            // one of those conditions), not a second UNKNOWN.
            if test.execution.adapter.as_str() != RUST_CARGO_ADAPTER_ID
                || !matches!(record.runner.kind.as_str(), "cargo-test" | "cargo-llvm-cov")
            {
                return CheckValue::Unknown;
            }
            test.execution.adapter.clone()
        }
    };
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
        return CheckValue::Stale;
    }
    // §1315 / §9 / §11.2: reconstruct the current Execution State subject
    // through the RECORD'S OWN adapter -- resolved from the registry, never
    // hardcoded to rust-cargo -- and compare it to the recorded hash. An
    // adapter the registry cannot resolve, or one that cannot prove its
    // current input closure is complete, cannot prove freshness: UNKNOWN,
    // never a guessed STALE or a guessed PASS (基本仕様 §7.8). A subject
    // that moved -- input manifest, HEAD, toolchain -- is STALE even when
    // the Test and target hashes are unchanged.
    let Ok(runner) = registry.test_runner(&adapter) else {
        return CheckValue::Unknown;
    };
    let draft = runner.current_execution_state(root, test, &record.runner.kind);
    match hash_execution_state_draft(&adapter, &draft) {
        None => CheckValue::Unknown,
        Some(current) if Some(&current) == execution_state.hash.as_ref() => CheckValue::Pass,
        Some(_) => CheckValue::Stale,
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
    use std::sync::Arc;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };
    use vtest_adapter_api::{
        AdapterCapability, AdapterDescriptor, AdapterError, AdapterRegistration,
        ExecutionStateDraft, RunnerOutcome, TestRunnerAdapter,
    };
    use vtest_model::{
        AdapterId, CanonicalProjection, EvidenceHashes, EvidenceRecord, EvidenceTargetHash,
        ExecutionDescriptor, ExecutionStateSubject, ProjectPath, ReqId, Revision, RunnerInfo,
        ScanSummary, SourceFunction, SourceLocation, SourceRange, SpecId, TargetExecution,
        TargetExecutionObservation, TargetRef, TestEntity, TestId, TestResult, TestSuite, VoId,
    };
    use vtest_scan::rust_cargo_execution_state_hash;
    use vtest_store::{
        approval_subject_hash, init_project, new_record_id, vo_record_subject, ApprovalDependency,
        ApprovalRecord, Approver, AuditBasisRecord, AuditReasonRecord, AuditTargetVerdictRecord,
        AuditorRecord, SpecRecord, SpecRef,
    };

    /// @vtest.id TEST-VERIFY-001
    /// @vtest.covers VO-VERIFY-001
    /// @vtest.target crates/vtest-verify/src/lib.rs::compare_evidence_recency
    /// @vtest.intent Evidence recency ordering uses actual RFC3339 instant, not lexical order
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
        // Every fixture Test covers VO-ONE; give it a real record so `vo`
        // audit subjects (required by evaluate_test_audit's set-equality
        // check, §8.5) can resolve a current hash.
        fs::write(
            layout.vo_dir().join("VO-ONE.yaml"),
            VoRecord {
                id: VoId::new("VO-ONE"),
                parent: None,
                requirements: Vec::new(),
                spec_refs: Vec::new(),
                claim: "fixture claim".to_owned(),
                dimensions: Vec::new(),
                coverage_policy: None,
                combinations: Vec::new(),
                representative_cases: Vec::new(),
                status: None,
                created: "2026-08-08T00:00:00Z".to_owned(),
                updated: "2026-08-08T00:00:00Z".to_owned(),
            }
            .to_yaml(),
        )
        .expect("write VO-ONE fixture record");
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
    ) -> String {
        let canonical = test.targets[0].normalized();
        write_static_audit_at(
            layout,
            verdict,
            static_audit_subjects(test, hash),
            Some(&canonical),
            "2026-08-08T00:00:00Z",
        )
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

    /// Write a legacy-shape record (no per-target verdicts). Used by tests whose
    /// records are expected to be excluded (stale subjects), where the shape does
    /// not matter because staleness dominates.
    fn write_static_audit_with_subjects(
        layout: &VerifyLayout,
        verdict: &str,
        subjects: Vec<AuditSubjectRecord>,
    ) -> String {
        write_static_audit_at(layout, verdict, subjects, None, "2026-08-08T00:00:00Z")
    }

    fn write_static_audit_at(
        layout: &VerifyLayout,
        verdict: &str,
        mut subjects: Vec<AuditSubjectRecord>,
        canonical: Option<&str>,
        audited_at: &str,
    ) -> String {
        let config = load_config(&layout.root).expect("load fixture config");
        subjects.push(AuditSubjectRecord {
            id: Some("CONFIG".to_owned()),
            locator: None,
            hash: static_audit_config_subject_hash(&config),
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
                .map(|rule| {
                    let target_scoped = matches!(rule, "DA-002" | "DA-003");
                    // Per-target mode: the target-scoped rules carry the declared
                    // target at `verdict`, the rest are PASS, and the record folds
                    // to `verdict`. Legacy mode keeps the old shape (DA-001 carries
                    // `verdict`, no per-target list).
                    let (rule_verdict, targets) = match (canonical, target_scoped) {
                        (Some(canonical), true) => (
                            verdict,
                            vec![AuditTargetVerdictRecord {
                                target: canonical.to_owned(),
                                verdict: verdict.to_owned(),
                                basis: vec![AuditBasisRecord {
                                    kind: "test-code".to_owned(),
                                    reference: "tests/static.rs:1".to_owned(),
                                }],
                            }],
                        ),
                        (Some(_), false) => ("PASS", Vec::new()),
                        (None, _) => (if rule == "DA-001" { verdict } else { "PASS" }, Vec::new()),
                    };
                    AuditReasonRecord {
                        rule: Some(rule.to_owned()),
                        verdict: Some(rule_verdict.to_owned()),
                        claim: format!("fixture result for {rule}"),
                        basis: vec![AuditBasisRecord {
                            kind: "test-code".to_owned(),
                            reference: "tests/static.rs:1".to_owned(),
                        }],
                        targets,
                    }
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

    /// Write a per-target record whose DA-002 and DA-003 verdicts differ, so the
    /// join can be exercised where reachability (DA-002) and result verification
    /// (DA-003) diverge. Other rules are PASS; the record folds accordingly.
    fn write_static_audit_split(
        layout: &VerifyLayout,
        test: &TestEntity,
        da002: &str,
        da003: &str,
        audited_at: &str,
    ) -> String {
        let canonical = test.targets[0].normalized();
        let mut subjects = static_audit_subjects(test, test.content_hash.clone());
        let config = load_config(&layout.root).expect("load fixture config");
        subjects.push(AuditSubjectRecord {
            id: Some("CONFIG".to_owned()),
            locator: None,
            hash: static_audit_config_subject_hash(&config),
        });
        let rule_verdict = |rule: &str| match rule {
            "DA-002" => da002,
            "DA-003" => da003,
            _ => "PASS",
        };
        let record_verdict = if da002 == "FAIL" || da003 == "FAIL" {
            "FAIL"
        } else if da002 == "UNKNOWN" || da003 == "UNKNOWN" {
            "UNKNOWN"
        } else {
            "PASS"
        };
        let id = new_record_id();
        let record = AuditRecord {
            id: id.clone(),
            kind: "static".to_owned(),
            bundle_id: None,
            subjects,
            verdict: record_verdict.to_owned(),
            reasons: ["DA-001", "DA-002", "DA-003", "DA-004", "DA-005", "DA-006"]
                .into_iter()
                .map(|rule| AuditReasonRecord {
                    rule: Some(rule.to_owned()),
                    verdict: Some(rule_verdict(rule).to_owned()),
                    claim: format!("fixture result for {rule}"),
                    basis: vec![AuditBasisRecord {
                        kind: "test-code".to_owned(),
                        reference: "tests/static.rs:1".to_owned(),
                    }],
                    targets: if matches!(rule, "DA-002" | "DA-003") {
                        vec![AuditTargetVerdictRecord {
                            target: canonical.clone(),
                            verdict: rule_verdict(rule).to_owned(),
                            basis: vec![AuditBasisRecord {
                                kind: "test-code".to_owned(),
                                reference: "tests/static.rs:1".to_owned(),
                            }],
                        }]
                    } else {
                        Vec::new()
                    },
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

    /// Build a valid Evidence (passing `evidence_record_validity`) whose
    /// per-target target_execution carries `result`/`count`, keyed to the Test.
    fn valid_runtime_evidence(
        layout: &VerifyLayout,
        test: &TestEntity,
        scan: &ScanResult,
        result: CheckValue,
        count: Option<u64>,
    ) -> BTreeMap<String, EvidenceRecord> {
        let adapter = test.execution.adapter.clone();
        let kind = "cargo-llvm-cov";
        let hash = rust_cargo_execution_state_hash(&layout.root, &adapter, test, kind, false)
            .expect("fixture execution state hash resolves");
        let target_hashes = test
            .targets
            .iter()
            .map(|target_ref| EvidenceTargetHash {
                target: target_ref.normalized(),
                target_construct: find_target_source(scan, target_ref)
                    .expect("resolve target")
                    .content_hash
                    .clone(),
            })
            .collect::<Vec<_>>();
        let target_execution_targets = test
            .targets
            .iter()
            .map(|target_ref| TargetExecutionObservation {
                target: find_target_source(scan, target_ref)
                    .expect("resolve target")
                    .target
                    .normalized(),
                result,
                count,
            })
            .collect::<Vec<_>>();
        let record = EvidenceRecord {
            id: new_record_id(),
            test_id: test.id.clone(),
            adapter: Some(adapter),
            result: TestResult::Pass,
            executed_at: "2026-08-08T00:00:00Z".to_owned(),
            revision: Revision {
                commit: Some("abc123".to_owned()),
                dirty: false,
            },
            execution_state: Some(ExecutionStateSubject {
                schema: "rust-cargo-execution-state-v1".to_owned(),
                complete: true,
                hash: Some(hash),
            }),
            hashes: EvidenceHashes {
                test_subject: Some(test.content_hash.clone()),
                targets: target_hashes,
                compatibility: None,
            },
            runner: RunnerInfo {
                kind: kind.to_owned(),
                command: "cargo llvm-cov".to_owned(),
                exit_code: 0,
            },
            target_execution: TargetExecution {
                checked: true,
                method: Some("llvm-cov".to_owned()),
                result: Some(result),
                targets: target_execution_targets,
                compatibility_count: None,
            },
            log_ref: "cache/logs/e.log".to_owned(),
        };
        let mut evidence = BTreeMap::new();
        evidence.insert(test.id.as_str().to_owned(), record);
        evidence
    }

    /// A `TestRunnerAdapter` test double that reports a fixed, caller-supplied
    /// Execution State draft regardless of input -- lets a test exercise the
    /// registry-dispatched `current_execution_state` path for an adapter the
    /// built-in production registry does not register.
    struct FixedDraftRunner {
        draft: ExecutionStateDraft,
    }

    impl TestRunnerAdapter for FixedDraftRunner {
        fn run(
            &self,
            _root: &Path,
            _config: &CanonicalProjection,
            _test: &TestEntity,
        ) -> Result<RunnerOutcome, AdapterError> {
            unimplemented!("not exercised by evidence_record_validity")
        }

        fn current_execution_state(
            &self,
            _root: &Path,
            _test: &TestEntity,
            _runner_kind: &str,
        ) -> ExecutionStateDraft {
            self.draft.clone()
        }
    }

    fn synthetic_registry(draft: ExecutionStateDraft) -> AdapterRegistry {
        let mut registration = AdapterRegistration::new(AdapterDescriptor {
            id: AdapterId::new("synthetic"),
            languages: vec!["synthetic".to_owned()],
            capabilities: vec![AdapterCapability::TestRunner],
            config_namespace: "synthetic".to_owned(),
        });
        registration.test_runner = Some(Arc::new(FixedDraftRunner { draft }));
        AdapterRegistry::from_registrations([registration]).expect("registry")
    }

    fn with_execution_adapter(test: &TestEntity, adapter: &str) -> TestEntity {
        let mut test = test.clone();
        test.execution.adapter = AdapterId::new(adapter);
        test
    }

    /// @vtest.id TEST-VERIFY-019
    /// @vtest.covers VO-ADAPTER-10
    /// @vtest.target crates/vtest-verify/src/lib.rs::evidence_record_validity
    /// @vtest.intent An adapter-less Evidence for a rust-cargo Test, whose
    /// runner kind is a recognised Rust runner kind and whose content hashes
    /// are all current, is compat Evidence and reaches PASS (基本仕様 §7.8
    /// line 488; same branch also stated by VO-EXEC-12). Fixed: this branch
    /// previously returned STALE unconditionally before any runner/hash
    /// inspection.
    #[test]
    fn evidence_validity_adapter_less_rust_cargo_compat_reaches_pass() {
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let test = &scan.tests[0];
        let mut record = valid_runtime_evidence(&layout, test, &scan, CheckValue::Pass, Some(1))
            .remove(test.id.as_str())
            .expect("evidence for test");
        record.adapter = None;
        assert_eq!(
            evidence_record_validity(&layout.root, &record, test, &scan, built_in_test_runners()),
            CheckValue::Pass
        );
    }

    /// @vtest.id TEST-VERIFY-020
    /// @vtest.covers VO-ADAPTER-10
    /// @vtest.target crates/vtest-verify/src/lib.rs::evidence_record_validity
    /// @vtest.intent An adapter-less Evidence whose runner kind is not a
    /// recognised Rust runner kind cannot uniquely confirm Rust execution:
    /// UNKNOWN, never a guessed STALE (same branch also stated by
    /// VO-EXEC-12).
    #[test]
    fn evidence_validity_adapter_less_unconfirmable_runner_is_unknown() {
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let test = &scan.tests[0];
        let mut record = valid_runtime_evidence(&layout, test, &scan, CheckValue::Pass, Some(1))
            .remove(test.id.as_str())
            .expect("evidence for test");
        record.adapter = None;
        record.runner.kind = "synth-runner".to_owned();
        assert_eq!(
            evidence_record_validity(&layout.root, &record, test, &scan, built_in_test_runners()),
            CheckValue::Unknown
        );
    }

    /// @vtest.id TEST-VERIFY-021
    /// @vtest.covers VO-ADAPTER-05
    /// @vtest.target crates/vtest-verify/src/lib.rs::evidence_record_validity
    /// @vtest.intent An Evidence whose adapter matches the current Test's
    /// execution adapter, for an adapter the running registry cannot resolve
    /// a TestRunnerAdapter for, cannot have its input closure proven
    /// complete: UNKNOWN, not the pre-fix false STALE from an unconditional
    /// rust-cargo re-derivation.
    #[test]
    fn evidence_validity_explicit_unresolvable_adapter_is_unknown_not_stale() {
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let test = with_execution_adapter(&scan.tests[0], "synthetic");
        let record = valid_runtime_evidence(&layout, &test, &scan, CheckValue::Pass, Some(1))
            .remove(test.id.as_str())
            .expect("evidence for test");
        assert_eq!(
            evidence_record_validity(&layout.root, &record, &test, &scan, built_in_test_runners()),
            CheckValue::Unknown
        );
    }

    /// @vtest.id TEST-VERIFY-022
    /// @vtest.covers VO-ADAPTER-05
    /// @vtest.target crates/vtest-verify/src/lib.rs::evidence_record_validity
    /// @vtest.intent PASS is reachable for a non-rust-cargo adapter once its
    /// TestRunnerAdapter is resolvable through the registry -- freshness
    /// re-derivation dispatches via the record's own adapter and schema,
    /// never a hardcoded rust-cargo path (詳細設計 line 810/1390). Stretch:
    /// the production `built_in_test_runners()` registers only rust-cargo
    /// (see closure notes), so this exercises a caller-supplied registry
    /// directly, and keeps the fixture's rust-cargo target locators rather
    /// than modelling a full non-Rust scan.
    #[test]
    fn evidence_validity_registered_non_rust_cargo_adapter_reaches_pass() {
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let test = with_execution_adapter(&scan.tests[0], "synthetic");
        let draft = ExecutionStateDraft {
            schema_id: "synthetic-execution-state-v1".to_owned(),
            schema_version: "1".to_owned(),
            complete: true,
            head_revision: None,
            runner_kind: "synth-runner".to_owned(),
            invocation: CanonicalProjection::Null,
            toolchain_identity: "synthetic-toolchain".to_owned(),
            effective_config: CanonicalProjection::Null,
            inputs: Vec::new(),
        };
        let hash = hash_execution_state_draft(&test.execution.adapter, &draft)
            .expect("a complete draft always hashes");
        let mut record = valid_runtime_evidence(&layout, &test, &scan, CheckValue::Pass, Some(1))
            .remove(test.id.as_str())
            .expect("evidence for test");
        record.runner.kind = "synth-runner".to_owned();
        record.execution_state = Some(ExecutionStateSubject {
            schema: draft.schema_id.clone(),
            complete: true,
            hash: Some(hash),
        });
        let registry = synthetic_registry(draft);
        assert_eq!(
            evidence_record_validity(&layout.root, &record, &test, &scan, &registry),
            CheckValue::Pass
        );
    }

    /// Write a minimal `kind: vo-coverage` Audit Record binding one VO
    /// subject, mirroring `write_static_audit_at`'s construction style.
    fn write_vo_coverage_audit(
        layout: &VerifyLayout,
        vo_id: &str,
        vo_hash: &ContentHash,
        verdict: &str,
        audited_at: &str,
    ) -> String {
        let id = new_record_id();
        let record = AuditRecord {
            id: id.clone(),
            kind: "vo-coverage".to_owned(),
            bundle_id: None,
            subjects: vec![AuditSubjectRecord {
                id: Some(vo_id.to_owned()),
                locator: None,
                hash: vo_hash.clone(),
            }],
            verdict: verdict.to_owned(),
            reasons: vec![AuditReasonRecord {
                rule: None,
                verdict: None,
                claim: "fixture vo-coverage result".to_owned(),
                basis: vec![AuditBasisRecord {
                    kind: "test-code".to_owned(),
                    reference: "tests/static.rs:1".to_owned(),
                }],
                targets: Vec::new(),
            }],
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
            record.to_yaml().expect("serialise vo-coverage audit"),
        )
        .expect("write vo-coverage audit");
        id
    }

    /// Write a minimal current, approved VO record (empty upstream closure)
    /// so `vo_is_approved` reports true, letting a test reach the
    /// `audit_value == Pass` leg of `evaluate_vo_coverage` without a full
    /// SPEC/REQ chain.
    fn write_approved_vo(layout: &VerifyLayout, vo_id: &str) -> ContentHash {
        let path = layout.vo_dir().join(format!("{vo_id}.yaml"));
        let text = VoRecord {
            id: VoId::new(vo_id),
            parent: None,
            requirements: Vec::new(),
            spec_refs: Vec::new(),
            claim: "fixture".to_owned(),
            dimensions: Vec::new(),
            coverage_policy: None,
            combinations: Vec::new(),
            representative_cases: Vec::new(),
            status: None,
            created: "2026-01-01".to_owned(),
            updated: "2026-01-01".to_owned(),
        }
        .to_yaml();
        fs::write(&path, &text).expect("write VO record");
        let vo = read_vo(layout, vo_id).expect("read VO record");
        let approval_id = new_record_id();
        let dependencies: Vec<ApprovalDependency> = Vec::new();
        let approval = ApprovalRecord {
            id: approval_id.clone(),
            subject: VoId::new(vo_id),
            subject_hash: approval_subject_hash(&vo_record_subject(&vo), &dependencies),
            dependencies: Some(dependencies),
            approver: Approver {
                kind: "human".to_owned(),
                id: "reviewer".to_owned(),
                model: None,
            },
            basis: Vec::new(),
            approved_at: "2026-08-08T00:00:00Z".to_owned(),
        };
        fs::write(
            layout.approvals_dir().join(format!("{approval_id}.yaml")),
            approval.to_yaml(),
        )
        .expect("write approval");
        ContentHash::from_text(&text)
    }

    /// Register a SPEC record whose `sha256` matches `source` exactly (a
    /// "current" SPEC per 詳細設計 §1.3 L87). Returns the source text's
    /// subject hash so callers can independently compute the "spec" audit
    /// subject via `hash_spec_audit_subject`.
    fn write_current_spec(layout: &VerifyLayout, spec_id: &str, path: &str, source: &str) {
        let full_path = layout.root.join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).expect("create fixture Specification source directory");
        }
        fs::write(&full_path, source).expect("write fixture Specification source");
        let record = SpecRecord {
            id: SpecId::new(spec_id),
            kind: "document".to_owned(),
            path: path.to_owned(),
            sha256: hash_specification_source(source),
            title: None,
            note: None,
            registered_at: "2026-01-01T00:00:00Z".to_owned(),
        };
        fs::write(
            layout.spec_dir().join(format!("{spec_id}.yaml")),
            record.to_yaml(),
        )
        .expect("write fixture SPEC record");
    }

    fn write_req(
        layout: &VerifyLayout,
        req_id: &str,
        status: &str,
        spec_refs: Vec<SpecRef>,
    ) -> ContentHash {
        let text = ReqRecord {
            id: ReqId::new(req_id),
            parent: None,
            spec_refs,
            summary: "fixture requirement".to_owned(),
            status: status.to_owned(),
            created: "2026-01-01T00:00:00Z".to_owned(),
            updated: "2026-01-01T00:00:00Z".to_owned(),
        }
        .to_yaml();
        fs::write(layout.req_dir().join(format!("{req_id}.yaml")), &text)
            .expect("write fixture REQ record");
        ContentHash::from_text(&text)
    }

    fn write_vo_with_requirements(layout: &VerifyLayout, vo_id: &str, requirements: &[&str]) {
        let text = VoRecord {
            id: VoId::new(vo_id),
            parent: None,
            requirements: requirements.iter().map(|id| ReqId::new(*id)).collect(),
            spec_refs: Vec::new(),
            claim: "fixture".to_owned(),
            dimensions: Vec::new(),
            coverage_policy: None,
            combinations: Vec::new(),
            representative_cases: Vec::new(),
            status: None,
            created: "2026-01-01T00:00:00Z".to_owned(),
            updated: "2026-01-01T00:00:00Z".to_owned(),
        }
        .to_yaml();
        fs::write(layout.vo_dir().join(format!("{vo_id}.yaml")), text)
            .expect("write fixture VO record");
    }

    fn write_spec_coverage_audit(
        layout: &VerifyLayout,
        subjects: Vec<AuditSubjectRecord>,
        verdict: &str,
        audited_at: &str,
    ) -> String {
        let id = new_record_id();
        let record = AuditRecord {
            id: id.clone(),
            kind: "spec-coverage".to_owned(),
            bundle_id: None,
            subjects,
            verdict: verdict.to_owned(),
            reasons: vec![AuditReasonRecord {
                rule: None,
                verdict: None,
                claim: "fixture spec-coverage result".to_owned(),
                basis: vec![
                    AuditBasisRecord {
                        kind: "spec".to_owned(),
                        reference: "SPEC-FX#1".to_owned(),
                    },
                    AuditBasisRecord {
                        kind: "req".to_owned(),
                        reference: "REQ-FX".to_owned(),
                    },
                ],
                targets: Vec::new(),
            }],
            exclusions: Vec::new(),
            auditor: AuditorRecord {
                kind: "agent".to_owned(),
                id: "fixture".to_owned(),
                model: Some("fixture-model".to_owned()),
            },
            confidence: Some("high".to_owned()),
            audited_at: audited_at.to_owned(),
            revision: Revision {
                commit: None,
                dirty: true,
            },
        };
        fs::write(
            layout.audits_dir().join(format!("{id}.yaml")),
            record.to_yaml().expect("serialise spec-coverage audit"),
        )
        .expect("write spec-coverage audit");
        id
    }

    /// @vtest.id TEST-VERIFY-040
    /// @vtest.covers VO-EXIST-04
    /// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_spec_coverage
    /// @vtest.intent full verification or a requested scope with 0 registered
    /// SPECs yields spec_coverage = MISSING, never PASS -- even when every
    /// selected REQ already has a linked VO (基本仕様 §7.4, 別紙C §18.3.5).
    #[test]
    fn spec_coverage_zero_registered_specs_is_missing_never_pass() {
        let (layout, scan) = static_fixture(&[]);
        write_req(&layout, "REQ-X", "active", Vec::new());
        write_vo_with_requirements(&layout, "VO-X", &["REQ-X"]);
        let selection = ScopeSelection {
            entity_scope: None,
            test_ids: BTreeSet::new(),
            vo_ids: BTreeSet::from(["VO-X".to_owned()]),
            req_ids: BTreeSet::from(["REQ-X".to_owned()]),
            spec_ids: BTreeSet::new(),
        };
        let (value, basis) = evaluate_spec_coverage(&layout.root, &layout, &scan, &selection);
        assert_eq!(value, CheckValue::Missing, "basis: {basis:?}");
        assert!(basis
            .iter()
            .any(|line| line.contains("no registered Specification")));
    }

    /// @vtest.id TEST-VERIFY-041
    /// @vtest.covers VO-PLAN-02
    /// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_one_spec_coverage
    /// @vtest.intent 詳細設計 §1.3 L87: a SPEC record whose registered sha256
    /// no longer matches its current Specification source is STALE, checked
    /// independently of any spec-coverage audit.
    #[test]
    fn spec_coverage_registered_sha256_mismatch_is_stale() {
        let (layout, scan) = static_fixture(&[]);
        write_current_spec(&layout, "SPEC-FX", "docs/spec.md", "original text");
        // Mutate the Specification source without re-registering the SPEC
        // record, so the record's `sha256` field goes stale.
        fs::write(layout.root.join("docs/spec.md"), "mutated text")
            .expect("mutate fixture Specification source");
        let (value, reason) = evaluate_one_spec_coverage(&layout.root, &layout, &scan, "SPEC-FX");
        assert_eq!(value, CheckValue::Stale, "reason: {reason}");
    }

    /// @vtest.id TEST-VERIFY-049
    /// @vtest.covers VO-PLAN-02
    /// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_one_spec_coverage
    /// @vtest.intent 詳細設計 L1349 / 別紙C §18.3.5: a SPEC record whose
    /// referenced Specification source no longer exists on disk is MISSING,
    /// even though the record itself is still readable (the INTAKE-08
    /// re-skeptic finding: deleting the SPEC's source left spec_coverage PASS).
    #[test]
    fn spec_coverage_missing_specification_source_is_missing() {
        let (layout, scan) = static_fixture(&[]);
        write_current_spec(&layout, "SPEC-FX", "docs/spec.md", "fixture text");
        write_req(
            &layout,
            "REQ-FX",
            "active",
            vec![SpecRef {
                spec: SpecId::new("SPEC-FX"),
                section: "1".to_owned(),
            }],
        );
        fs::remove_file(layout.root.join("docs/spec.md"))
            .expect("delete fixture Specification source");
        let (value, reason) = evaluate_one_spec_coverage(&layout.root, &layout, &scan, "SPEC-FX");
        assert_eq!(value, CheckValue::Missing, "reason: {reason}");
    }

    /// @vtest.id TEST-VERIFY-042
    /// @vtest.covers VO-PLAN-02
    /// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_one_spec_coverage
    /// @vtest.intent 詳細設計 L1144/L1350: 0 active REQ referencing a current
    /// SPEC keeps spec_coverage MISSING regardless of a submission -- even a
    /// spec-coverage audit bound to the (degenerate) spec-only subject set
    /// does not promote it.
    #[test]
    fn spec_coverage_zero_active_req_stays_missing_even_with_a_submission() {
        let (layout, scan) = static_fixture(&[]);
        write_current_spec(&layout, "SPEC-FX", "docs/spec.md", "fixture text");
        let spec_hash = hash_spec_audit_subject(
            &read_text(&layout.spec_dir().join("SPEC-FX.yaml")).expect("read SPEC record"),
            "fixture text",
        );
        write_spec_coverage_audit(
            &layout,
            vec![AuditSubjectRecord {
                id: Some("SPEC-FX".to_owned()),
                locator: None,
                hash: spec_hash,
            }],
            "PASS",
            "2026-08-08T00:00:00Z",
        );
        let (value, reason) = evaluate_one_spec_coverage(&layout.root, &layout, &scan, "SPEC-FX");
        assert_eq!(value, CheckValue::Missing, "reason: {reason}");
    }

    /// @vtest.id TEST-VERIFY-043
    /// @vtest.covers VO-PLAN-03
    /// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_one_spec_coverage
    /// @vtest.intent 詳細設計 L1350: an active REQ whose reference to the
    /// target SPEC has an empty `section` is MISMATCH.
    #[test]
    fn spec_coverage_active_req_with_empty_section_is_mismatch() {
        let (layout, scan) = static_fixture(&[]);
        write_current_spec(&layout, "SPEC-FX", "docs/spec.md", "fixture text");
        write_req(
            &layout,
            "REQ-FX",
            "active",
            vec![SpecRef {
                spec: SpecId::new("SPEC-FX"),
                section: String::new(),
            }],
        );
        let (value, reason) = evaluate_one_spec_coverage(&layout.root, &layout, &scan, "SPEC-FX");
        assert_eq!(value, CheckValue::Mismatch, "reason: {reason}");
    }

    /// @vtest.id TEST-VERIFY-044
    /// @vtest.covers VO-PLAN-02
    /// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_one_spec_coverage
    /// @vtest.intent a valid COMPLETE (submitted as PASS) spec-coverage audit
    /// whose recorded subjects exactly equal the required set (SPEC + the
    /// complete active-REQ set) yields PASS.
    #[test]
    fn spec_coverage_valid_complete_audit_is_pass() {
        let (layout, scan) = static_fixture(&[]);
        write_current_spec(&layout, "SPEC-FX", "docs/spec.md", "fixture text");
        let req_hash = write_req(
            &layout,
            "REQ-FX",
            "active",
            vec![SpecRef {
                spec: SpecId::new("SPEC-FX"),
                section: "1".to_owned(),
            }],
        );
        let spec_hash = hash_spec_audit_subject(
            &read_text(&layout.spec_dir().join("SPEC-FX.yaml")).expect("read SPEC record"),
            "fixture text",
        );
        write_spec_coverage_audit(
            &layout,
            vec![
                AuditSubjectRecord {
                    id: Some("SPEC-FX".to_owned()),
                    locator: None,
                    hash: spec_hash,
                },
                AuditSubjectRecord {
                    id: Some("REQ-FX".to_owned()),
                    locator: None,
                    hash: req_hash,
                },
            ],
            "PASS",
            "2026-08-08T00:00:00Z",
        );
        let (value, reason) = evaluate_one_spec_coverage(&layout.root, &layout, &scan, "SPEC-FX");
        assert_eq!(value, CheckValue::Pass, "reason: {reason}");
    }

    /// @vtest.id TEST-VERIFY-045
    /// @vtest.covers VO-PLAN-05
    /// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_one_spec_coverage
    /// @vtest.intent 詳細設計 L1352: a current SPEC with a well-formed active
    /// REQ but zero spec-coverage audits yields NOT_CHECKED, not PASS.
    #[test]
    fn spec_coverage_no_audit_is_not_checked() {
        let (layout, scan) = static_fixture(&[]);
        write_current_spec(&layout, "SPEC-FX", "docs/spec.md", "fixture text");
        write_req(
            &layout,
            "REQ-FX",
            "active",
            vec![SpecRef {
                spec: SpecId::new("SPEC-FX"),
                section: "1".to_owned(),
            }],
        );
        let (value, reason) = evaluate_one_spec_coverage(&layout.root, &layout, &scan, "SPEC-FX");
        assert_eq!(value, CheckValue::NotChecked, "reason: {reason}");
    }

    /// @vtest.id TEST-VERIFY-046
    /// @vtest.covers VO-PLAN-05
    /// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_one_spec_coverage
    /// @vtest.intent §8.5 set-equality: a spec-coverage audit bound to only
    /// the SPEC subject (omitting the required REQ subject) is invalid, so
    /// the item is STALE, not PASS, even though the audit's own verdict is
    /// PASS.
    #[test]
    fn spec_coverage_audit_missing_a_required_req_subject_is_stale() {
        let (layout, scan) = static_fixture(&[]);
        write_current_spec(&layout, "SPEC-FX", "docs/spec.md", "fixture text");
        write_req(
            &layout,
            "REQ-FX",
            "active",
            vec![SpecRef {
                spec: SpecId::new("SPEC-FX"),
                section: "1".to_owned(),
            }],
        );
        let spec_hash = hash_spec_audit_subject(
            &read_text(&layout.spec_dir().join("SPEC-FX.yaml")).expect("read SPEC record"),
            "fixture text",
        );
        write_spec_coverage_audit(
            &layout,
            vec![AuditSubjectRecord {
                id: Some("SPEC-FX".to_owned()),
                locator: None,
                hash: spec_hash,
            }],
            "PASS",
            "2026-08-08T00:00:00Z",
        );
        let (value, reason) = evaluate_one_spec_coverage(&layout.root, &layout, &scan, "SPEC-FX");
        assert_eq!(value, CheckValue::Stale, "reason: {reason}");
    }

    /// @vtest.id TEST-VERIFY-047
    /// @vtest.covers VO-PLAN-02
    /// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_spec_coverage
    /// @vtest.intent multiple SPECs in scope fold with 基本仕様 §4.3's
    /// fail-closed priority: one PASS SPEC and one MISSING SPEC combine to
    /// an overall MISSING, not PASS.
    #[test]
    fn spec_coverage_folds_multiple_specs_fail_closed() {
        let (layout, scan) = static_fixture(&[]);
        write_current_spec(&layout, "SPEC-PASS", "docs/pass.md", "pass text");
        let req_hash = write_req(
            &layout,
            "REQ-PASS",
            "active",
            vec![SpecRef {
                spec: SpecId::new("SPEC-PASS"),
                section: "1".to_owned(),
            }],
        );
        let spec_pass_hash = hash_spec_audit_subject(
            &read_text(&layout.spec_dir().join("SPEC-PASS.yaml")).expect("read SPEC record"),
            "pass text",
        );
        write_spec_coverage_audit(
            &layout,
            vec![
                AuditSubjectRecord {
                    id: Some("SPEC-PASS".to_owned()),
                    locator: None,
                    hash: spec_pass_hash,
                },
                AuditSubjectRecord {
                    id: Some("REQ-PASS".to_owned()),
                    locator: None,
                    hash: req_hash,
                },
            ],
            "PASS",
            "2026-08-08T00:00:00Z",
        );
        // SPEC-MISSING has 0 active REQ referencing it.
        write_current_spec(&layout, "SPEC-MISSING", "docs/missing.md", "missing text");
        let selection = ScopeSelection {
            entity_scope: None,
            test_ids: BTreeSet::new(),
            vo_ids: BTreeSet::new(),
            req_ids: BTreeSet::new(),
            spec_ids: BTreeSet::from(["SPEC-PASS".to_owned(), "SPEC-MISSING".to_owned()]),
        };
        let (value, basis) = evaluate_spec_coverage(&layout.root, &layout, &scan, &selection);
        assert_eq!(value, CheckValue::Missing, "basis: {basis:?}");
        assert!(basis
            .iter()
            .any(|line| line.starts_with("SPEC SPEC-PASS: Pass")));
        assert!(basis
            .iter()
            .any(|line| line.starts_with("SPEC SPEC-MISSING: Missing")));
    }

    /// @vtest.id TEST-VERIFY-048
    /// @vtest.covers VO-PLAN-03
    /// @vtest.target crates/vtest-verify/src/lib.rs::build_tree
    /// @vtest.intent 詳細設計 §11.1 step 9 ("SPECはspec_coverageとREQ部分木の
    /// 合成"): a top-level REQ referencing a SPEC attaches under that SPEC's
    /// own tree node, which owns the spec_coverage item; the REQ node itself
    /// no longer carries spec_coverage (that axis moved off of it).
    #[test]
    fn spec_node_owns_spec_coverage_and_attaches_its_referencing_req_subtree() {
        let (layout, scan) = static_fixture(&[]);
        write_req(
            &layout,
            "REQ-FX",
            "active",
            vec![SpecRef {
                spec: SpecId::new("SPEC-FX"),
                section: "1".to_owned(),
            }],
        );
        let selection = ScopeSelection {
            entity_scope: None,
            test_ids: BTreeSet::new(),
            vo_ids: BTreeSet::new(),
            req_ids: BTreeSet::from(["REQ-FX".to_owned()]),
            spec_ids: BTreeSet::from(["SPEC-FX".to_owned()]),
        };
        let items = vec![ReportItem {
            item: "spec_coverage".to_owned(),
            value: CheckValue::Pass,
            basis: vec!["fixture".to_owned()],
        }];
        let tree = build_tree(&layout, &selection, &scan, &items);
        assert_eq!(tree.len(), 1, "tree: {tree:?}");
        assert_eq!(tree[0].kind, "spec");
        assert_eq!(tree[0].id, "SPEC-FX");
        assert_eq!(
            tree[0]
                .items
                .iter()
                .map(|item| item.item.as_str())
                .collect::<Vec<_>>(),
            vec!["spec_coverage"]
        );
        assert_eq!(
            tree[0].children.len(),
            1,
            "children: {:?}",
            tree[0].children
        );
        assert_eq!(tree[0].children[0].kind, "req");
        assert_eq!(tree[0].children[0].id, "REQ-FX");
        assert!(tree[0].children[0].items.is_empty());
    }

    /// @vtest.id TEST-VERIFY-023
    /// @vtest.covers VO-SEMAUDIT-05
    /// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_vo_coverage
    /// @vtest.intent §8.5, the same two-stage rule as evaluate_test_audit
    /// (VO-SEMAUDIT-05): among the valid records bound to one VO, the
    /// chronologically LATEST valid record's verdict is authoritative, not a
    /// severity-max fold that ignores recency -- an older UNKNOWN and a
    /// newer PASS must fold to PASS (with the approval gate satisfied).
    /// Stretch: VO-SEMAUDIT-05's claim is stated for test-semantic /
    /// impl-consistency audits; evaluate_vo_coverage applies the identical
    /// §8.5 recency rule to vo-coverage audits under its own approval-gating
    /// branch, so this covers the closest existing VO record for that
    /// shared rule (the registry is retiring; no new VO is minted). Fixed:
    /// this fold previously used combine_values (severity max), which picks
    /// UNKNOWN over PASS regardless of which record is newer.
    #[test]
    fn vo_coverage_selects_the_latest_valid_verdict_over_severity_max() {
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let vo_hash = write_approved_vo(&layout, "VO-ONE");
        write_vo_coverage_audit(
            &layout,
            "VO-ONE",
            &vo_hash,
            "UNKNOWN",
            "2026-08-08T00:00:00Z",
        );
        write_vo_coverage_audit(&layout, "VO-ONE", &vo_hash, "PASS", "2026-08-09T00:00:00Z");
        let selection = ScopeSelection {
            entity_scope: None,
            test_ids: BTreeSet::new(),
            vo_ids: BTreeSet::from(["VO-ONE".to_owned()]),
            req_ids: BTreeSet::new(),
            spec_ids: BTreeSet::new(),
        };
        assert_eq!(
            evaluate_vo_coverage(&layout.root, &layout, &scan, &selection).0,
            CheckValue::Pass
        );
    }

    /// @vtest.id TEST-VERIFY-002
    /// @vtest.covers VO-VERIFY-002
    /// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_static_audit
    /// @vtest.intent DA-002 UNKNOWN rescued to PASS by valid runtime target_execution (count>0)
    #[test]
    fn static_audit_da002_unknown_is_rescued_by_runtime_target_execution() {
        // DA-002 is statically UNKNOWN but DA-003 PASS (result asserted in body).
        // A valid Evidence whose per-target target_execution is PASS (count > 0)
        // proves reachability, so the item is PASS — a satisfied reachability
        // yields no UNKNOWN at computation time, not an UNKNOWN→PASS promotion
        // (詳細設計 §7.1 L963, §7.3, 別紙C §18.3.2 L104).
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let test = &scan.tests[0];
        write_static_audit_split(&layout, test, "UNKNOWN", "PASS", "2026-08-08T00:00:00Z");
        let evidence = valid_runtime_evidence(&layout, test, &scan, CheckValue::Pass, Some(3));
        assert_eq!(
            evaluate_static_audit(&layout.root, &layout, &evidence, &scan).0,
            CheckValue::Pass
        );
    }

    /// @vtest.id TEST-VERIFY-003
    /// @vtest.covers VO-VERIFY-002
    /// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_static_audit
    /// @vtest.intent Runtime rescue denied when measured count is zero; item stays UNKNOWN
    #[test]
    fn static_audit_runtime_rescue_needs_a_positive_measured_count() {
        // The same DA-002 UNKNOWN target is not rescued when coverage measured a
        // zero count (target_execution FAIL): the item stays UNKNOWN
        // (詳細設計 §7.3 L1031, 別紙C §18.3.2 L110).
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let test = &scan.tests[0];
        write_static_audit_split(&layout, test, "UNKNOWN", "PASS", "2026-08-08T00:00:00Z");
        let evidence = valid_runtime_evidence(&layout, test, &scan, CheckValue::Fail, Some(0));
        assert_eq!(
            evaluate_static_audit(&layout.root, &layout, &evidence, &scan).0,
            CheckValue::Unknown
        );
    }

    /// @vtest.id TEST-VERIFY-004
    /// @vtest.covers VO-VERIFY-002
    /// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_static_audit
    /// @vtest.intent Per-target DA-002 FAIL dominates a newer UNKNOWN record
    #[test]
    fn static_audit_da002_fail_dominates_a_newer_unknown_record() {
        // Two subject-current records for the same target: an older FAIL and a
        // newer UNKNOWN — a contradiction only hand-written records can hold.
        // FAIL dominates per §8.5 applied per target; selection does not pick
        // the latest to escape the FAIL (詳細設計 §7.3 L1016, 別紙C §18.3.2 L107).
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let test = &scan.tests[0];
        write_static_audit_split(&layout, test, "FAIL", "PASS", "2026-08-08T00:00:00Z");
        write_static_audit_split(&layout, test, "UNKNOWN", "PASS", "2026-08-09T00:00:00Z");
        assert_eq!(
            evaluate_static_audit(&layout.root, &layout, &BTreeMap::new(), &scan).0,
            CheckValue::Fail
        );
    }

    /// @vtest.id TEST-VERIFY-005
    /// @vtest.covers VO-VERIFY-002
    /// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_static_audit
    /// @vtest.intent DA-003 UNKNOWN gets no runtime rescue; item is UNKNOWN
    #[test]
    fn static_audit_da003_unknown_is_not_rescued_by_runtime() {
        // DA-002 is statically PASS (reachable) but DA-003 is UNKNOWN. Coverage
        // proves execution, not result verification, so the item is UNKNOWN with
        // no runtime rescue for DA-003 (詳細設計 §7.3 L1019, 別紙C §18.3.2 L111).
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let test = &scan.tests[0];
        write_static_audit_split(&layout, test, "PASS", "UNKNOWN", "2026-08-08T00:00:00Z");
        assert_eq!(
            evaluate_static_audit(&layout.root, &layout, &BTreeMap::new(), &scan).0,
            CheckValue::Unknown
        );
    }

    /// @vtest.id TEST-VERIFY-006
    /// @vtest.covers VO-VERIFY-002
    /// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_static_audit
    /// @vtest.intent Statically-unproven DA-002 with no Evidence stays UNKNOWN, not PASS
    #[test]
    fn static_audit_da002_unknown_without_runtime_stays_unknown() {
        // A statically-unproven DA-002 (UNKNOWN) is not reachable without runtime
        // proof; with no Evidence the item is UNKNOWN, never a vacuous PASS
        // (詳細設計 §7.3 L1025, 別紙C §18.3.2 L110).
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let test = &scan.tests[0];
        write_static_audit_split(&layout, test, "UNKNOWN", "PASS", "2026-08-08T00:00:00Z");
        assert_eq!(
            evaluate_static_audit(&layout.root, &layout, &BTreeMap::new(), &scan).0,
            CheckValue::Unknown
        );
    }

    /// @vtest.id TEST-VERIFY-007
    /// @vtest.covers VO-VERIFY-002
    /// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_static_audit
    /// @vtest.intent Per-target list naming undeclared target is malformed (E-SCAN-010,
    /// excluded); with no valid record remaining the item is STALE, not a
    /// distinct UNKNOWN (詳細設計 §3.6). Fixed: this pinned the pre-fix
    /// per-test `malformed -> Unknown` override; the correct value is STALE.
    #[test]
    fn static_audit_per_target_set_mismatch_is_malformed_stale() {
        // A subject-current record whose per-target list names a target the Test
        // does not declare is malformed (E-SCAN-010): excluded, its verdicts not
        // extracted. No valid record remains, so the item falls through to the
        // same "no valid record" rule as a stale-content record: STALE.
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let test = &scan.tests[0];
        write_static_audit_at(
            &layout,
            "PASS",
            static_audit_subjects(test, test.content_hash.clone()),
            Some("rust-cargo::src/lib.rs::not_declared"),
            "2026-08-08T00:00:00Z",
        );
        assert_eq!(
            evaluate_static_audit(&layout.root, &layout, &BTreeMap::new(), &scan).0,
            CheckValue::Stale
        );
    }

    /// @vtest.id TEST-VERIFY-018
    /// @vtest.covers VO-VERIFY-002
    /// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_static_audit
    /// @vtest.intent A malformed record does not blank out a valid sibling record's
    /// verdict for the same Test (詳細設計 §3.6: malformed is excluded, not an
    /// override of the fold over the remaining valid records).
    #[test]
    fn static_audit_valid_record_wins_over_a_malformed_sibling() {
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let test = &scan.tests[0];
        // A valid, current PASS record for TEST-ONE.
        write_static_audit(&layout, test, "PASS", test.content_hash.clone());
        // A second, malformed record for the SAME Test (per-target list names an
        // undeclared target) must not blank the valid PASS above to UNKNOWN.
        write_static_audit_at(
            &layout,
            "PASS",
            static_audit_subjects(test, test.content_hash.clone()),
            Some("rust-cargo::src/lib.rs::not_declared"),
            "2026-08-08T00:00:00Z",
        );
        assert_eq!(
            evaluate_static_audit(&layout.root, &layout, &BTreeMap::new(), &scan).0,
            CheckValue::Pass
        );
    }

    /// @vtest.id TEST-VERIFY-008
    /// @vtest.covers VO-VERIFY-002
    /// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_static_audit
    /// @vtest.intent Only current per-test records used; FAIL wins over PASS/UNKNOWN
    #[test]
    fn static_audit_uses_only_current_per_test_records_and_fail_wins() {
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let test = &scan.tests[0];
        write_static_audit(&layout, test, "UNKNOWN", test.content_hash.clone());
        write_static_audit(&layout, test, "PASS", test.content_hash.clone());
        assert_eq!(
            evaluate_static_audit(&layout.root, &layout, &BTreeMap::new(), &scan).0,
            CheckValue::Pass
        );

        write_static_audit(&layout, test, "FAIL", test.content_hash.clone());
        assert_eq!(
            evaluate_static_audit(&layout.root, &layout, &BTreeMap::new(), &scan).0,
            CheckValue::Fail
        );
    }

    /// @vtest.id TEST-VERIFY-009
    /// @vtest.covers VO-VERIFY-002
    /// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_static_audit
    /// @vtest.intent Assertion-macro config change stales the static audit record
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
        assert_eq!(
            evaluate_static_audit(&layout.root, &layout, &BTreeMap::new(), &scan).0,
            CheckValue::Stale
        );
    }

    /// @vtest.id TEST-VERIFY-010
    /// @vtest.covers VO-VERIFY-002
    /// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_static_audit
    /// @vtest.intent Moving the Test locator stales the static audit record
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
        assert_eq!(
            evaluate_static_audit(&layout.root, &layout, &BTreeMap::new(), &scan).0,
            CheckValue::Stale
        );
    }

    /// @vtest.id TEST-VERIFY-011
    /// @vtest.covers VO-VERIFY-002
    /// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_static_audit
    /// @vtest.intent A subject hash mismatch marks the static audit stale
    #[test]
    fn static_audit_marks_hash_mismatch_stale() {
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        write_static_audit(
            &layout,
            &scan.tests[0],
            "PASS",
            ContentHash::from_text("historic test body"),
        );
        assert_eq!(
            evaluate_static_audit(&layout.root, &layout, &BTreeMap::new(), &scan).0,
            CheckValue::Stale
        );
    }

    /// @vtest.id TEST-VERIFY-012
    /// @vtest.covers VO-VERIFY-002
    /// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_static_audit
    /// @vtest.intent Any non-current bound subject makes the static audit stale
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
        assert_eq!(
            evaluate_static_audit(&layout.root, &layout, &BTreeMap::new(), &scan).0,
            CheckValue::Stale
        );
    }

    /// @vtest.id TEST-VERIFY-013
    /// @vtest.covers VO-VERIFY-002
    /// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_static_audit
    /// @vtest.intent Missing/substituted declared-target subject makes audit stale
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
        assert_eq!(
            evaluate_static_audit(&layout.root, &layout, &BTreeMap::new(), &scan).0,
            CheckValue::Stale
        );

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
        assert_eq!(
            evaluate_static_audit(&layout.root, &layout, &BTreeMap::new(), &scan).0,
            CheckValue::Stale
        );
    }

    /// @vtest.id TEST-VERIFY-014
    /// @vtest.covers VO-VERIFY-002
    /// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_static_audit
    /// @vtest.intent A record binding multiple Tests fails to parse (rejected by
    /// `AuditRecord::validate`) and is treated as malformed for both bound
    /// Tests (E-SCAN-010); with no valid record for either, both are STALE.
    /// Fixed: this pinned the pre-fix per-test `malformed -> Unknown`
    /// override; the correct value is STALE, not a distinct UNKNOWN.
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
        assert_eq!(
            evaluate_static_audit(&layout.root, &layout, &BTreeMap::new(), &scan).0,
            CheckValue::Stale
        );
    }

    /// @vtest.id TEST-VERIFY-015
    /// @vtest.covers VO-VERIFY-002
    /// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_static_audit
    /// @vtest.intent Latest record chosen by actual RFC3339 instant across offsets
    #[test]
    fn static_audit_orders_offsets_by_the_actual_rfc3339_instant() {
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let test = &scan.tests[0];
        let canonical = test.targets[0].normalized();
        // 09:00+09:00 is 00:00Z.  It is lexically later than 00:30Z but
        // chronologically earlier, so the later PASS must be selected.
        write_static_audit_at(
            &layout,
            "UNKNOWN",
            static_audit_subjects(test, test.content_hash.clone()),
            Some(&canonical),
            "2026-08-08T09:00:00+09:00",
        );
        write_static_audit_at(
            &layout,
            "PASS",
            static_audit_subjects(test, test.content_hash.clone()),
            Some(&canonical),
            "2026-08-08T00:30:00Z",
        );
        assert_eq!(
            evaluate_static_audit(&layout.root, &layout, &BTreeMap::new(), &scan).0,
            CheckValue::Pass
        );
    }

    /// @vtest.id TEST-VERIFY-016
    /// @vtest.covers VO-VERIFY-002
    /// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_static_audit
    /// @vtest.intent A registered Test with no static audit yields NotChecked
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
            evaluate_static_audit(&layout.root, &layout, &BTreeMap::new(), &scan).0,
            CheckValue::NotChecked
        );
    }

    /// @vtest.id TEST-VERIFY-017
    /// @vtest.covers VO-VERIFY-002
    /// @vtest.target crates/vtest-verify/src/lib.rs::evaluate_static_audit
    /// @vtest.intent A malformed static record surfaces as STALE, not silently
    /// ignored as NOT_CHECKED (詳細設計 §3.6: an audit attempt was made for
    /// this Test, so the "never audited" NOT_CHECKED value does not apply).
    /// Fixed: this pinned the pre-fix per-test `malformed -> Unknown`
    /// override; the correct value is STALE.
    #[test]
    fn malformed_static_audit_surfaces_as_stale_not_silently_ignored() {
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let id = new_record_id();
        fs::write(
            layout.audits_dir().join(format!("{id}.yaml")),
            format!(
                "id: {id}\nkind: static\nbundle_id: null\nsubjects:\n  - id: TEST-ONE\n    hash: invalid\nverdict: PASS\n"
            ),
        )
        .expect("write malformed static audit");
        let result = evaluate_static_audit(&layout.root, &layout, &BTreeMap::new(), &scan);
        assert_eq!(result.0, CheckValue::Stale, "basis: {:?}", result.1);
    }

    /// Fixed VO-EXEC-06: an invalid Evidence's freshness verdict must be
    /// propagated verbatim into runtime_result / target_execution rather than
    /// flattened to STALE, so an incomplete Execution State snapshot reads as
    /// UNKNOWN and a foreign-adapter Evidence reads as MISMATCH -- the same
    /// distinctions evaluate_test_execution already preserves.
    #[test]
    fn runtime_result_propagates_unknown_freshness_instead_of_flattening_to_stale() {
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let test = &scan.tests[0];
        let mut evidence = valid_runtime_evidence(&layout, test, &scan, CheckValue::Pass, Some(1));
        evidence
            .get_mut(test.id.as_str())
            .expect("evidence for test")
            .execution_state
            .as_mut()
            .expect("execution state")
            .complete = false;
        assert_eq!(
            evaluate_runtime(&layout.root, &evidence, &scan).0,
            CheckValue::Unknown
        );
    }

    #[test]
    fn target_execution_propagates_unknown_freshness_instead_of_flattening_to_stale() {
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let test = &scan.tests[0];
        let mut evidence = valid_runtime_evidence(&layout, test, &scan, CheckValue::Pass, Some(1));
        evidence
            .get_mut(test.id.as_str())
            .expect("evidence for test")
            .execution_state
            .as_mut()
            .expect("execution state")
            .complete = false;
        assert_eq!(
            evaluate_target_execution(&layout.root, &evidence, &scan).0,
            CheckValue::Unknown
        );
    }

    #[test]
    fn runtime_result_propagates_mismatch_freshness_instead_of_flattening_to_stale() {
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let test = &scan.tests[0];
        let mut evidence = valid_runtime_evidence(&layout, test, &scan, CheckValue::Pass, Some(1));
        evidence
            .get_mut(test.id.as_str())
            .expect("evidence for test")
            .adapter = Some(AdapterId::new("python-pytest"));
        assert_eq!(
            evaluate_runtime(&layout.root, &evidence, &scan).0,
            CheckValue::Mismatch
        );
    }

    #[test]
    fn target_execution_propagates_mismatch_freshness_instead_of_flattening_to_stale() {
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let test = &scan.tests[0];
        let mut evidence = valid_runtime_evidence(&layout, test, &scan, CheckValue::Pass, Some(1));
        evidence
            .get_mut(test.id.as_str())
            .expect("evidence for test")
            .adapter = Some(AdapterId::new("python-pytest"));
        assert_eq!(
            evaluate_target_execution(&layout.root, &evidence, &scan).0,
            CheckValue::Mismatch
        );
    }

    /// Fixed VO-AGG-03: an unassociated malformed static record must fold
    /// through combine_values (§4.3) as an Unknown-flavoured ground, so a
    /// coexisting higher-priority child value (NOT_CHECKED, STALE, ...) is
    /// never demoted to UNKNOWN by the ad-hoc override this replaces.
    #[test]
    fn static_audit_unassociated_malformed_record_does_not_downgrade_a_notchecked_aggregate() {
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let id = new_record_id();
        fs::write(
            layout.audits_dir().join(format!("{id}.yaml")),
            format!("id: {id}\nkind: static\nthis_is: garbage\n"),
        )
        .expect("write unassociated malformed static audit");
        let result = evaluate_static_audit(&layout.root, &layout, &BTreeMap::new(), &scan);
        assert_eq!(result.0, CheckValue::NotChecked, "basis: {:?}", result.1);
    }

    #[test]
    fn static_audit_unassociated_malformed_record_does_not_downgrade_a_stale_aggregate() {
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let test = &scan.tests[0];
        // Same shape as static_audit_requires_the_exact_declared_target_subject:
        // one subject only, so the record is excluded as STALE.
        write_static_audit_with_subjects(
            &layout,
            "PASS",
            vec![AuditSubjectRecord {
                id: Some(test.id.as_str().to_owned()),
                locator: None,
                hash: test.content_hash.clone(),
            }],
        );
        let id = new_record_id();
        fs::write(
            layout.audits_dir().join(format!("{id}.yaml")),
            format!("id: {id}\nkind: static\nthis_is: garbage\n"),
        )
        .expect("write unassociated malformed static audit");
        let result = evaluate_static_audit(&layout.root, &layout, &BTreeMap::new(), &scan);
        assert_eq!(result.0, CheckValue::Stale, "basis: {:?}", result.1);
    }

    #[test]
    fn static_audit_unassociated_malformed_record_does_not_outrank_a_fail_aggregate() {
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let test = &scan.tests[0];
        write_static_audit_split(&layout, test, "FAIL", "PASS", "2026-08-08T00:00:00Z");
        let id = new_record_id();
        fs::write(
            layout.audits_dir().join(format!("{id}.yaml")),
            format!("id: {id}\nkind: static\nthis_is: garbage\n"),
        )
        .expect("write unassociated malformed static audit");
        let result = evaluate_static_audit(&layout.root, &layout, &BTreeMap::new(), &scan);
        assert_eq!(result.0, CheckValue::Fail, "basis: {:?}", result.1);
    }

    /// Write a `test-semantic` or `impl-consistency` audit record binding
    /// the complete required subject set (§8.4/§8.5: Test + covered VO(s) +
    /// all declared target(s), all at their CURRENT hash) so the record is
    /// valid under the set-equality validity rule, isolating
    /// evaluate_test_audit's per-test fold from subject-completeness
    /// concerns rather than from subject presence entirely.
    fn write_test_audit_at(
        layout: &VerifyLayout,
        scan: &ScanResult,
        kind: &str,
        test: &TestEntity,
        verdict: &str,
        audited_at: &str,
    ) -> String {
        let mut subjects = vec![AuditSubjectRecord {
            id: Some(test.id.as_str().to_owned()),
            locator: None,
            hash: test.content_hash.clone(),
        }];
        for vo_id in &test.covers {
            let text = fs::read_to_string(layout.vo_dir().join(format!("{vo_id}.yaml")))
                .expect("fixture VO record for the covered VO");
            subjects.push(AuditSubjectRecord {
                id: Some(vo_id.to_string()),
                locator: None,
                hash: ContentHash::from_text(&text),
            });
        }
        for target in &test.targets {
            let source =
                find_target_source(scan, target).expect("fixture declared target resolves");
            subjects.push(AuditSubjectRecord {
                id: None,
                locator: Some(source.target.normalized()),
                hash: source.content_hash.clone(),
            });
        }
        let id = new_record_id();
        let record = AuditRecord {
            id: id.clone(),
            kind: kind.to_owned(),
            bundle_id: None,
            subjects,
            verdict: verdict.to_owned(),
            reasons: vec![AuditReasonRecord {
                rule: None,
                verdict: None,
                claim: format!("fixture result for {kind}"),
                basis: vec![AuditBasisRecord {
                    kind: "test-code".to_owned(),
                    reference: "tests/semantic.rs:1".to_owned(),
                }],
                targets: Vec::new(),
            }],
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
            record.to_yaml().expect("serialise test audit"),
        )
        .expect("write test audit");
        id
    }

    /// Fixed VO-SEMAUDIT-05: among valid records with no FAIL, evaluate_test_audit
    /// must select the chronologically LATEST verdict (§8.5), not the
    /// severity-max fold that lets an older UNKNOWN permanently outrank a
    /// newer PASS.
    #[test]
    fn semantic_audit_selects_the_latest_valid_verdict_when_no_fail_exists() {
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let test = &scan.tests[0];
        write_test_audit_at(&layout, &scan, "test-semantic", test, "UNKNOWN", "2026-08-08T00:00:00Z");
        write_test_audit_at(&layout, &scan, "test-semantic", test, "PASS", "2026-08-09T00:00:00Z");
        assert_eq!(
            evaluate_test_audit(&layout.root, &layout, &scan, "test-semantic").0,
            CheckValue::Pass
        );
    }

    #[test]
    fn semantic_audit_fail_dominates_regardless_of_recency() {
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let test = &scan.tests[0];
        write_test_audit_at(&layout, &scan, "test-semantic", test, "FAIL", "2026-08-08T00:00:00Z");
        write_test_audit_at(&layout, &scan, "test-semantic", test, "PASS", "2026-08-09T00:00:00Z");
        assert_eq!(
            evaluate_test_audit(&layout.root, &layout, &scan, "test-semantic").0,
            CheckValue::Fail
        );
    }

    #[test]
    fn impl_consistency_selects_the_latest_valid_verdict_when_no_mismatch_exists() {
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let test = &scan.tests[0];
        write_test_audit_at(&layout, &scan, "impl-consistency", test, "UNKNOWN", "2026-08-08T00:00:00Z");
        write_test_audit_at(&layout, &scan, "impl-consistency", test, "PASS", "2026-08-09T00:00:00Z");
        assert_eq!(
            evaluate_test_audit(&layout.root, &layout, &scan, "impl-consistency").0,
            CheckValue::Pass
        );
    }

    #[test]
    fn impl_consistency_fail_verdict_maps_to_mismatch_and_dominates() {
        let (layout, scan) = static_fixture(&["TEST-ONE"]);
        let test = &scan.tests[0];
        write_test_audit_at(&layout, &scan, "impl-consistency", test, "FAIL", "2026-08-08T00:00:00Z");
        write_test_audit_at(&layout, &scan, "impl-consistency", test, "PASS", "2026-08-09T00:00:00Z");
        assert_eq!(
            evaluate_test_audit(&layout.root, &layout, &scan, "impl-consistency").0,
            CheckValue::Mismatch
        );
    }
}
