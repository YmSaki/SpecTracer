//! Judgment-record protocol: `vtest audit bundle` / `vtest audit submit`
//! and MCP `audit_bundle` / `audit_submit` (本冊 §8, SPEC-390/SPEC-391,
//! DES-428〜DES-432, DS-692〜DS-739, DS-1084〜DS-1090, DS-1210〜DS-1212).
//!
//! - `bundle` hands a subject the machine cannot decide to an external
//!   judge: it writes everything needed for the judgment, plus the content
//!   hashes of the subject and its upstream dependency closure at bundle
//!   time, to `.verify/cache/bundles/<ULID>.json` (derived, rebuildable).
//! - `submit` checks a judge's result against that bundle (DS-715〜DS-720,
//!   in DS-1089's order) and, when accepted, appends a judgment record to
//!   `.verify/decisions/<ULID>.yaml` bound to those hashes (DS-721).
//!
//! A judgment never changes a verification state (DS-693/DS-708/DS-1212),
//! and its `reason` / `exclusions` are optional: an empty reason is never a
//! ground for rejection (DS-722/DS-723).
//!
//! Disclosed limit: `--include-failed` (SPEC-391's usage line) is not
//! implemented, because the canon gives it no semantics beyond its name.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::{json, Value};
use vtest_adapter_api::AdapterRegistry;
use vtest_model::{ContentHash, SectionNode, TargetRef, TestEntity, VoRecord};
use vtest_scan::ScanResult;
use vtest_store::{
    approval::{build_document_node_index, read_all_vos, DocumentNodeIndex},
    decisions::{
        read_all_decisions, write_decision, Actor, DecisionDependency, DecisionRecord, ReasonEntry,
        BASIS_KINDS, DECISION_VALUES, JUDGMENT_KINDS,
    },
    doc_registry::read_all_docs,
    StoreError, VerifyLayout,
};

