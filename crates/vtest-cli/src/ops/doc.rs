//! `vtest doc add|list|show`: registers/inspects `.verify/doc/<id>.json`
//! node-tree files directly (本冊 §3.1, DES-585/586/595, DS-1015-1017/
//! 1681-1684). See `vtest_store::doc_registry`'s module doc comment for why
//! there is no separate persisted registry record — a prior version of this
//! module invented one (`.verify/doc/<id>.yaml`), found to have no
//! canonical grounding, and removed (team-lead ruling 2026-09-09; see
//! `reports/closure-trace.md`).

use vtest_store::{
    doc_registry::{
        doc_exists, read_all_docs, read_doc_view, read_node_tree, unresolved_derives_from,
        write_doc, DocView,
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

    let file = read_node_tree(project_root, &args.path)?;
    write_doc(layout, &args.id, &file)?;
    Ok(read_doc_view(layout, &args.id)?)
}

pub struct ListResult {
    pub records: Vec<DocView>,
    /// DS-1018: dangling `derives_from` links across the registered set.
    pub unresolved: Vec<(String, String)>,
}

/// DS-1015/1016: `list` (optionally `--tree`/`--roots` — both are rendering
/// choices over the same full set, so this returns everything and leaves
/// the rendering axis to the caller, matching `ops::verify`'s own
/// data/rendering split).
pub fn list(layout: &VerifyLayout) -> Result<ListResult, DocOpError> {
    let map = read_all_docs(layout)?;
    let unresolved = unresolved_derives_from(&map);
    let records = map.into_values().collect();
    Ok(ListResult {
        records,
        unresolved,
    })
}

/// DS-1017/1682: returns id・content_hash（都度計算）・derives_from（参照先
/// ノード id の並びのみ、DS-1682）・根指定（`root[]` の有無から導出）。DS-1017
/// の「鮮度（content_hash と実ファイルの一致）」は実装していない —
/// `.verify/doc/<id>.json` 自体が正典の内容であり、比較対象となる「別の
/// 実ファイル」が正本のどこにも定義されていないため（stopped_on 参照）。
/// 実効承認状態は `ops::approval::show` の対象種別 `document` 経由で別途
/// 取得できる（BD-306「対象種別ごとに別の承認規則・別の承認コマンドを
/// 設けない」により、`doc show` 側で二重に計算しない）。
pub fn show(layout: &VerifyLayout, id: &str) -> Result<DocView, DocOpError> {
    read_doc_view(layout, id)
        .map_err(|_| DocOpError::Usage(format!("no document '{id}' is registered")))
}
