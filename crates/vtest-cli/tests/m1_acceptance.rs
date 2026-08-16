//! M1 command-line acceptance coverage.
//!
//! These tests deliberately exercise the installed test binary rather than
//! calling the CLI library.  Each generated project has an independent
//! temporary directory so the suite remains safe under Rust's parallel test
//! runner and never changes the tracked calc fixture.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT_PROJECT: AtomicU64 = AtomicU64::new(0);

struct TempProject {
    root: PathBuf,
}

impl TempProject {
    fn new(name: &str) -> Self {
        let sequence = NEXT_PROJECT.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "vtest-cli-m1-{name}-{}-{nanos}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create temporary project root");
        Self { root }
    }

    fn initialize(&self, name: &str) {
        let output = invoke(&self.root, "init", &["--name", name]);
        assert_success(&output, 0, "initialize temporary project");
        assert_json_envelope(&stdout(&output));
    }

    fn copy_fixture(&self, relative: &str) {
        copy_tree(&fixture_path(relative), &self.root);
    }

    fn copy_case(&self, case: &str, destination: &str) {
        let source = fixture_path(&format!("m1/cases/{case}"));
        let destination = self.root.join(destination);
        fs::create_dir_all(destination.parent().expect("case destination has a parent"))
            .expect("create case destination directory");
        fs::copy(source, destination).expect("copy tracked M1 case fixture");
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

impl Drop for TempProject {
    fn drop(&mut self) {
        if self.root.exists() {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

fn invoke(project: &Path, command: &str, args: &[&str]) -> Output {
    invoke_format(project, "json", command, args)
}

fn invoke_format(project: &Path, format: &str, command: &str, args: &[&str]) -> Output {
    let mut invocation = Command::new(env!("CARGO_BIN_EXE_vtest"));
    invocation
        .arg("--project")
        .arg(project)
        .arg("--format")
        .arg(format)
        .arg(command)
        .args(args);
    invocation.output().expect("run vtest process")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("vtest emits UTF-8 JSON")
}

fn assert_success(output: &Output, expected: i32, context: &str) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{context}: stdout={} stderr={}",
        stdout(output),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_json_envelope(json: &str) {
    let value: serde_json::Value = serde_json::from_str(json).expect("valid JSON envelope");
    let object = value.as_object().expect("JSON envelope is an object");
    assert!(object.get("ok").is_some_and(|value| value.is_boolean()));
    assert!(object.contains_key("data"), "missing envelope data: {json}");
    let diagnostics = object
        .get("diagnostics")
        .and_then(serde_json::Value::as_array)
        .expect("envelope diagnostics is an array");
    for diagnostic in diagnostics {
        let diagnostic = diagnostic.as_object().expect("diagnostic is an object");
        for field in ["code", "severity", "message"] {
            assert!(
                diagnostic.get(field).is_some_and(|value| value.is_string()),
                "diagnostic lacks string field {field}: {json}"
            );
        }
    }
}

fn assert_scan_diagnostics_have_locations(json: &str) {
    let value: serde_json::Value = serde_json::from_str(json).expect("valid scan JSON");
    for diagnostic in value["diagnostics"]
        .as_array()
        .expect("scan diagnostics is an array")
    {
        assert!(
            diagnostic["location"]["path"].is_string(),
            "scanner diagnostic has no file location: {diagnostic}"
        );
    }
}

/// @vtest.id TEST-CLI-048
/// @vtest.covers VO-CLI-013
/// @vtest.target crates/vtest-cli/src/lib.rs::run
/// @vtest.intent scan/doctor/text agree on calc fixture: 7 tests, E-SCAN-003, exit 1, runnable selection
#[test]
fn m1_calc_fixture_extracts_tests_and_scan_matches_doctor() {
    let fixture = fixture_path("")
        .canonicalize()
        .expect("tracked calc fixture exists");

    let scan = invoke(&fixture, "scan", &[]);
    let doctor = invoke(&fixture, "doctor", &[]);
    let text_scan = invoke_format(&fixture, "text", "scan", &[]);
    assert_success(
        &scan,
        1,
        "calc fixture scan has its intentional dangling VO error",
    );
    assert_success(&doctor, 1, "doctor has the same diagnostics as scan");
    assert_success(&text_scan, 1, "text scan reports the same error state");
    let text = stdout(&text_scan);
    assert!(text.starts_with("NG\n") || text.starts_with("NG\r\n"));
    assert!(text.contains("E-SCAN-003"));
    assert!(text.contains("W-SCAN-101"));

    let scan_json = stdout(&scan);
    assert_json_envelope(&scan_json);
    assert_scan_diagnostics_have_locations(&scan_json);
    assert_eq!(
        scan_json,
        stdout(&doctor),
        "scan and doctor must be equivalent"
    );
    assert!(scan_json.contains("\"ok\": false"));
    assert!(scan_json.contains("\"tests\": 7"));
    assert!(scan_json.contains("\"E-SCAN-003\""));
    assert!(scan_json.contains("\"W-SCAN-101\""));

    let envelope: serde_json::Value = serde_json::from_str(&scan_json).unwrap();
    let tests = envelope["data"]["tests"].as_array().unwrap();
    let expected = [
        ("TEST-CALC-ADD", "adds_two_integers"),
        ("TEST-CALC-TABLE", "table_driven_additions"),
        ("TEST-CALC-ASSERT-TRUE", "assert_true_only"),
        ("TEST-CALC-NO-CALL", "target_not_called"),
        ("TEST-CALC-NO-ASSERT", "no_result_assertion"),
        ("TEST-CALC-SELF-COMPARE", "self_compare"),
        ("TEST-CALC-DANGLING", "dangling_vo_reference"),
    ];
    assert_eq!(tests.len(), expected.len());
    for (id, filter) in expected {
        let test = tests
            .iter()
            .find(|test| test["id"] == id)
            .unwrap_or_else(|| panic!("missing {id}"));
        assert_eq!(test["filter"], filter, "wrong filter for {id}");
        assert_eq!(test["package"], "calc-fixture", "wrong package for {id}");
        assert_eq!(test["test_target"]["kind"], "integration_test");
        assert_eq!(test["test_target"]["name"], "calc_test");
    }

    let exact = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .current_dir(&fixture)
        .args([
            "test",
            "-p",
            "calc-fixture",
            "--test",
            "calc_test",
            "--",
            "--exact",
            "adds_two_integers",
        ])
        .output()
        .expect("execute the scanner-derived cargo test selection");
    assert_success(
        &exact,
        0,
        "package, integration target, and exact filter must select one runnable test",
    );
    assert!(
        stdout(&exact).contains("test adds_two_integers ... ok"),
        "exact cargo selection did not execute the expected test: stdout={} stderr={}",
        stdout(&exact),
        String::from_utf8_lossy(&exact.stderr)
    );
}

/// @vtest.id TEST-CLI-049
/// @vtest.covers VO-CLI-013
/// @vtest.target crates/vtest-cli/src/lib.rs::run
/// @vtest.intent warning-only scan (W-SCAN-101) exits 0 with ok:true and located diagnostic
#[test]
fn m1_warning_only_scan_exits_zero() {
    let project = TempProject::new("warning-only");
    project.copy_fixture("m1/base");

    let clean = invoke(&project.root, "scan", &[]);
    assert_success(&clean, 0, "tracked clean fixture must scan successfully");
    assert_json_envelope(&stdout(&clean));

    project.copy_case("unregistered.rs", "tests/unregistered.rs");

    let output = invoke(&project.root, "scan", &[]);
    assert_success(&output, 0, "warning-only scan must not fail verification");
    let json = stdout(&output);
    assert_json_envelope(&json);
    assert_scan_diagnostics_have_locations(&json);
    assert!(json.contains("\"ok\": true"));
    assert!(json.contains("\"W-SCAN-101\""));
    assert!(!json.contains("\"severity\": \"error\""));
    let envelope: serde_json::Value = serde_json::from_str(&json).unwrap();
    let warning = envelope["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|diagnostic| diagnostic["code"] == "W-SCAN-101")
        .unwrap();
    assert_eq!(warning["location"]["path"], "tests/unregistered.rs");
}

/// @vtest.id TEST-CLI-050
/// @vtest.covers VO-CLI-013
/// @vtest.target crates/vtest-cli/src/lib.rs::run
/// @vtest.intent re-init of existing project is a JSON usage error E-OP-001, exit 2
#[test]
fn m1_repeated_init_is_a_json_usage_error() {
    let project = TempProject::new("usage-error");
    project.initialize("usage-error");

    let output = invoke(&project.root, "init", &[]);
    assert_success(
        &output,
        2,
        "initializing an existing project is a usage error",
    );
    let json = stdout(&output);
    assert_json_envelope(&json);
    assert!(json.contains("\"ok\": false"));
    assert!(json.contains("\"E-OP-001\""));
}

/// @vtest.id TEST-CLI-051
/// @vtest.covers VO-CLI-013
/// @vtest.target crates/vtest-cli/src/lib.rs::run
/// @vtest.intent malformed canonical config yields E-CONFIG-001 config error, exit 2
#[test]
fn m1_invalid_project_config_is_a_json_internal_error() {
    let project = TempProject::new("internal-error");
    project.copy_fixture("m1/base");
    fs::write(
        project.root.join(".verify/config.yaml"),
        "version: [not-a-number]\n",
    )
    .expect("replace config with invalid canonical data");

    let output = invoke(&project.root, "scan", &[]);
    // W1/W2 config validation rejects a malformed canonical config with
    // E-CONFIG-001 (an operation/usage error, exit 2) rather than the
    // pre-validation internal error this case originally expected.
    assert_success(
        &output,
        2,
        "a malformed canonical project configuration is a config error",
    );
    let json = stdout(&output);
    assert_json_envelope(&json);
    assert!(json.contains("\"ok\": false"));
    assert!(json.contains("\"E-CONFIG-001\""));
}

/// @vtest.id TEST-CLI-052
/// @vtest.covers VO-CLI-013
/// @vtest.target crates/vtest-cli/src/lib.rs::run
/// @vtest.intent scan reports E-SCAN-002..010 matrix with correct file locations, exit 1
#[test]
fn m1_error_diagnostic_matrix_is_reported_by_the_cli() {
    let project = TempProject::new("diagnostics");
    project.copy_fixture("m1/base");
    project.copy_case("diagnostics.rs", "tests/diagnostics.rs");
    project.copy_case("REQ-ORPHAN.yaml", ".verify/req/REQ-ORPHAN.yaml");
    project.copy_case(
        "REL-BROKEN.yaml",
        ".verify/rel/01J8XVZK3Q0000000000000000.yaml",
    );
    project.copy_case("VO-FILE-NAME.yaml", ".verify/vo/VO-FILE-NAME.yaml");

    let output = invoke(&project.root, "scan", &[]);
    assert_success(&output, 1, "scanner diagnostics must fail the command");
    let json = stdout(&output);
    assert_json_envelope(&json);
    assert_scan_diagnostics_have_locations(&json);
    assert!(json.contains("\"ok\": false"));
    let envelope: serde_json::Value = serde_json::from_str(&json).unwrap();
    let diagnostics = envelope["diagnostics"].as_array().unwrap();
    for (code, file) in [
        ("E-SCAN-002", "tests/diagnostics.rs"),
        ("E-SCAN-003", "tests/diagnostics.rs"),
        ("E-SCAN-004", "tests/diagnostics.rs"),
        ("E-SCAN-005", "tests/diagnostics.rs"),
        ("E-SCAN-006", "tests/diagnostics.rs"),
        ("E-SCAN-007", "tests/diagnostics.rs"),
        ("E-SCAN-008", ".verify/req/REQ-ORPHAN.yaml"),
        ("E-SCAN-009", ".verify/rel/01J8XVZK3Q0000000000000000.yaml"),
        ("E-SCAN-010", ".verify/vo/VO-FILE-NAME.yaml"),
    ] {
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic["code"] == code)
            .unwrap_or_else(|| panic!("missing {code}: {json}"));
        assert_eq!(
            diagnostic["location"]["path"], file,
            "wrong {code} location"
        );
    }
}

/// @vtest.id TEST-CLI-098
/// @vtest.covers VO-CLI-013
/// @vtest.target crates/vtest-adapter-rust/src/discovery.rs::parse_annotations
/// @vtest.intent a declaration with an unknown key and a duplicate key reports both defects, and repeated same-kind defects are not collapsed to one
#[test]
fn m1_declaration_with_mixed_annotation_defects_reports_every_defect() {
    let project = TempProject::new("diagnostics-multi");
    project.copy_fixture("m1/base");
    project.copy_case("diagnostics_multi.rs", "tests/diagnostics_multi.rs");

    let output = invoke(&project.root, "scan", &[]);
    assert_success(&output, 1, "scanner diagnostics must fail the command");
    let json = stdout(&output);
    assert_json_envelope(&json);
    let envelope: serde_json::Value = serde_json::from_str(&json).unwrap();
    let diagnostics = envelope["diagnostics"].as_array().unwrap();

    // Two distinct unknown keys (typo-one, typo-two) must both surface as
    // E-SCAN-006, and the duplicate `id` key must independently surface as
    // E-SCAN-005 in the same declaration -- none of the three may be
    // swallowed by the others.
    let unknown_key_count = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic["code"] == "E-SCAN-006"
                && diagnostic["location"]["path"] == "tests/diagnostics_multi.rs"
        })
        .count();
    assert_eq!(
        unknown_key_count, 2,
        "expected both unknown keys reported, got: {json}"
    );
    let duplicate_key_count = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic["code"] == "E-SCAN-005"
                && diagnostic["location"]["path"] == "tests/diagnostics_multi.rs"
        })
        .count();
    assert_eq!(
        duplicate_key_count, 1,
        "expected the duplicate `id` key reported alongside the unknown keys, got: {json}"
    );
}
