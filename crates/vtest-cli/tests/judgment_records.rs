//! Acceptance coverage for the judgment-record protocol: `vtest audit
//! bundle` / `vtest audit submit` (本冊 §8, DS-692〜DS-724, DS-1084〜DS-1090).

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

use serde_json::{json, Value};
use vtest_model::{
    DerivesFrom, DocumentFile, DocumentId, NodeSource, RootNode, SentenceNode, VoId, VoRecord,
};
use vtest_store::{write_document_file, write_vo_record, VerifyLayout};

static NEXT: AtomicU64 = AtomicU64::new(0);

const TESTS: &str = "/// @vtest.id TEST-GOOD\n\
/// @vtest.covers VO-X\n\
/// @vtest.target src/lib.rs::add\n\
/// @vtest.intent adds\n\
#[test]\n\
fn good() {\n    assert_eq!(judge_fixture::add(2, 3), 5);\n}\n";

fn fixture(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "vtest-judgment-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("tests")).unwrap();
    vtest_store::init_project(&root, "judgment").unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"judge-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
    )
    .unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\n",
    )
    .unwrap();
    fs::write(root.join("tests/judge.rs"), TESTS).unwrap();

    let layout = VerifyLayout::new(&root);
    let source = |id: &str| NodeSource {
        doc: format!("{id}.md"),
        heading: "judgment".to_owned(),
        lines: [1, 1],
    };
    let document = DocumentFile {
        schema_version: "0.1".to_owned(),
        root: vec![RootNode {
            id: DocumentId::new("ROOT-001"),
            statement: "judgment root".to_owned(),
            description: None,
            source: source("root"),
        }],
        request: vec![SentenceNode {
            id: DocumentId::new("R-001"),
            statement: "adding works".to_owned(),
            description: None,
            derives_from: vec![DocumentId::new("ROOT-001")],
            cites: None,
            source: source("request"),
        }],
        require: Vec::new(),
        spec: Vec::new(),
        detailed_spec: Vec::new(),
        basic_design: Vec::new(),
        design: Vec::new(),
    };
    write_document_file(&layout, "judgment", &document).unwrap();
    write_vo_record(
        &layout,
        &VoRecord {
            id: VoId::new("VO-X"),
            parent: None,
            derives_from: vec![DerivesFrom {
                doc: DocumentId::new("R-001"),
                anchor: None,
                note: None,
            }],
            claim: "add returns the sum".to_owned(),
            dimensions: Vec::new(),
            coverage_policy: None,
            combinations: Vec::new(),
            representative_cases: Vec::new(),
            created: "2026-11-04T00:00:00Z".to_owned(),
            updated: "2026-11-04T00:00:00Z".to_owned(),
        },
    )
    .unwrap();
    root
}

fn vtest(root: &Path, args: &[&str]) -> (i32, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_vtest"))
        .arg("--project")
        .arg(root)
        .args(["--format", "json"])
        .args(args)
        .output()
        .unwrap();
    let envelope = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&output.stdout)));
    (output.status.code().unwrap_or(-1), envelope)
}

fn bundle(root: &Path) -> Value {
    let (code, envelope) = vtest(root, &["audit", "bundle", "--test", "TEST-GOOD"]);
    assert_eq!(code, 0, "{envelope}");
    envelope["data"].clone()
}

fn submission(bundle: &Value) -> Value {
    json!({
        "bundle_id": bundle["bundle_id"],
        "subject": "TEST-GOOD",
        "judgment_kind": "test-semantic",
        "decision": "accepted",
        "actor": {"kind": "human", "id": "reviewer"}
    })
}

fn submit(root: &Path, submission: &Value) -> (i32, Value) {
    let path = root.join("result.json");
    fs::write(&path, submission.to_string()).unwrap();
    vtest(root, &["audit", "submit", "--file", path.to_str().unwrap()])
}

fn code_of(envelope: &Value) -> &str {
    envelope["diagnostics"][0]["code"]
        .as_str()
        .unwrap_or_default()
}

fn decision_files(root: &Path) -> Vec<PathBuf> {
    fs::read_dir(root.join(".verify/decisions"))
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "yaml"))
                .collect()
        })
        .unwrap_or_default()
}

