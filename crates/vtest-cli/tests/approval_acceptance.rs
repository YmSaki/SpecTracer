//! Acceptance coverage for `vtest approval create|withdraw|show` (本冊 §3.5,
//! DS-1050-1062, DS-1461-1490).

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
use vtest_store::{init_project, write_document_file, write_vo_record};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn temp_root(name: &str) -> PathBuf {
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let root = env::temp_dir().join(format!(
        "vtest-cli-approval-acceptance-{name}-{}-{nanos}-{sequence}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create fixture root");
    root
}

fn source(id: &str) -> NodeSource {
    NodeSource {
        doc: format!("{id}.md"),
        heading: "fixture".to_owned(),
        lines: [1, 1],
    }
}

fn document_file() -> DocumentFile {
    DocumentFile {
        schema_version: "0.1".to_owned(),
        root: vec![RootNode {
            id: DocumentId::new("ROOT-001"),
            statement: "fixture root".to_owned(),
            description: None,
            source: source("root"),
        }],
        request: vec![SentenceNode {
            id: DocumentId::new("R-001"),
            statement: "fixture requirement".to_owned(),
            description: None,
            derives_from: vec![DocumentId::new("ROOT-001")],
            cites: None,
            source: source("req"),
        }],
        require: Vec::new(),
        spec: Vec::new(),
        detailed_spec: Vec::new(),
        basic_design: Vec::new(),
        design: Vec::new(),
    }
}

fn vo(id: &str) -> VoRecord {
    VoRecord {
        id: VoId::new(id),
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
    }
}

fn build_fixture_project(root: &std::path::Path) {
    let layout = init_project(root, "vtest-approval-fixture").expect("init .verify/ layout");
    write_document_file(&layout, "fixture", &document_file()).expect("write document file");
    write_vo_record(&layout, &vo("VO-APPROVAL-DOUBLE")).expect("write VO record");
}

/// Counts `.yaml` record files under `.verify/approvals/`, excluding the
/// `.gitkeep` placeholder `init_project` writes into every record directory.
fn approval_record_count(root: &std::path::Path) -> usize {
    let approvals_dir = root.join(".verify").join("approvals");
    fs::read_dir(&approvals_dir).map_or(0, |entries| {
        entries
            .flatten()
            .filter(|entry| {
                entry.path().extension().and_then(|value| value.to_str()) == Some("yaml")
            })
            .count()
    })
}

fn cli(root: &std::path::Path, command: Command) -> Cli {
    Cli {
        project: root.to_path_buf(),
        format: OutputFormat::Json,
        quiet: true,
        command,
    }
}

fn create_command(subject_id: &str, state: ApprovedStateArg, supersedes: Vec<String>) -> Command {
    Command::Approval(ApprovalCommand::Create {
        subject_type: SubjectTypeArg::Vo,
        subject_id: subject_id.to_owned(),
        state,
        approver_kind: ApproverKindArg::Human,
        approver_id: "reviewer".to_owned(),
        model: None,
        basis: Vec::new(),
        supersedes,
    })
}

/// DS-1058/E-APPROVAL-001: a VO that does not exist is a usage rejection,
/// not a silently-written record.
/// @vtest.id TEST-APPROVAL-UNRESOLVED-VO-SUBJECT
/// @vtest.covers VO-APPROVAL-CREATE-UNRESOLVED-SUBJECT-REJECTED
/// @vtest.target crates/vtest-cli/src/ops/approval.rs::create
/// @vtest.intent creating an approval for a VO that does not exist is a usage rejection, not a silently-written record
#[test]
fn create_for_an_unresolved_vo_subject_is_a_usage_error() {
    let root = temp_root("unresolved-vo");
    build_fixture_project(&root);
    let exit = run(cli(
        &root,
        create_command("VO-DOES-NOT-EXIST", ApprovedStateArg::Approved, Vec::new()),
    ));
    assert_eq!(exit, ExitCode::Usage);
    assert_eq!(
        approval_record_count(&root),
        0,
        "no record should be written on rejection"
    );
}

/// DS-1052: `--subject-type judgment` is explicitly rejected (no judgment
/// record domain exists in this codebase yet) rather than silently
/// mishandled.
/// @vtest.id TEST-APPROVAL-JUDGMENT-SUBJECT-TYPE
/// @vtest.covers VO-APPROVAL-CREATE-UNRESOLVED-SUBJECT-REJECTED
/// @vtest.target crates/vtest-cli/src/ops/approval.rs::create
/// @vtest.intent --subject-type judgment can never resolve (no judgment record domain exists yet), so create rejects it as a usage error rather than mishandling it silently
#[test]
fn create_with_subject_type_judgment_is_a_usage_error() {
    let root = temp_root("judgment-unsupported");
    build_fixture_project(&root);
    let exit = run(cli(
        &root,
        Command::Approval(ApprovalCommand::Create {
            subject_type: SubjectTypeArg::Judgment,
            subject_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".to_owned(),
            state: ApprovedStateArg::Approved,
            approver_kind: ApproverKindArg::Human,
            approver_id: "reviewer".to_owned(),
            model: None,
            basis: Vec::new(),
            supersedes: Vec::new(),
        }),
    ));
    assert_eq!(exit, ExitCode::Usage);
}

/// A successful `create` writes exactly one record to `.verify/approvals/`
/// and exits 0.
/// @vtest.id TEST-APPROVAL-RESOLVED-VO-SUBJECT
/// @vtest.covers VO-APPROVAL-CREATE-VO-SUBJECT-WRITES-RECORD
/// @vtest.target crates/vtest-cli/src/ops/approval.rs::create
/// @vtest.intent a successful create for a resolved VO subject writes exactly one record and exits 0
#[test]
fn create_for_a_resolved_vo_subject_writes_a_record_and_exits_ok() {
    let root = temp_root("resolved-vo");
    build_fixture_project(&root);
    let exit = run(cli(
        &root,
        create_command("VO-APPROVAL-DOUBLE", ApprovedStateArg::Approved, Vec::new()),
    ));
    assert_eq!(exit, ExitCode::Ok);
    assert_eq!(approval_record_count(&root), 1);
    let record_path = fs::read_dir(root.join(".verify").join("approvals"))
        .expect("approvals dir exists")
        .flatten()
        .find(|entry| entry.path().extension().and_then(|value| value.to_str()) == Some("yaml"))
        .expect("one approval record")
        .path();
    let record = vtest_store::read_approval(&record_path).expect("approval record parses");
    assert_eq!(record.subject, "VO-APPROVAL-DOUBLE");
}

/// DS-1466/1467: after one `approved` record with a current subject_hash and
/// dependency closure, `show` reports effective state `approved`.
/// @vtest.id TEST-APPROVAL-SHOW-EFFECTIVE-APPROVED
/// @vtest.covers VO-APPROVAL-EFFECTIVE-STATE-FROM-VALID-RECORD-SET
/// @vtest.target crates/vtest-cli/src/ops/approval.rs::show
/// @vtest.intent after one approved record with a current subject_hash and dependency closure, show reports effective state approved
#[test]
fn show_reports_approved_after_a_single_current_approved_record() {
    let root = temp_root("effective-approved");
    build_fixture_project(&root);
    let create_exit = run(cli(
        &root,
        create_command("VO-APPROVAL-DOUBLE", ApprovedStateArg::Approved, Vec::new()),
    ));
    assert_eq!(create_exit, ExitCode::Ok);

    let show_exit = run(cli(
        &root,
        Command::Approval(ApprovalCommand::Show {
            subject_type: SubjectTypeArg::Vo,
            subject_id: "VO-APPROVAL-DOUBLE".to_owned(),
        }),
    ));
    assert_eq!(show_exit, ExitCode::Ok);
    let layout = vtest_store::VerifyLayout::new(&root);
    let shown = vtest_cli::ops::approval::show(&layout, "vo", "VO-APPROVAL-DOUBLE")
        .expect("approval show succeeds");
    assert_eq!(
        shown.effective_state,
        vtest_store::approval::EffectiveApprovalState::Approved
    );
}

/// BD-307/DS-1056: `withdraw` writes a new record (`state: withdrawn`,
/// `supersedes: [approval-id]`) rather than mutating the original — the
/// approvals directory grows from 1 to 2 records, and the effective state
/// drops back to `draft` (DS-1468/1469).
/// @vtest.id TEST-APPROVAL-WITHDRAW-NEW-RECORD
/// @vtest.covers VO-APPROVAL-WITHDRAW-NEW-RECORD-SUPERSEDES
/// @vtest.target crates/vtest-cli/src/ops/approval.rs::withdraw
/// @vtest.intent withdraw writes a new record (state withdrawn, supersedes the original id) instead of mutating the original
#[test]
fn withdraw_writes_a_new_record_referencing_the_original() {
    let root = temp_root("withdraw");
    build_fixture_project(&root);
    let create_exit = run(cli(
        &root,
        create_command("VO-APPROVAL-DOUBLE", ApprovedStateArg::Approved, Vec::new()),
    ));
    assert_eq!(create_exit, ExitCode::Ok);

    let approvals_dir = root.join(".verify").join("approvals");
    let created_id = fs::read_dir(&approvals_dir)
        .expect("approvals dir exists")
        .flatten()
        .find(|entry| entry.path().extension().and_then(|value| value.to_str()) == Some("yaml"))
        .expect("one .yaml record written")
        .path()
        .file_stem()
        .expect("file stem")
        .to_string_lossy()
        .into_owned();

    let withdraw_exit = run(cli(
        &root,
        Command::Approval(ApprovalCommand::Withdraw {
            approval_id: created_id.clone(),
            approver_kind: ApproverKindArg::Human,
            approver_id: "reviewer".to_owned(),
            model: None,
            basis: Vec::new(),
        }),
    ));
    assert_eq!(withdraw_exit, ExitCode::Ok);
    assert_eq!(
        approval_record_count(&root),
        2,
        "withdraw must add a record, not mutate the original"
    );

    // BD-307: the new record's `supersedes` must actually name the id
    // `withdraw` was given, not merely be present.
    let layout = vtest_store::VerifyLayout::new(&root);
    let records = vtest_store::approval::read_all_approvals(&layout).expect("read written records");
    let withdrawal = records
        .iter()
        .find(|record| record.approved_state == "withdrawn")
        .expect("the withdrawal record must exist");
    assert_eq!(
        withdrawal.supersedes,
        vec![created_id],
        "the withdrawal record must supersede exactly the approval id it was given"
    );
}

/// BD-307/DS-1058: `withdraw` re-resolves the target's subject against its
/// *current* state (it is `create` on the target's own `subject_type`/
/// `subject_id`, not a verbatim copy of the target record's binding) — if
/// the subject has since become unresolvable, `withdraw` fails with
/// E-APPROVAL-001 exactly as `create` would, rather than silently writing
/// a withdrawal record bound to a stale (now-nonexistent) subject.
/// @vtest.id TEST-APPROVAL-WITHDRAW-RE-RESOLVE
/// @vtest.covers VO-APPROVAL-CREATE-UNRESOLVED-SUBJECT-REJECTED
/// @vtest.target crates/vtest-cli/src/ops/approval.rs::withdraw
/// @vtest.intent withdraw re-resolves the target's subject against its current state and fails with a usage error if the subject has become unresolvable, rather than writing a withdrawal bound to a stale subject
#[test]
fn withdraw_re_resolves_the_subject_and_fails_if_it_is_now_unresolvable() {
    let root = temp_root("withdraw-re-resolve");
    build_fixture_project(&root);
    let create_exit = run(cli(
        &root,
        create_command("VO-APPROVAL-DOUBLE", ApprovedStateArg::Approved, Vec::new()),
    ));
    assert_eq!(create_exit, ExitCode::Ok);

    let approvals_dir = root.join(".verify").join("approvals");
    let created_id = fs::read_dir(&approvals_dir)
        .expect("approvals dir exists")
        .flatten()
        .find(|entry| entry.path().extension().and_then(|value| value.to_str()) == Some("yaml"))
        .expect("one .yaml record written")
        .path()
        .file_stem()
        .expect("file stem")
        .to_string_lossy()
        .into_owned();

    // Remove the VO the approval targets -- its subject can no longer be
    // resolved at all.
    let layout = vtest_store::VerifyLayout::new(&root);
    fs::remove_file(layout.vo_dir().join("VO-APPROVAL-DOUBLE.yaml"))
        .expect("remove the VO record to make the subject unresolvable");

    let withdraw_exit = run(cli(
        &root,
        Command::Approval(ApprovalCommand::Withdraw {
            approval_id: created_id,
            approver_kind: ApproverKindArg::Human,
            approver_id: "reviewer".to_owned(),
            model: None,
            basis: Vec::new(),
        }),
    ));
    assert_eq!(
        withdraw_exit,
        ExitCode::Usage,
        "withdraw must re-resolve the subject (DS-1058), not copy the target's stale binding"
    );
    assert_eq!(
        approval_record_count(&root),
        1,
        "no withdrawal record should be written when the subject cannot be re-resolved"
    );
}

/// DS-1051/1480: `--subject-type document` resolves against
/// `.verify/doc/*.json` node ids (not the VO domain `--subject-type vo`
/// exercises everywhere else in this file) and writes a record.
/// @vtest.id TEST-APPROVAL-RESOLVED-DOCUMENT-SUBJECT
/// @vtest.covers VO-APPROVAL-DOCUMENT-SUBJECT-ID, VO-APPROVAL-DOCUMENT-DEPENDENCY-CLOSURE
/// @vtest.target crates/vtest-cli/src/ops/approval.rs::create
/// @vtest.intent --subject-type document resolves against .verify/doc/*.json node ids and writes a record
#[test]
fn create_for_a_resolved_document_subject_writes_a_record_and_exits_ok() {
    let root = temp_root("resolved-document");
    build_fixture_project(&root);
    let exit = run(cli(
        &root,
        Command::Approval(ApprovalCommand::Create {
            subject_type: SubjectTypeArg::Document,
            subject_id: "R-001".to_owned(),
            state: ApprovedStateArg::Approved,
            approver_kind: ApproverKindArg::Human,
            approver_id: "reviewer".to_owned(),
            model: None,
            basis: Vec::new(),
            supersedes: Vec::new(),
        }),
    ));
    assert_eq!(exit, ExitCode::Ok);
    assert_eq!(approval_record_count(&root), 1);
}

/// DS-1058/E-APPROVAL-001: a document node id that does not exist in any
/// registered `.verify/doc/*.json` file is a usage rejection under
/// `--subject-type document`, mirroring the `vo` case.
/// @vtest.id TEST-APPROVAL-UNRESOLVED-DOCUMENT-SUBJECT
/// @vtest.covers VO-APPROVAL-CREATE-UNRESOLVED-SUBJECT-REJECTED
/// @vtest.target crates/vtest-cli/src/ops/approval.rs::create
/// @vtest.intent a document node id that does not exist in any registered .verify/doc/*.json file is a usage rejection under --subject-type document
#[test]
fn create_for_an_unresolved_document_subject_is_a_usage_error() {
    let root = temp_root("unresolved-document");
    build_fixture_project(&root);
    let exit = run(cli(
        &root,
        Command::Approval(ApprovalCommand::Create {
            subject_type: SubjectTypeArg::Document,
            subject_id: "R-DOES-NOT-EXIST".to_owned(),
            state: ApprovedStateArg::Approved,
            approver_kind: ApproverKindArg::Human,
            approver_id: "reviewer".to_owned(),
            model: None,
            basis: Vec::new(),
            supersedes: Vec::new(),
        }),
    ));
    assert_eq!(exit, ExitCode::Usage);
    assert_eq!(approval_record_count(&root), 0);
}

/// DS-1059/E-APPROVAL-002: `withdraw` naming an approval id that does not
/// exist is a usage rejection.
/// @vtest.id TEST-APPROVAL-WITHDRAW-UNKNOWN-ID
/// @vtest.covers VO-APPROVAL-WITHDRAW-UNKNOWN-ID-REJECTED
/// @vtest.target crates/vtest-cli/src/ops/approval.rs::withdraw
/// @vtest.intent withdraw naming an approval id that does not exist is a usage rejection
#[test]
fn withdraw_of_an_unknown_approval_id_is_a_usage_error() {
    let root = temp_root("withdraw-unknown");
    build_fixture_project(&root);
    let exit = run(cli(
        &root,
        Command::Approval(ApprovalCommand::Withdraw {
            approval_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".to_owned(),
            approver_kind: ApproverKindArg::Human,
            approver_id: "reviewer".to_owned(),
            model: None,
            basis: Vec::new(),
        }),
    ));
    assert_eq!(exit, ExitCode::Usage);
}

fn show_command(subject_id: &str) -> Command {
    Command::Approval(ApprovalCommand::Show {
        subject_type: SubjectTypeArg::Vo,
        subject_id: subject_id.to_owned(),
    })
}

fn withdraw_command(approval_id: &str) -> Command {
    Command::Approval(ApprovalCommand::Withdraw {
        approval_id: approval_id.to_owned(),
        approver_kind: ApproverKindArg::Human,
        approver_id: "reviewer".to_owned(),
        model: None,
        basis: Vec::new(),
    })
}

fn approval_paths(root: &std::path::Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = fs::read_dir(root.join(".verify").join("approvals"))
        .expect("approvals dir exists")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("yaml"))
        .collect();
    paths.sort();
    paths
}

