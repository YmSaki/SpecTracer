//! MCP stdio transport for the existing `vtest --format json` application.
//!
//! The adapter deliberately delegates tool execution to the CLI binary.  This
//! keeps the MCP transport from growing a second decision engine while the
//! application layer is being extracted.  Every tool call therefore performs a
//! deterministic mtime freshness check before delegating to the CLI envelope.

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

const SAFE_RECORD_ID_CHARS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-";

#[derive(Default)]
struct MtimeRescan {
    last_scan: Option<Vec<(String, u128)>>,
}

/// Run the MCP JSON-RPC server over stdin/stdout until EOF.
pub fn serve(root: &Path) -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::BufWriter::new(io::stdout().lock());
    let mut mtime_rescan = MtimeRescan::default();
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
                    json_rpc_error(-32700, format!("invalid JSON: {error}"), "E-OP-001"),
                )?;
                continue;
            }
        };
        let Some(request_object) = request.as_object() else {
            write_response(
                &mut stdout,
                None,
                json_rpc_error(-32600, "request must be a JSON object", "E-OP-001"),
            )?;
            continue;
        };
        let id = request_object.get("id").cloned();
        if let Some(id) = &id {
            if !(id.is_null() || id.is_string() || id.is_number()) {
                write_response(
                    &mut stdout,
                    None,
                    json_rpc_error(
                        -32600,
                        "request id must be a string, number, or null",
                        "E-OP-001",
                    ),
                )?;
                continue;
            }
        }
        if request_object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
            write_response(
                &mut stdout,
                id.as_ref(),
                json_rpc_error(-32600, "jsonrpc must be `2.0`", "E-OP-001"),
            )?;
            continue;
        }
        let Some(method) = request_object.get("method").and_then(Value::as_str) else {
            write_response(
                &mut stdout,
                id.as_ref(),
                json_rpc_error(-32600, "method must be a string", "E-OP-001"),
            )?;
            continue;
        };
        if method == "notifications/initialized" || method.starts_with("notifications/") {
            continue;
        }
        let is_notification = !request_object.contains_key("id");
        let result = match method {
            "initialize" => method_result(request_object, initialize_result),
            "ping" => method_result(request_object, || json!({})),
            "tools/list" => method_result(request_object, tools_list_result),
            "tools/call" => {
                let params = request_object
                    .get("params")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                tools_call_result(root, &params, &mut mtime_rescan)
            }
            _ => json_rpc_error(
                -32601,
                format!("method `{method}` is not supported"),
                "E-OP-001",
            ),
        };
        if !is_notification {
            write_response(&mut stdout, id.as_ref(), result)?;
        }
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

fn json_rpc_error(code: i64, message: impl Into<String>, diagnostic_code: &str) -> Value {
    let message = message.into();
    let envelope = failure_envelope(diagnostic_code, message.clone(), Vec::new());
    json!({
        "jsonrpc_error": {
            "code": code,
            "message": message,
            "isError": true,
            "structuredContent": envelope.clone(),
            "data": {
                "isError": true,
                "structuredContent": envelope
            }
        }
    })
}

fn method_result(request: &Map<String, Value>, build: impl FnOnce() -> Value) -> Value {
    if let Some(params) = request.get("params") {
        if !params.is_object() {
            return json_rpc_error(-32602, "params must be an object", "E-OP-001");
        }
    }
    build()
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
                "inputSchema": tool_input_schema(name)
            })
        }).collect::<Vec<_>>()
    })
}

