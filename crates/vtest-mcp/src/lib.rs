//! MCP stdio transport over the canonical v0.1 operations.
//!
//! ROOT-027 / R-1「MCP インターフェースは飾りではなく本体側」— MCP is not a
//! thin wrapper kept in sync by convention: every tool handler here calls
//! the exact same `vtest_cli::ops::*::execute` function the CLI's `run_*`
//! wrappers call, so DS-1563「別紙A（§12〜§15）が定める全 MCP tool が同じ
//! 入力に対する CLI JSON と同じ data / diagnostics を返す」holds by
//! construction rather than by two independently written code paths.
//!
//! 別紙A §13.2 defines a 23-tool MCP taxonomy (DS-1193〜DS-1229, confirmed by
//! direct citation search, team-lead ruling 2026-09-10 / レビュー #15). This
//! slice implements the 9 whose canonical operation already exists in the
//! current CLI: `scan` (DS-1193), `doc_list`/`doc_get`/`doc_upsert`
//! (DS-1194/1195 — note the canonical names: not `doc_list`/`doc_show`/
//! `doc_add`, which this module used before the rename this review round
//! corrected), `approval_create`/`approval_withdraw`/`approval_get`
//! (DS-1196/1197/1198), `run_tests` (DS-1213, not `run`), `verify`
//! (DS-1214). Tool **names** are taken verbatim from these nodes, not
//! invented to mirror the CLI subcommand name.
//!
//! `init`/`doctor` are **not** in 別紙A §13.2's 23-tool list at all — they
//! stay CLI-only (`vtest-cli`'s own subcommands) and are not exposed as MCP
//! tools here (previously they were, wrongly; removed this review round).
//!
//! The remaining 14 canonical tools this slice does not implement
//! (`vo_list`/`vo_get`/`vo_upsert`/`vo_expand`/`vo_approve`, `test_query`/
//! `test_get`/`test_create`/`test_edit`, `form_get`, `audit_static`/
//! `audit_bundle`/`audit_submit`, `report`) have no corresponding CLI
//! subcommand to mirror in this slice — building the underlying `ops::*`
//! for them is out of this task's declared scope, not an invented
//! deviation. Reported in `reports/closure-trace.md`'s unimplemented table.
//!
//! `approval_create`'s `subject_type: "judgment"` is rejected with the same
//! disclosed error the CLI uses (`vtest_cli::ops::approval`'s module doc
//! comment) — no judgment-record domain exists in this codebase.
//!
//! An MCP-only Structured-Edit tool with an apply/re-verify/rollback
//! contract (別紙A §15.2/§15.4, E-OP-003: apply-then-verify failure —
//! unparseable result, generated declaration mismatched against desired
//! state, or a change exceeding one Test's range — rolls back to the
//! pre-apply byte sequence and aborts, leaving no Test ID / Evidence /
//! judgment-record side effect) is a real, non-empty citation in
//! `docs/canonical/specification.json` (confirmed present at commit
//! `58d03fb`; an earlier version of this comment, checked against `79e43fa`,
//! wrongly reported it absent — see `reports/closure-trace.md` for the
//! correction). It is not implemented here: it is a Create/Edit tool over
//! Structured Test Operations, a CLI/MCP surface this closure-slice's
//! declared scope (parity with `vtest-cli`'s existing `init`/`scan`/
//! `doctor`/`run`/`verify`/`doc`/`approval` subcommands) does not cover —
//! `vtest-cli` itself has no `create`/`edit` subcommand to mirror. Reported
//! as declined (out of this task's scope), not as an upstream silence.

use std::{
    fs,
    io::{self, BufRead, Write},
    path::Path,
    time::UNIX_EPOCH,
};

use serde_json::{json, Map, Value};
use vtest_cli::ops;
use vtest_model::ExitCode;

/// 正本 別紙A §13.2 の23 tool taxonomy のうち、この closure-slice が実装する
/// 9件（DS-1193/1194/1195/1196/1197/1198/1213/1214 相当）。`init`/`doctor`
/// は正本の23 tool一覧に無いため、CLI には残しつつ MCP からは外した（team-
/// lead ruling 2026-09-10, レビュー #15）。未登録14 tool は
/// `reports/closure-trace.md` の未実装表に開示している。
const TOOL_NAMES: &[&str] = &[
    "scan",
    "run_tests",
    "verify",
    "approval_create",
    "approval_withdraw",
    "approval_get",
    "doc_upsert",
    "doc_list",
    "doc_get",
];

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
    let envelope = failure_envelope(diagnostic_code, message.clone());
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
        ));
    };
    if let Some(key) = params
        .keys()
        .find(|key| *key != "name" && *key != "arguments")
    {
        return tool_result(failure_envelope(
            "E-OP-001",
            format!("tools/call does not accept parameter `{key}`"),
        ));
    }
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return tool_result(failure_envelope(
            "E-OP-001",
            "tools/call requires string name",
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
        ));
    };
    if !TOOL_NAMES.contains(&name) {
        return tool_result(failure_envelope(
            "E-OP-001",
            format!("unknown MCP tool `{name}`"),
        ));
    }
    if let Err(error) = validate_tool_arguments(name, arguments) {
        return tool_result(error);
    }
    // Every tool re-scans internally via `ops::*::execute` (which itself
    // calls `scan_project`), so — unlike the predecessor transport that
    // shelled out to a separately-scanning CLI process per call — no
    // freshness check is needed before dispatch. It is kept only to expose
    // an explicit `scan` no-op fast path for a caller that wants a single
    // freshness probe without paying for a `verify`/`run`.
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
    let registry = match resolve_registry() {
        Ok(registry) => registry,
        Err(envelope) => return Some(envelope),
    };
    let (_, scan) = ops::scan::execute(root, &registry);
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
        "scan" => (json!({}), Vec::new()),
        "run_tests" => (
            json!({
                "test": {"type": "array", "items": {"type": "string"}},
                "vo": {"type": "string"},
                "all": {"type": "boolean"},
                "fast": {"type": "boolean"}
            }),
            Vec::new(),
        ),
        "verify" => (
            json!({
                "items": {"type": "array", "items": {"type": "string"}},
                "doc": {"type": "string"},
                "vo": {"type": "string"},
                "test": {"type": "string"},
                "gate": {"type": "string"},
                "summary": {"type": "boolean"}
            }),
            Vec::new(),
        ),
        "approval_create" => (
            json!({
                "subject": {
                    "type": "object",
                    "properties": {
                        "type": {"type": "string"},
                        "id": {"type": "string"}
                    }
                },
                "state": {"type": "string"},
                "approver": {
                    "type": "object",
                    "properties": {
                        "kind": {"type": "string"},
                        "id": {"type": "string"},
                        "model": {"type": "string"}
                    }
                },
                "basis": {"type": "array", "items": {"type": "string"}},
                "supersedes": {"type": "array", "items": {"type": "string"}}
            }),
            vec!["subject", "state", "approver"],
        ),
        "approval_withdraw" => (
            json!({
                "approval_id": {"type": "string"},
                "approver": {
                    "type": "object",
                    "properties": {
                        "kind": {"type": "string"},
                        "id": {"type": "string"},
                        "model": {"type": "string"}
                    }
                },
                "basis": {"type": "array", "items": {"type": "string"}}
            }),
            vec!["approval_id", "approver"],
        ),
        "approval_get" => (
            json!({
                "subject": {
                    "type": "object",
                    "properties": {
                        "type": {"type": "string"},
                        "id": {"type": "string"}
                    }
                }
            }),
            vec!["subject"],
        ),
        "doc_upsert" => (
            json!({
                "id": {"type": "string"},
                "path": {"type": "string"},
                "derives_from": {"type": "array", "items": {"type": "string"}},
                "root": {"type": "boolean"},
                "update": {"type": "boolean"}
            }),
            vec!["id", "path"],
        ),
        // DS-1194: both tools accept `tree`/`roots`; `doc_get` additionally
        // requires `id`.
        "doc_list" => (
            json!({"tree": {"type": "boolean"}, "roots": {"type": "boolean"}}),
            Vec::new(),
        ),
        "doc_get" => (
            json!({
                "id": {"type": "string"},
                "tree": {"type": "boolean"},
                "roots": {"type": "boolean"}
            }),
            vec!["id"],
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

fn validate_tool_arguments(name: &str, args: &Map<String, Value>) -> Result<(), Value> {
    let allowed: &[&str] = match name {
        "scan" => &[],
        "run_tests" => &["test", "vo", "all", "fast"],
        "verify" => &["items", "doc", "vo", "test", "gate", "summary"],
        "approval_create" => &["subject", "state", "approver", "basis", "supersedes"],
        "approval_withdraw" => &["approval_id", "approver", "basis"],
        "approval_get" => &["subject"],
        "doc_upsert" => &["id", "path", "derives_from", "root", "update"],
        "doc_list" => &["tree", "roots"],
        "doc_get" => &["id", "tree", "roots"],
        _ => &[],
    };
    if let Some(key) = args.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(failure_envelope(
            "E-OP-001",
            format!("{name} does not accept argument `{key}`"),
        ));
    }
    match name {
        "scan" => Ok(()),
        "run_tests" => {
            optional_string_array(args, "test")?;
            optional_nonempty_string(args, "vo")?;
            optional_bool(args, "all")?;
            optional_bool(args, "fast")
        }
        "verify" => {
            optional_string_array(args, "items")?;
            for key in ["doc", "vo", "test", "gate"] {
                optional_nonempty_string(args, key)?;
            }
            optional_bool(args, "summary")
        }
        // `approval_*` argument presence/type is checked by the tool
        // functions themselves (`approval_create_tool`/etc, via
        // `Value::pointer`) since their shape is nested, not flat — this
        // layer only enforces the flat allowed-key set above.
        "approval_create" | "approval_withdraw" | "approval_get" => Ok(()),
        "doc_upsert" => {
            optional_nonempty_string(args, "id")?;
            optional_nonempty_string(args, "path")?;
            optional_string_array(args, "derives_from")?;
            optional_bool(args, "root")?;
            optional_bool(args, "update")
        }
        "doc_list" => {
            optional_bool(args, "tree")?;
            optional_bool(args, "roots")
        }
        "doc_get" => {
            optional_nonempty_string(args, "id")?;
            optional_bool(args, "tree")?;
            optional_bool(args, "roots")
        }
        _ => Ok(()),
    }
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
        ));
    };
    if items.iter().any(|item| item.as_str().is_none()) {
        return Err(failure_envelope(
            "E-OP-001",
            format!("argument `{key}` must contain only strings"),
        ));
    }
    Ok(())
}

