//! M4 command-line acceptance coverage for execution and Evidence.
//!
//! These tests exercise the public binary against isolated copies of the
//! tracked M1 fixture.  Each project is committed before execution so the
//! Evidence revision binding can be checked as well as the content hashes.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::Value;
use vtest_store::{read_evidence, VerifyLayout};

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
            "vtest-cli-m4-{name}-{}-{nanos}-{sequence}",
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
            ["config", "user.email", "m4-acceptance@example.invalid"],
            "configure temporary git email",
        );
        run_git(
            &self.root,
            ["config", "user.name", "M4 Acceptance"],
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
            ["commit", "-qm", "M4 acceptance baseline"],
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
    assert!(
        object.get("diagnostics").is_some_and(Value::is_array),
        "missing diagnostics array: {json}"
    );
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

fn evidence_files(project: &TempProject) -> Vec<PathBuf> {
    let directory = VerifyLayout::new(&project.root).evidence_dir();
    if !directory.exists() {
        return Vec::new();
    }
    let mut files = fs::read_dir(directory)
        .expect("read evidence directory")
        .map(|entry| entry.expect("read evidence entry").path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("yaml"))
        .collect::<Vec<_>>();
    files.sort();
    files
}

fn report_item<'a>(response: &'a Value, item: &str) -> &'a Value {
    response["data"]["report"]["items"]
        .as_array()
        .expect("verification report items is an array")
        .iter()
        .find(|entry| entry["item"] == item)
        .unwrap_or_else(|| panic!("missing report item {item}: {response}"))
}

#[test]
fn m4_run_fast_records_one_evidence_per_registered_test() {
    let project = TempProject::from_m1_base("run-fast");
    project.commit_baseline();

    let run = assert_ok(
        &invoke(&project.root, "run", &["--all", "--fast"]),
        "run all registered tests in fast mode",
    );
    let evidence = run["data"]["evidence"]
        .as_array()
        .expect("run data contains evidence array");
    assert_eq!(evidence.len(), 1, "one registered Test has one Evidence");
    assert_eq!(evidence[0]["test_id"], "TEST-M1-CLEAN");
    assert_eq!(evidence[0]["result"], "PASS");
    // §442/§3.7: a not-checked execution stores result null with an empty
    // targets list; the NOT_CHECKED verification value is derived by verify.
    assert_eq!(evidence[0]["target_execution"]["checked"], false);
    assert!(evidence[0]["target_execution"]["result"].is_null());
    assert!(evidence[0]["target_execution"]["targets"]
        .as_array()
        .is_some_and(Vec::is_empty));
    assert_eq!(evidence[0]["runner"]["kind"], "cargo-test");
    assert!(evidence[0]["revision"]["commit"].as_str().is_some());
    assert_eq!(evidence_files(&project).len(), 1);

    let record = read_evidence(&evidence_files(&project)[0]).expect("read generated Evidence");
    assert_eq!(record.test_id.as_str(), "TEST-M1-CLEAN");
    assert!(!record.revision.dirty);
    assert!(project.root.join(".verify").join(&record.log_ref).is_file());

    let verify = assert_ok(
        &invoke(&project.root, "verify", &["--items", "evidence_validity"]),
        "verify the freshly generated Evidence",
    );
    assert_eq!(report_item(&verify, "evidence_validity")["value"], "PASS");
}

#[test]
fn m4_multi_target_evidence_records_every_declared_target_hash() {
    let project = TempProject::from_m1_base("multi-target");
    fs::write(
        project.root.join("src/lib.rs"),
        "pub fn known() {}\npub fn also_known() {}\n",
    )
    .expect("add a second target function");
    let registered = project.root.join("tests/registered.rs");
    let source = fs::read_to_string(&registered).expect("read registered test");
    let source = source
        .replace("/// @vtest.target src/lib.rs::known", "/// @vtest.target src/lib.rs::known\n/// @vtest.target src/lib.rs::also_known")
        .replace(
            "/// @vtest.intent provides a clean M1 scan baseline",
            "/// @vtest.intent provides a clean M4 multi-target baseline\n/// @vtest.kind integration-normal",
        );
    fs::write(&registered, source).expect("add the second target annotation");
    project.commit_baseline();

    let run = assert_ok(
        &invoke(&project.root, "run", &["--all", "--fast"]),
        "run a Test with two declared targets",
    );
    let evidence = run["data"]["evidence"]
        .as_array()
        .expect("run data contains evidence array");
    assert_eq!(evidence.len(), 1);
    assert_eq!(
        evidence[0]["hashes"]["targets"].as_array().unwrap().len(),
        2
    );

    let files = evidence_files(&project);
    assert_eq!(files.len(), 1);
    let record = read_evidence(&files[0]).expect("read multi-target Evidence");
    assert_eq!(record.hashes.targets.len(), 2);
    assert_ne!(
        record.hashes.targets[0].target_construct,
        record.hashes.targets[1].target_construct
    );

    let verify = assert_ok(
        &invoke(&project.root, "verify", &["--items", "evidence_validity"]),
        "verify all target hashes",
    );
    assert_eq!(report_item(&verify, "evidence_validity")["value"], "PASS");
}

