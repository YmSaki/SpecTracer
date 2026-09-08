//! `vtest doc add|list|show`: the Document registry entity (本冊 §12.2,
//! DS-1015〜1017/1681〜1684, DES-595, BD-072/331). See
//! `vtest_model::doc_registry`'s module doc comment for what this entity is
//! and the realignment (canon commit `757fdcc`) it now follows.

use vtest_model::DocRegistryRecord;
use vtest_store::{
    hash_doc_registry_path, read_all_doc_registry, read_doc_registry_record,
    unresolved_derives_from, write_doc_registry_record, StoreError, VerifyLayout,
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
    pub title: Option<String>,
    /// DS-1681: a bare list of upstream document ids (no per-link
    /// anchor/note).
    pub derives_from: Vec<String>,
    /// `None` = neither `--root` nor `--no-root` given (DS-1683: keep
    /// the current value on `--update`, default `false` on a fresh `add`).
    pub root: Option<bool>,
    pub update: bool,
}

/// DS-1683/1684/DES-595: `add` creates a new record (rejecting a
/// duplicate id unless `--update`); `--update` recomputes `content_hash`
/// from the current `--path` node-tree JSON file and may combine with
/// `--root`/`--no-root`.
pub fn add(
    project_root: &std::path::Path,
    layout: &VerifyLayout,
    args: AddArgs,
) -> Result<DocRegistryRecord, DocOpError> {
    let existing = read_doc_registry_record(layout, &args.id).ok();
    if existing.is_some() && !args.update {
        return Err(DocOpError::Usage(format!(
            "document '{}' is already registered; use --update to re-register it",
            args.id
        )));
    }
    if existing.is_none() && args.update {
        return Err(DocOpError::Usage(format!(
            "--update given but no document '{}' is registered yet",
            args.id
        )));
    }

    let content_hash = hash_doc_registry_path(project_root, &args.path)?;
    let derives_from = args.derives_from;
    let root = args
        .root
        .unwrap_or_else(|| existing.as_ref().is_some_and(|record| record.root));
    let title = args
        .title
        .or_else(|| existing.and_then(|record| record.title));

    let record = DocRegistryRecord {
        id: args.id,
        path: args.path,
        title,
        content_hash,
        derives_from,
        root,
        registered_at: vtest_store::records::now_rfc3339(),
    };
    write_doc_registry_record(layout, &record)?;
    Ok(record)
}

pub struct ListResult {
    pub records: Vec<DocRegistryRecord>,
    /// DS-1018: registry-level dangling `derives_from` links.
    pub unresolved: Vec<(String, String)>,
}

/// DS-1015/1016: `list` (optionally `--tree`/`--roots` — both are rendering
/// choices over the same full record set, so this returns everything and
/// leaves the rendering axis to the caller, matching `ops::verify`'s own
/// data/rendering split).
pub fn list(layout: &VerifyLayout) -> Result<ListResult, DocOpError> {
    let map = read_all_doc_registry(layout)?;
    let unresolved = unresolved_derives_from(&map)
        .into_iter()
        .map(|(from, to)| (from.to_owned(), to.to_owned()))
        .collect();
    let records = map.into_values().collect();
    Ok(ListResult {
        records,
        unresolved,
    })
}

/// DS-1017/1682: `path`・`content_hash`・`derives_from`（参照先ノード id の
/// 並びのみ、DS-1682 — anchor は持たない）・根指定・鮮度（`content_hash` と
/// 実ファイルの一致）を返す。実効
/// 承認状態は `ops::approval::show` の対象種別 `document` 経由で別途取得
/// できる（この closure-slice では `vo`/`document` の実効承認は `vtest
/// approval show` の責務であり、`doc show` 側で二重に計算しない — BD-306
/// 「対象種別ごとに別の承認規則・別の承認コマンドを設けない」の裏を返す
/// と、`doc show` が独自に承認状態を再計算するのは対象種別ごとに承認経路
/// を増やすことになる）。
pub struct ShowResult {
    pub record: DocRegistryRecord,
    /// `true` when `content_hash` still matches the current `--path` file
    /// (DS-1017's freshness field). `Err` when the file cannot currently be
    /// read (moved/deleted since registration) — reported, not silently
    /// treated as stale or fresh.
    pub fresh: Result<bool, String>,
}

pub fn show(
    project_root: &std::path::Path,
    layout: &VerifyLayout,
    id: &str,
) -> Result<ShowResult, DocOpError> {
    let record = read_doc_registry_record(layout, id)
        .map_err(|_| DocOpError::Usage(format!("no document '{id}' is registered")))?;
    let fresh = match hash_doc_registry_path(project_root, &record.path) {
        Ok(current) => Ok(current == record.content_hash),
        Err(error) => Err(error.to_string()),
    };
    Ok(ShowResult { record, fresh })
}