/// Dispatches one MCP tool call to the same `vtest_cli::ops::*::execute`
/// function the CLI's `run_*` wrapper for the equivalent subcommand calls,
/// in-process — no subprocess, no second implementation of the operation.
fn dispatch_tool(root: &Path, name: &str, args: &Value) -> Value {
    match name {
        "scan" => match resolve_registry() {
            Ok(registry) => ops::scan::execute(root, &registry).1,
            Err(envelope) => envelope,
        },
        "run_tests" => run_tool(root, args),
        "verify" => verify_tool(root, args),
        "approval_create" => approval_create_tool(root, args),
        "approval_withdraw" => approval_withdraw_tool(root, args),
        "approval_get" => approval_get_tool(root, args),
        "doc_upsert" => doc_add_tool(root, args),
        "doc_list" => doc_list_tool(root, args),
        "doc_get" => doc_show_tool(root, args),
        _ => failure_envelope("E-OP-001", format!("unknown MCP tool `{name}`")),
    }
}

/// BD-305: `approval_create`（`subject: { type, id }`）— the same canonical
/// creation path `vtest approval create` calls (`ops::approval::create`).
fn approval_create_tool(root: &Path, args: &Value) -> Value {
    let layout = vtest_store::VerifyLayout::new(root);
    let Some(subject_type) = args.pointer("/subject/type").and_then(Value::as_str) else {
        return failure_envelope("E-OP-001", "approval_create requires subject.type");
    };
    let Some(subject_id) = args.pointer("/subject/id").and_then(Value::as_str) else {
        return failure_envelope("E-OP-001", "approval_create requires subject.id");
    };
    let Some(state) = string_arg(args, "state") else {
        return failure_envelope("E-OP-001", "approval_create requires state");
    };
    let Some(approver_kind) = args.pointer("/approver/kind").and_then(Value::as_str) else {
        return failure_envelope("E-OP-001", "approval_create requires approver.kind");
    };
    let Some(approver_id) = args.pointer("/approver/id").and_then(Value::as_str) else {
        return failure_envelope("E-OP-001", "approval_create requires approver.id");
    };
    let approver_model = args
        .pointer("/approver/model")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let basis = string_array_arg(args, "basis");
    let supersedes = string_array_arg(args, "supersedes");

    let result = ops::approval::create(
        &layout,
        ops::approval::CreateArgs {
            subject_type: subject_type.to_owned(),
            subject_id: subject_id.to_owned(),
            approved_state: state.to_owned(),
            approver_kind: approver_kind.to_owned(),
            approver_id: approver_id.to_owned(),
            approver_model,
            basis,
            supersedes,
        },
    );
    approval_record_envelope(result)
}

/// BD-307: `approval_withdraw` — same canonical path as `vtest approval
/// withdraw`.
fn approval_withdraw_tool(root: &Path, args: &Value) -> Value {
    let layout = vtest_store::VerifyLayout::new(root);
    let Some(approval_id) = string_arg(args, "approval_id") else {
        return failure_envelope("E-OP-001", "approval_withdraw requires approval_id");
    };
    let Some(approver_kind) = args.pointer("/approver/kind").and_then(Value::as_str) else {
        return failure_envelope("E-OP-001", "approval_withdraw requires approver.kind");
    };
    let Some(approver_id) = args.pointer("/approver/id").and_then(Value::as_str) else {
        return failure_envelope("E-OP-001", "approval_withdraw requires approver.id");
    };
    let approver_model = args
        .pointer("/approver/model")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let basis = string_array_arg(args, "basis");

    let result = ops::approval::withdraw(
        &layout,
        ops::approval::WithdrawArgs {
            approval_id: approval_id.to_owned(),
            approver_kind: approver_kind.to_owned(),
            approver_id: approver_id.to_owned(),
            approver_model,
            basis,
        },
    );
    approval_record_envelope(result)
}

fn approval_record_envelope(
    result: Result<vtest_store::records::ApprovalRecord, ops::approval::ApprovalOpError>,
) -> Value {
    match result {
        Ok(record) => success_envelope(
            true,
            json!({
                "id": record.id,
                "subject_type": record.subject_type,
                "subject": record.subject,
                "approved_state": record.approved_state,
                "supersedes": record.supersedes,
                "approved_at": record.approved_at,
            }),
            &[],
        ),
        Err(error) => approval_error_envelope(&error),
    }
}

fn approval_error_envelope(error: &ops::approval::ApprovalOpError) -> Value {
    use ops::approval::ApprovalOpError;
    match error {
        // DS-1058: see the identical CLI-side comment in
        // `vtest_cli::approval_error_exit` — this is DS-1058's unresolved-
        // subject case (E-APPROVAL-001), not an E-OP-001 input-validation
        // failure.
        ApprovalOpError::JudgmentSubjectTypeUnsupported => {
            failure_envelope("E-APPROVAL-001", error.to_string())
        }
        ApprovalOpError::UnresolvedSubject(_) => {
            failure_envelope("E-APPROVAL-001", error.to_string())
        }
        ApprovalOpError::InvalidRequest(_) => failure_envelope("E-APPROVAL-002", error.to_string()),
        ApprovalOpError::Store(_) => failure_envelope("E-CORE-001", error.to_string()),
    }
}

/// BD-308: `approval_get` — the subject's full record history plus its
/// current effective承認 state.
fn approval_get_tool(root: &Path, args: &Value) -> Value {
    let layout = vtest_store::VerifyLayout::new(root);
    let Some(subject_type) = args.pointer("/subject/type").and_then(Value::as_str) else {
        return failure_envelope("E-OP-001", "approval_get requires subject.type");
    };
    let Some(subject_id) = args.pointer("/subject/id").and_then(Value::as_str) else {
        return failure_envelope("E-OP-001", "approval_get requires subject.id");
    };
    match ops::approval::show(&layout, subject_type, subject_id) {
        Ok(result) => {
            let effective = match result.effective_state {
                vtest_store::approval::EffectiveApprovalState::Draft => "draft",
                vtest_store::approval::EffectiveApprovalState::Approved => "approved",
            };
            success_envelope(
                true,
                json!({
                    "records": result.records.iter().map(|record| json!({
                        "id": record.id,
                        "subject_type": record.subject_type,
                        "subject": record.subject,
                        "approved_state": record.approved_state,
                        "supersedes": record.supersedes,
                        "approved_at": record.approved_at,
                    })).collect::<Vec<_>>(),
                    "effective_state": effective,
                }),
                &[],
            )
        }
        Err(error) => approval_error_envelope(&error),
    }
}