#[test]
fn m4_target_mutation_makes_evidence_stale() {
    let project = TempProject::from_m1_base("stale-target");
    project.commit_baseline();
    assert_ok(
        &invoke(&project.root, "run", &["--all", "--fast"]),
        "record baseline Evidence before target mutation",
    );

    fs::write(
        project.root.join("src/lib.rs"),
        "pub fn known() { let changed = 1; let _ = changed; }\n",
    )
    .expect("mutate target source");
    let verify = invoke(&project.root, "verify", &["--items", "evidence_validity"]);
    assert_exit(&verify, 1, "target mutation invalidates Evidence");
    let verify = envelope(&verify);
    assert_eq!(verify["ok"], false);
    assert_eq!(report_item(&verify, "evidence_validity")["value"], "STALE");
}

#[test]
fn m4_build_failure_reports_e_exec_001_without_evidence() {
    let project = TempProject::from_m1_base("build-failure");
    project.commit_baseline();
    fs::write(project.root.join("src/lib.rs"), "pub fn known( {\n")
        .expect("break target source before execution");

    let run = invoke(&project.root, "run", &["--all", "--fast"]);
    assert_exit(&run, 1, "build failure is a verification failure");
    let response = envelope(&run);
    assert_eq!(response["ok"], false);
    assert!(
        response["diagnostics"]
            .as_array()
            .expect("diagnostics is an array")
            .iter()
            .any(|diagnostic| diagnostic["code"] == "E-EXEC-001"),
        "missing E-EXEC-001: {response}"
    );
    assert!(response["data"]["evidence"]
        .as_array()
        .expect("run data contains evidence array")
        .is_empty());
    assert!(
        evidence_files(&project).is_empty(),
        "build failure must not publish Evidence"
    );
}

#[test]
fn m4_ignored_test_emits_no_evidence() {
    let project = TempProject::from_m1_base("ignored");
    project.commit_baseline();
    let registered = project.root.join("tests/registered.rs");
    let source = fs::read_to_string(&registered).expect("read registered test");
    fs::write(&registered, source.replace("#[test]", "#[test]\n#[ignore]"))
        .expect("mark registered test ignored");

    let run = assert_ok(
        &invoke(&project.root, "run", &["--all", "--fast"]),
        "ignored test remains a successful non-execution",
    );
    assert!(run["data"]["evidence"]
        .as_array()
        .expect("run data contains evidence array")
        .is_empty());
    assert!(evidence_files(&project).is_empty());
}

#[test]
fn m4_missing_result_line_emits_e_exec_002_without_evidence() {
    let project = TempProject::from_m1_base("missing-result");
    project.commit_baseline();
    let registered = project.root.join("tests/registered.rs");
    let source = fs::read_to_string(&registered).expect("read registered test");
    fs::write(
        &registered,
        format!("#[cfg(feature = \"never-enabled\")]\n{source}"),
    )
    .expect("compile registered test out of the test binary");

    let run = invoke(&project.root, "run", &["--all", "--fast"]);
    assert_exit(&run, 1, "missing result line is a verification failure");
    let response = envelope(&run);
    assert!(
        response["diagnostics"]
            .as_array()
            .expect("diagnostics is an array")
            .iter()
            .any(|diagnostic| diagnostic["code"] == "E-EXEC-002"),
        "missing E-EXEC-002: {response}"
    );
    assert!(evidence_files(&project).is_empty());
}
