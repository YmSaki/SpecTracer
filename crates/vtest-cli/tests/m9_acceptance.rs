//! M9 MCP stdio acceptance coverage.

use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{json, Value};

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
            "vtest-cli-m9-{name}-{}-{nanos}-{sequence}",
            std::process::id()
        ));
        copy_tree(&fixture_path("m1/base"), &root);
        for directory in ["approvals", "audits", "evidence", "rel"] {
            fs::create_dir_all(root.join(".verify").join(directory))
                .expect("restore canonical record directory");
        }
        Self { root }
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

fn mcp_requests(project: &Path, requests: &[Value]) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_vtest"))
        .args(["--project", project.to_str().unwrap(), "mcp"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn vtest MCP server");
    {
        let stdin = child.stdin.as_mut().expect("MCP stdin");
        for request in requests {
            serde_json::to_writer(&mut *stdin, request).expect("write MCP request");
            stdin.write_all(b"\n").expect("write MCP request newline");
        }
    }
    let output = child.wait_with_output().expect("wait for MCP server");
    assert!(
        output.status.success(),
        "MCP server failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).expect("MCP response is JSON"))
        .collect()
}

fn direct_cli(project: &Path, command: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_vtest"))
        .args(["--project", project.to_str().unwrap(), "--format", "json"])
        .args(command)
        .output()
        .expect("run CLI command");
    serde_json::from_slice(&output.stdout).expect("CLI response is JSON")
}

#[test]
fn m9_stdio_lists_tools_and_preserves_cli_scan_envelope() {
    let project = TempProject::from_m1_base("protocol");
    let responses = mcp_requests(
        &project.root,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
            json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"scan","arguments":{}}}),
        ],
    );
    assert_eq!(responses.len(), 3);
    assert_eq!(responses[0]["result"]["serverInfo"]["name"], "vtest");
    let tools = responses[1]["result"]["tools"]
        .as_array()
        .expect("tools/list returns tools");
    for name in [
        "scan",
        "spec_list",
        "req_upsert",
        "vo_approve",
        "form_get",
        "test_create",
        "audit_submit",
        "run_tests",
        "verify",
        "report",
    ] {
        assert!(
            tools.iter().any(|tool| tool["name"] == name),
            "missing tool {name}"
        );
    }
    let mcp_scan = &responses[2]["result"]["structuredContent"];
    assert_eq!(mcp_scan, &direct_cli(&project.root, &["scan"]));
    assert_eq!(responses[2]["result"]["isError"], false);
}

#[test]
fn m9_reference_flow_reaches_existing_cli_operations() {
    let project = TempProject::from_m1_base("flow");
    let answers = json!({
        "target": "src/lib.rs::known",
        "covers": ["VO-KNOWN"],
        "behavior": "MCP generated behavior",
        "test_kind": "normal",
        "input": "ordinary input",
        "expect": "expected result",
        "fn_name": "mcp_generated_test"
    });
    let responses = mcp_requests(
        &project.root,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"form_get","arguments":{"kind":"rust-unit-function"}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"test_create","arguments":{"form":"rust-unit-function","answers":answers,"id":"TEST-M9-CREATED","dry_run":true}}}),
            json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"test_create","arguments":{"form":"rust-unit-function","answers":answers,"id":"TEST-M9-CREATED"}}}),
            json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"test_query","arguments":{"source":"src/lib.rs::known"}}}),
            json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"audit_static","arguments":{"test":"TEST-M9-CREATED"}}}),
            json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"run_tests","arguments":{"test":"TEST-M9-CREATED","fast":true}}}),
        ],
    );
    assert_eq!(responses.len(), 6);
    assert_eq!(responses[0]["result"]["structuredContent"]["ok"], true);
    assert_eq!(
        responses[1]["result"]["structuredContent"]["data"]["dry_run"],
        true
    );
    assert_eq!(responses[2]["result"]["structuredContent"]["ok"], true);
    assert!(responses[3]["result"]["structuredContent"]["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|test| test["id"] == "TEST-M9-CREATED"));
    assert!(responses[4]["result"]["structuredContent"]["data"].is_object());
    assert!(responses[5]["result"]["structuredContent"]["data"].is_object());
}

#[test]
fn m9_invalid_tool_input_returns_error_code_message_and_candidates() {
    let project = TempProject::from_m1_base("errors");
    let response = mcp_requests(
        &project.root,
        &[json!({
            "jsonrpc":"2.0",
            "id":1,
            "method":"tools/call",
            "params":{
                "name":"test_create",
                "arguments":{
                    "form":"rust-unit-function",
                    "answers":{
                        "target":"src/lib.rs::knwon",
                        "covers":["VO-KNOWN"],
                        "behavior":"bad target",
                        "test_kind":"normal",
                        "input":"ordinary",
                        "expect":"result",
                        "fn_name":"bad_mcp_test"
                    },
                    "id":"TEST-M9-BAD",
                    "dry_run":true
                }
            }
        })],
    );
    let result = &response[0]["result"];
    assert_eq!(result["isError"], true);
    let envelope = &result["structuredContent"];
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["diagnostics"][0]["code"], "E-OP-001");
    assert_eq!(
        envelope["diagnostics"][0]["candidates"][0],
        "src/lib.rs::known"
    );
}
