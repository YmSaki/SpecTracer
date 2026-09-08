//! Support for `vtest doc add/list/show` operating directly on the
//! `.verify/doc/<id>.json` node-tree files (本冊 §3.1, DES-585/586/595,
//! DS-1015〜1017/1681〜1684).
//!
//! An earlier version of this module persisted a separate
//! `.verify/doc/<id>.yaml` "registry record" (id/path/content_hash/
//! derives_from/root/registered_at). That shape has **no canonical
//! grounding**: DES-586 requires an upstream document's file to be a JSON
//! object matching `specification.json`'s own schema (the node-tree shape
//! `vtest_model::DocumentFile` already models) — nothing else. DES-585
//! states the file carries no identifying field at all ("上流文書のファイル
//! は、当該文書を識別する field を持たない") — the filename itself is the
//! identity, which is incompatible with a second record also claiming to
//! hold that identity. DES-595 says "1 document = 1 JSON ファイル" (singular
//! — one file, not a JSON file plus a companion record). No node under 本冊
//! §3.1 (DES-573/583/585/586) or DS-1681〜1684 describes a persisted
//! id/path/content_hash/derives_from/root registry shape. Team-lead
//! confirmed this reading (2026-09-09) after a citation search came back
//! empty and directed removal.
//!
//! Consequently, this module has no write path and no on-disk record type:
//! `id` is the file's own name (DES-585), `content_hash` is computed live
//! from the file's current content on every call (DES-595's document
//! subject hash, not stored), and "root designation" / "derives_from" are
//! derived by reading the file's own `root[]` array and its nodes' own
//! `derives_from` edges, not tracked separately. `--derives-from` (DS-1003/
//! 1681) mutates the node-tree file itself before it is written (see
//! `apply_derives_from`); `--root`/`--no-root` (DS-1683) validates the
//! source file's own layer placement rather than mutating it, since
//! DS-1658 ties a node's id prefix to its layer and this module does not
//! fabricate a new id to move a node between layers (see `apply_root`).

use std::{collections::BTreeMap, path::Path};

use vtest_model::{document_file_subject_hash, ContentHash, DocumentFile};

use crate::{canonical::document_file_from_json, records::read_text, StoreError, VerifyLayout};

/// A read-only, fully-derived view of one registered document — nothing
/// here is persisted separately from `.verify/doc/<id>.json` itself.
pub struct DocView {
    pub id: String,
    /// DS-1017: the document's current path — `.verify/doc/<id>.json`
    /// itself, the file's own canonical location, not the original
    /// `--path` argument (which is not stored anywhere, DES-585/595).
    pub path: std::path::PathBuf,
    pub content_hash: ContentHash,
    /// DS-1683: whether this document currently has any `root`-layer nodes
    /// (derived from the file's own `root[]` array, not a stored flag).
    pub is_root: bool,
    /// The union of every node's own `derives_from` edges across all seven
    /// layer arrays (DS-1681/1682's "参照先ノード id の並び" as observed
    /// directly in the node tree, not a separate document-level field).
    pub derives_from: Vec<String>,
    pub file: DocumentFile,
}

fn doc_path(layout: &VerifyLayout, id: &str) -> std::path::PathBuf {
    layout.doc_dir().join(format!("{id}.json"))
}

/// Reads and parses an arbitrary node-tree JSON file (used for `--path`,
/// which may point outside `.verify/doc/` when registering a document that
/// isn't there yet).
pub fn read_node_tree(
    project_root: &Path,
    relative_path: &str,
) -> Result<DocumentFile, StoreError> {
    let absolute = project_root.join(relative_path);
    let text = read_text(&absolute)?;
    document_file_from_json(&text)
}

