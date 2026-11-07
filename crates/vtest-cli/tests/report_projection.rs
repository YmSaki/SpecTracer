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

/// Sends one `tools/call` to a real `vtest mcp` process over stdio and
/// returns the tool's structured envelope.
fn mcp_tool(root: &Path, name: &str, arguments: Value) -> Value {
    use std::io::{BufRead, BufReader, Write};
    let mut child = Command::new(env!("CARGO_BIN_EXE_vtest"))
        .arg("--project")
        .arg(root)
        .arg("mcp")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut stdin = child.stdin.take().unwrap();
        for request in [
            serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
            serde_json::json!({
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
    let response = BufReader::new(output.stdout.as_slice())
        .lines()
        .map(|line| serde_json::from_str::<Value>(&line.unwrap()).unwrap())
        .find(|response| response["id"] == 2)
        .unwrap_or_else(|| panic!("{}", String::from_utf8_lossy(&output.stderr)));
    response["result"]["structuredContent"].clone()
}

/// PR #150 review / #178: a scan error that the report itself reports (an
/// unresolved target) must not make MCP replace the report with the bare
/// scan envelope. The CLI and a real stdio `tools/call` return the same
/// `ok`, `data` and `diagnostics` for `report` and for `verify`.
/// @vtest.id TEST-REPORT-MCP-TRANSPORT-KEEPS-REPORT-ON-SCAN-ERRORS
/// @vtest.covers VO-MCP-TOOL-CLI-EQUIVALENCE, VO-REPORT-CLI-MCP-PARITY
/// @vtest.target crates/vtest-mcp/src/lib.rs::tools_call_result
/// @vtest.intent 未解決targetを含むプロジェクトで、CLIのreport/verifyと実stdio MCPのtools/callが同じok・data・diagnosticsを返し、MCPがscanの件数だけを返さないことを確認する
#[test]
fn mcp_report_over_stdio_matches_the_cli_when_the_scan_has_errors() {
    let root = fixture("mcp-unresolved");
    fs::write(
        root.join("tests/report.rs"),
        TESTS.replace(
            "@vtest.target src/lib.rs::sub",
            "@vtest.target src/lib.rs::missing",
        ),
    )
    .unwrap();

    for (tool, cli, arguments) in [
        (
            "report",
            vec!["report", "--from", "TEST-A", "--direction", "down"],
            serde_json::json!({"from": "TEST-A", "direction": "down"}),
        ),
        ("report", vec!["report"], serde_json::json!({})),
        ("verify", vec!["verify"], serde_json::json!({})),
    ] {
        let (code, cli) = vtest(&root, &cli);
        assert_ne!(code, 0, "an unresolved target is not PASS: {cli}");
        assert!(
            cli["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|diagnostic| diagnostic["code"] == "E-SCAN-004"),
            "{cli}"
        );
        let mcp = mcp_tool(&root, tool, arguments.clone());
        for key in ["ok", "data", "diagnostics"] {
            assert_eq!(cli[key], mcp[key], "{tool} {arguments} {key}: {mcp}");
        }
        assert!(mcp["data"]["state"].is_string(), "{tool}: {mcp}");
        if tool == "report" {
            for key in ["tree", "pending", "projection"] {
                assert!(!mcp["data"][key].is_null(), "{tool} {key}: {mcp}");
            }
        }
    }
    fs::remove_dir_all(&root).ok();
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
        "rust-cargo::src/lib.rs::add",
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

/// PR #150 review: a Source node is keyed by its canonical Locator *with*
/// its adapter. Two adapters' Sources that share a locator value stay two
/// nodes, each with its own `targets` edge, instead of one overwriting the
/// other; a bare `--from` value that both share is rejected as ambiguous.
/// @vtest.id TEST-REPORT-SRC-NODES-KEYED-BY-ADAPTER
/// @vtest.covers VO-REPORT-SRC-NODE-IS-CANONICAL-LOCATOR
/// @vtest.target crates/vtest-cli/src/ops/report.rs::project
/// @vtest.intent adapterだけが異なる同値Locatorの2 Sourceが別ノードとして残り、各Testからのtargets辺が独立に残ること、共有する素のlocator値の--fromは曖昧として拒否し、adapter付きidでは辿れることを確認する
#[test]
fn sources_of_two_adapters_with_the_same_locator_value_stay_distinct() {
    let root = fixture("mixed-adapters");
    let registry = vtest_cli::adapters::builtin_registry().unwrap();
    let mut scan = vtest_scan::scan_project(&root, &registry).unwrap();
    let rust = scan
        .sources
        .iter()
        .find(|source| source.locator.value == "src/lib.rs::add")
        .cloned()
        .expect("rust-cargo Source for add");
    let mut other = rust.clone();
    other.locator.adapter = vtest_model::AdapterId::new("synthetic");
    scan.sources.push(other.clone());
    let mut test = scan
        .tests
        .iter()
        .find(|test| test.id.as_str() == "TEST-B")
        .cloned()
        .unwrap();
    test.id = vtest_model::TestId::new("TEST-SYNTHETIC");
    test.targets = vec![vtest_model::TargetRef::Locator(other.locator.clone())];
    scan.tests.push(test);

    let layout = VerifyLayout::new(&root);
    let args = |from: Option<&str>| vtest_cli::ops::report::ProjectionArgs {
        from: from.map(str::to_owned),
        view: None,
        depth: from.map(|_| 1),
        direction: None,
    };
    let projection = serde_json::to_value(
        vtest_cli::ops::report::project(&layout, &scan, &[], &args(None)).unwrap(),
    )
    .unwrap();
    let ids = node_ids(&projection);
    for id in ["rust-cargo::src/lib.rs::add", "synthetic::src/lib.rs::add"] {
        assert!(ids.contains(&id.to_owned()), "{id} missing from {ids:?}");
    }
    find_edge(
        &projection,
        "TEST-A",
        "targets",
        "rust-cargo::src/lib.rs::add",
    );
    find_edge(
        &projection,
        "TEST-SYNTHETIC",
        "targets",
        "synthetic::src/lib.rs::add",
    );

    let ambiguous =
        vtest_cli::ops::report::project(&layout, &scan, &[], &args(Some("src/lib.rs::add")));
    assert!(
        ambiguous.is_err(),
        "a value two adapters share is ambiguous"
    );
    let from_synthetic = serde_json::to_value(
        vtest_cli::ops::report::project(
            &layout,
            &scan,
            &[],
            &args(Some("synthetic::src/lib.rs::add")),
        )
        .unwrap(),
    )
    .unwrap();
    let ids = node_ids(&from_synthetic);
    assert!(ids.contains(&"TEST-SYNTHETIC".to_owned()), "{ids:?}");
    assert!(!ids.contains(&"TEST-A".to_owned()), "{ids:?}");
    fs::remove_dir_all(&root).ok();
}

/// PR #150 review: a permanent SRC ID that two Sources declare is a
/// collision (E-SCAN-011), and the TargetRef naming it stays unresolved
/// (DS-520). The projection draws no `targets` edge for it, whichever order
/// the scan lists the colliding Sources in, and still shows both Sources.
/// @vtest.id TEST-REPORT-SRC-ID-COLLISION-DRAWS-NO-TARGETS-EDGE
/// @vtest.covers VO-REPORT-SRC-ID-COLLISION-LEAVES-TARGET-UNRESOLVED
/// @vtest.target crates/vtest-cli/src/ops/report.rs::project
/// @vtest.intent 2つのSourceが同じ@vtest.src-idを宣言し、TestがそのSRC IDをtargetにしたとき、E-SCAN-011を保ったままprojectionが先頭Sourceへのtargets辺を作らず、Source順を入れ替えても投影が変わらないことを確認する
#[test]
fn a_duplicated_src_id_draws_no_targets_edge() {
    let root = fixture("src-id-collision");
    fs::write(
        root.join("src/lib.rs"),
        "/// @vtest.src-id SRC-X\n\
         pub fn add(a: i32, b: i32) -> i32 { a + b }\n\
         /// @vtest.src-id SRC-X\n\
         pub fn sub(a: i32, b: i32, out: &mut i32) { *out = a - b; }\n",
    )
    .unwrap();
    fs::write(
        root.join("tests/report.rs"),
        TESTS.replace("@vtest.target src/lib.rs::add", "@vtest.target SRC-X"),
    )
    .unwrap();

    let registry = vtest_cli::adapters::builtin_registry().unwrap();
    let mut scan = vtest_scan::scan_project(&root, &registry).unwrap();
    assert!(
        scan.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-SCAN-011"),
        "the collision keeps its diagnostic: {:?}",
        scan.diagnostics
    );
    let layout = VerifyLayout::new(&root);
    let args = vtest_cli::ops::report::ProjectionArgs {
        from: None,
        view: None,
        depth: None,
        direction: None,
    };
    let mut projected = || {
        let projection = serde_json::to_value(
            vtest_cli::ops::report::project(&layout, &scan, &[], &args).unwrap(),
        )
        .unwrap();
        scan.sources.reverse();
        projection
    };
    let first = projected();
    let reversed = projected();
    for projection in [&first, &reversed] {
        let ids = node_ids(projection);
        for id in ["rust-cargo::src/lib.rs::add", "rust-cargo::src/lib.rs::sub"] {
            assert!(ids.contains(&id.to_owned()), "{id} missing from {ids:?}");
        }
        let edges = projection["edges"].as_array().unwrap();
        assert!(
            !edges
                .iter()
                .any(|edge| edge["from"] == "TEST-A" && edge["relation"] == "targets"),
            "an unresolved SRC ID gives TEST-A no targets edge: {projection}"
        );
        find_edge(
            projection,
            "TEST-B",
            "targets",
            "rust-cargo::src/lib.rs::sub",
        );
    }
    assert_eq!(
        first, reversed,
        "Source order does not change the projection"
    );
    fs::remove_dir_all(&root).ok();
}

/// PR #150 review: `report.evidence` shows the latest Evidence chosen by
/// verify's rule (#132). A newer invalid record next to an older valid PASS
/// is shown as invalid; the older PASS is never offered as the basis.
/// @vtest.id TEST-REPORT-EVIDENCE-SAME-SELECTION-AS-VERIFY
/// @vtest.covers VO-REPORT-SAME-AGGREGATION-AS-VERIFY
/// @vtest.target crates/vtest-cli/src/ops/report.rs::execute
/// @vtest.intent 新しい不正Evidenceと古い正常PASSがあるとき、reportのevidence参照が古いPASSへ戻らず不正な最新を示し、状態・diagnosticsもverifyと一致することを確認する
#[test]
fn report_evidence_does_not_fall_back_to_an_older_pass() {
    let root = fixture("invalid-latest");
    // Cargo would otherwise write Cargo.lock during the run, which changes
    // the Execution State mid-run (E-EXEC-004).
    let lock = Command::new("cargo")
        .args(["generate-lockfile", "--offline"])
        .current_dir(&root)
        .status()
        .unwrap();
    assert!(lock.success());
    let (code, run) = vtest(&root, &["run", "--test", "TEST-A", "--fast"]);
    assert_eq!(code, 0, "{run}");
    let (_, control) = vtest(&root, &["report"]);
    let evidence = control["data"]["evidence"].as_array().unwrap();
    let valid = evidence
        .iter()
        .find(|entry| entry["test"] == "TEST-A")
        .unwrap_or_else(|| panic!("{control}"));
    assert_eq!(valid["result"], "PASS", "{valid}");
    let valid_id = valid["evidence"].as_str().unwrap().to_owned();

    let dir = root.join(".verify").join("evidence");
    let text = fs::read_to_string(dir.join(format!("{valid_id}.yaml"))).unwrap();
    let later_id = "7ZZZZZZZZZZZZZZZZZZZZZZZZZ";
    fs::write(
        dir.join(format!("{later_id}.yaml")),
        format!("{}unexpected: true\n", text.replace(&valid_id, later_id)),
    )
    .unwrap();

    let (verify_code, verify) = vtest(&root, &["verify"]);
    let (report_code, report) = vtest(&root, &["report"]);
    assert_eq!(verify_code, report_code);
    for key in ["state", "result", "tree"] {
        assert_eq!(verify["data"][key], report["data"][key], "{key}");
    }
    assert_eq!(verify["diagnostics"], report["diagnostics"]);
    assert!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| diagnostic["code"] == "E-SCAN-010"),
        "{report}"
    );
    let latest = report["data"]["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["test"] == "TEST-A")
        .unwrap_or_else(|| panic!("{report}"))
        .clone();
    assert_eq!(latest["evidence"], later_id, "{latest}");
    assert!(latest["result"].is_null(), "{latest}");
    assert!(latest["invalid"].is_string(), "{latest}");
    assert_ne!(
        latest["evidence"],
        valid_id.as_str(),
        "no fall back to the older PASS"
    );
    fs::remove_dir_all(&root).ok();
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