fn record_id(path: &std::path::Path) -> String {
    path.file_stem()
        .expect("file stem")
        .to_string_lossy()
        .into_owned()
}

fn effective(root: &std::path::Path, subject_id: &str) -> vtest_cli::ops::approval::ShowResult {
    let layout = vtest_store::VerifyLayout::new(root);
    vtest_cli::ops::approval::show(&layout, "vo", subject_id).expect("approval show succeeds")
}

/// DS-1676: a schema-invalid record in the approval set (here an undeclared
/// field appended to the withdrawal) is never skipped. Skipping it would
/// drop the withdrawal and resurrect the approval it withdrew, so show,
/// create and withdraw all fail instead.
/// @vtest.id TEST-APPROVAL-BROKEN-RECORD-FAILS-CLOSED
/// @vtest.covers VO-STORE-APPROVAL-UNKNOWN-FIELD-REJECTED
/// @vtest.target crates/vtest-cli/src/ops/approval.rs::show
/// @vtest.intent a schema-broken withdrawal makes show, create and withdraw fail instead of reading approved
#[test]
fn a_schema_broken_withdrawal_fails_closed_instead_of_resurrecting_the_approval() {
    let root = temp_root("broken-withdrawal");
    build_fixture_project(&root);
    assert_eq!(
        run(cli(
            &root,
            create_command("VO-APPROVAL-DOUBLE", ApprovedStateArg::Approved, Vec::new()),
        )),
        ExitCode::Ok
    );
    let approved_id = record_id(&approval_paths(&root)[0]);
    assert_eq!(
        run(cli(&root, withdraw_command(&approved_id))),
        ExitCode::Ok
    );
    assert_eq!(
        effective(&root, "VO-APPROVAL-DOUBLE").effective_state,
        vtest_store::approval::EffectiveApprovalState::Draft
    );

    let withdrawal = approval_paths(&root)
        .into_iter()
        .find(|path| record_id(path) != approved_id)
        .expect("withdrawal record exists");
    let mut text = fs::read_to_string(&withdrawal).expect("read withdrawal");
    text.push_str("unexpected: true\n");
    fs::write(&withdrawal, text).expect("corrupt withdrawal");

    let layout = vtest_store::VerifyLayout::new(&root);
    match vtest_cli::ops::approval::show(&layout, "vo", "VO-APPROVAL-DOUBLE") {
        Err(vtest_cli::ops::approval::ApprovalOpError::Store(error)) => {
            assert_eq!(error.record_read_code(), Some("E-SCAN-010"), "{error}");
        }
        Err(other) => panic!("unexpected error: {other}"),
        Ok(result) => panic!(
            "show must fail on a broken record, got {:?}",
            result.effective_state
        ),
    }
    assert_eq!(
        run(cli(&root, show_command("VO-APPROVAL-DOUBLE"))),
        ExitCode::Usage
    );
    assert_eq!(
        run(cli(
            &root,
            create_command("VO-APPROVAL-DOUBLE", ApprovedStateArg::Approved, Vec::new()),
        )),
        ExitCode::Usage
    );
    assert_eq!(
        run(cli(&root, withdraw_command(&approved_id))),
        ExitCode::Usage
    );
    assert_eq!(
        approval_record_count(&root),
        2,
        "no record may be appended on top of an unreadable approval set"
    );
}

