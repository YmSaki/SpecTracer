//! End-to-end regressions for Issue #234 (`vtest run` side) and Issue #266:
//! when `vtest run` rejects Evidence with `E-EXEC-004`, the diagnostic names
//! what changed between the pre-run and post-run Execution State snapshots
//! (such as `Cargo.lock` or files written by a test), matches between CLI and
//! MCP, and never leaks absolute paths or file contents.
#![expect(
    clippy::unwrap_used,
    reason = "a panic in a test helper is the assertion; `allow-unwrap-in-tests` covers only `#[test]` functions"
)]

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

fn temp_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "vtest-exec-state-diff-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(root)
        .args(
            [
                &["-c", "commit.gpgsign=false", "-c", "gpg.format=openpgp"][..],
                args,
            ]
            .concat(),
        )
        .status()
        .unwrap_or_else(|error| panic!("git {args:?}: {error}"));
    assert!(status.success(), "git {args:?} failed");
}

fn vtest(root: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_vtest"));
    for variable in [
        "RUSTC_WRAPPER",
        "CARGO_LLVM_COV",
        "CARGO_LLVM_COV_TARGET_DIR",
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_INCREMENTAL",
        "CARGO_TARGET_DIR",
        "CARGO_BUILD_TARGET_DIR",
    ] {
        command.env_remove(variable);
    }
    command.arg("--project").arg(root);
    command
}

fn cli_json(root: &Path, args: &[&str]) -> (i32, Value) {
    let output = vtest(root)
        .args(["--format", "json"])
        .args(args)
        .output()
        .unwrap();
    let envelope = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "non-JSON output ({error}): {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (output.status.code().expect("exit code"), envelope)
}

