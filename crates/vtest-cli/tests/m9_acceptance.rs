//! M9 MCP stdio acceptance coverage.

use std::{
    env, fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
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
    let lines = requests
        .iter()
        .map(|request| serde_json::to_string(request).expect("serialize MCP request"))
        .collect::<Vec<_>>();
    mcp_lines(project, &lines)
}

fn mcp_lines(project: &Path, lines: &[String]) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_vtest"))
        .args(["--project", project.to_str().unwrap(), "mcp"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn vtest MCP server");
    {
        let stdin = child.stdin.as_mut().expect("MCP stdin");
        for line in lines {
            stdin.write_all(line.as_bytes()).expect("write MCP request");
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

fn mcp_form_get_with_mtime_change(project: &Path, form_path: &Path) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_vtest"))
        .args(["--project", project.to_str().unwrap(), "mcp"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn interactive MCP server");
    let mut stdin = child.stdin.take().expect("MCP stdin");
    let stdout = child.stdout.take().expect("MCP stdout");
    let mut reader = BufReader::new(stdout);
    let request = |id| {
        serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {"name": "form_get", "arguments": {"kind": "rust-unit-function"}}
        }))
        .expect("serialize form_get request")
    };
    let first_request = request(1);
    stdin
        .write_all(first_request.as_bytes())
        .expect("write first form_get request");
    stdin.write_all(b"\n").expect("write first request newline");
    stdin.flush().expect("flush first form_get request");
    let mut first = String::new();
    reader
        .read_line(&mut first)
        .expect("read first form_get response");

    thread::sleep(Duration::from_millis(20));
    fs::write(
        form_path,
        "kind: rust-unit-function\ntitle: second form title\nfields:\n  - name: target\n    question: target\n    type: string\n    required: true\ntemplate: |\n  body\n",
    )
    .expect("rewrite form schema");
    let second_request = request(2);
    stdin
        .write_all(second_request.as_bytes())
        .expect("write second form_get request");
    stdin
        .write_all(b"\n")
        .expect("write second request newline");
    stdin.flush().expect("flush second form_get request");
    let mut second = String::new();
    reader
        .read_line(&mut second)
        .expect("read second form_get response");
    drop(stdin);
    let status = child.wait().expect("wait for interactive MCP server");
    assert!(status.success(), "interactive MCP server must exit cleanly");
    [first, second]
        .into_iter()
        .map(|line| serde_json::from_str(&line).expect("interactive MCP response is JSON"))
        .collect()
}

fn mcp_call(project: &Path, name: &str, arguments: Value) -> Value {
    let response = mcp_requests(
        project,
        &[json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {"name": name, "arguments": arguments}
        })],
    );
    response.into_iter().next().expect("one MCP response")
}

fn mcp_envelope(response: &Value) -> &Value {
    &response["result"]["structuredContent"]
}

fn assert_mcp_ok<'a>(response: &'a Value, context: &str) -> &'a Value {
    assert_eq!(
        response["result"]["isError"], false,
        "{context}: {response}"
    );
    let envelope = mcp_envelope(response);
    assert_eq!(envelope["ok"], true, "{context}: {response}");
    envelope
}

fn assert_mcp_failure<'a>(response: &'a Value, code: &str, context: &str) -> &'a Value {
    assert_eq!(response["result"]["isError"], true, "{context}: {response}");
    let envelope = mcp_envelope(response);
    assert_eq!(envelope["ok"], false, "{context}: {response}");
    assert_eq!(
        envelope["diagnostics"][0]["code"], code,
        "{context}: {response}"
    );
    assert!(
        envelope["diagnostics"][0]["message"].is_string(),
        "{context}: {response}"
    );
    envelope
}

fn assert_equivalent_cli(mcp: &Value, cli: &Value, context: &str) {
    assert_eq!(
        mcp["result"]["isError"],
        cli["ok"] != Value::Bool(true),
        "{context}: error flag mismatch; mcp={mcp}; cli={cli}"
    );
    let mut mcp_normalized = mcp_envelope(mcp).clone();
    let mut cli_normalized = cli.clone();
    normalize_dynamic(&mut mcp_normalized, None);
    normalize_dynamic(&mut cli_normalized, None);
    assert_eq!(
        mcp_normalized, cli_normalized,
        "{context}: MCP/CLI values differ"
    );
}

