//! `rust-cargo`'s Structured Test capability (BD-197/BD-310, issue #73):
//! the Rust grammar questions `test create` / `test edit` ask. Moved here
//! from `vtest-scan::operations` so core holds no Rust parser.

use std::{
    fs,
    ops::Range,
    path::{Path, PathBuf},
};

use syn::spanned::Spanned;
use vtest_adapter_api::{BuiltInForm, ParsedLocator, StructuredTestAdapter};

use crate::RustLocator;

#[derive(Debug, Default)]
pub struct RustStructuredTest;

/// 別紙A §14.1: the built-in `rust-unit-function` Form (BD-248).
pub const RUST_UNIT_FUNCTION_FORM: &str = r#"kind: rust-unit-function
adapter: rust-cargo
title: Rust function unit test
fields:
  - name: target
    question: Target source symbol?
    type: symbol
    required: true
    validate: [symbol-exists]
  - name: covers
    question: Verification objectives?
    type: vo-ref-list
    required: true
    validate: [vo-exists]
  - name: behavior
    question: Behavior under test?
    type: string
    required: true
  - name: test_kind
    question: Test kind?
    type: enum
    options: [normal, error, boundary, regression]
    required: true
  - name: input
    question: Input conditions?
    type: string
    required: true
  - name: expect
    question: Expected result?
    type: string
    required: true
    validate: [enum-variant-exists]
  - name: fn_name
    question: Test function name?
    type: ident
    required: true
    validate: [unique-fn-name]
  - name: file
    question: Destination Rust file?
    type: path
    required: false
    validate: [rust-file]
template: |
  /// @vtest.id {test_id}
  /// @vtest.covers {covers}
  /// @vtest.target {target}
  /// @vtest.intent {behavior}
  /// @vtest.input {input}
  /// @vtest.expect {expect}
  /// @vtest.kind unit-{test_kind}
  #[test]
  fn {fn_name}() {
      todo!("implement test body")
  }
"#;

/// 別紙A §14.3: the built-in `rust-integration` Form (DS-1241).
pub const RUST_INTEGRATION_FORM: &str = r#"kind: rust-integration
adapter: rust-cargo
title: Rust integration test
fields:
  - name: targets
    question: Target source symbols?
    type: symbol-list
    required: true
    validate: [symbols-exist]
  - name: covers
    question: Verification objectives?
    type: vo-ref-list
    required: true
    validate: [vo-exists]
  - name: behavior
    question: Behavior under test?
    type: string
    required: true
  - name: test_kind
    question: Test kind?
    type: enum
    options: [normal, error, boundary, regression]
    required: true
  - name: input
    question: Input conditions?
    type: string
    required: true
  - name: expect
    question: Expected result?
    type: string
    required: true
    validate: [enum-variant-exists]
  - name: fn_name
    question: Test function name?
    type: ident
    required: true
    validate: [unique-fn-name]
  - name: file
    question: Destination Rust file?
    type: path
    required: true
    validate: [rust-file]
template: |
  /// @vtest.id {test_id}
  /// @vtest.covers {covers}
  /// @vtest.target {targets}
  /// @vtest.intent {behavior}
  /// @vtest.input {input}
  /// @vtest.expect {expect}
  /// @vtest.kind unit-{test_kind}
  #[test]
  fn {fn_name}() {
      todo!("implement test body")
  }
"#;

/// DES-373: the built-in Form kinds `rust-cargo` owns.
pub const BUILT_IN_FORMS: &[BuiltInForm] = &[
    BuiltInForm {
        kind: "rust-unit-function",
        schema: RUST_UNIT_FUNCTION_FORM,
    },
    BuiltInForm {
        kind: "rust-integration",
        schema: RUST_INTEGRATION_FORM,
    },
];

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

// PR #26 review round 3 要確認C-1: この `read_dir`/`entries.flatten()` は
// `d423f8a` が `vtest-scan::validate_relations`/`validate_approval_status`
// で塞いだのと同じ形の I/O fail-open（ディレクトリが開けない、または列挙中
// に個々の `DirEntry` が `Err` を返すと、そのディレクトリ／エントリを
// 無診断でスキップする）だが、意図的に塞いでいない: 呼び出し元
// `vtest-scan::operations::validate_enum_variant` は `type_name` の Rust
// 識別子としての妥当性チェックや `enum_variants` による parse 失敗も同じ
// 「見つけられなければ検証をスキップし `Ok(())` を返す」扱いを既にしており
// （`variants.is_empty()`）、これは `enum-variant-exists` Structured
// Operation 入力検証のベストエフォート設計そのもの — I/O 失敗だけを
// fail-closed にしても、識別子形式や parse 失敗という他の「見つからない」
// 経路はそのまま素通りするので非対称は解消しない。影響範囲も、この関数の
// 裁定（chain_integrity・検証状態）ではなく、`vtest scan enum-variant-exists`
// 単体の Structured Operation 入力検証（本冊:1641、別紙A:539）に留まる:
// I/O 失敗時に本来出すべき E-OP-001（不正な値）が見逃され得るが、それ以上
// 状態が壊れることはない。塞ぐなら `validate_enum_variant` 全体の
// 「見つからなければ受理する」設計を Owner 裁定で見直す必要があり、この
// 箇所だけを個別に fail-closed にするのは非対称を悪化させるため見送った。
fn collect_rust_files(directory: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|value| value.to_str()) != Some("target") {
                collect_rust_files(&path, files);
            }
        } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

