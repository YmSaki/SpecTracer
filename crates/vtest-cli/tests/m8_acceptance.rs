//! M8 command-line acceptance coverage for Structured Test Operations.

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
            "vtest-cli-m8-{name}-{}-{nanos}-{sequence}",
            std::process::id()
        ));
        copy_tree(&fixture_path("m1/base"), &root);
        for directory in ["approvals", "audits", "evidence", "rel"] {
            fs::create_dir_all(root.join(".verify").join(directory))
                .expect("restore canonical record directory");
        }
        Self { root }
    }

    fn write_answers(&self, name: &str, target: &str, covers: &str, function: &str) {
        fs::write(
            self.root.join(name),
            format!(
                "form: rust-unit-function\nanswers:\n  target: {target}\n  covers: [{covers}]\n  behavior: generated behavior\n  test_kind: normal\n  input: ordinary input\n  expect: expected result\n  fn_name: {function}\n"
            ),
        )
        .expect("write form answers");
    }

    fn add_alt_vo(&self) {
        fs::write(
            self.root.join(".verify/vo/VO-ALT.yaml"),
            "id: VO-ALT\nparent: null\nrequirements: []\nspec_refs: []\nclaim: alternate behavior\ndimensions: []\ncoverage_policy: null\nrepresentative_cases: []\nstatus: draft\ncreated: '2026-01-01'\nupdated: '2026-01-01'\n",
        )
        .expect("write alternate VO");
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
    Command::new(env!("CARGO_BIN_EXE_vtest"))
        .arg("--project")
        .arg(project)
        .args(["--format", "json", command])
        .args(args)
        .output()
        .expect("run vtest process")
}

