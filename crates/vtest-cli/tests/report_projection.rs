//! Acceptance coverage for `vtest report`, its projection and the
//! `pending` section (SPEC-401, DS-1133〜DS-1147, DS-1215, DS-1441,
//! DS-1505, DS-1524).

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

use serde_json::Value;
use vtest_model::{
    DerivesFrom, DocumentFile, DocumentId, NodeSource, RootNode, SentenceNode, VoId, VoRecord,
};
use vtest_store::{write_document_file, write_vo_record, VerifyLayout};

static NEXT: AtomicU64 = AtomicU64::new(0);

const TESTS: &str = "/// @vtest.id TEST-A\n\
/// @vtest.covers VO-CHILD\n\
/// @vtest.target src/lib.rs::add\n\
/// @vtest.case small numbers\n\
/// @vtest.intent adds\n\
#[test]\n\
fn a() {\n    assert_eq!(report_fixture::add(2, 3), 5);\n}\n\
\n\
/// @vtest.id TEST-B\n\
/// @vtest.covers VO-CHILD\n\
/// @vtest.target src/lib.rs::sub\n\
/// @vtest.intent subtracts\n\
#[test]\n\
fn b() {\n    let mut out = 0;\n    report_fixture::sub(3, 1, &mut out);\n    assert_eq!(out, 2);\n}\n";

fn vo(id: &str, parent: Option<&str>, anchor: Option<&str>) -> VoRecord {
    VoRecord {
        id: VoId::new(id),
        parent: parent.map(VoId::new),
        derives_from: vec![DerivesFrom {
            doc: DocumentId::new("R-001"),
            anchor: anchor.map(str::to_owned),
            note: anchor.map(|_| "the parent clause".to_owned()),
        }],
        claim: format!("{id} holds"),
        dimensions: Vec::new(),
        coverage_policy: None,
        combinations: Vec::new(),
        representative_cases: Vec::new(),
        created: "2026-11-04T00:00:00Z".to_owned(),
        updated: "2026-11-04T00:00:00Z".to_owned(),
    }
}

