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
        build_document_node_index, document_dependencies, effective_approval_state, node_freshness,
        read_all_approvals, EffectiveApprovalState,
    },
    doc_registry::{
        apply_derives_from, apply_root, doc_exists, document_derives_from,
        document_top_level_node_ids, read_all_docs, read_doc_view, read_node_tree,
        unresolved_derives_from, write_doc, DocView,
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
    /// DS-1683「根指定は…新規登録時にのみ定まり…登録後は変更しない」/
    /// BD-331 (同旨、両ノードとも `233caec`/PR #50 で書き換え。旧 DS-1014
    /// 「`--update` は `--root`/`--no-root` を併せて根指定も更新できる」は
    /// この裁定と両立せず退役済み). Whether `--root`/`--no-root` was given
    /// at all (regardless of its value), independent of `root`'s own bool
    /// value -- needed only to detect and reject the forbidden combination
    /// with `update: true`; `root`'s value itself is never consulted when
    /// `update` is `true`.
    pub root_specified: bool,
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
    // DS-1683/BD-331 literally forbid *changing* the root designation
    // after registration -- they say nothing about rejecting a flag that,
    // by its own value, would be a no-op (`--no-root`, or MCP's explicit
    // `"root": false`, on `--update`, cannot actually change anything:
    // `apply_root(_, false)` is a no-op regardless). Rejecting the
    // presence of the flag at all here, rather than only rejecting when
    // it would actually attempt a change, is this module's own
    // fail-closed derivation, not DS-1683/BD-331's literal text -- team-
    // lead ruling 2026-09-10 (see `reports/closure-trace.md`'s stopped_on
    // history): flagged rather than silently narrowed.
    if args.update && args.root_specified {
        return Err(DocOpError::Usage(
            "--root/--no-root cannot be combined with --update: DS-1683/BD-331 fix the root \
             designation at initial registration only and forbid changing it afterward"
                .to_owned(),
        ));
    }

    let mut file = read_node_tree(project_root, &args.path)?;
    // DS-1683: root designation is never touched on --update (checked
    // above); apply_root only ever runs its assertion on initial
    // registration.
    if !args.update {
        apply_root(&file, args.root).map_err(|error| DocOpError::Usage(error.to_string()))?;
    }
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
    /// DS-1017 new / DS-1194: document id -> (top-level node id ->
    /// freshness), the same per-node computation `ShowResult.freshness`
    /// runs for a single document, run here for every registered document
    /// — DS-1194 names "鮮度" as part of `doc_list`'s own output, not just
    /// `doc show`'s, so this is not a coarser placeholder.
    pub freshness: std::collections::BTreeMap<String, BTreeMap<String, Option<bool>>>,
}

/// DS-1015/1016: `list` (optionally `--tree`/`--roots` — both are rendering
/// choices over the same full set, so this returns everything and leaves
/// the rendering axis to the caller, matching `ops::verify`'s own
/// data/rendering split).
pub fn list(layout: &VerifyLayout) -> Result<ListResult, DocOpError> {
    let map = read_all_docs(layout)?;
    let unresolved = unresolved_derives_from(&map);
    let document_chain = document_derives_from(&map);

    let doc_index = build_document_node_index(layout)?;
    let approvals = read_all_approvals(layout)?;
    let freshness = map
        .values()
        .map(|view| {
            let node_ids = document_top_level_node_ids(&view.file);
            (
                view.id.clone(),
                node_freshness(&doc_index, &approvals, &node_ids),
            )
        })
        .collect();

    let records = map.into_values().collect();
    Ok(ListResult {
        records,
        unresolved,
        document_chain,
        freshness,
    })
}

/// DS-1017/1682 output: `view` carries id・path・content_hash・
/// derives_from（参照先ノード id の並びのみ、DS-1682）・根指定。
///
/// `freshness`（DS-1017新: 「当該document subject hash〔DES-572〕と、
/// 当該documentをdependencyに含む承認・判断記録が保存したdependency
/// entryのhashとの一致（DS-862・DS-1601・DS-1605）」、`233caec`/PR #50で
/// 明文化）: node id → `Some(true)`（一致=鮮度あり）/`Some(false)`
/// （不一致=陳腐化）/`None`（この node をdependencyとして含む承認・判断
/// 記録が1件も無い＝比較対象なし、真偽に丸めない）。判断記録ドメインは
/// このコードベースに存在しないため、承認記録のみを対象とする（DS-1052
/// 同様の開示、`reports/closure-trace.md`参照）。
///
/// `approval_states`（「実効承認状態」、node id → `draft`/`approved`、
/// DS-1466）: DS-1017新は「各トップレベルノードの id を subject とする
/// 実効承認」と明文化した（`233caec`/PR #50）。node id → 状態 の map と
/// いう形は、この明文引用と一致する。
pub struct ShowResult {
    pub view: DocView,
    pub freshness: BTreeMap<String, Option<bool>>,
    pub approval_states: BTreeMap<String, String>,
}

pub fn show(layout: &VerifyLayout, id: &str) -> Result<ShowResult, DocOpError> {
    let view = read_doc_view(layout, id)
        .map_err(|_| DocOpError::Usage(format!("no document '{id}' is registered")))?;

    let doc_index = build_document_node_index(layout)?;
    let approvals = read_all_approvals(layout)?;
    let node_ids = document_top_level_node_ids(&view.file);
    let freshness = node_freshness(&doc_index, &approvals, &node_ids);

    let mut approval_states = BTreeMap::new();
    for node_id in &node_ids {
        let Some((hash, _)) = doc_index.get(node_id) else {
            continue;
        };
        let dependencies = document_dependencies(&doc_index, node_id);
        let matching: Vec<_> = approvals
            .iter()
            .filter(|record| record.subject_type == "document" && &record.subject == node_id)
            .cloned()
            .collect();
        let state = effective_approval_state(&matching, "document", node_id, hash, &dependencies);
        let label = match state {
            EffectiveApprovalState::Draft => "draft",
            EffectiveApprovalState::Approved => "approved",
        };
        approval_states.insert(node_id.clone(), label.to_owned());
    }

    Ok(ShowResult {
        view,
        freshness,
        approval_states,
    })
}