#[derive(Debug, thiserror::Error)]
pub enum JudgmentOpError {
    /// Usage error (exit 2): selector misuse, an unknown subject, or a
    /// `judgment_kind` / subject-type combination outside DS-694〜DS-700 /
    /// DS-1085.
    #[error("{0}")]
    Usage(String),
    /// DS-702〜DS-704: a declared target or upstream document cannot be
    /// resolved, so no bundle is generated. `code` is E-SCAN-004 /
    /// E-SCAN-011 / E-SCAN-012.
    #[error("{message}")]
    Unresolved { code: &'static str, message: String },
    /// DS-715〜DS-720: a rejected submission. `code` is E-AUDIT-001/002/003/
    /// 004/008.
    #[error("{message}")]
    Rejected { code: &'static str, message: String },
    #[error("store error: {0}")]
    Store(#[from] StoreError),
    /// Writing the bundle cache failed: an internal error.
    #[error("cannot write {path}: {source}")]
    Cache {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

impl JudgmentOpError {
    /// The diagnostic code for an operation rejection (exit 2). `None`
    /// means an internal error.
    pub fn code(&self) -> Option<&'static str> {
        match self {
            Self::Usage(_) => Some("E-OP-001"),
            Self::Unresolved { code, .. } | Self::Rejected { code, .. } => Some(code),
            Self::Store(error) => error.record_read_code(),
            Self::Cache { .. } => None,
        }
    }
}

fn rejected(code: &'static str, message: impl Into<String>) -> JudgmentOpError {
    JudgmentOpError::Rejected {
        code,
        message: message.into(),
    }
}

/// The subject a judgment is about.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Subject {
    Test(String),
    Vo(String),
}

impl Subject {
    fn id(&self) -> &str {
        match self {
            Self::Test(id) | Self::Vo(id) => id,
        }
    }
}

/// DS-1084/DS-1085/DS-694〜DS-700: resolves the judgment kind for a subject.
pub fn resolve_kind(subject: &Subject, kind: Option<&str>) -> Result<String, JudgmentOpError> {
    match (subject, kind) {
        (Subject::Test(_), None) => Ok("test-semantic".to_owned()),
        (_, Some(kind)) if !JUDGMENT_KINDS.contains(&kind) => Err(JudgmentOpError::Usage(format!(
            "--kind must be one of {}; got `{kind}`",
            JUDGMENT_KINDS.join(", ")
        ))),
        (Subject::Test(_), Some(kind)) => Ok(kind.to_owned()),
        (Subject::Vo(_), Some("case-coverage")) => Ok("case-coverage".to_owned()),
        (Subject::Vo(_), _) => Err(JudgmentOpError::Usage(
            "--vo requires --kind case-coverage (DS-1085)".to_owned(),
        )),
    }
}

/// The subject hash and upstream dependency closure a judgment binds to
/// (DS-432/DS-435/DS-721/DS-724).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Binding {
    pub subject_hash: ContentHash,
    pub dependencies: Vec<DecisionDependency>,
}

fn document_closure(
    index: &DocumentNodeIndex,
    start: &str,
    out: &mut BTreeMap<(String, String), ContentHash>,
) -> Result<(), JudgmentOpError> {
    let mut stack = vec![start.to_owned()];
    let mut seen = BTreeSet::new();
    while let Some(node) = stack.pop() {
        if !seen.insert(node.clone()) {
            continue;
        }
        let Some((hash, parents)) = index.get(&node) else {
            return Err(JudgmentOpError::Unresolved {
                code: "E-SCAN-012",
                message: format!("upstream document node {node} does not exist (DS-702/DS-703)"),
            });
        };
        out.insert(("document".to_owned(), node), hash.clone());
        stack.extend(parents.iter().cloned());
    }
    Ok(())
}

/// A VO, its parent chain, and every upstream document node.
fn vo_closure(
    vos: &BTreeMap<String, VoRecord>,
    index: &DocumentNodeIndex,
    id: &str,
    out: &mut BTreeMap<(String, String), ContentHash>,
) -> Result<(), JudgmentOpError> {
    let mut cursor = Some(id.to_owned());
    let mut seen = BTreeSet::new();
    while let Some(current) = cursor {
        if !seen.insert(current.clone()) {
            break;
        }
        let Some(vo) = vos.get(&current) else {
            return Err(JudgmentOpError::Unresolved {
                code: "E-SCAN-003",
                message: format!("VO {current} does not exist (DS-702)"),
            });
        };
        out.insert(
            ("vo".to_owned(), current.clone()),
            vtest_model::vo_subject_hash(vo),
        );
        for entry in &vo.derives_from {
            document_closure(index, entry.doc.as_str(), out)?;
        }
        cursor = vo.parent.as_ref().map(|parent| parent.as_str().to_owned());
    }
    Ok(())
}

fn target_label(target: &TargetRef) -> String {
    match target {
        TargetRef::Locator(locator) => locator.value.clone(),
        TargetRef::SrcId(src_id) => src_id.as_str().to_owned(),
    }
}

/// DS-702〜DS-704: every declared target must resolve to exactly one
/// source construct.
fn resolve_target<'a>(
    scan: &'a ScanResult,
    target: &TargetRef,
) -> Result<&'a vtest_model::SourceFunction, JudgmentOpError> {
    let matches: Vec<_> = scan
        .sources
        .iter()
        .filter(|source| match target {
            TargetRef::Locator(locator) => &source.locator == locator,
            TargetRef::SrcId(src_id) => source.src_id.as_ref() == Some(src_id),
        })
        .collect();
    match matches.as_slice() {
        [one] => Ok(one),
        [] => Err(JudgmentOpError::Unresolved {
            code: "E-SCAN-004",
            message: format!(
                "declared target {} does not exist (DS-702/DS-703)",
                target_label(target)
            ),
        }),
        _ => Err(JudgmentOpError::Unresolved {
            code: "E-SCAN-011",
            message: format!(
                "declared target {} is ambiguous (DS-702/DS-704)",
                target_label(target)
            ),
        }),
    }
}

/// DS-536: a judgment subject must be one Test. A Test id declared more
/// than once (doctor's E-SCAN-002) names no single subject, so the
/// operation is refused rather than picking one of the candidates.
fn find_test<'a>(scan: &'a ScanResult, id: &str) -> Result<&'a TestEntity, JudgmentOpError> {
    let matches: Vec<&TestEntity> = scan
        .tests
        .iter()
        .filter(|test| test.id.as_str() == id)
        .collect();
    match matches.as_slice() {
        [one] => Ok(one),
        [] => Err(JudgmentOpError::Usage(format!(
            "no Test with id '{id}' exists"
        ))),
        many => Err(duplicate_test_id(id, many)),
    }
}