/// @vtest.id TEST-JUDGMENT-SUBMIT-REASON-OPTIONAL
/// @vtest.covers VO-JUDGMENT-SUBMIT-REASON-OPTIONAL, VO-JUDGMENT-BUNDLE-KIND-COPIED
/// @vtest.target crates/vtest-ops/src/judgment.rs::submit
/// @vtest.intent a submission without reason or exclusions is accepted and written to .verify/decisions with the bundle's judgment_kind and hashes
#[test]
fn a_submission_without_a_reason_is_accepted_and_recorded() {
    let root = fixture("reason");
    let bundle = bundle(&root);
    assert_eq!(bundle["judgment_kind"], "test-semantic");
    assert!(root
        .join(format!(
            ".verify/cache/bundles/{}.json",
            bundle["bundle_id"].as_str().unwrap()
        ))
        .is_file());

    let (code, envelope) = submit(&root, &submission(&bundle));
    assert_eq!(code, 0, "{envelope}");
    assert_eq!(envelope["data"]["accepted"], true);
    let id = envelope["data"]["decision_id"].as_str().unwrap();
    let text = fs::read_to_string(root.join(format!(".verify/decisions/{id}.yaml"))).unwrap();
    let record = vtest_store::decisions::DecisionRecord::from_yaml(&text, id).unwrap();
    assert!(record.reason.is_empty());
    assert_eq!(record.judgment_kind, "test-semantic");
    assert_eq!(
        serde_json::to_value(&record.subject_hash).unwrap(),
        bundle["subject_hash"]
    );
    assert_eq!(
        serde_json::to_value(&record.dependencies).unwrap(),
        bundle["dependencies"]
    );

    // An explicit empty reason list is just as acceptable.
    let mut explicit = submission(&bundle);
    explicit["reason"] = json!([]);
    explicit["exclusions"] = json!([]);
    let (code, envelope) = submit(&root, &explicit);
    assert_eq!(code, 0, "{envelope}");
}