impl StructuredTestAdapter for RustStructuredTest {
    fn built_in_forms(&self) -> &'static [BuiltInForm] {
        BUILT_IN_FORMS
    }

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

    fn language(&self) -> &'static str {
        "Rust"
    }

    fn source_extension(&self) -> &'static str {
        "rs"
    }

    fn source_files(&self, directory: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        collect_rust_files(directory, &mut files);
        files
    }

    fn parse_locator(&self, value: &str) -> Option<ParsedLocator> {
        let parsed = RustLocator::parse(value)?;
        Some(ParsedLocator {
            locator: parsed.to_locator(),
            file: parsed.path,
            name: parsed.item_path,
        })
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

    /// @vtest.id TEST-ADAPTER-RUST-STRUCTURED-TEST-LOCATOR-AND-FILES
    /// @vtest.covers VO-ADAPTER-RUST-STRUCTURED-TEST-CAPABILITY
    /// @vtest.target crates/vtest-adapter-rust/src/structured_test.rs::RustStructuredTest::parse_locator
    /// @vtest.intent the rust-cargo Structured Test capability names its language and file extension, splits a locator into the normalized locator, file and name that discovery uses, rejects a value that is not a locator, and lists .rs files below a directory except those under target
    #[test]
    fn rust_structured_test_answers_the_locator_and_file_questions() {
        let rust = RustStructuredTest;
        assert_eq!(rust.language(), "Rust");
        assert_eq!(rust.source_extension(), "rs");

        let parsed = rust.parse_locator(r"src\lib.rs::module::function").unwrap();
        assert_eq!(parsed.file, "src/lib.rs");
        assert_eq!(parsed.name, "module::function");
        assert_eq!(
            parsed.locator,
            crate::locator_from_declared_value("src/lib.rs::module::function")
        );
        assert_eq!(rust.parse_locator("src/lib.txt::function"), None);
        assert_eq!(rust.parse_locator("function"), None);

        let root = std::env::temp_dir().join(format!(
            "vtest-adapter-rust-source-files-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src/nested")).unwrap();
        fs::create_dir_all(root.join("target/debug")).unwrap();
        for file in [
            "src/lib.rs",
            "src/nested/mod.rs",
            "src/notes.md",
            "target/debug/build.rs",
        ] {
            fs::write(root.join(file), "").unwrap();
        }
        let mut files = rust.source_files(&root);
        files.sort();
        assert_eq!(
            files,
            vec![root.join("src/lib.rs"), root.join("src/nested/mod.rs")]
        );
        assert!(rust.source_files(&root.join("missing")).is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    /// @vtest.id TEST-STORE-FORM-RUST-UNIT-FUNCTION-SCHEMA-SHAPE-001
    /// @vtest.covers VO-STORE-FORM-RUST-UNIT-FUNCTION-SCHEMA-SHAPE
    /// @vtest.target crates/vtest-adapter-rust/src/structured_test.rs::RUST_UNIT_FUNCTION_FORM
    /// @vtest.intent 組込Form `rust-unit-function` がkind/8 field/`@vtest.target {target}` templateを持つことを検証する。
    #[test]
    fn built_in_unit_form_parses() {
        let form = vtest_store::parse_form_schema(RUST_UNIT_FUNCTION_FORM).unwrap();
        assert_eq!(form.kind, "rust-unit-function");
        assert_eq!(form.adapter.as_deref(), Some("rust-cargo"));
        assert_eq!(form.fields.len(), 8);
        assert!(form.template.contains("@vtest.target {target}"));
    }

    /// @vtest.id TEST-STORE-FORM-RUST-INTEGRATION-KIND-LINE-MATCHES-UNIT-001
    /// @vtest.covers VO-STORE-FORM-RUST-INTEGRATION-KIND-LINE-MATCHES-UNIT
    /// @vtest.target crates/vtest-adapter-rust/src/structured_test.rs::RUST_INTEGRATION_FORM
    /// @vtest.intent `rust-integration`のtemplateが`@vtest.kind`行で`rust-unit-function`と一致し`integration-`を生成しないことを検証する。
    /// 別紙A §14.3「§14.1との差分はこの2点であり、他は同一」（`target`→
    /// `targets`必須化と`file`のrequired化の2点のみ）。`rust-integration`の
    /// templateが出力する`@vtest.kind`行は`rust-unit-function`と同一の
    /// `unit-{test_kind}`でなければならず、`integration-{test_kind}`を
    /// 生成してはならない（pr3-decisions.md Owner裁定3）。
    #[test]
    fn built_in_integration_form_kind_line_matches_the_unit_form() {
        let unit = vtest_store::parse_form_schema(RUST_UNIT_FUNCTION_FORM).unwrap();
        let integration = vtest_store::parse_form_schema(RUST_INTEGRATION_FORM).unwrap();
        assert_eq!(integration.kind, "rust-integration");
        let kind_line = |template: &str| {
            template
                .lines()
                .find(|line| line.contains("@vtest.kind"))
                .unwrap()
                .trim()
                .to_owned()
        };
        assert_eq!(kind_line(&unit.template), kind_line(&integration.template));
        assert!(integration
            .template
            .contains("@vtest.kind unit-{test_kind}"));
        assert!(!integration.template.contains("@vtest.kind integration-"));
    }
}
