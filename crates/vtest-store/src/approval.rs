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

/// Follows `derives_from` edges from `start` (inclusive of `start` itself)
/// to a fixed point, accumulating each visited node's current subject hash.
/// A `derives_from` target absent from `index` (a dangling link) is skipped
/// here without diagnosing it — that is E-SCAN-012's job, not this
/// dependency-closure builder's.
fn collect_document_closure(
    index: &DocumentNodeIndex,
    start: &str,
    out: &mut BTreeMap<String, ContentHash>,
) {
    let mut stack = vec![start.to_owned()];
    let mut seen = BTreeSet::new();
    while let Some(id) = stack.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        if let Some((hash, derives_from)) = index.get(&id) {
            out.insert(id.clone(), hash.clone());
            for parent in derives_from {
                stack.push(parent.clone());
            }
        }
    }
}

/// DS-1480: a document subject's dependency closure is its own "実効的な
/// 上流（自分の辺∪先祖の辺）" — the ancestors reachable by following
/// `derives_from`, recursively closed. The subject node itself is excluded
/// (DS-456: bound separately via `subject_hash`, not duplicated into
/// `dependencies`).
pub fn document_dependencies(index: &DocumentNodeIndex, subject_id: &str) -> Vec<DependencyRecord> {
    let mut out = BTreeMap::new();
    if let Some((_, derives_from)) = index.get(subject_id) {
        for parent in derives_from {
            collect_document_closure(index, parent, &mut out);
        }
    }
    out.into_iter()
        .map(|(entity, hash)| DependencyRecord { entity, hash })
        .collect()
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
) -> Vec<DependencyRecord> {
    let mut out = BTreeMap::new();
    let mut chain = Vec::new();
    let mut seen_vo = BTreeSet::new();
    let mut cursor = vos.get(subject.as_str()).and_then(|vo| vo.parent.clone());
    while let Some(parent_id) = cursor {
        if !seen_vo.insert(parent_id.as_str().to_owned()) {
            break; // cycle guard; a parent cycle is a separate, already-diagnosed defect elsewhere
        }
        let Some(parent_vo) = vos.get(parent_id.as_str()) else {
            break; // dangling parent reference; not this builder's diagnostic to raise
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
        collect_document_closure(doc_index, &doc_id, &mut out);
    }

    out.into_iter()
        .map(|(entity, hash)| DependencyRecord { entity, hash })
        .collect()
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
/// domain and shape at read time); this function only applies the
/// *current-binding* half of validity (subject_hash + dependencies exact
/// match against `current_hash`/`current_dependencies`).
pub fn effective_approval_state(
    records: &[crate::records::ApprovalRecord],
    subject_type: &str,
    subject_id: &str,
    current_hash: &ContentHash,
    current_dependencies: &[DependencyRecord],
) -> EffectiveApprovalState {
    let current_deps: BTreeSet<(String, String)> = current_dependencies
        .iter()
        .map(|dependency| {
            (
                dependency.entity.clone(),
                dependency.hash.as_str().to_owned(),
            )
        })
        .collect();

    let valid: Vec<&crate::records::ApprovalRecord> = records
        .iter()
        .filter(|record| {
            record.subject_type == subject_type
                && record.subject == subject_id
                && &record.subject_hash == current_hash
                && {
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

    if effective.is_empty() {
        return EffectiveApprovalState::Draft;
    }
    if effective
        .iter()
        .any(|record| record.approved_state != "approved")
    {
        return EffectiveApprovalState::Draft;
    }
    EffectiveApprovalState::Approved
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

/// Reads every Approval record under `.verify/approvals/`. Records that fail
/// to parse are skipped with their error appended to `diagnostics` rather
/// than aborting — mirrors `validate_approval_status`'s existing E-SCAN-010
/// reporting shape in `vtest-scan`, so this reader stays usable from a
/// context (CLI `approval show`) that wants best-effort history even when one
/// file on disk is malformed.
pub fn read_all_approvals(
    layout: &VerifyLayout,
) -> Result<Vec<crate::records::ApprovalRecord>, StoreError> {
    let dir = layout.approvals_dir();
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(StoreError::Io { path: dir, source }),
    };
    let mut records = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| StoreError::Io {
            path: dir.clone(),
            source,
        })?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("yaml") {
            continue;
        }
        if let Ok(record) = crate::records::read_approval(&path) {
            records.push(record);
        }
    }
    Ok(records)
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
    #[test]
    fn document_dependencies_follows_derives_from_recursively() {
        let index = index_with(&[
            ("ROOT-001", "root", &[]),
            ("R-001", "req", &["ROOT-001"]),
            ("SPEC-001", "spec", &["R-001"]),
        ]);
        let deps = document_dependencies(&index, "SPEC-001");
        let entities: BTreeSet<&str> = deps.iter().map(|dep| dep.entity.as_str()).collect();
        assert_eq!(entities, BTreeSet::from(["R-001", "ROOT-001"]));
        assert!(
            !entities.contains("SPEC-001"),
            "the subject itself must not appear in its own dependency closure (DS-456)"
        );
    }

    /// DS-1480: a subject with no `derives_from` edges has an empty closure.
    #[test]
    fn document_dependencies_of_a_leaf_node_is_empty() {
        let index = index_with(&[("ROOT-001", "root", &[])]);
        assert!(document_dependencies(&index, "ROOT-001").is_empty());
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
    #[test]
    fn vo_dependencies_includes_parent_chain_and_document_closure() {
        let doc_index = index_with(&[("ROOT-001", "root", &[]), ("R-001", "req", &["ROOT-001"])]);
        let mut vos = BTreeMap::new();
        vos.insert("VO-PARENT".to_owned(), vo("VO-PARENT", None, "ROOT-001"));
        vos.insert(
            "VO-CHILD".to_owned(),
            vo("VO-CHILD", Some("VO-PARENT"), "R-001"),
        );

        let deps = vo_dependencies(&vos, &doc_index, &VoId::new("VO-CHILD"));
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
}
