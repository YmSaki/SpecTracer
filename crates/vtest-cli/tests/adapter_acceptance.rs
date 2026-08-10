//! Acceptance freeze for the language-adapter separation contract.
//!
//! This file intentionally lands before production implementation. Tests
//! marked RED_REQUIRED in `tests/ACCEPTANCE.md` must fail on baseline 575ea72
//! and turn green only in their owning W1-W7 wave.

use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{json, Value};

static NEXT_PROJECT: AtomicU64 = AtomicU64::new(0);
const FIXED_ITEMS: [&str; 12] = [
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
        let project = Self::empty(name);
        copy_tree(&fixture_path("calc/m1/base"), &project.root);
        for directory in ["approvals", "audits", "evidence", "rel"] {
            fs::create_dir_all(project.root.join(".verify").join(directory))
                .expect("create canonical record directory");
        }
        project
    }

    fn empty(name: &str) -> Self {
        let sequence = NEXT_PROJECT.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "vtest-adapter-acceptance-{name}-{}-{nanos}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create temporary acceptance project");
        Self { root }
    }

    fn commit_baseline(&self) {
        run_git(&self.root, &["init", "-q"]);
        run_git(
            &self.root,
            &["config", "user.email", "adapter-acceptance@example.invalid"],
        );
        run_git(&self.root, &["config", "user.name", "Adapter Acceptance"]);
        run_git(&self.root, &["config", "commit.gpgsign", "false"]);
        run_git(&self.root, &["add", "."]);
        run_git(&self.root, &["commit", "-qm", "acceptance baseline"]);
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        if self.root.exists() {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture_path(relative: &str) -> PathBuf {
    repository_root().join("tests/fixtures").join(relative)
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create fixture destination");
    for entry in fs::read_dir(source).expect("read fixture source") {
        let entry = entry.expect("read fixture entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type().expect("read fixture type").is_dir() {
            copy_tree(&source_path, &destination_path);
        } else {
            fs::copy(source_path, destination_path).expect("copy fixture file");
        }
    }
}

fn run_git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?}: stdout={} stderr={}",
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
        .expect("run vtest")
}

fn envelope(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "invalid JSON envelope ({error}): stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
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

fn report_item<'a>(response: &'a Value, item: &str) -> &'a Value {
    response["data"]["report"]["items"]
        .as_array()
        .expect("report items array")
        .iter()
        .find(|entry| entry["item"] == item)
        .unwrap_or_else(|| panic!("missing report item {item}: {response}"))
}

fn diagnostic_codes(response: &Value) -> Vec<&str> {
    response["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|diagnostic| diagnostic["code"].as_str())
        .collect()
}

fn only_yaml(directory: &Path) -> PathBuf {
    let mut files = fs::read_dir(directory)
        .expect("read record directory")
        .map(|entry| entry.expect("read record entry").path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("yaml"))
        .collect::<Vec<_>>();
    files.sort();
    assert_eq!(files.len(), 1, "expected one YAML record in {directory:?}");
    files.remove(0)
}

fn bundle(project: &TempProject, kind: &str, selector: &[&str]) -> String {
    let mut args = vec!["bundle", "--kind", kind];
    args.extend_from_slice(selector);
    let output = invoke(&project.root, "audit", &args);
    assert_exit(&output, 0, "generate semantic bundle");
    envelope(&output)["data"]["bundle_id"]
        .as_str()
        .expect("bundle ID")
        .to_owned()
}

fn submit_audit(project: &TempProject, bundle_id: &str, kind: &str) -> Value {
    let verdict = if kind == "vo-coverage" {
        "COMPLETE"
    } else {
        "PASS"
    };
    submit_audit_verdict(project, bundle_id, kind, verdict)
}

fn submit_audit_verdict(
    project: &TempProject,
    bundle_id: &str,
    kind: &str,
    verdict: &str,
) -> Value {
    let submission_path = project.root.join(format!("{kind}-submission.json"));
    let reasons = if kind == "vo-coverage" {
        json!([{
            "kind": "decomposition-viewpoint",
            "claim": "the Specification-bound VO is covered by its Test",
            "basis": [{"kind": "spec", "ref": "SPEC-ADAPTER#contract"}]
        }])
    } else {
        json!([{
            "claim": "acceptance fixture subjects are consistent",
            "basis": [{"kind": "spec", "ref": "SPEC-ADAPTER#contract"}]
        }])
    };
    fs::write(
        &submission_path,
        serde_json::to_vec_pretty(&json!({
            "bundle_id": bundle_id,
            "kind": kind,
            "verdict": verdict,
            "reasons": reasons,
            "exclusions": [],
            "auditor": {"kind": "agent", "id": "adapter-acceptance", "model": "test"},
            "confidence": "high"
        }))
        .expect("serialize audit submission"),
    )
    .expect("write audit submission");
    let path = submission_path.to_string_lossy().into_owned();
    let output = invoke(&project.root, "audit", &["submit", "--file", &path]);
    assert_exit(&output, 0, "submit semantic Audit");
    envelope(&output)
}

fn prepare_spec_dependency(project: &TempProject) {
    fs::create_dir_all(project.root.join("docs")).expect("create specification directory");
    fs::write(
        project.root.join("docs/adapter-spec.md"),
        "# Adapter contract\n\nThe known operation remains consistent.\n",
    )
    .expect("write specification source");
    let add = invoke(
        &project.root,
        "spec",
        &[
            "add",
            "--id",
            "SPEC-ADAPTER",
            "--path",
            "docs/adapter-spec.md",
        ],
    );
    assert_exit(&add, 0, "register specification source");
    fs::write(
        project.root.join(".verify/req/REQ-ADAPTER.yaml"),
        "id: REQ-ADAPTER\nparent: null\nspec_refs:\n  - spec: SPEC-ADAPTER\n    section: contract\nsummary: adapter contract\nstatus: active\ncreated: '2026-01-01'\nupdated: '2026-01-01'\nversion: 1\n",
    )
    .expect("write Specification-bound REQ");
    let vo_path = project.root.join(".verify/vo/VO-KNOWN.yaml");
    let vo = fs::read_to_string(&vo_path).expect("read fixture VO");
    fs::write(
        vo_path,
        vo.replace("requirements: []", "requirements:\n  - REQ-ADAPTER")
            .replace(
                "spec_refs: []",
                "spec_refs:\n  - spec: SPEC-ADAPTER\n    section: contract",
            ),
    )
    .expect("bind VO to Specification closure");
}

fn mcp_call(project: &Path, name: &str, arguments: Value) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_vtest"))
        .arg("--project")
        .arg(project)
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn MCP server");
    let request = serde_json::to_vec(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {"name": name, "arguments": arguments}
    }))
    .expect("serialize MCP request");
    {
        let stdin = child.stdin.as_mut().expect("MCP stdin");
        stdin.write_all(&request).expect("write MCP request");
        stdin.write_all(b"\n").expect("write MCP newline");
    }
    let output = child.wait_with_output().expect("wait for MCP server");
    assert!(
        output.status.success(),
        "MCP server failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let line = String::from_utf8(output.stdout).expect("MCP output is UTF-8");
    serde_json::from_str(line.trim()).expect("MCP response is JSON")
}

