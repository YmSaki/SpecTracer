//! `rust-cargo`'s Structured Test capability (BD-197/BD-310, issue #73):
//! the Rust grammar questions `test create` / `test edit` ask. Moved here
//! from `vtest-scan::operations` so core holds no Rust parser.

use std::ops::Range;

use syn::spanned::Spanned;
use vtest_adapter_api::StructuredTestAdapter;

#[derive(Debug, Default)]
pub struct RustStructuredTest;

/// Byte offset of a `proc_macro2` line (1-based) / column (0-based,
/// counted in characters) pair.
fn source_offset(source: &str, line: usize, column: usize) -> Option<usize> {
    crate::line_column_byte_offset(source, &crate::line_offsets(source), line, column)
}

fn collect_enum_variants(items: &[syn::Item], type_name: &str, variants: &mut Vec<String>) {
    for item in items {
        match item {
            syn::Item::Enum(item_enum) if item_enum.ident == type_name => {
                variants.extend(
                    item_enum
                        .variants
                        .iter()
                        .map(|variant| variant.ident.to_string()),
                );
            }
            syn::Item::Mod(item_mod) => {
                if let Some((_, items)) = &item_mod.content {
                    collect_enum_variants(items, type_name, variants);
                }
            }
            _ => {}
        }
    }
}

impl StructuredTestAdapter for RustStructuredTest {
    fn check_test_construct(&self, text: &str) -> Result<(), String> {
        syn::parse_str::<syn::ItemFn>(text)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn check_source_file(&self, text: &str) -> Result<(), String> {
        syn::parse_file(text)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn check_body(&self, text: &str) -> Result<(), String> {
        syn::parse_str::<syn::Block>(text)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn is_identifier(&self, name: &str) -> bool {
        syn::parse_str::<syn::Ident>(name).is_ok()
    }

    fn body_range(&self, construct: &str) -> Result<Range<usize>, String> {
        let parsed = syn::parse_str::<syn::ItemFn>(construct).map_err(|error| error.to_string())?;
        let span = parsed.block.span();
        let start = source_offset(construct, span.start().line, span.start().column)
            .ok_or_else(|| "Test body start is out of range".to_owned())?;
        let end = source_offset(construct, span.end().line, span.end().column)
            .ok_or_else(|| "Test body end is out of range".to_owned())?;
        if start >= end || construct.get(start..end).is_none() {
            return Err("Test function body range is invalid".to_owned());
        }
        Ok(start..end)
    }

    fn enum_variants(&self, source: &str, type_name: &str) -> Option<Vec<String>> {
        let parsed = syn::parse_file(source).ok()?;
        let mut variants = Vec::new();
        collect_enum_variants(&parsed.items, type_name, &mut variants);
        Some(variants)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @vtest.id TEST-ADAPTER-RUST-STRUCTURED-TEST-SYNTAX
    /// @vtest.covers VO-ADAPTER-RUST-STRUCTURED-TEST-CAPABILITY
    /// @vtest.target crates/vtest-adapter-rust/src/structured_test.rs::RustStructuredTest::body_range
    /// @vtest.intent the rust-cargo Structured Test capability accepts valid Rust and rejects invalid Rust for constructs, files, bodies and identifiers, locates a function body, and lists enum variants including nested modules
    #[test]
    fn rust_structured_test_answers_the_grammar_questions() {
        let rust = RustStructuredTest;
        let function = "#[test]\nfn adds() {\n    assert_eq!(1 + 1, 2);\n}";
        assert!(rust.check_test_construct(function).is_ok());
        assert!(rust.check_test_construct("fn broken(").is_err());
        assert!(rust.check_source_file("fn a() {}\nfn b() {}\n").is_ok());
        assert!(rust.check_source_file("fn a() {").is_err());
        assert!(rust.check_body("{ let x = 1; }").is_ok());
        assert!(rust.check_body("{ let = ; }").is_err());
        assert!(rust.is_identifier("adds"));
        assert!(!rust.is_identifier("not an ident"));
        let range = rust.body_range(function).unwrap();
        assert_eq!(&function[range], "{\n    assert_eq!(1 + 1, 2);\n}");
        assert_eq!(
            rust.enum_variants("enum Kind { A, B }\nmod inner { enum Kind { C } }", "Kind"),
            Some(vec!["A".to_owned(), "B".to_owned(), "C".to_owned()])
        );
        assert_eq!(rust.enum_variants("enum {", "Kind"), None);
    }
}