fn duplicate_test_id(id: &str, tests: &[&TestEntity]) -> JudgmentOpError {
    let paths: Vec<&str> = tests
        .iter()
        .map(|test| test.location.path.as_str())
        .collect();
    JudgmentOpError::Unresolved {
        code: "E-SCAN-002",
        message: format!(
            "Test id {id} is declared more than once ({}), so it names no single \
             judgment material (DS-536)",
            paths.join(", ")
        ),
    }
}

/// Every Test is listed once; a repeated id among the Tests handed to the
/// judge would collapse in the binding, so it is refused (E-SCAN-002).
fn unique_ids(tests: Vec<&TestEntity>) -> Result<Vec<&TestEntity>, JudgmentOpError> {
    let mut by_id: BTreeMap<&str, Vec<&TestEntity>> = BTreeMap::new();
    for test in &tests {
        by_id.entry(test.id.as_str()).or_default().push(test);
    }
    if let Some((id, many)) = by_id.into_iter().find(|(_, group)| group.len() > 1) {
        return Err(duplicate_test_id(id, &many));
    }
    Ok(tests)
}

/// The other Tests a Test-subject bundle hands to the judge as
/// `related_tests`: those covering one of its VOs, or named in its
/// `related`. The binding and the bundle both use this one set.
fn related_tests<'a>(
    scan: &'a ScanResult,
    test: &TestEntity,
) -> Result<Vec<&'a TestEntity>, JudgmentOpError> {
    unique_ids(
        scan.tests
            .iter()
            .filter(|other| other.id != test.id)
            .filter(|other| {
                other.covers.iter().any(|vo| test.covers.contains(vo))
                    || test.related.contains(&other.id)
            })
            .collect(),
    )
}

/// The Tests a VO-subject bundle hands to the judge: every Test covering it.
fn covering_tests<'a>(
    scan: &'a ScanResult,
    vo: &str,
) -> Result<Vec<&'a TestEntity>, JudgmentOpError> {
    unique_ids(
        scan.tests
            .iter()
            .filter(|test| test.covers.iter().any(|covered| covered.as_str() == vo))
            .collect(),
    )
}

/// Resolves every declared target of `test` and binds each one's content
/// hash (DS-702: an unresolved target gives no bundle; DS-718: a changed or
/// deleted target invalidates it).
fn bind_targets(
    scan: &ScanResult,
    test: &TestEntity,
    deps: &mut BTreeMap<(String, String), ContentHash>,
) -> Result<(), JudgmentOpError> {
    for target in &test.targets {
        let source = resolve_target(scan, target)?;
        deps.insert(
            ("target".to_owned(), target_label(target)),
            source.content_hash.clone(),
        );
    }
    Ok(())
}

/// Computes the current binding for a subject. Used both when bundling and,
/// again, when a submission arrives, so DS-718's comparison is between the
/// same computation at two times.
pub fn current_binding(
    layout: &VerifyLayout,
    scan: &ScanResult,
    subject: &Subject,
) -> Result<Binding, JudgmentOpError> {
    let vos = read_all_vos(layout)?;
    let index = build_document_node_index(layout)?;
    let mut deps = BTreeMap::new();
    let subject_hash = match subject {
        Subject::Test(id) => {
            let test = find_test(scan, id)?;
            for vo in &test.covers {
                vo_closure(&vos, &index, vo.as_str(), &mut deps)?;
            }
            bind_targets(scan, test, &mut deps)?;
            // `related_tests` is judgment material too: adding, removing or
            // changing one invalidates the bundle (DS-718/DS-435).
            for other in related_tests(scan, test)? {
                deps.insert(
                    ("test".to_owned(), other.id.as_str().to_owned()),
                    test_material_hash(other),
                );
            }
            test.content_hash.clone()
        }
        Subject::Vo(id) => {
            let Some(vo) = vos.get(id) else {
                return Err(JudgmentOpError::Usage(format!(
                    "no VO with id '{id}' exists"
                )));
            };
            vo_closure(&vos, &index, id, &mut deps)?;
            deps.remove(&("vo".to_owned(), id.clone()));
            // The bundle hands every covering Test to the judge as judgment
            // material (`related_tests`), so the binding carries that set
            // and each member's material hash: adding, removing or changing
            // a covering Test invalidates the bundle (DS-718). Each covering
            // Test's declared targets must resolve (DS-702) and are bound
            // like a Test subject's.
            for test in covering_tests(scan, id)? {
                deps.insert(
                    ("test".to_owned(), test.id.as_str().to_owned()),
                    test_material_hash(test),
                );
                bind_targets(scan, test, &mut deps)?;
            }
            vtest_model::vo_subject_hash(vo)
        }
    };
    Ok(Binding {
        subject_hash,
        dependencies: deps
            .into_iter()
            .map(|((kind, id), hash)| DecisionDependency { kind, id, hash })
            .collect(),
    })
}