fn normalize_dynamic(value: &mut Value, key: Option<&str>) {
    match value {
        Value::Object(object) => {
            for (child_key, child) in object {
                if matches!(
                    child_key.as_str(),
                    "created"
                        | "updated"
                        | "approved_at"
                        | "executed_at"
                        | "audited_at"
                        | "generated_at"
                        | "recorded_at"
                ) && child.is_string()
                {
                    *child = json!("<dynamic-time>");
                    continue;
                }
                normalize_dynamic(child, Some(child_key));
            }
        }
        Value::Array(items) => {
            for item in items {
                normalize_dynamic(item, key);
            }
        }
        Value::String(text) => {
            if (key.is_some_and(|key| key.ends_with("_id") || key == "id")) && is_ulid(text) {
                *text = "<dynamic-id>".to_owned();
            } else {
                *text = normalize_ulid_tokens(text);
            }
        }
        _ => {}
    }
}

fn is_ulid(value: &str) -> bool {
    value.len() == 26
        && value
            .bytes()
            .all(|byte| b"0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(&byte))
}

fn normalize_ulid_tokens(value: &str) -> String {
    let mut output = String::new();
    let mut token = String::new();
    for character in value.chars().chain(std::iter::once('\0')) {
        if character.is_ascii_uppercase() || character.is_ascii_digit() {
            token.push(character);
        } else {
            if is_ulid(&token) {
                output.push_str("<dynamic-id>");
            } else {
                output.push_str(&token);
            }
            token.clear();
            if character != '\0' {
                output.push(character);
            }
        }
    }
    output
}

fn direct_cli(project: &Path, command: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_vtest"))
        .args(["--project", project.to_str().unwrap(), "--format", "json"])
        .args(command)
        .output()
        .expect("run CLI command");
    serde_json::from_slice(&output.stdout).expect("CLI response is JSON")
}

fn prepare_spec_fixture(project: &TempProject) {
    fs::create_dir_all(project.root.join("docs")).expect("create M9 specification directory");
    fs::write(
        project.root.join("docs/m9-spec.md"),
        "# M9 specification\n\nThe MCP flow has a traceable specification.\n",
    )
    .expect("write M9 specification document");
    let response = direct_cli(
        &project.root,
        &[
            "spec",
            "add",
            "--id",
            "SPEC-M9-FLOW",
            "--path",
            "docs/m9-spec.md",
            "--kind",
            "document",
        ],
    );
    assert_eq!(
        response["ok"], true,
        "register M9 specification: {response}"
    );
}

fn generated_test_answers(id: &str) -> Value {
    json!({
        "target": "src/lib.rs::known",
        "covers": ["VO-M9-FLOW"],
        "behavior": "MCP generated behavior",
        "test_kind": "normal",
        "input": "ordinary input",
        "expect": "expected result",
        "fn_name": id
    })
}

/// @vtest.id TEST-CLI-089
/// @vtest.covers VO-CLI-018
/// @vtest.target crates/vtest-cli/src/lib.rs::run_mcp
/// @vtest.intent MCP initialize/tools.list advertises all tools and scan matches CLI envelope
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
    let expected_tools = [
        "scan",
        "spec_list",
        "spec_get",
        "req_list",
        "req_get",
        "req_upsert",
        "vo_list",
        "vo_get",
        "vo_upsert",
        "vo_expand",
        "vo_approve",
        "test_query",
        "test_get",
        "form_get",
        "test_create",
        "test_edit",
        "audit_static",
        "audit_bundle",
        "audit_submit",
        "run_tests",
        "verify",
        "report",
    ];
    let listed_names = tools
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        listed_names, expected_tools,
        "tools/list must be complete and deterministic"
    );
    for name in expected_tools {
        assert!(
            tools.iter().any(|tool| tool["name"] == name),
            "missing tool {name}"
        );
    }
    assert_eq!(tools[0]["inputSchema"]["additionalProperties"], false);
    assert_eq!(
        tools
            .iter()
            .find(|tool| tool["name"] == "test_create")
            .unwrap()["inputSchema"]["required"],
        json!(["form", "answers"])
    );
    let mcp_scan = &responses[2]["result"]["structuredContent"];
    assert_eq!(mcp_scan, &direct_cli(&project.root, &["scan"]));
    assert_eq!(responses[2]["result"]["isError"], false);
}

/// @vtest.id TEST-CLI-090
/// @vtest.covers VO-CLI-018
/// @vtest.target crates/vtest-cli/src/lib.rs::run_mcp
/// @vtest.intent JSON-RPC notifications yield no response and don't consume next request
#[test]
fn m9_notifications_are_silent_and_do_not_consume_following_requests() {
    let project = TempProject::from_m1_base("notifications");
    let responses = mcp_requests(
        &project.root,
        &[
            json!({"jsonrpc":"2.0","method":"tools/list","params":{}}),
            json!({"jsonrpc":"2.0","id":7,"method":"ping","params":{}}),
        ],
    );
    assert_eq!(
        responses.len(),
        1,
        "notification must not produce a response"
    );
    assert_eq!(responses[0]["id"], 7);
    assert_eq!(responses[0]["result"], json!({}));
}

