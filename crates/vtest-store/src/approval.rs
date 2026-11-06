//! Approval domain support: upstream dependency-closure construction and
//! effective-approval-state computation, per 本冊 §3.5 (承認レコード) and
//! DS-1050〜DS-1062 / DS-1461〜DS-1490 (§18.3.7 承認と判断記録の分離).
//!
//! This module only covers the two subject types this closure-slice
//! implements end to end — `vo` (DS-1050, DS-1487) and `document` (DS-1051,
//! DS-1480). `judgment` (DS-1052) is deliberately out of scope: it requires
//! reading a `.verify/decisions/` judgment record, and no `DecisionRecord` /
//! `JudgmentRecord` type exists anywhere in this codebase — building that
//! domain (本冊 §8.5, a large separate normative surface) is not something
//! this module invents. `vtest-cli`'s `approval create --subject-type
//! judgment` rejects with a disclosed error rather than silently doing the
//! wrong thing.

use std::collections::{BTreeMap, BTreeSet};

use vtest_model::{
    root_node_subject_hash, section_node_subject_hash, sentence_node_subject_hash, vo_subject_hash,
    ContentHash, DocumentFile, RootNode, SectionNode, SentenceNode, VoId, VoRecord,
};

use crate::{read_document_dir, records::DependencyRecord, StoreError, VerifyLayout};

/// A flat id -> (subject hash, `derives_from` edges) index over every node in
/// every `.verify/doc/*.json` file, built fresh from canonical records (this
/// crate never persists a derived index — see `AGENTS.md`'s "Canonical and
/// derived data remain separate").
pub struct DocumentNodeIndex(BTreeMap<String, (ContentHash, Vec<String>)>);

impl DocumentNodeIndex {
    fn insert(&mut self, id: String, hash: ContentHash, derives_from: Vec<String>) {
        self.0.insert(id, (hash, derives_from));
    }

    pub fn get(&self, id: &str) -> Option<&(ContentHash, Vec<String>)> {
        self.0.get(id)
    }

    /// Every node id with its `derives_from` targets, in id order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &[String])> {
        self.0
            .iter()
            .map(|(id, (_, parents))| (id.as_str(), parents.as_slice()))
    }
}

/// Builds the node index by reading every document file under `.verify/doc/`.
/// A node id that appears in more than one file, or a `derives_from` target
/// that resolves to nothing, is not this function's concern — E-SCAN-006 /
/// E-SCAN-012 style resolution is the scan layer's job (see
/// `vtest-verify`'s `evaluate_chain_integrity`); this builder just records
/// what each file declares.
pub fn build_document_node_index(layout: &VerifyLayout) -> Result<DocumentNodeIndex, StoreError> {
    let mut index = DocumentNodeIndex(BTreeMap::new());
    let listing = read_document_dir(&layout.doc_dir())?;
    for name in &listing.names {
        let file = crate::canonical::read_document_file(layout, name)?;
        index_document_file(&file, &mut index);
    }
    Ok(index)
}

fn index_document_file(file: &DocumentFile, index: &mut DocumentNodeIndex) {
    for node in &file.root {
        index_root_node(node, index);
    }
    for node in &file.request {
        index_sentence_node(node, index);
    }
    for section in file
        .require
        .iter()
        .chain(&file.spec)
        .chain(&file.detailed_spec)
        .chain(&file.basic_design)
        .chain(&file.design)
    {
        index_section_node(section, index);
    }
}

fn index_root_node(node: &RootNode, index: &mut DocumentNodeIndex) {
    index.insert(
        node.id.as_str().to_owned(),
        root_node_subject_hash(node),
        Vec::new(),
    );
}

fn index_sentence_node(node: &SentenceNode, index: &mut DocumentNodeIndex) {
    let derives_from = node
        .derives_from
        .iter()
        .map(|id| id.as_str().to_owned())
        .collect();
    index.insert(
        node.id.as_str().to_owned(),
        sentence_node_subject_hash(node),
        derives_from,
    );
}

fn index_section_node(node: &SectionNode, index: &mut DocumentNodeIndex) {
    let derives_from = node
        .derives_from
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|id| id.as_str().to_owned())
        .collect();
    index.insert(
        node.id.as_str().to_owned(),
        section_node_subject_hash(node),
        derives_from,
    );
    for item in node.items.as_deref().unwrap_or(&[]) {
        index_sentence_node(item, index);
    }
    for section in node.sections.as_deref().unwrap_or(&[]) {
        index_section_node(section, index);
    }
}

/// A dependency closure that cannot be resolved completely: a dangling VO
/// `parent`, a VO `derives_from` or document `derives_from` that names no
/// document node, or a cycle. DS-1058/DS-914/DS-1298: an Approval cannot be
/// bound to such a closure (E-APPROVAL-001), and no effective `approved` is
/// derived from it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnresolvedDependency(pub String);

