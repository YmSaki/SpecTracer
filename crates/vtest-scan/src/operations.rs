use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path},
};

use serde::Serialize;
use syn::spanned::Spanned;
use vtest_adapter_rust::RustLocator;
use vtest_model::{
    CheckValue, ContentHash, Diagnostic, SourceLocation, TargetRef, TestEntity, TestResult,
};
use vtest_store::{
    load_config, load_form_schema, read_entity_ids, read_evidence, read_record_ids, write_atomic,
    yaml_scalar_value, FormAnswers, FormSchema, FormValue, VerifyLayout,
};

use crate::{adapter_scan_includes, ScanResult};

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
    let schema = load_form_schema(&layout, form_kind)
        .map_err(|error| Diagnostic::error("E-OP-001", error.to_string()))?;
    let answers = validate_form_answers(root, &schema, supplied, &scan)?;
    let test_id = select_test_id(&scan, &answers, explicit_id)?;
    let file = destination_file(supplied)?;
    validate_rust_file(root, &file)?;
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
                .with_candidates(test_id_candidates(&scan, test_id))
        })?;
    let mut desired = DesiredTest::from_current(&current);
    if let Some(supplied) = supplied {
        let layout = VerifyLayout::new(root);
        let schema = load_form_schema(&layout, &supplied.form)
            .map_err(|error| Diagnostic::error("E-OP-001", error.to_string()))?;
        let answers = validate_form_answers_for(root, &schema, supplied, &scan, Some(test_id))?;
        desired.apply_complete_answers(&schema, &answers)?;
    } else {
        desired.apply_sets(set)?;
    }
    validate_desired_test(root, &scan, &current, &desired)?;

    let path = root.join(Path::new(&current.location.file));
    let original = fs::read_to_string(&path)
        .map_err(|error| Diagnostic::error("E-CORE-001", error.to_string()))?;
    let start_byte: usize = current.location.start_byte.try_into().map_err(|_| {
        Diagnostic::error(
            "E-OP-002",
            format!("Test `{test_id}` start offset is out of range"),
        )
    })?;
    let end_byte: usize = current.location.end_byte.try_into().map_err(|_| {
        Diagnostic::error(
            "E-OP-002",
            format!("Test `{test_id}` end offset is out of range"),
        )
    })?;
    let range = start_byte..end_byte;
    let current_slice = original.get(range.clone()).ok_or_else(|| {
        Diagnostic::error(
            "E-OP-002",
            format!("Test `{test_id}` source range is stale"),
        )
    })?;
    if ContentHash::from_text(current_slice) != current.content_hash {
        return Err(Diagnostic::error(
            "E-OP-002",
            format!("Test `{test_id}` changed before edit could be applied"),
        ));
    }
    let indent = line_indent(&original, start_byte);
    let normalized_current = deindent(current_slice, &indent);
    let normalized_replacement = render_edited_test(&normalized_current, &current, &desired, body)?;
    let rendered = indent_multiline(&normalized_replacement, &indent);
    let prospective = format!(
        "{}{}{}",
        &original[..start_byte],
        rendered,
        &original[end_byte..]
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
        file: current.location.file.clone(),
        dry_run,
        changed,
        start_byte,
        end_byte: start_byte + rendered.len(),
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
        let targets = std::iter::once(&test.target)
            .chain(&test.additional_targets)
            .map(target_string)
            .collect();
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
            fn_name: test.filter.clone(),
            file: test.location.file.clone(),
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
    if desired.file != current.location.file {
        return Err(Diagnostic::error(
            "E-OP-003",
            "Structured Edit cannot move a Test to another file",
        ));
    }
    // 基本仕様:126-134「文字集合は [A-Z0-9-]、接頭辞は種別ごとに固定
    // (`TEST-` 等)。推奨形式は `TEST-<領域>-<連番>` だが、ツールは形式を
    // 強制せず一意性のみを強制する」。ID の書式（接頭辞）は強制しない。
    // 存在しない VO を参照した場合の解決可能性検査は直後で行う。
    if desired.covers.is_empty() {
        return Err(Diagnostic::error(
            "E-OP-001",
            "covers must contain at least one VO ID",
        ));
    }
    let entity_ids = read_entity_ids(root).map_err(|error| {
        Diagnostic::error("E-CORE-001", format!("could not read entity IDs: {error}"))
    })?;
    for id in &desired.covers {
        // vtest_store::read_entity_ids returns [doc, vo] (canonical 2-slot
        // layout); index 1 is the VO id list.
        if !entity_ids[1].iter().any(|candidate| candidate == id) {
            return Err(
                Diagnostic::error("E-OP-001", format!("VO `{id}` does not exist"))
                    .with_candidates(id_candidates(&entity_ids[1], id)),
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
        let Some(locator) = RustLocator::parse(target).map(|parsed| parsed.to_locator()) else {
            return Err(Diagnostic::error(
                "E-OP-001",
                format!("invalid source locator `{target}`"),
            )
            .with_candidates(symbol_candidates(scan, target)));
        };
        if !scan.sources.iter().any(|source| source.locator == locator) {
            return Err(Diagnostic::error(
                "E-OP-001",
                format!("source symbol `{target}` does not exist"),
            )
            .with_candidates(symbol_candidates(scan, target)));
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
        source.location.file == desired.file
            && RustLocator::parse(&source.locator.value)
                .is_some_and(|parsed| parsed.item_path == desired.fn_name)
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
            .with_candidates(test_id_candidates(scan, related)));
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
    if desired.fn_name != current.filter {
        let from = format!("fn {}", current.filter);
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
        && std::iter::once(&test.target)
            .chain(&test.additional_targets)
            .map(target_string)
            .collect::<Vec<_>>()
            == desired.targets
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
        && test.filter == desired.fn_name
        && test.location.file == desired.file
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
        TargetRef::Locator(locator) => locator.value.clone(),
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
                .with_candidates(test_id_candidates(scan, id))
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
                    record.target_execution.result
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
    unregistered
        .sort_by(|left, right| (&left.file, left.start_byte).cmp(&(&right.file, right.start_byte)));
    TestSelection {
        tests,
        unregistered,
    }
}

pub fn query_tests(scan: &ScanResult, source: &str) -> Result<Vec<TestEntity>, Diagnostic> {
    let Some(locator) = RustLocator::parse(source).map(|parsed| parsed.to_locator()) else {
        return Err(
            Diagnostic::error("E-OP-001", format!("invalid source locator `{source}`"))
                .with_candidates(symbol_candidates(scan, source)),
        );
    };
    if !scan.sources.iter().any(|item| item.locator == locator) {
        return Err(Diagnostic::error(
            "E-OP-001",
            format!("source symbol `{source}` does not exist"),
        )
        .with_candidates(symbol_candidates(scan, source)));
    }
    let mut tests = scan
        .tests
        .iter()
        .filter(|test| {
            std::iter::once(&test.target)
                .chain(&test.additional_targets)
                .any(|target| matches!(target, TargetRef::Locator(target) if target == &locator))
        })
        .cloned()
        .collect::<Vec<_>>();
    tests.sort_by(|left, right| left.id.as_str().cmp(right.id.as_str()));
    Ok(tests)
}

pub fn validate_form_answers(
    root: &Path,
    schema: &FormSchema,
    supplied: &FormAnswers,
    scan: &ScanResult,
) -> Result<BTreeMap<String, FormValue>, Diagnostic> {
    validate_form_answers_for(root, schema, supplied, scan, None)
}

fn validate_form_answers_for(
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
                    validate_symbols(scan, &field.name, value, false)?;
                }
                "symbols-exist" => {
                    validate_symbols(scan, &field.name, value, true)?;
                }
                "vo-exists" => {
                    for id in value.values() {
                        // vtest_store::read_entity_ids returns [doc, vo]
                        // (canonical 2-slot layout); index 1 is the VO id
                        // list.
                        if !entity_ids[1].iter().any(|candidate| candidate == id) {
                            return Err(Diagnostic::error(
                                "E-OP-001",
                                format!("VO `{id}` does not exist"),
                            )
                            .with_candidates(id_candidates(&entity_ids[1], id)));
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
                            .with_candidates(test_id_candidates(scan, id)));
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
                        location.file.clone()
                    } else {
                        destination_file(supplied)?
                    };
                    if scan.sources.iter().any(|source| {
                        source.location.file == destination
                            && RustLocator::parse(&source.locator.value)
                                .is_some_and(|parsed| parsed.item_path == name)
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
                    validate_rust_file(root, relative)?;
                }
                "enum-variant-exists" => {
                    validate_enum_variant(root, scalar(value, &field.name)?)?;
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

fn render_form_template(
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

fn unresolved_placeholder(line: &str) -> Option<String> {
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
    let target_values = std::iter::once(&test.target)
        .chain(&test.additional_targets)
        .map(target_string)
        .collect::<Vec<_>>();
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

fn validate_value_shape(
    field: &vtest_store::FormField,
    value: &FormValue,
) -> Result<(), Diagnostic> {
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
            if RustLocator::parse(scalar(value, &field.name)?).is_none() {
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
                .any(|value| RustLocator::parse(value).is_none())
            {
                return Err(Diagnostic::error(
                    "E-OP-001",
                    format!("answer `{}` contains an invalid source locator", field.name),
                ));
            }
        }
        // 基本仕様:126-134「ツールは形式を強制せず一意性のみを強制する」。
        // `vo-ref` / `vo-ref-list` / `test-ref` は ID の接頭辞書式を強制し
        // ない。値の非空・list/scalar形状は上の共通チェックと `scalar()`
        // が担う。参照先の解決可能性は `vo-exists` / `test-exists`
        // validator（呼び出し元の `field.validate` ループ）が別途検査する。
        "vo-ref" | "test-ref" => {
            let _ = scalar(value, &field.name)?;
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

fn validate_symbols(
    scan: &ScanResult,
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
        let Some(locator) = RustLocator::parse(symbol).map(|parsed| parsed.to_locator()) else {
            return Err(Diagnostic::error(
                "E-OP-001",
                format!("invalid source locator `{symbol}`"),
            )
            .with_candidates(symbol_candidates(scan, symbol)));
        };
        if !scan.sources.iter().any(|source| source.locator == locator) {
            return Err(Diagnostic::error(
                "E-OP-001",
                format!("source symbol `{symbol}` does not exist"),
            )
            .with_candidates(symbol_candidates(scan, symbol)));
        }
    }
    Ok(())
}

fn scalar<'a>(value: &'a FormValue, field: &str) -> Result<&'a str, Diagnostic> {
    match value {
        FormValue::Scalar(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(Diagnostic::error(
            "E-OP-001",
            format!("answer `{field}` must be a non-empty scalar"),
        )),
    }
}

fn destination_file(answers: &FormAnswers) -> Result<String, Diagnostic> {
    if let Some(value) = answers.answers.get("file") {
        return scalar(value, "file").map(|value| value.replace('\\', "/"));
    }
    if let Some(value) = answers.answers.get("target") {
        return RustLocator::parse(scalar(value, "target")?)
            .map(|locator| locator.path)
            .ok_or_else(|| Diagnostic::error("E-OP-001", "target is not a locator"));
    }
    Err(Diagnostic::error(
        "E-OP-001",
        "answers require `file` when no single target is present",
    ))
}

fn validate_rust_file(root: &Path, relative: &str) -> Result<(), Diagnostic> {
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
    let config =
        load_config(root).map_err(|error| Diagnostic::error("E-CORE-001", error.to_string()))?;
    // レビュー round 2 項目【F】: `adapters: []` の拒否メッセージ自体には
    // 仕様上の対応コードが無い（本冊:158「無効なadapter設定」は非空
    // adapters list内の各entryの妥当性を指す並列項目であり、「listが
    // 空である」ことの読みには当てはまらない — `adapter_scan_includes`
    // のdoc commentを参照）。しかし `Diagnostic` はコードを必須とするため
    // 「コードなし」は表現できない。ここは `validate_rust_file` という
    // Structured Operation 候補検証の一部であり、この関数の他の全ての
    // 拒否と同じくE-OP-001（本冊:1641「Structured Operationの入力検証
    // 失敗」、別紙A:541「`rust-file` \| ... \| E-OP-001＋候補」）を使う。
    let includes =
        adapter_scan_includes(&config).map_err(|error| Diagnostic::error("E-OP-001", error))?;
    if !includes
        .iter()
        .any(|include| relative_path.starts_with(include))
    {
        return Err(Diagnostic::error(
            "E-OP-001",
            format!("Rust file is outside every registered adapter's scan.include: `{relative}`"),
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

fn validate_enum_variant(root: &Path, value: &str) -> Result<(), Diagnostic> {
    let Some((type_path, variant)) = value.rsplit_once("::") else {
        return Ok(());
    };
    let type_name = type_path.rsplit("::").next().unwrap_or(type_path);
    if syn::parse_str::<syn::Ident>(type_name).is_err()
        || syn::parse_str::<syn::Ident>(variant).is_err()
    {
        return Ok(());
    }
    let config =
        load_config(root).map_err(|error| Diagnostic::error("E-CORE-001", error.to_string()))?;
    // レビュー round 2 項目【F】: `validate_rust_file` と同じ理由（上記の
    // doc comment を参照）。ここは `enum-variant-exists` 候補検証の一部
    // であり、この関数の他の全ての拒否と同じくE-OP-001（本冊:1641、
    // 別紙A:539「`enum-variant-exists` \| ... \| E-OP-001＋候補」）を使う。
    let includes =
        adapter_scan_includes(&config).map_err(|error| Diagnostic::error("E-OP-001", error))?;
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

// PR #26 review round 3 要確認C-1: この `read_dir`/`entries.flatten()` は
// `d423f8a` が `vtest-scan::validate_relations`/`validate_approval_status`
// で塞いだのと同じ形の I/O fail-open（ディレクトリが開けない、または列挙中
// に個々の `DirEntry` が `Err` を返すと、そのディレクトリ／エントリを
// 無診断でスキップする）だが、意図的に塞いでいない: 呼び出し元
// `validate_enum_variant` は `type_name` の Rust 識別子としての妥当性
// チェック（1445-1448行）や `syn::parse_file` の失敗（1469-1471行）も同じ
// 「見つけられなければ検証をスキップし `Ok(())` を返す」扱いを既にしており
// （1474行 `variants.is_empty()`）、これは `enum-variant-exists` Structured
// Operation 入力検証のベストエフォート設計そのもの — I/O 失敗だけを
// fail-closed にしても、識別子形式や parse 失敗という他の「見つからない」
// 経路はそのまま素通りするので非対称は解消しない。影響範囲も、この関数の
// 裁定（chain_integrity・検証状態）ではなく、`vtest scan enum-variant-exists`
// 単体の Structured Operation 入力検証（本冊:1641、別紙A:539）に留まる:
// I/O 失敗時に本来出すべき E-OP-001（不正な値）が見逃され得るが、それ以上
// 状態が壊れることはない。塞ぐなら `validate_enum_variant` 全体の
// 「見つからなければ受理する」設計を Owner 裁定で見直す必要があり、この
// 箇所だけを個別に fail-closed にするのは非対称を悪化させるため見送った。
fn collect_rust_files(directory: &Path, files: &mut Vec<std::path::PathBuf>) {
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

fn symbol_candidates(scan: &ScanResult, requested: &str) -> Vec<String> {
    let item = requested
        .rsplit_once("::")
        .map_or(requested, |(_, item)| item);
    // `source.locator.value` は `rust-cargo` の場合 `<path>::<item_path>`
    // で、`path` 自体は `::` を含まない（`RustLocator::parse`）ため、opaque
    // な `value` 全体を末尾の `::` で分割しても `item_path` の末尾要素と
    // 一致する。この関数は候補表示用のあいまい一致であり、value の内部
    // 構文を core の判定条件として使ってはいない（PR3 canonical化の範囲外、
    // `pr3-decisions.md`「保留中の論点」）。
    let mut exact_suffix = scan
        .sources
        .iter()
        .filter(|source| {
            source
                .locator
                .value
                .rsplit("::")
                .next()
                .is_some_and(|candidate| candidate == item)
        })
        .map(|source| source.locator.value.clone())
        .collect::<Vec<_>>();
    let mut near = scan
        .sources
        .iter()
        .filter(|source| {
            source
                .locator
                .value
                .rsplit("::")
                .next()
                .is_some_and(|candidate| edit_distance(candidate, item) <= 2)
        })
        .map(|source| source.locator.value.clone())
        .collect::<Vec<_>>();
    exact_suffix.sort();
    exact_suffix.dedup();
    near.sort();
    near.dedup();
    near.retain(|candidate| !exact_suffix.contains(candidate));
    exact_suffix.append(&mut near);
    exact_suffix
}

fn test_id_candidates(scan: &ScanResult, requested: &str) -> Vec<String> {
    let ids = scan
        .tests
        .iter()
        .map(|test| test.id.as_str().to_owned())
        .collect::<Vec<_>>();
    id_candidates(&ids, requested)
}

fn id_candidates(ids: &[String], requested: &str) -> Vec<String> {
    let mut candidates = ids
        .iter()
        .filter(|candidate| edit_distance(candidate, requested) <= 2)
        .cloned()
        .collect::<Vec<_>>();
    candidates.sort();
    candidates
}

fn edit_distance(left: &str, right: &str) -> usize {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_distance_handles_transcription_errors() {
        assert_eq!(edit_distance("parse", "prase"), 2);
        assert_eq!(edit_distance("add", "add"), 0);
        assert_eq!(edit_distance("subtract", "add"), 7);
    }

    fn field(name: &str, field_type: &str) -> vtest_store::FormField {
        vtest_store::FormField {
            name: name.to_owned(),
            question: String::new(),
            field_type: field_type.to_owned(),
            required: false,
            options: Vec::new(),
            validate: Vec::new(),
        }
    }

    // 基本仕様:126-134「文字集合は [A-Z0-9-]、接頭辞は種別ごとに固定
    // (`TEST-` 等)。推奨形式は `TEST-<領域>-<連番>` だが、ツールは形式を
    // 強制せず一意性のみを強制する」。`vo-ref` / `vo-ref-list` / `test-ref`
    // は接頭辞書式を拒否理由にしない(PM 裁定・pr3-decisions.md 裁定7)。

    #[test]
    fn vo_ref_field_does_not_enforce_an_id_prefix() {
        let field = field("covers", "vo-ref");
        let value = FormValue::Scalar("WIDGET-ADD".to_owned());
        assert!(validate_value_shape(&field, &value).is_ok());
    }

    #[test]
    fn vo_ref_list_field_does_not_enforce_an_id_prefix() {
        let field = field("covers", "vo-ref-list");
        let value = FormValue::List(vec!["WIDGET-ADD".to_owned(), "GADGET-ADD".to_owned()]);
        assert!(validate_value_shape(&field, &value).is_ok());
    }

    #[test]
    fn test_ref_field_does_not_enforce_an_id_prefix() {
        let field = field("related", "test-ref");
        let value = FormValue::Scalar("WIDGET-CHECK".to_owned());
        assert!(validate_value_shape(&field, &value).is_ok());
    }

    #[test]
    fn vo_ref_field_still_rejects_an_empty_scalar() {
        let field = field("covers", "vo-ref");
        let value = FormValue::Scalar(String::new());
        assert!(validate_value_shape(&field, &value).is_err());
    }

    // レビュー round 2 項目【J】は "vo-ref-list" のアーム削除
    // （PM 裁定7、`fc6e5de`）でこの型の list/scalar 形状検査が失われたと
    // 指摘した。実際には形状検査は `validate_value_shape` 冒頭の共通
    // `list_type` 判定（この関数の先頭、`"symbol-list" | "vo-ref-list"`）が
    // マッチアームより前に行っており、削除されたのは接頭辞検査（基本:130
    // により復活させてはならない）だけだった。この test は共通判定が
    // "vo-ref-list" を対象に含んでいることを固定する回帰テスト。
    #[test]
    fn vo_ref_list_field_rejects_a_scalar() {
        let field = field("covers", "vo-ref-list");
        let value = FormValue::Scalar("WIDGET-ADD".to_owned());
        assert!(validate_value_shape(&field, &value).is_err());
    }
}
