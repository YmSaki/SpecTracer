//! M2 command-line acceptance coverage.
//!
//! Each case starts from the tracked M1 clean fixture but copies it into an
//! independent temporary directory.  The assertions therefore exercise the
//! public binary and canonical records without mutating a tracked fixture.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::Value;

static NEXT_PROJECT: AtomicU64 = AtomicU64::new(0);

struct TempProject {
    root: PathBuf,
}

impl TempProject {
    fn from_m1_base(name: &str) -> Self {
        let sequence = NEXT_PROJECT.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "vtest-cli-m2-{name}-{}-{nanos}-{sequence}",
            std::process::id()
        ));
        copy_tree(&fixture_path("m1/base"), &root);
        // Git cannot preserve the empty canonical approval directory.  A
        // project created by `vtest init` has it, so restore that empty part
        // of the tracked fixture before exercising approval creation.
        fs::create_dir_all(root.join(".verify/approvals"))
            .expect("restore canonical approval directory");
        Self { root }
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        if self.root.exists() {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

fn fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/calc")
        .join(relative)
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create fixture destination");
    for entry in fs::read_dir(source).expect("read tracked fixture directory") {
        let entry = entry.expect("read tracked fixture entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type().expect("read fixture file type").is_dir() {
            copy_tree(&source_path, &destination_path);
        } else {
            fs::copy(source_path, destination_path).expect("copy tracked fixture file");
        }
    }
}

fn invoke(project: &Path, command: &str, args: &[&str]) -> Output {
    let mut invocation = Command::new(env!("CARGO_BIN_EXE_vtest"));
    invocation
        .arg("--project")
        .arg(project)
        .args(["--format", "json", command])
        .args(args);
    invocation.output().expect("run vtest process")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("vtest emits UTF-8 JSON")
}

fn assert_exit(output: &Output, expected: i32, context: &str) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{context}: stdout={} stderr={}",
        stdout(output),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn envelope(output: &Output) -> Value {
    let json = stdout(output);
    let value: Value = serde_json::from_str(&json).expect("valid JSON envelope");
    let object = value.as_object().expect("JSON envelope is an object");
    assert!(object.get("ok").is_some_and(Value::is_boolean));
    assert!(object.contains_key("data"), "missing data: {json}");
    assert!(
        object
            .get("diagnostics")
            .is_some_and(|diagnostics| diagnostics.is_array()),
        "missing diagnostics array: {json}"
    );
    value
}

fn assert_ok(output: &Output, context: &str) -> Value {
    assert_exit(output, 0, context);
    let value = envelope(output);
    assert_eq!(value["ok"], Value::Bool(true), "{context}: {value}");
    value
}

fn assert_usage_error(output: &Output, context: &str) -> Value {
    assert_exit(output, 2, context);
    let value = envelope(output);
    assert_eq!(value["ok"], Value::Bool(false), "{context}: {value}");
    assert_eq!(value["data"], Value::Null, "{context}: {value}");
    assert!(
        value["diagnostics"]
            .as_array()
            .expect("diagnostics is an array")
            .iter()
            .any(|diagnostic| diagnostic["code"] == "E-OP-001"),
        "{context}: {value}"
    );
    value
}

