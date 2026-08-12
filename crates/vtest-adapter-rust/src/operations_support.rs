//! Store-free Rust-specific validation and candidate helpers for the
//! rust-cargo Structured Test Operations. These move to vtest-adapter-rust
//! with the rest of the operation logic; isolating them first keeps the
//! cross-crate move mechanical.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path};

use syn::spanned::Spanned;
use vtest_model::{
    Diagnostic, FormAnswers, FormField, FormSchema, FormValue, SourceFunction, TargetRef,
    TestEntity,
};

use crate::{Locator, RUST_CARGO_ADAPTER_ID as RUST_ADAPTER_ID};

pub fn validate_value_shape(field: &FormField, value: &FormValue) -> Result<(), Diagnostic> {
    let list_type = matches!(field.field_type.as_str(), "symbol-list" | "vo-ref-list");
    if list_type != matches!(value, FormValue::List(_)) {
        return Err(Diagnostic::error(
            "E-OP-001",
            format!(
                "answer `{}` must be {}",
                field.name,
                if list_type { "a list" } else { "a scalar" }
            ),
        ));
    }
    match field.field_type.as_str() {
        "symbol" => {
            if Locator::parse(scalar(value, &field.name)?).is_none() {
                return Err(Diagnostic::error(
                    "E-OP-001",
                    format!("answer `{}` is not a source locator", field.name),
                ));
            }
        }
        "symbol-list" => {
            if value
                .values()
                .iter()
                .any(|value| Locator::parse(value).is_none())
            {
                return Err(Diagnostic::error(
                    "E-OP-001",
                    format!("answer `{}` contains an invalid source locator", field.name),
                ));
            }
        }
        "vo-ref" => {
            let value = scalar(value, &field.name)?;
            if !value.starts_with("VO-") {
                return Err(Diagnostic::error(
                    "E-OP-001",
                    format!("answer `{}` must be a VO ID", field.name),
                ));
            }
        }
        "vo-ref-list" => {
            if value.values().iter().any(|value| !value.starts_with("VO-")) {
                return Err(Diagnostic::error(
                    "E-OP-001",
                    format!("answer `{}` must contain VO IDs", field.name),
                ));
            }
        }
        "test-ref" => {
            let value = scalar(value, &field.name)?;
            if !value.starts_with("TEST-") {
                return Err(Diagnostic::error(
                    "E-OP-001",
                    format!("answer `{}` must be a Test ID", field.name),
                ));
            }
        }
        "enum" => {
            let value = scalar(value, &field.name)?;
            if !field.options.iter().any(|option| option == value) {
                return Err(Diagnostic::error(
                    "E-OP-001",
                    format!("invalid value `{value}` for `{}`", field.name),
                )
                .with_candidates(field.options.clone()));
            }
        }
        "ident" => {
            let value = scalar(value, &field.name)?;
            if syn::parse_str::<syn::Ident>(value).is_err() {
                return Err(Diagnostic::error(
                    "E-OP-001",
                    format!("answer `{}` is not a Rust identifier", field.name),
                ));
            }
        }
        "path" | "string" => {
            let _ = scalar(value, &field.name)?;
        }
        _ => {}
    }
    Ok(())
}

pub fn validate_symbols(
    sources: &[SourceFunction],
    field: &str,
    value: &FormValue,
    expect_list: bool,
) -> Result<(), Diagnostic> {
    if expect_list != matches!(value, FormValue::List(_)) {
        return Err(Diagnostic::error(
            "E-OP-001",
            format!("answer `{field}` has the wrong type"),
        ));
    }
    for symbol in value.values() {
        let Some(locator) = Locator::parse(symbol) else {
            return Err(Diagnostic::error(
                "E-OP-001",
                format!("invalid source locator `{symbol}`"),
            )
            .with_candidates(symbol_candidates(sources, symbol)));
        };
        if !sources
            .iter()
            .any(|source| source_rust_locator(source).as_ref() == Some(&locator))
        {
            return Err(Diagnostic::error(
                "E-OP-001",
                format!("source symbol `{symbol}` does not exist"),
            )
            .with_candidates(symbol_candidates(sources, symbol)));
        }
    }
    Ok(())
}

pub fn scalar<'a>(value: &'a FormValue, field: &str) -> Result<&'a str, Diagnostic> {
    match value {
        FormValue::Scalar(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(Diagnostic::error(
            "E-OP-001",
            format!("answer `{field}` must be a non-empty scalar"),
        )),
    }
}

