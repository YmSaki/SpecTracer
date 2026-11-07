//! #85: `vtest test create` / `vtest test edit` (SPEC-385 / SPEC-386) drive the
//! adapter's Structured Test capability end to end through the binary.
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use vtest_model::{
    DerivesFrom, DocumentFile, DocumentId, NodeSource, RootNode, SentenceNode, VoId, VoRecord,
};

const ANSWERS: &str = "form: rust-unit-function\n\
answers:\n\
\x20 target: src/lib.rs::add\n\
\x20 covers:\n\
\x20   - VO-OP-ADD\n\
\x20 behavior: add sums two numbers\n\
\x20 test_kind: normal\n\
\x20 input: one and two\n\
\x20 expect: three\n\
\x20 fn_name: add_sums_two_numbers\n\
\x20 file: tests/calc.rs\n";

const CALC: &str = "use ops_fixture::add;\n";

fn fixture(name: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "vtest-test-op-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).unwrap();
    let layout = vtest_store::init_project(&root, "ops-fixture").unwrap();
    let source = NodeSource {
        doc: "fixture.md".to_owned(),
        heading: "fixture".to_owned(),
        lines: [1, 1],
    };
    let document = DocumentFile {
        schema_version: "0.1".to_owned(),
        root: vec![RootNode {
            id: DocumentId::new("ROOT-001"),
            statement: "fixture root".to_owned(),
            description: None,
            source: source.clone(),
        }],
        request: vec![SentenceNode {
            id: DocumentId::new("R-001"),
            statement: "add sums two numbers".to_owned(),
            description: None,
            derives_from: vec![DocumentId::new("ROOT-001")],
            cites: None,
            source,
        }],
        require: Vec::new(),
        spec: Vec::new(),
        detailed_spec: Vec::new(),
        basic_design: Vec::new(),
        design: Vec::new(),
    };
    vtest_store::write_document_file(&layout, "fixture", &document).unwrap();
    vtest_store::write_vo_record(
        &layout,
        &VoRecord {
            id: VoId::new("VO-OP-ADD"),
            parent: None,
            derives_from: vec![DerivesFrom {
                doc: DocumentId::new("R-001"),
                anchor: None,
                note: None,
            }],
            claim: "add sums two numbers".to_owned(),
            dimensions: Vec::new(),
            coverage_policy: None,
            combinations: Vec::new(),
            representative_cases: Vec::new(),
            created: "2026-11-04T00:00:00Z".to_owned(),
            updated: "2026-11-04T00:00:00Z".to_owned(),
        },
    )
    .unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"ops-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "pub fn add(a: u32, b: u32) -> u32 {\n    a + b\n}\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("tests")).unwrap();
    fs::write(root.join("tests/calc.rs"), CALC).unwrap();
    fs::write(root.join("answers.yaml"), ANSWERS).unwrap();
    root
}

fn vtest(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vtest"))
        .current_dir(root)
        .arg("--project")
        .arg(root)
        .args(["--format", "json"])
        .args(args)
        .output()
        .unwrap()
}