/// @vtest.id TEST-JUDGMENT-SUBMIT-STALE-HASH
/// @vtest.covers VO-JUDGMENT-SUBMIT-STALE-HASH-E-AUDIT-002
/// @vtest.target crates/vtest-ops/src/judgment.rs::submit
/// @vtest.intent changing the declared target after the bundle was recorded makes the submission fail with E-AUDIT-002 and writes no record
#[test]
fn a_target_changed_after_bundling_is_rejected_with_e_audit_002() {
    let root = fixture("stale");
    let bundle = bundle(&root);
    fs::write(
        root.join("src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { b + a }\n",
    )
    .unwrap();
    let (code, envelope) = submit(&root, &submission(&bundle));
    assert_eq!(code, 2, "{envelope}");
    assert_eq!(code_of(&envelope), "E-AUDIT-002");
    assert!(decision_files(&root).is_empty());
}

/// @vtest.id TEST-JUDGMENT-SUBMIT-ORDERED-CHECKS
/// @vtest.covers VO-JUDGMENT-SUBMIT-ORDERED-CHECKS, VO-JUDGMENT-SUBMIT-UNKNOWN-BUNDLE-E-AUDIT-001, VO-JUDGMENT-SUBMIT-SUPERSEDES-E-AUDIT-008
/// @vtest.target crates/vtest-ops/src/judgment.rs::submit
/// @vtest.intent each DS-1089 check rejects with its own code: unknown bundle E-AUDIT-001, subject or kind mismatch E-AUDIT-003, bad decision E-AUDIT-004, bad supersedes E-AUDIT-008
#[test]
fn each_submission_check_rejects_with_its_own_code() {
    let root = fixture("codes");
    let bundle = bundle(&root);

    let mut unknown = submission(&bundle);
    unknown["bundle_id"] = json!("01ZZZZZZZZZZZZZZZZZZZZZZZZ");
    // The bundle check runs first, even before the schema check.
    unknown["decision"] = json!("maybe");
    let (code, envelope) = submit(&root, &unknown);
    assert_eq!((code, code_of(&envelope)), (2, "E-AUDIT-001"), "{envelope}");

    let mut subject = submission(&bundle);
    subject["subject"] = json!("TEST-OTHER");
    let (_, envelope) = submit(&root, &subject);
    assert_eq!(code_of(&envelope), "E-AUDIT-003", "{envelope}");

    let mut kind = submission(&bundle);
    kind["judgment_kind"] = json!("case-coverage");
    let (_, envelope) = submit(&root, &kind);
    assert_eq!(code_of(&envelope), "E-AUDIT-003", "{envelope}");

    let mut missing_actor = submission(&bundle);
    missing_actor.as_object_mut().unwrap().remove("actor");
    let (_, envelope) = submit(&root, &missing_actor);
    assert_eq!(code_of(&envelope), "E-AUDIT-003", "{envelope}");

    let mut decision = submission(&bundle);
    decision["decision"] = json!("maybe");
    let (_, envelope) = submit(&root, &decision);
    assert_eq!(code_of(&envelope), "E-AUDIT-004", "{envelope}");

    let mut supersedes = submission(&bundle);
    supersedes["supersedes"] = json!(["01ZZZZZZZZZZZZZZZZZZZZZZZZ"]);
    let (_, envelope) = submit(&root, &supersedes);
    assert_eq!(code_of(&envelope), "E-AUDIT-008", "{envelope}");
    assert!(decision_files(&root).is_empty());

    // A valid supersedes chain is accepted.
    let (code, first) = submit(&root, &submission(&bundle));
    assert_eq!(code, 0, "{first}");
    let mut second = submission(&bundle);
    second["supersedes"] = json!([first["data"]["decision_id"]]);
    second["decision"] = json!("rejected");
    let (code, envelope) = submit(&root, &second);
    assert_eq!(code, 0, "{envelope}");
    assert_eq!(decision_files(&root).len(), 2);
}

/// @vtest.id TEST-JUDGMENT-ACCEPTANCE-NO-STATE-CHANGE
/// @vtest.covers VO-JUDGMENT-ACCEPTANCE-NO-STATE-CHANGE
/// @vtest.target crates/vtest-ops/src/judgment.rs::submit
/// @vtest.intent verify reports the same states and exit code before and after a judgment is accepted
#[test]
fn an_accepted_judgment_does_not_change_verification_state() {
    let root = fixture("state");
    let (before_code, before) = vtest(&root, &["verify"]);
    let bundle = bundle(&root);
    let (code, envelope) = submit(&root, &submission(&bundle));
    assert_eq!(code, 0, "{envelope}");
    let (after_code, after) = vtest(&root, &["verify"]);
    assert_eq!(before_code, after_code);
    assert_eq!(before["data"], after["data"]);
}

/// @vtest.id TEST-JUDGMENT-BUNDLE-SELECTION
/// @vtest.covers VO-JUDGMENT-BUNDLE-VO-REQUIRES-CASE-COVERAGE, VO-JUDGMENT-BUNDLE-UNRESOLVED-TARGET
/// @vtest.target crates/vtest-ops/src/judgment.rs::bundle
/// @vtest.intent --vo without --kind case-coverage is a usage error, and a Test whose declared target does not resolve gets no bundle
#[test]
fn bundle_rejects_bad_kind_combinations_and_unresolved_targets() {
    let root = fixture("selection");
    for args in [
        vec!["audit", "bundle", "--vo", "VO-X"],
        vec!["audit", "bundle", "--vo", "VO-X", "--kind", "test-semantic"],
        vec!["audit", "bundle", "--test", "TEST-GOOD", "--kind", "nope"],
    ] {
        let (code, envelope) = vtest(&root, &args);
        assert_eq!((code, code_of(&envelope)), (2, "E-OP-001"), "{args:?}");
    }
    let (code, envelope) = vtest(
        &root,
        &["audit", "bundle", "--vo", "VO-X", "--kind", "case-coverage"],
    );
    assert_eq!(code, 0, "{envelope}");
    assert_eq!(envelope["data"]["judgment_kind"], "case-coverage");

    let bundles = || {
        fs::read_dir(root.join(".verify/cache/bundles"))
            .unwrap()
            .count()
    };
    let before = bundles();
    fs::write(root.join("src/lib.rs"), "pub fn other() {}\n").unwrap();
    let (code, envelope) = vtest(&root, &["audit", "bundle", "--test", "TEST-GOOD"]);
    assert_eq!((code, code_of(&envelope)), (2, "E-SCAN-004"), "{envelope}");
    assert_eq!(bundles(), before, "no bundle is generated");
}

fn vo_bundle(root: &Path) -> Value {
    let (code, envelope) = vtest(
        root,
        &["audit", "bundle", "--vo", "VO-X", "--kind", "case-coverage"],
    );
    assert_eq!(code, 0, "{envelope}");
    envelope["data"].clone()
}

fn vo_submission(bundle: &Value) -> Value {
    json!({
        "bundle_id": bundle["bundle_id"],
        "subject": "VO-X",
        "judgment_kind": "case-coverage",
        "decision": "accepted",
        "actor": {"kind": "human", "id": "reviewer"}
    })
}

/// @vtest.id TEST-JUDGMENT-VO-SUBMIT-COVERING-TESTS-BOUND
/// @vtest.covers VO-JUDGMENT-SUBMIT-STALE-HASH-E-AUDIT-002
/// @vtest.target crates/vtest-ops/src/judgment.rs::submit
/// @vtest.target crates/vtest-ops/src/judgment.rs::current_binding
/// @vtest.intent a VO bundle binds the covering Tests it hands the judge: changing, adding or removing a covering Test after bundling makes the submission fail with E-AUDIT-002, while an unchanged one is accepted
#[test]
fn a_vo_bundle_rejects_changed_added_or_removed_covering_tests() {
    let other = "\n/// @vtest.id TEST-OTHER\n\
/// @vtest.covers VO-X\n\
/// @vtest.target src/lib.rs::add\n\
/// @vtest.intent adds again\n\
#[test]\n\
fn other() {\n    assert_eq!(judge_fixture::add(1, 1), 2);\n}\n";
    let edits: [(&str, String); 3] = [
        (
            "changed",
            TESTS.replace("@vtest.intent adds", "@vtest.intent sums"),
        ),
        ("added", format!("{TESTS}{other}")),
        ("removed", String::new()),
    ];
    for (name, replacement) in edits {
        let root = fixture(&format!("vo-{name}"));
        let bundle = vo_bundle(&root);
        let bound: Vec<&str> = bundle["dependencies"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|dep| dep["kind"] == "test")
            .filter_map(|dep| dep["id"].as_str())
            .collect();
        assert_eq!(bound, ["TEST-GOOD"], "{name}: {bundle}");
        fs::write(root.join("tests/judge.rs"), replacement).unwrap();
        let (code, envelope) = submit(&root, &vo_submission(&bundle));
        assert_eq!(code, 2, "{name}: {envelope}");
        assert_eq!(code_of(&envelope), "E-AUDIT-002", "{name}");
        assert!(decision_files(&root).is_empty(), "{name}");
    }

    let root = fixture("vo-unchanged");
    let bundle = vo_bundle(&root);
    let (code, envelope) = submit(&root, &vo_submission(&bundle));
    assert_eq!(code, 0, "{envelope}");
}

/// @vtest.id TEST-JUDGMENT-IMPL-CONSISTENCY-BUNDLE-DOCUMENTS
/// @vtest.covers VO-JUDGMENT-IMPL-CONSISTENCY-BUNDLE-DOCUMENTS
/// @vtest.target crates/vtest-ops/src/judgment.rs::bundle
/// @vtest.intent an impl-consistency bundle carries every upstream document node of the covered VO with its full content and the target source's full text; a test-semantic bundle carries neither
#[test]
fn an_impl_consistency_bundle_carries_documents_and_source_text() {
    let root = fixture("impl");
    let (code, envelope) = vtest(
        &root,
        &[
            "audit",
            "bundle",
            "--test",
            "TEST-GOOD",
            "--kind",
            "impl-consistency",
        ],
    );
    assert_eq!(code, 0, "{envelope}");
    let bundle = &envelope["data"];
    let documents = bundle["documents"].as_array().unwrap();
    let ids: Vec<&str> = documents
        .iter()
        .filter_map(|doc| doc["id"].as_str())
        .collect();
    assert_eq!(ids, ["R-001", "ROOT-001"], "{bundle}");
    let request = &documents[0];
    assert_eq!(request["doc"], "judgment");
    assert_eq!(request["node"]["statement"], "adding works");
    assert_eq!(request["node"]["derives_from"], json!(["ROOT-001"]));
    assert_eq!(request["node"]["source"]["doc"], "request.md");
    assert_eq!(documents[1]["node"]["statement"], "judgment root");
    assert_eq!(
        bundle["targets"][0]["source_text"],
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\n"
    );

    let plain = bundle_of_kind(&root, "test-semantic");
    assert_eq!(plain["documents"], json!([]));
    assert!(plain["targets"][0].get("source_text").is_none());
}

fn bundle_of_kind(root: &Path, kind: &str) -> Value {
    let (code, envelope) = vtest(
        root,
        &["audit", "bundle", "--test", "TEST-GOOD", "--kind", kind],
    );
    assert_eq!(code, 0, "{envelope}");
    envelope["data"].clone()
}

const OTHER: &str = "\n/// @vtest.id TEST-OTHER\n\
/// @vtest.covers VO-X\n\
/// @vtest.target src/lib.rs::add\n\
/// @vtest.intent adds again\n\
#[test]\n\
fn other() {\n    assert_eq!(judge_fixture::add(1, 1), 2);\n}\n";

const THIRD: &str = "\n/// @vtest.id TEST-THIRD\n\
/// @vtest.covers VO-X\n\
/// @vtest.target src/lib.rs::add\n\
/// @vtest.intent adds a third time\n\
#[test]\n\
fn third() {\n    assert_eq!(judge_fixture::add(0, 0), 0);\n}\n";

fn bundle_count(root: &Path) -> usize {
    fs::read_dir(root.join(".verify/cache/bundles"))
        .map(|entries| entries.count())
        .unwrap_or(0)
}

fn case_coverage_bundle(root: &Path) -> Value {
    bundle_of_kind(root, "case-coverage")
}

fn case_coverage_submission(bundle: &Value) -> Value {
    let mut submission = submission(bundle);
    submission["judgment_kind"] = json!("case-coverage");
    submission
}

/// @vtest.id TEST-JUDGMENT-TEST-SUBMIT-RELATED-TESTS-BOUND
/// @vtest.covers VO-JUDGMENT-SUBMIT-STALE-HASH-E-AUDIT-002
/// @vtest.target crates/vtest-ops/src/judgment.rs::submit
/// @vtest.target crates/vtest-ops/src/judgment.rs::current_binding
/// @vtest.intent a Test bundle binds the related Tests it hands the judge: changing, adding or removing a related Test after bundling makes the submission fail with E-AUDIT-002 and writes no record, while an unchanged one is accepted
#[test]
fn a_test_bundle_rejects_changed_added_or_removed_related_tests() {
    let edits: [(&str, String); 3] = [
        (
            "changed",
            format!(
                "{TESTS}{}",
                OTHER.replace("@vtest.intent adds again", "@vtest.intent sums again")
            ),
        ),
        ("added", format!("{TESTS}{OTHER}{THIRD}")),
        ("removed", TESTS.to_owned()),
    ];
    for (name, replacement) in edits {
        let root = fixture(&format!("related-{name}"));
        fs::write(root.join("tests/judge.rs"), format!("{TESTS}{OTHER}")).unwrap();
        let bundle = case_coverage_bundle(&root);
        let related: Vec<&str> = bundle["related_tests"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|test| test["id"].as_str())
            .collect();
        assert_eq!(related, ["TEST-OTHER"], "{name}: {bundle}");
        let bound: Vec<&str> = bundle["dependencies"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|dep| dep["kind"] == "test")
            .filter_map(|dep| dep["id"].as_str())
            .collect();
        assert_eq!(bound, ["TEST-OTHER"], "{name}: {bundle}");
        fs::write(root.join("tests/judge.rs"), replacement).unwrap();
        let (code, envelope) = submit(&root, &case_coverage_submission(&bundle));
        assert_eq!(code, 2, "{name}: {envelope}");
        assert_eq!(code_of(&envelope), "E-AUDIT-002", "{name}");
        assert!(decision_files(&root).is_empty(), "{name}");
    }

    let root = fixture("related-unchanged");
    fs::write(root.join("tests/judge.rs"), format!("{TESTS}{OTHER}")).unwrap();
    let bundle = case_coverage_bundle(&root);
    let (code, envelope) = submit(&root, &case_coverage_submission(&bundle));
    assert_eq!(code, 0, "{envelope}");
    assert_eq!(decision_files(&root).len(), 1);
}

/// @vtest.id TEST-JUDGMENT-DUPLICATE-TEST-ID-REFUSED
/// @vtest.covers VO-JUDGMENT-DUPLICATE-TEST-ID-REFUSED
/// @vtest.target crates/vtest-ops/src/judgment.rs::bundle
/// @vtest.target crates/vtest-ops/src/judgment.rs::submit
/// @vtest.intent a Test id declared in two files is refused with E-SCAN-002 by audit bundle (Test or VO subject) and by audit submit, and neither a bundle nor a judgment record is written
#[test]
fn a_duplicated_test_id_is_refused_with_e_scan_002() {
    let duplicate = TESTS.replace("fn good()", "fn good_again()");

    let root = fixture("duplicate-bundle");
    fs::write(root.join("tests/again.rs"), &duplicate).unwrap();
    for args in [
        vec!["audit", "bundle", "--test", "TEST-GOOD"],
        vec!["audit", "bundle", "--vo", "VO-X", "--kind", "case-coverage"],
    ] {
        let (code, envelope) = vtest(&root, &args);
        assert_eq!(
            (code, code_of(&envelope)),
            (2, "E-SCAN-002"),
            "{args:?}: {envelope}"
        );
    }
    assert_eq!(bundle_count(&root), 0, "no bundle is generated");

    let root = fixture("duplicate-submit");
    let bundle = bundle(&root);
    fs::write(root.join("tests/again.rs"), &duplicate).unwrap();
    let (code, envelope) = submit(&root, &submission(&bundle));
    assert_eq!((code, code_of(&envelope)), (2, "E-SCAN-002"), "{envelope}");
    assert!(decision_files(&root).is_empty());
}

/// @vtest.id TEST-JUDGMENT-VO-COVERING-TEST-TARGETS-BOUND
/// @vtest.covers VO-JUDGMENT-BUNDLE-UNRESOLVED-TARGET, VO-JUDGMENT-SUBMIT-STALE-HASH-E-AUDIT-002
/// @vtest.target crates/vtest-ops/src/judgment.rs::bundle
/// @vtest.target crates/vtest-ops/src/judgment.rs::current_binding
/// @vtest.intent a VO bundle resolves every covering Test's declared target: an unresolved one gives E-SCAN-004 and no bundle, and changing or deleting the target after bundling makes the submission fail with E-AUDIT-002 and writes no record
#[test]
fn a_vo_bundle_resolves_and_binds_covering_test_targets() {
    let root = fixture("vo-unresolved");
    fs::write(
        root.join("tests/judge.rs"),
        TESTS.replace("src/lib.rs::add", "src/lib.rs::missing"),
    )
    .unwrap();
    let (code, envelope) = vtest(
        &root,
        &["audit", "bundle", "--vo", "VO-X", "--kind", "case-coverage"],
    );
    assert_eq!((code, code_of(&envelope)), (2, "E-SCAN-004"), "{envelope}");
    assert_eq!(bundle_count(&root), 0, "no bundle is generated");

    for (name, source) in [
        ("changed", "pub fn add(a: i32, b: i32) -> i32 { b + a }\n"),
        ("deleted", "pub fn other() {}\n"),
    ] {
        let root = fixture(&format!("vo-target-{name}"));
        let bundle = vo_bundle(&root);
        assert!(
            bundle["dependencies"]
                .as_array()
                .unwrap()
                .iter()
                .any(|dep| dep["kind"] == "target" && dep["id"] == "src/lib.rs::add"),
            "{name}: {bundle}"
        );
        fs::write(root.join("src/lib.rs"), source).unwrap();
        let (code, envelope) = submit(&root, &vo_submission(&bundle));
        assert_eq!(
            (code, code_of(&envelope)),
            (2, "E-AUDIT-002"),
            "{name}: {envelope}"
        );
        assert!(decision_files(&root).is_empty(), "{name}");
    }
}

/// Calls one tool through the real `vtest mcp` stdio transport and returns
/// its envelope (`structuredContent`).
fn mcp_call(root: &Path, name: &str, arguments: &Value) -> Value {
    use std::io::{BufRead, BufReader, Write};
    use std::process::Stdio;
    let mut child = Command::new(env!("CARGO_BIN_EXE_vtest"))
        .arg("--project")
        .arg(root)
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut stdin = child.stdin.take().unwrap();
        for request in [
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {"name": name, "arguments": arguments}
            }),
        ] {
            writeln!(stdin, "{request}").unwrap();
        }
    }
    let output = child.wait_with_output().unwrap();
    let response: Value = BufReader::new(output.stdout.as_slice())
        .lines()
        .map(|line| serde_json::from_str::<Value>(&line.unwrap()).unwrap())
        .find(|response| response["id"] == 2)
        .unwrap_or_else(|| panic!("{}", String::from_utf8_lossy(&output.stderr)));
    response["result"]["structuredContent"].clone()
}

