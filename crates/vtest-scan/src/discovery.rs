//! Rust-specific source-location helpers extracted from the scanner. These move
//! to `vtest-adapter-rust` together with the rest of the Rust discovery code
//! (the scanner, Cargo manifest and module resolution, annotation parsing);
//! grouping the language-specific surface into one module isolates it first so
//! the cross-crate move is mechanical.

use std::collections::BTreeMap;

use syn::{Attribute, Expr, ExprLit, Lit, Meta};
use vtest_model::{AdapterId, ProjectPath, SourceLocation, SourceRange, SrcId, TargetRef};

use crate::RUST_ADAPTER_ID;

#[derive(Clone, Debug, Eq, PartialEq)]
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

    pub fn as_target(&self) -> TargetRef {
        TargetRef::Locator {
            adapter: AdapterId::new(RUST_ADAPTER_ID),
            value: self.as_string(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum TestTarget {
    Lib,
    Bin(String),
    IntegrationTest(String),
    Unknown,
}

pub(crate) struct ParsedAnnotations {
    pub(crate) values: BTreeMap<String, String>,
    pub(crate) repeated: BTreeMap<String, Vec<String>>,
}

pub(crate) fn parse_annotations(attrs: &[Attribute]) -> Option<ParsedAnnotations> {
    let mut lines = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("doc") {
            continue;
        }
        let Meta::NameValue(value) = &attr.meta else {
            continue;
        };
        let Expr::Lit(ExprLit {
            lit: Lit::Str(text),
            ..
        }) = &value.value
        else {
            continue;
        };
        lines.extend(text.value().lines().map(|line| line.trim().to_owned()));
    }
    if !lines.iter().any(|line| line.contains("@vtest.")) {
        return None;
    }
    let mut values = BTreeMap::new();
    let mut repeated = BTreeMap::<String, Vec<String>>::new();
    const KNOWN: &[&str] = &[
        "id", "covers", "target", "intent", "input", "expect", "kind", "case", "related", "src-id",
    ];
    let mut had_error = false;
    for line in lines {
        let Some(annotation) = line.strip_prefix("@vtest.") else {
            continue;
        };
        let (key, value) = if let Some(separator) = annotation.find(char::is_whitespace) {
            annotation.split_at(separator)
        } else {
            (annotation, "")
        };
        let key = key.trim().to_owned();
        let value = value.trim().to_owned();
        if !KNOWN.contains(&key.as_str()) {
            // The caller cannot attach a parser diagnostic without losing the
            // source location, so retain a sentinel that is handled below.
            values.insert("__unknown_key__".to_owned(), key);
            had_error = true;
            continue;
        }
        if matches!(key.as_str(), "case" | "related" | "target") {
            repeated.entry(key).or_default().push(value);
        } else if values.insert(key.clone(), value).is_some() {
            values.insert("__duplicate_key__".to_owned(), key);
            had_error = true;
        }
    }
    if had_error {
        // Preserve parse information in a deterministic diagnostic channel.
        // `parse_annotations` itself stays total and its caller emits the
        // proper location-aware diagnostic.
        if let Some(key) = values.remove("__unknown_key__") {
            values.insert("__parse_error__".to_owned(), format!("unknown:{key}"));
        } else if let Some(key) = values.remove("__duplicate_key__") {
            values.insert("__parse_error__".to_owned(), format!("duplicate:{key}"));
        }
    }
    Some(ParsedAnnotations { values, repeated })
}

pub(crate) fn is_test_function(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "test")
    })
}

pub(crate) fn parse_src_id(attrs: &[Attribute]) -> Option<SrcId> {
    parse_annotations(attrs)
        .and_then(|annotations| annotations.values.get("src-id").cloned())
        .map(SrcId::new)
}

pub(crate) fn join_module_path(prefix: &str, item_path: &str) -> String {
    if prefix.is_empty() {
        item_path.to_owned()
    } else {
        format!("{prefix}::{item_path}")
    }
}

pub(crate) fn line_offsets(source: &str) -> Vec<usize> {
    let mut offsets = vec![0];
    for (index, byte) in source.as_bytes().iter().enumerate() {
        if *byte == b'\n' {
            offsets.push(index + 1);
        }
    }
    offsets
}

pub(crate) fn make_location(
    relative: &str,
    function: &str,
    span: proc_macro2::Span,
    source: &str,
    offsets: &[usize],
) -> SourceLocation {
    let start = span.start();
    let end = span.end();
    let start_line = start.line.max(1);
    let end_line = end.line.max(start_line);
    let start_byte = offsets.get(start_line - 1).copied().unwrap_or(0) + start.column;
    let end_byte = offsets.get(end_line - 1).copied().unwrap_or(source.len()) + end.column;
    SourceLocation {
        adapter: AdapterId::new(RUST_ADAPTER_ID),
        path: ProjectPath::new(relative),
        locator: function.to_owned(),
        byte_range: SourceRange {
            start: start_byte,
            end: end_byte.min(source.len()),
            start_line,
            end_line,
        },
    }
}

pub(crate) fn source_slice<'a>(source: &'a str, location: &SourceLocation) -> &'a str {
    source
        .get(location.byte_range.start..location.byte_range.end)
        .unwrap_or("")
}