impl std::fmt::Display for UnresolvedDependency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Follows `derives_from` edges from `start` (inclusive of `start` itself)
/// to a fixed point, accumulating each visited node's current subject hash.
/// A `derives_from` target absent from `index` (a dangling link) or a cycle
/// makes the closure unresolvable: a silently shrunk closure would let an
/// Approval bind to, and stay current against, an incomplete upstream
/// (DS-1058/DS-1298).
fn collect_document_closure(
    index: &DocumentNodeIndex,
    start: &str,
    out: &mut BTreeMap<String, ContentHash>,
) -> Result<(), UnresolvedDependency> {
    fn visit(
        index: &DocumentNodeIndex,
        id: &str,
        path: &mut Vec<String>,
        done: &mut BTreeSet<String>,
        out: &mut BTreeMap<String, ContentHash>,
    ) -> Result<(), UnresolvedDependency> {
        if done.contains(id) {
            return Ok(());
        }
        if path.iter().any(|entry| entry == id) {
            return Err(UnresolvedDependency(format!(
                "document derives_from cycle through '{id}' ({})",
                path.join(" -> ")
            )));
        }
        let Some((hash, derives_from)) = index.get(id) else {
            return Err(UnresolvedDependency(match path.last() {
                Some(from) => format!("document node '{from}' derives from missing node '{id}'"),
                None => format!("dependency names missing document node '{id}'"),
            }));
        };
        out.insert(id.to_owned(), hash.clone());
        path.push(id.to_owned());
        for parent in derives_from {
            visit(index, parent, path, done, out)?;
        }
        path.pop();
        done.insert(id.to_owned());
        Ok(())
    }
    visit(index, start, &mut Vec::new(), &mut BTreeSet::new(), out)
}

/// DS-1480: a document subject's dependency closure is its own "実効的な
/// 上流（自分の辺∪先祖の辺）" — the ancestors reachable by following
/// `derives_from`, recursively closed. The subject node itself is excluded
/// (DS-456: bound separately via `subject_hash`, not duplicated into
/// `dependencies`).
pub fn document_dependencies(
    index: &DocumentNodeIndex,
    subject_id: &str,
) -> Result<Vec<DependencyRecord>, UnresolvedDependency> {
    let mut out = BTreeMap::new();
    let Some((_, derives_from)) = index.get(subject_id) else {
        return Err(UnresolvedDependency(format!(
            "no document node with id '{subject_id}' exists"
        )));
    };
    for parent in derives_from {
        if parent == subject_id {
            return Err(UnresolvedDependency(format!(
                "document node '{subject_id}' derives from itself"
            )));
        }
        collect_document_closure(index, parent, &mut out)?;
    }
    if out.contains_key(subject_id) {
        return Err(UnresolvedDependency(format!(
            "document derives_from cycle through '{subject_id}'"
        )));
    }
    Ok(out
        .into_iter()
        .map(|(entity, hash)| DependencyRecord { entity, hash })
        .collect())
}

/// DS-1017 new (`233caec`/PR #50): "鮮度" — "当該 document subject hash
/// （DES-572）と、当該 document を dependency に含む承認・判断記録が保存
/// した dependency entry の hash との一致（DS-862・DS-1601・DS-1605）",
/// computed per node id (each of `node_ids`, expected to already be
/// top-level-only — see `doc_registry::document_top_level_node_ids`'s doc
/// comment for why a section's nested `items`/`sections` children must
/// not be included). For each node id: `Some(true)` if every `dependencies
/// []` entry across `approvals` naming it as `entity` still carries that
/// node's *current* subject hash (from `doc_index`); `Some(false)` if any
/// entry disagrees; `None` if no record depends on the node at all (never
/// rounded to `Some(true)`). No judgment-record domain exists in this
/// codebase (see this module's own doc comment), so only Approval records
/// are consulted — the same disclosed scope boundary as elsewhere in this
/// module.
pub fn node_freshness(
    doc_index: &DocumentNodeIndex,
    approvals: &[crate::records::ApprovalRecord],
    node_ids: &BTreeSet<String>,
) -> BTreeMap<String, Option<bool>> {
    let mut out = BTreeMap::new();
    for node_id in node_ids {
        let Some((hash, _)) = doc_index.get(node_id) else {
            continue;
        };
        let recorded_hashes: Vec<_> = approvals
            .iter()
            .flat_map(|record| &record.dependencies)
            .filter(|dependency| &dependency.entity == node_id)
            .map(|dependency| &dependency.hash)
            .collect();
        let freshness = if recorded_hashes.is_empty() {
            None
        } else {
            Some(recorded_hashes.iter().all(|recorded| *recorded == hash))
        };
        out.insert(node_id.clone(), freshness);
    }
    out
}

/// DS-1487: a VO subject's dependency closure is the recursive parent-VO
/// chain, the document nodes that VO and each parent VO reference via
/// `derives_from`, and each of those document nodes' own effective upstream
/// (自分の辺∪先祖の辺) recursively closed. The subject VO itself is excluded
/// (DS-456, same reasoning as the document case).
pub fn vo_dependencies(
    vos: &BTreeMap<String, VoRecord>,
    doc_index: &DocumentNodeIndex,
    subject: &VoId,
) -> Result<Vec<DependencyRecord>, UnresolvedDependency> {
    let mut out = BTreeMap::new();
    let mut chain = Vec::new();
    let mut seen_vo = BTreeSet::from([subject.as_str().to_owned()]);
    let mut cursor = vos.get(subject.as_str()).and_then(|vo| vo.parent.clone());
    while let Some(parent_id) = cursor {
        if !seen_vo.insert(parent_id.as_str().to_owned()) {
            return Err(UnresolvedDependency(format!(
                "VO parent chain of '{}' has a cycle through '{}'",
                subject.as_str(),
                parent_id.as_str()
            )));
        }
        let Some(parent_vo) = vos.get(parent_id.as_str()) else {
            return Err(UnresolvedDependency(format!(
                "VO parent '{}' in the chain of '{}' does not exist",
                parent_id.as_str(),
                subject.as_str()
            )));
        };
        out.insert(parent_id.as_str().to_owned(), vo_subject_hash(parent_vo));
        chain.push(parent_id.clone());
        cursor = parent_vo.parent.clone();
    }

    let mut doc_starts = BTreeSet::new();
    for vo_id in std::iter::once(subject.clone()).chain(chain) {
        if let Some(record) = vos.get(vo_id.as_str()) {
            for derives_from in &record.derives_from {
                doc_starts.insert(derives_from.doc.as_str().to_owned());
            }
        }
    }
    for doc_id in doc_starts {
        collect_document_closure(doc_index, &doc_id, &mut out)?;
    }

    Ok(out
        .into_iter()
        .map(|(entity, hash)| DependencyRecord { entity, hash })
        .collect())
}