/// DS-556: an existing record whose `supersedes` no longer resolves is
/// excluded from the effective set with W-STORE-005, so the subject is not
/// approved; a well-formed re-approval of the same subject still is.
/// @vtest.id TEST-APPROVAL-INVALID-SUPERSEDES-NOT-APPROVED
/// @vtest.covers VO-STORE-APPROVAL-SUPERSEDES-GRAPH-EXCLUDED
/// @vtest.target crates/vtest-cli/src/ops/approval.rs::show
/// @vtest.intent an approval whose stored supersedes is unresolved reads as draft with W-STORE-005
#[test]
fn an_approval_with_an_unresolved_stored_supersedes_is_not_approved() {
    let root = temp_root("invalid-supersedes");
    build_fixture_project(&root);
    assert_eq!(
        run(cli(
            &root,
            create_command("VO-APPROVAL-DOUBLE", ApprovedStateArg::Approved, Vec::new()),
        )),
        ExitCode::Ok
    );
    let first = approval_paths(&root)[0].clone();
    let first_id = record_id(&first);
    assert_eq!(
        run(cli(
            &root,
            create_command(
                "VO-APPROVAL-DOUBLE",
                ApprovedStateArg::Approved,
                vec![first_id.clone()],
            ),
        )),
        ExitCode::Ok
    );
    let reapproved = effective(&root, "VO-APPROVAL-DOUBLE");
    assert_eq!(
        reapproved.effective_state,
        vtest_store::approval::EffectiveApprovalState::Approved
    );
    assert!(reapproved.diagnostics.is_empty());

    let second = approval_paths(&root)
        .into_iter()
        .find(|path| record_id(path) != first_id)
        .expect("second record exists");
    let mut record = vtest_store::records::read_approval(&second).expect("read second record");
    record.supersedes = vec!["01ZZZZZZZZZZZZZZZZZZZZZZZZ".to_owned()];
    fs::write(&second, record.to_yaml().expect("serialize")).expect("rewrite supersedes");

    let shown = effective(&root, "VO-APPROVAL-DOUBLE");
    assert_eq!(
        shown.effective_state,
        vtest_store::approval::EffectiveApprovalState::Draft
    );
    assert!(
        shown
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "W-STORE-005"),
        "{:?}",
        shown.diagnostics
    );
}

