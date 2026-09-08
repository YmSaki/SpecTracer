//! The Document registry entity `vtest doc add/list/show` manages (本冊
//! §12.2 "`vtest doc add / list / show`", DS-1015〜1017/1681〜1684,
//! DES-595, BD-072/331).
//!
//! Realigned to the node-tree model (PR #47, canon commit `757fdcc`,
//! superseding the earlier DS-1000〜1018/DES-482 coarse-registry reading
//! this module originally implemented — see `reports/closure-trace.md` for
//! that history). `--path` now names an *already-built*
//! `.verify/doc/<name>.json` node-tree file (the same file shape
//! [`crate::DocumentFile`] already models) — "1 document = 1 JSON ファイル"
//! (DES-595) — not a raw markdown source. A [`DocRegistryRecord`] is a
//! lightweight pointer/metadata record about one such file: its id, the
//! path to the JSON file, that file's document-level subject hash
//! ([`crate::document_file_subject_hash`], not a plain sha256 of the file's
//! bytes — DES-595 explicitly rules that out), which other registered
//! documents it derives from (DS-1681: a bare list of upstream document
//! ids, no per-link anchor/note — unlike the VO record's own
//! [`crate::DerivesFrom`], which does carry anchor/note), and whether it
//! registers as an orphan_detection root (DS-1683).
//!
//! Storage: `.verify/doc/<id>.yaml`, alongside (not instead of) the
//! `.verify/doc/<name>.json` node-tree files it points at — disjoint
//! extensions keep the two file kinds from colliding in the same
//! directory (BD-319/320's JSON/upstream-document vs YAML/other-record
//! split).

use serde::{Deserialize, Serialize};

/// A registered document record (`.verify/doc/<id>.yaml`).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocRegistryRecord {
    pub id: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub content_hash: crate::ContentHash,
    /// DS-1681/1682: a bare list of upstream document ids — no per-link
    /// anchor/note (that shape belongs to the VO record's own
    /// `derives_from`, [`crate::DerivesFrom`], not this one).
    #[serde(default)]
    pub derives_from: Vec<String>,
    /// DS-1683: whether `--root` was given at registration (this
    /// document's top-level nodes belong in the `root` layer array).
    pub root: bool,
    pub registered_at: String,
}