fn mcp_call(root: &Path, name: &str, arguments: Value) -> Value {
    let mut child = vtest(root)
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

fn init_rust_fixture(
    root: &Path,
    lib_rs: &str,
    commit_lockfile: bool,
    extra_files: &[(&str, &str)],
) {
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"exec_state_diff_fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), lib_rs).unwrap();
    fs::write(root.join(".gitignore"), "target/\n").unwrap();
    for (rel, content) in extra_files {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
    if commit_lockfile {
        let status = Command::new("cargo")
            .current_dir(root)
            .args(["generate-lockfile", "--offline"])
            .status()
            .unwrap();
        assert!(status.success(), "cargo generate-lockfile failed");
    }

    let layout = vtest_store::init_project(root, "exec-state-diff-fixture").unwrap();
    let source = |id: &str| NodeSource {
        doc: format!("{id}.md"),
        heading: "fixture".to_owned(),
        lines: [1, 1],
    };
    let doc = DocumentFile {
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
    vtest_store::write_document_file(&layout, "fixture", &doc).unwrap();
    vtest_store::write_vo_record(
        &layout,
        &VoRecord {
            id: VoId::new("VO-X"),
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
            created: "2026-11-18T00:00:00Z".to_owned(),
            updated: "2026-11-18T00:00:00Z".to_owned(),
        },
    )
    .unwrap();

    git(root, &["init", "-q"]);
    git(root, &["config", "user.name", "fixture"]);
    git(root, &["config", "user.email", "fixture@example.com"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "initial"]);
}

fn evidence_count(root: &Path) -> usize {
    fs::read_dir(root.join(".verify/evidence")).map_or(0, |entries| {
        entries
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "yaml"))
            .count()
    })
}

/// #266: on an initial `vtest run --all` in a Rust project without `Cargo.lock`,
/// Cargo creates `Cargo.lock` during execution, triggering `E-EXEC-004`. Both
/// CLI and MCP must report `Cargo.lock` in the diagnostic without writing
/// Evidence or exposing absolute paths.
/// @vtest.id TEST-E2E-E-EXEC-004-REPORTS-CARGO-LOCK-CLI-MCP
/// @vtest.covers VO-EXEC-STATE-PREPOST-STABILITY, VO-MCP-TOOL-CLI-EQUIVALENCE
/// @vtest.target crates/vtest-ops/src/run.rs::run
/// @vtest.intent 初期Cargo.lockがないRustプロジェクトで初回vtest run --allを実行したとき、CLIとMCPの両方でE-EXEC-004の診断にCargo.lockが表示され、絶対パスが出ず、Evidenceが記録されないことを確認する
#[test]
fn initial_run_without_cargo_lock_reports_cargo_lock_in_cli_and_mcp() {
    let lib_rs = "pub fn target() -> i32 { 1 }\n\n\
                  #[cfg(test)]\nmod tests {\n    \
                  /// @vtest.id TEST-X\n    /// @vtest.covers VO-X\n    \
                  /// @vtest.target src/lib.rs::target\n    /// @vtest.intent returns one\n    \
                  #[test]\n    fn returns_one() {\n        assert_eq!(super::target(), 1);\n    }\n}\n";

    let cli_root = temp_root("no-lock-cli");
    init_rust_fixture(&cli_root, lib_rs, false, &[]);
    let (exit, cli_env) = cli_json(&cli_root, &["run", "--all", "--fast"]);
    assert_eq!(exit, 1, "{cli_env}");
    assert_eq!(cli_env["ok"], false, "{cli_env}");
    assert_eq!(cli_env["data"]["evidence"], 0, "{cli_env}");
    assert_eq!(evidence_count(&cli_root), 0);

    let mcp_root = temp_root("no-lock-mcp");
    init_rust_fixture(&mcp_root, lib_rs, false, &[]);
    let mcp_env = mcp_call(&mcp_root, "run_tests", json!({"all": true, "fast": true}));
    assert_eq!(evidence_count(&mcp_root), 0);

    assert_eq!(
        mcp_env, cli_env,
        "CLI and MCP must emit identical envelopes"
    );
    let diag = &cli_env["diagnostics"][0];
    assert_eq!(diag["code"], "E-EXEC-004", "{cli_env}");
    let message = diag["message"].as_str().unwrap();
    assert!(
        message.contains("added: workspace-root:Cargo.lock"),
        "expected Cargo.lock in E-EXEC-004 message: {message}"
    );
    assert!(
        !message.contains(cli_root.to_string_lossy().as_ref()),
        "E-EXEC-004 message must not leak absolute root path: {message}"
    );

    fs::remove_dir_all(cli_root).ok();
    fs::remove_dir_all(mcp_root).ok();
}

/// #234: when a test writes or modifies files inside the project directory
/// during execution, `E-EXEC-004` reports the relative paths of those files
/// without leaking absolute paths or file contents.
/// @vtest.id TEST-E2E-E-EXEC-004-REPORTS-TEST-CREATED-FILES
/// @vtest.covers VO-EXEC-STATE-PREPOST-STABILITY
/// @vtest.target crates/vtest-exec/src/lib.rs::run_tests_with_runner
/// @vtest.intent テストがプロジェクト内にファイルを新規作成・変更した場合に、E-EXEC-004が相対パスを示し絶対パスやファイル内容を出さないことを確認する
#[test]
fn test_writing_and_modifying_project_files_reports_relative_paths_without_leaks() {
    let lib_rs = "pub fn target() -> i32 { 1 }\n\n\
                  #[cfg(test)]\nmod tests {\n    \
                  /// @vtest.id TEST-WRITES-FILES\n    /// @vtest.covers VO-X\n    \
                  /// @vtest.target src/lib.rs::target\n    /// @vtest.intent writes project files\n    \
                  #[test]\n    fn writes_project_files() {\n        \
                      let root = std::path::Path::new(env!(\"CARGO_MANIFEST_DIR\"));\n        \
                      std::fs::create_dir_all(root.join(\"generated\")).unwrap();\n        \
                      std::fs::write(root.join(\"generated/created.txt\"), \"SECRET_CREATED_PAYLOAD\").unwrap();\n        \
                      std::fs::write(root.join(\"data/existing.txt\"), \"SECRET_MODIFIED_PAYLOAD\").unwrap();\n        \
                      assert_eq!(super::target(), 1);\n    \
                  }\n}\n";

    let root = temp_root("test-writes");
    init_rust_fixture(
        &root,
        lib_rs,
        true,
        &[("data/existing.txt", "ORIGINAL_PAYLOAD")],
    );
    let (exit, envelope) = cli_json(&root, &["run", "--all", "--fast"]);
    assert_eq!(exit, 1, "{envelope}");
    assert_eq!(envelope["ok"], false, "{envelope}");
    assert_eq!(envelope["data"]["evidence"], 0, "{envelope}");
    assert_eq!(evidence_count(&root), 0);

    let diag = &envelope["diagnostics"][0];
    assert_eq!(diag["code"], "E-EXEC-004", "{envelope}");
    let message = diag["message"].as_str().unwrap();
    assert!(
        message.contains("added: workspace-root:generated/created.txt"),
        "expected added path in message: {message}"
    );
    assert!(
        message.contains("modified: workspace-root:data/existing.txt"),
        "expected modified path in message: {message}"
    );
    for forbidden in [
        root.to_string_lossy().as_ref(),
        "SECRET_CREATED_PAYLOAD",
        "SECRET_MODIFIED_PAYLOAD",
        "ORIGINAL_PAYLOAD",
    ] {
        assert!(
            !message.contains(forbidden),
            "E-EXEC-004 leaked forbidden value {forbidden:?}: {message}"
        );
    }

    fs::remove_dir_all(root).ok();
}
