use crate::{AdapterId, ContentHash, SrcId};
use serde::{Deserialize, Serialize};

/// Identifies a source construct by an adapter ID and an adapter-owned
/// opaque locator value (詳細設計 v0.1 本冊:632-635「`Locator { adapter:
/// AdapterId, value: String }`」、本冊:522「`value`はadapter所有のopaque
/// 文字列である。coreがpath、module、symbol種別を分解しない」)。
///
/// `value`'s internal syntax (path separators, module paths, symbol names,
/// file extensions, ...) is owned entirely by the adapter named in
/// `adapter`. `vtest-model` and `vtest-scan` (core) compare `value` only for
/// exact equality and never parse or decompose it — the `rust-cargo`
/// adapter (`vtest-adapter-rust`) owns the one concrete syntax that exists
/// today (`pr3-decisions.md` Owner裁定2「core は adapter を registry で
/// 引いて resolution を委譲する / rust-cargo が Rust locator の解析を所有
/// する」).
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Locator {
    pub adapter: AdapterId,
    pub value: String,
}

/// References a source-level verification target.
///
/// A target can be identified either by its source locator or by a registered
/// source identifier.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum TargetRef {
    /// Identifies the target by its location in source code.
    Locator(Locator),
    /// Identifies the target by its registered source identifier.
    SrcId(SrcId),
}

/// Project-relative path to a source file (詳細設計 v0.1 本冊:639,
/// `SourceLocation.path: ProjectPath`).
///
/// The spec references this type by name as a `SourceLocation` field type
/// but defines no struct body for it anywhere in the four canonical spec
/// files (grep for `struct ProjectPath` / `type ProjectPath` finds nothing;
/// see `hash27-model-spec.md` §7) — its shape is implementation discretion.
/// This crate represents it as an opaque, forward-slash-normalized,
/// project-relative path string, the same normalization the `rust-cargo`
/// adapter already applied to the old `SourceLocation.file` field. Core does
/// not parse or validate its internal syntax (本冊:521-522 "coreはpath、
/// module、symbol種別を分解しない" — the same "core treats adapter-owned
/// strings as opaque" rule this crate already applies to `Locator.value`).
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectPath(pub String);

impl ProjectPath {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Half-open byte range `[start, end)` within a source file (詳細設計 v0.1
/// 本冊:641, `SourceLocation.byte_range: SourceRange`).
///
/// Same discretion note as [`ProjectPath`]: the spec references this type by
/// name but defines no struct body anywhere in the four canonical spec files
/// (grep for `struct SourceRange` finds nothing; see `hash27-model-spec.md`
/// §2). This crate keeps the zero-based, half-open `[start, end)` convention
/// the old `SourceLocation.start_byte`/`end_byte` fields already used.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct SourceRange {
    pub start: u64,
    pub end: u64,
}

/// Physical location of a source construct (詳細設計 v0.1 本冊:637-642、
/// §5.2、逐語):
///
/// ```text
/// pub struct SourceLocation {
///     pub adapter: AdapterId,
///     pub path: ProjectPath,
///     pub locator: String,            // adapter所有のopaque construct locator
///     pub byte_range: SourceRange,
/// }
/// ```
///
/// `locator` is the adapter-owned opaque construct locator (別紙C:313 —
/// distinct from `path`, which already carries the file; for `rust-cargo`
/// this is the item path portion only, e.g. `"module::function"`, matching
/// what the old `function` field held). Line numbers (`start_line`/
/// `end_line`) are not part of this shape — the spec's own struct has none
/// (only `byte_range: SourceRange`). See the PR report for what carrying
/// line numbers only in this crate would have cost, and why this crate does
/// not reintroduce them here.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub adapter: AdapterId,
    pub path: ProjectPath,
    pub locator: String,
    pub byte_range: SourceRange,
}

/// Represents a discovered source function and its identity information.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceFunction {
    pub locator: Locator,
    pub src_id: Option<SrcId>,
    pub location: SourceLocation,
    pub content_hash: ContentHash,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 本冊:522「`value`はadapter所有のopaque文字列である。coreがpath、
    /// module、symbol種別を分解しない」: `vtest-model` の `Locator` は
    /// `adapter` と `value` の完全一致だけで比較する。内部構文の parse・
    /// 正規化（`RustLocator`）は `vtest-adapter-rust` 側へ移った
    /// （`pr3-decisions.md` Owner裁定2「rust-cargo が Rust locator の解析
    /// を所有する」）。
    #[test]
    fn locators_compare_by_adapter_and_opaque_value_only() {
        let left = Locator {
            adapter: AdapterId::new("rust-cargo"),
            value: "src/lib.rs::module::function".to_owned(),
        };
        let same = Locator {
            adapter: AdapterId::new("rust-cargo"),
            value: "src/lib.rs::module::function".to_owned(),
        };
        let different_value = Locator {
            adapter: AdapterId::new("rust-cargo"),
            value: "src/lib.rs::module::other".to_owned(),
        };
        let different_adapter = Locator {
            adapter: AdapterId::new("other-lang"),
            value: "src/lib.rs::module::function".to_owned(),
        };
        assert_eq!(left, same);
        assert_ne!(left, different_value);
        assert_ne!(left, different_adapter);
    }
}