/// DS-1003/1681 — same canonical path as `vtest doc add`. `derives_from`
/// here is a bare array of upstream node id strings, matching the CLI's
/// `--derives-from` (repeatable flag) shape rather than the retired
/// `[{doc, anchor?, note?}]` per-registry-record shape.
fn doc_add_tool(root: &Path, args: &Value) -> Value {
    let layout = vtest_store::VerifyLayout::new(root);
    let Some(id) = string_arg(args, "id") else {
        return failure_envelope("E-OP-001", "doc_add requires id");
    };
    let Some(path) = string_arg(args, "path") else {
        return failure_envelope("E-OP-001", "doc_add requires path");
    };
    let update = bool_arg(args, "update");
    // DS-1685: distinguish "derives_from key absent" (None -- leave
    // unchanged) from "derives_from: []" (Some(empty) -- replace with
    // empty), which JSON can express and the CLI's repeatable-value flag
    // cannot (see `AddArgs::derives_from`'s doc comment).
    let derives_from: Option<Vec<String>> = args.get("derives_from").map(|value| {
        value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect()
    });
    // DS-1195: `root` is a plain bool (absent = false, the JSON-schema
    // default for an unset boolean property).
    let root_arg = bool_arg(args, "root");
    // DS-1683/BD-331 (`233caec`/PR #50): root designation is fixed at
    // initial registration only -- `root` given at all alongside
    // `update: true` is rejected by `ops::doc::add` (via
    // `root_specified`), regardless of its value.
    let root_specified = args.get("root").is_some();

    match ops::doc::add(
        root,
        &layout,
        ops::doc::AddArgs {
            id: id.to_owned(),
            path: path.to_owned(),
            derives_from,
            root: root_arg,
            root_specified,
            update,
        },
    ) {
        Ok(view) => success_envelope(true, doc_view_json(&view), &[]),
        Err(error) => doc_error_envelope(&error),
    }
}

/// DS-1194: `tree`/`roots` are an input axis, not decoration -- the CLI's
/// own `doc list --tree`/`--roots` (`render_doc_list_text`) switch what is
/// rendered rather than always rendering everything, and this tool must
/// do the same instead of unconditionally emitting every field regardless
/// of what was asked for.
///
/// `roots: true` -> only `roots` (mirrors the CLI's own early return for
/// `--roots`, which does not also render the record list). `tree: true`
/// -> `records` + `document_chain` (the tree view needs both: the chain
/// to draw, the records to know every registered id). Neither given ->
/// the CLI's own default flat listing, `records` alone. `unresolved_
/// derives_from` (DS-1018) is not view-gated in the CLI (`render_doc_
/// list_text` always appends it after the tree/flat/roots branch), so it
/// is always present here too.
fn doc_list_tool(root: &Path, args: &Value) -> Value {
    let layout = vtest_store::VerifyLayout::new(root);
    let want_tree = bool_arg(args, "tree");
    let want_roots = bool_arg(args, "roots");
    match ops::doc::list(&layout) {
        Ok(result) => {
            let mut data = json!({
                "unresolved_derives_from": result.unresolved,
            });
            if want_roots {
                data["roots"] = json!(result
                    .records
                    .iter()
                    .filter(|view| view.is_root)
                    .map(|view| view.id.clone())
                    .collect::<Vec<_>>());
            } else {
                let records: Vec<_> = result.records.iter().map(doc_view_json).collect();
                data["records"] = json!(records);
                data["freshness"] = json!(result.freshness);
                if want_tree {
                    data["document_chain"] = json!(result.document_chain);
                }
            }
            success_envelope(true, data, &[])
        }
        Err(error) => doc_error_envelope(&error),
    }
}

/// DS-1194: `doc_get` accepts `tree`/`roots` alongside `id` (the same
/// input axis `doc_list` takes). Since a single-document `show` has no
/// registered-set context of its own, `tree`/`roots` reuse `ops::doc::list`
/// to compute the same corpus-wide `document_chain`/`roots` `doc_list`
/// would, and add just this document's own slice of them -- `document_
/// chain` keyed to `id`'s own resolved parent document ids, `roots` as the
/// full current root set (there being no single document's "own" root set
/// to narrow it to).
fn doc_show_tool(root: &Path, args: &Value) -> Value {
    let layout = vtest_store::VerifyLayout::new(root);
    let Some(id) = string_arg(args, "id") else {
        return failure_envelope("E-OP-001", "doc_show requires id");
    };
    let want_tree = bool_arg(args, "tree");
    let want_roots = bool_arg(args, "roots");
    match ops::doc::show(&layout, id) {
        Ok(result) => {
            let mut data = doc_view_json(&result.view);
            // DS-1017 new: `freshness` is not part of `doc_view_json`'s
            // base shape at all (see that function's own doc comment) --
            // this is the one place that actually computed it, via
            // `ops::doc::show`'s per-node, cross-referencing computation
            // (see `ops::doc::ShowResult`'s doc comment).
            data["freshness"] = json!(result.freshness);
            data["approval_states"] = json!(result.approval_states);
            if want_tree || want_roots {
                match ops::doc::list(&layout) {
                    Ok(list_result) => {
                        if want_tree {
                            data["document_chain"] = json!(list_result
                                .document_chain
                                .get(id)
                                .cloned()
                                .unwrap_or_default());
                        }
                        if want_roots {
                            data["roots"] = json!(list_result
                                .records
                                .iter()
                                .filter(|record| record.is_root)
                                .map(|record| record.id.clone())
                                .collect::<Vec<_>>());
                        }
                    }
                    Err(error) => return doc_error_envelope(&error),
                }
            }
            success_envelope(true, data, &[])
        }
        Err(error) => doc_error_envelope(&error),
    }
}

// DS-1017 new/DS-1194: `freshness` is deliberately not part of this base
// JSON shape -- see the identical CLI-side comment on `vtest_cli`'s own
// `doc_view_json`.
fn doc_view_json(view: &vtest_store::doc_registry::DocView) -> Value {
    json!({
        "id": view.id,
        "path": view.path.to_string_lossy(),
        "content_hash": view.content_hash.as_str(),
        "derives_from": view.derives_from,
        "root": view.is_root,
    })
}

fn doc_error_envelope(error: &ops::doc::DocOpError) -> Value {
    match error {
        ops::doc::DocOpError::Usage(_) => failure_envelope("E-OP-001", error.to_string()),
        ops::doc::DocOpError::Store(_) => failure_envelope("E-CORE-001", error.to_string()),
    }
}

fn string_array_arg(args: &Value, key: &str) -> Vec<String> {
    args.get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn run_tool(root: &Path, args: &Value) -> Value {
    let test_ids = string_array_arg(args, "test");
    let vo = string_arg(args, "vo").map(str::to_owned);
    let all = bool_arg(args, "all");
    let fast = bool_arg(args, "fast");

    // Mirrors `vtest-cli`'s `run_run`: resolve config + scan, then hand the
    // same `ops::run::run` the CLI calls the resolved scan and target.
    let config = vtest_store::load_config(root);
    if let Err(error) = config {
        return failure_envelope("E-CONFIG-001", error.to_string());
    }
    let registry = match resolve_registry() {
        Ok(registry) => registry,
        Err(envelope) => return envelope,
    };
    let scan = match vtest_scan::scan_project(root, &registry) {
        Ok(scan) => scan,
        Err(error) => {
            let code = error.code().unwrap_or("E-CORE-001");
            return failure_envelope(code, error.to_string());
        }
    };
    let layout = vtest_store::VerifyLayout::new(root);
    let target = if let Some(vo_id) = vo {
        ops::run::RunTarget::Vo(vo_id)
    } else if all {
        ops::run::RunTarget::All
    } else {
        ops::run::RunTarget::Test(test_ids)
    };
    match ops::run::run(root, &layout, &scan, &target, fast, &registry) {
        Ok(result) => {
            let has_errors = result.has_errors();
            let data = json!({
                "evidence": result.evidence.len(),
                "evidence_ids": result.evidence.iter().map(|record| record.id.clone()).collect::<Vec<_>>(),
                "fast": fast,
            });
            success_envelope(!has_errors, data, &result.diagnostics)
        }
        Err(
            error @ (ops::run::RunOpError::UnknownTestId(_) | ops::run::RunOpError::UnknownVoId(_)),
        ) => failure_envelope("E-OP-001", error.to_string()),
        Err(ref error @ ops::run::RunOpError::Execution(ref inner)) => {
            // DS-745/DS-923 (E-ADAPTER-003) — mirrors the CLI's `run_run`
            // mapping (`vtest-cli/src/lib.rs`) so MCP and CLI report the
            // same code for the same input (DS-1563).
            match inner.code() {
                Some(code) => failure_envelope(code, error.to_string()),
                None => failure_envelope("E-CORE-001", error.to_string()),
            }
        }
        Err(error @ ops::run::RunOpError::Store(_)) => {
            failure_envelope("E-CORE-001", error.to_string())
        }
    }
}

fn verify_tool(root: &Path, args: &Value) -> Value {
    let items = args
        .get("items")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let doc = string_arg(args, "doc").map(str::to_owned);
    let vo = string_arg(args, "vo").map(str::to_owned);
    let test = string_arg(args, "test").map(str::to_owned);
    let gate = string_arg(args, "gate");
    let summary = bool_arg(args, "summary");

    let registry = match resolve_registry() {
        Ok(registry) => registry,
        Err(envelope) => return envelope,
    };
    match ops::verify::execute(root, &items, doc, vo, test, gate, summary, &registry) {
        Ok((exit, data, diagnostics)) => success_envelope(exit == ExitCode::Ok, data, &diagnostics),
        Err(ops::verify::VerifyOpError::Usage { code, message }) => failure_envelope(code, message),
        Err(ops::verify::VerifyOpError::Scan(error)) => {
            let code = error.code().unwrap_or("E-CORE-001");
            failure_envelope(code, error.to_string())
        }
    }
}

fn success_envelope<T: serde::Serialize>(
    ok: bool,
    data: T,
    diagnostics: &[vtest_model::Diagnostic],
) -> Value {
    json!({
        "ok": ok,
        "data": data,
        "diagnostics": diagnostics
    })
}

fn string_arg<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(Value::as_str)
}

