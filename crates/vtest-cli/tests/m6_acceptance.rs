//! M6 command-line acceptance coverage for aggregation, scope, and reports.

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
            "vtest-cli-m6-{name}-{}-{nanos}-{sequence}",
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
            ["config", "user.email", "m6-acceptance@example.invalid"],
            "configure temporary git email",
        );
        run_git(
            &self.root,
            ["config", "user.name", "M6 Acceptance"],
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
            ["commit", "-qm", "M6 acceptance baseline"],
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

fn invoke_text(project: &Path, command: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vtest"))
        .arg("--project")
        .arg(project)
        .args(["--format", "text", command])
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

fn report_item<'a>(response: &'a Value, item: &str) -> &'a Value {
    response["data"]["report"]["items"]
        .as_array()
        .expect("verification report items is an array")
        .iter()
        .find(|entry| entry["item"] == item)
        .unwrap_or_else(|| panic!("missing report item {item}: {response}"))
}

fn bundle(project: &TempProject, kind: &str, selector: &[&str]) -> String {
    let mut args = vec!["bundle", "--kind", kind];
    args.extend_from_slice(selector);
    let output = invoke(&project.root, "audit", &args);
    assert_exit(&output, 0, "generate M6 audit bundle");
    let value = envelope(&output);
    value["data"]["bundle_id"]
        .as_str()
        .expect("bundle response has bundle_id")
        .to_owned()
}

fn submit(project: &TempProject, bundle_id: &str, kind: &str, reasons: Value) -> Value {
    let file = project.root.join(format!("m6-{kind}.json"));
    let verdict = if kind == "vo-coverage" {
        "COMPLETE"
    } else {
        "PASS"
    };
    fs::write(
        &file,
        serde_json::to_vec_pretty(&serde_json::json!({
            "bundle_id": bundle_id,
            "kind": kind,
            "verdict": verdict,
            "reasons": reasons,
            "exclusions": [],
            "auditor": {"kind": "agent", "id": "m6-acceptance", "model": "acceptance"},
            "confidence": "high"
        }))
        .expect("serialize M6 audit submission"),
    )
    .expect("write M6 audit submission");
    let path = file.to_string_lossy().into_owned();
    let output = invoke(&project.root, "audit", &["submit", "--file", &path]);
    assert_exit(&output, 0, "submit M6 audit result");
    let value = envelope(&output);
    value["data"].clone()
}

#[test]
fn m6_entity_scope_selects_test_vo_and_report() {
    let project = TempProject::from_m1_base("entity-scope");
    fs::write(
        project.root.join(".verify/req/REQ-M6-SCOPE.yaml"),
        "id: REQ-M6-SCOPE\nparent: null\nspec_refs: []\nsummary: scope selection\nstatus: active\ncreated: '2026-01-01'\nupdated: '2026-01-01'\nversion: 1\n",
    )
    .expect("add REQ for entity scope");
    let vo_path = project.root.join(".verify/vo/VO-KNOWN.yaml");
    let vo = fs::read_to_string(&vo_path).expect("read scoped VO");
    fs::write(
        vo_path,
        vo.replace("requirements: []", "requirements:\n  - REQ-M6-SCOPE"),
    )
    .expect("link VO to scoped REQ");
    project.commit_baseline();
    let run = invoke(&project.root, "run", &["--all", "--fast"]);
    assert_exit(&run, 0, "create Evidence for scoped verification");

    let test = invoke(
        &project.root,
        "verify",
        &["--test", "TEST-M1-CLEAN", "--items", "evidence_validity"],
    );
    assert_exit(&test, 0, "verify a selected Test");
    let test_json = envelope(&test);
    assert_eq!(test_json["ok"], true);
    assert_eq!(test_json["data"]["report"]["entity_scope"]["kind"], "test");
    assert_eq!(
        test_json["data"]["report"]["entity_scope"]["id"],
        "TEST-M1-CLEAN"
    );
    assert_eq!(
        report_item(&test_json, "evidence_validity")["value"],
        "PASS"
    );

    let vo = invoke(
        &project.root,
        "verify",
        &["--vo", "VO-KNOWN", "--items", "test_existence"],
    );
    assert_exit(&vo, 0, "verify a selected VO");
    let vo_json = envelope(&vo);
    assert_eq!(vo_json["data"]["report"]["entity_scope"]["kind"], "vo");
    assert_eq!(report_item(&vo_json, "test_existence")["value"], "PASS");

    let req = invoke(
        &project.root,
        "verify",
        &["--req", "REQ-M6-SCOPE", "--items", "test_existence"],
    );
    assert_exit(&req, 0, "verify a selected REQ");
    let req_json = envelope(&req);
    assert_eq!(req_json["data"]["report"]["entity_scope"]["kind"], "req");
    assert_eq!(report_item(&req_json, "test_existence")["value"], "PASS");

    let report = invoke(
        &project.root,
        "report",
        &["--test", "TEST-M1-CLEAN", "--items", "evidence_validity"],
    );
    assert_exit(&report, 0, "report a selected Test");
    let report_json = envelope(&report);
    assert_eq!(report_json["data"]["report"]["result"], "PASS");
}

