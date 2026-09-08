//! The coarse Document registry entity `vtest doc add/list/show` manages
//! (本冊 §12.2 "`vtest doc add / list / show`", DS-1000〜DS-1018, DES-482,
//! BD-072).
//!
//! This is a distinct entity from [`crate::DocumentFile`] (the fine node-tree
//! content of one upstream document — root/request/require/spec/
//! detailed_spec/basic_design/design arrays of individually addressable
//! `ROOT-`/`R-`/`SPEC-`/`DS-`/etc. nodes, `.verify/doc/<name>.json`). DES-482
//! only asks `doc add` to bind a plain sha256 of the `--path` file to a
//! registry record — it does not parse or generate node-tree content — so a
//! [`DocRegistryRecord`] tracks one registered source document at the whole-
//! file level: its id, its source path, that file's content hash, which
//! other registered documents it derives from (with a per-link anchor/note,
//! DS-1004/1005 — distinct from [`crate::DerivesFrom`], which is the VO
//! record's own, differently-shaped `derives_from`), and whether it is an
//! orphan_detection root (DS-1010/1011).
//!
//! Storage: `.verify/doc/<id>.yaml`, alongside (not instead of) the existing
//! `.verify/doc/<name>.json` node-tree files — BD-319/320 "上流文書のファイル
//! 形式は JSON とし、その他のレコードのファイル形式はすべて YAML とする"
//! reads naturally as drawing the JSON/YAML line at "is this the upstream
//! document's own node content, or a record about a document" — a registry
//! record is the latter. The extension keeps the two file kinds from
//! colliding in the same directory.

use serde::{Deserialize, Serialize};

/// One `--derives-from <doc-id> [--anchor <text>] [--note <text>]` entry
/// (DS-1003/1004/1005/1006/1007): a link to another registered document
/// (`--derives-from`, 0 or more; empty means this document is a root
/// candidate, DS-1003), with an optional per-link anchor (an opaque string
/// naming where in the target document this link points — DS-1007 forbids
/// resolving/validating it) and an optional per-link note (a free-text
/// derivation rationale).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DocRegistryDerivesFrom {
    pub doc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// A registered document record (`.verify/doc/<id>.yaml`).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocRegistryRecord {
    pub id: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub content_hash: crate::ContentHash,
    #[serde(default)]
    pub derives_from: Vec<DocRegistryDerivesFrom>,
    /// DS-1010/1011: whether this document is an explicit orphan_detection
    /// root (`--root`/`--no-root`).
    pub root: bool,
    pub registered_at: String,
}
