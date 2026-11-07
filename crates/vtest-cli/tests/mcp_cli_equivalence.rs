//! DS-1563 over the real transport (#178): the same input gives the same
//! `ok` / `data` / `diagnostics` from the CLI and from `vtest mcp`
//! `tools/call`, and a scan error elsewhere in the project does not stop an
//! MCP operation before it applies its own checks.

use std::{
    fs,
    io::{BufRead, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

use serde_json::{json, Value};
use vtest_model::{
    DerivesFrom, DocumentFile, DocumentId, NodeSource, RootNode, SentenceNode, VoId, VoRecord,
};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A project with ROOT-001 → R-001 and VO-X deriving from `vo_derives_from`.
fn fixture(name: &str, vo_derives_from: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "vtest-mcp-cli-equivalence-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).unwrap();
    let layout = vtest_store::init_project(&root, "mcp-cli-equivalence").unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"mcp_cli_equivalence\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), "pub fn target() {}\n").unwrap();
    let source = |id: &str| NodeSource {
        doc: format!("{id}.md"),
        heading: "fixture".to_owned(),
        lines: [1, 1],
    };
    let file = DocumentFile {
        schema_version: "0.1".to_owned(),
        root: vec![RootNode {
            id: DocumentId::new("ROOT-001"),
            statement: "root".to_owned(),
            description: None,
            source: source("ROOT-001"),
        }],
        request: vec![SentenceNode {
            id: DocumentId::new("R-001"),
            statement: "request".to_owned(),
            description: None,
            derives_from: vec![DocumentId::new("ROOT-001")],
            cites: None,
            source: source("R-001"),
        }],
        require: Vec::new(),
        spec: Vec::new(),
        detailed_spec: Vec::new(),
        basic_design: Vec::new(),
        design: Vec::new(),
    };
    vtest_store::write_document_file(&layout, "fixture", &file).unwrap();
    let now = "2026-11-07T00:00:00Z".to_owned();
    vtest_store::canonical::write_vo_record(
        &layout,
        &VoRecord {
            id: VoId::new("VO-X"),
            parent: None,
            derives_from: vec![DerivesFrom {
                doc: DocumentId::new(vo_derives_from),
                anchor: None,
                note: None,
            }],
            claim: "claim".to_owned(),
            dimensions: Vec::new(),
            coverage_policy: None,
            combinations: Vec::new(),
            representative_cases: Vec::new(),
            created: now.clone(),
            updated: now,
        },
    )
    .unwrap();
    root
}

/// A Test unrelated to the approval whose target does not resolve.
fn add_unresolved_target_test(root: &Path) {
    fs::write(
        root.join("src/lib.rs"),
        "pub fn target() {}\n\n#[cfg(test)]\nmod tests {\n    \
         /// @vtest.id TEST-X\n    /// @vtest.covers VO-X\n    \
         /// @vtest.target src/lib.rs::missing\n    /// @vtest.intent fixture\n    \
         #[test]\n    fn targets_nothing() {\n        super::target();\n    }\n}\n",
    )
    .unwrap();
}

fn cli(root: &Path, args: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_vtest"))
        .arg("--project")
        .arg(root)
        .args(["--format", "json"])
        .args(args)
        .output()
        .unwrap();
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "non-JSON output ({error}): {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

/// One `tools/call` through the real `vtest mcp` stdio server; returns the
/// tool's `structuredContent` envelope.
fn mcp(root: &Path, name: &str, arguments: Value) -> Value {
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
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let responses: Vec<Value> = output
        .stdout
        .lines()
        .map(|line| serde_json::from_str(&line.unwrap()).unwrap())
        .collect();
    assert_eq!(responses.len(), 2, "{responses:?}");
    responses[1]["result"]["structuredContent"].clone()
}

fn approval_records(root: &Path) -> usize {
    fs::read_dir(root.join(".verify/approvals")).map_or(0, |entries| {
        entries
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "yaml"))
            .count()
    })
}

fn codes(envelope: &Value) -> Vec<Value> {
    envelope["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|diagnostic| diagnostic["code"].clone())
        .collect()
}

/// Drops the fields that differ between two runs (record id, time).
fn stable_data(envelope: &Value) -> Value {
    let mut data = envelope["data"].clone();
    if let Some(object) = data.as_object_mut() {
        for volatile in ["id", "approved_at"] {
            object.remove(volatile);
        }
    }
    data
}

/// @vtest.id TEST-MCP-TRANSPORT-APPROVAL-MATCHES-CLI
/// @vtest.covers VO-MCP-TOOL-CLI-EQUIVALENCE
/// @vtest.target crates/vtest-mcp/src/lib.rs::tools_call_result
/// @vtest.intent 無関係なTestに未解決target(E-SCAN-004)がある状態と、承認対象VO自身のderives_fromが存在しないdocumentを指す状態のそれぞれで、CLIのapproval createと実際のvtest mcp stdioのtools/callのapproval_createが、同じok・data・診断コード・承認記録の作成有無になることを確認する
#[test]
fn approval_create_gives_the_same_result_over_cli_and_the_mcp_transport() {
    let cli_args = [
        "approval",
        "create",
        "--subject-type",
        "vo",
        "--subject-id",
        "VO-X",
        "--state",
        "approved",
        "--approver-kind",
        "agent",
        "--approver-id",
        "bot",
        "--model",
        "m",
    ];
    let mcp_args = json!({
        "subject": {"type": "vo", "id": "VO-X"},
        "state": "approved",
        "approver": {"kind": "agent", "id": "bot", "model": "m"}
    });
    // (case, VO-X's derives_from, add the unrelated E-SCAN-004, accepted)
    for (name, derives_from, unrelated_error, accepted) in [
        ("unrelated-scan-error", "R-001", true, true),
        ("dangling-subject", "R-999", false, false),
    ] {
        let prepare = |surface: &str| {
            let root = fixture(&format!("{name}-{surface}"), derives_from);
            if unrelated_error {
                add_unresolved_target_test(&root);
                let scan = cli(&root, &["scan"]);
                assert!(
                    codes(&scan).contains(&json!("E-SCAN-004")),
                    "fixture must start with E-SCAN-004: {scan}"
                );
            }
            root
        };
        let cli_root = prepare("cli");
        let via_cli = cli(&cli_root, &cli_args);
        let mcp_root = prepare("mcp");
        let via_mcp = mcp(&mcp_root, "approval_create", mcp_args.clone());

        assert_eq!(via_cli["ok"], json!(accepted), "{name} cli: {via_cli}");
        assert_eq!(via_mcp["ok"], via_cli["ok"], "{name} mcp: {via_mcp}");
        assert_eq!(codes(&via_mcp), codes(&via_cli), "{name}");
        assert_eq!(stable_data(&via_mcp), stable_data(&via_cli), "{name}");
        assert_eq!(approval_records(&cli_root), usize::from(accepted), "{name}");
        assert_eq!(
            approval_records(&mcp_root),
            approval_records(&cli_root),
            "{name}"
        );
        fs::remove_dir_all(cli_root).ok();
        fs::remove_dir_all(mcp_root).ok();
    }
}