/// DS-1003「`--derives-from` は、登録する document のトップレベルノードが
/// 持つ上流ノード id への辺（0件可＝根候補）であり、`document` という単位
/// そのものに対する導出リンクではない」/ DS-1681（anchor/note を持たない
/// 上流ノード id への辺）/ DS-1685「対象トップレベルノードの既存
/// `derives_from` を指定した id 並びで置き換える。追記ではない」/
/// DS-1686「登録する document が持つ `request`/`require`/`spec`/
/// `detailed_spec`/`basic_design`/`design` いずれかの層のトップレベル
/// ノードすべてへ、同一の id 並びを一律に適用する」/ DS-1687「トップレベル
/// ノードが `root` 層のみで構成される場合、`root` 層ノードは
/// `derives_from` field を持たない（DS-1592/1593）ため登録を拒否する」
/// （PR #49, commit `24c3cbe` で明文化。以前は導出として開示していた）。
pub fn apply_derives_from(
    file: &mut DocumentFile,
    derives_from: &[String],
) -> Result<(), StoreError> {
    if derives_from.is_empty() {
        return Ok(());
    }
    let ids: Vec<vtest_model::DocumentId> = derives_from
        .iter()
        .map(|id| vtest_model::DocumentId::new(id.clone()))
        .collect();
    let mut touched = false;
    for node in &mut file.request {
        node.derives_from = ids.clone();
        touched = true;
    }
    for section in file
        .require
        .iter_mut()
        .chain(&mut file.spec)
        .chain(&mut file.detailed_spec)
        .chain(&mut file.basic_design)
        .chain(&mut file.design)
    {
        section.derives_from = Some(ids.clone());
        touched = true;
    }
    if !touched {
        return Err(StoreError::InvalidConfig(
            "--derives-from has no top-level node to attach to: the document has no \
             request/require/spec/detailed_spec/basic_design/design entries (root-layer \
             nodes have no derives_from field, DS-1592/1593)"
                .to_owned(),
        ));
    }
    Ok(())
}

/// DS-1683: root-ness is the fact of *which layer array a node sits in*
/// ("根指定は登録先の層配列という形でノード自体に属し"), and DS-1658 ties a
/// node's id prefix to the layer it may legally sit in
/// (`validate_node_id`: "id's prefix does not match the layer array it was
/// placed in" is a hard write-time rejection). A `ROOT-…`-prefixed node can
/// only ever sit in `root[]`; any other prefix can only sit in its own
/// non-root layer. `--root`/`--no-root` therefore cannot *move* a node
/// between layers without inventing it a new id (fabricating an identity
/// this module refuses to do); the flag instead **validates** that the
/// source file's own layer placement (already tied to the ids the
/// registrant chose) matches what was requested, rejecting a mismatch
/// rather than silently accepting or silently converting it.
///
/// `Some(true)` (`--root`) requires `root[]` to be non-empty and every
/// other layer array to be empty. `Some(false)` (`--no-root`) requires
/// `root[]` to be empty. `None` (neither flag given) performs no check —
/// the file's own layer assignment is registered unchanged, matching this
/// module's behavior before this flag existed.
pub fn apply_root(file: &DocumentFile, requested_root: Option<bool>) -> Result<(), StoreError> {
    let has_non_root = !file.request.is_empty()
        || !file.require.is_empty()
        || !file.spec.is_empty()
        || !file.detailed_spec.is_empty()
        || !file.basic_design.is_empty()
        || !file.design.is_empty();
    match requested_root {
        None => Ok(()),
        Some(true) => {
            if file.root.is_empty() {
                Err(StoreError::InvalidConfig(
                    "--root was given but the source file's own root[] layer is empty; \
                     DS-1658 ties a node's id prefix to the layer it may sit in, so --root \
                     cannot move a non-ROOT-prefixed node into root[] without fabricating it \
                     a new id -- the source file must already carry ROOT-prefixed nodes in \
                     root[]"
                        .to_owned(),
                ))
            } else if has_non_root {
                Err(StoreError::InvalidConfig(
                    "--root was given but the source file has content in a non-root layer \
                     array as well as root[]; a document registered under --root must be \
                     entirely root-layer content"
                        .to_owned(),
                ))
            } else {
                Ok(())
            }
        }
        Some(false) => {
            if file.root.is_empty() {
                Ok(())
            } else {
                Err(StoreError::InvalidConfig(
                    "--no-root was given but the source file's own root[] layer is \
                     non-empty; DS-1658 ties a node's id prefix to the layer it may sit in, \
                     so --no-root cannot move a ROOT-prefixed node out of root[] without \
                     fabricating it a new id"
                        .to_owned(),
                ))
            }
        }
    }
}

fn derives_from_of(file: &DocumentFile) -> Vec<String> {
    let mut out = Vec::new();
    for node in &file.request {
        out.extend(node.derives_from.iter().map(|id| id.as_str().to_owned()));
    }
    for section in file
        .require
        .iter()
        .chain(&file.spec)
        .chain(&file.detailed_spec)
        .chain(&file.basic_design)
        .chain(&file.design)
    {
        collect_section_derives_from(section, &mut out);
    }
    out.sort();
    out.dedup();
    out
}