fn assert_traceability(project: &TempProject, expected: &str) {
    let output = invoke(&project.root, "verify", &["--items", "test_traceability"]);
    assert_exit(&output, 1, "traceability violation is verification NG");
    let response = envelope(&output);
    assert_eq!(
        report_item(&response, "test_traceability")["value"],
        expected
    );
}

#[test]
fn adapter_boundary_fixture_is_non_rust_and_non_adjacent() {
    let root = fixture_path("adapters/synthetic");
    let manifest: Value = serde_json::from_slice(
        &fs::read(root.join("manifest.json")).expect("read synthetic manifest"),
    )
    .expect("parse synthetic manifest");
    let metadata: Value = serde_json::from_slice(
        &fs::read(root.join("metadata/tests.json")).expect("read synthetic metadata"),
    )
    .expect("parse synthetic metadata");
    let source_bytes =
        fs::read(root.join("source/cases.synth")).expect("read non-Rust synthetic source bytes");
    let source = String::from_utf8(source_bytes.clone()).expect("synthetic source is UTF-8");
    let collisions: Value = serde_json::from_slice(
        &fs::read(fixture_path("adapters/mixed/collisions.json"))
            .expect("read mixed-adapter collisions"),
    )
    .expect("parse mixed-adapter collisions");

    assert_eq!(manifest["adapter"], "synthetic");
    assert_eq!(manifest["capabilities"]["coverage"], false);
    assert_eq!(manifest["source_location"]["path"], "source/cases.synth");
    let start = manifest["source_location"]["range"]["start"]
        .as_u64()
        .expect("source range start") as usize;
    let end = manifest["source_location"]["range"]["end"]
        .as_u64()
        .expect("source range end") as usize;
    assert!(start < end, "synthetic source range must be non-empty");
    assert!(
        end <= source_bytes.len(),
        "synthetic source range must be current"
    );
    assert!(
        source_bytes[start..end].starts_with(b"scenario \"adding two values\""),
        "source range must identify the complete Test construct"
    );
    assert!(source.contains("scenario \"adding two values\""));
    assert!(!source.contains("#[test]"));
    assert!(!source.contains("/// @vtest"));
    assert_eq!(
        metadata["tests"][0]["targets"][0]["value"],
        "component(add)/scenario[happy]"
    );
    assert!(!metadata["tests"][0]
        .as_object()
        .unwrap()
        .contains_key("filter"));
    assert!(!metadata["tests"][0]
        .as_object()
        .unwrap()
        .contains_key("package"));
    assert!(!metadata["tests"][0]
        .as_object()
        .unwrap()
        .contains_key("test_target"));
    assert_eq!(collisions["test_id_collision"][0]["id"], "TEST-COLLISION");
    assert_eq!(collisions["test_id_collision"][1]["id"], "TEST-COLLISION");
    assert_eq!(collisions["src_id_collision"][0]["id"], "SRC-COLLISION");
    assert_eq!(collisions["src_id_collision"][1]["id"], "SRC-COLLISION");
}