/// @vtest.id TEST-CLI-091
/// @vtest.covers VO-CLI-018
/// @vtest.target crates/vtest-cli/src/lib.rs::run_mcp
/// @vtest.intent form_get reloads schema after form file mtime changes mid-session
#[test]
fn m9_form_get_refreshes_after_form_mtime_changes_in_one_server_session() {
    let project = TempProject::from_m1_base("form-mtime");
    let forms = project.root.join(".verify/forms");
    fs::create_dir_all(&forms).expect("create form directory");
    let form_path = forms.join("rust-unit-function.yaml");
    fs::write(
        &form_path,
        "kind: rust-unit-function\ntitle: first form title\nfields:\n  - name: target\n    question: target\n    type: string\n    required: true\ntemplate: |\n  body\n",
    )
    .expect("write initial form schema");

    let responses = mcp_form_get_with_mtime_change(&project.root, &form_path);
    assert_eq!(responses.len(), 2);
    assert_eq!(
        mcp_envelope(&responses[0])["data"]["title"],
        "first form title"
    );
    assert_eq!(
        mcp_envelope(&responses[1])["data"]["title"],
        "second form title"
    );
}

/// @vtest.id TEST-CLI-092
/// @vtest.covers VO-CLI-018
/// @vtest.target crates/vtest-cli/src/lib.rs::run_mcp
/// @vtest.intent Every advertised MCP tool produces envelope equivalent to CLI output
#[test]
fn m9_all_advertised_tools_match_cli_envelopes() {
    let project = TempProject::from_m1_base("parity");
    prepare_spec_fixture(&project);
    let cli_project = TempProject::from_m1_base("parity-cli");
    prepare_spec_fixture(&cli_project);

    let req_args = json!({
        "id": "REQ-M9-PARITY",
        "summary": "MCP parity requirement",
        "specs": ["SPEC-M9-FLOW"],
        "sections": ["1"]
    });
    let req_write = mcp_call(&project.root, "req_upsert", req_args.clone());
    assert_mcp_ok(&req_write, "req_upsert");
    assert_equivalent_cli(
        &req_write,
        &direct_cli(
            &cli_project.root,
            &[
                "req",
                "add",
                "--id",
                "REQ-M9-PARITY",
                "--summary",
                "MCP parity requirement",
                "--spec",
                "SPEC-M9-FLOW",
                "--sections",
                "1",
            ],
        ),
        "req_upsert",
    );

    let vo_args = json!({
        "id": "VO-M9-PARITY",
        "claim": "MCP parity verification object",
        "requirements": ["REQ-M9-PARITY"],
        "specs": ["SPEC-M9-FLOW"],
        "sections": ["1"]
    });
    let vo_write = mcp_call(&project.root, "vo_upsert", vo_args);
    assert_mcp_ok(&vo_write, "vo_upsert");
    assert_equivalent_cli(
        &vo_write,
        &direct_cli(
            &cli_project.root,
            &[
                "vo",
                "add",
                "--id",
                "VO-M9-PARITY",
                "--claim",
                "MCP parity verification object",
                "--req",
                "REQ-M9-PARITY",
                "--spec",
                "SPEC-M9-FLOW",
                "--sections",
                "1",
            ],
        ),
        "vo_upsert",
    );

    let read_cases = [
        ("scan", json!({}), vec!["scan"]),
        ("spec_list", json!({}), vec!["spec", "list"]),
        (
            "spec_get",
            json!({"id": "SPEC-M9-FLOW"}),
            vec!["spec", "show", "SPEC-M9-FLOW"],
        ),
        (
            "req_list",
            json!({"tree": true}),
            vec!["req", "list", "--tree"],
        ),
        (
            "req_get",
            json!({"id": "REQ-M9-PARITY"}),
            vec!["req", "show", "REQ-M9-PARITY"],
        ),
        (
            "vo_list",
            json!({"status": "draft"}),
            vec!["vo", "list", "--status", "draft"],
        ),
        (
            "vo_get",
            json!({"id": "VO-M9-PARITY"}),
            vec!["vo", "show", "VO-M9-PARITY"],
        ),
        (
            "test_query",
            json!({"source": "src/lib.rs::known"}),
            vec!["test", "query", "--source", "src/lib.rs::known"],
        ),
        (
            "test_get",
            json!({"id": "TEST-M1-CLEAN"}),
            vec!["test", "show", "TEST-M1-CLEAN"],
        ),
    ];
    for (name, arguments, command) in read_cases {
        let mcp = mcp_call(&project.root, name, arguments);
        let cli_args = command.to_vec();
        assert_equivalent_cli(&mcp, &direct_cli(&project.root, &cli_args), name);
    }

    let form = mcp_call(
        &project.root,
        "form_get",
        json!({"kind": "rust-unit-function"}),
    );
    let form_envelope = assert_mcp_ok(&form, "form_get");
    assert_eq!(form_envelope["data"]["kind"], "rust-unit-function");
    assert!(form_envelope["data"]["fields"].is_array());

    let mut parity_answers = generated_test_answers("mcp_parity_dry");
    parity_answers["covers"] = json!(["VO-M9-PARITY"]);
    let dry_create_args = json!({
        "form": "rust-unit-function",
        "answers": parity_answers,
        "id": "TEST-M9-DRY",
        "dry_run": true
    });
    fs::write(
        project.root.join("m9-parity-answers.yaml"),
        "form: 'rust-unit-function'\nanswers:\n  target: 'src/lib.rs::known'\n  covers:\n    - 'VO-M9-PARITY'\n  behavior: 'MCP generated behavior'\n  test_kind: 'normal'\n  input: 'ordinary input'\n  expect: 'expected result'\n  fn_name: 'mcp_parity_dry'\n",
    )
    .expect("write CLI parity answers");
    let dry_create = mcp_call(&project.root, "test_create", dry_create_args.clone());
    assert_equivalent_cli(
        &dry_create,
        &direct_cli(
            &project.root,
            &[
                "test",
                "create",
                "--form",
                "rust-unit-function",
                "--answers",
                "m9-parity-answers.yaml",
                "--id",
                "TEST-M9-DRY",
                "--dry-run",
            ],
        ),
        "test_create",
    );

    let dry_edit_args = json!({
        "id": "TEST-M1-CLEAN",
        "set": {"intent": "MCP parity dry run"},
        "dry_run": true
    });
    let dry_edit = mcp_call(&project.root, "test_edit", dry_edit_args);
    assert_mcp_ok(&dry_edit, "test_edit dry_run");
    assert_eq!(
        dry_edit["result"]["structuredContent"]["data"]["dry_run"],
        true
    );

    let vo_expand_args = json!({
        "id": "VO-M9-EXPAND",
        "claim": "expansion parity",
        "dimensions": ["region=us,eu"],
        "policy": "full-product"
    });
    assert_mcp_ok(
        &mcp_call(&project.root, "vo_upsert", vo_expand_args),
        "vo_upsert expansion fixture",
    );
    let expand = mcp_call(
        &project.root,
        "vo_expand",
        json!({"id": "VO-M9-EXPAND", "dry_run": true}),
    );
    assert_equivalent_cli(
        &expand,
        &direct_cli(
            &project.root,
            &["vo", "expand", "VO-M9-EXPAND", "--dry-run"],
        ),
        "vo_expand",
    );

    let approval = mcp_call(
        &project.root,
        "vo_approve",
        json!({
            "id": "VO-M9-PARITY",
            "approver": {"kind": "agent", "id": "m9-parity"},
            "basis": []
        }),
    );
    assert_mcp_ok(&approval, "vo_approve");
    assert_equivalent_cli(
        &approval,
        &direct_cli(
            &project.root,
            &[
                "vo",
                "approve",
                "VO-M9-PARITY",
                "--approver-kind",
                "agent",
                "--approver-id",
                "m9-parity",
            ],
        ),
        "vo_approve",
    );

    let static_audit = mcp_call(
        &project.root,
        "audit_static",
        json!({"test": "TEST-M1-CLEAN"}),
    );
    assert_equivalent_cli(
        &static_audit,
        &direct_cli(
            &project.root,
            &["audit", "static", "--test", "TEST-M1-CLEAN"],
        ),
        "audit_static",
    );

    let bundle = mcp_call(
        &project.root,
        "audit_bundle",
        json!({
            "kind": "test-semantic",
            "test": "TEST-M1-CLEAN",
            "include_failed": true
        }),
    );
    let bundle_data = assert_mcp_ok(&bundle, "audit_bundle")["data"].clone();
    let bundle_id = bundle_data["bundle_id"]
        .as_str()
        .expect("bundle id")
        .to_owned();
    assert_equivalent_cli(
        &bundle,
        &direct_cli(
            &project.root,
            &[
                "audit",
                "bundle",
                "--kind",
                "test-semantic",
                "--test",
                "TEST-M1-CLEAN",
                "--include-failed",
            ],
        ),
        "audit_bundle",
    );

    let submission = json!({
        "bundle_id": bundle_id,
        "kind": "test-semantic",
        "verdict": "PASS",
        "reasons": [{
            "claim": "the MCP parity flow has a semantic basis",
            "basis": [{"kind": "test-code", "ref": "tests/registered.rs::clean_scan_baseline"}]
        }],
        "exclusions": [],
        "auditor": {"kind": "agent", "id": "m9-parity", "model": "test"},
        "confidence": "high"
    });
    let submission_path = project.root.join("m9-parity-submission.json");
    fs::write(
        &submission_path,
        serde_json::to_vec_pretty(&submission).expect("serialize CLI parity submission"),
    )
    .expect("write CLI parity submission");
    let submission_path = submission_path.to_string_lossy().into_owned();
    let submitted = mcp_call(
        &project.root,
        "audit_submit",
        json!({"submission": submission}),
    );
    assert_mcp_ok(&submitted, "audit_submit");
    assert_equivalent_cli(
        &submitted,
        &direct_cli(
            &project.root,
            &["audit", "submit", "--file", &submission_path],
        ),
        "audit_submit",
    );

    let run = mcp_call(
        &project.root,
        "run_tests",
        json!({"test": "TEST-M1-CLEAN", "fast": true}),
    );
    assert_equivalent_cli(
        &run,
        &direct_cli(&project.root, &["run", "--test", "TEST-M1-CLEAN", "--fast"]),
        "run_tests",
    );
    for name in ["verify", "report"] {
        let mcp = mcp_call(
            &project.root,
            name,
            json!({"test": "TEST-M1-CLEAN", "items": ["test_existence"]}),
        );
        let cli = direct_cli(
            &project.root,
            &[name, "--test", "TEST-M1-CLEAN", "--items", "test_existence"],
        );
        assert_equivalent_cli(&mcp, &cli, name);
    }
}

