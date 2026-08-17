use std::{collections::BTreeMap, fs, path::Path};

use serde::Serialize;
use vtest_adapter_api::{
    resolve_structured_test_adapter, AdapterError, AdapterRegistry, StructuredEditFields,
};
use vtest_model::{
    CheckValue, ContentHash, Diagnostic, SourceLocation, TargetRef, TestEntity, TestResult,
};
use vtest_store::{
    load_config, load_form_schema, read_entity_ids, read_evidence, read_record_ids, write_atomic,
    yaml_scalar_value, FormAnswers, FormSchema, FormValue, VerifyLayout,
};

use vtest_adapter_rust::operations_support::*;

use crate::{Locator, ScanResult, RUST_ADAPTER_ID};

/// Maps a registry rejection to the diagnostic codes 詳細設計 §17.1 defines:
/// a missing StructuredTest capability is E-ADAPTER-004 specifically (別紙C
/// §18.3.7's explicit assignment); every other rejection (unknown adapter,
/// duplicate kind ownership, owner mismatch, ambiguous or absent
/// compatibility match) is E-ADAPTER-001 ("adapterが未登録、重複、または
/// registryの宣言と実装が不一致"), matching how `run_scan` already recodes
/// any `AdapterError` surfaced during discovery. Both map to exit 2 -- core
/// never falls back to any adapter on rejection.
fn registry_resolution_diagnostic(error: AdapterError) -> Diagnostic {
    match error {
        AdapterError::MissingCapability(message) => Diagnostic::error("E-ADAPTER-004", message),
        other => Diagnostic::error("E-ADAPTER-001", other.to_string()),
    }
}

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
    registry: &AdapterRegistry,
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
    // Registry resolution (別紙A §14.2-§14.4, 別紙C §18.3.7) happens before any
    // other validation or file mutation: the Form kind must resolve to
    // exactly one StructuredTest-capable adapter, with no fallback.
    resolve_structured_test_adapter(registry, &schema).map_err(registry_resolution_diagnostic)?;
    let answers = validate_form_answers(includes, root, &schema, supplied, &scan)?;
    let test_id = select_test_id(&scan, &answers, explicit_id)?;
    let file = destination_file(supplied)?;
    validate_rust_file(includes, root, &file)?;
    let path = root.join(Path::new(&file));
    let original = fs::read_to_string(&path)
        .map_err(|error| Diagnostic::error("E-CORE-001", error.to_string()))?;
    let rendered = render_form_template(&schema, &answers, &test_id)?;
    check_test_item_fn(&rendered).map_err(|error| {
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
    check_rust_file_parses(&prospective).map_err(|error| {
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
    registry: &AdapterRegistry,
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
    // The Test's own declaring adapter must resolve to a registered
    // StructuredTest-capable adapter BEFORE any file mutation -- this is the
    // adapter whose `render_edit` owns declaration syntax for this Test,
    // regardless of whether the edit is driven by --answers, --set, or
    // --body-file (別紙A §14.3, §15).
    let structured_adapter = registry
        .structured_test(&current.location.adapter)
        .map_err(registry_resolution_diagnostic)?;
    let mut desired = DesiredTest::from_current(&current);
    if let Some(supplied) = supplied {
        let layout = VerifyLayout::new(root);
        let config = load_config(root)
            .map_err(|error| Diagnostic::error("E-CORE-001", error.to_string()))?;
        let includes = &config.rust_cargo().scan.include;
        let schema = load_form_schema(&layout, &supplied.form)
            .map_err(|error| Diagnostic::error("E-OP-001", error.to_string()))?;
        let (form_adapter_id, _) =
            resolve_structured_test_adapter(registry, &schema).map_err(registry_resolution_diagnostic)?;
        if form_adapter_id != current.location.adapter {
            return Err(Diagnostic::error(
                "E-ADAPTER-001",
                format!(
                    "form `{}` is owned by adapter `{form_adapter_id}`, but Test `{test_id}` is owned by `{}`",
                    schema.kind, current.location.adapter
                ),
            ));
        }
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
    let fields = structured_edit_fields(&desired);
    let normalized_replacement = structured_adapter.render_edit(
        &normalized_current,
        current.id.as_str(),
        &current.execution.selector,
        &fields,
        body,
    )?;
    let rendered = indent_multiline(&normalized_replacement, &indent);
    let prospective = format!(
        "{}{}{}",
        &original[..current.location.byte_range.start],
        rendered,
        &original[current.location.byte_range.end..]
    );
    check_rust_file_parses(&prospective).map_err(|error| {
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
    if !is_rust_ident(&desired.fn_name) {
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

/// Converts the core-validated desired state into the adapter-neutral DTO
/// handed to `StructuredTestAdapter::render_edit`. This is the boundary: core
/// interprets form answers / `--set` values (business rules), but never
/// renders adapter-owned declaration syntax itself (基本仕様 §6.1).
fn structured_edit_fields(desired: &DesiredTest) -> StructuredEditFields {
    StructuredEditFields {
        id: desired.id.clone(),
        covers: desired.covers.clone(),
        targets: desired.targets.clone(),
        intent: desired.intent.clone(),
        input: desired.input.clone(),
        expect: desired.expect.clone(),
        kind: desired.kind.clone(),
        cases: desired.cases.clone(),
        related: desired.related.clone(),
        fn_name: desired.fn_name.clone(),
        file: desired.file.clone(),
    }
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
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use vtest_store::init_project;

    /// @vtest.id TEST-SCAN-018
    /// @vtest.covers VO-SCAN-003
    /// @vtest.target crates/vtest-adapter-rust/src/operations_support.rs::edit_distance
    /// @vtest.intent edit_distance returns correct Levenshtein distances for transposition and disjoint strings
    #[test]
    fn edit_distance_handles_transcription_errors() {
        assert_eq!(edit_distance("parse", "prase"), 2);
        assert_eq!(edit_distance("add", "add"), 0);
        assert_eq!(edit_distance("subtract", "add"), 7);
    }

    fn temp_project(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("vtest-structop-{label}-{suffix}"));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"structop-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
        )
        .unwrap();
        init_project(&root, "structop-fixture").unwrap();
        fs::write(
            root.join(".verify/vo/VO-CALC-ADD.yaml"),
            "id: VO-CALC-ADD\nparent: null\nrequirements: []\nspec_refs: []\nclaim: addition works\ndimensions: []\ncoverage_policy: null\nrepresentative_cases: []\nstatus: draft\ncreated: '2026-01-01'\nupdated: '2026-01-01'\n",
        )
        .unwrap();
        root
    }

    fn registry() -> AdapterRegistry {
        AdapterRegistry::from_registrations([vtest_adapter_rust::rust_cargo_registration()])
            .expect("built-in rust-cargo registration is well-formed")
    }

    /// @vtest.id TEST-SCAN-030
    /// @vtest.covers VO-REGISTRY-05
    /// @vtest.target crates/vtest-scan/src/operations.rs::edit_test
    /// @vtest.intent Editing a `/** */` block-doc-declared Test round-trips through edit_test without duplicating the declaration or corrupting the dry-run payload
    #[test]
    fn edit_test_round_trips_a_block_doc_comment_declaration() {
        let root = temp_project("block-doc");
        fs::write(
            root.join("src/lib.rs"),
            "pub fn add(left: i32, right: i32) -> i32 { left + right }\n\n/**\nfree text before\n@vtest.id TEST-ADD-001\n@vtest.covers VO-CALC-ADD\n@vtest.target src/lib.rs::add\n@vtest.intent add returns the sum of its two arguments\n*/\n#[test]\nfn add_returns_sum() {\n    assert_eq!(add(1, 2), 3);\n}\n",
        )
        .unwrap();

        let scan = crate::scan_project(&root).unwrap();
        let discovered = scan
            .tests
            .iter()
            .find(|test| test.id.as_str() == "TEST-ADD-001")
            .expect("block doc comment Test is discovered cleanly, with zero diagnostics");
        assert_eq!(
            discovered.intent,
            "add returns the sum of its two arguments"
        );

        let registry = registry();
        let set = parse_test_set_values(&[
            "intent=add returns the arithmetic sum".to_owned(),
        ])
        .unwrap();

        let dry_run = edit_test(&root, &registry, "TEST-ADD-001", None, &set, None, true)
            .expect("dry-run edit of a block doc comment Test must succeed, not E-OP-003");
        assert_eq!(
            dry_run.rendered.matches("@vtest.id TEST-ADD-001").count(),
            1,
            "dry-run rendering must not emit a second, duplicated declaration"
        );
        assert!(dry_run.rendered.contains("free text before"));

        let applied = edit_test(&root, &registry, "TEST-ADD-001", None, &set, None, false)
            .expect("edit of a block doc comment Test must succeed, not roll back with E-OP-003");
        assert!(applied.changed);
        let after = fs::read_to_string(root.join("src/lib.rs")).unwrap();
        assert!(after.contains("add returns the arithmetic sum"));
        assert_eq!(after.matches("@vtest.id TEST-ADD-001").count(), 1);

        let rescanned = crate::scan_project(&root).unwrap();
        let edited = rescanned
            .tests
            .iter()
            .find(|test| test.id.as_str() == "TEST-ADD-001")
            .expect("edited Test is still recognized by the scanner after the write");
        assert_eq!(edited.intent, "add returns the arithmetic sum");
        fs::remove_dir_all(root).unwrap();
    }

    /// @vtest.id TEST-SCAN-031
    /// @vtest.covers VO-STRUCTOP-09
    /// @vtest.target crates/vtest-adapter-api/src/lib.rs::resolve_structured_test_adapter
    /// @vtest.intent A Form declaring an unregistered adapter is rejected before any file mutation, not silently applied by rust-cargo
    #[test]
    fn create_test_rejects_a_form_owned_by_an_unregistered_adapter() {
        let root = temp_project("unowned");
        fs::write(
            root.join("src/lib.rs"),
            "pub fn add(left: i32, right: i32) -> i32 { left + right }\n",
        )
        .unwrap();
        // The template renders VALID Rust -- deliberately identical in shape
        // to rust-unit-function's -- so the only thing that can prevent
        // create_test from succeeding is the registry gate itself, exactly
        // like the confirmed dossier repro (a syntactically-invalid template
        // would fail downstream regardless, understating the defect).
        fs::write(
            root.join(".verify/forms/py-unit.yaml"),
            "kind: py-unit\nadapter: python-pytest\ntitle: Python-flavored unit test\nfields:\n  - name: target\n    question: Target?\n    type: symbol\n    required: true\n    validate: [symbol-exists]\n  - name: covers\n    question: Verification objectives?\n    type: vo-ref-list\n    required: true\n    validate: [vo-exists]\n  - name: behavior\n    question: Behavior?\n    type: string\n    required: true\n  - name: fn_name\n    question: Test function name?\n    type: ident\n    required: true\ntemplate: |\n  /// @vtest.id {test_id}\n  /// @vtest.covers {covers}\n  /// @vtest.target {target}\n  /// @vtest.intent {behavior}\n  #[test]\n  fn {fn_name}() {\n      todo!(\"implement test body\")\n  }\n",
        )
        .unwrap();
        fs::write(
            root.join("answers.yaml"),
            "form: py-unit\nanswers:\n  target: src/lib.rs::add\n  covers: [VO-CALC-ADD]\n  behavior: adds two integers\n  fn_name: py_unit_placeholder\n",
        )
        .unwrap();

        let before = fs::read_to_string(root.join("src/lib.rs")).unwrap();
        let registry = registry();
        let supplied = vtest_store::read_form_answers(&root.join("answers.yaml")).unwrap();
        let error = create_test(&root, &registry, "py-unit", &supplied, None, false)
            .expect_err("a Form owned by an unregistered adapter must be rejected");
        assert_eq!(error.code, "E-ADAPTER-001");
        assert_eq!(
            fs::read_to_string(root.join("src/lib.rs")).unwrap(),
            before,
            "rejection must leave every file byte-identical"
        );
        let rescanned = crate::scan_project(&root).unwrap();
        assert!(
            rescanned.tests.is_empty(),
            "no Test may be materialized from a Form owned by an unregistered adapter"
        );
        fs::remove_dir_all(root).unwrap();
    }

    /// @vtest.id TEST-SCAN-032
    /// @vtest.covers VO-STRUCTOP-10
    /// @vtest.target crates/vtest-adapter-api/src/lib.rs::resolve_structured_test_adapter
    /// @vtest.intent A Form with no adapter and no rust-cargo-recognized validator is rejected on 0 compatibility matches, never falling back to rust-cargo
    #[test]
    fn create_test_rejects_an_adapterless_form_with_no_compatibility_match() {
        let root = temp_project("no-match");
        fs::write(
            root.join("src/lib.rs"),
            "pub fn add(left: i32, right: i32) -> i32 { left + right }\n",
        )
        .unwrap();
        // Same principle as TEST-SCAN-031: the template renders VALID Rust,
        // targeting the existing `add` symbol directly, and none of the
        // fields carry a rust-cargo validator -- so only the 0-match
        // resolution gate, never a downstream syntax failure, can block it.
        fs::write(
            root.join(".verify/forms/pytest-case.yaml"),
            "kind: pytest-case\ntitle: Pytest case\nfields:\n  - name: covers\n    question: Verification objectives?\n    type: vo-ref-list\n    required: true\n    validate: [vo-exists]\n  - name: behavior\n    question: Behavior?\n    type: string\n    required: true\n  - name: file\n    question: File?\n    type: path\n    required: true\n  - name: fn_name\n    question: Test function name?\n    type: ident\n    required: true\ntemplate: |\n  /// @vtest.id {test_id}\n  /// @vtest.covers {covers}\n  /// @vtest.target src/lib.rs::add\n  /// @vtest.intent {behavior}\n  #[test]\n  fn {fn_name}() {\n      todo!(\"implement test body\")\n  }\n",
        )
        .unwrap();
        fs::write(
            root.join("answers.yaml"),
            "form: pytest-case\nanswers:\n  covers: [VO-CALC-ADD]\n  behavior: adds two integers\n  file: src/lib.rs\n  fn_name: pytest_style_placeholder\n",
        )
        .unwrap();

        let before = fs::read_to_string(root.join("src/lib.rs")).unwrap();
        let registry = registry();
        let supplied = vtest_store::read_form_answers(&root.join("answers.yaml")).unwrap();
        let error = create_test(&root, &registry, "pytest-case", &supplied, None, false)
            .expect_err("a Form with no adapter and no compatibility match must be rejected");
        assert_eq!(error.code, "E-ADAPTER-001");
        assert_eq!(
            fs::read_to_string(root.join("src/lib.rs")).unwrap(),
            before,
            "rejection must leave every file byte-identical; core must never fall back to rust-cargo"
        );
        fs::remove_dir_all(root).unwrap();
    }

    /// @vtest.id TEST-SCAN-033
    /// @vtest.covers VO-STRUCTOP-09
    /// @vtest.target crates/vtest-adapter-api/src/lib.rs::resolve_structured_test_adapter
    /// @vtest.intent Duplicate built-in kind ownership, missing StructuredTest capability, and 2+ compatibility matches all reject fail-closed, distinguishing E-ADAPTER-004 from every other rejection
    #[test]
    fn resolve_structured_test_adapter_rejects_duplicates_and_missing_capability() {
        use std::sync::Arc;
        use vtest_adapter_api::{AdapterCapability, AdapterDescriptor, AdapterRegistration, StructuredTestAdapter};
        use vtest_model::{AdapterId, FormSchema};

        struct SharedKindAdapter;
        impl StructuredTestAdapter for SharedKindAdapter {
            fn built_in_form_kinds(&self) -> Vec<String> {
                vec!["shared-kind".to_owned()]
            }
            fn accepts_compatibility_form(&self, _schema: &serde_json::Value) -> bool {
                false
            }
            fn render_edit(
                &self,
                _current_source: &str,
                _current_id: &str,
                _current_selector: &str,
                _desired: &StructuredEditFields,
                _body: Option<&str>,
            ) -> Result<String, Diagnostic> {
                Err(Diagnostic::error("E-OP-002", "synthetic adapter does not render"))
            }
        }

        fn synthetic(id: &str, capable: bool) -> AdapterRegistration {
            let mut registration = AdapterRegistration::new(AdapterDescriptor {
                id: AdapterId::new(id),
                languages: vec!["fixture".to_owned()],
                capabilities: if capable {
                    vec![AdapterCapability::StructuredTest]
                } else {
                    Vec::new()
                },
                config_namespace: id.to_owned(),
            });
            if capable {
                registration.structured_test = Some(Arc::new(SharedKindAdapter));
            }
            registration
        }

        let registry = AdapterRegistry::from_registrations([
            synthetic("synthetic-a", true),
            synthetic("synthetic-b", true),
            synthetic("synthetic-c", false),
        ])
        .unwrap();

        let duplicate_schema = FormSchema {
            kind: "shared-kind".to_owned(),
            adapter: Some("synthetic-a".to_owned()),
            title: "fixture".to_owned(),
            fields: Vec::new(),
            template: String::new(),
        };
        let error = match resolve_structured_test_adapter(&registry, &duplicate_schema) {
            Ok(_) => panic!("a kind declared built-in by two adapters must be rejected"),
            Err(error) => error,
        };
        assert!(matches!(error, AdapterError::Registration(_)));

        let missing_capability_schema = FormSchema {
            kind: "unowned-kind".to_owned(),
            adapter: Some("synthetic-c".to_owned()),
            title: "fixture".to_owned(),
            fields: Vec::new(),
            template: String::new(),
        };
        let error = match resolve_structured_test_adapter(&registry, &missing_capability_schema) {
            Ok(_) => panic!("an adapter without the StructuredTest capability must be rejected"),
            Err(error) => error,
        };
        assert!(matches!(error, AdapterError::MissingCapability(_)));

        let ambiguous_schema = FormSchema {
            kind: "shared-kind".to_owned(),
            adapter: None,
            title: "fixture".to_owned(),
            fields: Vec::new(),
            template: String::new(),
        };
        let error = match resolve_structured_test_adapter(&registry, &ambiguous_schema) {
            Ok(_) => panic!("2+ compatibility matches must be rejected, never resolved by fallback"),
            Err(error) => error,
        };
        assert!(matches!(error, AdapterError::Registration(_)));
    }
}