fn envelope(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout is not a JSON envelope ({error}): {}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

fn create(root: &Path, dry_run: bool) -> serde_json::Value {
    let mut args = vec![
        "test",
        "create",
        "--form",
        "rust-unit-function",
        "--answers",
        "answers.yaml",
    ];
    if dry_run {
        args.push("--dry-run");
    }
    let output = vtest(root, &args);
    let envelope = envelope(&output);
    assert_eq!(output.status.code(), Some(0), "{envelope}");
    envelope
}

/// @vtest.id TEST-CLI-TEST-CREATE-DRY-RUN-NO-WRITE
/// @vtest.covers VO-TEST-OP-CREATE-DRY-RUN-NO-WRITE, VO-TEST-OP-CREATE-OUTPUT
/// @vtest.target crates/vtest-cli/src/ops/test_op.rs::create
/// @vtest.intent test create --dry-run reports the generated Test ID, insertion position and diff and leaves every file unchanged
#[test]
fn create_dry_run_reports_the_insertion_and_writes_nothing() {
    let root = fixture("dry-run");
    let envelope = create(&root, true);
    assert_eq!(envelope["data"]["dry_run"], true);
    assert_eq!(envelope["data"]["file"], "tests/calc.rs");
    let test_id = envelope["data"]["test_id"].as_str().unwrap();
    assert!(test_id.starts_with("TEST-"), "{envelope}");
    assert!(
        envelope["data"]["position"]["start_byte"].as_u64().unwrap() >= CALC.len() as u64,
        "create appends after the existing source: {envelope}"
    );
    let diff = envelope["data"]["diff"].as_str().unwrap();
    assert!(diff.starts_with("--- a/tests/calc.rs\n+++ b/tests/calc.rs\n"));
    assert!(
        diff.contains(&format!("+/// @vtest.id {test_id}\n")),
        "{diff}"
    );
    assert!(diff.contains("+fn add_sums_two_numbers() {\n"), "{diff}");
    assert_eq!(
        fs::read_to_string(root.join("tests/calc.rs")).unwrap(),
        CALC
    );
    fs::remove_dir_all(root).unwrap();
}

/// @vtest.id TEST-CLI-TEST-CREATE-SCAN-RECOGNIZES-RESULT
/// @vtest.covers VO-TEST-OP-CREATE-SCAN-RECOGNIZED
/// @vtest.target crates/vtest-cli/src/ops/test_op.rs::create
/// @vtest.intent a Test written by test create is discovered by the next scan with the same Test ID, covers and target
#[test]
fn created_test_is_recognized_by_scan() {
    let root = fixture("create");
    let before = envelope(&vtest(&root, &["scan"]))["data"]["tests"]
        .as_u64()
        .unwrap();
    let created = create(&root, false);
    let test_id = created["data"]["test_id"].as_str().unwrap().to_owned();
    assert_eq!(created["data"]["dry_run"], false);
    assert_eq!(created["data"]["changed"], true);

    let after = envelope(&vtest(&root, &["scan"]))["data"]["tests"]
        .as_u64()
        .unwrap();
    assert_eq!(after, before + 1, "scan must discover the created Test");
    let source = fs::read_to_string(root.join("tests/calc.rs")).unwrap();
    assert!(source.starts_with(CALC), "existing source must be kept");
    for line in [
        format!("/// @vtest.id {test_id}"),
        "/// @vtest.covers VO-OP-ADD".to_owned(),
        "/// @vtest.target src/lib.rs::add".to_owned(),
        "/// @vtest.intent add sums two numbers".to_owned(),
    ] {
        assert!(source.contains(&line), "missing `{line}` in:\n{source}");
    }
    fs::remove_dir_all(root).unwrap();
}

/// @vtest.id TEST-CLI-TEST-EDIT-SET-IDEMPOTENT
/// @vtest.covers VO-TEST-OP-EDIT-SET-NAMED-FIELDS, VO-TEST-OP-EDIT-IDEMPOTENT, VO-TEST-OP-EDIT-OUTPUT
/// @vtest.target crates/vtest-cli/src/ops/test_op.rs::edit
/// @vtest.intent test edit --set changes only the named field with a diff, and re-applying the same desired state changes nothing
#[test]
fn edit_set_changes_only_the_named_field_and_is_idempotent() {
    let root = fixture("edit");
    let test_id = create(&root, false)["data"]["test_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let before = fs::read_to_string(root.join("tests/calc.rs")).unwrap();

    let args = [
        "test",
        "edit",
        test_id.as_str(),
        "--set",
        "intent=add returns the sum of both numbers",
    ];
    let first = vtest(&root, &args);
    let first_envelope = envelope(&first);
    assert_eq!(first.status.code(), Some(0), "{first_envelope}");
    assert_eq!(first_envelope["data"]["changed"], true);
    let diff = first_envelope["data"]["diff"].as_str().unwrap();
    assert!(
        diff.contains("-/// @vtest.intent add sums two numbers\n"),
        "{diff}"
    );
    assert!(
        diff.contains("+/// @vtest.intent add returns the sum of both numbers\n"),
        "{diff}"
    );
    let after = fs::read_to_string(root.join("tests/calc.rs")).unwrap();
    assert_eq!(
        after,
        before.replace(
            "/// @vtest.intent add sums two numbers",
            "/// @vtest.intent add returns the sum of both numbers"
        ),
        "only the named field may change"
    );

    let second = vtest(&root, &args);
    let second_envelope = envelope(&second);
    assert_eq!(second.status.code(), Some(0), "{second_envelope}");
    assert_eq!(second_envelope["data"]["changed"], false);
    assert_eq!(
        second_envelope["data"]["diff"], "",
        "an unchanged re-apply has no diff: {second_envelope}"
    );
    assert_eq!(
        fs::read_to_string(root.join("tests/calc.rs")).unwrap(),
        after
    );
    fs::remove_dir_all(root).unwrap();
}

/// @vtest.id TEST-CLI-TEST-OP-REJECTION-NO-WRITE
/// @vtest.covers VO-TEST-OP-ANSWER-ERROR-E-OP-001
/// @vtest.target crates/vtest-cli/src/ops/test_op.rs::create
/// @vtest.intent an invalid answer is rejected as E-OP-001 with exit 2 and no data, and an unknown Test ID for edit is E-OP-002, both without writing
#[test]
fn rejected_operations_exit_two_and_write_nothing() {
    let root = fixture("reject");
    fs::write(
        root.join("answers.yaml"),
        ANSWERS.replace("VO-OP-ADD", "VO-OP-ADDD"),
    )
    .unwrap();
    let output = vtest(
        &root,
        &[
            "test",
            "create",
            "--form",
            "rust-unit-function",
            "--answers",
            "answers.yaml",
        ],
    );
    let rejected = envelope(&output);
    assert_eq!(output.status.code(), Some(2), "{rejected}");
    assert!(rejected["data"].is_null());
    assert_eq!(rejected["diagnostics"][0]["code"], "E-OP-001");
    assert!(
        rejected["diagnostics"][0]["candidates"]
            .as_array()
            .is_some_and(|candidates| candidates.iter().any(|c| c == "VO-OP-ADD")),
        "the misspelt VO must come with candidates: {rejected}"
    );

    let output = vtest(
        &root,
        &["test", "edit", "TEST-NOPE", "--set", "intent=anything"],
    );
    let missing = envelope(&output);
    assert_eq!(output.status.code(), Some(2), "{missing}");
    assert_eq!(missing["diagnostics"][0]["code"], "E-OP-002");
    assert_eq!(
        fs::read_to_string(root.join("tests/calc.rs")).unwrap(),
        CALC
    );
    fs::remove_dir_all(root).unwrap();
}

/// Writes `tests/calc.rs` (or `src/lib.rs` for in-module Tests) for the
/// edit-only fixtures below and returns the written text.
fn write_source(root: &Path, relative: &str, text: &str) -> String {
    fs::write(root.join(relative), text).unwrap();
    text.to_owned()
}

fn edit(root: &Path, args: &[&str]) -> serde_json::Value {
    let output = vtest(root, args);
    let envelope = envelope(&output);
    assert_eq!(output.status.code(), Some(0), "{envelope}");
    envelope
}

/// @vtest.id TEST-CLI-TEST-EDIT-IN-MODULE-TEST
/// @vtest.covers VO-TEST-OP-EDIT-SET-NAMED-FIELDS
/// @vtest.target crates/vtest-cli/src/ops/test_op.rs::edit
/// @vtest.intent test edit --set on a Test inside `#[cfg(test)] mod tests` changes only the named field, and a fn_name rename keeps the Test in its module
#[test]
fn edit_set_works_on_a_test_inside_a_module() {
    let root = fixture("in-module");
    let before = write_source(
        &root,
        "src/lib.rs",
        "pub fn add(a: u32, b: u32) -> u32 {\n    a + b\n}\n\n\
         #[cfg(test)]\n\
         mod tests {\n\
         \x20   use super::*;\n\n\
         \x20   /// @vtest.id TEST-NESTED\n\
         \x20   /// @vtest.covers VO-OP-ADD\n\
         \x20   /// @vtest.target src/lib.rs::add\n\
         \x20   /// @vtest.intent original intent\n\
         \x20   #[test]\n\
         \x20   fn adds() {\n\
         \x20       assert_eq!(add(1, 2), 3);\n\
         \x20   }\n\
         }\n",
    );

    edit(
        &root,
        &["test", "edit", "TEST-NESTED", "--set", "intent=new intent"],
    );
    let after = fs::read_to_string(root.join("src/lib.rs")).unwrap();
    assert_eq!(
        after,
        before.replace("@vtest.intent original intent", "@vtest.intent new intent")
    );

    edit(
        &root,
        &["test", "edit", "TEST-NESTED", "--set", "fn_name=adds_two"],
    );
    let renamed = fs::read_to_string(root.join("src/lib.rs")).unwrap();
    assert_eq!(renamed, after.replace("fn adds()", "fn adds_two()"));
    fs::remove_dir_all(root).unwrap();
}

/// @vtest.id TEST-CLI-TEST-EDIT-SRC-ID-TARGET
/// @vtest.covers VO-TEST-OP-EDIT-SET-NAMED-FIELDS
/// @vtest.target crates/vtest-cli/src/ops/test_op.rs::edit
/// @vtest.intent test edit --set on a Test whose target is a permanent SRC ID keeps the SRC-ADD spelling and changes only the named field, and an undeclared SRC ID is E-OP-001 with no write
#[test]
fn edit_set_keeps_a_permanent_src_id_target() {
    let root = fixture("src-id");
    fs::write(
        root.join("src/lib.rs"),
        "/// @vtest.src-id SRC-ADD\npub fn add(a: u32, b: u32) -> u32 {\n    a + b\n}\n",
    )
    .unwrap();
    let before = write_source(
        &root,
        "tests/calc.rs",
        "use ops_fixture::add;\n\n\
         /// @vtest.id TEST-FLAT\n\
         /// @vtest.covers VO-OP-ADD\n\
         /// @vtest.target SRC-ADD\n\
         /// @vtest.intent original intent\n\
         #[test]\n\
         fn adds() {\n\
         \x20   assert_eq!(add(1, 2), 3);\n\
         }\n",
    );

    edit(
        &root,
        &["test", "edit", "TEST-FLAT", "--set", "intent=new intent"],
    );
    let after = fs::read_to_string(root.join("tests/calc.rs")).unwrap();
    assert_eq!(
        after,
        before.replace("@vtest.intent original intent", "@vtest.intent new intent")
    );

    let output = vtest(
        &root,
        &["test", "edit", "TEST-FLAT", "--set", "target=SRC-ADDD"],
    );
    let rejected = envelope(&output);
    assert_eq!(output.status.code(), Some(2), "{rejected}");
    assert_eq!(rejected["diagnostics"][0]["code"], "E-OP-001");
    assert!(
        rejected["diagnostics"][0]["candidates"]
            .as_array()
            .is_some_and(|candidates| candidates.iter().any(|c| c == "SRC-ADD")),
        "{rejected}"
    );
    assert_eq!(
        fs::read_to_string(root.join("tests/calc.rs")).unwrap(),
        after
    );
    fs::remove_dir_all(root).unwrap();
}

/// @vtest.id TEST-CLI-TEST-EDIT-BODY-NON-ASCII-LINE
/// @vtest.covers VO-TEST-OP-EDIT-BODY-FILE
/// @vtest.target crates/vtest-cli/src/ops/test_op.rs::edit
/// @vtest.intent test edit --body-file replaces the whole body of a one-line Test whose line holds a Japanese literal, leaving its metadata and the rest of the file unchanged
#[test]
fn edit_body_file_replaces_a_one_line_body_with_non_ascii_text() {
    let root = fixture("body-utf8");
    let before = write_source(
        &root,
        "tests/calc.rs",
        "use ops_fixture::add;\n\n\
         /// @vtest.id TEST-FLAT\n\
         /// @vtest.covers VO-OP-ADD\n\
         /// @vtest.target src/lib.rs::add\n\
         /// @vtest.intent original intent\n\
         #[test]\n\
         fn adds() { assert_eq!(\"値\", \"値\"); }\n\n\
         // 後続の行\n",
    );
    fs::write(root.join("body.rs"), "{ assert_eq!(add(1, 2), 3); }\n").unwrap();

    let edited = edit(
        &root,
        &["test", "edit", "TEST-FLAT", "--body-file", "body.rs"],
    );
    assert_eq!(edited["data"]["changed"], true, "{edited}");
    assert_eq!(
        fs::read_to_string(root.join("tests/calc.rs")).unwrap(),
        before.replace(
            "{ assert_eq!(\"値\", \"値\"); }",
            "{ assert_eq!(add(1, 2), 3); }"
        )
    );
    fs::remove_dir_all(root).unwrap();
}

/// @vtest.id TEST-CLI-INIT-WRITES-ADAPTER-BUILT-IN-FORMS
/// @vtest.covers VO-STORE-INIT-PROJECT-CANONICAL-LAYOUT, VO-FORM-KIND-UNIQUE-OWNER-DECLARED
/// @vtest.target crates/vtest-cli/src/ops/init.rs::execute
/// @vtest.intent vtest init writes the registered adapter's built-in Form Schemas into .verify/forms/, each declaring its owner adapter
#[test]
fn init_writes_the_adapters_built_in_forms_with_their_owner() {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "vtest-test-op-init-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).unwrap();
    let output = vtest(&root, &["init"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    for kind in ["rust-unit-function", "rust-integration"] {
        let schema =
            fs::read_to_string(root.join(".verify/forms").join(format!("{kind}.yaml"))).unwrap();
        let parsed = vtest_store::parse_form_schema(&schema).unwrap();
        assert_eq!(parsed.kind, kind);
        assert_eq!(parsed.adapter.as_deref(), Some("rust-cargo"));
    }
    fs::remove_dir_all(root).unwrap();
}