/// @vtest.id TEST-CLI-093
/// @vtest.covers VO-CLI-018
/// @vtest.target crates/vtest-cli/src/lib.rs::run_mcp
/// @vtest.intent req/vo upsert edits supported fields and fail-closed rejects unsupported updates
#[test]
fn m9_existing_record_upserts_edit_supported_fields_and_reject_unsupported_updates() {
    let project = TempProject::from_m1_base("upsert-edit");
    prepare_spec_fixture(&project);

    assert_mcp_ok(
        &mcp_call(
            &project.root,
            "req_upsert",
            json!({
                "id": "REQ-M9-EDIT",
                "summary": "initial requirement",
                "specs": ["SPEC-M9-FLOW"],
                "sections": ["1"]
            }),
        ),
        "create editable REQ",
    );
    assert_mcp_ok(
        &mcp_call(
            &project.root,
            "req_upsert",
            json!({
                "id": "REQ-M9-OTHER",
                "summary": "unrelated requirement"
            }),
        ),
        "create unrelated REQ",
    );
    let req_other_before = mcp_envelope(&mcp_call(
        &project.root,
        "req_get",
        json!({"id": "REQ-M9-OTHER"}),
    ))["data"]
        .clone();
    let req_initial = mcp_envelope(&mcp_call(
        &project.root,
        "req_get",
        json!({"id": "REQ-M9-EDIT"}),
    ))["data"]
        .clone();

    assert_mcp_ok(
        &mcp_call(
            &project.root,
            "req_upsert",
            json!({
                "id": "REQ-M9-EDIT",
                "summary": "updated requirement",
                "parent": "REQ-M9-OTHER"
            }),
        ),
        "edit supported REQ fields",
    );
    let req_updated = mcp_envelope(&mcp_call(
        &project.root,
        "req_get",
        json!({"id": "REQ-M9-EDIT"}),
    ))["data"]
        .clone();
    assert_eq!(req_updated["summary"], "updated requirement");
    assert_eq!(req_updated["parent"], "REQ-M9-OTHER");
    assert_eq!(req_updated["spec_refs"], req_initial["spec_refs"]);

    let req_unsupported = mcp_call(
        &project.root,
        "req_upsert",
        json!({
            "id": "REQ-M9-EDIT",
            "summary": "must not silently discard",
            "specs": ["SPEC-M9-FLOW"],
            "sections": ["2"]
        }),
    );
    assert_mcp_failure(
        &req_unsupported,
        "E-OP-001",
        "reject unsupported existing REQ fields",
    );
    assert_eq!(
        mcp_envelope(&mcp_call(
            &project.root,
            "req_get",
            json!({"id": "REQ-M9-EDIT"}),
        ))["data"],
        req_updated,
        "unsupported REQ update must not write any fields"
    );
    assert_eq!(
        mcp_envelope(&mcp_call(
            &project.root,
            "req_get",
            json!({"id": "REQ-M9-OTHER"}),
        ))["data"],
        req_other_before,
        "REQ update must not alter unrelated records"
    );

    assert_mcp_ok(
        &mcp_call(
            &project.root,
            "vo_upsert",
            json!({
                "id": "VO-M9-EDIT",
                "claim": "initial verification claim",
                "requirements": ["REQ-M9-EDIT"],
                "specs": ["SPEC-M9-FLOW"],
                "sections": ["1"],
                "dimensions": ["region=us,eu"],
                "policy": "full-product"
            }),
        ),
        "create editable VO",
    );
    let vo_other_before = mcp_envelope(&mcp_call(
        &project.root,
        "vo_get",
        json!({"id": "VO-KNOWN"}),
    ))["data"]
        .clone();
    let vo_initial = mcp_envelope(&mcp_call(
        &project.root,
        "vo_get",
        json!({"id": "VO-M9-EDIT"}),
    ))["data"]
        .clone();

    assert_mcp_ok(
        &mcp_call(
            &project.root,
            "vo_upsert",
            json!({
                "id": "VO-M9-EDIT",
                "claim": "updated verification claim",
                "parent": "VO-KNOWN"
            }),
        ),
        "edit supported VO fields",
    );
    let vo_updated = mcp_envelope(&mcp_call(
        &project.root,
        "vo_get",
        json!({"id": "VO-M9-EDIT"}),
    ))["data"]
        .clone();
    assert_eq!(vo_updated["claim"], "updated verification claim");
    assert_eq!(vo_updated["parent"], "VO-KNOWN");
    for field in [
        "requirements",
        "spec_refs",
        "dimensions",
        "coverage_policy",
        "combinations",
    ] {
        assert_eq!(vo_updated[field], vo_initial[field], "VO field {field}");
    }

    let vo_unsupported = mcp_call(
        &project.root,
        "vo_upsert",
        json!({
            "id": "VO-M9-EDIT",
            "claim": "must not silently discard",
            "requirements": ["REQ-M9-OTHER"],
            "specs": ["SPEC-M9-FLOW"],
            "sections": ["2"],
            "dimensions": ["region=apac"],
            "policy": "explicit",
            "combinations": ["region=apac"]
        }),
    );
    assert_mcp_failure(
        &vo_unsupported,
        "E-OP-001",
        "reject unsupported existing VO fields",
    );
    assert_eq!(
        mcp_envelope(&mcp_call(
            &project.root,
            "vo_get",
            json!({"id": "VO-M9-EDIT"}),
        ))["data"],
        vo_updated,
        "unsupported VO update must not write any fields"
    );
    assert_eq!(
        mcp_envelope(&mcp_call(
            &project.root,
            "vo_get",
            json!({"id": "VO-KNOWN"}),
        ))["data"],
        vo_other_before,
        "VO update must not alter unrelated records"
    );
}