fn tools_call_result(root: &Path, params: &Value, mtime_rescan: &mut MtimeRescan) -> Value {
    let Some(params) = params.as_object() else {
        return tool_result(failure_envelope(
            "E-OP-001",
            "tools/call params must be an object",
            Vec::new(),
        ));
    };
    if let Some(key) = params
        .keys()
        .find(|key| *key != "name" && *key != "arguments")
    {
        return tool_result(failure_envelope(
            "E-OP-001",
            format!("tools/call does not accept parameter `{key}`"),
            Vec::new(),
        ));
    }
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return tool_result(failure_envelope(
            "E-OP-001",
            "tools/call requires string name",
            Vec::new(),
        ));
    };
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let Some(arguments) = arguments.as_object() else {
        return tool_result(failure_envelope(
            "E-OP-001",
            "tools/call arguments must be an object",
            Vec::new(),
        ));
    };
    if !TOOL_NAMES.contains(&name) {
        return tool_result(failure_envelope(
            "E-OP-001",
            format!("unknown MCP tool `{name}`"),
            Vec::new(),
        ));
    }
    if let Err(error) = validate_tool_arguments(root, name, arguments) {
        return tool_result(error);
    }
    if name != "scan" {
        if let Some(scan) = rescan_if_changed(root, mtime_rescan) {
            return tool_result(scan);
        }
    }
    let envelope = dispatch_tool(root, name, &Value::Object(arguments.clone()));
    if name == "scan" && envelope.get("ok") == Some(&Value::Bool(true)) {
        mtime_rescan.last_scan = project_mtime_snapshot(root).ok();
    }
    tool_result(envelope)
}

fn rescan_if_changed(root: &Path, state: &mut MtimeRescan) -> Option<Value> {
    let current = match project_mtime_snapshot(root) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            return Some(failure_envelope(
                "E-CORE-001",
                format!("cannot inspect project mtimes: {error}"),
                Vec::new(),
            ))
        }
    };
    if state
        .last_scan
        .as_ref()
        .is_some_and(|previous| previous == &current)
    {
        return None;
    }
    let scan = run_cli(root, &["scan"]);
    if scan.get("ok") == Some(&Value::Bool(true)) {
        state.last_scan = Some(current);
        None
    } else {
        Some(scan)
    }
}

fn project_mtime_snapshot(root: &Path) -> io::Result<Vec<(String, u128)>> {
    fn visit(root: &Path, directory: &Path, files: &mut Vec<(String, u128)>) -> io::Result<()> {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            let relative = path.strip_prefix(root).unwrap_or(&path);
            if relative.starts_with(Path::new(".git"))
                || relative.starts_with(Path::new(".verify/cache"))
                || relative.starts_with(Path::new("target"))
            {
                continue;
            }
            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                visit(root, &path, files)?;
            } else if metadata.is_file() {
                let modified = metadata
                    .modified()?
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos();
                files.push((relative.to_string_lossy().replace('\\', "/"), modified));
            }
        }
        Ok(())
    }

    let mut files = Vec::new();
    visit(root, root, &mut files)?;
    files.sort_unstable();
    Ok(files)
}

fn tool_result(envelope: Value) -> Value {
    let is_error = envelope.get("ok") == Some(&Value::Bool(false));
    let text = serde_json::to_string(&envelope).unwrap_or_else(|_| "{}".to_owned());
    json!({
        "content": [{"type": "text", "text": text}],
        "structuredContent": envelope,
        "isError": is_error
    })
}

