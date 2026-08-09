//! Alpha.2 acceptance freeze.
//!
//! These tests are derived from Annex C after the W0 specification merge.  The
//! file is intentionally added before production implementation so that the
//! old implementation produces reproducible RED results instead of silently
//! becoming the acceptance oracle.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::Value;

static NEXT_PROJECT: AtomicU64 = AtomicU64::new(0);

const FIXED_SCOPE: [&str; 12] = [
    "spec_coverage",
    "vo_decomposition",
    "vo_coverage",
    "test_existence",
    "static_audit",
    "semantic_audit",
    "impl_consistency",
    "test_execution",
    "runtime_result",
    "target_execution",
    "evidence_validity",
    "test_traceability",
];

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
            "vtest-cli-alpha2-{name}-{}-{nanos}-{sequence}",
            std::process::id()
        ));
        copy_tree(&fixture_path("m1/base"), &root);
        Self { root }
    }

    fn empty(name: &str) -> Self {
        let sequence = NEXT_PROJECT.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "vtest-cli-alpha2-{name}-{}-{nanos}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create temporary project root");
        Self { root }
    }

    fn write_v2_config(&self, adapter_id: &str, scope: &[&str]) {
        let items = scope
            .iter()
            .map(|item| format!("    - {item}"))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(
            self.root.join(".verify/config.yaml"),
            format!(
                "version: 2\nproject:\n  name: alpha2\nadapters:\n  - id: {adapter_id}\n    roots: [.]\n    scan:\n      include: [src, tests]\n      assertion_macros: []\n    run:\n      coverage: off\nverify:\n  full_scope:\n{items}\n"
            ),
        )
        .expect("write version 2 configuration");
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
    for entry in fs::read_dir(source).expect("read fixture directory") {
        let entry = entry.expect("read fixture entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type().expect("read fixture entry type").is_dir() {
            copy_tree(&source_path, &destination_path);
        } else {
            fs::copy(source_path, destination_path).expect("copy fixture file");
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

fn invoke_text(project: &Path, command: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vtest"))
        .arg("--project")
        .arg(project)
        .args(["--format", "text", command])
        .args(args)
        .output()
        .expect("run vtest process")
}

fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "vtest did not emit JSON: {error}; stdout={}; stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn report_items(value: &Value) -> &[Value] {
    value["data"]["report"]["items"]
        .as_array()
        .expect("verification report items array")
}

fn diagnostic_code(value: &Value, code: &str) -> bool {
    value["diagnostics"]
        .as_array()
        .is_some_and(|diagnostics| diagnostics.iter().any(|item| item["code"] == code))
}

#[test]
fn acceptance_init_writes_version_two_and_exactly_twelve_items() {
    let project = TempProject::empty("init");
    let output = invoke(&project.root, "init", &["--name", "alpha2-init"]);
    assert_eq!(output.status.code(), Some(0));
    let config =
        fs::read_to_string(project.root.join(".verify/config.yaml")).expect("init writes config");
    assert!(config.contains("version: 2"));
    assert!(config.contains("adapters:"));
    assert!(config.contains("- id: rust-cargo"));
    assert_eq!(
        config
            .lines()
            .filter_map(|line| line.trim().strip_prefix("- "))
            .filter(|item| FIXED_SCOPE.contains(item))
            .count(),
        FIXED_SCOPE.len()
    );
}

#[test]
fn acceptance_v1_eleven_item_scope_is_completed_in_memory() {
    let project = TempProject::from_m1_base("v1-scope");
    let output = invoke(&project.root, "verify", &[]);
    let value = json(&output);
    let names = report_items(&value)
        .iter()
        .filter_map(|item| item["item"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(names.len(), FIXED_SCOPE.len(), "report: {value}");
    assert!(names.contains(&"test_traceability"), "report: {value}");
    assert_eq!(
        fs::read_to_string(project.root.join(".verify/config.yaml"))
            .expect("read v1 config")
            .lines()
            .find(|line| line.starts_with("version:")),
        Some("version: 1")
    );
}

#[test]
fn acceptance_v2_scope_cannot_be_used_to_remove_a_check() {
    let project = TempProject::from_m1_base("v2-scope-reject");
    let scope = &FIXED_SCOPE[..FIXED_SCOPE.len() - 1];
    project.write_v2_config("rust-cargo", scope);
    let output = invoke(&project.root, "verify", &[]);
    assert_eq!(output.status.code(), Some(2));
    let value = json(&output);
    assert!(!value["ok"].as_bool().unwrap_or(true));
    assert!(diagnostic_code(&value, "E-CONFIG-001"), "response: {value}");
}

#[test]
fn acceptance_unknown_adapter_is_not_silently_treated_as_rust() {
    let project = TempProject::from_m1_base("unknown-adapter");
    project.write_v2_config("missing-adapter", &FIXED_SCOPE);
    let output = invoke(&project.root, "scan", &[]);
    assert_eq!(output.status.code(), Some(2));
    let value = json(&output);
    assert!(
        diagnostic_code(&value, "E-ADAPTER-001"),
        "response: {value}"
    );
}

#[test]
fn acceptance_scan_domain_output_has_execution_and_neutral_target_collection() {
    let project = TempProject::from_m1_base("neutral-scan");
    let output = invoke(&project.root, "scan", &[]);
    let value = json(&output);
    let test = value["data"]["tests"]
        .as_array()
        .and_then(|tests| tests.first())
        .expect("scan returns a Test entity");
    assert!(test.get("execution").is_some(), "scan result: {value}");
    assert!(test["targets"].is_array(), "scan result: {value}");
}

#[test]
fn acceptance_execution_evidence_contains_current_execution_state() {
    let project = TempProject::from_m1_base("execution-state");
    let output = invoke(&project.root, "run", &["--all", "--fast"]);
    let value = json(&output);
    let evidence = value["data"]["evidence"]
        .as_array()
        .and_then(|records| records.first())
        .expect("run returns Evidence");
    assert!(
        evidence.get("execution_state").is_some(),
        "execution state is required: {value}"
    );
    assert!(
        evidence["execution_state"]["hash"].is_string(),
        "execution state hash is required: {value}"
    );
}

#[test]
fn acceptance_static_audit_bundle_includes_analysis_dependency_subjects() {
    let project = TempProject::from_m1_base("audit-closure");
    let output = invoke(
        &project.root,
        "audit",
        &[
            "bundle",
            "--kind",
            "test-semantic",
            "--test",
            "TEST-M1-CLEAN",
            "--include-failed",
        ],
    );
    assert_eq!(output.status.code(), Some(0));
    let response = json(&output);
    let bundle_path = response["data"]["path"].as_str().expect("bundle path");
    let bundle = serde_json::from_str::<Value>(
        &fs::read_to_string(project.root.join(bundle_path)).expect("read bundle"),
    )
    .expect("valid bundle");
    assert!(
        bundle["subjects"]
            .as_array()
            .is_some_and(|subjects| subjects
                .iter()
                .any(|subject| subject["kind"] == "static_analysis_source")),
        "bundle lacks static analysis closure: {bundle}"
    );
}

#[test]
fn acceptance_spec_coverage_is_a_supported_fourth_bundle_kind() {
    let project = TempProject::from_m1_base("spec-coverage-bundle");
    let output = invoke(
        &project.root,
        "audit",
        &[
            "bundle",
            "--kind",
            "spec-coverage",
            "--req",
            "REQ-MISSING",
            "--include-failed",
        ],
    );
    assert_ne!(
        output.status.code(),
        Some(2),
        "unsupported bundle kind: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn acceptance_limited_scope_text_tree_does_not_repeat_ancestor_branch() {
    let project = TempProject::from_m1_base("tree");
    let output = invoke_text(&project.root, "report", &["--items", "test_existence"]);
    let text = String::from_utf8(output.stdout).expect("text report");
    assert!(!text.contains("├─ │"), "malformed tree prefix: {text}");
    assert!(!text.contains("├─ ├─"), "malformed tree prefix: {text}");
    assert!(
        text.contains("scope"),
        "limited scope must be visible: {text}"
    );
}

#[test]
fn acceptance_mcp_advertises_spec_coverage_bundle_and_neutral_scan_fields() {
    let project = TempProject::from_m1_base("mcp-contract");
    let input = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{}}\n";
    let output = Command::new(env!("CARGO_BIN_EXE_vtest"))
        .arg("--project")
        .arg(&project.root)
        .arg("mcp")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .take()
                .expect("MCP stdin")
                .write_all(input.as_bytes())?;
            child.wait_with_output()
        })
        .expect("run MCP process");
    let stdout = String::from_utf8(output.stdout).expect("MCP output");
    assert!(
        stdout.contains("spec-coverage"),
        "MCP bundle schema: {stdout}"
    );
}
