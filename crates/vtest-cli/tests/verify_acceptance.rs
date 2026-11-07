//! Acceptance coverage for the canonical `vtest verify` CLI surface.
//!
//! これは正典 v0.1 の CLI 表面（SPEC-398 / DS-1104 / DS-927〜DS-934）に対する
//! 受入テストである。前身の `m1_acceptance.rs` … `m9_acceptance.rs` は旧
//! 12 項目 CLI を対象としており、正典モデル上でコンパイルできないため
//! `Cargo.toml` の `autotests = false` によりビルド対象から外してある。

use std::path::{Path, PathBuf};

use vtest_cli::{run, Cli, Command, OutputFormat};
use vtest_model::ExitCode;

fn temp_root(name: &str) -> PathBuf {
    // A nanosecond-timestamp suffix alone collides under parallel test
    // execution on Windows' coarser clock resolution -- see
    // `vtest-scan`'s `fixture()` doc comment for the confirmed root cause
    // of a previously-unconfirmed flaky failure elsewhere in this
    // workspace.
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "vtest-cli-{name}-{}-{sequence}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("create fixture root");
    root
}

fn cli(root: &Path, command: Command) -> Cli {
    Cli {
        project: root.to_path_buf(),
        format: OutputFormat::Json,
        quiet: true,
        command,
    }
}

fn verify_command() -> Command {
    Command::Verify {
        items: Vec::new(),
        doc: None,
        vo: None,
        test: None,
        gate: None,
        summary: false,
    }
}

/// Running `verify` outside any project is an operation rejection, not a
/// verification result. DS-929「終了コード`2`は、操作拒否…（検証結果は生成
/// しない）」。とりわけ「プロジェクトが無いので PASS」には決してならない。
/// @vtest.id TEST-VERIFY-OUTSIDE-PROJECT-REJECTION
/// @vtest.covers VO-VERIFY-OUTSIDE-PROJECT-OPERATION-REJECTION
/// @vtest.target crates/vtest-cli/src/lib.rs::run
/// @vtest.intent verify outside any project is exit 2, never exit 0
#[test]
fn verify_outside_a_project_is_an_operation_rejection_not_a_pass() {
    let root = temp_root("no-project");
    assert_eq!(
        run(cli(&root, verify_command())),
        ExitCode::Usage,
        "a missing .verify/ must be exit 2, never exit 0"
    );
}

/// DS-1116「config の `gates` に同名の定義が無ければ E-CONFIG-002・終了コード
/// 2 で拒否し、検証を実行しない」。
/// @vtest.id TEST-VERIFY-UNDEFINED-GATE-REJECTED
/// @vtest.covers VO-VERIFY-UNDEFINED-GATE-NAME-REJECTED
/// @vtest.target crates/vtest-cli/src/lib.rs::run
/// @vtest.intent an undefined --gate name is rejected with exit 2 before verification runs
#[test]
fn an_undefined_gate_name_is_rejected_before_verification_runs() {
    let root = temp_root("unknown-gate");
    assert_eq!(run(cli(&root, Command::Init { name: None })), ExitCode::Ok);
    let command = Command::Verify {
        items: Vec::new(),
        doc: None,
        vo: None,
        test: None,
        gate: Some("no-such-gate".to_owned()),
        summary: false,
    };
    assert_eq!(run(cli(&root, command)), ExitCode::Usage);
}

/// DS-1154: the E-CONFIG-002 message names the requested gate and every
/// defined gate; matching stays exact and case-sensitive (`Release` does not
/// select `release`).
/// @vtest.id TEST-VERIFY-UNDEFINED-GATE-LISTS-DEFINED
/// @vtest.covers VO-VERIFY-UNDEFINED-GATE-CANDIDATES
/// @vtest.target crates/vtest-cli/src/ops/verify.rs::execute
/// @vtest.intent an unknown gate name yields E-CONFIG-002 whose message lists the requested and all defined gate names, with no partial result
#[test]
fn an_undefined_gate_error_lists_the_defined_gate_names() {
    let root = temp_root("unknown-gate-candidates");
    let layout = vtest_store::init_project(&root, "gate-candidates").expect("init");
    let mut config = vtest_store::load_config(&root).expect("load config");
    for name in ["release", "beta"] {
        config.gates.push(vtest_store::GateConfig {
            name: name.to_owned(),
            require: vtest_store::GateRequirement {
                verification: "PASS".to_owned(),
                approvals: Vec::new(),
            },
        });
    }
    std::fs::write(layout.config(), config.to_yaml()).expect("write config");

    let command = Command::Verify {
        items: Vec::new(),
        doc: None,
        vo: None,
        test: None,
        gate: Some("Release".to_owned()),
        summary: false,
    };
    assert_eq!(run(cli(&root, command)), ExitCode::Usage);

    let registry = vtest_cli::adapters::builtin_registry().expect("registry");
    let Err(vtest_cli::ops::verify::VerifyOpError::Usage {
        code,
        message,
        candidates,
    }) = vtest_cli::ops::verify::execute(
        &root,
        &[],
        None,
        None,
        None,
        Some("Release"),
        false,
        &registry,
    )
    else {
        panic!("an unknown gate must be a usage error with no verification result");
    };
    assert_eq!(code, "E-CONFIG-002");
    assert!(message.contains("'Release'"), "{message}");
    assert!(
        message.contains("release") && message.contains("beta"),
        "{message}"
    );
    assert_eq!(
        candidates,
        Some(vec!["release".to_owned(), "beta".to_owned()])
    );
}