fn collect_section_derives_from(section: &vtest_model::SectionNode, out: &mut Vec<String>) {
    if let Some(derives_from) = &section.derives_from {
        out.extend(derives_from.iter().map(|id| id.as_str().to_owned()));
    }
    for item in section.items.as_deref().unwrap_or(&[]) {
        out.extend(item.derives_from.iter().map(|id| id.as_str().to_owned()));
    }
    for child in section.sections.as_deref().unwrap_or(&[]) {
        collect_section_derives_from(child, out);
    }
}

fn to_view(id: &str, path: std::path::PathBuf, file: DocumentFile) -> DocView {
    let content_hash = document_file_subject_hash(&file);
    let is_root = !file.root.is_empty();
    let derives_from = derives_from_of(&file);
    DocView {
        id: id.to_owned(),
        path,
        content_hash,
        is_root,
        derives_from,
        file,
    }
}

/// Reads `.verify/doc/<id>.json` and derives its view (DES-595/DS-1017).
pub fn read_doc_view(layout: &VerifyLayout, id: &str) -> Result<DocView, StoreError> {
    let path = doc_path(layout, id);
    let text = read_text(&path)?;
    let file = document_file_from_json(&text)?;
    Ok(to_view(id, path, file))
}

/// Writes `file` to `.verify/doc/<id>.json` — DES-595's "1 document = 1
/// JSON ファイル" registration act itself: this canonical location is the
/// only persisted state a registered document has.
pub fn write_doc(layout: &VerifyLayout, id: &str, file: &DocumentFile) -> Result<(), StoreError> {
    let json = crate::canonical::document_file_to_json(file)?;
    let path = doc_path(layout, id);
    crate::records::write_atomic(&path, &json)
}

pub fn doc_exists(layout: &VerifyLayout, id: &str) -> bool {
    doc_path(layout, id).exists()
}

/// Every `.verify/doc/<id>.json` node-tree file, keyed by id (the file
/// stem, DES-585), with its derived view.
pub fn read_all_docs(layout: &VerifyLayout) -> Result<BTreeMap<String, DocView>, StoreError> {
    let listing = crate::read_document_dir(&layout.doc_dir())?;
    let mut views = BTreeMap::new();
    for name in listing.names {
        let view = read_doc_view(layout, &name)?;
        views.insert(name, view);
    }
    Ok(views)
}

/// DS-1018: a `derives_from` edge that does not resolve to any node id
/// present across the full registered set is a dangling link. This checks
/// against every node id in every registered document (not just top-level
/// document ids), since DS-1681's edges name upstream *node* ids, not
/// document ids.
pub fn unresolved_derives_from(views: &BTreeMap<String, DocView>) -> Vec<(String, String)> {
    let mut known_ids = std::collections::BTreeSet::new();
    for view in views.values() {
        collect_all_ids(&view.file, &mut known_ids);
    }
    let mut unresolved = Vec::new();
    for view in views.values() {
        for target in &view.derives_from {
            if !known_ids.contains(target) {
                unresolved.push((view.id.clone(), target.clone()));
            }
        }
    }
    unresolved
}

/// DS-1015: `doc list --tree` renders the `derives_from` *document* chain.
/// `DocView::derives_from` holds *node* ids (DS-1681/1682), and a document
/// may hold many nodes (DES-585/595 — the file, not any single node, is the
/// identified unit), so the document-level parent of `doc` is whichever
/// registered document *contains* the node id `doc` derives from, not the
/// literal string. This resolves that node-id -> owning-document-id
/// mapping across the whole registered set and returns, for each document
/// id, the deduplicated set of parent document ids; a `derives_from` target
/// with no owning document in the registered set (see
/// [`unresolved_derives_from`]) is silently dropped here, not treated as a
/// parent.
pub fn document_derives_from(views: &BTreeMap<String, DocView>) -> BTreeMap<String, Vec<String>> {
    let mut owner: BTreeMap<String, String> = BTreeMap::new();
    for view in views.values() {
        let mut ids = std::collections::BTreeSet::new();
        collect_all_ids(&view.file, &mut ids);
        for id in ids {
            owner.entry(id).or_insert_with(|| view.id.clone());
        }
    }
    let mut result = BTreeMap::new();
    for view in views.values() {
        let mut parents: Vec<String> = view
            .derives_from
            .iter()
            .filter_map(|target| owner.get(target).cloned())
            .filter(|parent| parent != &view.id)
            .collect();
        parents.sort();
        parents.dedup();
        result.insert(view.id.clone(), parents);
    }
    result
}

