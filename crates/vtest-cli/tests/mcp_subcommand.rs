//! SPEC-403: `vtest mcp` starts the MCP server over stdio (#83).

use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};

/// `vtest mcp` answers initialize -> tools/list -> tools/call(scan) over
/// stdio as newline-delimited JSON-RPC, and exits once stdin closes.
/// @vtest.id TEST-CLI-MCP-SUBCOMMAND-ROUND-TRIP
/// @vtest.covers VO-CLI-MCP-SUBCOMMAND-STDIO
/// @vtest.target crates/vtest-cli/src/main.rs::main
/// @vtest.intent vtest mcp serves initialize, tools/list and tools/call scan over stdio
#[test]
fn vtest_mcp_serves_initialize_tools_list_and_scan_over_stdio() {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "vtest-mcp-subcommand-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&root).expect("create fixture root");
    vtest_store::init_project(&root, "mcp-subcommand").expect("init project");

    let mut child = Command::new(env!("CARGO_BIN_EXE_vtest"))
        .arg("--project")
        .arg(&root)
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn vtest mcp");
    {
        let mut stdin = child.stdin.take().expect("stdin");
        for request in [
            serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
            serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {"name": "scan", "arguments": {}}
            }),
        ] {
            writeln!(stdin, "{request}").expect("write request");
        }
    }
    let output = child.wait_with_output().expect("vtest mcp exits");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let responses: Vec<serde_json::Value> = BufReader::new(output.stdout.as_slice())
        .lines()
        .map(|line| serde_json::from_str(&line.expect("line")).expect("JSON response"))
        .collect();
    assert_eq!(responses.len(), 3, "{responses:?}");
    assert_eq!(responses[0]["id"], 1);
    assert_eq!(responses[0]["result"]["serverInfo"]["name"], "vtest");
    let tools = responses[1]["result"]["tools"]
        .as_array()
        .expect("tools list");
    assert!(tools.iter().any(|tool| tool["name"] == "scan"), "{tools:?}");
    assert!(tools.iter().any(|tool| tool["name"] == "verify"));
    assert_eq!(responses[2]["id"], 3);
    assert_ne!(responses[2]["result"]["isError"], true, "{}", responses[2]);
    assert!(responses[2]["result"]["structuredContent"]["data"].is_object());
}

/// The library entry point cannot serve MCP (vtest-mcp depends on it), so
/// `vtest_cli::run` rejects `Command::Mcp` as a usage error instead of
/// silently doing nothing.
/// @vtest.id TEST-CLI-MCP-LIBRARY-RUN-REJECTS
/// @vtest.covers VO-CLI-MCP-SUBCOMMAND-STDIO
/// @vtest.target crates/vtest-cli/src/lib.rs::run
/// @vtest.intent vtest_cli::run reports a usage error for Command::Mcp instead of a silent no-op
#[test]
fn the_library_run_rejects_the_mcp_command() {
    let exit = vtest_cli::run(vtest_cli::Cli {
        project: std::env::temp_dir(),
        format: vtest_cli::OutputFormat::Json,
        quiet: true,
        command: vtest_cli::Command::Mcp,
    });
    assert_eq!(exit, vtest_model::ExitCode::Usage);
}