/// Writes `gates` (in this order) into a fresh project's config.yaml.
fn project_with_gates(name: &str, gates: &[&str]) -> PathBuf {
    let root = temp_root(name);
    let layout = vtest_store::init_project(&root, name).expect("init");
    let mut config = vtest_store::load_config(&root).expect("load config");
    for gate in gates {
        config.gates.push(vtest_store::GateConfig {
            name: (*gate).to_owned(),
            require: vtest_store::GateRequirement {
                verification: "PASS".to_owned(),
                approvals: Vec::new(),
            },
        });
    }
    std::fs::write(layout.config(), config.to_yaml()).expect("write config");
    root
}

/// The real `vtest verify --gate <name> --format json` process's stdout.
fn cli_verify_gate_json(root: &Path, gate: &str) -> (Option<i32>, serde_json::Value) {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_vtest"))
        .arg("--project")
        .arg(root)
        .args(["verify", "--gate", gate, "--format", "json"])
        .output()
        .expect("run vtest verify");
    let value = serde_json::from_slice(&output.stdout).expect("CLI JSON");
    (output.status.code(), value)
}

/// The real `vtest mcp` process's `tools/call verify {"gate": <name>}` result.
fn mcp_verify_gate(root: &Path, gate: &str) -> serde_json::Value {
    use std::io::{BufRead, Write};
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_vtest"))
        .arg("--project")
        .arg(root)
        .arg("mcp")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn vtest mcp");
    {
        let mut stdin = child.stdin.take().expect("stdin");
        for request in [
            serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
            serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {"name": "verify", "arguments": {"gate": gate}}
            }),
        ] {
            writeln!(stdin, "{request}").expect("write request");
        }
    }
    let output = child.wait_with_output().expect("vtest mcp exits");
    let response = std::io::BufReader::new(output.stdout.as_slice())
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(&line.expect("line")).unwrap())
        .find(|response| response["id"] == 2)
        .expect("tools/call response");
    assert_eq!(response["result"]["isError"], true, "{response}");
    response["result"]["structuredContent"].clone()
}

/// DS-1154/DS-1563: for an unknown gate the real CLI `--format json` and the
/// real stdio MCP `tools/call verify` return the same envelope, including
/// `candidates` — the defined names in config order, or `[]` when no gate
/// is defined.
/// @vtest.id TEST-VERIFY-UNDEFINED-GATE-CLI-MCP-SAME
/// @vtest.covers VO-VERIFY-UNDEFINED-GATE-CANDIDATES, VO-MCP-TOOL-CLI-EQUIVALENCE
/// @vtest.target crates/vtest-cli/src/lib.rs::run
/// @vtest.intent an unknown gate yields an identical E-CONFIG-002 envelope with the same candidates array from the real CLI JSON and the real stdio MCP verify tool, with and without defined gates
#[test]
fn an_undefined_gate_gives_the_same_candidates_from_cli_json_and_mcp() {
    for (name, gates, expected) in [
        (
            "gate-same-defined",
            &["release", "beta"][..],
            serde_json::json!(["release", "beta"]),
        ),
        ("gate-same-none", &[][..], serde_json::json!([])),
    ] {
        let root = project_with_gates(name, gates);
        let (exit, cli) = cli_verify_gate_json(&root, "Release");
        let mcp = mcp_verify_gate(&root, "Release");
        assert_eq!(exit, Some(2), "{name}: {cli}");
        assert_eq!(cli, mcp, "{name}: CLI JSON and MCP must be identical");
        assert_eq!(cli["ok"], false, "{name}");
        assert_eq!(cli["data"], serde_json::Value::Null, "{name}");
        let diagnostic = &cli["diagnostics"][0];
        assert_eq!(diagnostic["code"], "E-CONFIG-002", "{name}");
        assert_eq!(diagnostic["candidates"], expected, "{name}");
    }
}