#[test]
fn m6_complete_fixture_is_ok_for_all_eleven_items() {
    let project = TempProject::from_m1_base("complete");
    fs::create_dir_all(project.root.join("docs")).expect("create fixture docs");
    fs::write(
        project.root.join("docs/m6-spec.md"),
        "# M6 fixture specification\nThe known operation returns successfully.\n",
    )
    .expect("write fixture specification");
    let spec = invoke(
        &project.root,
        "spec",
        &[
            "add",
            "--id",
            "SPEC-M6-FIXTURE",
            "--path",
            "docs/m6-spec.md",
        ],
    );
    assert_exit(&spec, 0, "register complete fixture SPEC");
    fs::write(
        project.root.join(".verify/req/REQ-M6-FIXTURE.yaml"),
        "id: REQ-M6-FIXTURE\nparent: null\nspec_refs:\n  - spec: SPEC-M6-FIXTURE\n    section: 1\nsummary: complete fixture\nstatus: active\ncreated: '2026-01-01'\nupdated: '2026-01-01'\nversion: 1\n",
    )
    .expect("write complete fixture REQ");
    let vo_path = project.root.join(".verify/vo/VO-KNOWN.yaml");
    let vo = fs::read_to_string(&vo_path).expect("read complete fixture VO");
    fs::write(
        vo_path,
        vo.replace("requirements: []", "requirements:\n  - REQ-M6-FIXTURE")
            .replace(
                "spec_refs: []",
                "spec_refs:\n  - spec: SPEC-M6-FIXTURE\n    section: 1",
            ),
    )
    .expect("link complete fixture VO");
    fs::write(
        project.root.join("tests/registered.rs"),
        "fn known() {}\n\n/// @vtest.id TEST-M1-CLEAN\n/// @vtest.covers VO-KNOWN\n/// @vtest.target tests/registered.rs::known\n/// @vtest.intent a complete M6 fixture test\n#[test]\nfn clean_scan_baseline() {\n    assert_eq!(known(), ());\n}\n",
    )
    .expect("make fixture target and assertion same-file");
    project.commit_baseline();

    let static_audit = invoke(&project.root, "audit", &["static", "--all"]);
    assert_exit(&static_audit, 0, "complete fixture static audit");

    let semantic_bundle = bundle(&project, "test-semantic", &["--test", "TEST-M1-CLEAN"]);
    submit(
        &project,
        &semantic_bundle,
        "test-semantic",
        serde_json::json!([{
            "claim": "the test checks the declared target result",
            "basis": [{"kind": "test-code", "ref": "tests/registered.rs::clean_scan_baseline"}]
        }]),
    );
    let implementation_bundle = bundle(&project, "impl-consistency", &["--test", "TEST-M1-CLEAN"]);
    submit(
        &project,
        &implementation_bundle,
        "impl-consistency",
        serde_json::json!([{
            "claim": "the declared target exists and matches the fixture",
            "basis": [{"kind": "target-code", "ref": "tests/registered.rs::known"}]
        }]),
    );
    let vo_bundle = bundle(&project, "vo-coverage", &["--vo", "VO-KNOWN"]);
    let vo_audit = submit(
        &project,
        &vo_bundle,
        "vo-coverage",
        serde_json::json!([{
            "kind": "decomposition-viewpoint",
            "claim": "the fixture VO is covered by its leaf test",
            "basis": [{"kind": "spec", "ref": "SPEC-M6-FIXTURE#1"}]
        }]),
    );
    let vo_audit_id = vo_audit["audit_id"]
        .as_str()
        .expect("vo audit id")
        .to_owned();
    let mut approval_args = vec![
        "approve",
        "VO-KNOWN",
        "--approver-kind",
        "human",
        "--approver-id",
        "m6-reviewer",
        "--basis",
    ];
    approval_args.push(&vo_audit_id);
    let approval = invoke(&project.root, "vo", &approval_args);
    assert_exit(&approval, 0, "approve complete fixture VO");
    // Approval derives the VO status and therefore changes its canonical hash.
    // Regenerate the coverage audit after approval so the current record is
    // the one that can participate in a PASS; the earlier immutable record
    // must remain stale and ignored.
    let current_vo_bundle = bundle(&project, "vo-coverage", &["--vo", "VO-KNOWN"]);
    submit(
        &project,
        &current_vo_bundle,
        "vo-coverage",
        serde_json::json!([{
            "kind": "decomposition-viewpoint",
            "claim": "the approved fixture VO is covered by its leaf test",
            "basis": [{"kind": "spec", "ref": "SPEC-M6-FIXTURE#1"}]
        }]),
    );
    let current_semantic_bundle = bundle(&project, "test-semantic", &["--test", "TEST-M1-CLEAN"]);
    submit(
        &project,
        &current_semantic_bundle,
        "test-semantic",
        serde_json::json!([{
            "claim": "the approved fixture test checks the declared target result",
            "basis": [{"kind": "test-code", "ref": "tests/registered.rs::clean_scan_baseline"}]
        }]),
    );
    let current_implementation_bundle =
        bundle(&project, "impl-consistency", &["--test", "TEST-M1-CLEAN"]);
    submit(
        &project,
        &current_implementation_bundle,
        "impl-consistency",
        serde_json::json!([{
            "claim": "the approved declared target exists and matches the fixture",
            "basis": [{"kind": "target-code", "ref": "tests/registered.rs::known"}]
        }]),
    );

    let run = invoke(&project.root, "run", &["--all", "--fast"]);
    assert_exit(&run, 0, "record complete fixture Evidence");
    let evidence_dir = project.root.join(".verify/evidence");
    let evidence_path = fs::read_dir(&evidence_dir)
        .expect("read complete fixture Evidence")
        .map(|entry| entry.expect("read Evidence entry").path())
        .find(|path| path.extension().and_then(|ext| ext.to_str()) == Some("yaml"))
        .expect("one complete fixture Evidence record");
    let evidence = fs::read_to_string(&evidence_path).expect("read Evidence YAML");
    fs::write(
        &evidence_path,
        // Fabricate a measured target execution from the §442 not-checked form
        // (checked false, method/result null, empty targets).
        evidence
            .replace("checked: false", "checked: true")
            .replace("method: null", "method: llvm-cov")
            .replace("result: null", "result: 'PASS'"),
    )
    .expect("mark fixture target execution measured");
    let verify = invoke(&project.root, "verify", &[]);
    assert_exit(&verify, 0, "complete fixture verifies all eleven items");
    let json = envelope(&verify);
    assert_eq!(json["ok"], true, "complete fixture: {json}");
    for item in [
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
    ] {
        assert_eq!(report_item(&json, item)["value"], "PASS", "{item}: {json}");
    }
}