/// The hash of a covering Test's judgment material: its construct hash
/// (code) together with the normalized fields the bundle shows the judge.
fn test_material_hash(test: &TestEntity) -> ContentHash {
    let material = json!({
        "content_hash": test.content_hash,
        "intent": test.intent,
        "input": test.input,
        "expect": test.expect,
        "kind": test.kind,
        "cases": test.cases,
        "covers": test.covers.iter().map(|vo| vo.as_str()).collect::<Vec<_>>(),
        "targets": test.targets.iter().map(target_label).collect::<Vec<_>>(),
    });
    ContentHash::from_text(&material.to_string())
}

/// Every document node's full content, keyed by node id. A section node is
/// shown without its nested `sections` / `items` (each nested node is its
/// own entry), so a closure lists exactly the nodes it depends on.
fn document_node_texts(
    layout: &VerifyLayout,
) -> Result<BTreeMap<String, (String, Value)>, JudgmentOpError> {
    fn section(doc: &str, node: &SectionNode, out: &mut BTreeMap<String, (String, Value)>) {
        out.insert(
            node.id.as_str().to_owned(),
            (
                doc.to_owned(),
                json!({
                    "title": node.title,
                    "description": node.description,
                    "derives_from": node.derives_from,
                    "source": node.source,
                }),
            ),
        );
        for item in node.items.as_deref().unwrap_or(&[]) {
            out.insert(item.id.as_str().to_owned(), (doc.to_owned(), json!(item)));
        }
        for child in node.sections.as_deref().unwrap_or(&[]) {
            section(doc, child, out);
        }
    }
    let mut out = BTreeMap::new();
    for (doc, view) in read_all_docs(layout)? {
        let file = &view.file;
        for node in &file.root {
            out.insert(node.id.as_str().to_owned(), (doc.clone(), json!(node)));
        }
        for node in &file.request {
            out.insert(node.id.as_str().to_owned(), (doc.clone(), json!(node)));
        }
        for node in file
            .require
            .iter()
            .chain(&file.spec)
            .chain(&file.detailed_spec)
            .chain(&file.basic_design)
            .chain(&file.design)
        {
            section(&doc, node, &mut out);
        }
    }
    Ok(out)
}

fn binding_json(binding: &Binding) -> (Value, Value) {
    (json!(binding.subject_hash), json!(binding.dependencies))
}

fn source_text(root: &Path, location: &vtest_model::SourceLocation) -> Option<String> {
    vtest_verify::read_construct_text(root, location)
}