/// SPEC-400: the retired 12 items are not checks. Naming one is a usage
/// error, not a silently narrowed scope.
/// @vtest.id TEST-VERIFY-RETIRED-CHECK-NAME
/// @vtest.covers VO-VERIFY-RETIRED-CHECK-NAME-USAGE-ERROR
/// @vtest.target crates/vtest-cli/src/lib.rs::run
/// @vtest.intent naming a retired 12-item check in --items is a usage error, not a silently narrowed scope
#[test]
fn a_retired_check_name_is_a_usage_error() {
    let root = temp_root("retired-item");
    assert_eq!(run(cli(&root, Command::Init { name: None })), ExitCode::Ok);
    let command = Command::Verify {
        items: vec!["spec_coverage".to_owned()],
        doc: None,
        vo: None,
        test: None,
        gate: None,
        summary: false,
    };
    assert_eq!(run(cli(&root, command)), ExitCode::Usage);
}

/// DS-1104: the entity axis takes at most one selector.
/// @vtest.id TEST-VERIFY-COMBINED-ENTITY-SELECTORS
/// @vtest.covers VO-VERIFY-ENTITY-AXIS-SINGLE-SELECTOR
/// @vtest.target crates/vtest-cli/src/lib.rs::run
/// @vtest.intent combining --doc and --vo entity selectors is a usage error
#[test]
fn combining_entity_selectors_is_a_usage_error() {
    let root = temp_root("two-entities");
    assert_eq!(run(cli(&root, Command::Init { name: None })), ExitCode::Ok);
    let command = Command::Verify {
        items: Vec::new(),
        doc: Some("R-001".to_owned()),
        vo: Some("VO-ONE".to_owned()),
        test: None,
        gate: None,
        summary: false,
    };
    assert_eq!(run(cli(&root, command)), ExitCode::Usage);
}

/// DS-935「`vtest scan` / `vtest doctor`では、registry・config・adapter契約の
/// 検証…がE-ADAPTER-* / E-CONFIG-*で拒否された場合は2とする」。A rejected
/// configuration is an operation rejection, not an internal error (exit 3).
/// @vtest.id TEST-SCAN-DOCTOR-REJECTED-CONFIG
/// @vtest.covers VO-SCAN-REJECTED-CONFIG-EXIT-TWO, VO-SCAN-LEGACY-TWELVE-FIELDS-E-CONFIG-001
/// @vtest.target crates/vtest-cli/src/lib.rs::run
/// @vtest.intent a config carrying the retired 12-item enumeration is rejected with exit 2 for both scan and doctor
#[test]
fn a_rejected_configuration_is_exit_two_for_scan_and_doctor() {
    let root = temp_root("bad-config");
    assert_eq!(run(cli(&root, Command::Init { name: None })), ExitCode::Ok);
    // The retired 12-item enumeration is E-CONFIG-001 regardless of version
    // (DS-1108) — the exact shape this repository's own `.verify/config.yaml`
    // still carries.
    let config = root.join(".verify").join("config.yaml");
    let text = std::fs::read_to_string(&config).expect("read config");
    std::fs::write(
        &config,
        text.replace(
            "- chain_integrity",
            "- spec_coverage\n    - chain_integrity",
        ),
    )
    .expect("write config");
    assert_eq!(run(cli(&root, Command::Scan)), ExitCode::Usage);
    assert_eq!(run(cli(&root, Command::Doctor)), ExitCode::Usage);
}

/// DS-1107「`config.yaml` の `verify.full_scope` は…項目選択 knob として
/// 使用しない」and DS-356 (version 2 rejects a `full_scope` with 重複・未知
/// 項目・欠落・余剰). A configuration naming fewer than the fixed four is
/// refused outright — never quietly honoured as a narrowed scope, which would
/// report three unrun checks as though they had passed.
/// @vtest.id TEST-VERIFY-SUBSET-FULL-SCOPE-REJECTED
/// @vtest.covers VO-VERIFY-CONFIG-FULL-SCOPE-NOT-ITEM-KNOB
/// @vtest.target crates/vtest-cli/src/lib.rs::run
/// @vtest.intent a config naming fewer than the fixed four checks in full_scope is rejected outright, never honoured as a narrowed selection
#[test]
fn a_subset_full_scope_is_rejected_not_honoured_as_a_selection() {
    let root = temp_root("subset-full-scope");
    assert_eq!(run(cli(&root, Command::Init { name: None })), ExitCode::Ok);
    let config = root.join(".verify").join("config.yaml");
    let text = std::fs::read_to_string(&config).expect("read config");
    let narrowed = text
        .replace("  - orphan_detection\n", "")
        .replace("  - target_binding\n", "")
        .replace("  - oracle_presence\n", "");
    assert_ne!(
        narrowed, text,
        "the fixture config must actually be narrowed"
    );
    std::fs::write(&config, narrowed).expect("write narrowed config");

    assert_eq!(
        run(cli(&root, verify_command())),
        ExitCode::Usage,
        "a subset full_scope must be rejected, never used as an item selection"
    );
}