#[test]
fn m6_leaf_without_test_is_missing_and_not_masked_by_another_leaf() {
    let project = TempProject::from_m1_base("leaf-missing");
    fs::write(
        project.root.join(".verify/vo/VO-UNTESTED.yaml"),
        "id: VO-UNTESTED\nparent: null\nrequirements: []\nspec_refs: []\nclaim: missing leaf\ndimensions: []\ncoverage_policy: null\nrepresentative_cases: []\nstatus: draft\ncreated: '2026-01-01'\nupdated: '2026-01-01'\nversion: 1\n",
    )
    .expect("add untested leaf VO");
    project.commit_baseline();

    let verify = invoke(&project.root, "verify", &["--items", "test_existence"]);
    assert_exit(&verify, 1, "an uncovered leaf VO is non-PASS");
    let json = envelope(&verify);
    assert_eq!(json["ok"], false);
    assert_eq!(report_item(&json, "test_existence")["value"], "MISSING");
    assert!(
        report_item(&json, "test_existence")["basis"]
            .as_array()
            .is_some_and(|basis| basis.iter().any(|entry| {
                entry
                    .as_str()
                    .is_some_and(|text| text.contains("VO-UNTESTED"))
            })),
        "missing leaf must be named in basis: {json}"
    );
}

#[test]
fn m6_missing_evidence_for_one_test_is_not_project_pass() {
    let project = TempProject::from_m1_base("partial-execution");
    let registered = project.root.join("tests/registered.rs");
    let mut source = fs::read_to_string(&registered).expect("read registered tests");
    source.push_str(
        "\n/// @vtest.id TEST-M1-UNEXECUTED\n/// @vtest.covers VO-KNOWN\n/// @vtest.target src/lib.rs::known\n/// @vtest.intent remains unexecuted\n#[test]\nfn remains_unexecuted() {}\n",
    );
    fs::write(registered, source).expect("add unexecuted registered test");
    project.commit_baseline();

    let run = invoke(&project.root, "run", &["--test", "TEST-M1-CLEAN", "--fast"]);
    assert_exit(&run, 0, "execute only one of two registered Tests");
    let verify = invoke(&project.root, "verify", &["--items", "test_execution"]);
    assert_exit(&verify, 1, "missing Evidence is fail-closed");
    let json = envelope(&verify);
    assert_eq!(
        report_item(&json, "test_execution")["value"],
        "NOT_EXECUTED"
    );
}