fn collect_all_ids(file: &DocumentFile, out: &mut std::collections::BTreeSet<String>) {
    for node in &file.root {
        out.insert(node.id.as_str().to_owned());
    }
    for node in &file.request {
        out.insert(node.id.as_str().to_owned());
    }
    for section in file
        .require
        .iter()
        .chain(&file.spec)
        .chain(&file.detailed_spec)
        .chain(&file.basic_design)
        .chain(&file.design)
    {
        collect_section_ids(section, out);
    }
}

fn collect_section_ids(
    section: &vtest_model::SectionNode,
    out: &mut std::collections::BTreeSet<String>,
) {
    out.insert(section.id.as_str().to_owned());
    for item in section.items.as_deref().unwrap_or(&[]) {
        out.insert(item.id.as_str().to_owned());
    }
    for child in section.sections.as_deref().unwrap_or(&[]) {
        collect_section_ids(child, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vtest_model::{DocumentId, NodeSource, RootNode, SentenceNode};

    fn temp_layout(name: &str) -> VerifyLayout {
        let root = std::env::temp_dir().join(format!(
            "vtest-store-doc-registry-{name}-{}",
            crate::records::new_record_id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        crate::init_project(&root, "fixture").unwrap();
        VerifyLayout::new(&root)
    }

    fn source() -> NodeSource {
        NodeSource {
            doc: "fixture.md".to_owned(),
            heading: "fixture".to_owned(),
            lines: [1, 1],
        }
    }

    fn sample_file() -> DocumentFile {
        DocumentFile {
            schema_version: "0.1".to_owned(),
            root: vec![RootNode {
                id: DocumentId::new("ROOT-001"),
                statement: "fixture root".to_owned(),
                description: None,
                source: source(),
            }],
            request: vec![SentenceNode {
                id: DocumentId::new("R-001"),
                statement: "fixture requirement".to_owned(),
                description: None,
                derives_from: vec![DocumentId::new("ROOT-001")],
                cites: None,
                source: source(),
            }],
            require: Vec::new(),
            spec: Vec::new(),
            detailed_spec: Vec::new(),
            basic_design: Vec::new(),
            design: Vec::new(),
        }
    }

    #[test]
    fn write_then_read_round_trips_and_derives_root_and_derives_from() {
        let layout = temp_layout("write-read");
        write_doc(&layout, "DOC-BASIC-001", &sample_file()).unwrap();
        let view = read_doc_view(&layout, "DOC-BASIC-001").unwrap();
        assert_eq!(view.id, "DOC-BASIC-001");
        assert!(
            view.is_root,
            "the fixture file has a non-empty root[] array"
        );
        assert_eq!(view.derives_from, vec!["ROOT-001".to_owned()]);
    }

    #[test]
    fn a_document_with_no_root_nodes_is_not_a_root() {
        let layout = temp_layout("no-root");
        let mut file = sample_file();
        file.root.clear();
        write_doc(&layout, "DOC-NO-ROOT", &file).unwrap();
        let view = read_doc_view(&layout, "DOC-NO-ROOT").unwrap();
        assert!(!view.is_root);
    }

    #[test]
    fn doc_exists_reflects_the_written_file() {
        let layout = temp_layout("exists");
        assert!(!doc_exists(&layout, "DOC-BASIC-001"));
        write_doc(&layout, "DOC-BASIC-001", &sample_file()).unwrap();
        assert!(doc_exists(&layout, "DOC-BASIC-001"));
    }

    #[test]
    fn unresolved_derives_from_reports_a_dangling_node_reference() {
        let layout = temp_layout("unresolved");
        write_doc(&layout, "DOC-BASIC-001", &sample_file()).unwrap();
        let views = read_all_docs(&layout).unwrap();
        // sample_file's R-001 derives_from ROOT-001, which is present -- no
        // dangling link yet.
        assert!(unresolved_derives_from(&views).is_empty());

        let mut file = sample_file();
        file.request[0].derives_from = vec![DocumentId::new("ROOT-999")];
        write_doc(&layout, "DOC-BASIC-001", &file).unwrap();
        let views = read_all_docs(&layout).unwrap();
        assert_eq!(
            unresolved_derives_from(&views),
            vec![("DOC-BASIC-001".to_owned(), "ROOT-999".to_owned())]
        );
    }
}