fn tool_input_schema(name: &str) -> Value {
    let (properties, required) = match name {
        "scan" | "spec_list" => (json!({}), Vec::<&str>::new()),
        "spec_get" | "req_get" | "vo_get" | "test_get" => {
            (json!({"id": {"type": "string"}}), vec!["id"])
        }
        "req_list" => (json!({"tree": {"type": "boolean"}}), Vec::new()),
        "req_upsert" => (
            json!({
                "id": {"type": "string"},
                "summary": {"type": "string"},
                "parent": {"type": "string"},
                "specs": {"type": "array", "items": {"type": "string"}},
                "sections": {"type": "array", "items": {"type": "string"}}
            }),
            vec!["id", "summary"],
        ),
        "vo_list" => (
            json!({
                "req": {"type": "string"},
                "status": {"type": "string", "enum": ["draft", "approved"]}
            }),
            Vec::new(),
        ),
        "vo_upsert" => (
            json!({
                "id": {"type": "string"},
                "claim": {"type": "string"},
                "parent": {"type": "string"},
                "requirements": {"type": "array", "items": {"type": "string"}},
                "specs": {"type": "array", "items": {"type": "string"}},
                "sections": {"type": "array", "items": {"type": "string"}},
                "dimensions": {"type": "array", "items": {"type": "string"}},
                "policy": {"type": "string", "enum": ["independent-axes", "full-product", "explicit"]},
                "combinations": {"type": "array", "items": {"type": "string"}}
            }),
            vec!["id", "claim"],
        ),
        "vo_expand" => (
            json!({"id": {"type": "string"}, "dry_run": {"type": "boolean"}}),
            vec!["id"],
        ),
        "vo_approve" => (
            json!({
                "id": {"type": "string"},
                "approver": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {"kind": {"type": "string"}, "id": {"type": "string"}},
                    "required": ["kind", "id"]
                },
                "model": {"type": "string"},
                "basis": {"type": "array", "items": {"type": "string"}}
            }),
            vec!["id", "approver"],
        ),
        "test_query" => (
            json!({
                "vo": {"type": "string"},
                "source": {"type": "string"},
                "unregistered": {"type": "boolean"}
            }),
            Vec::new(),
        ),
        "form_get" => (json!({"kind": {"type": "string"}}), vec!["kind"]),
        "test_create" => (
            json!({
                "form": {"type": "string"},
                "answers": {"type": "object"},
                "id": {"type": "string"},
                "dry_run": {"type": "boolean"}
            }),
            vec!["form", "answers"],
        ),
        "test_edit" => (
            json!({
                "id": {"type": "string"},
                "answers": {"type": "object"},
                "set": {"type": "object"},
                "body": {"type": "string"},
                "dry_run": {"type": "boolean"}
            }),
            vec!["id"],
        ),
        "audit_static" => (
            json!({"test": {"type": "string"}, "all": {"type": "boolean"}}),
            Vec::new(),
        ),
        "audit_bundle" => (
            json!({
                "kind": {"type": "string", "enum": ["test-semantic", "vo-coverage", "impl-consistency", "spec-coverage"]},
                "test": {"type": "string"},
                "vo": {"type": "string"},
                "req": {"type": "string"},
                "spec": {"type": "string"},
                "include_failed": {"type": "boolean"}
            }),
            vec!["kind"],
        ),
        "audit_submit" => (
            json!({"submission": {"type": "object"}}),
            vec!["submission"],
        ),
        "run_tests" => (
            json!({
                "test": {"type": "string"},
                "vo": {"type": "string"},
                "req": {"type": "string"},
                "all": {"type": "boolean"},
                "fast": {"type": "boolean"}
            }),
            Vec::new(),
        ),
        "verify" | "report" => (
            json!({
                "items": {"type": "array", "items": {"type": "string"}},
                "req": {"type": "string"},
                "vo": {"type": "string"},
                "test": {"type": "string"}
            }),
            Vec::new(),
        ),
        _ => (json!({}), Vec::new()),
    };
    let mut schema = json!({
        "type": "object",
        "additionalProperties": false,
        "properties": properties
    });
    if !required.is_empty() {
        schema["required"] = json!(required);
    }
    schema
}