fn bool_arg(args: &Value, key: &str) -> bool {
    args.get(key).and_then(Value::as_bool).unwrap_or(false)
}

/// composition root（`vtest_cli::adapters::builtin_registry`）を呼ぶ MCP側
/// の入口。cli の `resolve_registry` と同じ理由（BD-007/BD-102、
/// `vtest-scan`/`vtest-verify`/`vtest-exec` は自分で adapter を生成しない）
/// で、MCP tool 側もこの1箇所だけで registry を組み立てる。失敗
/// （理論上は固定1件の登録なので到達しない）は E-ADAPTER-001 として返す。
fn resolve_registry() -> Result<vtest_adapter_api::AdapterRegistry, Value> {
    vtest_cli::adapters::builtin_registry()
        .map_err(|error| failure_envelope("E-ADAPTER-001", error.to_string()))
}

fn failure_envelope(code: &str, message: impl Into<String>) -> Value {
    json!({
        "ok": false,
        "data": null,
        "diagnostics": [{
            "code": code,
            "severity": "error",
            "message": message.into()
        }]
    })
}

#[cfg(test)]
mod tests {
    //! DS-1563 equivalence: the MCP tool handler and the CLI wrapper for
    //! the same operation must return the same `data` / `diagnostics` for
    //! the same input. `dispatch_tool` (MCP side) and `vtest_cli::run`
    //! (CLI side) both bottom out in `vtest_cli::ops::verify::execute` /
    //! `ops::scan::execute`; this test drives both entry points and
    //! compares their JSON, so a future edit that special-cases one path
    //! (instead of changing the shared `ops::*` function) breaks the test
    //! rather than silently diverging.

    use super::*;
    use std::path::PathBuf;

    fn temp_root(name: &str) -> PathBuf {
        // A nanosecond-timestamp suffix alone collides under parallel test
        // execution on Windows' coarser clock resolution -- see
        // `vtest-scan`'s `fixture()` doc comment for the confirmed root
        // cause of a previously-unconfirmed flaky failure elsewhere in
        // this workspace.
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "vtest-mcp-{name}-{}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("create fixture root");
        root
    }

    /// Runs the CLI's `run_verify` wrapper end to end (through
    /// `vtest_cli::run`, exactly as the `vtest` binary does) and returns
    /// the JSON envelope it would have printed with `--format json`, by
    /// reconstructing it the same way `vtest_cli`'s own `run_verify` does:
    /// call the identical `ops::verify::execute` the CLI wrapper calls.
    /// (`vtest_cli::run` itself only prints and returns an `ExitCode` — it
    /// has no library entry point that hands back the envelope value, so
    /// the exit code is asserted to match separately below.)
    fn cli_verify_envelope(root: &Path) -> (ExitCode, Value) {
        match vtest_cli::ops::verify::execute(
            root,
            &[],
            None,
            None,
            None,
            None,
            false,
            &vtest_cli::adapters::builtin_registry().expect("builtin registry"),
        ) {
            Ok((exit, data, diagnostics)) => {
                let ok = exit == ExitCode::Ok;
                (
                    exit,
                    json!({"ok": ok, "data": data, "diagnostics": diagnostics}),
                )
            }
            Err(vtest_cli::ops::verify::VerifyOpError::Usage { code, message }) => {
                (ExitCode::Usage, failure_envelope(code, message))
            }
            Err(vtest_cli::ops::verify::VerifyOpError::Scan(error)) => {
                let code = error.code().unwrap_or("E-CORE-001");
                (ExitCode::Usage, failure_envelope(code, error.to_string()))
            }
        }
    }

    /// @vtest.id TEST-MCP-VERIFY-MATCHES-CLI
    /// @vtest.covers VO-MCP-TOOL-CLI-EQUIVALENCE
    /// @vtest.target crates/vtest-mcp/src/lib.rs::dispatch_tool
    /// @vtest.intent MCP `verify` tool returns the same envelope (data/diagnostics) as the CLI verify operation for the same input.
    #[test]
    fn mcp_verify_tool_matches_the_cli_verify_operation() {
        let root = temp_root("verify-equivalence");
        let init = vtest_cli::run(vtest_cli::Cli {
            project: root.clone(),
            format: vtest_cli::OutputFormat::Json,
            quiet: true,
            command: vtest_cli::Command::Init { name: None },
        });
        assert_eq!(init, ExitCode::Ok, "fixture project must initialise");

        let (cli_exit, cli_envelope) = cli_verify_envelope(&root);

        let mcp_envelope = dispatch_tool(&root, "verify", &json!({}));

        assert_eq!(
            mcp_envelope, cli_envelope,
            "MCP `verify` tool must return the same envelope as the CLI `verify` operation"
        );
        assert_eq!(
            mcp_envelope.get("ok"),
            Some(&Value::Bool(cli_exit == ExitCode::Ok)),
            "MCP `ok` must agree with the CLI exit code's OK/NG meaning"
        );
    }

    /// @vtest.id TEST-MCP-SCAN-MATCHES-CLI
    /// @vtest.covers VO-MCP-TOOL-CLI-EQUIVALENCE
    /// @vtest.target crates/vtest-mcp/src/lib.rs::dispatch_tool
    /// @vtest.intent MCP `scan` tool returns the same envelope as `ops::scan::execute`, which the CLI `run_scan` wrapper also calls.
    #[test]
    fn mcp_scan_tool_matches_the_cli_scan_operation() {
        let root = temp_root("scan-equivalence");
        let init = vtest_cli::run(vtest_cli::Cli {
            project: root.clone(),
            format: vtest_cli::OutputFormat::Json,
            quiet: true,
            command: vtest_cli::Command::Init { name: None },
        });
        assert_eq!(init, ExitCode::Ok, "fixture project must initialise");

        let (_, cli_envelope) = ops::scan::execute(
            &root,
            &vtest_cli::adapters::builtin_registry().expect("builtin registry"),
        );
        let mcp_envelope = dispatch_tool(&root, "scan", &json!({}));

        assert_eq!(
            mcp_envelope, cli_envelope,
            "MCP `scan` tool must return the same envelope as `ops::scan::execute`, \
             which the CLI's `run_scan` wrapper also calls"
        );
    }

    fn fixture_vo_project(root: &Path) {
        use vtest_model::{
            DerivesFrom, DocumentFile, DocumentId, NodeSource, RootNode, SentenceNode, VoId,
            VoRecord,
        };
        use vtest_store::{init_project, write_document_file, write_vo_record};

        let layout = init_project(root, "vtest-mcp-approval-fixture").expect("init .verify/");
        let source = NodeSource {
            doc: "fixture.md".to_owned(),
            heading: "fixture".to_owned(),
            lines: [1, 1],
        };
        let document = DocumentFile {
            schema_version: "0.1".to_owned(),
            root: vec![RootNode {
                id: DocumentId::new("ROOT-001"),
                statement: "fixture root".to_owned(),
                description: None,
                source: source.clone(),
            }],
            request: vec![SentenceNode {
                id: DocumentId::new("R-001"),
                statement: "fixture requirement".to_owned(),
                description: None,
                derives_from: vec![DocumentId::new("ROOT-001")],
                cites: None,
                source,
            }],
            require: Vec::new(),
            spec: Vec::new(),
            detailed_spec: Vec::new(),
            basic_design: Vec::new(),
            design: Vec::new(),
        };
        write_document_file(&layout, "fixture", &document).expect("write document file");
        write_vo_record(
            &layout,
            &VoRecord {
                id: VoId::new("VO-MCP-APPROVAL"),
                parent: None,
                derives_from: vec![DerivesFrom {
                    doc: DocumentId::new("R-001"),
                    anchor: None,
                    note: None,
                }],
                claim: "fixture claim".to_owned(),
                dimensions: Vec::new(),
                coverage_policy: None,
                combinations: Vec::new(),
                representative_cases: Vec::new(),
                created: "2026-09-09T00:00:00Z".to_owned(),
                updated: "2026-09-09T00:00:00Z".to_owned(),
            },
        )
        .expect("write VO record");
    }