/// An empty, freshly initialised project has no document nodes, no VOs and no
/// Tests. It must NOT come out as a complete-verification OK: DS-253「`vtest
/// verify` は部分的な登録・判断・実行状態を総合 `OK` として扱わない」。
///
/// This is the single most important negative case in this file — an empty
/// repository is exactly the input a false PASS would sail through.
/// @vtest.id TEST-VERIFY-EMPTY-PROJECT-NOT-OK
/// @vtest.covers VO-VERIFY-NO-RESULT-NOT-PASS, VO-VERIFY-INCOMPLETE-NOT-OVERALL-OK, VO-VERIFY-FOUR-CHECKS-ONLY
/// @vtest.target crates/vtest-cli/src/lib.rs::run
/// @vtest.intent an empty freshly-initialised project must evaluate and come out NG, never a complete-verification OK
#[test]
fn an_empty_project_is_never_a_complete_verification_ok() {
    let root = temp_root("empty");
    assert_eq!(run(cli(&root, Command::Init { name: None })), ExitCode::Ok);
    // Exit 1 specifically, not merely "not 0": verification must actually run
    // and return NG. Exit 2 would mean the run was rejected before evaluating
    // anything, which would leave the false-PASS question untested.
    assert_eq!(
        run(cli(&root, verify_command())),
        ExitCode::VerificationFailed,
        "an empty project must be evaluated and come out NG"
    );
}

/// End-to-end on a *populated* canonical project: a real `.verify/doc/` node,
/// a real VO deriving from it, and no Test covering that VO. The whole path
/// (config -> scan -> verify -> aggregation -> exit code) must run and land on
/// exit 1 via `chain_integrity = MISMATCH` (REQ-056 / ROOT-034: the retired
/// `test_existence` is folded into `chain_integrity`).
///
/// The empty-project case above cannot show this, because it never populates
/// anything; this is the test that proves the wiring works on real records.
/// @vtest.id TEST-VERIFY-UNCOVERED-LEAF-VO-NG
/// @vtest.covers VO-VERIFY-UNCOVERED-LEAF-VO-CHAIN-INTEGRITY-MISMATCH
/// @vtest.target crates/vtest-cli/src/lib.rs::run
/// @vtest.intent a populated project with a leaf VO no Test covers must run end to end and land on exit 1 via chain_integrity = MISMATCH
#[test]
fn a_populated_project_with_an_uncovered_leaf_vo_is_ng() {
    use vtest_model::{
        DerivesFrom, DocumentFile, DocumentId, NodeSource, RootNode, SentenceNode, VoId, VoRecord,
    };
    use vtest_store::{write_document_file, write_vo_record, VerifyLayout};

    let root = temp_root("populated");
    assert_eq!(run(cli(&root, Command::Init { name: None })), ExitCode::Ok);
    let layout = VerifyLayout::new(&root);

    let source = |id: &str| NodeSource {
        doc: format!("{id}.md"),
        heading: "acceptance".to_owned(),
        lines: [1, 1],
    };
    let document = DocumentFile {
        schema_version: "0.1".to_owned(),
        root: vec![RootNode {
            id: DocumentId::new("ROOT-001"),
            statement: "acceptance root".to_owned(),
            description: None,
            source: source("root"),
        }],
        request: vec![SentenceNode {
            id: DocumentId::new("R-001"),
            statement: "acceptance request".to_owned(),
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
    write_document_file(&layout, "acceptance", &document).expect("write document");
    write_vo_record(
        &layout,
        &VoRecord {
            id: VoId::new("VO-UNCOVERED"),
            parent: None,
            derives_from: vec![DerivesFrom {
                doc: DocumentId::new("R-001"),
                anchor: None,
                note: None,
            }],
            claim: "nothing covers this".to_owned(),
            dimensions: Vec::new(),
            coverage_policy: None,
            combinations: Vec::new(),
            representative_cases: Vec::new(),
            created: "2026-09-08T00:00:00Z".to_owned(),
            updated: "2026-09-08T00:00:00Z".to_owned(),
        },
    )
    .expect("write VO");

    assert_eq!(
        run(cli(&root, verify_command())),
        ExitCode::VerificationFailed,
        "a leaf VO with no covering Test must make the run NG"
    );
}
