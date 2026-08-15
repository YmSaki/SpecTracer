use std::{collections::BTreeMap, fs, path::Path};

use crate::{StoreError, VerifyLayout};

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
  /// @vtest.kind integration-{test_kind}
  #[test]
  fn {fn_name}() {
      todo!("implement test body")
  }
"#;

pub use vtest_model::{FormAnswers, FormField, FormSchema, FormValue};

pub fn load_form_schema(layout: &VerifyLayout, kind: &str) -> Result<FormSchema, StoreError> {
    if !safe_form_kind(kind) {
        return Err(StoreError::InvalidForm(format!(
            "form kind contains unsafe characters: `{kind}`"
        )));
    }
    let path = layout.forms_dir().join(format!("{kind}.yaml"));
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => match kind {
            "rust-unit-function" => RUST_UNIT_FUNCTION_FORM.to_owned(),
            "rust-integration" => RUST_INTEGRATION_FORM.to_owned(),
            _ => {
                return Err(StoreError::Io {
                    path,
                    source: error,
                })
            }
        },
        Err(source) => return Err(StoreError::Io { path, source }),
    };
    parse_form_schema(&text)
}

pub fn read_form_answers(path: &Path) -> Result<FormAnswers, StoreError> {
    let text = fs::read_to_string(path).map_err(|source| StoreError::Io {
        path: path.to_owned(),
        source,
    })?;
    parse_form_answers(&text)
}

pub fn parse_form_schema(text: &str) -> Result<FormSchema, StoreError> {
    let mut kind = None;
    let mut adapter = None;
    let mut title = None;
    let mut fields = Vec::new();
    let mut field = None::<FormField>;
    let mut template = String::new();
    let mut in_fields = false;
    let mut in_template = false;

    for raw in text.lines() {
        if in_template {
            if raw.starts_with("  ") || raw.is_empty() {
                template.push_str(raw.strip_prefix("  ").unwrap_or(raw));
                template.push('\n');
                continue;
            }
            return Err(StoreError::InvalidForm(
                "template block must remain indented".to_owned(),
            ));
        }
        let line = strip_comment(raw).trim_end();
        if line.trim().is_empty() {
            continue;
        }
        let trimmed = line.trim_start();
        if !line.starts_with(' ') {
            if let Some(previous) = field.take() {
                fields.push(validate_field(previous)?);
            }
            in_fields = false;
            if trimmed == "fields:" {
                in_fields = true;
                continue;
            }
            if trimmed == "template: |" {
                in_template = true;
                continue;
            }
            let (key, value) = property(trimmed, "form schema")?;
            match key {
                "kind" => kind = Some(parse_scalar(value, "form kind")?),
                "adapter" => adapter = Some(parse_scalar(value, "form adapter")?),
                "title" => title = Some(parse_scalar(value, "form title")?),
                _ => {
                    return Err(StoreError::InvalidForm(format!(
                        "unknown top-level field `{key}`"
                    )))
                }
            }
            continue;
        }
        if !in_fields {
            return Err(StoreError::InvalidForm(format!(
                "unexpected indented line `{trimmed}`"
            )));
        }
        if let Some(first_property) = trimmed.strip_prefix("- ") {
            if let Some(previous) = field.take() {
                fields.push(validate_field(previous)?);
            }
            field = Some(empty_field());
            set_field_property(field.as_mut().expect("field"), first_property)?;
        } else {
            let current = field.as_mut().ok_or_else(|| {
                StoreError::InvalidForm("field property appears before `- name`".to_owned())
            })?;
            set_field_property(current, trimmed)?;
        }
    }
    if let Some(previous) = field {
        fields.push(validate_field(previous)?);
    }
    let kind = kind.ok_or_else(|| StoreError::InvalidForm("missing `kind`".to_owned()))?;
    if !safe_form_kind(&kind) {
        return Err(StoreError::InvalidForm(format!(
            "form kind contains unsafe characters: `{kind}`"
        )));
    }
    let title = title.ok_or_else(|| StoreError::InvalidForm("missing `title`".to_owned()))?;
    if fields.is_empty() {
        return Err(StoreError::InvalidForm("form has no fields".to_owned()));
    }
    if template.is_empty() {
        return Err(StoreError::InvalidForm("form has no template".to_owned()));
    }
    let mut names = std::collections::BTreeSet::new();
    for item in &fields {
        if !names.insert(item.name.clone()) {
            return Err(StoreError::InvalidForm(format!(
                "duplicate field `{}`",
                item.name
            )));
        }
    }
    Ok(FormSchema {
        kind,
        adapter,
        title,
        fields,
        template,
    })
}

