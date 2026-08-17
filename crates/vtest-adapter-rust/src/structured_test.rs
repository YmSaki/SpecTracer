//! `rust-cargo`'s `StructuredTestAdapter`: owns the doc-comment declaration
//! syntax (`///` and `/** */`, both normative per 詳細設計 §4.2) and the
//! annotation-block regeneration rules of 別紙A §15.1-§15.4.
//!
//! Core hands over only the desired LOGICAL fields (`StructuredEditFields`);
//! this module is the sole place that decides declaration placement and
//! emits `@vtest.*` syntax for the edit path. It classifies the current
//! declaration's doc lines by re-parsing the source with `syn` and reading
//! the same `#[doc = "..."]` attributes discovery already normalizes both
//! comment forms into (crates/vtest-adapter-rust/src/discovery.rs), instead
//! of string-matching raw source lines against `///` -- the asymmetry that
//! made Edit Test permanently unusable for `/** */`-declared Tests.

use serde_json::Value;
use vtest_adapter_api::{StructuredEditFields, StructuredTestAdapter};
use vtest_model::Diagnostic;

use crate::operations_support::{check_rust_block, source_offset, test_body_byte_range};

pub const RUST_UNIT_FUNCTION_KIND: &str = "rust-unit-function";
pub const RUST_INTEGRATION_KIND: &str = "rust-integration";

/// Validators that only `rust-cargo`'s own built-in forms declare. A legacy
/// Form lacking `adapter` is accepted as compatible only if its schema
/// CONTENT names at least one of these -- never by inspecting `kind`'s
/// string alone (別紙A §14.2's matcher requirement).
const RUST_VALIDATORS: &[&str] = &[
    "symbol-exists",
    "symbols-exist",
    "unique-fn-name",
    "rust-file",
    "enum-variant-exists",
];

#[derive(Debug, Default)]
pub struct RustCargoStructuredTest;

impl StructuredTestAdapter for RustCargoStructuredTest {
    fn built_in_form_kinds(&self) -> Vec<String> {
        vec![
            RUST_UNIT_FUNCTION_KIND.to_owned(),
            RUST_INTEGRATION_KIND.to_owned(),
        ]
    }

    fn accepts_compatibility_form(&self, schema: &Value) -> bool {
        let Some(fields) = schema.get("fields").and_then(Value::as_array) else {
            return false;
        };
        fields.iter().any(|field| {
            field
                .get("validate")
                .and_then(Value::as_array)
                .is_some_and(|validators| {
                    validators
                        .iter()
                        .filter_map(Value::as_str)
                        .any(|validator| RUST_VALIDATORS.contains(&validator))
                })
        })
    }

    fn render_edit(
        &self,
        current_source: &str,
        current_id: &str,
        current_selector: &str,
        desired: &StructuredEditFields,
        body: Option<&str>,
    ) -> Result<String, Diagnostic> {
        render_edited_test(current_source, current_id, current_selector, desired, body)
    }
}

fn render_edited_test(
    current_source: &str,
    current_id: &str,
    current_selector: &str,
    desired: &StructuredEditFields,
    body: Option<&str>,
) -> Result<String, Diagnostic> {
    let parsed = syn::parse_str::<syn::ItemFn>(current_source).map_err(|error| {
        Diagnostic::error(
            "E-OP-002",
            format!("Test `{current_id}` source range could not be reparsed: {error}"),
        )
    })?;

    // Classify each LOGICAL doc line (not raw source line) the same way
    // discovery does: every `#[doc = "..."]` attribute, whether it desugared
    // from `///` (one attribute per line) or `/** ... */` (one attribute
    // whose value spans every line of the block), contributes lines in
    // source order. This is what makes both declaration forms round-trip
    // identically through Edit Test.
    let mut free_before = Vec::new();
    let mut free_after = Vec::new();
    let mut saw_annotation = false;
    let mut tail_start_span = None;
    for attr in &parsed.attrs {
        if !attr.path().is_ident("doc") {
            if tail_start_span.is_none() {
                tail_start_span = Some(attr_span_start(attr));
            }
            continue;
        }
        let syn::Meta::NameValue(value) = &attr.meta else {
            continue;
        };
        let syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(text),
            ..
        }) = &value.value
        else {
            continue;
        };
        for line in text.value().lines() {
            let trimmed = line.trim();
            if trimmed.contains("@vtest.") {
                saw_annotation = true;
                continue;
            }
            let rendered = if trimmed.is_empty() {
                "///".to_owned()
            } else {
                format!("/// {trimmed}")
            };
            if saw_annotation {
                free_after.push(rendered);
            } else {
                free_before.push(rendered);
            }
        }
    }
    let tail_start = match tail_start_span {
        Some(span) => source_offset(current_source, span.0, span.1).ok_or_else(|| {
            Diagnostic::error(
                "E-OP-002",
                format!("Test `{current_id}` source range is stale"),
            )
        })?,
        None => {
            use syn::spanned::Spanned;
            let span = parsed.sig.span().start();
            source_offset(current_source, span.line, span.column).ok_or_else(|| {
                Diagnostic::error(
                    "E-OP-002",
                    format!("Test `{current_id}` source range is stale"),
                )
            })?
        }
    };
    let mut function = current_source
        .get(tail_start..)
        .ok_or_else(|| {
            Diagnostic::error(
                "E-OP-002",
                format!("Test `{current_id}` has no function body in its source range"),
            )
        })?
        .to_owned();

    if desired.fn_name != current_selector {
        let from = format!("fn {current_selector}");
        let to = format!("fn {}", desired.fn_name);
        if !function.contains(&from) {
            return Err(Diagnostic::error(
                "E-OP-002",
                format!("Test `{current_id}` function signature could not be located"),
            ));
        }
        function = function.replacen(&from, &to, 1);
    }
    if let Some(body) = body {
        let body = normalize_body(body)?;
        let (start, end) =
            test_body_byte_range(&function).map_err(|error| Diagnostic::error("E-OP-002", error))?;
        function.replace_range(start..end, &body);
    }

    let mut lines = free_before;
    lines.push(format!("/// @vtest.id {}", desired.id));
    lines.push(format!("/// @vtest.covers {}", desired.covers.join(",")));
    for target in &desired.targets {
        lines.push(format!("/// @vtest.target {target}"));
    }
    lines.push(format!("/// @vtest.intent {}", desired.intent));
    if let Some(input) = &desired.input {
        lines.push(format!("/// @vtest.input {input}"));
    }
    if let Some(expect) = &desired.expect {
        lines.push(format!("/// @vtest.expect {expect}"));
    }
    if let Some(kind) = &desired.kind {
        lines.push(format!("/// @vtest.kind {kind}"));
    }
    for case in &desired.cases {
        lines.push(format!("/// @vtest.case {case}"));
    }
    for related in &desired.related {
        lines.push(format!("/// @vtest.related {related}"));
    }
    lines.extend(free_after);
    lines.push(function);
    Ok(lines.join("\n"))
}

