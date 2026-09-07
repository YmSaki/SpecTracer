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

/// Physical location of a source construct.
///
/// Line numbers are 1-based.
/// Byte offsets are zero-based and use a half-open `[start, end)` range.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: String,
    pub function: String,
    pub start_line: u64,
    pub end_line: u64,
    pub start_byte: u64,
    pub end_byte: u64,
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