/// @vtest.id TEST-CLI-094
/// @vtest.covers VO-CLI-018
/// @vtest.target crates/vtest-cli/src/lib.rs::run_mcp
/// @vtest.intent Full reference spec-to-verify flow completes over MCP stdio
#[test]
fn m9_reference_flow_completes_over_mcp_stdio() {
    let project = TempProject::from_m1_base("complete-flow");
    prepare_spec_fixture(&project);
    let registered = project.root.join("tests/registered.rs");
    let source = fs::read_to_string(&registered).expect("read registered M9 test source");
    fs::write(&registered, format!("use calc_m1_base::known;\n\n{source}"))
        .expect("make target callable from the generated integration test");

    assert_mcp_ok(
        &mcp_call(
            &project.root,
            "req_upsert",
            json!({
                "id": "REQ-M9-FLOW",
                "summary": "MCP reference-flow requirement",
                "specs": ["SPEC-M9-FLOW"],
                "sections": ["1"]
            }),
        ),
        "reference req_upsert",
    );
    assert_mcp_ok(
        &mcp_call(
            &project.root,
            "vo_upsert",
            json!({
                "id": "VO-M9-FLOW",
                "claim": "MCP reference-flow behavior",
                "requirements": ["REQ-M9-FLOW"],
                "specs": ["SPEC-M9-FLOW"],
                "sections": ["1"]
            }),
        ),
        "reference vo_upsert",
    );
    assert_mcp_ok(
        &mcp_call(
            &project.root,
            "vo_approve",
            json!({
                "id": "VO-M9-FLOW",
                "approver": {"kind": "agent", "id": "m9-flow"},
                "basis": []
            }),
        ),
        "reference vo_approve",
    );
    for (name, arguments) in [
        ("spec_get", json!({"id": "SPEC-M9-FLOW"})),
        ("req_get", json!({"id": "REQ-M9-FLOW"})),
        ("vo_get", json!({"id": "VO-M9-FLOW"})),
    ] {
        assert_mcp_ok(&mcp_call(&project.root, name, arguments), name);
    }

    assert_mcp_ok(
        &mcp_call(
            &project.root,
            "form_get",
            json!({"kind": "rust-unit-function"}),
        ),
        "reference form_get",
    );
    let answers = generated_test_answers("mcp_generated_test");
    let dry_run = mcp_call(
        &project.root,
        "test_create",
        json!({
            "form": "rust-unit-function",
            "answers": answers,
            "id": "TEST-M9-FLOW",
            "dry_run": true
        }),
    );
    assert_eq!(
        dry_run["result"]["isError"], false,
        "reference test_create dry-run: {dry_run}"
    );
    assert_eq!(mcp_envelope(&dry_run)["data"]["dry_run"], true);
    let created = mcp_call(
        &project.root,
        "test_create",
        json!({
            "form": "rust-unit-function",
            "answers": generated_test_answers("mcp_generated_test"),
            "id": "TEST-M9-FLOW"
        }),
    );
    assert_mcp_ok(&created, "reference test_create");
    assert_mcp_ok(
        &mcp_call(
            &project.root,
            "test_query",
            json!({"source": "src/lib.rs::known"}),
        ),
        "reference test_query",
    );
    assert_mcp_ok(
        &mcp_call(&project.root, "test_get", json!({"id": "TEST-M9-FLOW"})),
        "reference test_get",
    );
    let edited = mcp_call(
        &project.root,
        "test_edit",
        json!({
            "id": "TEST-M9-FLOW",
            "body": "let actual = known();\nassert_eq!(actual, ());"
        }),
    );
    assert_mcp_ok(&edited, "reference test_edit");

    let static_audit = mcp_call(
        &project.root,
        "audit_static",
        json!({"test": "TEST-M9-FLOW"}),
    );
    assert_eq!(
        static_audit["result"]["isError"], true,
        "reference audit_static must preserve its UNKNOWN result"
    );
    assert_eq!(mcp_envelope(&static_audit)["ok"], false);
    assert!(mcp_envelope(&static_audit)["data"]["audits"].is_array());
    let bundle = mcp_call(
        &project.root,
        "audit_bundle",
        json!({
            "kind": "test-semantic",
            "test": "TEST-M9-FLOW",
            "include_failed": true
        }),
    );
    let bundle_id = assert_mcp_ok(&bundle, "reference audit_bundle")["data"]["bundle_id"]
        .as_str()
        .expect("reference bundle id")
        .to_owned();
    let submission = json!({
        "bundle_id": bundle_id,
        "kind": "test-semantic",
        "verdict": "PASS",
        "reasons": [{
            "claim": "the generated test has a traceable semantic basis",
            "basis": [{"kind": "test-code", "ref": "tests/registered.rs::mcp_generated_test"}]
        }],
        "exclusions": [],
        "auditor": {"kind": "agent", "id": "m9-flow", "model": "test"},
        "confidence": "high"
    });
    assert_mcp_ok(
        &mcp_call(
            &project.root,
            "audit_submit",
            json!({"submission": submission}),
        ),
        "reference audit_submit",
    );
    let run = mcp_call(
        &project.root,
        "run_tests",
        json!({"test": "TEST-M9-FLOW", "fast": true}),
    );
    assert!(run["result"]["structuredContent"]["data"].is_object());
    assert!(run["result"]["structuredContent"]["diagnostics"].is_array());
    let items = [
        "test_existence",
        "semantic_audit",
        "test_execution",
        "runtime_result",
        "evidence_validity",
    ];
    for name in ["verify", "report"] {
        let response = mcp_call(
            &project.root,
            name,
            json!({"test": "TEST-M9-FLOW", "items": items}),
        );
        assert_eq!(
            response["result"]["isError"], true,
            "reference {name}: {response}"
        );
        assert_eq!(mcp_envelope(&response)["ok"], false);
        // Annex C §115: an Evidence whose `revision.commit` cannot be identified
        // is STALE and is not treated as FAIL or a valid PASS. This reference
        // flow records Evidence without committing, so freshness is STALE.
        assert_eq!(mcp_envelope(&response)["data"]["report"]["result"], "STALE");
    }
}

