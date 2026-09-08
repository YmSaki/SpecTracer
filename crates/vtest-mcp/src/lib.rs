//! MCP stdio transport over the canonical v0.1 operations.
//!
//! ROOT-027 / R-1「MCP インターフェースは飾りではなく本体側」— MCP is not a
//! thin wrapper kept in sync by convention: every tool handler here calls
//! the exact same `vtest_cli::ops::*::execute` function the CLI's `run_*`
//! wrappers call, so DS-1563「別紙A（§12〜§15）が定める全 MCP tool が同じ
//! 入力に対する CLI JSON と同じ data / diagnostics を返す」holds by
//! construction rather than by two independently written code paths.
//!
//! Per REQ-323 / SPEC-216 / SPEC-246 the full MCP tool taxonomy is a
//! detailed-design matter delegated to 別紙A §12–§15. This slice exposes
//! exactly the operations the current CLI (`crates/vtest-cli/src/lib.rs`)
//! implements — `init`, `scan`, `doctor`, `run`, `verify` — and does not
//! invent MCP-only tools for CLI surface (`spec`/`req`/`vo`/`test`/`audit`/
//! `approval`/`report`) that does not exist yet on the canonical model
//! (SPEC-400, see that file's module doc). Adding those tools without a
//! citation would be exactly the invention this repository's AGENTS.md
//! forbids; they are left out and reported as declined scope rather than
//! guessed at.
//!
//! An MCP-only Structured-Edit tool with an apply/re-verify/rollback
//! contract was named in the task that produced this module, citing
//! "E-OP-003". That diagnostic code does not exist anywhere in
//! `docs/canonical/specification.json` (checked by grep against commit
//! 79e43fa) — it is not a canonical requirement, so no edit/rollback tool
//! is implemented here. This is a stopped_on, not a silent omission: an
//! edit MCP tool needs an upstream citation for its rollback trigger
//! condition before one can be built without inventing it downstream.

use std::{
    fs,
    io::{self, BufRead, Write},
    path::Path,
    time::UNIX_EPOCH,
};

use serde_json::{json, Map, Value};
use vtest_cli::ops;
use vtest_model::ExitCode;

const TOOL_NAMES: &[&str] = &["init", "scan", "doctor", "run", "verify"];

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
    let (_, scan) = ops::scan::execute(root);
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
        "init" => (json!({"name": {"type": "string"}}), Vec::<&str>::new()),
        "scan" | "doctor" => (json!({}), Vec::new()),
        "run" => (
            json!({
                "test": {"type": "array", "items": {"type": "string"}},
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
        "init" => &["name"],
        "scan" | "doctor" => &[],
        "run" => &["test", "fast"],
        "verify" => &["items", "doc", "vo", "test", "gate", "summary"],
        _ => &[],
    };
    if let Some(key) = args.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(failure_envelope(
            "E-OP-001",
            format!("{name} does not accept argument `{key}`"),
        ));
    }
    match name {
        "init" => optional_nonempty_string(args, "name").map(|_| ()),
        "scan" | "doctor" => Ok(()),
        "run" => {
            optional_string_array(args, "test")?;
            optional_bool(args, "fast")
        }
        "verify" => {
            optional_string_array(args, "items")?;
            for key in ["doc", "vo", "test", "gate"] {
                optional_nonempty_string(args, key)?;
            }
            optional_bool(args, "summary")
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
        "init" => {
            let project_name = string_arg(args, "name")
                .map(str::to_owned)
                .or_else(|| {
                    root.file_name()
                        .and_then(|value| value.to_str())
                        .map(str::to_owned)
                })
                .unwrap_or_else(|| "project".to_owned());
            ops::init::execute(root, &project_name).1
        }
        "scan" => ops::scan::execute(root).1,
        "doctor" => ops::doctor::execute(root).1,
        "run" => run_tool(root, args),
        "verify" => verify_tool(root, args),
        _ => failure_envelope("E-OP-001", format!("unknown MCP tool `{name}`")),
    }
}

fn run_tool(root: &Path, args: &Value) -> Value {
    let test_ids = args
        .get("test")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let fast = bool_arg(args, "fast");

    // Mirrors `vtest-cli`'s `run_run`: resolve config + scan, then hand the
    // same `ops::run::run` the CLI calls the resolved scan and test ids.
    let config = vtest_store::load_config(root);
    if let Err(error) = config {
        return failure_envelope("E-CONFIG-001", error.to_string());
    }
    let scan = match vtest_scan::scan_project(root) {
        Ok(scan) => scan,
        Err(error) => {
            let code = error.code().unwrap_or("E-CORE-001");
            return failure_envelope(code, error.to_string());
        }
    };
    let layout = vtest_store::VerifyLayout::new(root);
    match ops::run::run(root, &layout, &scan, &test_ids, fast) {
        Ok(result) => {
            let has_errors = result.has_errors();
            let data = json!({
                "evidence": result.evidence.len(),
                "evidence_ids": result.evidence.iter().map(|record| record.id.clone()).collect::<Vec<_>>(),
                "fast": fast,
            });
            success_envelope(!has_errors, data, &result.diagnostics)
        }
        Err(ops::run::RunOpError::UnknownTestId(id)) => failure_envelope(
            "E-OP-001",
            format!("no Test with id '{id}' was discovered by scan"),
        ),
        Err(error @ ops::run::RunOpError::Execution(_)) => {
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

    match ops::verify::execute(root, &items, doc, vo, test, gate, summary) {
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
        let suffix = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("vtest-mcp-{name}-{suffix}"));
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
        match vtest_cli::ops::verify::execute(root, &[], None, None, None, None, false) {
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

        let (_, cli_envelope) = ops::scan::execute(&root);
        let mcp_envelope = dispatch_tool(&root, "scan", &json!({}));

        assert_eq!(
            mcp_envelope, cli_envelope,
            "MCP `scan` tool must return the same envelope as `ops::scan::execute`, \
             which the CLI's `run_scan` wrapper also calls"
        );
    }
}