pub fn parse_form_answers(text: &str) -> Result<FormAnswers, StoreError> {
    parse_form_answers_inner(text).map_err(|error| match error {
        StoreError::InvalidForm(message) => StoreError::InvalidAnswers(message),
        other => other,
    })
}

fn parse_form_answers_inner(text: &str) -> Result<FormAnswers, StoreError> {
    let mut form = None;
    let mut answers = BTreeMap::new();
    let mut in_answers = false;
    let mut list_key = None::<String>;

    for raw in text.lines() {
        let line = strip_comment(raw).trim_end();
        if line.trim().is_empty() {
            continue;
        }
        let trimmed = line.trim_start();
        if !line.starts_with(' ') {
            list_key = None;
            if trimmed == "answers:" {
                in_answers = true;
                continue;
            }
            in_answers = false;
            let (key, value) = property(trimmed, "answers file")?;
            match key {
                "form" => form = Some(parse_scalar(value, "form")?),
                _ => {
                    return Err(StoreError::InvalidAnswers(format!(
                        "unknown top-level field `{key}`"
                    )))
                }
            }
            continue;
        }
        if !in_answers {
            return Err(StoreError::InvalidAnswers(format!(
                "unexpected indented line `{trimmed}`"
            )));
        }
        if let Some(value) = trimmed.strip_prefix("- ") {
            let key = list_key.as_ref().ok_or_else(|| {
                StoreError::InvalidAnswers("list item has no answer field".to_owned())
            })?;
            let value = parse_scalar(value, key)?;
            match answers.get_mut(key) {
                Some(FormValue::List(values)) => values.push(value),
                _ => unreachable!("list key is initialized as a list"),
            }
            continue;
        }
        let (key, value) = property(trimmed, "answers")?;
        if answers.contains_key(key) {
            return Err(StoreError::InvalidAnswers(format!(
                "duplicate answer `{key}`"
            )));
        }
        if value.is_empty() {
            list_key = Some(key.to_owned());
            answers.insert(key.to_owned(), FormValue::List(Vec::new()));
        } else if value.starts_with('[') {
            list_key = None;
            answers.insert(key.to_owned(), FormValue::List(parse_list(value, key)?));
        } else {
            list_key = None;
            answers.insert(key.to_owned(), FormValue::Scalar(parse_scalar(value, key)?));
        }
    }
    let form = form.ok_or_else(|| StoreError::InvalidAnswers("missing `form`".to_owned()))?;
    if answers.is_empty() {
        return Err(StoreError::InvalidAnswers("missing `answers`".to_owned()));
    }
    Ok(FormAnswers { form, answers })
}

fn empty_field() -> FormField {
    FormField {
        name: String::new(),
        question: String::new(),
        field_type: String::new(),
        required: false,
        options: Vec::new(),
        validate: Vec::new(),
    }
}

fn set_field_property(field: &mut FormField, text: &str) -> Result<(), StoreError> {
    let (key, value) = property(text, "form field")?;
    match key {
        "name" => field.name = parse_scalar(value, "field name")?,
        "question" => field.question = parse_scalar(value, "field question")?,
        "type" => field.field_type = parse_scalar(value, "field type")?,
        "required" => {
            field.required = match value {
                "true" => true,
                "false" => false,
                _ => {
                    return Err(StoreError::InvalidForm(format!(
                        "required must be true or false, got `{value}`"
                    )))
                }
            }
        }
        "options" => field.options = parse_list(value, "options")?,
        "validate" => field.validate = parse_list(value, "validate")?,
        _ => {
            return Err(StoreError::InvalidForm(format!(
                "unknown field property `{key}`"
            )))
        }
    }
    Ok(())
}

fn validate_field(field: FormField) -> Result<FormField, StoreError> {
    if field.name.is_empty() || field.question.is_empty() || field.field_type.is_empty() {
        return Err(StoreError::InvalidForm(
            "every field requires name, question, and type".to_owned(),
        ));
    }
    const TYPES: &[&str] = &[
        "symbol",
        "symbol-list",
        "vo-ref",
        "vo-ref-list",
        "test-ref",
        "enum",
        "string",
        "ident",
        "path",
    ];
    if !TYPES.contains(&field.field_type.as_str()) {
        return Err(StoreError::InvalidForm(format!(
            "unknown type `{}` for field `{}`",
            field.field_type, field.name
        )));
    }
    if field.field_type == "enum" && field.options.is_empty() {
        return Err(StoreError::InvalidForm(format!(
            "enum field `{}` has no options",
            field.name
        )));
    }
    for validator in &field.validate {
        let compatible = matches!(
            (validator.as_str(), field.field_type.as_str()),
            ("symbol-exists", "symbol")
                | ("symbols-exist", "symbol-list")
                | ("vo-exists", "vo-ref" | "vo-ref-list")
                | ("test-exists", "test-ref")
                | ("enum-variant-exists", "string")
                | ("unique-fn-name", "ident")
                | ("rust-file", "path")
        );
        if !compatible {
            return Err(StoreError::InvalidForm(format!(
                "validator `{validator}` is not supported for type `{}` on field `{}`",
                field.field_type, field.name
            )));
        }
    }
    Ok(field)
}