/// @vtest.id TEST-CLI-095
/// @vtest.covers VO-CLI-018
/// @vtest.target crates/vtest-cli/src/lib.rs::run_mcp
/// @vtest.intent MCP reference flow reaches existing CLI create/query/audit/run operations
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
    assert_eq!(responses[4]["result"]["isError"], true);
    assert_eq!(responses[4]["result"]["structuredContent"]["ok"], false);
    assert!(responses[4]["result"]["structuredContent"]["data"].is_object());
    assert_eq!(responses[5]["result"]["isError"], false);
    assert_eq!(responses[5]["result"]["structuredContent"]["ok"], true);
    let evidence = responses[5]["result"]["structuredContent"]["data"]["evidence"]
        .as_array()
        .expect("run_tests returns evidence");
    assert!(!evidence.is_empty());
    assert!(evidence.iter().any(|record| record["result"] != "PASS"));
    assert!(responses[5]["result"]["structuredContent"]["data"].is_object());
}

/// @vtest.id TEST-CLI-096
/// @vtest.covers VO-CLI-018
/// @vtest.target crates/vtest-cli/src/lib.rs::run_mcp
/// @vtest.intent Invalid tool input returns E-OP-001 with message and candidate suggestions
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

/// @vtest.id TEST-CLI-097
/// @vtest.covers VO-CLI-018
/// @vtest.target crates/vtest-cli/src/lib.rs::run_mcp
/// @vtest.intent Protocol/error matrix is fail-closed with correct codes and no audit writes
#[test]
fn m9_protocol_and_error_matrix_is_fail_closed_without_writes() {
    let project = TempProject::from_m1_base("error-matrix");
    let audit_count = || {
        fs::read_dir(project.root.join(".verify/audits"))
            .expect("read audit directory")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry.path().extension().and_then(|value| value.to_str()) == Some("yaml")
            })
            .count()
    };
    let before_audits = audit_count();
    let invalid_symbol_answers = json!({
        "target": "src/lib.rs::knwon",
        "covers": ["VO-KNOWN"],
        "behavior": "invalid symbol",
        "test_kind": "normal",
        "input": "input",
        "expect": "expect",
        "fn_name": "bad_mcp_test"
    });
    let lines = vec![
        "{not-json".to_owned(),
        serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "unsupported/method",
            "params": {}
        }))
        .unwrap(),
        serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {"name": "unknown_tool", "arguments": {}}
        }))
        .unwrap(),
        serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {"name": "spec_get", "arguments": {}}
        }))
        .unwrap(),
        serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {"name": "req_list", "arguments": {"tree": "true"}}
        }))
        .unwrap(),
        serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {"name": "vo_get", "arguments": {"id": "VO-../../escape"}}
        }))
        .unwrap(),
        serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/call",
            "params": {"name": "form_get", "arguments": {"kind": "unknown-form"}}
        }))
        .unwrap(),
        serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "tools/call",
            "params": {"name": "test_query", "arguments": {}}
        }))
        .unwrap(),
        serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "tools/call",
            "params": {
                "name": "test_create",
                "arguments": {
                    "form": "rust-unit-function",
                    "answers": invalid_symbol_answers,
                    "id": "TEST-M9-BAD",
                    "dry_run": true
                }
            }
        }))
        .unwrap(),
        serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "tools/call",
            "params": {
                "name": "audit_submit",
                "arguments": {
                    "submission": {
                        "bundle_id": "00000000000000000000000000",
                        "kind": "test-semantic"
                    }
                }
            }
        }))
        .unwrap(),
        serde_json::to_string(&json!({"id": 11, "method": "scan", "params": {}})).unwrap(),
        serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "id": 12,
            "method": "tools/call",
            "params": {"name": "scan", "arguments": {}}
        }))
        .unwrap(),
    ];
    let responses = mcp_lines(&project.root, &lines);
    assert_eq!(responses.len(), lines.len());
    assert_eq!(responses[0]["error"]["code"], -32700);
    assert_eq!(responses[0]["error"]["data"]["isError"], true);
    assert_eq!(
        responses[0]["error"]["data"]["structuredContent"]["ok"],
        false
    );
    assert_eq!(responses[1]["error"]["code"], -32601);
    assert_eq!(responses[1]["error"]["data"]["isError"], true);
    assert_mcp_failure(&responses[2], "E-OP-001", "unknown MCP tool");
    assert_mcp_failure(&responses[3], "E-OP-001", "missing required ID");
    assert_mcp_failure(&responses[4], "E-OP-001", "wrong boolean type");
    assert_mcp_failure(&responses[5], "E-OP-001", "path-like ID");
    assert_mcp_failure(&responses[6], "E-OP-001", "invalid form");
    assert_mcp_failure(&responses[7], "E-OP-001", "missing selector");
    let symbol_error = assert_mcp_failure(&responses[8], "E-OP-001", "invalid symbol");
    assert_eq!(
        symbol_error["diagnostics"][0]["candidates"][0],
        "src/lib.rs::known"
    );
    assert_mcp_failure(&responses[9], "E-AUDIT-005", "invalid audit submission");
    assert_eq!(responses[10]["error"]["code"], -32600);
    assert_eq!(responses[10]["error"]["data"]["isError"], true);
    assert_mcp_ok(&responses[11], "valid request after error matrix");
    assert_eq!(
        audit_count(),
        before_audits,
        "invalid submissions must not write audits"
    );
}