fn validate_tool_arguments(
    root: &Path,
    name: &str,
    args: &Map<String, Value>,
) -> Result<(), Value> {
    if !TOOL_NAMES.contains(&name) {
        return Err(failure_envelope(
            "E-OP-001",
            format!("unknown MCP tool `{name}`"),
            Vec::new(),
        ));
    }
    let allowed = match name {
        "scan" | "spec_list" => &[][..],
        "spec_get" | "req_get" | "vo_get" | "test_get" => &["id"][..],
        "req_list" => &["tree"][..],
        "req_upsert" => &["id", "summary", "parent", "specs", "sections"][..],
        "vo_list" => &["req", "status"][..],
        "vo_upsert" => &[
            "id",
            "claim",
            "parent",
            "requirements",
            "specs",
            "sections",
            "dimensions",
            "policy",
            "combinations",
        ][..],
        "vo_expand" => &["id", "dry_run"][..],
        "vo_approve" => &["id", "approver", "model", "basis"][..],
        "test_query" => &["vo", "source", "unregistered"][..],
        "form_get" => &["kind"][..],
        "test_create" => &["form", "answers", "id", "dry_run"][..],
        "test_edit" => &["id", "answers", "set", "body", "dry_run"][..],
        "audit_static" => &["test", "all"][..],
        "audit_bundle" => &["kind", "test", "vo", "req", "spec", "include_failed"][..],
        "audit_submit" => &["submission"][..],
        "run_tests" => &["test", "vo", "req", "all", "fast"][..],
        "verify" | "report" => &["items", "req", "vo", "test"][..],
        _ => &[][..],
    };
    if let Some(key) = args.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(failure_envelope(
            "E-OP-001",
            format!("{name} does not accept argument `{key}`"),
            Vec::new(),
        ));
    }

    let result = match name {
        "scan" | "spec_list" => Ok(()),
        "spec_get" => validate_required_id(args, "id", "SPEC-").map(|_| ()),
        "req_list" => optional_bool(args, "tree"),
        "req_get" => validate_required_id(args, "id", "REQ-").map(|_| ()),
        "req_upsert" => {
            validate_required_id(args, "id", "REQ-")?;
            required_nonempty_string(args, "summary")?;
            optional_id(args, "parent", "REQ-")?;
            optional_string_array(args, "specs")?;
            optional_string_array(args, "sections")
        }
        "vo_list" => {
            optional_id(args, "req", "REQ-")?;
            if let Some(status) = optional_nonempty_string(args, "status")? {
                if !matches!(status, "draft" | "approved") {
                    return Err(failure_envelope(
                        "E-OP-001",
                        "VO status filter must be draft or approved",
                        vec!["draft".to_owned(), "approved".to_owned()],
                    ));
                }
            }
            Ok(())
        }
        "vo_get" => validate_required_id(args, "id", "VO-").map(|_| ()),
        "vo_upsert" => {
            validate_required_id(args, "id", "VO-")?;
            required_nonempty_string(args, "claim")?;
            optional_id(args, "parent", "VO-")?;
            for key in [
                "requirements",
                "specs",
                "sections",
                "dimensions",
                "combinations",
            ] {
                optional_string_array(args, key)?;
            }
            if let Some(policy) = optional_nonempty_string(args, "policy")? {
                if !matches!(policy, "independent-axes" | "full-product" | "explicit") {
                    return Err(failure_envelope(
                        "E-OP-001",
                        "unsupported VO coverage policy",
                        vec![
                            "independent-axes".to_owned(),
                            "full-product".to_owned(),
                            "explicit".to_owned(),
                        ],
                    ));
                }
            }
            Ok(())
        }
        "vo_expand" => {
            validate_required_id(args, "id", "VO-")?;
            optional_bool(args, "dry_run")
        }
        "vo_approve" => {
            validate_required_id(args, "id", "VO-")?;
            let approver = required_object(args, "approver")?;
            reject_unknown_object_keys("approver", approver, &["kind", "id"])?;
            required_nonempty_string(approver, "kind")?;
            required_nonempty_string(approver, "id")?;
            optional_nonempty_string(args, "model")?;
            optional_string_array(args, "basis")
        }
        "test_query" => {
            optional_nonempty_string(args, "vo")?;
            optional_nonempty_string(args, "source")?;
            optional_bool(args, "unregistered")?;
            let selectors = usize::from(args.get("vo").is_some())
                + usize::from(args.get("source").is_some())
                + usize::from(args.get("unregistered") == Some(&Value::Bool(true)));
            if selectors != 1 {
                Err(failure_envelope(
                    "E-OP-001",
                    "test_query requires exactly one of vo, source, or unregistered",
                    Vec::new(),
                ))
            } else {
                Ok(())
            }
        }
        "test_get" => validate_required_id(args, "id", "TEST-").map(|_| ()),
        "form_get" => {
            let kind = required_nonempty_string(args, "kind")?;
            load_form_schema(&VerifyLayout::new(root), kind)
                .map(|_| ())
                .map_err(|error| failure_envelope("E-OP-001", error.to_string(), Vec::new()))
        }
        "test_create" => {
            let form = required_nonempty_string(args, "form")?;
            required_object(args, "answers")?;
            if let Some(id) = optional_nonempty_string(args, "id")? {
                validate_id_value(id, "TEST-")?;
            }
            optional_bool(args, "dry_run")?;
            load_form_schema(&VerifyLayout::new(root), form)
                .map(|_| ())
                .map_err(|error| failure_envelope("E-OP-001", error.to_string(), Vec::new()))
        }
        "test_edit" => {
            let id = required_nonempty_string(args, "id")?;
            validate_id_value(id, "TEST-")?;
            if let Some(answers) = args.get("answers") {
                if !answers.is_object() {
                    return Err(failure_envelope(
                        "E-OP-001",
                        "test_edit answers must be an object",
                        Vec::new(),
                    ));
                }
            }
            if let Some(set) = args.get("set") {
                if !set.is_object() {
                    return Err(failure_envelope(
                        "E-OP-001",
                        "test_edit set must be an object",
                        Vec::new(),
                    ));
                }
            }
            if let Some(body) = args.get("body") {
                if body.as_str().is_none_or(|value| value.is_empty()) {
                    return Err(failure_envelope(
                        "E-OP-001",
                        "test_edit body must be a non-empty string",
                        Vec::new(),
                    ));
                }
            }
            optional_bool(args, "dry_run")?;
            if !args.contains_key("answers")
                && !args.contains_key("set")
                && !args.contains_key("body")
            {
                Err(failure_envelope(
                    "E-OP-001",
                    "test_edit requires answers, set, or body",
                    Vec::new(),
                ))
            } else {
                Ok(())
            }
        }
        "audit_static" => {
            optional_id(args, "test", "TEST-")?;
            optional_bool(args, "all")?;
            let selected = usize::from(args.get("test").is_some())
                + usize::from(args.get("all") == Some(&Value::Bool(true)));
            if selected != 1 {
                Err(failure_envelope(
                    "E-OP-001",
                    "audit_static requires exactly one of test or all",
                    Vec::new(),
                ))
            } else {
                Ok(())
            }
        }
        "audit_bundle" => {
            let kind = required_nonempty_string(args, "kind")?;
            if !matches!(
                kind,
                "test-semantic" | "vo-coverage" | "impl-consistency" | "spec-coverage"
            ) {
                return Err(failure_envelope(
                    "E-OP-001",
                    format!("unsupported audit bundle kind {kind}"),
                    vec![
                        "test-semantic".to_owned(),
                        "vo-coverage".to_owned(),
                        "impl-consistency".to_owned(),
                        "spec-coverage".to_owned(),
                    ],
                ));
            }
            optional_id(args, "test", "TEST-")?;
            optional_id(args, "vo", "VO-")?;
            optional_id(args, "req", "REQ-")?;
            optional_id(args, "spec", "SPEC-")?;
            optional_bool(args, "include_failed")?;
            let selected = usize::from(args.contains_key("test"))
                + usize::from(args.contains_key("vo"))
                + usize::from(args.contains_key("req"))
                + usize::from(args.contains_key("spec"));
            if selected != 1 {
                return Err(failure_envelope(
                    "E-OP-001",
                    "audit_bundle requires exactly one target",
                    Vec::new(),
                ));
            }
            let compatible = match kind {
                "test-semantic" => args.contains_key("test"),
                "vo-coverage" => args.contains_key("vo") || args.contains_key("req"),
                "impl-consistency" => args.contains_key("test") || args.contains_key("vo"),
                "spec-coverage" => args.contains_key("spec"),
                _ => false,
            };
            if !compatible {
                return Err(failure_envelope(
                    "E-OP-001",
                    format!("audit bundle {kind} target is not compatible"),
                    Vec::new(),
                ));
            }
            Ok(())
        }
        "audit_submit" => {
            let submission = required_object(args, "submission")?;
            required_nonempty_string(submission, "bundle_id")?;
            required_nonempty_string(submission, "kind")?;
            let bundle_id = submission
                .get("bundle_id")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if !is_safe_ulid(bundle_id) {
                return Err(failure_envelope(
                    "E-AUDIT-003",
                    "bundle_id is not a safe record id",
                    Vec::new(),
                ));
            }
            if let Some(verdict) = submission.get("verdict") {
                if !verdict.is_string() {
                    return Err(failure_envelope(
                        "E-AUDIT-004",
                        "verdict must be a string",
                        Vec::new(),
                    ));
                }
            }
            let Some(reasons) = submission.get("reasons").and_then(Value::as_array) else {
                return Err(failure_envelope(
                    "E-AUDIT-005",
                    "reasons must be a non-empty array",
                    Vec::new(),
                ));
            };
            if reasons.is_empty() {
                return Err(failure_envelope(
                    "E-AUDIT-005",
                    "reasons must be a non-empty array",
                    Vec::new(),
                ));
            }
            Ok(())
        }
        "run_tests" => {
            optional_id(args, "test", "TEST-")?;
            optional_id(args, "vo", "VO-")?;
            optional_id(args, "req", "REQ-")?;
            optional_bool(args, "all")?;
            optional_bool(args, "fast")?;
            let selected = usize::from(args.contains_key("test"))
                + usize::from(args.contains_key("vo"))
                + usize::from(args.contains_key("req"))
                + usize::from(args.get("all") == Some(&Value::Bool(true)));
            if selected != 1 {
                Err(failure_envelope(
                    "E-OP-001",
                    "run_tests requires exactly one selector",
                    Vec::new(),
                ))
            } else {
                Ok(())
            }
        }
        "verify" | "report" => {
            if let Some(items) = args.get("items") {
                let Some(items) = items.as_array() else {
                    return Err(failure_envelope(
                        "E-OP-001",
                        "items must be an array of strings",
                        Vec::new(),
                    ));
                };
                if items.is_empty()
                    || items
                        .iter()
                        .any(|item| item.as_str().is_none_or(|value| value.trim().is_empty()))
                {
                    return Err(failure_envelope(
                        "E-OP-001",
                        "items must not be empty",
                        Vec::new(),
                    ));
                }
            }
            optional_id(args, "req", "REQ-")?;
            optional_id(args, "vo", "VO-")?;
            optional_id(args, "test", "TEST-").map(|_| ())
        }
        _ => Ok(()),
    };
    result
}

