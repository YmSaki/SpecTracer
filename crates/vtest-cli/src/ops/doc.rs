//! `vtest doc add|list|show`: registers/inspects `.verify/doc/<id>.json`
//! node-tree files directly (本冊 §3.1, DES-585/586/595, DS-1015-1017/
//! 1681-1684). See `vtest_store::doc_registry`'s module doc comment for why
//! there is no separate persisted registry record — a prior version of this
//! module invented one (`.verify/doc/<id>.yaml`), found to have no
//! canonical grounding, and removed (team-lead ruling 2026-09-09; see
//! `reports/closure-trace.md`).

use std::collections::BTreeMap;

use vtest_store::{
    approval::{
        build_document_node_index, document_dependencies, effective_approval_state,
        read_all_approvals, EffectiveApprovalState,
    },
    doc_registry::{
        apply_derives_from, apply_root, doc_exists, document_derives_from, document_node_ids,
        read_all_docs, read_doc_view, read_node_tree, unresolved_derives_from, write_doc, DocView,
    },
    StoreError, VerifyLayout,
};

#[derive(Debug, thiserror::Error)]
pub enum DocOpError {
    #[error("{0}")]
    Usage(String),
    #[error("store error: {0}")]
    Store(#[from] StoreError),
}

pub struct AddArgs {
    pub id: String,
    pub path: String,
    /// DS-1003/1681/1685: bare upstream node ids to write onto every
    /// top-level node's own `derives_from`. `None` = the argument was not
    /// given (leave the file's own content untouched); `Some(ids)` =
    /// given, replacing every top-level node's `derives_from` with `ids`
    /// (DS-1685: replace, not append) even when `ids` is empty (clears
    /// them) -- see `vtest_store::doc_registry::apply_derives_from`'s doc
    /// comment for why this must not collapse to the same `Vec<String>`
    /// an omitted argument would produce.
    pub derives_from: Option<Vec<String>>,
    /// DS-1195: a plain bool. `true` asserts the source file's top-level
    /// content is entirely `root[]`; `false` performs no check (register
    /// the file's own layer assignment unchanged). See
    /// `vtest_store::doc_registry::apply_root`.
    pub root: bool,
    pub update: bool,
}

/// DES-595: registers `--path` (an already-built node-tree JSON file,
/// anywhere under the project) as `.verify/doc/<id>.json`. Rejects a
/// duplicate id unless `--update`; rejects `--update` of an id that was
/// never registered. `--path` is not stored anywhere beyond this call — the
/// canonical file *is* the registration (DES-585: the file has no
/// identifying field; the filename alone is the identity), so there is
/// nothing left to persist that a later call could read back out.
pub fn add(
    project_root: &std::path::Path,
    layout: &VerifyLayout,
    args: AddArgs,
) -> Result<DocView, DocOpError> {
    let existing = doc_exists(layout, &args.id);
    if existing && !args.update {
        return Err(DocOpError::Usage(format!(
            "document '{}' is already registered; use --update to re-register it",
            args.id
        )));
    }
    if !existing && args.update {
        return Err(DocOpError::Usage(format!(
            "--update given but no document '{}' is registered yet",
            args.id
        )));
    }

    let mut file = read_node_tree(project_root, &args.path)?;
    apply_root(&file, args.root).map_err(|error| DocOpError::Usage(error.to_string()))?;
    apply_derives_from(&mut file, args.derives_from.as_deref())
        .map_err(|error| DocOpError::Usage(error.to_string()))?;
    write_doc(layout, &args.id, &file)?;
    Ok(read_doc_view(layout, &args.id)?)
}

pub struct ListResult {
    pub records: Vec<DocView>,
    /// DS-1018: dangling `derives_from` links across the registered set.
    pub unresolved: Vec<(String, String)>,
    /// DS-1015: the document-level `derives_from` chain (document id ->
    /// parent document ids), resolved through node ownership — see
    /// `vtest_store::doc_registry::document_derives_from`'s doc comment for
    /// why this is not simply each document's raw `derives_from` (which
    /// holds node ids, not document ids).
    pub document_chain: std::collections::BTreeMap<String, Vec<String>>,
}

/// DS-1015/1016: `list` (optionally `--tree`/`--roots` — both are rendering
/// choices over the same full set, so this returns everything and leaves
/// the rendering axis to the caller, matching `ops::verify`'s own
/// data/rendering split).
pub fn list(layout: &VerifyLayout) -> Result<ListResult, DocOpError> {
    let map = read_all_docs(layout)?;
    let unresolved = unresolved_derives_from(&map);
    let document_chain = document_derives_from(&map);
    let records = map.into_values().collect();
    Ok(ListResult {
        records,
        unresolved,
        document_chain,
    })
}

/// DS-1017/1682 output: `view` carries id・path・content_hash・
/// derives_from（参照先ノード id の並びのみ、DS-1682）・根指定・鮮度
/// （`DocView.freshness`、常に`true` — その理由は`DocView::freshness`の
/// doc comment、および`reports/closure-trace.md`のstopped_on参照）。
///
/// `approval_states`（「実効承認状態」、node id → `draft`/`approved`）:
/// **この node id → 状態 の map という形自体、正本の直接引用ではない。**
/// DS-1017は「実効承認状態」を単数のものとして`doc show`の出力に挙げるが、
/// Approvalの`document` subject_typeはノード単位で束縛される（DS-1051）
/// 一方、正本には「登録document（複数ノードを持ちうる、DES-585/595）
/// 1件」を集約する単位が定義されていない。ノードごとのmapとして返す
/// 実装判断は、正本に無い集約規則をこのモジュールが発明したことになる
/// ため、`reports/closure-trace.md`のstopped_onに開示し、上流判断を
/// 仰いでいる。
pub struct ShowResult {
    pub view: DocView,
    pub approval_states: BTreeMap<String, String>,
}

pub fn show(layout: &VerifyLayout, id: &str) -> Result<ShowResult, DocOpError> {
    let view = read_doc_view(layout, id)
        .map_err(|_| DocOpError::Usage(format!("no document '{id}' is registered")))?;

    let doc_index = build_document_node_index(layout)?;
    let approvals = read_all_approvals(layout)?;
    let mut approval_states = BTreeMap::new();
    for node_id in document_node_ids(&view.file) {
        let Some((hash, _)) = doc_index.get(&node_id) else {
            continue;
        };
        let dependencies = document_dependencies(&doc_index, &node_id);
        let matching: Vec<_> = approvals
            .iter()
            .filter(|record| record.subject_type == "document" && record.subject == node_id)
            .cloned()
            .collect();
        let state = effective_approval_state(&matching, "document", &node_id, hash, &dependencies);
        let label = match state {
            EffectiveApprovalState::Draft => "draft",
            EffectiveApprovalState::Approved => "approved",
        };
        approval_states.insert(node_id, label.to_owned());
    }

    Ok(ShowResult {
        view,
        approval_states,
    })
}