#[test]
fn m6_limited_scope_keeps_other_items_not_checked_and_text_is_tree_like() {
    let project = TempProject::from_m1_base("limited-scope");
    project.commit_baseline();

    let verify = invoke(&project.root, "verify", &["--items", "test_existence"]);
    assert_exit(&verify, 0, "limited item scope can pass its requested item");
    let json = envelope(&verify);
    assert_eq!(json["data"]["report"]["scope_outside_not_checked"], true);
    assert_eq!(report_item(&json, "test_existence")["value"], "PASS");
    assert_eq!(report_item(&json, "static_audit")["value"], "NOT_CHECKED");

    let text = invoke_text(&project.root, "report", &["--items", "test_existence"]);
    assert_exit(&text, 0, "text report for a limited scope");
    let output = String::from_utf8(text.stdout).expect("text report is UTF-8");
    assert!(output.contains("Requested scope"), "text report: {output}");
    assert!(output.contains("test_existence"), "text report: {output}");
    assert!(output.contains("NOT_CHECKED"), "text report: {output}");
}

#[test]
fn m6_each_check_item_can_be_non_pass_without_aggregate_promotion() {
    let items = [
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
    ];
    for item in items {
        let project = TempProject::from_m1_base(item.replace('_', "-").as_str());
        if item == "test_existence" {
            fs::write(
                project.root.join(".verify/vo/VO-M6-UNCOVERED.yaml"),
                "id: VO-M6-UNCOVERED\nparent: null\nrequirements: []\nspec_refs: []\nclaim: uncovered\ndimensions: []\ncoverage_policy: null\nrepresentative_cases: []\nstatus: draft\ncreated: '2026-01-01'\nupdated: '2026-01-01'\nversion: 1\n",
            )
            .expect("add uncovered VO");
        }
        if item == "vo_decomposition" {
            fs::write(project.root.join("src/broken.rs"), "pub fn broken( {\n")
                .expect("add malformed source");
        }
        project.commit_baseline();
        let verify = invoke(&project.root, "verify", &["--items", item]);
        assert_exit(
            &verify,
            1,
            &format!("{item} must independently make verification NG"),
        );
        let json = envelope(&verify);
        assert_eq!(json["ok"], false, "{item}: {json}");
        assert_ne!(
            report_item(&json, item)["value"],
            "PASS",
            "{item} was promoted to PASS: {json}"
        );
    }
}