fn vo_with(id: &str, parent: Option<&str>, doc: &str) -> VoRecord {
    let mut record = vo(id);
    record.parent = parent.map(VoId::new);
    record.derives_from[0].doc = DocumentId::new(doc);
    record
}

fn project_with_vos(name: &str, vos: &[VoRecord]) -> PathBuf {
    let root = temp_root(name);
    let layout = init_project(&root, "vtest-approval-fixture").expect("init .verify/ layout");
    write_document_file(&layout, "fixture", &document_file()).expect("write document file");
    for record in vos {
        write_vo_record(&layout, record).expect("write VO record");
    }
    root
}

/// DS-1058: a VO whose parent or `derives_from` cannot be resolved has an
/// incomplete dependency closure, so create fails with E-APPROVAL-001 and
/// adds no record, and show never derives approved from it. A complete
/// multi-level closure still resolves.
/// @vtest.id TEST-APPROVAL-UNRESOLVED-DEPENDENCY-CLOSURE
/// @vtest.covers VO-APPROVAL-CREATE-UNRESOLVED-SUBJECT-REJECTED
/// @vtest.target crates/vtest-cli/src/ops/approval.rs::create
/// @vtest.intent a dangling parent or missing derives_from document rejects create and never reads approved
#[test]
fn create_rejects_a_vo_with_an_unresolved_dependency_closure() {
    for (name, records) in [
        (
            "missing-parent",
            vec![vo_with("VO-CHILD", Some("VO-MISSING"), "R-001")],
        ),
        (
            "missing-document",
            vec![vo_with("VO-CHILD", None, "DOC-MISSING")],
        ),
        (
            "ancestor-missing-document",
            vec![
                vo_with("VO-PARENT", None, "DOC-MISSING"),
                vo_with("VO-CHILD", Some("VO-PARENT"), "R-001"),
            ],
        ),
    ] {
        let root = project_with_vos(name, &records);
        let layout = vtest_store::VerifyLayout::new(&root);
        let result = vtest_cli::ops::approval::create(
            &layout,
            vtest_cli::ops::approval::CreateArgs {
                subject_type: "vo".to_owned(),
                subject_id: "VO-CHILD".to_owned(),
                approved_state: "approved".to_owned(),
                approver_kind: "human".to_owned(),
                approver_id: "reviewer".to_owned(),
                approver_model: None,
                basis: Vec::new(),
                supersedes: Vec::new(),
            },
        );
        assert!(
            matches!(
                result,
                Err(vtest_cli::ops::approval::ApprovalOpError::UnresolvedSubject(_))
            ),
            "{name}"
        );
        assert_eq!(
            run(cli(
                &root,
                create_command("VO-CHILD", ApprovedStateArg::Approved, Vec::new()),
            )),
            ExitCode::Usage,
            "{name}"
        );
        assert_eq!(approval_record_count(&root), 0, "{name}");
        assert_eq!(
            effective(&root, "VO-CHILD").effective_state,
            vtest_store::approval::EffectiveApprovalState::Draft,
            "{name}"
        );
    }

    let root = project_with_vos(
        "multi-level",
        &[
            vo_with("VO-PARENT", None, "ROOT-001"),
            vo_with("VO-CHILD", Some("VO-PARENT"), "R-001"),
        ],
    );
    assert_eq!(
        run(cli(
            &root,
            create_command("VO-CHILD", ApprovedStateArg::Approved, Vec::new()),
        )),
        ExitCode::Ok
    );
    assert_eq!(
        effective(&root, "VO-CHILD").effective_state,
        vtest_store::approval::EffectiveApprovalState::Approved
    );
}