fn attr_span_start(attr: &syn::Attribute) -> (usize, usize) {
    use syn::spanned::Spanned;
    let start = attr.span().start();
    (start.line, start.column)
}

fn normalize_body(body: &str) -> Result<String, Diagnostic> {
    let body = body.trim();
    let body = if body.starts_with('{') && body.ends_with('}') {
        body.to_owned()
    } else {
        format!("{{\n{body}\n}}")
    };
    check_rust_block(&body).map_err(|error| {
        Diagnostic::error(
            "E-OP-001",
            format!("body file does not contain a valid Rust block: {error}"),
        )
    })?;
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(fn_name: &str) -> StructuredEditFields {
        StructuredEditFields {
            id: "TEST-ARUST-030".to_owned(),
            covers: vec!["VO-ARUST-030".to_owned()],
            targets: vec!["src/lib.rs::add".to_owned()],
            intent: "add returns the sum".to_owned(),
            input: None,
            expect: None,
            kind: None,
            cases: Vec::new(),
            related: Vec::new(),
            fn_name: fn_name.to_owned(),
            file: "src/lib.rs".to_owned(),
        }
    }

    /// @vtest.id TEST-ARUST-030
    /// @vtest.covers VO-REGISTRY-05
    /// @vtest.target crates/vtest-adapter-rust/src/structured_test.rs::render_edited_test
    /// @vtest.intent A Test declared with the `/** */` block doc form round-trips through render_edit unchanged in shape
    #[test]
    fn block_doc_comment_declaration_round_trips() {
        let current_source = "/**\nfree text before\n@vtest.id TEST-ARUST-030\n@vtest.covers VO-ARUST-030\n@vtest.target src/lib.rs::add\n@vtest.intent add returns the sum\n*/\n#[test]\nfn add_returns_sum() {\n    assert_eq!(add(1, 2), 3);\n}";
        let rendered =
            render_edited_test(current_source, "TEST-ARUST-030", "add_returns_sum", &fields("add_returns_sum"), None)
                .expect("block doc comment declaration re-renders");
        assert!(rendered.contains("/// free text before"));
        assert!(rendered.contains("/// @vtest.id TEST-ARUST-030"));
        assert!(rendered.contains("/// @vtest.target src/lib.rs::add"));
        assert!(rendered.contains("fn add_returns_sum()"));
        assert!(rendered.contains("assert_eq!(add(1, 2), 3);"));
        assert!(
            syn::parse_str::<syn::ItemFn>(&rendered).is_ok(),
            "rendered replacement must remain valid Rust"
        );
    }

    /// @vtest.id TEST-ARUST-031
    /// @vtest.covers VO-REGISTRY-05
    /// @vtest.target crates/vtest-adapter-rust/src/structured_test.rs::render_edited_test
    /// @vtest.intent A Test declared with `///` line doc comments still renders correctly after moving behind the adapter trait
    #[test]
    fn line_doc_comment_declaration_still_renders() {
        let current_source = "/// @vtest.id TEST-ARUST-030\n/// @vtest.covers VO-ARUST-030\n/// @vtest.target src/lib.rs::add\n/// @vtest.intent add returns the sum\n#[test]\nfn add_returns_sum() {\n    assert_eq!(add(1, 2), 3);\n}";
        let rendered = render_edited_test(
            current_source,
            "TEST-ARUST-030",
            "add_returns_sum",
            &fields("adds_the_values"),
            None,
        )
        .expect("line doc comment declaration re-renders");
        assert!(rendered.contains("fn adds_the_values()"));
        assert!(!rendered.contains("free text before"));
    }
}
