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

/// SPEC-400: the retired 12 items are not checks. Naming one is a usage
/// error, not a silently narrowed scope.
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