fn property<'a>(text: &'a str, context: &str) -> Result<(&'a str, &'a str), StoreError> {
    text.split_once(':')
        .map(|(key, value)| (key.trim(), value.trim()))
        .ok_or_else(|| StoreError::InvalidForm(format!("invalid {context} line `{text}`")))
}

fn parse_list(value: &str, context: &str) -> Result<Vec<String>, StoreError> {
    let value = value.trim();
    let Some(inner) = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    else {
        return Err(StoreError::InvalidForm(format!(
            "{context} must be an inline list"
        )));
    };
    if inner.trim().is_empty() {
        return Ok(Vec::new());
    }
    inner
        .split(',')
        .map(|item| parse_scalar(item.trim(), context))
        .collect()
}

fn parse_scalar(value: &str, context: &str) -> Result<String, StoreError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(StoreError::InvalidForm(format!("empty {context}")));
    }
    if value.starts_with('\'') || value.ends_with('\'') {
        if value.len() < 2 || !value.starts_with('\'') || !value.ends_with('\'') {
            return Err(StoreError::InvalidForm(format!(
                "unterminated quoted {context}"
            )));
        }
        return Ok(value[1..value.len() - 1].replace("''", "'"));
    }
    if value.starts_with('"') || value.ends_with('"') {
        if value.len() < 2 || !value.starts_with('"') || !value.ends_with('"') {
            return Err(StoreError::InvalidForm(format!(
                "unterminated quoted {context}"
            )));
        }
        return Ok(value[1..value.len() - 1].to_owned());
    }
    Ok(value.to_owned())
}

fn strip_comment(line: &str) -> &str {
    let mut single = false;
    let mut double = false;
    for (index, character) in line.char_indices() {
        match character {
            '\'' if !double => single = !single,
            '"' if !single => double = !double,
            '#' if !single && !double => return &line[..index],
            _ => {}
        }
    }
    line
}

fn safe_form_kind(kind: &str) -> bool {
    !kind.is_empty()
        && kind
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @vtest.id TEST-STORE-007
    /// @vtest.covers VO-STORE-003
    /// @vtest.target crates/vtest-store/src/forms.rs::parse_form_schema
    /// @vtest.intent Built-in unit form parses to expected kind, field count, and template
    #[test]
    fn built_in_unit_form_parses() {
        let form = parse_form_schema(RUST_UNIT_FUNCTION_FORM).unwrap();
        assert_eq!(form.kind, "rust-unit-function");
        assert_eq!(form.fields.len(), 8);
        assert!(form.template.contains("@vtest.target {target}"));
    }

    /// @vtest.id TEST-STORE-008
    /// @vtest.covers VO-STORE-004
    /// @vtest.target crates/vtest-store/src/forms.rs::parse_form_answers
    /// @vtest.intent Answers parse both inline and block list forms into FormValue::List
    #[test]
    fn answers_support_inline_and_block_lists() {
        let parsed = parse_form_answers(
            "form: rust-unit-function\nanswers:\n  target: src/lib.rs::add\n  covers:\n    - VO-CALC-ADD\n  behavior: addition\n",
        )
        .unwrap();
        assert_eq!(parsed.form, "rust-unit-function");
        assert_eq!(
            parsed.answers.get("covers"),
            Some(&FormValue::List(vec!["VO-CALC-ADD".to_owned()]))
        );
    }

    /// @vtest.id TEST-STORE-009
    /// @vtest.covers VO-STORE-003
    /// @vtest.target crates/vtest-store/src/forms.rs::parse_form_schema
    /// @vtest.intent Unknown form field property causes parse_form_schema to error
    #[test]
    fn unknown_form_field_property_is_rejected() {
        let invalid = RUST_UNIT_FUNCTION_FORM.replace(
            "    required: true\n    validate: [symbol-exists]",
            "    required: true\n    surprise: yes\n    validate: [symbol-exists]",
        );
        assert!(parse_form_schema(&invalid).is_err());
    }
}
