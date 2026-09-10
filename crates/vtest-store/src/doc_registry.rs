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
/// `derives_from: None` means the argument was not given at all (register
/// the file's own content unchanged). `Some(&[])` means it *was* given,
/// with zero ids -- DS-1685's "既存の`derives_from`を指定した id 並びで
/// 置き換える" (replace, not append) means this clears every top-level
/// node's `derives_from` to empty, the same write path a non-empty list
/// takes, not a no-op. The two were previously conflated (an empty slice
/// treated identically to "not given"), which made a genuine
/// `--derives-from` (empty) request indistinguishable from omitting the
/// flag entirely (MCP JSON can express `"derives_from": []` distinctly
/// from an absent key; the CLI's repeatable-value flag shape cannot
/// express "given, zero times" the same way, a disclosed limitation of
/// that flag's own shape, not of this function).
pub fn apply_derives_from(
    file: &mut DocumentFile,
    derives_from: Option<&[String]>,
) -> Result<(), StoreError> {
    let Some(derives_from) = derives_from else {
        return Ok(());
    };
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
/// DS-1195 gives `doc_upsert`'s `root` a plain `bool` (not a three-valued
/// flag pair) — `true` requires `root[]` to be non-empty and every other
/// layer array to be empty (the assertion the previous `Some(true)` case
/// applied); `false` performs no check (the file's own layer assignment is
/// registered unchanged), matching this function's own `None` case before
/// this round's `Option<bool>` -> `bool` correction. The previous
/// `Some(false)` case's positive assertion ("root[] must be empty") is
/// folded into `false`'s no-op reading here — DS-1195 does not itself
/// define what `false` asserts, only that the field is a bool, so `--no
/// -root` and omitting the flag are now behaviourally identical on the
/// CLI (both map to `false`), simplifying rather than inventing a new
/// three-valued domain elsewhere.
pub fn apply_root(file: &DocumentFile, root: bool) -> Result<(), StoreError> {
    if !root {
        return Ok(());
    }
    let has_non_root = !file.request.is_empty()
        || !file.require.is_empty()
        || !file.spec.is_empty()
        || !file.detailed_spec.is_empty()
        || !file.basic_design.is_empty()
        || !file.design.is_empty();
    if file.root.is_empty() {
        Err(StoreError::InvalidConfig(
            "root: true was given but the source file's own root[] layer is empty; DS-1658 \
             ties a node's id prefix to the layer it may sit in, so this cannot move a \
             non-ROOT-prefixed node into root[] without fabricating it a new id -- the source \
             file must already carry ROOT-prefixed nodes in root[]"
                .to_owned(),
        ))
    } else if has_non_root {
        Err(StoreError::InvalidConfig(
            "root: true was given but the source file has content in a non-root layer array \
             as well as root[]; a document registered as root must be entirely root-layer \
             content"
                .to_owned(),
        ))
    } else {
        Ok(())
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
    // A node id that appears in more than one registered document's own
    // node tree is itself a distinct defect (E-SCAN-002-style collision,
    // this module's concern is display, not diagnosis) -- but this
    // function does not silently pick one owner over the other when it
    // happens: every document that owns a copy of the node id becomes a
    // parent, so the ambiguity is *shown* (the tree fans out to all
    // candidates), not resolved by an arbitrary first-write-wins choice.
    let mut owner: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for view in views.values() {
        let mut ids = std::collections::BTreeSet::new();
        collect_all_ids(&view.file, &mut ids);
        for id in ids {
            let owners = owner.entry(id).or_default();
            if !owners.contains(&view.id) {
                owners.push(view.id.clone());
            }
        }
    }
    let mut result = BTreeMap::new();
    for view in views.values() {
        let mut parents: Vec<String> = view
            .derives_from
            .iter()
            .filter_map(|target| owner.get(target).cloned())
            .flatten()
            .filter(|parent| parent != &view.id)
            .collect();
        parents.sort();
        parents.dedup();
        result.insert(view.id.clone(), parents);
    }
    result
}

/// Every node id (any of the seven layer arrays) a document file declares.
/// Exposed for DS-1017's "実効承認状態"／"鮮度" on `doc show`
/// (`233caec`/PR #50: "**各トップレベルノード**の id を subject とする実効
/// 承認"): Approval's `document` subject_type binds to individual node
/// ids (DS-1051), not to the registered file as a whole, so `ops::doc`
/// needs this set to compute one effective state / freshness per node the
/// document actually owns. **Top-level only** — a `SectionNode`'s own
/// `items`/`sections` children are not top-level nodes of the *document*,
/// they are children of that section; including them (as an earlier round
/// of this function did, reusing `collect_all_ids`'s deep walk meant for
/// a different purpose — resolving arbitrary `derives_from` targets
/// anywhere in the corpus, see `unresolved_derives_from`/
/// `document_derives_from` below) over-reported approval/freshness
/// coverage past what DS-1017 names.
pub fn document_top_level_node_ids(file: &DocumentFile) -> std::collections::BTreeSet<String> {
    let mut out = std::collections::BTreeSet::new();
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
        out.insert(section.id.as_str().to_owned());
    }
    out
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

    /// @vtest.id TEST-STORE-DOC-WRITE-READ-VIEW-FIELDS
    /// @vtest.covers VO-STORE-DOC-VIEW-IDENTITY-ROOT-DERIVES
    /// @vtest.target crates/vtest-store/src/doc_registry.rs::write_doc
    /// @vtest.target crates/vtest-store/src/doc_registry.rs::read_doc_view
    /// @vtest.intent 登録documentのid・root指定・derives_fromを読取りviewへ復元することを確認する
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

    /// @vtest.id TEST-STORE-DOC-NO-ROOT-LAYER-NOT-ROOT
    /// @vtest.covers VO-STORE-DOC-ROOT-MEMBERSHIP-FROM-LAYER
    /// @vtest.target crates/vtest-store/src/doc_registry.rs::read_doc_view
    /// @vtest.intent root層nodeを持たないdocumentをroot指定として扱わないことを確認する
    #[test]
    fn a_document_with_no_root_nodes_is_not_a_root() {
        let layout = temp_layout("no-root");
        let mut file = sample_file();
        file.root.clear();
        write_doc(&layout, "DOC-NO-ROOT", &file).unwrap();
        let view = read_doc_view(&layout, "DOC-NO-ROOT").unwrap();
        assert!(!view.is_root);
    }

    /// @vtest.id TEST-STORE-DOC-EXISTS-AFTER-REGISTRATION
    /// @vtest.covers VO-DOC-ADD-REGISTERS-NODE-TREE
    /// @vtest.target crates/vtest-store/src/doc_registry.rs::doc_exists
    /// @vtest.intent document JSON登録前後でdocumentの存在判定が反映されることを確認する
    #[test]
    fn doc_exists_reflects_the_written_file() {
        let layout = temp_layout("exists");
        assert!(!doc_exists(&layout, "DOC-BASIC-001"));
        write_doc(&layout, "DOC-BASIC-001", &sample_file()).unwrap();
        assert!(doc_exists(&layout, "DOC-BASIC-001"));
    }

    /// @vtest.id TEST-STORE-DOC-UNRESOLVED-DERIVES-FROM-DANGLING
    /// @vtest.covers VO-STORE-DOC-DANGLING-DERIVES-FROM
    /// @vtest.target crates/vtest-store/src/doc_registry.rs::unresolved_derives_from
    /// @vtest.intent 存在しない上流nodeへのderives_fromを文書鎖リンク切れとして列挙することを確認する
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

    /// A node id owned by more than one registered document (itself a
    /// separate defect this module does not diagnose) must not be silently
    /// resolved to a single first-write-wins owner in the document-level
    /// chain -- every owning document becomes a parent, so the ambiguity
    /// is shown rather than hidden.
    #[test]
    fn document_derives_from_lists_every_owner_when_a_node_id_is_duplicated() {
        let layout = temp_layout("duplicate-owner");
        let root_only = DocumentFile {
            schema_version: "0.1".to_owned(),
            root: vec![RootNode {
                id: DocumentId::new("ROOT-001"),
                statement: "fixture root".to_owned(),
                description: None,
                source: source(),
            }],
            request: Vec::new(),
            require: Vec::new(),
            spec: Vec::new(),
            detailed_spec: Vec::new(),
            basic_design: Vec::new(),
            design: Vec::new(),
        };
        // Two independently registered documents both happen to carry a
        // node with the same id (ROOT-001) -- an ownership collision.
        write_doc(&layout, "DOC-A", &root_only).unwrap();
        write_doc(&layout, "DOC-B", &root_only).unwrap();
        write_doc(&layout, "DOC-C", &sample_file()).unwrap(); // R-001 derives_from ROOT-001

        let views = read_all_docs(&layout).unwrap();
        let chain = document_derives_from(&views);
        assert_eq!(
            chain.get("DOC-C"),
            Some(&vec!["DOC-A".to_owned(), "DOC-B".to_owned()]),
            "DOC-C's derives_from target (ROOT-001) is owned by both DOC-A and DOC-B; both \
             must appear as parents, not just one"
        );
    }
}