/// The envelope with the per-call fields (fresh ids, timestamps) removed.
fn comparable(envelope: &Value) -> Value {
    let mut envelope = envelope.clone();
    if let Some(data) = envelope["data"].as_object_mut() {
        for key in ["bundle_id", "generated_at", "decision_id"] {
            data.remove(key);
        }
    }
    envelope
}

const BROKEN: &str = "/// @vtest.id TEST-BROKEN\n\
/// @vtest.covers VO-X\n\
/// @vtest.target src/lib.rs::missing\n\
/// @vtest.intent targets a function that does not exist\n\
#[test]\n\
fn broken() {}\n";

/// @vtest.id TEST-JUDGMENT-MCP-TRANSPORT-SAME-AS-CLI
/// @vtest.covers VO-JUDGMENT-MCP-SUBMIT-RECORD-ID, VO-JUDGMENT-SUBMIT-ORDERED-CHECKS
/// @vtest.target crates/vtest-mcp/src/lib.rs::tools_call_result
/// @vtest.intent through the real vtest mcp stdio transport, audit_bundle and audit_submit give the same ok/data/diagnostics as the CLI: an unrelated Test's unresolved target does not stop a bundle or submission of a sound Test, and a target deleted after bundling is submit's E-AUDIT-002 in both, with no record written
#[test]
fn mcp_audit_bundle_and_submit_match_the_cli_over_stdio() {
    let root = fixture("mcp-parity");
    fs::write(root.join("tests/broken.rs"), BROKEN).unwrap();
    let (code, _) = vtest(&root, &["scan"]);
    assert_ne!(code, 0, "the fixture carries an unrelated scan error");

    let (code, cli_bundle) = vtest(&root, &["audit", "bundle", "--test", "TEST-GOOD"]);
    assert_eq!(code, 0, "{cli_bundle}");
    let mcp_bundle = mcp_call(&root, "audit_bundle", &json!({"test": "TEST-GOOD"}));
    assert_eq!(comparable(&mcp_bundle), comparable(&cli_bundle));

    let (code, cli_submit) = submit(&root, &submission(&cli_bundle["data"]));
    assert_eq!(code, 0, "{cli_submit}");
    let mcp_submit = mcp_call(&root, "audit_submit", &submission(&mcp_bundle["data"]));
    assert_eq!(comparable(&mcp_submit), comparable(&cli_submit));
    assert_eq!(decision_files(&root).len(), 2);

    let root = fixture("mcp-parity-stale");
    let cli_bundle = bundle(&root);
    let mcp_bundle = mcp_call(&root, "audit_bundle", &json!({"test": "TEST-GOOD"}));
    assert_eq!(mcp_bundle["ok"], true, "{mcp_bundle}");
    fs::write(root.join("src/lib.rs"), "pub fn other() {}\n").unwrap();
    let (code, cli_submit) = submit(&root, &submission(&cli_bundle));
    assert_eq!(
        (code, code_of(&cli_submit)),
        (2, "E-AUDIT-002"),
        "{cli_submit}"
    );
    let mcp_submit = mcp_call(&root, "audit_submit", &submission(&mcp_bundle["data"]));
    assert_eq!(code_of(&mcp_submit), "E-AUDIT-002", "{mcp_submit}");
    assert_eq!(comparable(&mcp_submit)["ok"], cli_submit["ok"]);
    assert!(decision_files(&root).is_empty());
}