/// The 本冊 §3.5 / DS-1466 effective approval state: `draft` or `approved`.
/// No other value exists at this axis (approval is independent of the five
/// verification states — DS-089〜093, DS-1461/1462).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EffectiveApprovalState {
    Draft,
    Approved,
}

/// DS-1467 (validity), DS-1466 (effective set = valid records minus those
/// named by another valid record's `supersedes`), DS-1461/1462 (draft if
/// empty or any non-`approved` remains), DS-1466 (approved iff all remaining
/// are `approved`). `records` should already be schema-valid
/// (`ApprovalRecord::from_yaml` enforces `subject_type`/`approved_state`
/// domain and shape at read time); see [`effective_approval`] for the
/// `supersedes` graph rules and the diagnostics this drops.
pub fn effective_approval_state(
    records: &[crate::records::ApprovalRecord],
    subject_type: &str,
    subject_id: &str,
    current_hash: &ContentHash,
    current_dependencies: &[DependencyRecord],
) -> EffectiveApprovalState {
    effective_approval(
        records,
        subject_type,
        subject_id,
        current_hash,
        current_dependencies,
    )
    .0
}

/// [`effective_approval_state`] plus the W-STORE-005 warnings for records
/// whose `supersedes` cannot contribute.
///
/// DS-556/DS-1474: a record whose `supersedes` names an id that is not a
/// record of the same subject (missing, or another subject's record),
/// names itself, or lies on a `supersedes` cycle does not contribute to the
/// effective set — neither as a member nor by removing the records it
/// names — and is reported as W-STORE-005. The graph is checked on every
/// read, not only when a record is written, because a stored record can be
/// edited or its target removed afterwards.
pub fn effective_approval(
    records: &[crate::records::ApprovalRecord],
    subject_type: &str,
    subject_id: &str,
    current_hash: &ContentHash,
    current_dependencies: &[DependencyRecord],
) -> (EffectiveApprovalState, Vec<vtest_model::Diagnostic>) {
    let (state, _, diagnostics) = effective_set(
        records,
        subject_type,
        subject_id,
        current_hash,
        current_dependencies,
    );
    (state, diagnostics)
}

/// DS-1161: the approver ids of the records that make the subject
/// effectively `approved`. Empty whenever the effective state is `draft`,
/// so a stale, superseded, rejected or withdrawn record never lends its
/// approver to a gate role.
pub fn effective_approvers(
    records: &[crate::records::ApprovalRecord],
    subject_type: &str,
    subject_id: &str,
    current_hash: &ContentHash,
    current_dependencies: &[DependencyRecord],
) -> (BTreeSet<String>, Vec<vtest_model::Diagnostic>) {
    let (state, effective, diagnostics) = effective_set(
        records,
        subject_type,
        subject_id,
        current_hash,
        current_dependencies,
    );
    let approvers = if state == EffectiveApprovalState::Approved {
        effective
            .iter()
            .map(|record| record.approver.id.clone())
            .collect()
    } else {
        BTreeSet::new()
    };
    (approvers, diagnostics)
}

fn effective_set<'a>(
    records: &'a [crate::records::ApprovalRecord],
    subject_type: &str,
    subject_id: &str,
    current_hash: &ContentHash,
    current_dependencies: &[DependencyRecord],
) -> (
    EffectiveApprovalState,
    Vec<&'a crate::records::ApprovalRecord>,
    Vec<vtest_model::Diagnostic>,
) {
    let current_deps: BTreeSet<(String, String)> = current_dependencies
        .iter()
        .map(|dependency| {
            (
                dependency.entity.clone(),
                dependency.hash.as_str().to_owned(),
            )
        })
        .collect();
    let subject_records: Vec<&crate::records::ApprovalRecord> = records
        .iter()
        .filter(|record| record.subject_type == subject_type && record.subject == subject_id)
        .collect();
    let by_id: BTreeMap<&str, &crate::records::ApprovalRecord> = subject_records
        .iter()
        .map(|record| (record.id.as_str(), *record))
        .collect();

    let mut diagnostics = Vec::new();
    let mut ill_formed: BTreeSet<&str> = BTreeSet::new();
    for record in &subject_records {
        for target in &record.supersedes {
            let reason = if target == &record.id {
                Some("names itself")
            } else if !by_id.contains_key(target.as_str()) {
                Some("names no Approval record of the same subject")
            } else {
                None
            };
            if let Some(reason) = reason {
                ill_formed.insert(record.id.as_str());
                diagnostics.push(vtest_model::Diagnostic::warning(
                    "W-STORE-005",
                    format!(
                        "Approval {} supersedes '{target}', which {reason}; the record does \
                         not contribute to the effective set (DS-556/DS-1474)",
                        record.id
                    ),
                ));
            }
        }
    }
    for id in supersedes_cycle_members(&by_id) {
        if ill_formed.insert(id) {
            diagnostics.push(vtest_model::Diagnostic::warning(
                "W-STORE-005",
                format!(
                    "Approval {id} is on a supersedes cycle; the record does not contribute \
                     to the effective set (DS-556/DS-1474)"
                ),
            ));
        }
    }

    let valid: Vec<&crate::records::ApprovalRecord> = subject_records
        .iter()
        .copied()
        .filter(|record| !ill_formed.contains(record.id.as_str()))
        .filter(|record| {
            &record.subject_hash == current_hash && {
                let deps: BTreeSet<(String, String)> = record
                    .dependencies
                    .iter()
                    .map(|dependency| {
                        (
                            dependency.entity.clone(),
                            dependency.hash.as_str().to_owned(),
                        )
                    })
                    .collect();
                deps == current_deps
            }
        })
        .collect();

    let superseded: BTreeSet<&str> = valid
        .iter()
        .flat_map(|record| record.supersedes.iter().map(String::as_str))
        .collect();
    let effective: Vec<&&crate::records::ApprovalRecord> = valid
        .iter()
        .filter(|record| !superseded.contains(record.id.as_str()))
        .collect();

    // DS-556/DS-463: an ill-formed record does not contribute to the
    // effective set — neither as a member nor by removing the records it
    // names — and is reported as W-STORE-005. It is not a standing veto:
    // the state is decided by the effective set alone (DS-1466), so adding a
    // well-formed record for the current hash (a re-approval, or a record
    // that supersedes the ill-formed one) recovers without deleting history.
    let state = if effective.is_empty()
        || effective
            .iter()
            .any(|record| record.approved_state != "approved")
    {
        EffectiveApprovalState::Draft
    } else {
        EffectiveApprovalState::Approved
    };
    (state, effective.into_iter().copied().collect(), diagnostics)
}

