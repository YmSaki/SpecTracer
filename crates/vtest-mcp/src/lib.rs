//! MCP stdio transport for the existing `vtest --format json` application.
//!
//! The adapter deliberately delegates tool execution to the CLI binary.  This
//! keeps the MCP transport from growing a second decision engine while the
//! application layer is being extracted.  Every request therefore performs a
//! fresh CLI scan and returns the CLI JSON envelope unchanged.

use std::{
    fs,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{json, Map, Value};
use vtest_store::{load_form_schema, VerifyLayout};

const TOOL_NAMES: &[&str] = &[
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

/// Run the MCP JSON-RPC server over stdin/stdout until EOF.
pub fn serve(root: &Path) -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::BufWriter::new(io::stdout().lock());
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let request = match serde_json::from_str::<Value>(&line) {
            Ok(request) => request,
            Err(error) => {
                write_response(
                    &mut stdout,
                    None,
                    json_rpc_error(-32700, format!("invalid JSON: {error}")),
                )?;
                continue;
            }
        };
        let id = request.get("id").cloned();
        let method = request.get("method").and_then(Value::as_str).unwrap_or("");
        if method == "notifications/initialized" || method.starts_with("notifications/") {
            continue;
        }
        let result = match method {
            "initialize" => initialize_result(),
            "ping" => json!({}),
            "tools/list" => tools_list_result(),
            "tools/call" => {
                let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
                tools_call_result(root, &params)
            }
            _ => json_rpc_error(-32601, format!("method `{method}` is not supported")),
        };
        write_response(&mut stdout, id.as_ref(), result)?;
    }
    Ok(())
}

fn write_response(writer: &mut impl Write, id: Option<&Value>, payload: Value) -> io::Result<()> {
    let response = if payload.get("jsonrpc_error").is_some() {
        let error = payload
            .get("jsonrpc_error")
            .cloned()
            .unwrap_or_else(|| json!({"code": -32603, "message": "unknown error"}));
        json!({"jsonrpc": "2.0", "id": id.cloned().unwrap_or(Value::Null), "error": error})
    } else {
        json!({"jsonrpc": "2.0", "id": id.cloned().unwrap_or(Value::Null), "result": payload})
    };
    serde_json::to_writer(&mut *writer, &response)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    writer.write_all(b"\n")?;
    writer.flush()
}

fn json_rpc_error(code: i64, message: impl Into<String>) -> Value {
    json!({"jsonrpc_error": {"code": code, "message": message.into()}})
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": "2025-06-18",
        "capabilities": {"tools": {"listChanged": false}},
        "serverInfo": {"name": "vtest", "version": env!("CARGO_PKG_VERSION")}
    })
}

fn tools_list_result() -> Value {
    json!({
        "tools": TOOL_NAMES.iter().map(|name| {
            json!({
                "name": name,
                "description": format!("SpecTracer {name} operation"),
                "inputSchema": {"type": "object", "additionalProperties": true}
            })
        }).collect::<Vec<_>>()
    })
}

fn tools_call_result(root: &Path, params: &Value) -> Value {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let arguments = params
        .get("arguments")
        .filter(|value| value.is_object())
        .cloned()
        .unwrap_or_else(|| json!({}));
    let envelope = dispatch_tool(root, name, &arguments);
    let is_error = envelope.get("ok") == Some(&Value::Bool(false));
    let text = serde_json::to_string(&envelope).unwrap_or_else(|_| "{}".to_owned());
    json!({
        "content": [{"type": "text", "text": text}],
        "structuredContent": envelope,
        "isError": is_error
    })
}