pub fn destination_file(answers: &FormAnswers) -> Result<String, Diagnostic> {
    if let Some(value) = answers.answers.get("file") {
        return scalar(value, "file").map(|value| value.replace('\\', "/"));
    }
    if let Some(value) = answers.answers.get("target") {
        return Locator::parse(scalar(value, "target")?)
            .map(|locator| locator.path)
            .ok_or_else(|| Diagnostic::error("E-OP-001", "target is not a locator"));
    }
    Err(Diagnostic::error(
        "E-OP-001",
        "answers require `file` when no single target is present",
    ))
}

pub fn validate_rust_file(
    includes: &[String],
    root: &Path,
    relative: &str,
) -> Result<(), Diagnostic> {
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
        || relative_path.extension().and_then(|value| value.to_str()) != Some("rs")
    {
        return Err(Diagnostic::error(
            "E-OP-001",
            format!("Rust file must be a project-relative .rs path: `{relative}`"),
        ));
    }
    let path = root.join(relative_path);
    if !path.is_file() {
        return Err(Diagnostic::error(
            "E-OP-001",
            format!("Rust file does not exist: `{relative}`"),
        ));
    }
    if !includes
        .iter()
        .any(|include| relative_path.starts_with(Path::new(include)))
    {
        return Err(Diagnostic::error(
            "E-OP-001",
            format!("Rust file is outside rust-cargo scan.include: `{relative}`"),
        ));
    }
    let canonical_root = fs::canonicalize(root)
        .map_err(|error| Diagnostic::error("E-CORE-001", error.to_string()))?;
    let canonical_path = fs::canonicalize(&path)
        .map_err(|error| Diagnostic::error("E-CORE-001", error.to_string()))?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(Diagnostic::error(
            "E-OP-001",
            format!("Rust file resolves outside the project: `{relative}`"),
        ));
    }
    Ok(())
}

pub fn validate_enum_variant(
    includes: &[String],
    root: &Path,
    value: &str,
) -> Result<(), Diagnostic> {
    let Some((type_path, variant)) = value.rsplit_once("::") else {
        return Ok(());
    };
    let type_name = type_path.rsplit("::").next().unwrap_or(type_path);
    if syn::parse_str::<syn::Ident>(type_name).is_err()
        || syn::parse_str::<syn::Ident>(variant).is_err()
    {
        return Ok(());
    }
    let mut files = Vec::new();
    for include in includes {
        collect_rust_files(&root.join(include), &mut files);
    }
    files.sort();
    files.dedup();
    let mut variants = Vec::new();
    for file in files {
        let Ok(source) = fs::read_to_string(file) else {
            continue;
        };
        let Ok(parsed) = syn::parse_file(&source) else {
            continue;
        };
        collect_enum_variants(&parsed.items, type_name, &mut variants);
    }
    if variants.is_empty() || variants.iter().any(|candidate| candidate == variant) {
        return Ok(());
    }
    variants.sort();
    variants.dedup();
    Err(
        Diagnostic::error("E-OP-001", format!("enum variant `{value}` does not exist"))
            .with_candidates(
                variants
                    .into_iter()
                    .map(|variant| format!("{type_path}::{variant}")),
            ),
    )
}