    /// DS-1563 equivalence for the Approval domain: `approval_get` (MCP) and
    /// `ops::approval::show` (the same function the CLI's `approval show`
    /// wrapper calls) must return the same effective state and record
    /// history for the same on-disk records — this only compares reads
    /// (`show`/`approval_get`), not `create`/`approval_create`, because a
    /// freshly created record's `id`/`approved_at` are non-deterministic
    /// (ULID + timestamp) and would never compare equal across two
    /// independently invoked creations.
    ///
    /// @vtest.id TEST-MCP-APPROVAL-GET-MATCHES-CLI
    /// @vtest.covers VO-MCP-TOOL-CLI-EQUIVALENCE
    /// @vtest.target crates/vtest-mcp/src/lib.rs::dispatch_tool
    /// @vtest.intent MCP `approval_get` returns the same envelope as `ops::approval::show`, which the CLI `approval show` wrapper also calls.
    #[test]
    fn mcp_approval_get_tool_matches_the_ops_approval_show_operation() {
        let root = temp_root("approval-equivalence");
        fixture_vo_project(&root);
        let layout = vtest_store::VerifyLayout::new(&root);

        ops::approval::create(
            &layout,
            ops::approval::CreateArgs {
                subject_type: "vo".to_owned(),
                subject_id: "VO-MCP-APPROVAL".to_owned(),
                approved_state: "approved".to_owned(),
                approver_kind: "human".to_owned(),
                approver_id: "reviewer".to_owned(),
                approver_model: None,
                basis: Vec::new(),
                supersedes: Vec::new(),
            },
        )
        .expect("create must succeed against a resolvable VO subject");

        let direct = ops::approval::show(&layout, "vo", "VO-MCP-APPROVAL")
            .expect("direct ops::approval::show must succeed");
        let direct_effective = match direct.effective_state {
            vtest_store::approval::EffectiveApprovalState::Draft => "draft",
            vtest_store::approval::EffectiveApprovalState::Approved => "approved",
        };
        let direct_envelope = success_envelope(
            true,
            json!({
                "records": direct.records.iter().map(|record| json!({
                    "id": record.id,
                    "subject_type": record.subject_type,
                    "subject": record.subject,
                    "approved_state": record.approved_state,
                    "supersedes": record.supersedes,
                    "approved_at": record.approved_at,
                })).collect::<Vec<_>>(),
                "effective_state": direct_effective,
            }),
            &[],
        );

        let mcp_envelope = dispatch_tool(
            &root,
            "approval_get",
            &json!({"subject": {"type": "vo", "id": "VO-MCP-APPROVAL"}}),
        );

        // DS-1563: same input, same root, same records already on disk --
        // the full envelope (data + diagnostics) must match exactly, not
        // merely a couple of hand-picked fields.
        assert_eq!(
            mcp_envelope, direct_envelope,
            "MCP `approval_get` must return the same envelope (data + diagnostics) as the \
             shared `ops::approval::show` the CLI `approval show` wrapper also calls"
        );
    }

    /// DS-1563 equivalence for the Document registry: `doc_show` (MCP) vs
    /// `ops::doc::show` (the shared function the CLI `doc show` wrapper also
    /// calls).
    /// @vtest.id TEST-MCP-DOC-GET-MATCHES-CLI
    /// @vtest.covers VO-MCP-TOOL-CLI-EQUIVALENCE
    /// @vtest.target crates/vtest-mcp/src/lib.rs::dispatch_tool
    /// @vtest.intent MCP `doc_get` returns the same envelope as `ops::doc::show`, which the CLI `doc show` wrapper also calls (also exercises DS-1194 tree/roots population).
    #[test]
    fn mcp_doc_get_tool_matches_the_ops_doc_show_operation() {
        let root = temp_root("doc-equivalence");
        let init = vtest_cli::run(vtest_cli::Cli {
            project: root.clone(),
            format: vtest_cli::OutputFormat::Json,
            quiet: true,
            command: vtest_cli::Command::Init { name: None },
        });
        assert_eq!(init, ExitCode::Ok, "fixture project must initialise");
        // DES-595: `--path` names an already-built node-tree JSON file, the
        // same shape `.verify/doc/<name>.json` already uses.
        fs::write(
            root.join("basic-spec.json"),
            r#"{"schema_version":"0.1","root":[{"id":"ROOT-001","statement":"fixture root","source":{"doc":"fixture.md","heading":"fixture","lines":[1,1]}}],"request":[],"require":[],"spec":[],"detailed_spec":[],"basic_design":[],"design":[]}"#,
        )
        .expect("write source file");

        let layout = vtest_store::VerifyLayout::new(&root);
        ops::doc::add(
            &root,
            &layout,
            ops::doc::AddArgs {
                id: "DOC-BASIC-001".to_owned(),
                path: "basic-spec.json".to_owned(),
                derives_from: None,
                root: false,
                root_specified: false,
                update: false,
            },
        )
        .expect("add must succeed against a real node-tree file");

        let direct =
            ops::doc::show(&layout, "DOC-BASIC-001").expect("direct ops::doc::show must succeed");
        let mut direct_data = doc_view_json(&direct.view);
        direct_data["freshness"] = json!(direct.freshness);
        direct_data["approval_states"] = json!(direct.approval_states);
        let direct_envelope = success_envelope(true, direct_data, &[]);
        let mcp_envelope = dispatch_tool(&root, "doc_get", &json!({"id": "DOC-BASIC-001"}));

        // DS-1563: same root, same registered document -- content_hash is
        // deterministic (a pure function of the file's own content), so
        // the full envelope must match exactly, not merely a couple of
        // hand-picked fields.
        assert_eq!(
            mcp_envelope, direct_envelope,
            "MCP `doc_get` must return the same envelope (data + diagnostics) as the shared \
             `ops::doc::show` the CLI `doc show` wrapper also calls"
        );

        // DS-1194: `doc_get` accepts `tree`/`roots` (previously rejected
        // with E-OP-001) and populates `document_chain`/`roots`.
        let with_tree_and_roots = dispatch_tool(
            &root,
            "doc_get",
            &json!({"id": "DOC-BASIC-001", "tree": true, "roots": true}),
        );
        assert_eq!(with_tree_and_roots["ok"], Value::Bool(true));
        assert!(
            with_tree_and_roots["data"]["document_chain"].is_array(),
            "DS-1194: doc_get with tree:true must populate document_chain, got {with_tree_and_roots:?}"
        );
        assert_eq!(
            with_tree_and_roots["data"]["roots"],
            json!(["DOC-BASIC-001"]),
            "DS-1194: doc_get with roots:true must populate the current root set"
        );
    }

    /// DS-1563 equivalence for `approval_create`: MCP and `ops::approval::
    /// create` (the same function the CLI's `approval create` wrapper
    /// calls) must produce the same record shape for the same logical
    /// input, on two independent fixture projects (each call writes a real
    /// record with a fresh ULID `id`/`approved_at`, so this compares every
    /// *other* field rather than expecting the two records to be
    /// byte-identical).
    ///
    /// @vtest.id TEST-MCP-APPROVAL-CREATE-MATCHES-CLI
    /// @vtest.covers VO-MCP-TOOL-CLI-EQUIVALENCE
    /// @vtest.target crates/vtest-mcp/src/lib.rs::dispatch_tool
    /// @vtest.intent MCP `approval_create` produces the same record shape as `ops::approval::create`, which the CLI `approval create` wrapper also calls.
    #[test]
    fn mcp_approval_create_tool_matches_the_ops_approval_create_operation() {
        let direct_root = temp_root("approval-create-equivalence-direct");
        fixture_vo_project(&direct_root);
        let direct_layout = vtest_store::VerifyLayout::new(&direct_root);
        let direct = ops::approval::create(
            &direct_layout,
            ops::approval::CreateArgs {
                subject_type: "vo".to_owned(),
                subject_id: "VO-MCP-APPROVAL".to_owned(),
                approved_state: "approved".to_owned(),
                approver_kind: "human".to_owned(),
                approver_id: "reviewer".to_owned(),
                approver_model: None,
                basis: Vec::new(),
                supersedes: Vec::new(),
            },
        )
        .expect("direct ops::approval::create must succeed");

        let mcp_root = temp_root("approval-create-equivalence-mcp");
        fixture_vo_project(&mcp_root);
        let mut mcp_envelope = dispatch_tool(
            &mcp_root,
            "approval_create",
            &json!({
                "subject": {"type": "vo", "id": "VO-MCP-APPROVAL"},
                "state": "approved",
                "approver": {"kind": "human", "id": "reviewer"}
            }),
        );

        // DS-1563: compare the full envelope (data + diagnostics), not a
        // hand-picked subset -- `id`/`approved_at` are the only fields that
        // must differ between two independently-created records (fresh
        // ULID / timestamp per call), so normalise exactly those two.
        let mut direct_envelope = approval_record_envelope(Ok(direct));
        for envelope in [&mut mcp_envelope, &mut direct_envelope] {
            envelope["data"]["id"] = json!("<id>");
            envelope["data"]["approved_at"] = json!("<approved_at>");
        }
        assert_eq!(
            mcp_envelope, direct_envelope,
            "MCP `approval_create` must return the same envelope (data + diagnostics) as the \
             shared `ops::approval::create` the CLI `approval create` wrapper also calls, \
             id/approved_at normalised (fresh per call)"
        );
    }

