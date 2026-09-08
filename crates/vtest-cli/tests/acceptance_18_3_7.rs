//! Acceptance coverage for 別紙C §18.3.7 承認と判断記録の分離
//! (DES-546/547).
//!
//! DES-546「実効承認の導出は `approved_state` を参照する」and DES-547
//! 「承認主体は種別（`human` / `agent`）と識別子を記録する」are already
//! exercised at the record layer by `crates/vtest-store/src/records.rs`'s
//! `approval_round_trip_requires_a_traceable_approver` and at the CLI
//! layer by `crates/vtest-cli/tests/approval_acceptance.rs`. This file adds
//! the one assertion those don't already cover: that the CLI's effective-
//! state derivation (`ops::approval::show`) is driven by `approved_state`
//! alone, and an `agent` approver is recorded and traceable identically to
//! a `human` one (DES-547's two-kind domain, not just `human`).

use std::{
    env, fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use vtest_cli::{
    run, ApprovalCommand, ApprovedStateArg, ApproverKindArg, Cli, Command, OutputFormat,
    SubjectTypeArg,
};
use vtest_model::{
    DerivesFrom, DocumentFile, DocumentId, ExitCode, NodeSource, RootNode, SentenceNode, VoId,
    VoRecord,
};
use vtest_store::{
    approval::EffectiveApprovalState, init_project, write_document_file, write_vo_record,
};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn temp_root(name: &str) -> PathBuf {
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let root = env::temp_dir().join(format!(
        "vtest-cli-acceptance-18-3-7-{name}-{}-{nanos}-{sequence}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create fixture root");
    root
}

fn build_fixture(root: &std::path::Path) {
    let layout = init_project(root, "acceptance-18-3-7-fixture").expect("init .verify/ layout");
    let source = NodeSource {
        doc: "fixture.md".to_owned(),
        heading: "fixture".to_owned(),
        lines: [1, 1],
    };
    write_document_file(
        &layout,
        "fixture",
        &DocumentFile {
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
        },
    )
    .expect("write document file");
    write_vo_record(
        &layout,
        &VoRecord {
            id: VoId::new("VO-ACCEPTANCE-18-3-7"),
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

fn cli(root: &std::path::Path, command: Command) -> Cli {
    Cli {
        project: root.to_path_buf(),
        format: OutputFormat::Json,
        quiet: true,
        command,
    }
}

/// DES-547: an `agent` approver is recorded and drives the effective state
/// exactly as a `human` one does — DES-547's domain is the two kinds
/// together, not `human` alone (the domain the rest of this suite's fixture
/// coverage happens to exercise).
#[test]
fn an_agent_approver_is_traceable_and_drives_effective_state() {
    let root = temp_root("agent-approver");
    build_fixture(&root);
    let exit = run(cli(
        &root,
        Command::Approval(ApprovalCommand::Create {
            subject_type: SubjectTypeArg::Vo,
            subject_id: "VO-ACCEPTANCE-18-3-7".to_owned(),
            state: ApprovedStateArg::Approved,
            approver_kind: ApproverKindArg::Agent,
            approver_id: "claude-fable-5-1".to_owned(),
            model: Some("claude-fable-5-1".to_owned()),
            basis: Vec::new(),
            supersedes: Vec::new(),
        }),
    ));
    assert_eq!(exit, ExitCode::Ok);

    let layout = vtest_store::VerifyLayout::new(&root);
    let result = vtest_cli::ops::approval::show(&layout, "vo", "VO-ACCEPTANCE-18-3-7")
        .expect("show must succeed");
    assert_eq!(
        result.effective_state,
        EffectiveApprovalState::Approved,
        "DES-546: effective approval derives from approved_state alone -- an agent-kind \
         approver is not treated differently from a human one"
    );
    assert_eq!(result.records.len(), 1);
    assert_eq!(result.records[0].approver.kind, "agent");
    assert_eq!(result.records[0].approver.id, "claude-fable-5-1");
}

/// DES-546: withdrawing (a fresh record with `approved_state: withdrawn`
/// superseding the prior one) drops the effective state back to `draft`
/// purely because `approved_state` changed on the record the effective-set
/// computation reads -- no separate "withdrawal" concept exists outside
/// that field.
#[test]
fn effective_state_tracks_approved_state_across_a_withdrawal() {
    let root = temp_root("state-tracking");
    build_fixture(&root);
    let layout = vtest_store::VerifyLayout::new(&root);

    let created = vtest_cli::ops::approval::create(
        &layout,
        vtest_cli::ops::approval::CreateArgs {
            subject_type: "vo".to_owned(),
            subject_id: "VO-ACCEPTANCE-18-3-7".to_owned(),
            approved_state: "approved".to_owned(),
            approver_kind: "human".to_owned(),
            approver_id: "reviewer".to_owned(),
            approver_model: None,
            basis: Vec::new(),
            supersedes: Vec::new(),
        },
    )
    .expect("create must succeed");

    let before = vtest_cli::ops::approval::show(&layout, "vo", "VO-ACCEPTANCE-18-3-7")
        .expect("show must succeed");
    assert_eq!(before.effective_state, EffectiveApprovalState::Approved);

    vtest_cli::ops::approval::withdraw(
        &layout,
        vtest_cli::ops::approval::WithdrawArgs {
            approval_id: created.id,
            approver_kind: "human".to_owned(),
            approver_id: "reviewer".to_owned(),
            approver_model: None,
            basis: Vec::new(),
        },
    )
    .expect("withdraw must succeed");

    let after = vtest_cli::ops::approval::show(&layout, "vo", "VO-ACCEPTANCE-18-3-7")
        .expect("show must succeed");
    assert_eq!(
        after.effective_state,
        EffectiveApprovalState::Draft,
        "DES-546: effective state must fall back to draft once the referenced record's \
         approved_state chain no longer resolves to an all-approved effective set"
    );
}
