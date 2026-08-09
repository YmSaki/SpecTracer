//! Rust-owned built-in Form Schemas.
//!
//! The schema parser and answer types remain in `vtest-store` as neutral
//! persistence primitives.  Only this adapter supplies Rust-specific forms.

use std::{fs, io::ErrorKind};

use vtest_store::{FormSchema, StoreError, VerifyLayout};

pub const RUST_UNIT_FUNCTION_FORM: &str = r#"kind: rust-unit-function
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

pub const RUST_INTEGRATION_FORM: &str = r#"kind: rust-integration
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
  /// @vtest.kind integration-{test_kind}
  #[test]
  fn {fn_name}() {
      todo!("implement test body")
  }
"#;

/// Write the built-in Rust forms without replacing a user-customized schema.
pub fn ensure_builtin_forms(layout: &VerifyLayout) -> Result<(), StoreError> {
    fs::create_dir_all(layout.forms_dir()).map_err(|source| StoreError::Io {
        path: layout.forms_dir(),
        source,
    })?;
    for (kind, text) in [
        ("rust-unit-function", RUST_UNIT_FUNCTION_FORM),
        ("rust-integration", RUST_INTEGRATION_FORM),
    ] {
        let path = layout.forms_dir().join(format!("{kind}.yaml"));
        if !path.exists() {
            fs::write(&path, text).map_err(|source| StoreError::Io {
                path: path.clone(),
                source,
            })?;
        }
    }
    Ok(())
}

/// Load a project override, or fall back to the Rust adapter's built-in form.
pub fn load_form_schema(layout: &VerifyLayout, kind: &str) -> Result<FormSchema, StoreError> {
    match vtest_store::load_form_schema(layout, kind) {
        Ok(schema) => Ok(schema),
        Err(error) => {
            let missing = matches!(
                &error,
                StoreError::Io { source, .. } if source.kind() == ErrorKind::NotFound
            );
            if !missing {
                return Err(error);
            }
            let text = match kind {
                "rust-unit-function" => Some(RUST_UNIT_FUNCTION_FORM),
                "rust-integration" => Some(RUST_INTEGRATION_FORM),
                _ => None,
            };
            text.map(vtest_store::parse_form_schema)
                .unwrap_or(Err(error))
        }
    }
}