    /// DS-1563 equivalence for `approval_withdraw`: MCP and `ops::approval::
    /// withdraw` (the same function the CLI's `approval withdraw` wrapper
    /// calls), each targeting its own fixture's real prior `create`.
    ///
    /// @vtest.id TEST-MCP-APPROVAL-WITHDRAW-MATCHES-CLI
    /// @vtest.covers VO-MCP-TOOL-CLI-EQUIVALENCE
    /// @vtest.target crates/vtest-mcp/src/lib.rs::dispatch_tool
    /// @vtest.intent MCP `approval_withdraw` produces the same result as `ops::approval::withdraw`, which the CLI `approval withdraw` wrapper also calls.
    #[test]
    fn mcp_approval_withdraw_tool_matches_the_ops_approval_withdraw_operation() {
        let direct_root = temp_root("approval-withdraw-equivalence-direct");
        fixture_vo_project(&direct_root);
        let direct_layout = vtest_store::VerifyLayout::new(&direct_root);
        let direct_created = ops::approval::create(
            &direct_layout,
            ops::approval::CreateArgs {
                subject_type: "vo".to_owned(),
                subject_id: "VO-MCP-APPROVAL".to_owned(),
                approved_state: "approved".to_owned(),
                approver_kind: "human".to_owned(),
                approver_id: "reviewer".to_owned(),
                approver_model: None,
                basis: Vec::new(),
                supersedes: Vec::new(),
            },
        )
        .expect("direct ops::approval::create must succeed");
        let direct_withdrawn = ops::approval::withdraw(
            &direct_layout,
            ops::approval::WithdrawArgs {
                approval_id: direct_created.id.clone(),
                approver_kind: "human".to_owned(),
                approver_id: "reviewer".to_owned(),
                approver_model: None,
                basis: Vec::new(),
            },
        )
        .expect("direct ops::approval::withdraw must succeed");

        let mcp_root = temp_root("approval-withdraw-equivalence-mcp");
        fixture_vo_project(&mcp_root);
        let mcp_layout = vtest_store::VerifyLayout::new(&mcp_root);
        let mcp_created = ops::approval::create(
            &mcp_layout,
            ops::approval::CreateArgs {
                subject_type: "vo".to_owned(),
                subject_id: "VO-MCP-APPROVAL".to_owned(),
                approved_state: "approved".to_owned(),
                approver_kind: "human".to_owned(),
                approver_id: "reviewer".to_owned(),
                approver_model: None,
                basis: Vec::new(),
                supersedes: Vec::new(),
            },
        )
        .expect("fixture ops::approval::create must succeed");
        let mut mcp_envelope = dispatch_tool(
            &mcp_root,
            "approval_withdraw",
            &json!({
                "approval_id": mcp_created.id,
                "approver": {"kind": "human", "id": "reviewer"}
            }),
        );

        // BD-307: `supersedes` must actually name the id `withdraw` was
        // given, on each side independently -- blindly normalising its
        // *content* the way `id`/`approved_at` are normalised (as the
        // previous version of this test did) would hide a real regression
        // (e.g. an empty or wrong `supersedes`), so this is checked
        // against each side's own known input before the envelope
        // comparison below normalises it for the cross-root diff.
        assert_eq!(
            mcp_envelope["data"]["supersedes"],
            json!([mcp_created.id]),
            "MCP `approval_withdraw` must supersede exactly the id it was given"
        );
        assert_eq!(
            direct_withdrawn.supersedes,
            vec![direct_created.id.clone()],
            "direct `ops::approval::withdraw` must supersede exactly the id it was given"
        );

        // DS-1563: compare the full envelope (data + diagnostics) -- `id`/
        // `approved_at` differ per call (fresh ULID/timestamp), and
        // `supersedes[0]` is each root's own freshly-created id (just
        // verified above against each side's own input), so normalise
        // exactly those three rather than a hand-picked subset.
        let mut direct_envelope = approval_record_envelope(Ok(direct_withdrawn));
        for envelope in [&mut mcp_envelope, &mut direct_envelope] {
            envelope["data"]["id"] = json!("<id>");
            envelope["data"]["approved_at"] = json!("<approved_at>");
            envelope["data"]["supersedes"] = json!(["<superseded-id>"]);
        }
        assert_eq!(
            mcp_envelope, direct_envelope,
            "MCP `approval_withdraw` must return the same envelope (data + diagnostics) as \
             the shared `ops::approval::withdraw` the CLI `approval withdraw` wrapper also \
             calls, id/approved_at/supersedes normalised (fresh per call, content verified \
             above)"
        );
    }