/// `vtest audit bundle (--test TEST-X | --vo VO-X) [--kind ...]` / MCP
/// `audit_bundle` (SPEC-391, DES-428, DS-1084〜DS-1086, DS-1210). Returns
/// the bundle body and writes it to `.verify/cache/bundles/<bundle_id>.json`.
pub fn bundle(
    root: &Path,
    scan: &ScanResult,
    registry: &AdapterRegistry,
    subject: &Subject,
    kind: Option<&str>,
) -> Result<Value, JudgmentOpError> {
    let layout = VerifyLayout::new(root);
    let judgment_kind = resolve_kind(subject, kind)?;
    let binding = current_binding(&layout, scan, subject)?;
    let vos = read_all_vos(&layout)?;

    let vo_json = |vo: &VoRecord| {
        json!({
            "id": vo.id.as_str(),
            "claim": vo.claim,
            "parent": vo.parent.as_ref().map(|parent| parent.as_str()),
            "derives_from": vo.derives_from,
            "dimensions": vo.dimensions,
            "coverage_policy": vo.coverage_policy,
            "combinations": vo.combinations,
            "representative_cases": vo.representative_cases,
        })
    };
    let test_json = |test: &TestEntity| {
        json!({
            "id": test.id.as_str(),
            "intent": test.intent,
            "input": test.input,
            "expect": test.expect,
            "kind": test.kind,
            "cases": test.cases,
            "covers": test.covers.iter().map(|vo| vo.as_str()).collect::<Vec<_>>(),
            "targets": test.targets.iter().map(target_label).collect::<Vec<_>>(),
            "path": test.location.path.as_str(),
            "code": source_text(root, &test.location),
            "content_hash": test.content_hash,
        })
    };

    let (test_value, covered, related_tests, targets, static_analysis) = match subject {
        Subject::Test(id) => {
            let test = find_test(scan, id)?;
            let covered: Vec<String> = test
                .covers
                .iter()
                .map(|vo| vo.as_str().to_owned())
                .collect();
            let related: Vec<Value> = related_tests(scan, test)?
                .into_iter()
                .map(|other| json!({"id": other.id.as_str(), "intent": other.intent, "cases": other.cases}))
                .collect();
            let mut targets = Vec::new();
            for target in &test.targets {
                let source = resolve_target(scan, target)?;
                let mut entry = json!({
                    "ref": target_label(target),
                    "path": source.location.path.as_str(),
                    "code": source_text(root, &source.location),
                    "content_hash": source.content_hash,
                });
                // DES-426: impl-consistency carries the source's full text.
                if judgment_kind == "impl-consistency" {
                    entry["source_text"] = json!(std::fs::read_to_string(
                        root.join(source.location.path.as_str())
                    )
                    .ok());
                }
                targets.push(entry);
            }
            let static_analysis = match registry.static_analysis(test.execution.adapter.as_str()) {
                Some(analyzer) => match source_text(root, &test.location) {
                    Some(text) => {
                        let rules = analyzer.audit_rules(vtest_adapter_api::StaticAnalysisInput {
                            test,
                            construct_text: &text,
                            content_hash: &test.content_hash,
                            extra_assertion_macros: &[],
                        });
                        json!({"rules": rules.iter().map(|rule| json!({
                            "rule": rule.rule,
                            "target": rule.target,
                            "verdict": rule.outcome.as_str(),
                            "reason": rule.reason,
                        })).collect::<Vec<_>>()})
                    }
                    None => json!({"not_analyzed": "test source could not be read"}),
                },
                None => json!({"not_analyzed": "adapter has no Static Analysis capability"}),
            };
            (test_json(test), covered, related, targets, static_analysis)
        }
        Subject::Vo(id) => {
            let covering: Vec<Value> = covering_tests(scan, id)?
                .into_iter()
                .map(test_json)
                .collect();
            (
                Value::Null,
                vec![id.clone()],
                covering,
                Vec::new(),
                Value::Null,
            )
        }
    };
    // DES-427: case-coverage carries every covered VO's dimensions /
    // policy / representative cases; the VO entries carry them for every
    // kind.
    let vo_values: Vec<Value> = covered
        .iter()
        .filter_map(|id| vos.get(id))
        .map(vo_json)
        .collect();
    let prior: Vec<DecisionRecord> = read_all_decisions(&layout)?
        .into_iter()
        .filter(|record| record.subject == subject.id() && record.judgment_kind == judgment_kind)
        .collect();

    // DES-426: impl-consistency carries the complete upstream document
    // subject set (the same closure the binding holds, §3.5's rule) with
    // each node's full content.
    let documents: Value = if judgment_kind == "impl-consistency" {
        let texts = document_node_texts(&layout)?;
        Value::Array(
            binding
                .dependencies
                .iter()
                .filter(|dep| dep.kind == "document")
                .map(|dep| {
                    let (doc, node) = texts
                        .get(&dep.id)
                        .cloned()
                        .unwrap_or((String::new(), Value::Null));
                    json!({"id": dep.id, "doc": doc, "content_hash": dep.hash, "node": node})
                })
                .collect(),
        )
    } else {
        Value::Array(Vec::new())
    };

    let bundle_id = vtest_store::new_record_id();
    let (subject_hash, dependencies) = binding_json(&binding);
    let body = json!({
        "bundle_id": bundle_id,
        "generated_at": vtest_store::records::now_rfc3339(),
        "revision": vtest_exec::git_revision(root),
        "subject": subject.id(),
        "subject_type": match subject { Subject::Test(_) => "test", Subject::Vo(_) => "vo" },
        "judgment_kind": judgment_kind,
        "test": test_value,
        "vos": vo_values,
        "targets": targets,
        "related_tests": related_tests,
        "documents": documents,
        "static_analysis": static_analysis,
        "prior_decisions": prior,
        "subject_hash": subject_hash,
        "dependencies": dependencies,
    });

    let dir = layout.cache_dir().join("bundles");
    let path = dir.join(format!("{bundle_id}.json"));
    let text = serde_json::to_string_pretty(&body).unwrap_or_default();
    std::fs::create_dir_all(&dir)
        .and_then(|()| std::fs::write(&path, text))
        .map_err(|source| JudgmentOpError::Cache {
            path: path.display().to_string(),
            source,
        })?;
    Ok(body)
}