fn envelope(output: &Output) -> Value {
    let text = String::from_utf8(output.stdout.clone()).expect("vtest emits UTF-8 JSON");
    serde_json::from_str(&text).unwrap_or_else(|error| {
        panic!(
            "invalid JSON envelope ({error}): stdout={text} stderr={}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn assert_exit(output: &Output, expected: i32, context: &str) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{context}: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_ok(output: &Output, context: &str) -> Value {
    assert_exit(output, 0, context);
    let value = envelope(output);
    assert_eq!(value["ok"], true, "{context}: {value}");
    value
}

fn scanned_test(response: &Value, id: &str) -> Value {
    response["data"]["tests"]
        .as_array()
        .expect("scan has tests")
        .iter()
        .find(|test| test["id"] == id)
        .cloned()
        .unwrap_or_else(|| panic!("scan did not contain {id}: {response}"))
}

#[test]
fn m8_invalid_symbol_is_rejected_with_candidates() {
    let project = TempProject::from_m1_base("invalid-symbol");
    project.write_answers("bad.yaml", "src/lib.rs::knwon", "VO-KNOWN", "bad_symbol");
    let output = invoke(
        &project.root,
        "test",
        &[
            "create",
            "--form",
            "rust-unit-function",
            "--answers",
            "bad.yaml",
            "--id",
            "TEST-M8-BAD",
            "--dry-run",
        ],
    );
    assert_exit(&output, 2, "invalid source symbol is a usage error");
    let value = envelope(&output);
    let diagnostic = &value["diagnostics"][0];
    assert_eq!(diagnostic["code"], "E-OP-001");
    assert_eq!(diagnostic["candidates"][0], "src/lib.rs::known");
}

#[test]
fn m8_test_create_is_scanned_and_exposed_by_queries() {
    let project = TempProject::from_m1_base("create");
    project.write_answers(
        "answers.yaml",
        "src/lib.rs::known",
        "VO-KNOWN",
        "generated_m8_test",
    );
    let source_before = fs::read_to_string(project.root.join("src/lib.rs")).expect("read source");
    let dry_run = invoke(
        &project.root,
        "test",
        &[
            "create",
            "--form",
            "rust-unit-function",
            "--answers",
            "answers.yaml",
            "--id",
            "TEST-M8-CREATED",
            "--dry-run",
        ],
    );
    let dry_data = assert_ok(&dry_run, "dry-run create");
    assert_eq!(dry_data["data"]["changed"], true);
    assert_eq!(
        fs::read_to_string(project.root.join("src/lib.rs")).expect("read source after dry-run"),
        source_before
    );

    let created = invoke(
        &project.root,
        "test",
        &[
            "create",
            "--form",
            "rust-unit-function",
            "--answers",
            "answers.yaml",
            "--id",
            "TEST-M8-CREATED",
        ],
    );
    let created_data = assert_ok(&created, "create structured Test");
    assert_eq!(created_data["data"]["test_id"], "TEST-M8-CREATED");

    let scan = assert_ok(&invoke(&project.root, "scan", &[]), "rescan generated Test");
    let test = scanned_test(&scan, "TEST-M8-CREATED");
    assert_eq!(test["intent"], "generated behavior");
    assert_eq!(test["location"]["path"], "src/lib.rs");
    assert_eq!(test["target"]["value"]["value"], "src/lib.rs::known");

    let show = assert_ok(
        &invoke(&project.root, "test", &["show", "TEST-M8-CREATED"]),
        "show generated Test",
    );
    assert_eq!(show["data"]["id"], "TEST-M8-CREATED");
    let list = assert_ok(
        &invoke(&project.root, "test", &["list", "--vo", "VO-KNOWN"]),
        "list generated Test",
    );
    assert!(list["data"]["tests"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["id"] == "TEST-M8-CREATED"));
    let query = assert_ok(
        &invoke(
            &project.root,
            "test",
            &["query", "--source", "src/lib.rs::known"],
        ),
        "query target symbol",
    );
    assert!(query["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["id"] == "TEST-M8-CREATED"));
}

#[test]
fn m8_edit_changes_only_selected_test_and_preserves_other_hash() {
    let project = TempProject::from_m1_base("edit-boundary");
    project.add_alt_vo();
    project.write_answers(
        "first.yaml",
        "src/lib.rs::known",
        "VO-KNOWN",
        "first_m8_test",
    );
    project.write_answers(
        "second.yaml",
        "src/lib.rs::known",
        "VO-KNOWN",
        "second_m8_test",
    );
    for (answers, id) in [
        ("first.yaml", "TEST-M8-FIRST"),
        ("second.yaml", "TEST-M8-SECOND"),
    ] {
        assert_ok(
            &invoke(
                &project.root,
                "test",
                &[
                    "create",
                    "--form",
                    "rust-unit-function",
                    "--answers",
                    answers,
                    "--id",
                    id,
                ],
            ),
            "create boundary fixture Test",
        );
    }
    let before_scan = assert_ok(&invoke(&project.root, "scan", &[]), "scan before edit");
    let before_second = scanned_test(&before_scan, "TEST-M8-SECOND");
    let before_hash = before_second["content_hash"].clone();
    let edit = invoke(
        &project.root,
        "test",
        &["edit", "TEST-M8-FIRST", "--set", "covers=VO-ALT"],
    );
    let edit_data = assert_ok(&edit, "edit selected Test covers");
    assert_eq!(edit_data["data"]["changed"], true);
    let after_scan = assert_ok(&invoke(&project.root, "scan", &[]), "scan after edit");
    assert_eq!(
        scanned_test(&after_scan, "TEST-M8-FIRST")["covers"][0],
        "VO-ALT"
    );
    assert_eq!(
        scanned_test(&after_scan, "TEST-M8-SECOND")["content_hash"],
        before_hash,
        "editing one Test must preserve every other Test hash"
    );
}

#[test]
fn m8_reapplying_same_edit_is_byte_idempotent() {
    let project = TempProject::from_m1_base("idempotent");
    project.add_alt_vo();
    project.write_answers(
        "answers.yaml",
        "src/lib.rs::known",
        "VO-KNOWN",
        "idempotent_m8_test",
    );
    assert_ok(
        &invoke(
            &project.root,
            "test",
            &[
                "create",
                "--form",
                "rust-unit-function",
                "--answers",
                "answers.yaml",
                "--id",
                "TEST-M8-IDEMPOTENT",
            ],
        ),
        "create idempotence fixture Test",
    );
    let first = assert_ok(
        &invoke(
            &project.root,
            "test",
            &["edit", "TEST-M8-IDEMPOTENT", "--set", "covers=VO-ALT"],
        ),
        "apply desired state",
    );
    assert_eq!(first["data"]["changed"], true);
    let source_after_first =
        fs::read_to_string(project.root.join("src/lib.rs")).expect("read source after first edit");
    let second = assert_ok(
        &invoke(
            &project.root,
            "test",
            &["edit", "TEST-M8-IDEMPOTENT", "--set", "covers=VO-ALT"],
        ),
        "reapply desired state",
    );
    assert_eq!(second["data"]["changed"], false);
    assert_eq!(
        fs::read_to_string(project.root.join("src/lib.rs")).expect("read source after second edit"),
        source_after_first,
        "reapplying the same desired state must be byte-idempotent"
    );
}
