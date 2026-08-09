//! M5 command-line acceptance coverage for semantic audit bundles and submit.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::Value;
use vtest_store::{read_audit, read_record_ids, VerifyLayout};

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
            "vtest-cli-m5-{name}-{}-{nanos}-{sequence}",
            std::process::id()
        ));
        copy_tree(&fixture_path("m1/base"), &root);
        fs::create_dir_all(root.join(".verify/approvals"))
            .expect("restore canonical approval directory");
        Self { root }
    }

    fn commit_baseline(&self) {
        run_git(
            &self.root,
            ["init", "-q"],
            "initialize temporary git repository",
        );
        run_git(
            &self.root,
            ["config", "user.email", "m5-acceptance@example.invalid"],
            "configure temporary git email",
        );
        run_git(
            &self.root,
            ["config", "user.name", "M5 Acceptance"],
            "configure temporary git name",
        );
        run_git(
            &self.root,
            ["config", "commit.gpgsign", "false"],
            "disable signing for the disposable baseline commit",
        );
        run_git(&self.root, ["add", "."], "stage temporary baseline");
        run_git(
            &self.root,
            ["commit", "-qm", "M5 acceptance baseline"],
            "commit temporary baseline",
        );
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

fn run_git<const N: usize>(root: &Path, args: [&str; N], context: &str) {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "{context}: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
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

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("vtest emits UTF-8 JSON")
}

fn envelope(output: &Output) -> Value {
    let json = stdout(output);
    let value: Value = serde_json::from_str(&json).expect("valid JSON envelope");
    let object = value.as_object().expect("JSON envelope is an object");
    assert!(object.get("ok").is_some_and(Value::is_boolean));
    assert!(object.contains_key("data"), "missing data: {json}");
    assert!(object.get("diagnostics").is_some_and(Value::is_array));
    value
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

fn assert_ok(output: &Output, context: &str) -> Value {
    assert_exit(output, 0, context);
    let value = envelope(output);
    assert_eq!(value["ok"], true, "{context}: {value}");
    value
}

fn assert_usage_error(output: &Output, code: &str, context: &str) -> Value {
    assert_exit(output, 2, context);
    let value = envelope(output);
    assert_eq!(value["ok"], false, "{context}: {value}");
    assert!(
        value["diagnostics"]
            .as_array()
            .expect("diagnostics is an array")
            .iter()
            .any(|diagnostic| diagnostic["code"] == code),
        "missing {code}: {value}"
    );
    value
}

fn bundle(project: &TempProject, kind: &str, selector: &[&str]) -> (String, Value) {
    let mut args = vec!["bundle", "--kind", kind];
    args.extend_from_slice(selector);
    if kind == "test-semantic" {
        args.push("--include-failed");
    }
    let response = assert_ok(
        &invoke(&project.root, "audit", &args),
        "generate audit bundle",
    );
    let bundle_id = response["data"]["bundle_id"]
        .as_str()
        .expect("bundle response has bundle_id")
        .to_owned();
    let path = VerifyLayout::new(&project.root)
        .cache_dir()
        .join("bundles")
        .join(format!("{bundle_id}.json"));
    let text = fs::read_to_string(path).expect("read generated bundle cache");
    let value = serde_json::from_str(&text).expect("bundle cache is valid JSON");
    (bundle_id, value)
}

fn valid_submission(project: &TempProject, bundle_id: &str) -> (Value, PathBuf) {
    let file = project.root.join("m5-submission.json");
    let value = serde_json::json!({
        "bundle_id": bundle_id,
        "kind": "test-semantic",
        "verdict": "PASS",
        "reasons": [{
            "claim": "the test has a traceable semantic basis",
            "basis": [{"kind": "test-code", "ref": "tests/registered.rs::clean_scan_baseline"}]
        }],
        "exclusions": [],
        "auditor": {"kind": "agent", "id": "m5-acceptance", "model": "acceptance"},
        "confidence": "high"
    });
    fs::write(&file, serde_json::to_vec_pretty(&value).unwrap()).expect("write submission");
    let path = file.to_string_lossy().into_owned();
    (value, PathBuf::from(path))
}

fn submit(project: &TempProject, file: &Path, context: &str) -> Value {
    let file = file.to_string_lossy().into_owned();
    assert_ok(
        &invoke(&project.root, "audit", &["submit", "--file", &file]),
        context,
    )
}

#[test]
fn m5_bundles_include_schema_fields_for_all_audit_kinds() {
    let project = TempProject::from_m1_base("bundle-schema");
    project.commit_baseline();

    let (test_bundle_id, test_bundle) =
        bundle(&project, "test-semantic", &["--test", "TEST-M1-CLEAN"]);
    assert_eq!(test_bundle["bundle_id"], test_bundle_id);
    assert_eq!(test_bundle["kind"], "test-semantic");
    for key in [
        "generated_at",
        "revision",
        "test",
        "vos",
        "target",
        "related_tests",
        "sibling_tests",
        "static_audit",
        "prior_audits",
        "subjects",
    ] {
        assert!(
            !test_bundle[key].is_null(),
            "missing test-semantic field {key}"
        );
    }
    for key in [
        "id",
        "intent",
        "annotations",
        "location",
        "source",
        "content_hash",
    ] {
        assert!(
            !test_bundle["test"][key].is_null(),
            "missing test field {key}"
        );
    }
    for key in ["locator", "source", "content_hash"] {
        assert!(
            !test_bundle["target"][key].is_null(),
            "missing target field {key}"
        );
    }

    for (kind, selector) in [
        ("vo-coverage", ["--vo", "VO-KNOWN"].as_slice()),
        ("impl-consistency", ["--test", "TEST-M1-CLEAN"].as_slice()),
    ] {
        let (_, value) = bundle(&project, kind, selector);
        assert_eq!(value["kind"], kind);
        assert!(value["subjects"]
            .as_array()
            .is_some_and(|items| !items.is_empty()));
    }
}

#[test]
fn m5_empty_reasons_are_rejected_without_an_audit_record() {
    let project = TempProject::from_m1_base("empty-reasons");
    project.commit_baseline();
    let (bundle_id, _) = bundle(&project, "test-semantic", &["--test", "TEST-M1-CLEAN"]);
    let file = project.root.join("empty-reasons.json");
    fs::write(
        &file,
        serde_json::to_vec_pretty(&serde_json::json!({
            "bundle_id": bundle_id,
            "kind": "test-semantic",
            "verdict": "PASS",
            "reasons": [],
        }))
        .unwrap(),
    )
    .expect("write empty-reasons submission");
    let before =
        read_record_ids(&VerifyLayout::new(&project.root).audits_dir()).unwrap_or_default();
    let file = file.to_string_lossy().into_owned();
    assert_usage_error(
        &invoke(&project.root, "audit", &["submit", "--file", &file]),
        "E-AUDIT-005",
        "empty reasons must be rejected",
    );
    let after = read_record_ids(&VerifyLayout::new(&project.root).audits_dir()).unwrap_or_default();
    assert_eq!(
        before, after,
        "rejected submission must not append an audit"
    );
}

#[test]
fn m5_changed_test_rejects_submission_with_e_audit_002() {
    let project = TempProject::from_m1_base("stale-bundle");
    project.commit_baseline();
    let (bundle_id, _) = bundle(&project, "test-semantic", &["--test", "TEST-M1-CLEAN"]);
    let (_, file) = valid_submission(&project, &bundle_id);
    let registered = project.root.join("tests/registered.rs");
    let source = fs::read_to_string(&registered).expect("read registered test");
    let source = source.replace(
        "fn clean_scan_baseline() {}",
        "fn clean_scan_baseline() { let marker = 1; let _ = marker; }",
    );
    fs::write(&registered, source).expect("mutate Test source after bundle generation");

    let file = file.to_string_lossy().into_owned();
    let stale = invoke(&project.root, "audit", &["submit", "--file", &file]);
    assert_exit(&stale, 1, "changed Test must invalidate the cached bundle");
    let stale = envelope(&stale);
    assert!(stale["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|diagnostic| diagnostic["code"] == "E-AUDIT-002"));
    assert!(
        read_record_ids(&VerifyLayout::new(&project.root).audits_dir())
            .unwrap_or_default()
            .is_empty(),
        "stale bundle submission must not append an audit"
    );
}

#[test]
fn m5_accepted_audit_is_typed_and_becomes_stale_after_target_change() {
    let project = TempProject::from_m1_base("accepted-stale");
    project.commit_baseline();
    let (bundle_id, _) = bundle(&project, "test-semantic", &["--test", "TEST-M1-CLEAN"]);
    let (_, file) = valid_submission(&project, &bundle_id);
    let accepted = submit(&project, &file, "accept semantic audit");
    let audit_id = accepted["data"]["audit_id"]
        .as_str()
        .expect("accepted response has audit_id");
    let audit_path = VerifyLayout::new(&project.root)
        .audits_dir()
        .join(format!("{audit_id}.yaml"));
    let record = read_audit(&audit_path).expect("accepted audit is a typed canonical record");
    assert_eq!(record.kind, "test-semantic");
    assert_eq!(record.bundle_id.as_deref(), Some(bundle_id.as_str()));
    assert_eq!(record.verdict, "PASS");

    let verify = assert_ok(
        &invoke(&project.root, "verify", &["--items", "semantic_audit"]),
        "accepted semantic audit is currently valid",
    );
    assert_eq!(verify["data"]["report"]["result"], "PASS");

    fs::write(
        project.root.join("src/lib.rs"),
        "pub fn known() { let changed = 1; let _ = changed; }\n",
    )
    .expect("mutate target subject after acceptance");
    let verify = invoke(&project.root, "verify", &["--items", "semantic_audit"]);
    assert_exit(&verify, 1, "target mutation stales accepted audit");
    let verify = envelope(&verify);
    let item = verify["data"]["report"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["item"] == "semantic_audit")
        .unwrap();
    assert_eq!(item["value"], "STALE");
}