/// The submission (DES-430): `bundle_id`, `subject`, `judgment_kind`,
/// `decision`, `actor` are required; `supersedes`, `reason`, `exclusions`
/// are optional.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Submission {
    // Read from the raw value first (E-AUDIT-001 precedes schema checks);
    // kept here so the schema still requires it.
    #[allow(dead_code)]
    bundle_id: String,
    subject: String,
    judgment_kind: String,
    decision: String,
    actor: Actor,
    #[serde(default)]
    supersedes: Vec<String>,
    #[serde(default)]
    reason: Vec<ReasonEntry>,
    #[serde(default)]
    exclusions: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct SubmitResult {
    pub accepted: bool,
    /// The new judgment record's id (`.verify/decisions/<id>.yaml`).
    pub decision_id: String,
    pub subject: String,
    pub judgment_kind: String,
    pub decision: String,
}

fn safe_bundle_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric())
}

/// `vtest audit submit --file result.json` / MCP `audit_submit`
/// (DS-715〜DS-723, DS-1089, DS-1211). Checks, in DS-1089's order: the
/// bundle exists (E-AUDIT-001); the subject (E-AUDIT-003) and
/// judgment_kind (E-AUDIT-003) match it; the hashes recorded in the bundle
/// still match the current ones (E-AUDIT-002); the decision is accepted
/// (E-AUDIT-004); every supersedes id names an existing record with the
/// same subject and judgment kind and is not self-referential
/// (E-AUDIT-008). A schema violation in the submission is E-AUDIT-003.
/// `reason` / `exclusions` are never required (DS-722).
pub fn submit(
    root: &Path,
    scan: &ScanResult,
    submission: &Value,
) -> Result<SubmitResult, JudgmentOpError> {
    let layout = VerifyLayout::new(root);

    // E-AUDIT-001 comes first (DS-1089), so look up bundle_id before
    // validating the rest of the shape.
    let bundle_id = submission
        .get("bundle_id")
        .and_then(Value::as_str)
        .ok_or_else(|| rejected("E-AUDIT-001", "the submission has no bundle_id"))?;
    let bundle_path = layout
        .cache_dir()
        .join("bundles")
        .join(format!("{bundle_id}.json"));
    if !safe_bundle_id(bundle_id) || !bundle_path.is_file() {
        return Err(rejected(
            "E-AUDIT-001",
            format!("bundle {bundle_id} does not exist in .verify/cache/bundles/"),
        ));
    }
    let bundle: Value = std::fs::read_to_string(&bundle_path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .ok_or_else(|| rejected("E-AUDIT-001", format!("bundle {bundle_id} cannot be read")))?;

    let parsed: Submission = serde_json::from_value(submission.clone()).map_err(|error| {
        rejected(
            "E-AUDIT-003",
            format!("the submission violates the schema: {error}"),
        )
    })?;
    if Some(parsed.subject.as_str()) != bundle["subject"].as_str() {
        return Err(rejected(
            "E-AUDIT-003",
            format!(
                "subject {} does not match the bundle's subject",
                parsed.subject
            ),
        ));
    }
    if !JUDGMENT_KINDS.contains(&parsed.judgment_kind.as_str())
        || Some(parsed.judgment_kind.as_str()) != bundle["judgment_kind"].as_str()
    {
        return Err(rejected(
            "E-AUDIT-003",
            format!(
                "judgment_kind {} does not match the bundle's or is outside the domain",
                parsed.judgment_kind
            ),
        ));
    }
    if parsed.actor.kind != "human" && parsed.actor.kind != "agent" {
        return Err(rejected("E-AUDIT-003", "actor.kind must be human or agent"));
    }
    if let Some(bad) = parsed
        .reason
        .iter()
        .flat_map(|entry| &entry.basis)
        .find(|basis| !BASIS_KINDS.contains(&basis.kind.as_str()))
    {
        return Err(rejected(
            "E-AUDIT-003",
            format!(
                "reason basis kind `{}` is outside {}",
                bad.kind,
                BASIS_KINDS.join(" / ")
            ),
        ));
    }

    let subject = match bundle["subject_type"].as_str() {
        Some("vo") => Subject::Vo(parsed.subject.clone()),
        _ => Subject::Test(parsed.subject.clone()),
    };
    let recorded = Binding {
        subject_hash: serde_json::from_value(bundle["subject_hash"].clone())
            .map_err(|_| rejected("E-AUDIT-001", "the bundle carries no subject_hash"))?,
        dependencies: serde_json::from_value(bundle["dependencies"].clone())
            .map_err(|_| rejected("E-AUDIT-001", "the bundle carries no dependencies"))?,
    };
    // DS-718: anything that keeps the current binding from being computed
    // (a deleted Test, a target that no longer resolves) is also a change
    // since the bundle was recorded.
    match current_binding(&layout, scan, &subject) {
        Ok(current) if current == recorded => {}
        // DS-536: a duplicated Test id names no single subject; that is
        // reported as itself, not as a stale bundle.
        Err(
            error @ JudgmentOpError::Unresolved {
                code: "E-SCAN-002", ..
            },
        ) => return Err(error),
        Ok(_) | Err(JudgmentOpError::Usage(_)) | Err(JudgmentOpError::Unresolved { .. }) => {
            return Err(rejected(
                "E-AUDIT-002",
                format!(
                    "{} or its dependencies changed since bundle {bundle_id} was recorded",
                    parsed.subject
                ),
            ))
        }
        Err(other) => return Err(other),
    }

    if !DECISION_VALUES.contains(&parsed.decision.as_str()) {
        return Err(rejected(
            "E-AUDIT-004",
            format!(
                "decision `{}` is not one of {}",
                parsed.decision,
                DECISION_VALUES.join(" / ")
            ),
        ));
    }

    let existing = read_all_decisions(&layout)?;
    let decision_id = vtest_store::new_record_id();
    for target in &parsed.supersedes {
        if target == &decision_id {
            return Err(rejected(
                "E-AUDIT-008",
                "a judgment record cannot supersede itself",
            ));
        }
        let Some(found) = existing.iter().find(|record| &record.id == target) else {
            return Err(rejected(
                "E-AUDIT-008",
                format!("supersedes target {target} does not exist"),
            ));
        };
        if found.subject != parsed.subject || found.judgment_kind != parsed.judgment_kind {
            return Err(rejected(
                "E-AUDIT-008",
                format!("supersedes target {target} has a different subject or judgment_kind"),
            ));
        }
    }

    let record = DecisionRecord {
        id: decision_id.clone(),
        subject: parsed.subject.clone(),
        judgment_kind: parsed.judgment_kind.clone(),
        supersedes: parsed.supersedes,
        subject_hash: recorded.subject_hash,
        dependencies: recorded.dependencies,
        actor: parsed.actor,
        decision: parsed.decision.clone(),
        reason: parsed.reason,
        exclusions: parsed.exclusions,
        decided_at: vtest_store::records::now_rfc3339(),
        revision: serde_json::from_value(bundle["revision"].clone()).ok(),
    };
    write_decision(&layout, &record)?;
    Ok(SubmitResult {
        accepted: true,
        decision_id,
        subject: parsed.subject,
        judgment_kind: parsed.judgment_kind,
        decision: parsed.decision,
    })
}