fn validate_required_id<'a>(
    args: &'a Map<String, Value>,
    key: &str,
    prefix: &str,
) -> Result<&'a str, Value> {
    let value = required_nonempty_string(args, key)?;
    validate_id_value(value, prefix)?;
    Ok(value)
}

fn optional_id<'a>(
    args: &'a Map<String, Value>,
    key: &str,
    prefix: &str,
) -> Result<Option<&'a str>, Value> {
    let Some(value) = optional_nonempty_string(args, key)? else {
        return Ok(None);
    };
    validate_id_value(value, prefix)?;
    Ok(Some(value))
}

fn validate_id_value(value: &str, prefix: &str) -> Result<(), Value> {
    if value.starts_with(prefix)
        && value.len() > prefix.len()
        && value[prefix.len()..]
            .chars()
            .all(|character| SAFE_RECORD_ID_CHARS.contains(character))
        && value
            .chars()
            .all(|character| SAFE_RECORD_ID_CHARS.contains(character))
    {
        Ok(())
    } else {
        Err(failure_envelope(
            "E-OP-001",
            format!("id must be a safe `{prefix}` identifier"),
            Vec::new(),
        ))
    }
}

fn required_nonempty_string<'a>(args: &'a Map<String, Value>, key: &str) -> Result<&'a str, Value> {
    optional_nonempty_string(args, key)?.ok_or_else(|| {
        failure_envelope(
            "E-OP-001",
            format!("argument `{key}` is required and must be a non-empty string"),
            Vec::new(),
        )
    })
}