    /// DS-1563 equivalence for `run`: MCP and `ops::run::run` (the same
    /// function the CLI's `run` wrapper calls), each executing its own real
    /// fixture Test via `--fast`. Compares everything but `evidence_ids`
    /// (fresh ULIDs per invocation).
    ///
    /// @vtest.id TEST-MCP-RUN-TESTS-MATCHES-CLI
    /// @vtest.covers VO-MCP-TOOL-CLI-EQUIVALENCE
    /// @vtest.target crates/vtest-mcp/src/lib.rs::dispatch_tool
    /// @vtest.intent MCP `run_tests` produces the same result as `ops::run::run`, which the CLI `run` wrapper also calls.
    #[test]
    fn mcp_run_tests_tool_matches_the_ops_run_operation() {
        fn build_fixture_project(root: &Path) {
            use std::process::Command as ProcessCommand;
            use vtest_model::{
                DerivesFrom, DocumentFile, DocumentId, NodeSource, RootNode, SentenceNode, VoId,
                VoRecord,
            };
            use vtest_store::{init_project, write_document_file, write_vo_record};

            fs::create_dir_all(root.join("src")).expect("mkdir src");
            fs::create_dir_all(root.join("tests")).expect("mkdir tests");
            fs::write(
                root.join("Cargo.toml"),
                "[package]\nname = \"vtest-mcp-run-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
            )
            .expect("write Cargo.toml");
            fs::write(
                root.join("src").join("lib.rs"),
                "pub fn double(x: i32) -> i32 { x * 2 }\n",
            )
            .expect("write src/lib.rs");
            fs::write(
                root.join("tests").join("registered.rs"),
                "/// @vtest.id TEST-MCP-RUN-DOUBLE\n\
                 /// @vtest.covers VO-MCP-RUN-DOUBLE\n\
                 /// @vtest.target src/lib.rs::double\n\
                 /// @vtest.intent doubles the input\n\
                 #[test]\n\
                 fn it_doubles() {\n    assert_eq!(vtest_mcp_run_fixture::double(2), 4);\n}\n",
            )
            .expect("write test file");

            let git = |args: &[&str]| {
                let status = ProcessCommand::new("git")
                    .current_dir(root)
                    .args(
                        [
                            &["-c", "commit.gpgsign=false", "-c", "gpg.format=openpgp"][..],
                            args,
                        ]
                        .concat(),
                    )
                    .status()
                    .unwrap_or_else(|error| panic!("failed to run git {args:?}: {error}"));
                assert!(status.success(), "git {args:?} failed");
            };
            git(&["init", "-q"]);
            git(&["config", "user.email", "vtest-fixture@example.com"]);
            git(&["config", "user.name", "vtest fixture"]);
            git(&["add", "."]);
            git(&["commit", "-q", "-m", "initial fixture commit"]);

            let layout = init_project(root, "vtest-mcp-run-fixture").expect("init .verify/");
            let source = NodeSource {
                doc: "fixture.md".to_owned(),
                heading: "fixture".to_owned(),
                lines: [1, 1],
            };
            let document = DocumentFile {
                schema_version: "0.1".to_owned(),
                root: vec![RootNode {
                    id: DocumentId::new("ROOT-001"),
                    statement: "fixture root".to_owned(),
                    description: None,
                    source: source.clone(),
                }],
                request: vec![SentenceNode {
                    id: DocumentId::new("R-001"),
                    statement: "fixture requirement".to_owned(),
                    description: None,
                    derives_from: vec![DocumentId::new("ROOT-001")],
                    cites: None,
                    source,
                }],
                require: Vec::new(),
                spec: Vec::new(),
                detailed_spec: Vec::new(),
                basic_design: Vec::new(),
                design: Vec::new(),
            };
            write_document_file(&layout, "fixture", &document).expect("write document file");
            write_vo_record(
                &layout,
                &VoRecord {
                    id: VoId::new("VO-MCP-RUN-DOUBLE"),
                    parent: None,
                    derives_from: vec![DerivesFrom {
                        doc: DocumentId::new("R-001"),
                        anchor: None,
                        note: None,
                    }],
                    claim: "fixture claim".to_owned(),
                    dimensions: Vec::new(),
                    coverage_policy: None,
                    combinations: Vec::new(),
                    representative_cases: Vec::new(),
                    created: "2026-09-09T00:00:00Z".to_owned(),
                    updated: "2026-09-09T00:00:00Z".to_owned(),
                },
            )
            .expect("write VO record");
        }

        let direct_root = temp_root("run-equivalence-direct");
        build_fixture_project(&direct_root);
        let direct_layout = vtest_store::VerifyLayout::new(&direct_root);
        let direct_scan = vtest_scan::scan_project(
            &direct_root,
            &vtest_cli::adapters::builtin_registry().expect("builtin registry"),
        )
        .expect("direct fixture scan must succeed");
        let direct = ops::run::run(
            &direct_root,
            &direct_layout,
            &direct_scan,
            &ops::run::RunTarget::All,
            true,
            &vtest_cli::adapters::builtin_registry().expect("builtin registry"),
        )
        .expect("direct ops::run::run must succeed");

        let mcp_root = temp_root("run-equivalence-mcp");
        build_fixture_project(&mcp_root);
        let mut mcp_envelope =
            dispatch_tool(&mcp_root, "run_tests", &json!({"all": true, "fast": true}));

        // DS-1563 requires the same `data`/`diagnostics` for the same
        // input, not merely the same *counts* -- compare the full envelope
        // structurally, normalising only the one field guaranteed to
        // differ per invocation (`evidence_ids` are freshly generated
        // ULIDs, not part of the shared operation's semantic output).
        let mut direct_envelope = json!({
            "ok": !direct.has_errors(),
            "data": {
                "evidence": direct.evidence.len(),
                "evidence_ids": direct.evidence.iter().map(|record| record.id.clone()).collect::<Vec<_>>(),
                "fast": true,
            },
            "diagnostics": direct.diagnostics,
        });
        mcp_envelope["data"]["evidence_ids"] = json!("<ids>");
        direct_envelope["data"]["evidence_ids"] = json!("<ids>");

        assert_eq!(
            mcp_envelope, direct_envelope,
            "MCP `run_tests` must return the same envelope (data + diagnostics) as the shared \
             `ops::run::run` the CLI `run` wrapper also calls, evidence_ids normalised (fresh \
             ULIDs per invocation)"
        );
    }

    const FIXTURE_NODE_TREE: &str = r#"{"schema_version":"0.1","root":[{"id":"ROOT-001","statement":"fixture root","source":{"doc":"fixture.md","heading":"fixture","lines":[1,1]}}],"request":[],"require":[],"spec":[],"detailed_spec":[],"basic_design":[],"design":[]}"#;

    const FIXTURE_NODE_TREE_WITH_REQUEST: &str = r#"{"schema_version":"0.1","root":[{"id":"ROOT-001","statement":"fixture root","source":{"doc":"fixture.md","heading":"fixture","lines":[1,1]}}],"request":[{"id":"R-001","statement":"fixture requirement","derives_from":[],"source":{"doc":"fixture.md","heading":"fixture","lines":[1,1]}}],"require":[],"spec":[],"detailed_spec":[],"basic_design":[],"design":[]}"#;

    /// DS-1563 equivalence for `doc_add`: MCP and `ops::doc::add` (the same
    /// function the CLI's `doc add` wrapper calls), each registering its own
    /// fixture's node-tree file under the same id.
    ///
    /// @vtest.id TEST-MCP-DOC-UPSERT-MATCHES-CLI-ADD
    /// @vtest.covers VO-MCP-TOOL-CLI-EQUIVALENCE
    /// @vtest.target crates/vtest-mcp/src/lib.rs::dispatch_tool
    /// @vtest.intent MCP `doc_upsert` (initial registration) produces the same result as `ops::doc::add`, which the CLI `doc add` wrapper also calls.
    #[test]
    fn mcp_doc_upsert_tool_matches_the_ops_doc_add_operation() {
        let direct_root = temp_root("doc-add-equivalence-direct");
        let init = vtest_cli::run(vtest_cli::Cli {
            project: direct_root.clone(),
            format: vtest_cli::OutputFormat::Json,
            quiet: true,
            command: vtest_cli::Command::Init { name: None },
        });
        assert_eq!(init, ExitCode::Ok, "fixture project must initialise");
        fs::write(direct_root.join("basic-spec.json"), FIXTURE_NODE_TREE)
            .expect("write source file");
        let direct_layout = vtest_store::VerifyLayout::new(&direct_root);
        let direct = ops::doc::add(
            &direct_root,
            &direct_layout,
            ops::doc::AddArgs {
                id: "DOC-BASIC-001".to_owned(),
                path: "basic-spec.json".to_owned(),
                derives_from: None,
                root: false,
                root_specified: false,
                update: false,
            },
        )
        .expect("direct ops::doc::add must succeed");

        let mcp_root = temp_root("doc-add-equivalence-mcp");
        let init = vtest_cli::run(vtest_cli::Cli {
            project: mcp_root.clone(),
            format: vtest_cli::OutputFormat::Json,
            quiet: true,
            command: vtest_cli::Command::Init { name: None },
        });
        assert_eq!(init, ExitCode::Ok, "fixture project must initialise");
        fs::write(mcp_root.join("basic-spec.json"), FIXTURE_NODE_TREE).expect("write source file");
        let mut mcp_envelope = dispatch_tool(
            &mcp_root,
            "doc_upsert",
            &json!({"id": "DOC-BASIC-001", "path": "basic-spec.json"}),
        );

        // DS-1563: byte-identical input on both roots, so the full
        // envelope must match exactly, normalising only `path` (each
        // root's own absolute temp-directory path).
        let mut direct_envelope = success_envelope(true, doc_view_json(&direct), &[]);
        for envelope in [&mut mcp_envelope, &mut direct_envelope] {
            envelope["data"]["path"] = json!("<path>");
        }
        assert_eq!(
            mcp_envelope, direct_envelope,
            "MCP `doc_upsert` must return the same envelope (data + diagnostics) as the shared \
             `ops::doc::add` the CLI `doc add` wrapper also calls, for byte-identical input \
             (path normalised: each root has its own absolute temp-directory path)"
        );
    }

    /// DS-1683/BD-331: MCP `doc_upsert` given `root` (any value, including
    /// `false`) alongside `update: true` must be rejected -- root
    /// designation is fixed at initial registration only. Previously
    /// untested on the MCP side (only the CLI's own `--root`/`--no-root`
    /// + `--update` combination had coverage).
    ///
    /// @vtest.id TEST-MCP-DOC-UPSERT-REJECTS-ROOT-WITH-UPDATE
    /// @vtest.covers VO-DOC-ROOT-FIXED-AFTER-REGISTRATION, VO-DOC-ROOT-DESIGNATION-AT-ADD
    /// @vtest.target crates/vtest-mcp/src/lib.rs::dispatch_tool
    /// @vtest.intent MCP `doc_upsert` rejects `root` combined with `update: true` (E-OP-001), since root designation is fixed at initial registration.
    #[test]
    fn mcp_doc_upsert_tool_rejects_root_combined_with_update() {
        let root = temp_root("doc-upsert-root-update-reject");
        let init = vtest_cli::run(vtest_cli::Cli {
            project: root.clone(),
            format: vtest_cli::OutputFormat::Json,
            quiet: true,
            command: vtest_cli::Command::Init { name: None },
        });
        assert_eq!(init, ExitCode::Ok, "fixture project must initialise");
        fs::write(root.join("basic-spec.json"), FIXTURE_NODE_TREE).expect("write source file");
        let layout = vtest_store::VerifyLayout::new(&root);
        ops::doc::add(
            &root,
            &layout,
            ops::doc::AddArgs {
                id: "DOC-BASIC-001".to_owned(),
                path: "basic-spec.json".to_owned(),
                derives_from: None,
                root: false,
                root_specified: false,
                update: false,
            },
        )
        .expect("initial add must succeed");

        let envelope_true = dispatch_tool(
            &root,
            "doc_upsert",
            &json!({"id": "DOC-BASIC-001", "path": "basic-spec.json", "root": true, "update": true}),
        );
        assert_eq!(
            envelope_true["ok"],
            Value::Bool(false),
            "root: true combined with update: true must be rejected, got {envelope_true:?}"
        );
        assert_eq!(
            envelope_true["diagnostics"][0]["code"],
            Value::String("E-OP-001".to_owned()),
            "the rejection must carry E-OP-001 (this is a Usage error from ops::doc::add, not \
             a store-layer failure), got {envelope_true:?}"
        );

        let envelope_false = dispatch_tool(
            &root,
            "doc_upsert",
            &json!({"id": "DOC-BASIC-001", "path": "basic-spec.json", "root": false, "update": true}),
        );
        assert_eq!(
            envelope_false["ok"],
            Value::Bool(false),
            "root: false (still present) combined with update: true must be rejected too, got \
             {envelope_false:?}"
        );
        assert_eq!(
            envelope_false["diagnostics"][0]["code"],
            Value::String("E-OP-001".to_owned()),
            "got {envelope_false:?}"
        );
    }