fn dispatch_tool(root: &Path, name: &str, args: &Value) -> Value {
    if !TOOL_NAMES.contains(&name) {
        return failure_envelope("E-OP-001", format!("unknown MCP tool `{name}`"), Vec::new());
    }
    match name {
        "scan" => run_cli(root, &["scan"]),
        "spec_list" => run_cli(root, &["spec", "list"]),
        "spec_get" => required_id(root, args, "spec", "show", "SPEC-"),
        "req_list" => {
            let mut command = vec!["req".to_owned(), "list".to_owned()];
            if bool_arg(args, "tree") {
                command.push("--tree".to_owned());
            }
            run_cli_owned(root, command)
        }
        "req_get" => required_id(root, args, "req", "show", "REQ-"),
        "req_upsert" => req_upsert(root, args),
        "vo_list" => vo_list(root, args),
        "vo_get" => required_id(root, args, "vo", "show", "VO-"),
        "vo_upsert" => vo_upsert(root, args),
        "vo_expand" => vo_expand(root, args),
        "vo_approve" => vo_approve(root, args),
        "test_query" => test_query(root, args),
        "test_get" => required_id(root, args, "test", "show", "TEST-"),
        "form_get" => form_get(root, args),
        "test_create" => test_create(root, args),
        "test_edit" => test_edit(root, args),
        "audit_static" => audit_static(root, args),
        "audit_bundle" => audit_bundle(root, args),
        "audit_submit" => audit_submit(root, args),
        "run_tests" => run_tests(root, args),
        "verify" | "report" => verify_or_report(root, name, args),
        _ => unreachable!("tool list and dispatch must stay in sync"),
    }
}

fn required_id(root: &Path, args: &Value, group: &str, action: &str, prefix: &str) -> Value {
    let Some(id) = string_arg(args, "id") else {
        return failure_envelope(
            "E-OP-001",
            format!("{group}_{action} requires id"),
            Vec::new(),
        );
    };
    if !id.starts_with(prefix) {
        return failure_envelope(
            "E-OP-001",
            format!("id must start with `{prefix}`"),
            Vec::new(),
        );
    }
    run_cli_owned(
        root,
        vec![group.to_owned(), action.to_owned(), id.to_owned()],
    )
}

fn req_upsert(root: &Path, args: &Value) -> Value {
    let Some(id) = string_arg(args, "id") else {
        return failure_envelope("E-OP-001", "req_upsert requires id", Vec::new());
    };
    let Some(summary) = string_arg(args, "summary") else {
        return failure_envelope("E-OP-001", "req_upsert requires summary", Vec::new());
    };
    let exists = run_cli_owned(
        root,
        vec!["req".to_owned(), "show".to_owned(), id.to_owned()],
    )
    .get("ok")
        == Some(&Value::Bool(true));
    let mut command = vec![
        "req".to_owned(),
        if exists { "edit" } else { "add" }.to_owned(),
        id.to_owned(),
    ];
    command.extend(["--summary".to_owned(), summary.to_owned()]);
    if let Some(parent) = string_arg(args, "parent") {
        command.extend(["--parent".to_owned(), parent.to_owned()]);
    }
    if !exists {
        repeat_args(&mut command, "--spec", args.get("specs"));
        repeat_args(&mut command, "--sections", args.get("sections"));
    }
    run_cli_owned(root, command)
}

fn vo_list(root: &Path, args: &Value) -> Value {
    let mut command = vec!["vo".to_owned(), "list".to_owned()];
    if let Some(req) = string_arg(args, "req") {
        command.extend(["--req".to_owned(), req.to_owned()]);
    }
    if let Some(status) = string_arg(args, "status") {
        command.extend(["--status".to_owned(), status.to_owned()]);
    }
    run_cli_owned(root, command)
}

fn vo_upsert(root: &Path, args: &Value) -> Value {
    let Some(id) = string_arg(args, "id") else {
        return failure_envelope("E-OP-001", "vo_upsert requires id", Vec::new());
    };
    let Some(claim) = string_arg(args, "claim") else {
        return failure_envelope("E-OP-001", "vo_upsert requires claim", Vec::new());
    };
    let exists = run_cli_owned(
        root,
        vec!["vo".to_owned(), "show".to_owned(), id.to_owned()],
    )
    .get("ok")
        == Some(&Value::Bool(true));
    let mut command = vec![
        "vo".to_owned(),
        if exists { "edit" } else { "add" }.to_owned(),
        id.to_owned(),
    ];
    command.extend(["--claim".to_owned(), claim.to_owned()]);
    if let Some(parent) = string_arg(args, "parent") {
        command.extend(["--parent".to_owned(), parent.to_owned()]);
    }
    if !exists {
        repeat_args(&mut command, "--req", args.get("requirements"));
        repeat_args(&mut command, "--spec", args.get("specs"));
        repeat_args(&mut command, "--sections", args.get("sections"));
        repeat_args(&mut command, "--dimension", args.get("dimensions"));
        if let Some(policy) = string_arg(args, "policy") {
            command.extend(["--policy".to_owned(), policy.to_owned()]);
        }
        repeat_args(&mut command, "--combination", args.get("combinations"));
    }
    run_cli_owned(root, command)
}

