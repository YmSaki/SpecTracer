use std::{collections::BTreeMap, fs, path::Path};

use serde::Serialize;
use syn::spanned::Spanned;
use vtest_model::{
    CheckValue, ContentHash, Diagnostic, SourceLocation, TargetRef, TestEntity, TestResult,
};
use vtest_store::{
    load_config, load_form_schema, read_entity_ids, read_evidence, read_record_ids, write_atomic,
    yaml_scalar_value, FormAnswers, FormSchema, FormValue, VerifyLayout,
};

use crate::operations_support::*;
use crate::{Locator, ScanResult, RUST_ADAPTER_ID};

#[derive(Clone, Debug, Serialize)]
pub struct TestSelection {
    pub tests: Vec<TestEntity>,
    pub unregistered: Vec<SourceLocation>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TestView {
    #[serde(flatten)]
    pub test: TestEntity,
    pub audits: Vec<AuditState>,
    pub evidence: Vec<EvidenceState>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AuditState {
    pub id: String,
    pub kind: Option<String>,
    pub verdict: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct EvidenceState {
    pub id: String,
    pub result: TestResult,
    pub target_execution: CheckValue,
    pub executed_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct TestMutationResult {
    pub test_id: String,
    pub file: String,
    pub dry_run: bool,
    pub changed: bool,
    pub start_byte: usize,
    pub end_byte: usize,
    pub rendered: String,
}

pub fn create_test(
    root: &Path,
    form_kind: &str,
    supplied: &FormAnswers,
    explicit_id: Option<&str>,
    dry_run: bool,
) -> Result<TestMutationResult, Diagnostic> {
    let scan = crate::scan_project(root)
        .map_err(|error| Diagnostic::error("E-CORE-001", error.to_string()))?;
    let layout = VerifyLayout::new(root);
    let config =
        load_config(root).map_err(|error| Diagnostic::error("E-CORE-001", error.to_string()))?;
    let includes = &config.rust_cargo().scan.include;
    let schema = load_form_schema(&layout, form_kind)
        .map_err(|error| Diagnostic::error("E-OP-001", error.to_string()))?;
    let answers = validate_form_answers(includes, root, &schema, supplied, &scan)?;
    let test_id = select_test_id(&scan, &answers, explicit_id)?;
    let file = destination_file(supplied)?;
    validate_rust_file(includes, root, &file)?;
    let path = root.join(Path::new(&file));
    let original = fs::read_to_string(&path)
        .map_err(|error| Diagnostic::error("E-CORE-001", error.to_string()))?;
    let rendered = render_form_template(&schema, &answers, &test_id)?;
    syn::parse_str::<syn::ItemFn>(&rendered).map_err(|error| {
        Diagnostic::error(
            "E-OP-001",
            format!("form rendered an invalid Rust test function: {error}"),
        )
    })?;
    let separator = if original.is_empty() || original.ends_with("\n\n") {
        String::new()
    } else if original.ends_with('\n') {
        "\n".to_owned()
    } else {
        "\n\n".to_owned()
    };
    let start_byte = original.len() + separator.len();
    let prospective = format!("{original}{separator}{rendered}");
    syn::parse_file(&prospective).map_err(|error| {
        Diagnostic::error(
            "E-OP-001",
            format!("test insertion would make `{file}` invalid Rust: {error}"),
        )
    })?;
    let result = TestMutationResult {
        test_id: test_id.clone(),
        file: file.clone(),
        dry_run,
        changed: true,
        start_byte,
        end_byte: prospective.len(),
        rendered,
    };
    if dry_run {
        return Ok(result);
    }

    let before_hashes = test_hashes(&scan);
    write_atomic(&path, &prospective)
        .map_err(|error| Diagnostic::error("E-CORE-001", error.to_string()))?;
    let verified = verify_create(root, &test_id, &answers, &before_hashes);
    if let Err(diagnostic) = verified {
        rollback(&path, &original, diagnostic)?;
    }
    Ok(result)
}

pub fn parse_test_set_values(values: &[String]) -> Result<BTreeMap<String, FormValue>, Diagnostic> {
    let mut parsed = BTreeMap::new();
    for value in values {
        let Some((name, value)) = value.split_once('=') else {
            return Err(Diagnostic::error(
                "E-OP-001",
                format!("--set requires field=value, got `{value}`"),
            ));
        };
        let name = name.trim();
        if name.is_empty() || parsed.contains_key(name) {
            return Err(Diagnostic::error(
                "E-OP-001",
                format!("duplicate or empty --set field `{name}`"),
            ));
        }
        let value = value.trim();
        if value.is_empty() {
            return Err(Diagnostic::error(
                "E-OP-001",
                format!("--set field `{name}` has an empty value"),
            ));
        }
        let value = if matches!(name, "covers" | "targets" | "case" | "related") {
            FormValue::List(
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(str::to_owned)
                    .collect(),
            )
        } else {
            FormValue::Scalar(value.to_owned())
        };
        parsed.insert(name.to_owned(), value);
    }
    Ok(parsed)
}

pub fn edit_test(
    root: &Path,
    test_id: &str,
    supplied: Option<&FormAnswers>,
    set: &BTreeMap<String, FormValue>,
    body: Option<&str>,
    dry_run: bool,
) -> Result<TestMutationResult, Diagnostic> {
    if supplied.is_some() && !set.is_empty() {
        return Err(Diagnostic::error(
            "E-OP-001",
            "test edit accepts either --answers or --set, not both",
        ));
    }
    if supplied.is_none() && set.is_empty() && body.is_none() {
        return Err(Diagnostic::error(
            "E-OP-001",
            "test edit requires --answers, --set, or --body-file",
        ));
    }
    let scan = crate::scan_project(root)
        .map_err(|error| Diagnostic::error("E-CORE-001", error.to_string()))?;
    let current = scan
        .tests
        .iter()
        .find(|test| test.id.as_str() == test_id)
        .cloned()
        .ok_or_else(|| {
            Diagnostic::error("E-OP-002", format!("Test `{test_id}` could not be located"))
                .with_candidates(test_id_candidates(&scan.tests, test_id))
        })?;
    let mut desired = DesiredTest::from_current(&current);
    if let Some(supplied) = supplied {
        let layout = VerifyLayout::new(root);
        let config = load_config(root)
            .map_err(|error| Diagnostic::error("E-CORE-001", error.to_string()))?;
        let includes = &config.rust_cargo().scan.include;
        let schema = load_form_schema(&layout, &supplied.form)
            .map_err(|error| Diagnostic::error("E-OP-001", error.to_string()))?;
        let answers =
            validate_form_answers_for(includes, root, &schema, supplied, &scan, Some(test_id))?;
        desired.apply_complete_answers(&schema, &answers)?;
    } else {
        desired.apply_sets(set)?;
    }
    validate_desired_test(root, &scan, &current, &desired)?;

    let path = root.join(Path::new(current.location.path.as_str()));
    let original = fs::read_to_string(&path)
        .map_err(|error| Diagnostic::error("E-CORE-001", error.to_string()))?;
    let range = current.location.byte_range.start..current.location.byte_range.end;
    let current_slice = original.get(range.clone()).ok_or_else(|| {
        Diagnostic::error(
            "E-OP-002",
            format!("Test `{test_id}` source range is stale"),
        )
    })?;
    let refreshed = crate::scan_project(root)
        .map_err(|error| Diagnostic::error("E-CORE-001", error.to_string()))?;
    let unchanged = refreshed.tests.iter().any(|test| {
        test.id == current.id
            && test.location == current.location
            && test.content_hash == current.content_hash
    });
    if !unchanged {
        return Err(Diagnostic::error(
            "E-OP-002",
            format!("Test `{test_id}` changed before edit could be applied"),
        ));
    }
    let indent = line_indent(&original, current.location.byte_range.start);
    let normalized_current = deindent(current_slice, &indent);
    let normalized_replacement = render_edited_test(&normalized_current, &current, &desired, body)?;
    let rendered = indent_multiline(&normalized_replacement, &indent);
    let prospective = format!(
        "{}{}{}",
        &original[..current.location.byte_range.start],
        rendered,
        &original[current.location.byte_range.end..]
    );
    syn::parse_file(&prospective).map_err(|error| {
        Diagnostic::error(
            "E-OP-003",
            format!("edited Test `{test_id}` would make the file invalid: {error}"),
        )
    })?;
    let changed = prospective != original;
    let result = TestMutationResult {
        test_id: test_id.to_owned(),
        file: current.location.path.as_str().to_owned(),
        dry_run,
        changed,
        start_byte: current.location.byte_range.start,
        end_byte: current.location.byte_range.start + rendered.len(),
        rendered,
    };
    if dry_run || !changed {
        return Ok(result);
    }
    let before_hashes = test_hashes(&scan);
    let latest = fs::read_to_string(&path)
        .map_err(|error| Diagnostic::error("E-CORE-001", error.to_string()))?;
    if latest != original {
        return Err(Diagnostic::error(
            "E-OP-002",
            format!("Test `{test_id}` file changed concurrently"),
        ));
    }
    write_atomic(&path, &prospective)
        .map_err(|error| Diagnostic::error("E-CORE-001", error.to_string()))?;
    let verified = verify_edit(root, test_id, &desired, &before_hashes);
    if let Err(diagnostic) = verified {
        rollback(&path, &original, diagnostic)?;
    }
    Ok(result)
}

#[derive(Clone, Debug)]
struct DesiredTest {
    id: String,
    covers: Vec<String>,
    targets: Vec<String>,
    intent: String,
    input: Option<String>,
    expect: Option<String>,
    kind: Option<String>,
    cases: Vec<String>,
    related: Vec<String>,
    fn_name: String,
    file: String,
}

impl DesiredTest {
    fn from_current(test: &TestEntity) -> Self {
        let targets = test.targets.iter().map(target_string).collect();
        Self {
            id: test.id.as_str().to_owned(),
            covers: test
                .covers
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect(),
            targets,
            intent: test.intent.clone(),
            input: test.input.clone(),
            expect: test.expect.clone(),
            kind: test.kind.clone(),
            cases: test.cases.clone(),
            related: test
                .related
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect(),
            fn_name: test.execution.selector.clone(),
            file: test.location.path.as_str().to_owned(),
        }
    }

    fn apply_complete_answers(
        &mut self,
        schema: &FormSchema,
        answers: &BTreeMap<String, FormValue>,
    ) -> Result<(), Diagnostic> {
        self.covers = list_answer(answers, "covers")?;
        self.targets = if answers.contains_key("targets") {
            list_answer(answers, "targets")?
        } else {
            vec![scalar_answer(answers, "target")?]
        };
        self.intent = scalar_answer(answers, "behavior")?;
        self.input = answers.get("input").map(FormValue::render);
        self.expect = answers.get("expect").map(FormValue::render);
        self.fn_name = scalar_answer(answers, "fn_name")?;
        self.cases.clear();
        self.related.clear();
        if let Some(file) = answers.get("file") {
            self.file = file.render().replace('\\', "/");
        }
        let test_kind = answers.get("test_kind").map(FormValue::render);
        self.kind = test_kind.map(|kind| {
            if schema.kind == "rust-integration" {
                format!("integration-{kind}")
            } else {
                format!("unit-{kind}")
            }
        });
        Ok(())
    }

    fn apply_sets(&mut self, set: &BTreeMap<String, FormValue>) -> Result<(), Diagnostic> {
        const ALLOWED: &[&str] = &[
            "covers",
            "target",
            "targets",
            "intent",
            "behavior",
            "input",
            "expect",
            "kind",
            "test_kind",
            "case",
            "related",
            "fn_name",
        ];
        for (name, value) in set {
            if !ALLOWED.contains(&name.as_str()) {
                return Err(Diagnostic::error(
                    "E-OP-001",
                    format!("test edit cannot set field `{name}`"),
                ));
            }
            match name.as_str() {
                "covers" => self.covers = value_strings(value),
                "target" => self.targets = vec![value.render()],
                "targets" => self.targets = value_strings(value),
                "intent" | "behavior" => self.intent = value.render(),
                "input" => self.input = Some(value.render()),
                "expect" => self.expect = Some(value.render()),
                "kind" => self.kind = Some(value.render()),
                "test_kind" => {
                    let prefix = if self.targets.len() > 1 {
                        "integration"
                    } else {
                        "unit"
                    };
                    self.kind = Some(format!("{prefix}-{}", value.render()));
                }
                "case" => self.cases = value_strings(value),
                "related" => self.related = value_strings(value),
                "fn_name" => self.fn_name = value.render(),
                _ => unreachable!(),
            }
        }
        Ok(())
    }
}

fn validate_desired_test(
    root: &Path,
    scan: &ScanResult,
    current: &TestEntity,
    desired: &DesiredTest,
) -> Result<(), Diagnostic> {
    if desired.id != current.id.as_str() {
        return Err(Diagnostic::error(
            "E-OP-001",
            "Structured Edit cannot change a Test ID",
        ));
    }
    if desired.file != current.location.path.as_str() {
        return Err(Diagnostic::error(
            "E-OP-003",
            "Structured Edit cannot move a Test to another file",
        ));
    }
    if desired.covers.is_empty() || desired.covers.iter().any(|id| !id.starts_with("VO-")) {
        return Err(Diagnostic::error(
            "E-OP-001",
            "covers must contain at least one VO ID",
        ));
    }
    let entity_ids = read_entity_ids(root).map_err(|error| {
        Diagnostic::error("E-CORE-001", format!("could not read entity IDs: {error}"))
    })?;
    for id in &desired.covers {
        if !entity_ids[2].iter().any(|candidate| candidate == id) {
            return Err(
                Diagnostic::error("E-OP-001", format!("VO `{id}` does not exist"))
                    .with_candidates(id_candidates(&entity_ids[2], id)),
            );
        }
    }
    if desired.targets.is_empty() {
        return Err(Diagnostic::error(
            "E-OP-001",
            "target must contain at least one source locator",
        ));
    }
    if desired.targets.len() > 1
        && !desired
            .kind
            .as_deref()
            .is_some_and(|kind| kind.starts_with("integration"))
    {
        return Err(Diagnostic::error(
            "E-OP-001",
            "multiple targets are allowed only for integration tests",
        ));
    }
    for target in &desired.targets {
        let Some(locator) = Locator::parse(target) else {
            return Err(Diagnostic::error(
                "E-OP-001",
                format!("invalid source locator `{target}`"),
            )
            .with_candidates(symbol_candidates(&scan.sources, target)));
        };
        if !scan
            .sources
            .iter()
            .any(|source| source_rust_locator(source).as_ref() == Some(&locator))
        {
            return Err(Diagnostic::error(
                "E-OP-001",
                format!("source symbol `{target}` does not exist"),
            )
            .with_candidates(symbol_candidates(&scan.sources, target)));
        }
    }
    if desired.intent.trim().is_empty() {
        return Err(Diagnostic::error("E-OP-001", "intent must not be empty"));
    }
    if syn::parse_str::<syn::Ident>(&desired.fn_name).is_err() {
        return Err(Diagnostic::error(
            "E-OP-001",
            format!("`{}` is not a Rust identifier", desired.fn_name),
        ));
    }
    if scan.sources.iter().any(|source| {
        source.location.path.as_str() == desired.file
            && source_rust_locator(source)
                .is_some_and(|locator| locator.item_path == desired.fn_name)
            && source.location != current.location
    }) {
        return Err(Diagnostic::error(
            "E-OP-001",
            format!(
                "function `{}` already exists in `{}`",
                desired.fn_name, desired.file
            ),
        ));
    }
    for related in &desired.related {
        if related == current.id.as_str() {
            continue;
        }
        if !scan.tests.iter().any(|test| test.id.as_str() == related) {
            return Err(Diagnostic::error(
                "E-OP-001",
                format!("related Test `{related}` does not exist"),
            )
            .with_candidates(test_id_candidates(&scan.tests, related)));
        }
    }
    Ok(())
}

fn render_edited_test(
    current_source: &str,
    current: &TestEntity,
    desired: &DesiredTest,
    body: Option<&str>,
) -> Result<String, Diagnostic> {
    let mut free_before = Vec::new();
    let mut free_after = Vec::new();
    let mut tail = Vec::new();
    let mut in_tail = false;
    let mut saw_annotation = false;
    for line in current_source.lines() {
        let trimmed = line.trim_start();
        if !in_tail && trimmed.starts_with("///") {
            if trimmed.contains("@vtest.") {
                saw_annotation = true;
            } else if saw_annotation {
                free_after.push(line.to_owned());
            } else {
                free_before.push(line.to_owned());
            }
        } else {
            in_tail = true;
            tail.push(line.to_owned());
        }
    }
    if tail.is_empty() {
        return Err(Diagnostic::error(
            "E-OP-002",
            format!(
                "Test `{}` has no function body in its source range",
                current.id
            ),
        ));
    }
    let mut function = tail.join("\n");
    if desired.fn_name != current.execution.selector {
        let from = format!("fn {}", current.execution.selector);
        let to = format!("fn {}", desired.fn_name);
        if !function.contains(&from) {
            return Err(Diagnostic::error(
                "E-OP-002",
                format!(
                    "Test `{}` function signature could not be located",
                    current.id
                ),
            ));
        }
        function = function.replacen(&from, &to, 1);
    }
    if let Some(body) = body {
        let body = normalize_body(body)?;
        let parsed = syn::parse_str::<syn::ItemFn>(&function).map_err(|error| {
            Diagnostic::error(
                "E-OP-002",
                format!("Test function could not be reparsed before body edit: {error}"),
            )
        })?;
        let span = parsed.block.span();
        let start = source_offset(&function, span.start().line, span.start().column)
            .ok_or_else(|| Diagnostic::error("E-OP-002", "Test body start is out of range"))?;
        let end = source_offset(&function, span.end().line, span.end().column)
            .ok_or_else(|| Diagnostic::error("E-OP-002", "Test body end is out of range"))?;
        if start >= end || function.get(start..end).is_none() {
            return Err(Diagnostic::error(
                "E-OP-002",
                "Test function body range is invalid",
            ));
        }
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

fn normalize_body(body: &str) -> Result<String, Diagnostic> {
    let body = body.trim();
    let body = if body.starts_with('{') && body.ends_with('}') {
        body.to_owned()
    } else {
        format!("{{\n{body}\n}}")
    };
    syn::parse_str::<syn::Block>(&body).map_err(|error| {
        Diagnostic::error(
            "E-OP-001",
            format!("body file does not contain a valid Rust block: {error}"),
        )
    })?;
    Ok(body)
}

fn verify_edit(
    root: &Path,
    test_id: &str,
    desired: &DesiredTest,
    before_hashes: &BTreeMap<String, ContentHash>,
) -> Result<(), Diagnostic> {
    let after = crate::scan_project(root).map_err(|error| {
        Diagnostic::error(
            "E-OP-003",
            format!("edited test could not be rescanned: {error}"),
        )
    })?;
    let edited = after
        .tests
        .iter()
        .find(|test| test.id.as_str() == test_id)
        .ok_or_else(|| {
            Diagnostic::error(
                "E-OP-003",
                format!("edited Test `{test_id}` was not recognized by the scanner"),
            )
        })?;
    if !test_matches_desired(edited, desired) {
        return Err(Diagnostic::error(
            "E-OP-003",
            format!("edited Test `{test_id}` does not match the desired state"),
        ));
    }
    verify_other_test_hashes(&after, before_hashes, Some(test_id))
}

fn test_matches_desired(test: &TestEntity, desired: &DesiredTest) -> bool {
    test.id.as_str() == desired.id
        && test.covers.iter().map(|id| id.as_str()).collect::<Vec<_>>()
            == desired
                .covers
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
        && test.targets.iter().map(target_string).collect::<Vec<_>>() == desired.targets
        && test.intent == desired.intent
        && test.input == desired.input
        && test.expect == desired.expect
        && test.kind == desired.kind
        && test.cases == desired.cases
        && test
            .related
            .iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>()
            == desired
                .related
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
        && test.execution.selector == desired.fn_name
        && test.location.path.as_str() == desired.file
}

fn line_indent(source: &str, byte: usize) -> String {
    let line_start = source[..byte].rfind('\n').map_or(0, |index| index + 1);
    source[line_start..byte].to_owned()
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

fn deindent(source: &str, indent: &str) -> String {
    source
        .lines()
        .enumerate()
        .map(|(index, line)| {
            if index == 0 {
                line
            } else {
                line.strip_prefix(indent).unwrap_or(line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn indent_multiline(source: &str, indent: &str) -> String {
    source.replace('\n', &format!("\n{indent}"))
}

fn target_string(target: &TargetRef) -> String {
    match target {
        TargetRef::Locator { adapter, value } if adapter.as_str() == RUST_ADAPTER_ID => {
            value.clone()
        }
        TargetRef::Locator { adapter, value } => format!("{adapter}::{value}"),
        TargetRef::SrcId(id) => id.as_str().to_owned(),
    }
}

fn value_strings(value: &FormValue) -> Vec<String> {
    match value {
        FormValue::List(values) => values.clone(),
        FormValue::Scalar(value) => vec![value.clone()],
    }
}

fn list_answer(
    answers: &BTreeMap<String, FormValue>,
    name: &str,
) -> Result<Vec<String>, Diagnostic> {
    match answers.get(name) {
        Some(FormValue::List(values)) if !values.is_empty() => Ok(values.clone()),
        _ => Err(Diagnostic::error(
            "E-OP-001",
            format!("desired state requires list answer `{name}`"),
        )),
    }
}

fn scalar_answer(answers: &BTreeMap<String, FormValue>, name: &str) -> Result<String, Diagnostic> {
    match answers.get(name) {
        Some(FormValue::Scalar(value)) if !value.trim().is_empty() => Ok(value.clone()),
        _ => Err(Diagnostic::error(
            "E-OP-001",
            format!("desired state requires scalar answer `{name}`"),
        )),
    }
}

pub fn show_test(root: &Path, scan: &ScanResult, id: &str) -> Result<TestView, Diagnostic> {
    let test = scan
        .tests
        .iter()
        .find(|test| test.id.as_str() == id)
        .cloned()
        .ok_or_else(|| {
            Diagnostic::error("E-OP-001", format!("Test `{id}` does not exist"))
                .with_candidates(test_id_candidates(&scan.tests, id))
        })?;
    let layout = VerifyLayout::new(root);
    let mut audits = Vec::new();
    let audit_ids = read_record_ids(&layout.audits_dir())
        .map_err(|error| Diagnostic::error("E-CORE-001", error.to_string()))?;
    for record_id in audit_ids {
        let path = layout.audits_dir().join(format!("{record_id}.yaml"));
        let text = fs::read_to_string(&path)
            .map_err(|error| Diagnostic::error("E-CORE-001", error.to_string()))?;
        if yaml_scalar_value(&text, "test_id").as_deref() == Some(id)
            || audit_mentions_test(&text, id)
        {
            audits.push(AuditState {
                id: record_id,
                kind: yaml_scalar_value(&text, "kind"),
                verdict: yaml_scalar_value(&text, "verdict"),
            });
        }
    }
    let mut evidence = Vec::new();
    let evidence_ids = read_record_ids(&layout.evidence_dir())
        .map_err(|error| Diagnostic::error("E-CORE-001", error.to_string()))?;
    for record_id in evidence_ids {
        let path = layout.evidence_dir().join(format!("{record_id}.yaml"));
        let record = read_evidence(&path)
            .map_err(|error| Diagnostic::error("E-CORE-001", error.to_string()))?;
        if record.test_id.as_str() == id {
            evidence.push(EvidenceState {
                id: record.id,
                result: record.result,
                target_execution: if record.target_execution.checked {
                    record
                        .target_execution
                        .result
                        .unwrap_or(CheckValue::Unknown)
                } else {
                    CheckValue::NotChecked
                },
                executed_at: record.executed_at,
            });
        }
    }
    evidence.sort_by(|left, right| left.executed_at.cmp(&right.executed_at));
    Ok(TestView {
        test,
        audits,
        evidence,
    })
}

fn audit_mentions_test(text: &str, test_id: &str) -> bool {
    let mut test_subject = false;
    for raw in text.lines() {
        let trimmed = raw.trim().trim_start_matches('-').trim();
        if let Some(value) = trimmed.strip_prefix("kind:") {
            test_subject = value.trim().trim_matches(['\'', '"']) == "test";
            continue;
        }
        if test_subject {
            if let Some(value) = trimmed.strip_prefix("id:") {
                if value.trim().trim_matches(['\'', '"']) == test_id {
                    return true;
                }
                test_subject = false;
            }
        }
    }
    false
}

pub fn list_tests(
    scan: &ScanResult,
    vo: Option<&str>,
    include_unregistered: bool,
) -> TestSelection {
    let mut tests = scan
        .tests
        .iter()
        .filter(|test| vo.is_none_or(|vo| test.covers.iter().any(|covered| covered.as_str() == vo)))
        .cloned()
        .collect::<Vec<_>>();
    tests.sort_by(|left, right| left.id.as_str().cmp(right.id.as_str()));
    let mut unregistered = if include_unregistered {
        scan.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "W-SCAN-101")
            .filter_map(|diagnostic| diagnostic.location.as_deref().cloned())
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    unregistered.sort_by(|left, right| {
        (left.path.as_str(), left.byte_range.start)
            .cmp(&(right.path.as_str(), right.byte_range.start))
    });
    TestSelection {
        tests,
        unregistered,
    }
}

pub fn query_tests(scan: &ScanResult, source: &str) -> Result<Vec<TestEntity>, Diagnostic> {
    let Some(locator) = Locator::parse(source) else {
        return Err(
            Diagnostic::error("E-OP-001", format!("invalid source locator `{source}`"))
                .with_candidates(symbol_candidates(&scan.sources, source)),
        );
    };
    if !scan
        .sources
        .iter()
        .any(|item| source_rust_locator(item).as_ref() == Some(&locator))
    {
        return Err(Diagnostic::error(
            "E-OP-001",
            format!("source symbol `{source}` does not exist"),
        )
        .with_candidates(symbol_candidates(&scan.sources, source)));
    }
    let mut tests = scan
        .tests
        .iter()
        .filter(|test| {
            test.targets
                .iter()
                .any(|target| rust_locator(target).as_ref() == Some(&locator))
        })
        .cloned()
        .collect::<Vec<_>>();
    tests.sort_by(|left, right| left.id.as_str().cmp(right.id.as_str()));
    Ok(tests)
}

pub fn validate_form_answers(
    includes: &[String],
    root: &Path,
    schema: &FormSchema,
    supplied: &FormAnswers,
    scan: &ScanResult,
) -> Result<BTreeMap<String, FormValue>, Diagnostic> {
    validate_form_answers_for(includes, root, schema, supplied, scan, None)
}

fn validate_form_answers_for(
    includes: &[String],
    root: &Path,
    schema: &FormSchema,
    supplied: &FormAnswers,
    scan: &ScanResult,
    edited_test_id: Option<&str>,
) -> Result<BTreeMap<String, FormValue>, Diagnostic> {
    if supplied.form != schema.kind {
        return Err(Diagnostic::error(
            "E-OP-001",
            format!(
                "answers form `{}` does not match requested form `{}`",
                supplied.form, schema.kind
            ),
        ));
    }
    let known = schema
        .fields
        .iter()
        .map(|field| field.name.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if let Some(unknown) = supplied
        .answers
        .keys()
        .find(|name| !known.contains(name.as_str()))
    {
        return Err(Diagnostic::error(
            "E-OP-001",
            format!("unknown answer field `{unknown}`"),
        ));
    }
    let entity_ids = read_entity_ids(root).map_err(|error| {
        Diagnostic::error("E-CORE-001", format!("could not read entity IDs: {error}"))
    })?;
    for field in &schema.fields {
        let value = supplied.answers.get(&field.name);
        if field.required && value.is_none_or(FormValue::is_empty) {
            return Err(Diagnostic::error(
                "E-OP-001",
                format!("required answer `{}` is missing", field.name),
            ));
        }
        let Some(value) = value else { continue };
        validate_value_shape(field, value)?;
        for validator in &field.validate {
            match validator.as_str() {
                "symbol-exists" => {
                    validate_symbols(&scan.sources, &field.name, value, false)?;
                }
                "symbols-exist" => {
                    validate_symbols(&scan.sources, &field.name, value, true)?;
                }
                "vo-exists" => {
                    for id in value.values() {
                        if !entity_ids[2].iter().any(|candidate| candidate == id) {
                            return Err(Diagnostic::error(
                                "E-OP-001",
                                format!("VO `{id}` does not exist"),
                            )
                            .with_candidates(id_candidates(&entity_ids[2], id)));
                        }
                    }
                }
                "test-exists" => {
                    for id in value.values() {
                        if !scan.tests.iter().any(|test| test.id.as_str() == id) {
                            return Err(Diagnostic::error(
                                "E-OP-001",
                                format!("Test `{id}` does not exist"),
                            )
                            .with_candidates(test_id_candidates(&scan.tests, id)));
                        }
                    }
                }
                "unique-fn-name" => {
                    let name = scalar(value, &field.name)?;
                    let edited_location = edited_test_id.and_then(|id| {
                        scan.tests
                            .iter()
                            .find(|test| test.id.as_str() == id)
                            .map(|test| &test.location)
                    });
                    let destination = if supplied.answers.contains_key("file") {
                        destination_file(supplied)?
                    } else if let Some(location) = edited_location {
                        location.path.as_str().to_owned()
                    } else {
                        destination_file(supplied)?
                    };
                    if scan.sources.iter().any(|source| {
                        source.location.path.as_str() == destination
                            && source_rust_locator(source)
                                .is_some_and(|locator| locator.item_path == name)
                            && edited_location != Some(&source.location)
                    }) {
                        return Err(Diagnostic::error(
                            "E-OP-001",
                            format!("function `{name}` already exists in `{destination}`"),
                        ));
                    }
                }
                "rust-file" => {
                    let relative = scalar(value, &field.name)?;
                    validate_rust_file(includes, root, relative)?;
                }
                "enum-variant-exists" => {
                    validate_enum_variant(includes, root, scalar(value, &field.name)?)?;
                }
                unknown => {
                    return Err(Diagnostic::error(
                        "E-OP-001",
                        format!(
                            "form `{}` uses unsupported validator `{unknown}`",
                            schema.kind
                        ),
                    ));
                }
            }
        }
    }
    Ok(supplied.answers.clone())
}

fn select_test_id(
    scan: &ScanResult,
    answers: &BTreeMap<String, FormValue>,
    explicit_id: Option<&str>,
) -> Result<String, Diagnostic> {
    if let Some(id) = explicit_id {
        if id.trim().is_empty() || id.chars().any(char::is_whitespace) {
            return Err(Diagnostic::error(
                "E-OP-001",
                "Test ID must be non-empty and contain no whitespace",
            ));
        }
        if scan.tests.iter().any(|test| test.id.as_str() == id) {
            return Err(Diagnostic::error(
                "E-OP-001",
                format!("Test ID `{id}` already exists"),
            ));
        }
        return Ok(id.to_owned());
    }
    let covers = answers
        .get("covers")
        .ok_or_else(|| Diagnostic::error("E-OP-001", "answers require `covers`"))?;
    let first = covers
        .values()
        .into_iter()
        .next()
        .ok_or_else(|| Diagnostic::error("E-OP-001", "answers require a non-empty `covers`"))?;
    let area = first
        .strip_prefix("VO-")
        .unwrap_or(first)
        .split('-')
        .next()
        .filter(|area| !area.is_empty())
        .unwrap_or("AUTO");
    let prefix = format!("TEST-{area}-");
    let next = scan
        .tests
        .iter()
        .filter_map(|test| test.id.as_str().strip_prefix(&prefix))
        .filter_map(|suffix| suffix.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| Diagnostic::error("E-OP-001", "Test ID sequence overflow"))?;
    let id = format!("{prefix}{next:03}");
    if scan.tests.iter().any(|test| test.id.as_str() == id) {
        return Err(Diagnostic::error(
            "E-OP-001",
            format!("generated Test ID `{id}` already exists"),
        ));
    }
    Ok(id)
}

fn verify_create(
    root: &Path,
    test_id: &str,
    answers: &BTreeMap<String, FormValue>,
    before_hashes: &BTreeMap<String, ContentHash>,
) -> Result<(), Diagnostic> {
    let after = crate::scan_project(root).map_err(|error| {
        Diagnostic::error(
            "E-OP-003",
            format!("created test could not be rescanned: {error}"),
        )
    })?;
    let created = after
        .tests
        .iter()
        .find(|test| test.id.as_str() == test_id)
        .ok_or_else(|| {
            Diagnostic::error(
                "E-OP-003",
                format!("created Test `{test_id}` was not recognized by the scanner"),
            )
        })?;
    if !test_matches_answers(created, answers) {
        return Err(Diagnostic::error(
            "E-OP-003",
            format!("created Test `{test_id}` does not match the desired state"),
        ));
    }
    verify_other_test_hashes(&after, before_hashes, None)
}

fn test_matches_answers(test: &TestEntity, answers: &BTreeMap<String, FormValue>) -> bool {
    let covers_match = answers.get("covers").is_none_or(|value| {
        value.values() == test.covers.iter().map(|id| id.as_str()).collect::<Vec<_>>()
    });
    let behavior_match = answers
        .get("behavior")
        .is_none_or(|value| value.render() == test.intent);
    let target_values = test.targets.iter().map(target_string).collect::<Vec<_>>();
    let targets_match = if let Some(value) = answers.get("target") {
        value
            .values()
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>()
            == target_values
    } else if let Some(value) = answers.get("targets") {
        value
            .values()
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>()
            == target_values
    } else {
        true
    };
    covers_match && behavior_match && targets_match
}

fn test_hashes(scan: &ScanResult) -> BTreeMap<String, ContentHash> {
    scan.tests
        .iter()
        .map(|test| (test.id.as_str().to_owned(), test.content_hash.clone()))
        .collect()
}

fn verify_other_test_hashes(
    after: &ScanResult,
    before: &BTreeMap<String, ContentHash>,
    edited_id: Option<&str>,
) -> Result<(), Diagnostic> {
    for (id, expected) in before {
        if edited_id == Some(id.as_str()) {
            continue;
        }
        let actual = after
            .tests
            .iter()
            .find(|test| test.id.as_str() == id)
            .map(|test| &test.content_hash);
        if actual != Some(expected) {
            return Err(Diagnostic::error(
                "E-OP-003",
                format!("operation changed Test `{id}` outside its edit boundary"),
            ));
        }
    }
    Ok(())
}

fn rollback(path: &Path, original: &str, diagnostic: Diagnostic) -> Result<(), Diagnostic> {
    write_atomic(path, original).map_err(|error| {
        Diagnostic::error(
            "E-CORE-001",
            format!(
                "operation failed and rollback of `{}` also failed: {error}; original error: {}",
                path.display(),
                diagnostic.message
            ),
        )
    })?;
    Err(diagnostic)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_distance_handles_transcription_errors() {
        assert_eq!(edit_distance("parse", "prase"), 2);
        assert_eq!(edit_distance("add", "add"), 0);
        assert_eq!(edit_distance("subtract", "add"), 7);
    }
}