/// Ids of records that lie on a `supersedes` cycle (following only edges
/// to records of the same subject).
fn supersedes_cycle_members<'a>(
    by_id: &BTreeMap<&'a str, &'a crate::records::ApprovalRecord>,
) -> BTreeSet<&'a str> {
    let mut members = BTreeSet::new();
    for &start in by_id.keys() {
        // `start` is on a cycle when it is reachable from its own targets.
        let mut stack: Vec<&str> = by_id[start]
            .supersedes
            .iter()
            .map(String::as_str)
            .filter(|target| *target != start)
            .collect();
        let mut seen = BTreeSet::new();
        while let Some(id) = stack.pop() {
            if id == start {
                members.insert(start);
                break;
            }
            if !seen.insert(id) {
                continue;
            }
            if let Some(record) = by_id.get(id) {
                stack.extend(record.supersedes.iter().map(String::as_str));
            }
        }
    }
    members
}

/// Reads every VO record under `.verify/vo/` into an id-keyed map, for
/// [`vo_dependencies`]. Propagates the first unreadable record as an error
/// (fail-closed) rather than silently omitting it from the closure: a
/// silently-shrunk closure would make a stale Approval record compare as
/// "current" when it should not (DS-1467).
pub fn read_all_vos(layout: &VerifyLayout) -> Result<BTreeMap<String, VoRecord>, StoreError> {
    let mut vos = BTreeMap::new();
    for id in crate::read_record_ids(&layout.vo_dir())? {
        let (record, _diagnostics) = crate::canonical::read_vo_record(layout, &id)?;
        vos.insert(id, record);
    }
    Ok(vos)
}