/// DS-1676 companion: an approval record that cannot be read at all (an
/// I/O failure, here a directory where a record file is expected) is an
/// operation failure, never an empty or shrunken approval set.
/// @vtest.id TEST-APPROVAL-UNREADABLE-RECORD-FAILS
/// @vtest.covers VO-STORE-APPROVAL-UNKNOWN-FIELD-REJECTED
/// @vtest.target crates/vtest-cli/src/ops/approval.rs::show
/// @vtest.intent an unreadable approval record makes show and create fail instead of being skipped
#[test]
fn an_unreadable_approval_record_makes_show_and_create_fail() {
    let root = temp_root("unreadable-record");
    build_fixture_project(&root);
    assert_eq!(
        run(cli(
            &root,
            create_command("VO-APPROVAL-DOUBLE", ApprovedStateArg::Approved, Vec::new()),
        )),
        ExitCode::Ok
    );
    fs::create_dir(
        root.join(".verify")
            .join("approvals")
            .join("01ZZZZZZZZZZZZZZZZZZZZZZZZ.yaml"),
    )
    .expect("create unreadable record path");
    let layout = vtest_store::VerifyLayout::new(&root);
    assert!(matches!(
        vtest_cli::ops::approval::show(&layout, "vo", "VO-APPROVAL-DOUBLE"),
        Err(vtest_cli::ops::approval::ApprovalOpError::Store(_))
    ));
    assert_ne!(
        run(cli(&root, show_command("VO-APPROVAL-DOUBLE"))),
        ExitCode::Ok
    );
    assert_ne!(
        run(cli(
            &root,
            create_command("VO-APPROVAL-DOUBLE", ApprovedStateArg::Approved, Vec::new()),
        )),
        ExitCode::Ok
    );
    let written_files = approval_paths(&root)
        .into_iter()
        .filter(|path| path.is_file())
        .count();
    assert_eq!(
        written_files, 1,
        "create must not append to an unreadable set"
    );
}
