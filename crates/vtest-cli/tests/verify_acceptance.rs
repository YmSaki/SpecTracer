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
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vtest-cli-{name}-{suffix}"));
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
    let exit = run(cli(&root, verify_command()));
    assert_ne!(exit, ExitCode::Ok, "an empty project must not verify as OK");
}