fn optional_nonempty_string<'a>(
    args: &'a Map<String, Value>,
    key: &str,
) -> Result<Option<&'a str>, Value> {
    match args.get(key) {
        None => Ok(None),
        Some(value) => value
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .map(Some)
            .ok_or_else(|| {
                failure_envelope(
                    "E-OP-001",
                    format!("argument `{key}` must be a non-empty string"),
                    Vec::new(),
                )
            }),
    }
}

fn optional_bool(args: &Map<String, Value>, key: &str) -> Result<(), Value> {
    if let Some(value) = args.get(key) {
        if !value.is_boolean() {
            return Err(failure_envelope(
                "E-OP-001",
                format!("argument `{key}` must be a boolean"),
                Vec::new(),
            ));
        }
    }
    Ok(())
}

fn optional_string_array(args: &Map<String, Value>, key: &str) -> Result<(), Value> {
    let Some(value) = args.get(key) else {
        return Ok(());
    };
    let Some(items) = value.as_array() else {
        return Err(failure_envelope(
            "E-OP-001",
            format!("argument `{key}` must be an array of strings"),
            Vec::new(),
        ));
    };
    if items
        .iter()
        .any(|item| item.as_str().is_none_or(|value| value.trim().is_empty()))
    {
        return Err(failure_envelope(
            "E-OP-001",
            format!("argument `{key}` must contain only non-empty strings"),
            Vec::new(),
        ));
    }
    Ok(())
}