/// Reads every Approval record under `.verify/approvals/`. The first record
/// that cannot be read or fails the schema is returned as the error
/// (fail-closed, E-SCAN-010 for a schema/id violation): skipping it would
/// silently drop, for example, a `withdrawn` record from the effective set
/// and bring back the `approved` it superseded (DS-1466/DS-1676).
pub fn read_all_approvals(
    layout: &VerifyLayout,
) -> Result<Vec<crate::records::ApprovalRecord>, StoreError> {
    let dir = layout.approvals_dir();
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(StoreError::Io { path: dir, source }),
    };
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| StoreError::Io {
            path: dir.clone(),
            source,
        })?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) == Some("yaml") {
            paths.push(path);
        }
    }
    // A deterministic order makes the reported first error deterministic.
    paths.sort();
    paths
        .iter()
        .map(|path| crate::records::read_approval(path))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::{ApprovalRecord, Approver};

    fn hash(seed: &str) -> ContentHash {
        vtest_model::ContentHash::from_text(seed)
    }

    fn index_with(entries: &[(&str, &str, &[&str])]) -> DocumentNodeIndex {
        let mut index = DocumentNodeIndex(BTreeMap::new());
        for (id, seed, derives_from) in entries {
            index.insert(
                (*id).to_owned(),
                hash(seed),
                derives_from
                    .iter()
                    .map(|value| (*value).to_owned())
                    .collect(),
            );
        }
        index
    }

    /// DS-1480: `document_dependencies` follows `derives_from` recursively
    /// (R-001 -> ROOT-001, transitively), excluding the subject itself.
    /// @vtest.id TEST-STORE-DOCUMENT-DEPENDENCIES-RECURSIVE-CLOSURE
    /// @vtest.covers VO-STORE-DOCUMENT-DEPENDENCIES-RECURSIVE-CLOSURE
    /// @vtest.target crates/vtest-store/src/approval.rs::document_dependencies
    /// @vtest.intent document nodeの上流依存をsubject自身を除いて再帰的に閉じることを確認する
    #[test]
    fn document_dependencies_follows_derives_from_recursively() {
        let index = index_with(&[
            ("ROOT-001", "root", &[]),
            ("R-001", "req", &["ROOT-001"]),
            ("SPEC-001", "spec", &["R-001"]),
        ]);
        let deps = document_dependencies(&index, "SPEC-001").expect("closure resolves");
        let entities: BTreeSet<&str> = deps.iter().map(|dep| dep.entity.as_str()).collect();
        assert_eq!(entities, BTreeSet::from(["R-001", "ROOT-001"]));
        assert!(
            !entities.contains("SPEC-001"),
            "the subject itself must not appear in its own dependency closure (DS-456)"
        );
    }

    /// DS-1480: a subject with no `derives_from` edges has an empty closure.
    /// @vtest.id TEST-STORE-DOCUMENT-DEPENDENCIES-LEAF-EMPTY
    /// @vtest.covers VO-STORE-DOCUMENT-DEPENDENCIES-RECURSIVE-CLOSURE
    /// @vtest.target crates/vtest-store/src/approval.rs::document_dependencies
    /// @vtest.intent 上流辺を持たないdocument nodeの依存closureが空であることを確認する
    #[test]
    fn document_dependencies_of_a_leaf_node_is_empty() {
        let index = index_with(&[("ROOT-001", "root", &[])]);
        assert!(document_dependencies(&index, "ROOT-001")
            .expect("closure resolves")
            .is_empty());
    }

    fn vo(id: &str, parent: Option<&str>, doc_id: &str) -> VoRecord {
        VoRecord {
            id: VoId::new(id),
            parent: parent.map(VoId::new),
            derives_from: vec![vtest_model::DerivesFrom {
                doc: vtest_model::DocumentId::new(doc_id),
                anchor: None,
                note: None,
            }],
            claim: "fixture claim".to_owned(),
            dimensions: Vec::new(),
            coverage_policy: None,
            combinations: Vec::new(),
            representative_cases: Vec::new(),
            created: "2026-09-09T00:00:00Z".to_owned(),
            updated: "2026-09-09T00:00:00Z".to_owned(),
        }
    }

    /// DS-1487: `vo_dependencies` includes the parent-VO chain and, for the
    /// subject VO and every ancestor VO, the document closure each one's
    /// own `derives_from` reaches.
    /// @vtest.id TEST-STORE-VO-DEPENDENCIES-PARENT-AND-DOCUMENT-CLOSURE
    /// @vtest.covers VO-STORE-VO-DEPENDENCIES-PARENT-AND-DOCUMENT-CLOSURE
    /// @vtest.target crates/vtest-store/src/approval.rs::vo_dependencies
    /// @vtest.intent VO依存closureが再帰parent VOと各VOのdocument上流closureを含むことを確認する
    #[test]
    fn vo_dependencies_includes_parent_chain_and_document_closure() {
        let doc_index = index_with(&[("ROOT-001", "root", &[]), ("R-001", "req", &["ROOT-001"])]);
        let mut vos = BTreeMap::new();
        vos.insert("VO-PARENT".to_owned(), vo("VO-PARENT", None, "ROOT-001"));
        vos.insert(
            "VO-CHILD".to_owned(),
            vo("VO-CHILD", Some("VO-PARENT"), "R-001"),
        );

        let deps =
            vo_dependencies(&vos, &doc_index, &VoId::new("VO-CHILD")).expect("closure resolves");
        let entities: BTreeSet<&str> = deps.iter().map(|dep| dep.entity.as_str()).collect();
        assert_eq!(
            entities,
            BTreeSet::from(["VO-PARENT", "R-001", "ROOT-001"]),
            "must include the parent VO and the document closure of both the subject and its \
             parent"
        );
        assert!(!entities.contains("VO-CHILD"));
    }

    fn approval_record(
        id: &str,
        subject_type: &str,
        subject: &str,
        subject_hash: ContentHash,
        dependencies: Vec<DependencyRecord>,
        approved_state: &str,
        supersedes: Vec<String>,
    ) -> ApprovalRecord {
        ApprovalRecord {
            id: id.to_owned(),
            subject_type: subject_type.to_owned(),
            subject: subject.to_owned(),
            subject_hash,
            dependencies,
            judgment_ref: None,
            approver: Approver {
                kind: "human".to_owned(),
                id: "reviewer".to_owned(),
                model: None,
            },
            approved_state: approved_state.to_owned(),
            basis: Vec::new(),
            supersedes,
            approved_at: "2026-09-09T00:00:00Z".to_owned(),
        }
    }

    /// DS-1467: a record whose `subject_hash` no longer matches the
    /// subject's current content hash is not valid -- it drops out of the
    /// effective set entirely (not merely "not approved"), so a subject
    /// with only such a record reads as `Draft`, identically to having no
    /// record at all.
    /// @vtest.id TEST-STORE-APPROVAL-STALE-SUBJECT-HASH-DRAFT
    /// @vtest.covers VO-STORE-APPROVAL-VALIDITY-CURRENT-CLOSURE
    /// @vtest.target crates/vtest-store/src/approval.rs::effective_approval_state
    /// @vtest.intent subject_hashが現在値と不一致の承認recordを実効集合へ寄与させないことを確認する
    #[test]
    fn effective_state_drops_a_record_with_a_stale_subject_hash() {
        let record = approval_record(
            "01A",
            "vo",
            "VO-X",
            hash("old-content"),
            Vec::new(),
            "approved",
            Vec::new(),
        );
        let state = effective_approval_state(&[record], "vo", "VO-X", &hash("new-content"), &[]);
        assert_eq!(state, EffectiveApprovalState::Draft);
    }

    /// DS-1467's "対象指定が一致すること" (subject identity match) condition:
    /// a record for a *different* subject entirely (either a different
    /// `subject_type` or a different `subject` id) must never count toward
    /// this subject's effective state, even if its `subject_hash` and
    /// `dependencies` happen to be otherwise well-formed.
    /// @vtest.id TEST-STORE-APPROVAL-DIFFERENT-SUBJECT-IGNORED
    /// @vtest.covers VO-STORE-APPROVAL-VALIDITY-CURRENT-CLOSURE
    /// @vtest.target crates/vtest-store/src/approval.rs::effective_approval_state
    /// @vtest.intent subject typeまたはIDが対象と異なる承認recordを実効集合へ寄与させないことを確認する
    #[test]
    fn effective_state_ignores_a_record_for_a_different_subject() {
        let wrong_type = approval_record(
            "01A",
            "document",
            "VO-X",
            hash("content"),
            Vec::new(),
            "approved",
            Vec::new(),
        );
        let wrong_id = approval_record(
            "01B",
            "vo",
            "VO-OTHER",
            hash("content"),
            Vec::new(),
            "approved",
            Vec::new(),
        );
        let state =
            effective_approval_state(&[wrong_type, wrong_id], "vo", "VO-X", &hash("content"), &[]);
        assert_eq!(
            state,
            EffectiveApprovalState::Draft,
            "neither record targets (vo, VO-X), so neither may contribute to its effective \
             state"
        );
    }

    /// DS-1467's "dependencies の entity・hash とも完全一致" condition: a
    /// record whose `dependencies` entity *set* differs from the current
    /// closure (a missing or an extra entity) is invalid, distinctly from
    /// the already-covered case of a matching entity with a differing
    /// hash.
    /// @vtest.id TEST-STORE-APPROVAL-DEPENDENCY-ENTITY-SET-MISMATCH-DRAFT
    /// @vtest.covers VO-STORE-APPROVAL-VALIDITY-CURRENT-CLOSURE
    /// @vtest.target crates/vtest-store/src/approval.rs::effective_approval_state
    /// @vtest.intent dependenciesのentity集合が現在closureと完全一致しない承認を無効にすることを確認する
    #[test]
    fn effective_state_drops_a_record_whose_dependency_entity_set_differs() {
        let record = approval_record(
            "01A",
            "vo",
            "VO-X",
            hash("content"),
            vec![DependencyRecord {
                entity: "ROOT-001".to_owned(),
                hash: hash("root"),
            }],
            "approved",
            Vec::new(),
        );
        // Current closure has an *additional* entity the record never
        // named -- same-hash-for-shared-entities is not enough; the sets
        // themselves must match exactly.
        let current_deps = [
            DependencyRecord {
                entity: "ROOT-001".to_owned(),
                hash: hash("root"),
            },
            DependencyRecord {
                entity: "ROOT-002".to_owned(),
                hash: hash("root2"),
            },
        ];
        let state =
            effective_approval_state(&[record], "vo", "VO-X", &hash("content"), &current_deps);
        assert_eq!(state, EffectiveApprovalState::Draft);
    }

    /// DS-1467: a record whose `dependencies` no longer match the subject's
    /// current dependency closure (entity or hash) is invalid for the same
    /// reason -- currentness is bound to both axes, not just the subject's
    /// own hash.
    /// @vtest.id TEST-STORE-APPROVAL-STALE-DEPENDENCY-HASH-DRAFT
    /// @vtest.covers VO-STORE-APPROVAL-VALIDITY-CURRENT-CLOSURE
    /// @vtest.target crates/vtest-store/src/approval.rs::effective_approval_state
    /// @vtest.intent dependency hashが現在closureと一致しない承認を無効にすることを確認する
    #[test]
    fn effective_state_drops_a_record_with_a_stale_dependency_closure() {
        let record = approval_record(
            "01A",
            "vo",
            "VO-X",
            hash("content"),
            vec![DependencyRecord {
                entity: "ROOT-001".to_owned(),
                hash: hash("old-root"),
            }],
            "approved",
            Vec::new(),
        );
        let current_deps = [DependencyRecord {
            entity: "ROOT-001".to_owned(),
            hash: hash("new-root"),
        }];
        let state =
            effective_approval_state(&[record], "vo", "VO-X", &hash("content"), &current_deps);
        assert_eq!(state, EffectiveApprovalState::Draft);
    }

    /// A single current, valid, unsuperseded `approved` record reads as
    /// `Approved`.
    /// @vtest.id TEST-STORE-APPROVAL-ONE-CURRENT-APPROVED-RECORD
    /// @vtest.covers VO-STORE-APPROVAL-EFFECTIVE-SET-STATE
    /// @vtest.target crates/vtest-store/src/approval.rs::effective_approval_state
    /// @vtest.intent currentかつ有効でsupersedeされていないapproved recordが実効approvedを導くことを確認する
    #[test]
    fn effective_state_is_approved_for_one_current_valid_approved_record() {
        let record = approval_record(
            "01A",
            "vo",
            "VO-X",
            hash("content"),
            Vec::new(),
            "approved",
            Vec::new(),
        );
        let state = effective_approval_state(&[record], "vo", "VO-X", &hash("content"), &[]);
        assert_eq!(state, EffectiveApprovalState::Approved);
    }

    /// DS-1466: a valid record named by another valid record's
    /// `supersedes` drops out of the effective set -- an `approved` record
    /// superseded by a `withdrawn` one reads as `Draft`, not `Approved`.
    /// @vtest.id TEST-STORE-APPROVAL-SUPERSEDED-RECORD-EXCLUDED
    /// @vtest.covers VO-STORE-APPROVAL-EFFECTIVE-SET-STATE
    /// @vtest.target crates/vtest-store/src/approval.rs::effective_approval_state
    /// @vtest.intent withdrawn recordにsupersedeされたapproved recordを除いた実効状態がdraftになることを確認する
    #[test]
    fn effective_state_excludes_a_superseded_record() {
        let approved = approval_record(
            "01A",
            "vo",
            "VO-X",
            hash("content"),
            Vec::new(),
            "approved",
            Vec::new(),
        );
        let withdrawn = approval_record(
            "01B",
            "vo",
            "VO-X",
            hash("content"),
            Vec::new(),
            "withdrawn",
            vec!["01A".to_owned()],
        );
        let state =
            effective_approval_state(&[approved, withdrawn], "vo", "VO-X", &hash("content"), &[]);
        assert_eq!(state, EffectiveApprovalState::Draft);
    }

    fn rec(id: &str, subject: &str, state: &str, supersedes: &[&str]) -> ApprovalRecord {
        approval_record(
            id,
            "vo",
            subject,
            hash("content"),
            Vec::new(),
            state,
            supersedes.iter().map(|value| (*value).to_owned()).collect(),
        )
    }

    fn state_and_codes(records: &[ApprovalRecord]) -> (EffectiveApprovalState, Vec<String>) {
        let (state, diagnostics) = effective_approval(records, "vo", "VO-X", &hash("content"), &[]);
        (
            state,
            diagnostics
                .into_iter()
                .map(|diagnostic| diagnostic.code)
                .collect(),
        )
    }

    /// DS-556/DS-1474: an existing record whose `supersedes` is unresolved,
    /// names another subject's record, names itself, or forms a cycle never
    /// contributes to the effective set and yields W-STORE-005, so a subject
    /// whose only approvals are ill-formed is draft.
    /// @vtest.id TEST-STORE-APPROVAL-SUPERSEDES-GRAPH-EXCLUDED
    /// @vtest.covers VO-STORE-APPROVAL-SUPERSEDES-GRAPH-EXCLUDED
    /// @vtest.target crates/vtest-store/src/approval.rs::effective_approval
    /// @vtest.intent ill-formed supersedes graphs are excluded with W-STORE-005 and never read as approved
    #[test]
    fn ill_formed_supersedes_graphs_are_excluded_with_a_warning() {
        let cases: Vec<(&str, Vec<ApprovalRecord>)> = vec![
            (
                "unresolved",
                vec![rec(
                    "01A",
                    "VO-X",
                    "approved",
                    &["01ZZZZZZZZZZZZZZZZZZZZZZZZ"],
                )],
            ),
            (
                "other subject",
                vec![
                    rec("01O", "VO-OTHER", "approved", &[]),
                    rec("01A", "VO-X", "approved", &["01O"]),
                ],
            ),
            ("self", vec![rec("01A", "VO-X", "approved", &["01A"])]),
            (
                "cycle",
                vec![
                    rec("01A", "VO-X", "approved", &["01B"]),
                    rec("01B", "VO-X", "approved", &["01A"]),
                ],
            ),
        ];
        for (name, records) in cases {
            let (state, codes) = state_and_codes(&records);
            assert_eq!(state, EffectiveApprovalState::Draft, "{name}");
            assert!(
                !codes.is_empty() && codes.iter().all(|code| code == "W-STORE-005"),
                "{name}: {codes:?}"
            );
        }
    }

    /// PR #133 review: an ill-formed record is not a standing veto
    /// (DS-556/DS-1466). It contributes nothing — neither as a member nor by
    /// removing what it names — and keeps its W-STORE-005, but a well-formed
    /// record for the current hash recovers the subject without deleting
    /// history: a plain re-approval, or an approval that supersedes the
    /// ill-formed record.
    /// @vtest.id TEST-STORE-APPROVAL-ILL-FORMED-RECORD-RECOVERABLE
    /// @vtest.covers VO-STORE-APPROVAL-SUPERSEDES-GRAPH-EXCLUDED
    /// @vtest.target crates/vtest-store/src/approval.rs::effective_approval
    /// @vtest.intent 不正なsupersedesを持つrecordが残っていても、現在hashへの正常な再承認、または不正recordをsupersedesする承認を追加すればapprovedへ回復し、W-STORE-005は残ることを確認する
    #[test]
    fn an_ill_formed_record_does_not_block_a_well_formed_reapproval() {
        let broken = rec("01A", "VO-X", "approved", &["01MISSING"]);
        let (state, codes) = state_and_codes(std::slice::from_ref(&broken));
        assert_eq!(state, EffectiveApprovalState::Draft);
        assert_eq!(codes, vec!["W-STORE-005".to_owned()]);

        let reapproved = [broken.clone(), rec("01B", "VO-X", "approved", &[])];
        let (state, codes) = state_and_codes(&reapproved);
        assert_eq!(state, EffectiveApprovalState::Approved);
        assert_eq!(codes, vec!["W-STORE-005".to_owned()]);

        let superseding = [broken.clone(), rec("01B", "VO-X", "approved", &["01A"])];
        let (state, codes) = state_and_codes(&superseding);
        assert_eq!(state, EffectiveApprovalState::Approved);
        assert_eq!(codes, vec!["W-STORE-005".to_owned()]);

        // DS-556 literally: an ill-formed withdrawal removes nothing, so the
        // approval it named stays in the effective set (reported, not
        // vetoed). A well-formed withdrawal is what makes the subject draft.
        let broken_withdrawal = [
            rec("01A", "VO-X", "approved", &[]),
            rec("01W", "VO-X", "withdrawn", &["01A", "01MISSING"]),
        ];
        let (state, codes) = state_and_codes(&broken_withdrawal);
        assert_eq!(state, EffectiveApprovalState::Approved);
        assert_eq!(codes, vec!["W-STORE-005".to_owned()]);
        let withdrawn_again = [
            rec("01A", "VO-X", "approved", &[]),
            rec("01W", "VO-X", "withdrawn", &["01A", "01MISSING"]),
            rec("01X", "VO-X", "withdrawn", &["01A"]),
        ];
        let (state, _) = state_and_codes(&withdrawn_again);
        assert_eq!(state, EffectiveApprovalState::Draft);
    }

    /// DS-1466/DS-1470: a well-formed re-approval chain (approve, withdraw,
    /// approve again superseding the withdrawal) is still approved and
    /// produces no W-STORE-005.
    /// @vtest.id TEST-STORE-APPROVAL-WELL-FORMED-REAPPROVAL
    /// @vtest.covers VO-STORE-APPROVAL-SUPERSEDES-GRAPH-EXCLUDED
    /// @vtest.target crates/vtest-store/src/approval.rs::effective_approval
    /// @vtest.intent a well-formed supersedes chain is not reported and re-approval reads as approved
    #[test]
    fn a_well_formed_reapproval_chain_is_approved_without_warnings() {
        let records = [
            rec("01A", "VO-X", "approved", &[]),
            rec("01B", "VO-X", "withdrawn", &["01A"]),
            rec("01C", "VO-X", "approved", &["01B"]),
            rec("01O", "VO-OTHER", "approved", &["01MISSING"]),
        ];
        let (state, codes) = state_and_codes(&records);
        assert_eq!(state, EffectiveApprovalState::Approved);
        assert!(codes.is_empty(), "{codes:?}");
    }

    /// DS-1058: a document closure that reaches a missing node or a cycle
    /// cannot be resolved completely and is an error, never a shrunken
    /// closure.
    /// @vtest.id TEST-STORE-DOCUMENT-DEPENDENCIES-UNRESOLVED
    /// @vtest.covers VO-APPROVAL-CREATE-UNRESOLVED-SUBJECT-REJECTED
    /// @vtest.target crates/vtest-store/src/approval.rs::document_dependencies
    /// @vtest.intent missing upstream nodes and derives_from cycles make the document closure unresolved
    #[test]
    fn document_dependencies_rejects_missing_nodes_and_cycles() {
        let missing = index_with(&[("SPEC-001", "spec", &["R-MISSING"])]);
        assert!(document_dependencies(&missing, "SPEC-001").is_err());
        let deep_missing = index_with(&[
            ("R-001", "req", &["ROOT-MISSING"]),
            ("SPEC-001", "spec", &["R-001"]),
        ]);
        assert!(document_dependencies(&deep_missing, "SPEC-001").is_err());
        let cycle = index_with(&[("A-001", "a", &["B-001"]), ("B-001", "b", &["A-001"])]);
        assert!(document_dependencies(&cycle, "A-001").is_err());
        let self_cycle = index_with(&[("A-001", "a", &["A-001"])]);
        assert!(document_dependencies(&self_cycle, "A-001").is_err());
        assert!(document_dependencies(&missing, "NOPE").is_err());
    }

    /// DS-1058: a VO whose parent chain is dangling or cyclic, or whose
    /// own or ancestor `derives_from` names a missing document, cannot be
    /// resolved completely.
    /// @vtest.id TEST-STORE-VO-DEPENDENCIES-UNRESOLVED
    /// @vtest.covers VO-APPROVAL-CREATE-UNRESOLVED-SUBJECT-REJECTED
    /// @vtest.target crates/vtest-store/src/approval.rs::vo_dependencies
    /// @vtest.intent dangling or cyclic parents and missing documents make the VO closure unresolved
    #[test]
    fn vo_dependencies_rejects_dangling_and_cyclic_references() {
        let doc_index = index_with(&[("ROOT-001", "root", &[])]);
        let child = VoId::new("VO-CHILD");

        let mut dangling = BTreeMap::new();
        dangling.insert(
            "VO-CHILD".to_owned(),
            vo("VO-CHILD", Some("VO-MISSING"), "ROOT-001"),
        );
        assert!(vo_dependencies(&dangling, &doc_index, &child).is_err());

        let mut cyclic = BTreeMap::new();
        cyclic.insert(
            "VO-CHILD".to_owned(),
            vo("VO-CHILD", Some("VO-PARENT"), "ROOT-001"),
        );
        cyclic.insert(
            "VO-PARENT".to_owned(),
            vo("VO-PARENT", Some("VO-CHILD"), "ROOT-001"),
        );
        assert!(vo_dependencies(&cyclic, &doc_index, &child).is_err());

        let mut missing_doc = BTreeMap::new();
        missing_doc.insert("VO-CHILD".to_owned(), vo("VO-CHILD", None, "DOC-MISSING"));
        assert!(vo_dependencies(&missing_doc, &doc_index, &child).is_err());

        let mut ancestor_missing_doc = BTreeMap::new();
        ancestor_missing_doc.insert(
            "VO-CHILD".to_owned(),
            vo("VO-CHILD", Some("VO-PARENT"), "ROOT-001"),
        );
        ancestor_missing_doc.insert("VO-PARENT".to_owned(), vo("VO-PARENT", None, "DOC-MISSING"));
        assert!(vo_dependencies(&ancestor_missing_doc, &doc_index, &child).is_err());
    }
}