fn vo_files(project: &TempProject) -> Vec<String> {
    let mut names = fs::read_dir(project.root.join(".verify/vo"))
        .expect("read canonical VO directory")
        .map(|entry| entry.expect("read VO entry"))
        .filter_map(|entry| {
            let path = entry.path();
            (path.extension().and_then(|value| value.to_str()) == Some("yaml"))
                .then(|| entry.file_name().to_string_lossy().into_owned())
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn canonical_vo_status(project: &TempProject, id: &str) -> String {
    // W2 stopped storing a `status` field on the VO record; the effective value
    // is derived from Approvals and surfaced by `vo show`.
    let shown = assert_ok(
        &invoke(&project.root, "vo", &["show", id]),
        "show VO for effective status",
    );
    shown["data"]["effective_status"]
        .as_str()
        .expect("vo show reports effective_status")
        .to_owned()
}

fn only_approval_file(project: &TempProject) -> PathBuf {
    let approvals = fs::read_dir(project.root.join(".verify/approvals"))
        .expect("read canonical approval directory")
        .map(|entry| entry.expect("read approval entry").path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("yaml"))
        .collect::<Vec<_>>();
    assert_eq!(approvals.len(), 1, "expected exactly one approval record");
    approvals.into_iter().next().expect("one approval record")
}

fn remove_approver_block(yaml: &str) -> String {
    let start = yaml
        .find("approver:\n")
        .expect("approval has approver block");
    let end = yaml[start..]
        .find("basis:")
        .map(|offset| start + offset)
        .expect("approval has basis after approver block");
    format!("{}{}", &yaml[..start], &yaml[end..])
}

fn assert_tree_child(response: &Value, root_id: &str, child_id: &str) {
    assert_eq!(response["data"]["tree"], true);
    let roots = response["data"]["items"]
        .as_array()
        .expect("tree items is an array")
        .iter()
        .filter(|item| item["id"] == root_id)
        .collect::<Vec<_>>();
    assert_eq!(
        roots.len(),
        1,
        "tree must return root {root_id} exactly once"
    );
    let children = roots[0]["children"]
        .as_array()
        .expect("tree root has children array");
    assert_eq!(
        children
            .iter()
            .filter(|item| item["id"] == child_id)
            .count(),
        1,
        "tree root {root_id} must return child {child_id} exactly once"
    );
    assert_eq!(children.len(), 1, "tree root has only its expected child");
}

/// @vtest.id TEST-CLI-053
/// @vtest.covers VO-CLI-011
/// @vtest.target crates/vtest-cli/src/lib.rs::run_vo
/// @vtest.intent Editing an approved VO invalidates approval and reverts effective status to draft
#[test]
fn m2_vo_edit_invalidates_approval_and_returns_to_effective_draft() {
    let project = TempProject::from_m1_base("approval-invalidation");

    let added = assert_ok(
        &invoke(
            &project.root,
            "vo",
            &["add", "--id", "VO-M2-APPROVAL", "--claim", "original claim"],
        ),
        "add a draft VO",
    );
    assert_eq!(added["data"]["id"], "VO-M2-APPROVAL");
    assert_eq!(
        canonical_vo_status(&project, "VO-M2-APPROVAL"),
        "draft",
        "a newly added VO is effectively draft"
    );

    let approved = assert_ok(
        &invoke(
            &project.root,
            "vo",
            &[
                "approve",
                "VO-M2-APPROVAL",
                "--approver-kind",
                "human",
                "--approver-id",
                "m2-acceptance",
            ],
        ),
        "approve the original VO content",
    );
    assert_eq!(approved["data"]["subject"], "VO-M2-APPROVAL");
    assert!(approved["data"]["subject_hash"].as_str().is_some());
    assert_eq!(
        canonical_vo_status(&project, "VO-M2-APPROVAL"),
        "approved",
        "approval updates the canonical VO status"
    );

    let approved_scan = assert_ok(
        &invoke(&project.root, "scan", &[]),
        "scan immediately after approval",
    );
    assert!(
        !approved_scan["diagnostics"]
            .as_array()
            .expect("diagnostics is an array")
            .iter()
            .any(|diagnostic| diagnostic["code"] == "W-STORE-001"),
        "valid approval must agree with canonical VO status: {approved_scan}"
    );

    let before_edit = assert_ok(
        &invoke(&project.root, "vo", &["show", "VO-M2-APPROVAL"]),
        "show an effectively approved VO",
    );
    assert_eq!(before_edit["data"]["effective_status"], "approved");

    let edited = assert_ok(
        &invoke(
            &project.root,
            "vo",
            &[
                "edit",
                "VO-M2-APPROVAL",
                "--claim",
                "changed claim invalidates approval",
            ],
        ),
        "edit the approved VO",
    );
    assert_eq!(edited["data"]["approval_invalidated"], true);
    assert_eq!(
        canonical_vo_status(&project, "VO-M2-APPROVAL"),
        "draft",
        "editing an approved VO restores canonical draft status"
    );

    let after_edit = assert_ok(
        &invoke(&project.root, "vo", &["show", "VO-M2-APPROVAL"]),
        "show the edited VO",
    );
    assert_eq!(
        after_edit["data"]["claim"],
        "changed claim invalidates approval"
    );
    assert_eq!(after_edit["data"]["effective_status"], "draft");

    let drafts = assert_ok(
        &invoke(&project.root, "vo", &["list", "--status", "draft"]),
        "list effective draft VOs after edit",
    );
    assert!(drafts["data"]["items"]
        .as_array()
        .expect("VO list items")
        .iter()
        .any(|item| item["id"] == "VO-M2-APPROVAL" && item["status"] == "draft"));
}

/// @vtest.id TEST-CLI-054
/// @vtest.covers VO-CLI-011
/// @vtest.target crates/vtest-cli/src/lib.rs::run_vo
/// @vtest.intent Full-product expand --dry-run lists cartesian children without writing records
#[test]
fn m2_full_product_expand_dry_run_lists_cartesian_children_without_writes() {
    let project = TempProject::from_m1_base("full-product-dry-run");
    assert_ok(
        &invoke(
            &project.root,
            "vo",
            &[
                "add",
                "--id",
                "VO-M2-PRODUCT",
                "--claim",
                "calculation is valid for each selected combination",
                "--dimension",
                "operand-sign=positive,negative",
                "--dimension",
                "operator=add,sub",
                "--policy",
                "full-product",
            ],
        ),
        "add a full-product parent VO",
    );
    let before = vo_files(&project);

    let expanded = assert_ok(
        &invoke(
            &project.root,
            "vo",
            &["expand", "VO-M2-PRODUCT", "--dry-run"],
        ),
        "dry-run full-product expansion",
    );
    assert_eq!(expanded["data"]["parent"], "VO-M2-PRODUCT");
    assert_eq!(expanded["data"]["dry_run"], true);
    let children = expanded["data"]["children"]
        .as_array()
        .expect("children is an array");
    let expected = [
        ("VO-M2-PRODUCT-POSITIVE-ADD", ["positive", "add"]),
        ("VO-M2-PRODUCT-POSITIVE-SUB", ["positive", "sub"]),
        ("VO-M2-PRODUCT-NEGATIVE-ADD", ["negative", "add"]),
        ("VO-M2-PRODUCT-NEGATIVE-SUB", ["negative", "sub"]),
    ];
    assert_eq!(children.len(), expected.len());
    for (child, (id, combination)) in children.iter().zip(expected) {
        assert_eq!(child["id"], id);
        assert_eq!(child["combination"], serde_json::json!(combination));
        assert_eq!(child["created"], false);
    }
    assert_eq!(
        vo_files(&project),
        before,
        "dry-run must not create child VO records"
    );
}

/// @vtest.id TEST-CLI-055
/// @vtest.covers VO-CLI-011
/// @vtest.target crates/vtest-cli/src/lib.rs::run_vo
/// @vtest.intent Dry-run expand rejects colliding partition slugs without creating children
#[test]
fn m2_dry_run_rejects_slug_collisions_without_returning_duplicate_children() {
    let project = TempProject::from_m1_base("slug-collision");
    assert_ok(
        &invoke(
            &project.root,
            "vo",
            &[
                "add",
                "--id",
                "VO-M2-COLLISION",
                "--claim",
                "colliding partitions must be rejected",
                "--dimension",
                "sign=a b,a-b",
                "--policy",
                "full-product",
            ],
        ),
        "add a parent with colliding partition slugs",
    );
    let before = vo_files(&project);

    assert_usage_error(
        &invoke(
            &project.root,
            "vo",
            &["expand", "VO-M2-COLLISION", "--dry-run"],
        ),
        "dry-run must reject duplicate child IDs instead of returning them",
    );
    assert_eq!(
        vo_files(&project),
        before,
        "a rejected dry-run must not create child VO records"
    );
}

/// @vtest.id TEST-CLI-056
/// @vtest.covers VO-CLI-008
/// @vtest.target crates/vtest-cli/src/lib.rs::run_scan
/// @vtest.intent Malformed approval is non-effective and scan reports E-SCAN-010 error
#[test]
fn m2_malformed_approval_is_not_effective_and_is_a_scan_error() {
    let project = TempProject::from_m1_base("malformed-approval");
    assert_ok(
        &invoke(
            &project.root,
            "vo",
            &[
                "add",
                "--id",
                "VO-M2-TAMPERED-APPROVAL",
                "--claim",
                "approval record must be structurally complete",
            ],
        ),
        "add a VO for approval-record validation",
    );
    let approval = assert_ok(
        &invoke(
            &project.root,
            "vo",
            &[
                "approve",
                "VO-M2-TAMPERED-APPROVAL",
                "--approver-kind",
                "human",
                "--approver-id",
                "m2-acceptance",
            ],
        ),
        "create the only approval record",
    );
    let approval_id = approval["data"]["id"]
        .as_str()
        .expect("approval has a record ID");
    let approval_path = only_approval_file(&project);
    assert_eq!(
        approval_path.file_stem().and_then(|value| value.to_str()),
        Some(approval_id),
        "approval response ID identifies its canonical file"
    );
    let tampered = remove_approver_block(
        &fs::read_to_string(&approval_path).expect("read generated approval record"),
    );
    fs::write(&approval_path, tampered).expect("remove the approver block from approval record");

    let shown = assert_ok(
        &invoke(&project.root, "vo", &["show", "VO-M2-TAMPERED-APPROVAL"]),
        "show VO after approval record tampering",
    );
    assert_eq!(shown["data"]["effective_status"], "draft");

    let scan = invoke(&project.root, "scan", &[]);
    assert_exit(&scan, 1, "malformed approval must fail scan");
    let scan = envelope(&scan);
    assert_eq!(scan["ok"], Value::Bool(false));
    let diagnostic = scan["diagnostics"]
        .as_array()
        .expect("diagnostics is an array")
        .iter()
        .find(|diagnostic| diagnostic["code"] == "E-SCAN-010")
        .unwrap_or_else(|| panic!("missing malformed approval diagnostic: {scan}"));
    assert_eq!(diagnostic["severity"], "error");
    assert_eq!(
        diagnostic["location"]["path"].as_str(),
        Some(format!(".verify/approvals/{approval_id}.yaml").as_str())
    );
}

/// @vtest.id TEST-CLI-057
/// @vtest.covers VO-CLI-011
/// @vtest.target crates/vtest-cli/src/lib.rs::run_vo
/// @vtest.intent Lowercase VO id is a usage error and writes no canonical record
#[test]
fn m2_lowercase_vo_id_is_a_usage_error_without_a_record_write() {
    let project = TempProject::from_m1_base("lowercase-id");
    let before = vo_files(&project);

    assert_usage_error(
        &invoke(
            &project.root,
            "vo",
            &[
                "add",
                "--id",
                "VO-lowercase",
                "--claim",
                "invalid identifiers must not be stored",
            ],
        ),
        "lowercase VO ID must be rejected before canonical write",
    );
    assert_eq!(vo_files(&project), before);
}

/// @vtest.id TEST-CLI-058
/// @vtest.covers VO-CLI-011
/// @vtest.target crates/vtest-cli/src/lib.rs::run_vo
/// @vtest.intent Path-like VO id is rejected without writing outside .verify/vo
#[test]
fn m2_path_like_vo_id_is_a_usage_error_without_an_outside_write() {
    let project = TempProject::from_m1_base("path-like-id");
    let escaped = project.root.join("M2-ESCAPED.yaml");
    assert!(
        !escaped.exists(),
        "temporary project starts without escaped file"
    );

    assert_usage_error(
        &invoke(
            &project.root,
            "vo",
            &[
                "add",
                "--id",
                "VO-/../../../M2-ESCAPED",
                "--claim",
                "path-like identifiers must not escape the VO directory",
            ],
        ),
        "path-like VO ID must be rejected before an outside write",
    );
    assert!(
        !escaped.exists(),
        "rejected path-like ID must not write outside .verify/vo"
    );
}

/// @vtest.id TEST-CLI-059
/// @vtest.covers VO-CLI-014
/// @vtest.target crates/vtest-cli/src/lib.rs::run_spec
/// @vtest.intent Invalid spec --kind is a usage error and creates no canonical record
#[test]
fn m2_invalid_spec_kind_is_a_usage_error_without_a_canonical_record() {
    let project = TempProject::from_m1_base("invalid-spec-kind");
    let document = project.root.join("docs/invalid-kind.md");
    fs::create_dir_all(document.parent().expect("document has a parent"))
        .expect("create fixture document directory");
    fs::write(&document, "# Invalid kind test\n").expect("write source document");
    let record = project.root.join(".verify/spec/SPEC-M2-INVALID-KIND.yaml");
    assert!(!record.exists(), "canonical SPEC record starts absent");

    assert_usage_error(
        &invoke(
            &project.root,
            "spec",
            &[
                "add",
                "--id",
                "SPEC-M2-INVALID-KIND",
                "--path",
                "docs/invalid-kind.md",
                "--kind",
                "invalid",
            ],
        ),
        "invalid SPEC kind must be rejected before canonical write",
    );
    assert!(
        !record.exists(),
        "invalid SPEC kind must not create a canonical record"
    );
}

/// @vtest.id TEST-CLI-060
/// @vtest.covers VO-CLI-011
/// @vtest.target crates/vtest-cli/src/lib.rs::run_vo
/// @vtest.intent REQ and VO --tree lists return each child exactly once
#[test]
fn m2_req_and_vo_tree_lists_include_each_child_once() {
    let project = TempProject::from_m1_base("record-trees");
    assert_ok(
        &invoke(
            &project.root,
            "req",
            &[
                "add",
                "--id",
                "REQ-M2-TREE-ROOT",
                "--summary",
                "root requirement",
            ],
        ),
        "add root requirement",
    );
    assert_ok(
        &invoke(
            &project.root,
            "req",
            &[
                "add",
                "--id",
                "REQ-M2-TREE-CHILD",
                "--summary",
                "child requirement",
                "--parent",
                "REQ-M2-TREE-ROOT",
            ],
        ),
        "add child requirement",
    );
    assert_ok(
        &invoke(
            &project.root,
            "vo",
            &[
                "add",
                "--id",
                "VO-M2-TREE-ROOT",
                "--claim",
                "root VO claim",
                "--req",
                "REQ-M2-TREE-ROOT",
            ],
        ),
        "add root VO",
    );
    assert_ok(
        &invoke(
            &project.root,
            "vo",
            &[
                "add",
                "--id",
                "VO-M2-TREE-CHILD",
                "--claim",
                "child VO claim",
                "--parent",
                "VO-M2-TREE-ROOT",
                "--req",
                "REQ-M2-TREE-CHILD",
            ],
        ),
        "add child VO",
    );

    let req_tree = assert_ok(
        &invoke(&project.root, "req", &["list", "--tree"]),
        "list REQ hierarchy",
    );
    assert_tree_child(&req_tree, "REQ-M2-TREE-ROOT", "REQ-M2-TREE-CHILD");

    let vo_tree = assert_ok(
        &invoke(&project.root, "vo", &["list", "--tree"]),
        "list VO hierarchy",
    );
    assert_tree_child(&vo_tree, "VO-M2-TREE-ROOT", "VO-M2-TREE-CHILD");
}

/// @vtest.id TEST-CLI-061
/// @vtest.covers VO-CLI-011
/// @vtest.target crates/vtest-cli/src/lib.rs::run_vo
/// @vtest.intent Explicit expansion uses only declared combinations; invalid combos rejected
#[test]
fn m2_explicit_vo_expansion_uses_only_declared_combinations() {
    let project = TempProject::from_m1_base("explicit-expansion");
    assert_ok(
        &invoke(
            &project.root,
            "vo",
            &[
                "add",
                "--id",
                "VO-M2-EXPLICIT",
                "--claim",
                "only explicitly selected combinations are required",
                "--dimension",
                "sign=positive,negative",
                "--dimension",
                "operator=add,sub",
                "--policy",
                "explicit",
                "--combination",
                "positive,add",
                "--combination",
                "negative,sub",
            ],
        ),
        "add explicit-combination parent VO",
    );
    let before = vo_files(&project);
    let expanded = assert_ok(
        &invoke(
            &project.root,
            "vo",
            &["expand", "VO-M2-EXPLICIT", "--dry-run"],
        ),
        "dry-run explicit expansion",
    );
    let children = expanded["data"]["children"]
        .as_array()
        .expect("children is an array");
    let expected = [
        ("VO-M2-EXPLICIT-POSITIVE-ADD", ["positive", "add"]),
        ("VO-M2-EXPLICIT-NEGATIVE-SUB", ["negative", "sub"]),
    ];
    assert_eq!(children.len(), expected.len());
    for (child, (id, combination)) in children.iter().zip(expected) {
        assert_eq!(child["id"], id);
        assert_eq!(child["combination"], serde_json::json!(combination));
        assert_eq!(child["created"], false);
    }
    assert_eq!(
        vo_files(&project),
        before,
        "explicit dry-run must not create child VO records"
    );

    for (id, combinations, context) in [
        (
            "VO-M2-EXPLICIT-UNKNOWN",
            vec!["unknown,add"],
            "undeclared explicit partition",
        ),
        (
            "VO-M2-EXPLICIT-DUPLICATE",
            vec!["positive,add", "positive,add"],
            "duplicate explicit combination",
        ),
    ] {
        let mut args = vec![
            "add",
            "--id",
            id,
            "--claim",
            "invalid explicit combinations must not be stored",
            "--dimension",
            "sign=positive,negative",
            "--dimension",
            "operator=add,sub",
            "--policy",
            "explicit",
        ];
        for combination in combinations {
            args.extend(["--combination", combination]);
        }
        assert_usage_error(
            &invoke(&project.root, "vo", &args),
            &format!("reject {context}"),
        );
        assert!(
            !project.root.join(format!(".verify/vo/{id}.yaml")).exists(),
            "{context} must not create a canonical VO record"
        );
    }
}

/// @vtest.id TEST-CLI-062
/// @vtest.covers VO-CLI-008
/// @vtest.target crates/vtest-cli/src/lib.rs::run_scan
/// @vtest.intent Mutating a registered spec document yields scan warning W-SCAN-104
#[test]
fn m2_mutating_registered_spec_document_reports_w_scan_104() {
    let project = TempProject::from_m1_base("stale-spec");
    let document = project.root.join("docs/m2-spec.md");
    fs::create_dir_all(document.parent().expect("document has a parent"))
        .expect("create fixture document directory");
    fs::write(&document, "# M2 specification\n\nOriginal content.\n")
        .expect("write registered document");

    let registered = assert_ok(
        &invoke(
            &project.root,
            "spec",
            &[
                "add",
                "--id",
                "SPEC-M2-DOCUMENT",
                "--path",
                "docs/m2-spec.md",
                "--kind",
                "document",
                "--title",
                "M2 acceptance document",
            ],
        ),
        "register a specification document",
    );
    assert_eq!(registered["data"]["path"], "docs/m2-spec.md");
    assert!(registered["data"]["sha256"].as_str().is_some());

    fs::write(&document, "# M2 specification\n\nChanged content.\n")
        .expect("mutate registered document");
    let scan = assert_ok(
        &invoke(&project.root, "scan", &[]),
        "a stale SPEC hash is a warning, not a scanner error",
    );
    let warning = scan["diagnostics"]
        .as_array()
        .expect("diagnostics is an array")
        .iter()
        .find(|diagnostic| diagnostic["code"] == "W-SCAN-104")
        .unwrap_or_else(|| panic!("missing W-SCAN-104: {scan}"));
    assert_eq!(warning["severity"], "warning");
    assert_eq!(
        warning["location"]["path"],
        ".verify/spec/SPEC-M2-DOCUMENT.yaml"
    );
}

/// @vtest.id TEST-CLI-063
/// @vtest.covers VO-CLI-011
/// @vtest.target crates/vtest-cli/src/lib.rs::run_vo
/// @vtest.intent vo show reports covering tests, audit state, and valid approval history
#[test]
fn m2_vo_show_reports_covering_tests_audit_state_and_valid_approval_history() {
    let project = TempProject::from_m1_base("vo-show");
    let initial = assert_ok(
        &invoke(&project.root, "vo", &["show", "VO-KNOWN"]),
        "show the M1 base VO",
    );
    assert_eq!(initial["data"]["audit_state"], "NOT_CHECKED");
    assert_eq!(
        initial["data"]["covering_tests"]
            .as_array()
            .expect("covering_tests is an array")
            .iter()
            .filter(|test| test["id"] == "TEST-M1-CLEAN")
            .count(),
        1,
        "VO-KNOWN must report its M1 covering test once"
    );

    let approval = assert_ok(
        &invoke(
            &project.root,
            "vo",
            &[
                "approve",
                "VO-KNOWN",
                "--approver-kind",
                "human",
                "--approver-id",
                "m2-acceptance",
            ],
        ),
        "approve the M1 base VO",
    );
    let approval_id = approval["data"]["id"]
        .as_str()
        .expect("approval response has an ID");

    let approved = assert_ok(
        &invoke(&project.root, "vo", &["show", "VO-KNOWN"]),
        "show the approved M1 base VO",
    );
    assert_eq!(approved["data"]["effective_status"], "approved");
    assert!(
        approved["data"]["approvals"]
            .as_array()
            .expect("approvals is an array")
            .iter()
            .any(|entry| entry["id"] == approval_id && entry["valid"] == true),
        "show must include the valid approval created for this VO: {approved}"
    );
}

/// @vtest.id TEST-CLI-064
/// @vtest.covers VO-CLI-011
/// @vtest.target crates/vtest-cli/src/lib.rs::run_vo
/// @vtest.intent Approval with a missing basis is rejected before mutating the VO
#[test]
fn m2_approval_rejects_a_missing_basis_before_mutating_the_vo() {
    let project = TempProject::from_m1_base("missing-approval-basis");
    assert_ok(
        &invoke(
            &project.root,
            "vo",
            &[
                "add",
                "--id",
                "VO-M2-BASIS",
                "--claim",
                "approval basis references an existing audit fact",
            ],
        ),
        "add a VO for basis validation",
    );

    assert_usage_error(
        &invoke(
            &project.root,
            "vo",
            &[
                "approve",
                "VO-M2-BASIS",
                "--approver-kind",
                "human",
                "--approver-id",
                "m2-acceptance",
                "--basis",
                "01J8XVZK3Q0000000000000000",
            ],
        ),
        "reject a missing audit basis",
    );
    assert_eq!(canonical_vo_status(&project, "VO-M2-BASIS"), "draft");
    assert!(
        fs::read_dir(project.root.join(".verify/approvals"))
            .expect("read approvals directory")
            .all(|entry| entry
                .expect("read approval entry")
                .path()
                .extension()
                .and_then(|value| value.to_str())
                != Some("yaml")),
        "a rejected approval must not append a record"
    );
}