fn vo_expand(root: &Path, args: &Value) -> Value {
    let Some(id) = string_arg(args, "id") else {
        return failure_envelope("E-OP-001", "vo_expand requires id", Vec::new());
    };
    let mut command = vec!["vo".to_owned(), "expand".to_owned(), id.to_owned()];
    if bool_arg(args, "dry_run") {
        command.push("--dry-run".to_owned());
    }
    run_cli_owned(root, command)
}

fn vo_approve(root: &Path, args: &Value) -> Value {
    let Some(id) = string_arg(args, "id") else {
        return failure_envelope("E-OP-001", "vo_approve requires id", Vec::new());
    };
    let Some(approver) = args.get("approver").and_then(Value::as_object) else {
        return failure_envelope("E-OP-001", "vo_approve requires approver", Vec::new());
    };
    let Some(kind) = approver.get("kind").and_then(Value::as_str) else {
        return failure_envelope("E-OP-001", "approver.kind is required", Vec::new());
    };
    let Some(approver_id) = approver.get("id").and_then(Value::as_str) else {
        return failure_envelope("E-OP-001", "approver.id is required", Vec::new());
    };
    let mut command = vec![
        "vo".to_owned(),
        "approve".to_owned(),
        id.to_owned(),
        "--approver-kind".to_owned(),
        kind.to_owned(),
        "--approver-id".to_owned(),
        approver_id.to_owned(),
    ];
    if let Some(model) = string_arg(args, "model") {
        command.extend(["--model".to_owned(), model.to_owned()]);
    }
    repeat_args(&mut command, "--basis", args.get("basis"));
    run_cli_owned(root, command)
}

fn test_query(root: &Path, args: &Value) -> Value {
    let selectors = usize::from(string_arg(args, "vo").is_some())
        + usize::from(string_arg(args, "source").is_some())
        + usize::from(bool_arg(args, "unregistered"));
    if selectors != 1 {
        return failure_envelope(
            "E-OP-001",
            "test_query requires exactly one of vo, source, or unregistered",
            Vec::new(),
        );
    }
    let command = if let Some(vo) = string_arg(args, "vo") {
        vec![
            "test".to_owned(),
            "list".to_owned(),
            "--vo".to_owned(),
            vo.to_owned(),
        ]
    } else if let Some(source) = string_arg(args, "source") {
        vec![
            "test".to_owned(),
            "query".to_owned(),
            "--source".to_owned(),
            source.to_owned(),
        ]
    } else {
        vec![
            "test".to_owned(),
            "list".to_owned(),
            "--unregistered".to_owned(),
        ]
    };
    run_cli_owned(root, command)
}

fn form_get(root: &Path, args: &Value) -> Value {
    let Some(kind) = string_arg(args, "kind") else {
        return failure_envelope("E-OP-001", "form_get requires kind", Vec::new());
    };
    match load_form_schema(&VerifyLayout::new(root), kind) {
        Ok(schema) => json!({"ok": true, "data": schema, "diagnostics": []}),
        Err(error) => failure_envelope("E-OP-001", error.to_string(), Vec::new()),
    }
}

fn test_create(root: &Path, args: &Value) -> Value {
    let Some(form) = string_arg(args, "form") else {
        return failure_envelope("E-OP-001", "test_create requires form", Vec::new());
    };
    let Some(answers) = args.get("answers").and_then(Value::as_object) else {
        return failure_envelope("E-OP-001", "test_create requires answers", Vec::new());
    };
    let path = match write_temp_file(root, "answers", &answers_yaml(form, answers)) {
        Ok(path) => path,
        Err(error) => return failure_envelope("E-CORE-001", error.to_string(), Vec::new()),
    };
    let mut command = vec![
        "test".to_owned(),
        "create".to_owned(),
        "--form".to_owned(),
        form.to_owned(),
        "--answers".to_owned(),
        project_relative(root, &path),
    ];
    if let Some(id) = string_arg(args, "id") {
        command.extend(["--id".to_owned(), id.to_owned()]);
    }
    if bool_arg(args, "dry_run") {
        command.push("--dry-run".to_owned());
    }
    let result = run_cli_owned(root, command);
    let _ = fs::remove_file(path);
    result
}