fn required_object<'a>(
    args: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a Map<String, Value>, Value> {
    args.get(key).and_then(Value::as_object).ok_or_else(|| {
        failure_envelope(
            "E-OP-001",
            format!("argument `{key}` is required and must be an object"),
            Vec::new(),
        )
    })
}

fn reject_unknown_object_keys(
    object_name: &str,
    object: &Map<String, Value>,
    allowed: &[&str],
) -> Result<(), Value> {
    if let Some(key) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(failure_envelope(
            "E-OP-001",
            format!("{object_name} does not accept argument `{key}`"),
            Vec::new(),
        ));
    }
    Ok(())
}

fn is_safe_ulid(value: &str) -> bool {
    value.len() == 26
        && value
            .bytes()
            .all(|byte| b"0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(&byte))
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
    let mut command = if exists {
        vec!["req".to_owned(), "edit".to_owned(), id.to_owned()]
    } else {
        vec![
            "req".to_owned(),
            "add".to_owned(),
            "--id".to_owned(),
            id.to_owned(),
        ]
    };
    command.extend(["--summary".to_owned(), summary.to_owned()]);
    if let Some(parent) = string_arg(args, "parent") {
        command.extend(["--parent".to_owned(), parent.to_owned()]);
    }
    if exists {
        if args.get("sections").is_some() || args.get("specs").is_some() {
            return failure_envelope(
                "E-OP-001",
                "req_upsert cannot update specs or sections for an existing REQ; use req add for those fields",
                vec!["parent".to_owned(), "summary".to_owned()],
            );
        }
    } else {
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
    let mut command = if exists {
        vec!["vo".to_owned(), "edit".to_owned(), id.to_owned()]
    } else {
        vec![
            "vo".to_owned(),
            "add".to_owned(),
            "--id".to_owned(),
            id.to_owned(),
        ]
    };
    command.extend(["--claim".to_owned(), claim.to_owned()]);
    if let Some(parent) = string_arg(args, "parent") {
        command.extend(["--parent".to_owned(), parent.to_owned()]);
    }
    if exists {
        let unsupported = [
            "combinations",
            "dimensions",
            "policy",
            "requirements",
            "sections",
            "specs",
        ];
        if let Some(field) = unsupported.iter().find(|field| args.get(**field).is_some()) {
            return failure_envelope(
                "E-OP-001",
                format!(
                    "vo_upsert cannot update {field} for an existing VO; use vo add for that field"
                ),
                vec!["claim".to_owned(), "parent".to_owned()],
            );
        }
    } else {
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
    let selectors = [
        ("test", "--test"),
        ("vo", "--vo"),
        ("req", "--req"),
        ("spec", "--spec"),
    ];
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
            path.to_string_lossy().into_owned(),
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = if stderr.trim().is_empty() {
            format!("CLI returned invalid JSON: {error}")
        } else {
            format!(
                "CLI returned invalid JSON: {error}; stderr: {}",
                stderr.trim()
            )
        };
        failure_envelope("E-CORE-001", detail, Vec::new())
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