#[test]
fn adapter_api_compile_contract_type_checks() {
    let manifest = fixture_path("adapters/api-contract/Cargo.toml");
    let output = Command::new("cargo")
        .args(["check", "--quiet", "--manifest-path"])
        .arg(&manifest)
        .output()
        .expect("type-check adapter API acceptance contract");
    assert!(
        output.status.success(),
        "adapter API compile contract failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn synthetic_variants_cover_capability_failure_mutation_and_ordering_inputs() {
    let root = fixture_path("adapters");
    let json_file = |relative: &str| -> Value {
        serde_json::from_slice(&fs::read(root.join(relative)).expect("read adapter fixture"))
            .expect("parse adapter fixture JSON")
    };
    let base = json_file("synthetic/metadata/tests.json");
    let changed = json_file("synthetic/metadata/tests.changed.json");
    assert_eq!(base["tests"][0]["id"], changed["tests"][0]["id"]);
    assert_ne!(
        base["tests"][0]["intent"], changed["tests"][0]["intent"],
        "metadata-only mutation must change a canonical logical value"
    );
    let no_runner = json_file("synthetic/manifest-no-runner.json");
    assert_eq!(no_runner["capabilities"]["runner"], false);
    assert_eq!(no_runner["capabilities"]["coverage"], false);
    let incomplete = json_file("synthetic/manifest-incomplete-analysis.json");
    assert_eq!(incomplete["static_analysis"]["complete"], false);
    let failed = json_file("synthetic/manifest-discovery-failure.json");
    assert_eq!(failed["complete"], false);
    assert_eq!(failed["diagnostics"][0]["code"], "E-SCAN-001");
    let targets = json_file("synthetic/target-observations.json");
    assert_eq!(targets["targets"][0]["result"], "PASS");
    assert_eq!(targets["targets"][1]["result"], "FAIL");
    assert_eq!(targets["targets"][2]["result"], "UNKNOWN");
    assert_eq!(targets["expected_aggregate"], "FAIL");
    let order_a = json_file("mixed/order-a.json");
    let order_b = json_file("mixed/order-b.json");
    assert_ne!(order_a["adapters"], order_b["adapters"]);
    assert_ne!(order_a["filesystem_entries"], order_b["filesystem_entries"]);
    assert_eq!(
        order_a["expected_test_order"],
        order_b["expected_test_order"]
    );
    let duplicate_form = json_file("forms/duplicate-kind.json");
    assert_eq!(duplicate_form["registrations"].as_array().unwrap().len(), 2);
    assert_eq!(duplicate_form["expected_write"], false);
    let ambiguous_form = json_file("forms/ambiguous-compatibility.json");
    assert_eq!(
        ambiguous_form["matching_adapters"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(ambiguous_form["rust_fallback_allowed"], false);
}

#[test]
fn duplicate_bare_and_prefixed_relation_payload_is_rejected() {
    let project = TempProject::from_m1_base("duplicate-relation-alias");
    let relation_dir = project.root.join(".verify/rel");
    fs::create_dir_all(&relation_dir).expect("create Relation directory");
    for name in [
        "01ARZ3NDEKTSV4RRFFQ69G5FAV.yaml",
        "REL-01ARZ3NDEKTSV4RRFFQ69G5FAV.yaml",
    ] {
        fs::copy(
            fixture_path(&format!("adapters/relations/{name}")),
            relation_dir.join(name),
        )
        .expect("copy Relation compatibility fixture");
    }
    let scan = invoke(&project.root, "scan", &[]);
    assert_exit(&scan, 1, "duplicate Relation aliases are scan errors");
    let response = envelope(&scan);
    assert!(diagnostic_codes(&response).contains(&"E-SCAN-010"));
}

fn cargo_metadata() -> Value {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(repository_root())
        .output()
        .expect("run cargo metadata");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("cargo metadata is JSON")
}

fn metadata_package<'a>(metadata: &'a Value, name: &str) -> &'a Value {
    metadata["packages"]
        .as_array()
        .expect("metadata packages")
        .iter()
        .find(|package| package["name"] == name)
        .unwrap_or_else(|| panic!("cargo metadata is missing package {name}"))
}

#[test]
fn adapter_api_crate_exposes_every_neutral_draft_type() {
    let root = repository_root();
    let metadata = cargo_metadata();
    let package = metadata_package(&metadata, "vtest-adapter-api");
    let dependencies = package["dependencies"]
        .as_array()
        .expect("adapter API dependencies")
        .iter()
        .filter_map(|dependency| dependency["name"].as_str())
        .collect::<Vec<_>>();
    for forbidden in ["syn", "quote", "rustc-demangle", "cargo_metadata"] {
        assert!(
            !dependencies.contains(&forbidden),
            "adapter API exposes forbidden dependency {forbidden}"
        );
    }
    let adapter_manifest = root.join("crates/vtest-adapter-api/Cargo.toml");
    assert!(
        adapter_manifest.is_file(),
        "vtest-adapter-api crate is required"
    );
    let adapter_api = fs::read_to_string(root.join("crates/vtest-adapter-api/src/lib.rs"))
        .expect("read adapter API source");
    for required in [
        "SourceFragment",
        "ManagedTestDraft",
        "DiscoveredTestDraft",
        "ManagedTestDraftLink",
        "SourceTargetDraft",
        "DiscoveryBatch",
        "TestWireCodec",
        "StaticAuditConfigDraft",
        "StaticAnalysisClosureDraft",
        "ExecutionStateDraft",
    ] {
        assert!(
            adapter_api.contains(required),
            "adapter API is missing {required}"
        );
    }
}

#[test]
fn core_model_is_neutral_and_has_the_fixed_check_set() {
    let root = repository_root();
    let model = fs::read_to_string(root.join("crates/vtest-model/src/lib.rs"))
        .expect("read neutral model source");
    for required in ["ExecutionDescriptor", "test_traceability", "SourceLocation"] {
        assert!(model.contains(required), "model is missing {required}");
    }
    for legacy in [
        "pub filter:",
        "pub package:",
        "pub test_target:",
        "pub enum TestTarget",
    ] {
        assert!(!model.contains(legacy), "model retains Rust field {legacy}");
    }
}

#[test]
fn orchestration_crates_have_no_direct_rust_analysis_dependencies() {
    let metadata = cargo_metadata();
    for crate_name in ["vtest-scan", "vtest-audit", "vtest-exec"] {
        let package = metadata_package(&metadata, crate_name);
        let dependencies = package["dependencies"]
            .as_array()
            .expect("package dependencies")
            .iter()
            .filter_map(|dependency| dependency["name"].as_str())
            .collect::<Vec<_>>();
        for forbidden in ["syn", "quote", "rustc-demangle"] {
            assert!(
                !dependencies.contains(&forbidden),
                "{crate_name} directly depends on {forbidden}"
            );
        }
    }
}

#[test]
fn default_verify_evaluates_the_fixed_twelve_items() {
    let project = TempProject::from_m1_base("fixed-twelve");
    let before = fs::read(project.root.join(".verify/config.yaml")).expect("read v1 config");
    let output = invoke(&project.root, "verify", &[]);
    assert_exit(&output, 1, "incomplete full verification is NG");
    let response = envelope(&output);
    let actual = response["data"]["report"]["items"]
        .as_array()
        .expect("report items")
        .iter()
        .map(|entry| entry["item"].as_str().expect("item name"))
        .collect::<Vec<_>>();
    assert_eq!(actual, FIXED_ITEMS);
    assert_eq!(
        fs::read(project.root.join(".verify/config.yaml")).expect("reread v1 config"),
        before,
        "v1 compatibility reads must not rewrite config"
    );
}

#[test]
fn version_one_without_full_scope_uses_fixed_twelve_without_rewrite() {
    let project = TempProject::from_m1_base("v1-no-full-scope");
    let config_path = project.root.join(".verify/config.yaml");
    fs::write(
        &config_path,
        "version: 1\nproject:\n  name: v1-no-scope\nscan:\n  include: [src, tests]\n  assertion_macros: []\nrun:\n  coverage: off\n",
    )
    .expect("write v1 config without full_scope");
    let before = fs::read(&config_path).expect("read compatibility config");
    let output = invoke(&project.root, "verify", &[]);
    assert_exit(
        &output,
        1,
        "v1 default full verification is NG for fixture gaps",
    );
    let response = envelope(&output);
    assert_eq!(
        response["data"]["report"]["items"]
            .as_array()
            .expect("report items")
            .len(),
        12
    );
    assert_eq!(
        report_item(&response, "test_traceability")["item"],
        "test_traceability"
    );
    assert_eq!(fs::read(&config_path).expect("reread v1 config"), before);
}

#[test]
fn unregistered_test_is_traceability_missing() {
    let project = TempProject::from_m1_base("unregistered");
    fs::write(
        project.root.join("tests/unregistered.rs"),
        "#[test]\nfn unmanaged_test_construct() {}\n",
    )
    .expect("write unmanaged Test");
    assert_traceability(&project, "MISSING");
}

#[test]
fn empty_covers_is_traceability_missing() {
    let project = TempProject::from_m1_base("empty-covers");
    let path = project.root.join("tests/registered.rs");
    let source = fs::read_to_string(&path).expect("read registered Test");
    fs::write(&path, source.replace("/// @vtest.covers VO-KNOWN\n", ""))
        .expect("remove covers metadata");
    assert_traceability(&project, "MISSING");
}

#[test]
fn dangling_vo_is_traceability_mismatch() {
    let project = TempProject::from_m1_base("dangling-vo");
    let path = project.root.join("tests/registered.rs");
    let source = fs::read_to_string(&path).expect("read registered Test");
    fs::write(&path, source.replace("VO-KNOWN", "VO-DOES-NOT-EXIST"))
        .expect("write dangling VO reference");
    assert_traceability(&project, "MISMATCH");
}

#[test]
fn duplicate_test_id_is_traceability_mismatch() {
    let project = TempProject::from_m1_base("duplicate-test-id");
    fs::write(
        project.root.join("tests/duplicate.rs"),
        "/// @vtest.id TEST-M1-CLEAN\n/// @vtest.covers VO-KNOWN\n/// @vtest.target src/lib.rs::known\n/// @vtest.intent duplicate ID\n#[test]\nfn duplicate_id() {}\n",
    )
    .expect("write duplicate Test ID");
    assert_traceability(&project, "MISMATCH");
}

#[test]
fn version_two_config_is_accepted_and_incomplete_scope_is_rejected() {
    let project = TempProject::from_m1_base("config-v2");
    fs::copy(
        fixture_path("adapters/config/v2-rust-cargo.yaml"),
        project.root.join(".verify/config.yaml"),
    )
    .expect("install v2 config");
    let valid = invoke(&project.root, "doctor", &[]);
    assert_exit(&valid, 0, "normative v2 rust-cargo config");

    fs::copy(
        fixture_path("adapters/config/v2-incomplete-scope.yaml"),
        project.root.join(".verify/config.yaml"),
    )
    .expect("install incomplete v2 config");
    let invalid = invoke(&project.root, "verify", &[]);
    assert_exit(&invalid, 2, "incomplete v2 scope is a usage error");
    assert!(
        diagnostic_codes(&envelope(&invalid)).contains(&"E-CONFIG-001"),
        "incomplete scope must report E-CONFIG-001"
    );
}

#[test]
fn version_two_duplicate_full_scope_is_e_config_001() {
    let project = TempProject::from_m1_base("v2-duplicate-scope");
    let config_path = project.root.join(".verify/config.yaml");
    let mut config = fs::read_to_string(fixture_path("adapters/config/v2-rust-cargo.yaml"))
        .expect("read normative v2 fixture");
    config = config.replace(
        "evidence_validity, test_traceability]",
        "evidence_validity, test_traceability, test_traceability]",
    );
    fs::write(&config_path, config).expect("write duplicate v2 scope");
    let output = invoke(&project.root, "verify", &[]);
    assert_exit(
        &output,
        2,
        "duplicate v2 scope is rejected before verification",
    );
    let response = envelope(&output);
    assert!(diagnostic_codes(&response).contains(&"E-CONFIG-001"));
    assert!(
        response["data"].is_null(),
        "no verification result on invalid config"
    );
}

fn adapter_config(project: &TempProject, adapters: &str) {
    fs::write(
        project.root.join(".verify/config.yaml"),
        format!(
            "version: 2\nproject:\n  name: adapter-registry\nadapters:\n{adapters}verify:\n  full_scope: [spec_coverage, vo_decomposition, vo_coverage, test_existence, static_audit, semantic_audit, impl_consistency, test_execution, runtime_result, target_execution, evidence_validity, test_traceability]\n"
        ),
    )
    .expect("write adapter registry config");
}

fn assert_adapter_usage_error(project: &TempProject, command: &str, expected_code: &str) {
    let output = invoke(&project.root, command, &[]);
    assert_exit(&output, 2, "invalid adapter registry is a usage error");
    let response = envelope(&output);
    assert!(
        diagnostic_codes(&response).contains(&expected_code),
        "{response}"
    );
    assert!(
        response["data"].is_null(),
        "rejected adapter operation has no result"
    );
}

#[test]
fn unknown_adapter_rejects_scan_without_result() {
    let project = TempProject::from_m1_base("unknown-adapter");
    adapter_config(
        &project,
        "  - id: not-registered\n    roots: [\".\"]\n    scan:\n      include: [src, tests]\n    run:\n      coverage: off\n",
    );
    assert_adapter_usage_error(&project, "scan", "E-ADAPTER-001");
}

#[test]
fn duplicate_adapter_id_rejects_scan_without_result() {
    let project = TempProject::from_m1_base("duplicate-adapter");
    adapter_config(
        &project,
        "  - id: rust-cargo\n    roots: [\".\"]\n    scan:\n      include: [src, tests]\n      assertion_macros: []\n    run:\n      coverage: off\n  - id: rust-cargo\n    roots: [tests]\n    scan:\n      include: [tests]\n      assertion_macros: []\n    run:\n      coverage: off\n",
    );
    assert_adapter_usage_error(&project, "scan", "E-ADAPTER-001");
}

#[test]
fn zero_adapter_registry_is_fail_closed() {
    let project = TempProject::from_m1_base("zero-adapter");
    fs::write(
        project.root.join(".verify/config.yaml"),
        "version: 2\nproject:\n  name: zero-adapter\nadapters: []\nverify:\n  full_scope: [spec_coverage, vo_decomposition, vo_coverage, test_existence, static_audit, semantic_audit, impl_consistency, test_execution, runtime_result, target_execution, evidence_validity, test_traceability]\n",
    )
    .expect("write empty adapter registry");
    assert_adapter_usage_error(&project, "scan", "E-ADAPTER-001");
}

#[test]
fn init_writes_v2_adapter_namespace_and_form_owner() {
    let project = TempProject::empty("init-v2");
    let init = invoke(&project.root, "init", &["--name", "adapter-init"]);
    assert_exit(&init, 0, "initialize adapter-aware repository");
    let config = fs::read_to_string(project.root.join(".verify/config.yaml"))
        .expect("read initialized config");
    assert!(config.contains("version: 2"));
    assert!(config.contains("adapters:"));
    assert!(config.contains("id: rust-cargo"));
    for kind in ["rust-unit-function", "rust-integration"] {
        let form = fs::read_to_string(project.root.join(format!(".verify/forms/{kind}.yaml")))
            .expect("read initialized form");
        assert!(
            form.contains("adapter: rust-cargo"),
            "form {kind} has no owner"
        );
    }
}

#[test]
fn static_audit_ignores_run_only_config_changes() {
    let project = TempProject::from_m1_base("static-config-projection");
    fs::write(
        project.root.join("tests/registered.rs"),
        "fn known() {}\n\n/// @vtest.id TEST-M1-CLEAN\n/// @vtest.covers VO-KNOWN\n/// @vtest.target tests/registered.rs::known\n/// @vtest.intent verifies the known target\n#[test]\nfn clean_scan_baseline() { assert_eq!(known(), ()); }\n",
    )
    .expect("write deterministic static-audit Test");
    project.commit_baseline();
    let audit = invoke(&project.root, "audit", &["static", "--all"]);
    assert_exit(&audit, 0, "create current static Audit");

    let config_path = project.root.join(".verify/config.yaml");
    let config = fs::read_to_string(&config_path).expect("read config");
    fs::write(
        &config_path,
        config.replace("coverage: off", "coverage: llvm-cov"),
    )
    .expect("change run-only config");
    let verify = invoke(&project.root, "verify", &["--items", "static_audit"]);
    assert_exit(&verify, 0, "run-only config does not stale static Audit");
    assert_eq!(
        report_item(&envelope(&verify), "static_audit")["value"],
        "PASS"
    );
}

#[test]
fn static_helper_only_change_stales_the_audit_record() {
    let project = TempProject::from_m1_base("static-helper-stale");
    fs::write(
        project.root.join("tests/registered.rs"),
        "fn known() {}\nfn consulted_helper() { known(); }\n\n/// @vtest.id TEST-M1-CLEAN\n/// @vtest.covers VO-KNOWN\n/// @vtest.target tests/registered.rs::known\n/// @vtest.intent reaches the target through a consulted helper\n#[test]\nfn clean_scan_baseline() { consulted_helper(); assert_eq!(1 + 1, 2); }\n",
    )
    .expect("write helper-bound static Test");
    project.commit_baseline();
    let audit = invoke(&project.root, "audit", &["static", "--all"]);
    assert!(
        matches!(audit.status.code(), Some(0 | 1)),
        "static audit must complete: {}",
        String::from_utf8_lossy(&audit.stdout)
    );
    assert!(
        envelope(&audit)["data"]["audits"]
            .as_array()
            .is_some_and(|audits| !audits.is_empty()),
        "static audit must persist a verdict"
    );
    let source_path = project.root.join("tests/registered.rs");
    let source = fs::read_to_string(&source_path).expect("read helper-bound Test");
    fs::write(
        &source_path,
        source.replace(
            "fn consulted_helper() { known(); }",
            "fn consulted_helper() { let changed = true; if changed { known(); } }",
        ),
    )
    .expect("change only consulted helper");
    let verify = invoke(&project.root, "verify", &["--items", "static_audit"]);
    assert_exit(
        &verify,
        1,
        "consulted helper change invalidates static Audit",
    );
    assert_eq!(
        report_item(&envelope(&verify), "static_audit")["value"],
        "STALE"
    );
}

#[test]
fn assertion_macro_change_stales_the_static_audit_record() {
    let project = TempProject::from_m1_base("assertion-macro-stale");
    fs::write(
        project.root.join("tests/registered.rs"),
        "fn known() {}\n\n/// @vtest.id TEST-M1-CLEAN\n/// @vtest.covers VO-KNOWN\n/// @vtest.target tests/registered.rs::known\n/// @vtest.intent verifies assertion config freshness\n#[test]\nfn clean_scan_baseline() { assert_eq!(known(), ()); }\n",
    )
    .expect("write static Test");
    project.commit_baseline();
    let audit = invoke(&project.root, "audit", &["static", "--all"]);
    assert_exit(&audit, 0, "record current static Audit");
    let config_path = project.root.join(".verify/config.yaml");
    let config = fs::read_to_string(&config_path).expect("read config");
    fs::write(
        &config_path,
        config.replace(
            "assertion_macros: []",
            "assertion_macros:\n    - check_known",
        ),
    )
    .expect("change assertion macro projection");
    let verify = invoke(&project.root, "verify", &["--items", "static_audit"]);
    assert_exit(
        &verify,
        1,
        "assertion macro change invalidates static Audit",
    );
    assert_eq!(
        report_item(&envelope(&verify), "static_audit")["value"],
        "STALE"
    );
}

#[test]
fn evidence_contains_neutral_subjects_and_complete_execution_state() {
    let project = TempProject::from_m1_base("execution-state");
    project.commit_baseline();
    let run = invoke(&project.root, "run", &["--all", "--fast"]);
    assert_exit(&run, 0, "record Evidence");
    let response = envelope(&run);
    let evidence = &response["data"]["evidence"][0];
    assert_eq!(evidence["adapter"], "rust-cargo");
    assert!(evidence["hashes"]["test_subject"].is_string());
    assert!(evidence["hashes"]["targets"].is_array());
    assert!(evidence["execution_state"]["subject"].is_string());
    assert_eq!(evidence["execution_state"]["complete"], true);
    assert!(evidence["execution_state"]["revision"]["commit"].is_string());
    assert!(evidence["execution_state"]["repository_inputs"].is_array());
}

#[test]
fn target_external_helper_change_stales_evidence() {
    let project = TempProject::from_m1_base("external-helper-stale");
    fs::write(
        project.root.join("src/lib.rs"),
        "mod helper;\npub fn known() { helper::runtime_input(); }\n",
    )
    .expect("write unchanged target construct");
    fs::write(
        project.root.join("src/helper.rs"),
        "pub fn runtime_input() { assert_eq!(2 + 2, 4); }\n",
    )
    .expect("write helper input");
    project.commit_baseline();
    let run = invoke(&project.root, "run", &["--all", "--fast"]);
    assert_exit(&run, 0, "record Evidence before helper change");
    fs::write(
        project.root.join("src/helper.rs"),
        "pub fn runtime_input() { assert_eq!(2 + 3, 5); }\n",
    )
    .expect("change only target-external helper");

    let verify = invoke(&project.root, "verify", &["--items", "evidence_validity"]);
    assert_exit(&verify, 1, "helper-only change invalidates Evidence");
    assert_eq!(
        report_item(&envelope(&verify), "evidence_validity")["value"],
        "STALE"
    );
}

#[test]
fn execution_state_mutation_reports_e_exec_004_without_evidence() {
    let project = TempProject::from_m1_base("execution-mutation");
    fs::write(project.root.join("src/runtime-input.txt"), "before\n")
        .expect("write execution input");
    fs::write(
        project.root.join("tests/registered.rs"),
        "/// @vtest.id TEST-M1-CLEAN\n/// @vtest.covers VO-KNOWN\n/// @vtest.target src/lib.rs::known\n/// @vtest.intent mutates an execution input during the run\n#[test]\nfn clean_scan_baseline() {\n    let path = concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/src/runtime-input.txt\");\n    std::fs::write(path, \"after\\n\").unwrap();\n}\n",
    )
    .expect("write mutating Test");
    project.commit_baseline();
    let run = invoke(&project.root, "run", &["--all", "--fast"]);
    assert_exit(&run, 1, "execution-state mutation rejects Evidence");
    let response = envelope(&run);
    assert!(diagnostic_codes(&response).contains(&"E-EXEC-004"));
    assert!(response["data"]["evidence"]
        .as_array()
        .is_some_and(Vec::is_empty));
}

#[test]
fn multi_target_evidence_keeps_target_specific_results() {
    let project = TempProject::from_m1_base("multi-target-neutral");
    fs::write(
        project.root.join("src/lib.rs"),
        "pub fn known() {}\npub fn also_known() {}\n",
    )
    .expect("write two target constructs");
    let test_path = project.root.join("tests/registered.rs");
    let test = fs::read_to_string(&test_path).expect("read registered Test");
    fs::write(
        &test_path,
        test.replace(
            "/// @vtest.target src/lib.rs::known",
            "/// @vtest.target src/lib.rs::known\n/// @vtest.target src/lib.rs::also_known",
        )
        .replace(
            "/// @vtest.intent provides a clean M1 scan baseline",
            "/// @vtest.intent provides a multi-target adapter fixture\n/// @vtest.kind integration-normal",
        ),
    )
    .expect("declare two targets");
    project.commit_baseline();
    let run = invoke(&project.root, "run", &["--all", "--fast"]);
    assert_exit(&run, 0, "record multi-target Evidence");
    let response = envelope(&run);
    let evidence = &response["data"]["evidence"][0];
    let targets = evidence["hashes"]["targets"]
        .as_array()
        .expect("neutral target hash entries");
    assert_eq!(targets.len(), 2);
    assert!(targets
        .iter()
        .all(|target| target["target_construct"].is_string()));
    let results = evidence["target_execution"]["targets"]
        .as_array()
        .expect("target-specific execution entries");
    assert_eq!(results.len(), 2);
}

#[test]
fn evidence_without_execution_state_is_compatibility_stale() {
    let project = TempProject::from_m1_base("missing-execution-state");
    project.commit_baseline();
    let run = invoke(&project.root, "run", &["--all", "--fast"]);
    assert_exit(&run, 0, "record current Evidence");
    let evidence_path = only_yaml(&project.root.join(".verify/evidence"));
    let evidence = fs::read_to_string(&evidence_path).expect("read Evidence YAML");
    let mut stripped = String::new();
    let mut skipping = false;
    for line in evidence.lines() {
        if line == "execution_state:" {
            skipping = true;
            continue;
        }
        if skipping && !line.starts_with(' ') && !line.is_empty() {
            skipping = false;
        }
        if !skipping {
            stripped.push_str(line);
            stripped.push('\n');
        }
    }
    fs::write(&evidence_path, stripped).expect("remove Execution State compatibility field");
    let verify = invoke(&project.root, "verify", &["--items", "evidence_validity"]);
    assert_exit(&verify, 1, "missing Execution State is stale");
    assert_eq!(
        report_item(&envelope(&verify), "evidence_validity")["value"],
        "STALE"
    );
}

#[test]
fn incomplete_current_execution_snapshot_is_unknown() {
    let project = TempProject::from_m1_base("incomplete-execution-snapshot");
    project.commit_baseline();
    let run = invoke(&project.root, "run", &["--all", "--fast"]);
    assert_exit(
        &run,
        0,
        "record Evidence before snapshot becomes incomplete",
    );
    let evidence_path = only_yaml(&project.root.join(".verify/evidence"));
    let evidence = fs::read_to_string(&evidence_path).expect("read Evidence YAML");
    let incomplete = if evidence.contains("execution_state:") {
        evidence.replacen("complete: true", "complete: false", 1)
    } else {
        format!(
            "{evidence}execution_state:\n  subject: sha256:0000000000000000000000000000000000000000000000000000000000000000\n  complete: false\n"
        )
    };
    fs::write(&evidence_path, incomplete).expect("make current snapshot incomplete");
    let verify = invoke(&project.root, "verify", &["--items", "evidence_validity"]);
    assert_exit(&verify, 1, "incomplete execution snapshot is non-PASS");
    assert_eq!(
        report_item(&envelope(&verify), "evidence_validity")["value"],
        "UNKNOWN"
    );
}

#[test]
fn evidence_without_revision_commit_is_stale() {
    let project = TempProject::from_m1_base("missing-revision");
    project.commit_baseline();
    let run = invoke(&project.root, "run", &["--all", "--fast"]);
    assert_exit(&run, 0, "record current Evidence");
    let evidence_path = only_yaml(&project.root.join(".verify/evidence"));
    let evidence = fs::read_to_string(&evidence_path).expect("read Evidence YAML");
    let stripped = evidence
        .lines()
        .filter(|line| {
            line.trim_start() != "commit: null" && !line.trim_start().starts_with("commit: ")
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&evidence_path, format!("{stripped}\n")).expect("remove Evidence commit");
    let verify = invoke(&project.root, "verify", &["--items", "evidence_validity"]);
    assert_exit(&verify, 1, "unknown Evidence revision is stale");
    assert_eq!(
        report_item(&envelope(&verify), "evidence_validity")["value"],
        "STALE"
    );
}

#[test]
fn head_change_without_test_or_target_change_stales_evidence() {
    let project = TempProject::from_m1_base("head-stale");
    project.commit_baseline();
    let run = invoke(&project.root, "run", &["--all", "--fast"]);
    assert_exit(&run, 0, "record Evidence at baseline HEAD");
    fs::write(project.root.join("HEAD-CHANGE.md"), "new revision\n")
        .expect("write unrelated revision input");
    run_git(&project.root, &["add", "HEAD-CHANGE.md"]);
    run_git(&project.root, &["commit", "-qm", "advance HEAD"]);
    let verify = invoke(&project.root, "verify", &["--items", "evidence_validity"]);
    assert_exit(&verify, 1, "HEAD mismatch invalidates Evidence");
    assert_eq!(
        report_item(&envelope(&verify), "evidence_validity")["value"],
        "STALE"
    );
}

#[test]
fn local_dependency_change_stales_evidence() {
    let project = TempProject::from_m1_base("local-dependency-stale");
    fs::create_dir_all(project.root.join("local-dep/src")).expect("create local dependency");
    fs::write(
        project.root.join("local-dep/Cargo.toml"),
        "[package]\nname = \"local-dep\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write local dependency manifest");
    fs::write(
        project.root.join("local-dep/src/lib.rs"),
        "pub fn touch() { assert_eq!(2 + 2, 4); }\n",
    )
    .expect("write local dependency input");
    let cargo_path = project.root.join("Cargo.toml");
    let cargo = fs::read_to_string(&cargo_path).expect("read fixture Cargo manifest");
    fs::write(
        &cargo_path,
        format!("{cargo}\n[dependencies]\nlocal-dep = {{ path = \"local-dep\" }}\n"),
    )
    .expect("link local dependency");
    fs::write(
        project.root.join("src/lib.rs"),
        "pub fn known() { local_dep::touch(); }\n",
    )
    .expect("make target depend on local package");
    let lock = Command::new("cargo")
        .arg("generate-lockfile")
        .current_dir(&project.root)
        .output()
        .expect("generate fixture lockfile");
    assert!(
        lock.status.success(),
        "generate lockfile: {}",
        String::from_utf8_lossy(&lock.stderr)
    );
    project.commit_baseline();
    let run = invoke(&project.root, "run", &["--all", "--fast"]);
    assert_exit(&run, 0, "record Evidence with local dependency");
    fs::write(
        project.root.join("local-dep/src/lib.rs"),
        "pub fn touch() { assert_eq!(2 + 3, 5); }\n",
    )
    .expect("change only local dependency input");
    let verify = invoke(&project.root, "verify", &["--items", "evidence_validity"]);
    assert_exit(&verify, 1, "local dependency change invalidates Evidence");
    assert_eq!(
        report_item(&envelope(&verify), "evidence_validity")["value"],
        "STALE"
    );
}

#[test]
fn specification_only_change_stales_impl_consistency() {
    let project = TempProject::from_m1_base("impl-spec-stale");
    prepare_spec_dependency(&project);
    project.commit_baseline();
    let bundle_id = bundle(&project, "impl-consistency", &["--test", "TEST-M1-CLEAN"]);
    submit_audit(&project, &bundle_id, "impl-consistency");
    fs::write(
        project.root.join("docs/adapter-spec.md"),
        "# Adapter contract\n\nThe Specification alone changed.\n",
    )
    .expect("change only Specification source");
    let verify = invoke(&project.root, "verify", &["--items", "impl_consistency"]);
    assert_exit(
        &verify,
        1,
        "Specification-only change stales impl-consistency",
    );
    assert_eq!(
        report_item(&envelope(&verify), "impl_consistency")["value"],
        "STALE"
    );
}

#[test]
fn impl_consistency_fail_maps_to_mismatch() {
    let project = TempProject::from_m1_base("impl-fail-mismatch");
    prepare_spec_dependency(&project);
    project.commit_baseline();
    let bundle_id = bundle(&project, "impl-consistency", &["--test", "TEST-M1-CLEAN"]);
    submit_audit_verdict(&project, &bundle_id, "impl-consistency", "FAIL");
    let verify = invoke(&project.root, "verify", &["--items", "impl_consistency"]);
    assert_exit(&verify, 1, "impl-consistency FAIL is verification NG");
    assert_eq!(
        report_item(&envelope(&verify), "impl_consistency")["value"],
        "MISMATCH"
    );
}

#[test]
fn specification_requirement_without_active_req_is_non_pass() {
    let project = TempProject::from_m1_base("spec-without-req");
    fs::create_dir_all(project.root.join("docs")).expect("create docs directory");
    fs::write(
        project.root.join("docs/unmapped-spec.md"),
        "# Required behavior\n\nA requirement exists without an active REQ.\n",
    )
    .expect("write Specification requirement");
    let add = invoke(
        &project.root,
        "spec",
        &[
            "add",
            "--id",
            "SPEC-UNMAPPED",
            "--path",
            "docs/unmapped-spec.md",
        ],
    );
    assert_exit(&add, 0, "register Specification without REQ");
    let verify = invoke(&project.root, "verify", &["--items", "spec_coverage"]);
    assert_exit(&verify, 1, "unmapped Specification requirement is non-PASS");
    let response = envelope(&verify);
    let value = report_item(&response, "spec_coverage")["value"]
        .as_str()
        .expect("spec_coverage value");
    assert_ne!(value, "PASS");
}

#[test]
fn specification_dependency_change_invalidates_vo_approval() {
    let project = TempProject::from_m1_base("approval-spec-stale");
    prepare_spec_dependency(&project);
    project.commit_baseline();
    let bundle_id = bundle(&project, "vo-coverage", &["--vo", "VO-KNOWN"]);
    let audit = submit_audit(&project, &bundle_id, "vo-coverage");
    let audit_id = audit["data"]["audit_id"].as_str().expect("Audit ID");
    let approve = invoke(
        &project.root,
        "vo",
        &[
            "approve",
            "VO-KNOWN",
            "--approver-kind",
            "human",
            "--approver-id",
            "adapter-owner",
            "--basis",
            audit_id,
        ],
    );
    assert_exit(&approve, 0, "approve dependency-bound VO");
    fs::write(
        project.root.join("docs/adapter-spec.md"),
        "# Adapter contract\n\nChanged after Approval.\n",
    )
    .expect("change upstream Specification");
    let show = invoke(&project.root, "vo", &["show", "VO-KNOWN"]);
    assert_exit(&show, 0, "show VO after dependency change");
    assert_eq!(envelope(&show)["data"]["effective_status"], "draft");
}

#[test]
fn cli_and_mcp_default_verify_share_the_fixed_contract() {
    let project = TempProject::from_m1_base("cli-mcp-parity");
    let cli_output = invoke(&project.root, "verify", &[]);
    assert_exit(&cli_output, 1, "CLI default verify");
    let cli = envelope(&cli_output);
    let mcp = mcp_call(&project.root, "verify", json!({}));
    let mcp_envelope = &mcp["result"]["structuredContent"];
    assert_eq!(mcp_envelope["data"], cli["data"]);
    assert_eq!(mcp_envelope["diagnostics"], cli["diagnostics"]);
    assert_eq!(
        cli["data"]["report"]["items"]
            .as_array()
            .expect("CLI report items")
            .len(),
        12
    );
    assert_eq!(
        report_item(&cli, "test_traceability")["item"],
        "test_traceability"
    );
}