fn test_edit(root: &Path, args: &Value) -> Value {
    let Some(id) = string_arg(args, "id") else {
        return failure_envelope("E-OP-001", "test_edit requires id", Vec::new());
    };
    let mut command = vec!["test".to_owned(), "edit".to_owned(), id.to_owned()];
    let mut temporary = Vec::new();
    if let Some(answers) = args.get("answers").and_then(Value::as_object) {
        let form = answers
            .get("form")
            .and_then(Value::as_str)
            .unwrap_or("rust-unit-function");
        match write_temp_file(root, "answers", &answers_yaml(form, answers)) {
            Ok(path) => {
                command.extend(["--answers".to_owned(), project_relative(root, &path)]);
                temporary.push(path);
            }
            Err(error) => return failure_envelope("E-CORE-001", error.to_string(), Vec::new()),
        }
    }
    if let Some(set) = args.get("set").and_then(Value::as_object) {
        for (key, value) in set {
            command.extend([
                "--set".to_owned(),
                format!("{key}={}", render_arg_value(value)),
            ]);
        }
    }
    if let Some(body) = args.get("body").and_then(Value::as_str) {
        match write_temp_file(root, "body", body) {
            Ok(path) => {
                command.extend(["--body-file".to_owned(), project_relative(root, &path)]);
                temporary.push(path);
            }
            Err(error) => return failure_envelope("E-CORE-001", error.to_string(), Vec::new()),
        }
    }
    if bool_arg(args, "dry_run") {
        command.push("--dry-run".to_owned());
    }
    let result = run_cli_owned(root, command);
    for path in temporary {
        let _ = fs::remove_file(path);
    }
    result
}

fn audit_static(root: &Path, args: &Value) -> Value {
    let mut command = vec!["audit".to_owned(), "static".to_owned()];
    if let Some(test) = string_arg(args, "test") {
        command.extend(["--test".to_owned(), test.to_owned()]);
    } else if bool_arg(args, "all") {
        command.push("--all".to_owned());
    } else {
        return failure_envelope("E-OP-001", "audit_static requires test or all", Vec::new());
    }
    run_cli_owned(root, command)
}

fn audit_bundle(root: &Path, args: &Value) -> Value {
    let Some(kind) = string_arg(args, "kind") else {
        return failure_envelope("E-OP-001", "audit_bundle requires kind", Vec::new());
    };
    let mut command = vec![
        "audit".to_owned(),
        "bundle".to_owned(),
        "--kind".to_owned(),
        kind.to_owned(),
    ];
    let selectors = [("test", "--test"), ("vo", "--vo"), ("req", "--req")];
    let selected = selectors
        .iter()
        .filter_map(|(key, flag)| string_arg(args, key).map(|value| (*flag, value.to_owned())))
        .collect::<Vec<_>>();
    if selected.len() != 1 {
        return failure_envelope(
            "E-OP-001",
            "audit_bundle requires exactly one target",
            Vec::new(),
        );
    }
    command.extend([selected[0].0.to_owned(), selected[0].1.clone()]);
    if bool_arg(args, "include_failed") {
        command.push("--include-failed".to_owned());
    }
    run_cli_owned(root, command)
}

fn audit_submit(root: &Path, args: &Value) -> Value {
    let Some(submission) = args.get("submission") else {
        return failure_envelope("E-OP-001", "audit_submit requires submission", Vec::new());
    };
    let text = match serde_json::to_string_pretty(submission) {
        Ok(text) => text,
        Err(error) => return failure_envelope("E-CORE-001", error.to_string(), Vec::new()),
    };
    let path = match write_temp_file(root, "audit-submit", &text) {
        Ok(path) => path,
        Err(error) => return failure_envelope("E-CORE-001", error.to_string(), Vec::new()),
    };
    let result = run_cli_owned(
        root,
        vec![
            "audit".to_owned(),
            "submit".to_owned(),
            "--file".to_owned(),
            project_relative(root, &path),
        ],
    );
    let _ = fs::remove_file(path);
    result
}