fn fixture(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "vtest-report-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("tests")).unwrap();
    vtest_store::init_project(&root, "report").unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"report-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
    )
    .unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\n\
         pub fn sub(a: i32, b: i32, out: &mut i32) { *out = a - b; }\n",
    )
    .unwrap();
    fs::write(root.join("tests/report.rs"), TESTS).unwrap();

    let layout = VerifyLayout::new(&root);
    let source = |id: &str| NodeSource {
        doc: format!("{id}.md"),
        heading: "report".to_owned(),
        lines: [1, 1],
    };
    let document = DocumentFile {
        schema_version: "0.1".to_owned(),
        root: vec![RootNode {
            id: DocumentId::new("ROOT-001"),
            statement: "report root".to_owned(),
            description: None,
            source: source("root"),
        }],
        request: vec![SentenceNode {
            id: DocumentId::new("R-001"),
            statement: "arithmetic works".to_owned(),
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
    write_document_file(&layout, "report", &document).unwrap();
    write_vo_record(&layout, &vo("VO-PARENT", None, Some("§1.2"))).unwrap();
    write_vo_record(&layout, &vo("VO-CHILD", Some("VO-PARENT"), None)).unwrap();
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

fn node_ids(projection: &Value) -> Vec<String> {
    projection["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| node["id"].as_str().unwrap().to_owned())
        .collect()
}

fn find_edge<'a>(projection: &'a Value, from: &str, relation: &str, to: &str) -> &'a Value {
    projection["edges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|edge| edge["from"] == from && edge["relation"] == relation && edge["to"] == to)
        .unwrap_or_else(|| panic!("no {from} {relation} {to} edge in {projection}"))
}

/// @vtest.id TEST-REPORT-SAME-AGGREGATION-AS-VERIFY
/// @vtest.covers VO-REPORT-SAME-AGGREGATION-AS-VERIFY
/// @vtest.target crates/vtest-cli/src/ops/report.rs::execute
/// @vtest.intent report returns the same scope, state, result, tree, structural checks, non-PASS count, pending section and exit code as verify for the same input
#[test]
fn report_runs_the_same_aggregation_as_verify() {
    let root = fixture("same");
    for scope in [
        vec![],
        vec!["--vo", "VO-PARENT"],
        vec!["--items", "chain_integrity"],
    ] {
        let mut verify_args = vec!["verify"];
        verify_args.extend(&scope);
        let mut report_args = vec!["report"];
        report_args.extend(&scope);
        let (verify_code, verify) = vtest(&root, &verify_args);
        let (report_code, report) = vtest(&root, &report_args);
        assert_eq!(verify_code, report_code, "{scope:?}");
        assert_eq!(verify["ok"], report["ok"]);
        for key in [
            "scope",
            "state",
            "result",
            "tree",
            "structural",
            "unevaluated",
            "non_pass",
            "pending",
        ] {
            assert_eq!(verify["data"][key], report["data"][key], "{key} {scope:?}");
        }
        assert!(report["data"]["projection"].is_object());
    }
}

/// @vtest.id TEST-REPORT-DOC-DOWN-PAIRS
/// @vtest.covers VO-REPORT-DOC-DOWN-DERIVES-FROM-PAIRS, VO-REPORT-ANCHOR-OMITTED-WHEN-ABSENT
/// @vtest.target crates/vtest-cli/src/ops/report.rs::project
/// @vtest.intent report --from a document node --direction down returns each derives_from edge as from / relation / to with the entry's anchor and note, and omits anchor when the entry has none
#[test]
fn a_document_traced_down_lists_derives_from_pairs_with_anchors() {
    let root = fixture("pairs");
    let (_, envelope) = vtest(&root, &["report", "--from", "R-001", "--direction", "down"]);
    let projection = &envelope["data"]["projection"];
    let ids = node_ids(projection);
    for id in ["R-001", "VO-PARENT", "VO-CHILD", "TEST-A", "TEST-B"] {
        assert!(ids.contains(&id.to_owned()), "{id} missing from {ids:?}");
    }
    assert!(
        !ids.contains(&"ROOT-001".to_owned()),
        "down never goes upstream"
    );
    let parent = find_edge(projection, "VO-PARENT", "derives_from", "R-001");
    assert_eq!(parent["anchor"], "§1.2");
    assert_eq!(parent["note"], "the parent clause");
    let child = find_edge(projection, "VO-CHILD", "derives_from", "R-001");
    assert!(child.get("anchor").is_none(), "{child}");
    assert!(child.get("note").is_none(), "{child}");
}

/// @vtest.id TEST-REPORT-PARENT-VO-BREAKDOWN
/// @vtest.covers VO-REPORT-PARENT-VO-BREAKDOWN, VO-REPORT-DEPTH-LIMIT
/// @vtest.target crates/vtest-cli/src/ops/report.rs::project
/// @vtest.intent a parent VO traced down lists every child VO and Test with its state and no Feature field, and --depth limits the number of steps followed
#[test]
fn a_parent_vo_traced_down_gives_the_feature_breakdown() {
    let root = fixture("feature");
    let (_, envelope) = vtest(
        &root,
        &["report", "--from", "VO-PARENT", "--direction", "down"],
    );
    let projection = &envelope["data"]["projection"];
    let nodes = projection["nodes"].as_array().unwrap();
    for id in ["VO-PARENT", "VO-CHILD", "TEST-A", "TEST-B"] {
        let node = nodes.iter().find(|node| node["id"] == id).unwrap();
        assert!(node["state"].is_string(), "{node}");
    }
    assert!(!envelope.to_string().contains("feature"));
    find_edge(projection, "VO-CHILD", "parent", "VO-PARENT");

    let (_, shallow) = vtest(
        &root,
        &[
            "report",
            "--from",
            "VO-PARENT",
            "--direction",
            "down",
            "--depth",
            "1",
        ],
    );
    assert_eq!(
        node_ids(&shallow["data"]["projection"]),
        vec!["VO-CHILD".to_owned(), "VO-PARENT".to_owned()]
    );
}

/// @vtest.id TEST-REPORT-VIEW-PRESETS
/// @vtest.covers VO-REPORT-VIEW-PRESET
/// @vtest.target crates/vtest-cli/src/ops/report.rs::project
/// @vtest.intent --view pm keeps documents and VOs, tester keeps VOs, Tests and targets, coder traces a target up to its document, and an unknown view or start node is E-OP-001
#[test]
fn view_presets_select_node_kinds() {
    let root = fixture("views");
    let kinds = |envelope: &Value| {
        let mut kinds: Vec<String> = envelope["data"]["projection"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|node| node["kind"].as_str().unwrap().to_owned())
            .collect();
        kinds.sort();
        kinds.dedup();
        kinds
    };
    let (_, pm) = vtest(&root, &["report", "--view", "pm"]);
    assert_eq!(kinds(&pm), vec!["doc", "vo"]);
    let (_, tester) = vtest(&root, &["report", "--view", "tester"]);
    assert_eq!(kinds(&tester), vec!["src", "test", "vo"]);
    let (_, coder) = vtest(
        &root,
        &[
            "report",
            "--view",
            "coder",
            "--from",
            "src/lib.rs::add",
            "--direction",
            "up",
        ],
    );
    let ids = node_ids(&coder["data"]["projection"]);
    for id in [
        "src/lib.rs::add",
        "TEST-A",
        "VO-CHILD",
        "VO-PARENT",
        "R-001",
        "ROOT-001",
    ] {
        assert!(ids.contains(&id.to_owned()), "{id} missing from {ids:?}");
    }
    assert!(!ids.contains(&"TEST-B".to_owned()));

    for args in [
        vec!["report", "--view", "boss"],
        vec!["report", "--from", "NOPE"],
        vec!["report", "--direction", "sideways"],
    ] {
        let (code, envelope) = vtest(&root, &args);
        assert_eq!(code, 2, "{args:?}");
        assert_eq!(envelope["diagnostics"][0]["code"], "E-OP-001", "{args:?}");
    }
}

/// @vtest.id TEST-PENDING-SECTION
/// @vtest.covers VO-PENDING-SECTION-IN-JSON, VO-PENDING-CASE-COVERAGE-ITEM, VO-PENDING-UNKNOWN-FROM-CHECK
/// @vtest.target crates/vtest-cli/src/ops/pending.rs::pending_items
/// @vtest.intent verify --format json carries a pending section with one case-coverage item (check null) per Test with cases and an unknown item for each UNKNOWN check, and report carries the same section
#[test]
fn verify_json_carries_the_pending_section() {
    let root = fixture("pending");
    let (_, envelope) = vtest(&root, &["verify"]);
    let pending = envelope["data"]["pending"].as_array().unwrap();
    let coverage: Vec<&Value> = pending
        .iter()
        .filter(|item| item["judgment_kind"] == "case-coverage")
        .collect();
    assert_eq!(coverage.len(), 1, "{pending:?}");
    assert_eq!(coverage[0]["subject"], "TEST-A");
    assert_eq!(coverage[0]["kind"], "unknown");
    assert!(coverage[0]["check"].is_null());

    // Every UNKNOWN check in the tree appears as a `kind: unknown` item.
    fn unknown_checks(node: &Value, out: &mut Vec<(String, String)>) {
        for check in node["checks"].as_array().into_iter().flatten() {
            if check["state"] == "UNKNOWN" {
                out.push((
                    node["id"].as_str().unwrap().to_owned(),
                    check["check"].as_str().unwrap().to_owned(),
                ));
            }
        }
        for child in node["children"].as_array().into_iter().flatten() {
            unknown_checks(child, out);
        }
    }
    let mut expected = Vec::new();
    for node in envelope["data"]["tree"].as_array().unwrap() {
        unknown_checks(node, &mut expected);
    }
    assert!(!expected.is_empty(), "fixture must yield an UNKNOWN check");
    for (subject, check) in expected {
        assert!(
            pending
                .iter()
                .any(|item| item["subject"] == subject.as_str()
                    && item["kind"] == "unknown"
                    && item["check"]["name"] == check.as_str()),
            "{subject} {check} not in {pending:?}"
        );
    }
}