    /// DS-1003/1681 equivalence: MCP `doc_add`'s `derives_from` argument
    /// writes onto the registered document's top-level node the same way
    /// the CLI's `--derives-from` flag (via `ops::doc::add`) does.
    ///
    /// @vtest.id TEST-MCP-DOC-UPSERT-DERIVES-FROM-TOP-LEVEL
    /// @vtest.covers VO-DOC-ADD-DERIVES-FROM-TOP-LEVEL-EDGE
    /// @vtest.target crates/vtest-mcp/src/lib.rs::dispatch_tool
    /// @vtest.intent MCP `doc_upsert`'s `derives_from` argument writes the edge onto the registered document's top-level node, matching `ops::doc::add`'s `--derives-from` behavior.
    #[test]
    fn mcp_doc_upsert_tool_applies_derives_from_like_ops_doc_add() {
        let direct_root = temp_root("doc-add-derives-from-direct");
        let init = vtest_cli::run(vtest_cli::Cli {
            project: direct_root.clone(),
            format: vtest_cli::OutputFormat::Json,
            quiet: true,
            command: vtest_cli::Command::Init { name: None },
        });
        assert_eq!(init, ExitCode::Ok, "fixture project must initialise");
        fs::write(
            direct_root.join("basic-spec.json"),
            FIXTURE_NODE_TREE_WITH_REQUEST,
        )
        .expect("write source file");
        let direct_layout = vtest_store::VerifyLayout::new(&direct_root);
        let direct = ops::doc::add(
            &direct_root,
            &direct_layout,
            ops::doc::AddArgs {
                id: "DOC-BASIC-001".to_owned(),
                path: "basic-spec.json".to_owned(),
                derives_from: Some(vec!["ROOT-001".to_owned()]),
                root: false,
                root_specified: false,
                update: false,
            },
        )
        .expect("direct ops::doc::add must succeed");

        let mcp_root = temp_root("doc-add-derives-from-mcp");
        let init = vtest_cli::run(vtest_cli::Cli {
            project: mcp_root.clone(),
            format: vtest_cli::OutputFormat::Json,
            quiet: true,
            command: vtest_cli::Command::Init { name: None },
        });
        assert_eq!(init, ExitCode::Ok, "fixture project must initialise");
        fs::write(
            mcp_root.join("basic-spec.json"),
            FIXTURE_NODE_TREE_WITH_REQUEST,
        )
        .expect("write source file");
        let mut mcp_envelope = dispatch_tool(
            &mcp_root,
            "doc_upsert",
            &json!({
                "id": "DOC-BASIC-001",
                "path": "basic-spec.json",
                "derives_from": ["ROOT-001"]
            }),
        );

        assert_eq!(
            direct.derives_from,
            vec!["ROOT-001".to_owned()],
            "direct ops::doc::add must have written derives_from onto the request-layer node"
        );
        let mut direct_envelope = success_envelope(true, doc_view_json(&direct), &[]);
        for envelope in [&mut mcp_envelope, &mut direct_envelope] {
            envelope["data"]["path"] = json!("<path>");
        }
        assert_eq!(
            mcp_envelope, direct_envelope,
            "MCP `doc_upsert` with `derives_from` must return the same envelope (data + \
             diagnostics) as the shared `ops::doc::add` the CLI `doc add --derives-from` \
             wrapper also calls, for byte-identical input (path normalised)"
        );
    }

    /// DS-1563 equivalence for `doc_list`: MCP and `ops::doc::list` (the
    /// same function the CLI's `doc list` wrapper calls), on the same
    /// on-disk registry records.
    ///
    /// @vtest.id TEST-MCP-DOC-LIST-MATCHES-CLI
    /// @vtest.covers VO-MCP-TOOL-CLI-EQUIVALENCE
    /// @vtest.target crates/vtest-mcp/src/lib.rs::dispatch_tool
    /// @vtest.intent MCP `doc_list` returns the same envelope as `ops::doc::list`, which the CLI `doc list` wrapper also calls, across the default/tree/roots (DS-1194) shapes.
    #[test]
    fn mcp_doc_list_tool_matches_the_ops_doc_list_operation() {
        let root = temp_root("doc-list-equivalence");
        let init = vtest_cli::run(vtest_cli::Cli {
            project: root.clone(),
            format: vtest_cli::OutputFormat::Json,
            quiet: true,
            command: vtest_cli::Command::Init { name: None },
        });
        assert_eq!(init, ExitCode::Ok, "fixture project must initialise");
        fs::write(root.join("basic-spec.json"), FIXTURE_NODE_TREE).expect("write source file");
        let layout = vtest_store::VerifyLayout::new(&root);
        ops::doc::add(
            &root,
            &layout,
            ops::doc::AddArgs {
                id: "DOC-BASIC-001".to_owned(),
                path: "basic-spec.json".to_owned(),
                derives_from: None,
                root: false,
                root_specified: false,
                update: false,
            },
        )
        .expect("add must succeed against a real node-tree file");

        let direct = ops::doc::list(&layout).expect("direct ops::doc::list must succeed");
        let direct_records: Vec<_> = direct.records.iter().map(doc_view_json).collect();

        // DS-1194: `tree`/`roots` shape which fields appear, matching the
        // CLI's own `render_doc_list_text` view-switching. Neither given
        // -> flat `records` (+ always-present `unresolved_derives_from`),
        // no `roots`/`document_chain`.
        let default_envelope = success_envelope(
            true,
            json!({
                "records": direct_records,
                "unresolved_derives_from": direct.unresolved.clone(),
                "freshness": direct.freshness.clone(),
            }),
            &[],
        );
        let mcp_default = dispatch_tool(&root, "doc_list", &json!({}));
        assert_eq!(
            mcp_default, default_envelope,
            "MCP `doc_list` (no tree/roots) must return the same envelope (data + diagnostics) \
             as the shared `ops::doc::list` the CLI `doc list` wrapper also calls, matching the \
             CLI's own default flat view"
        );

        // `tree: true` -> `records` + `document_chain`, no `roots`.
        let tree_envelope = success_envelope(
            true,
            json!({
                "records": direct_records,
                "unresolved_derives_from": direct.unresolved.clone(),
                "freshness": direct.freshness.clone(),
                "document_chain": direct.document_chain,
            }),
            &[],
        );
        let mcp_tree = dispatch_tool(&root, "doc_list", &json!({"tree": true}));
        assert_eq!(
            mcp_tree, tree_envelope,
            "MCP `doc_list` with tree:true must include document_chain, matching the CLI's own \
             --tree view"
        );

        // `roots: true` -> only `roots` (mirrors the CLI's own early
        // return for --roots, which does not also render the record
        // list).
        let roots_envelope = success_envelope(
            true,
            json!({
                "unresolved_derives_from": direct.unresolved,
                "roots": direct
                    .records
                    .iter()
                    .filter(|view| view.is_root)
                    .map(|view| view.id.clone())
                    .collect::<Vec<_>>(),
            }),
            &[],
        );
        let mcp_roots = dispatch_tool(&root, "doc_list", &json!({"roots": true}));
        assert_eq!(
            mcp_roots, roots_envelope,
            "MCP `doc_list` with roots:true must return only roots (+ unresolved_derives_from), \
             matching the CLI's own --roots early-return view, not also the record list"
        );
    }
}