fn run_tests(root: &Path, args: &Value) -> Value {
    let selectors = [("test", "--test"), ("vo", "--vo"), ("req", "--req")];
    let mut selected = selectors
        .iter()
        .filter_map(|(key, flag)| string_arg(args, key).map(|value| (*flag, value.to_owned())))
        .collect::<Vec<_>>();
    if bool_arg(args, "all") {
        selected.push(("--all", String::new()));
    }
    if selected.len() != 1 {
        return failure_envelope(
            "E-OP-001",
            "run_tests requires exactly one selector",
            Vec::new(),
        );
    }
    let mut command = vec!["run".to_owned(), selected[0].0.to_owned()];
    if !selected[0].1.is_empty() {
        command.push(selected[0].1.clone());
    }
    if bool_arg(args, "fast") {
        command.push("--fast".to_owned());
    }
    run_cli_owned(root, command)
}

fn verify_or_report(root: &Path, name: &str, args: &Value) -> Value {
    let mut command = vec![name.to_owned()];
    if let Some(items) = args.get("items") {
        let value = if let Some(items) = items.as_array() {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(",")
        } else {
            items.as_str().unwrap_or_default().to_owned()
        };
        if value.is_empty() {
            return failure_envelope("E-OP-001", "items must not be empty", Vec::new());
        }
        command.extend(["--items".to_owned(), value]);
    }
    for (key, flag) in [("req", "--req"), ("vo", "--vo"), ("test", "--test")] {
        if let Some(value) = string_arg(args, key) {
            command.extend([flag.to_owned(), value.to_owned()]);
        }
    }
    run_cli_owned(root, command)
}

fn run_cli(root: &Path, command: &[&str]) -> Value {
    run_cli_owned(
        root,
        command.iter().map(|item| (*item).to_owned()).collect(),
    )
}

fn run_cli_owned(root: &Path, command: Vec<String>) -> Value {
    let executable = match std::env::current_exe() {
        Ok(path) => path,
        Err(error) => return failure_envelope("E-CORE-001", error.to_string(), Vec::new()),
    };
    let output = match Command::new(executable)
        .arg("--project")
        .arg(root)
        .args(["--format", "json"])
        .args(command)
        .output()
    {
        Ok(output) => output,
        Err(error) => return failure_envelope("E-CORE-001", error.to_string(), Vec::new()),
    };
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        failure_envelope(
            "E-CORE-001",
            format!("CLI returned invalid JSON: {error}"),
            Vec::new(),
        )
    })
}

fn write_temp_file(root: &Path, prefix: &str, text: &str) -> io::Result<PathBuf> {
    let directory = root.join(".verify/cache/mcp");
    fs::create_dir_all(&directory)?;
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = directory.join(format!("{prefix}-{suffix}.yaml"));
    fs::write(&path, text)?;
    Ok(path)
}

fn project_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn answers_yaml(form: &str, answers: &Map<String, Value>) -> String {
    let mut text = format!("form: {}\nanswers:\n", yaml_scalar(form));
    for (key, value) in answers {
        if let Some(items) = value.as_array() {
            text.push_str(&format!("  {key}:\n"));
            for item in items {
                text.push_str(&format!("    - {}\n", yaml_scalar(&render_arg_value(item))));
            }
        } else {
            text.push_str(&format!(
                "  {key}: {}\n",
                yaml_scalar(&render_arg_value(value))
            ));
        }
    }
    text
}

fn yaml_scalar(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn render_arg_value(value: &Value) -> String {
    if let Some(text) = value.as_str() {
        return text.to_owned();
    }
    if let Some(items) = value.as_array() {
        return items
            .iter()
            .map(render_arg_value)
            .collect::<Vec<_>>()
            .join(",");
    }
    value.to_string()
}

fn repeat_args(command: &mut Vec<String>, flag: &str, value: Option<&Value>) {
    if let Some(items) = value.and_then(Value::as_array) {
        for item in items.iter().filter_map(Value::as_str) {
            command.extend([flag.to_owned(), item.to_owned()]);
        }
    }
}

fn string_arg<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(Value::as_str)
}

fn bool_arg(args: &Value, key: &str) -> bool {
    args.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn failure_envelope(code: &str, message: impl Into<String>, candidates: Vec<String>) -> Value {
    let mut diagnostic = json!({
        "code": code,
        "severity": "error",
        "message": message.into()
    });
    if !candidates.is_empty() {
        diagnostic["candidates"] = json!(candidates);
    }
    json!({"ok": false, "data": null, "diagnostics": [diagnostic]})
}
