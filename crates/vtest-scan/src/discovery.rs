//! Rust-specific source-location helpers extracted from the scanner. These move
//! to `vtest-adapter-rust` together with the rest of the Rust discovery code
//! (the scanner, Cargo manifest and module resolution, annotation parsing);
//! grouping the language-specific surface into one module isolates it first so
//! the cross-crate move is mechanical.

use vtest_model::{AdapterId, ProjectPath, SourceLocation, SourceRange};

use crate::RUST_ADAPTER_ID;

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