pub fn collect_rust_files(directory: &Path, files: &mut Vec<std::path::PathBuf>) {
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

pub fn collect_enum_variants(items: &[syn::Item], type_name: &str, variants: &mut Vec<String>) {
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

pub fn symbol_candidates(sources: &[SourceFunction], requested: &str) -> Vec<String> {
    let item = requested
        .rsplit_once("::")
        .map_or(requested, |(_, item)| item);
    let mut exact_suffix = sources
        .iter()
        .filter_map(source_rust_locator)
        .filter(|locator| {
            locator
                .item_path
                .rsplit("::")
                .next()
                .is_some_and(|candidate| candidate == item)
        })
        .map(|locator| locator.as_string())
        .collect::<Vec<_>>();
    let mut near = sources
        .iter()
        .filter_map(source_rust_locator)
        .filter(|locator| {
            locator
                .item_path
                .rsplit("::")
                .next()
                .is_some_and(|candidate| edit_distance(candidate, item) <= 2)
        })
        .map(|locator| locator.as_string())
        .collect::<Vec<_>>();
    exact_suffix.sort();
    exact_suffix.dedup();
    near.sort();
    near.dedup();
    near.retain(|candidate| !exact_suffix.contains(candidate));
    exact_suffix.append(&mut near);
    exact_suffix
}

pub fn rust_locator(target: &TargetRef) -> Option<Locator> {
    match target {
        TargetRef::Locator { adapter, value } if adapter.as_str() == RUST_ADAPTER_ID => {
            Locator::parse(value)
        }
        TargetRef::Locator { .. } | TargetRef::SrcId(_) => None,
    }
}

pub fn source_rust_locator(source: &SourceFunction) -> Option<Locator> {
    rust_locator(&source.target)
}

pub fn test_id_candidates(tests: &[TestEntity], requested: &str) -> Vec<String> {
    let ids = tests
        .iter()
        .map(|test| test.id.as_str().to_owned())
        .collect::<Vec<_>>();
    id_candidates(&ids, requested)
}

pub fn id_candidates(ids: &[String], requested: &str) -> Vec<String> {
    let mut candidates = ids
        .iter()
        .filter(|candidate| edit_distance(candidate, requested) <= 2)
        .cloned()
        .collect::<Vec<_>>();
    candidates.sort();
    candidates
}

pub fn edit_distance(left: &str, right: &str) -> usize {
    let right = right.chars().collect::<Vec<_>>();
    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    for (left_index, left_char) in left.chars().enumerate() {
        let mut current = vec![left_index + 1];
        for (right_index, right_char) in right.iter().enumerate() {
            current.push(std::cmp::min(
                std::cmp::min(current[right_index] + 1, previous[right_index + 1] + 1),
                previous[right_index] + usize::from(left_char != *right_char),
            ));
        }
        previous = current;
    }
    previous[right.len()]
}

pub fn render_form_template(
    schema: &FormSchema,
    answers: &BTreeMap<String, FormValue>,
    test_id: &str,
) -> Result<String, Diagnostic> {
    let mut rendered = String::new();
    for line in schema.template.lines() {
        if line.contains("{targets}") {
            let targets = answers
                .get("targets")
                .ok_or_else(|| Diagnostic::error("E-OP-001", "form template requires `targets`"))?;
            let FormValue::List(targets) = targets else {
                return Err(Diagnostic::error(
                    "E-OP-001",
                    "form template requires `targets` to be a list",
                ));
            };
            for target in targets {
                rendered.push_str(&line.replace("{targets}", target));
                rendered.push('\n');
            }
            continue;
        }
        let mut line = line.replace("{test_id}", test_id);
        for (name, value) in answers {
            line = line.replace(&format!("{{{name}}}"), &value.render());
        }
        if let Some(placeholder) = unresolved_placeholder(&line) {
            return Err(Diagnostic::error(
                "E-OP-001",
                format!(
                    "form `{}` template requires unanswered field `{placeholder}`",
                    schema.kind
                ),
            ));
        }
        rendered.push_str(&line);
        rendered.push('\n');
    }
    Ok(rendered)
}

pub fn unresolved_placeholder(line: &str) -> Option<String> {
    let mut remainder = line;
    while let Some(start) = remainder.find('{') {
        let after = &remainder[start + 1..];
        let end = after.find('}')?;
        let candidate = &after[..end];
        if !candidate.is_empty()
            && candidate
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            return Some(candidate.to_owned());
        }
        remainder = &after[end + 1..];
    }
    None
}

/// Rust-syntax checks used by the Structured Test Operations. They return the
/// parser error text so the core caller can attach its own operation diagnostic.
pub fn check_test_item_fn(source: &str) -> Result<(), String> {
    syn::parse_str::<syn::ItemFn>(source)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub fn check_rust_file_parses(source: &str) -> Result<(), String> {
    syn::parse_file(source)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub fn is_rust_ident(source: &str) -> bool {
    syn::parse_str::<syn::Ident>(source).is_ok()
}

pub fn check_rust_block(source: &str) -> Result<(), String> {
    syn::parse_str::<syn::Block>(source)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn source_offset(source: &str, line: usize, column: usize) -> Option<usize> {
    let mut offset = 0;
    for (index, current) in source.split_inclusive('\n').enumerate() {
        if index + 1 == line {
            let body = current.strip_suffix('\n').unwrap_or(current);
            return (column <= body.len()).then_some(offset + column);
        }
        offset += current.len();
    }
    None
}

/// Locates the byte range of a rust-cargo test function's body block so the
/// core edit path can splice in a new body without parsing Rust itself. The
/// error string is the operation diagnostic message the caller reports as-is.
pub fn test_body_byte_range(function: &str) -> Result<(usize, usize), String> {
    let parsed = syn::parse_str::<syn::ItemFn>(function).map_err(|error| {
        format!("Test function could not be reparsed before body edit: {error}")
    })?;
    let span = parsed.block.span();
    let start = source_offset(function, span.start().line, span.start().column)
        .ok_or_else(|| "Test body start is out of range".to_string())?;
    let end = source_offset(function, span.end().line, span.end().column)
        .ok_or_else(|| "Test body end is out of range".to_string())?;
    if start >= end || function.get(start..end).is_none() {
        return Err("Test function body range is invalid".to_string());
    }
    Ok((start, end))
}
