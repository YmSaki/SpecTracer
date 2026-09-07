use crate::{ContentHash, SrcId};
use serde::{Deserialize, Serialize};

// TODO/refactor:
// Locator is domain-neutral in concept but currently owns Rust-specific parsing.
// Split into SourceLocator + Rust-specific locator/parser.

/// Identifies a source construct by its source file path and item path.
///
/// A locator consists of the source file path and the construct's item path
/// within that file.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Locator {
    pub path: String,
    pub item_path: String,
}

impl Locator {
    pub fn parse(value: &str) -> Option<Self> {
        let separator = value.find("::")?;
        let (path, item_path) = value.split_at(separator);
        let item_path = item_path.strip_prefix("::")?;
        if path.is_empty() || item_path.is_empty() || !path.ends_with(".rs") {
            return None;
        }
        Some(Self {
            path: path.replace('\\', "/"),
            item_path: item_path.to_owned(),
        })
    }

    pub fn as_string(&self) -> String {
        format!("{}::{}", self.path, self.item_path)
    }
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

    /// @vtest.id TEST-MODEL-SOURCE-LOCATOR-PARSE
    /// @vtest.covers VO-MODEL-SOURCE-LOCATOR-PARSE
    /// @vtest.target crates/vtest-model/src/source.rs::Locator::parse
    /// @vtest.intent verifies that a source locator splits at the first separator
    #[test]
    fn locator_splits_at_first_separator() {
        let locator = Locator::parse("src/lib.rs::module::function").expect("valid locator");
        assert_eq!(locator.path, "src/lib.rs");
        assert_eq!(locator.item_path, "module::function");
    }
}
