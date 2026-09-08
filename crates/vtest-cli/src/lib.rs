//! Minimal canonical v0.1 CLI: `init`, `scan`, `verify`, `doctor`.
//!
//! この slice の CLI は正典 v0.1 モデルの上でだけ動く。前身モデルの
//! サブコマンド（`spec` / `req` / `audit` / `vo` / `test` / `run` /
//! `approval` / `mcp` / `report`）はこの版から外してある — それらは
//! `CheckValue` / `CheckItem` / `audits/` を前提としており、正典モデル上で
//! 意味を持たない（SPEC-400「旧モデルの12項目…は検査として存在しない」）。
//! 撤去そのものは移行チェーンの最終段（旧系撤去）の仕事であり、ここでは
//! `verify` を動かすために必要な範囲だけを先行して落としている。

pub mod ops;

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use vtest_model::{Diagnostic, ExitCode, JsonEnvelope};
use vtest_scan::scan_project;
use vtest_store::{load_config, VerifyLayout};
use vtest_verify::{check_name, CheckOutcome, TreeNode};

#[derive(Parser, Debug)]
#[command(name = "vtest", version, about = "Specification traceability verifier")]
pub struct Cli {
    /// Project root, or a path below a project containing `.verify/`.
    #[arg(long, global = true, default_value = ".")]
    pub project: PathBuf,

    /// Emit machine-readable JSON instead of human-readable text.
    #[arg(long, global = true, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,

    /// Suppress all output; communicate through the exit code only.
    #[arg(long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Create the `.verify/` canonical directory layout.
    Init {
        #[arg(long)]
        name: Option<String>,
    },
    /// Scan the repository and report consistency diagnostics.
    Scan,
    /// Validate configuration and adapter preconditions.
    Doctor,
    /// Execute one or more Tests and record Evidence (DS-1101-1103).
    Run {
        /// DS-744 target axis 1/3: explicit Test ids.
        #[arg(long = "test", value_name = "TEST_ID")]
        test: Vec<String>,
        /// DS-744 target axis 2/3: a VO subtree (parent-chain descendants),
        /// selecting every Test whose `covers` intersects it.
        #[arg(long = "vo", value_name = "VO_ID", conflicts_with = "test")]
        vo: Option<String>,
        /// DS-744 target axis 3/3: every Test the scan materialized.
        /// Also the default when neither `--test` nor `--vo` is given (kept
        /// for the CLI's pre-DS-744 default-target behavior).
        #[arg(long, conflicts_with_all = ["test", "vo"])]
        all: bool,
        /// DS-1102/DS-1103: cargo test only; `target_coverage` is recorded
        /// `checked: false` and `target_binding` takes no dynamic evidence.
        #[arg(long)]
        fast: bool,
    },
    /// Aggregate the four canonical checks and return OK / NG.
    ///
    /// SPEC-398「`vtest verify` は集約を実行し、`OK` / `NG` を返す」。
    Verify {
        /// Check axis: a subset of the fixed four checks (DS-1104).
        /// 省略時は固定4検査（DS-1106）。
        #[arg(long, value_delimiter = ',')]
        items: Vec<String>,

        /// Entity axis: a DOC / VO / Test subtree (DS-1104). At most one.
        #[arg(long)]
        doc: Option<String>,
        #[arg(long)]
        vo: Option<String>,
        #[arg(long)]
        test: Option<String>,

        /// Phase-gate evaluation (DS-1115).
        #[arg(long)]
        gate: Option<String>,

        /// DS-1117「`--summary` は総合 `OK` / `NG` と非 `PASS` 件数のみを
        /// 出力する」。
        #[arg(long)]
        summary: bool,
    },
    /// Approval domain: the sole canonical entry point for承認レコード
    /// (BD-304/305/306, 本冊 §3.5, DS-1050-1062/DS-1461-1490).
    #[command(subcommand)]
    Approval(ApprovalCommand),
    /// Document registry (本冊 §12.2, DS-1015-1017/1681-1684, DES-595,
    /// BD-072/331). See `vtest_model::doc_registry`'s module doc comment
    /// for how this points at, without replacing, the fine node-tree
    /// content already at `.verify/doc/<name>.json`.
    #[command(subcommand)]
    Doc(DocCommand),
}

#[derive(Subcommand, Debug)]
pub enum DocCommand {
    /// DS-1681/1683/1684, DES-595. `--path` names an already-built
    /// `.verify/doc/<name>.json` node-tree file (1 document = 1 JSON
    /// file). `--derives-from <ID>` is a repeatable bare upstream document
    /// id (DS-1681: no per-link anchor/note — that shape belongs to the VO
    /// record's own `derives_from`, not this one).
    Add {
        #[arg(long)]
        id: String,
        #[arg(long)]
        path: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long = "derives-from")]
        derives_from: Vec<String>,
        #[arg(long, conflicts_with = "no_root")]
        root: bool,
        #[arg(long = "no-root", conflicts_with = "root")]
        no_root: bool,
        #[arg(long)]
        update: bool,
    },
    /// DS-1015/1016: `--tree` renders the `derives_from` chain as a tree;
    /// `--roots` lists the current root set.
    List {
        #[arg(long)]
        tree: bool,
        #[arg(long)]
        roots: bool,
    },
    /// DS-1017/1682.
    Show { id: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum SubjectTypeArg {
    Vo,
    Document,
    Judgment,
}

impl SubjectTypeArg {
    fn as_str(self) -> &'static str {
        match self {
            SubjectTypeArg::Vo => "vo",
            SubjectTypeArg::Document => "document",
            SubjectTypeArg::Judgment => "judgment",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum ApprovedStateArg {
    Approved,
    Rejected,
    Withdrawn,
}

impl ApprovedStateArg {
    fn as_str(self) -> &'static str {
        match self {
            ApprovedStateArg::Approved => "approved",
            ApprovedStateArg::Rejected => "rejected",
            ApprovedStateArg::Withdrawn => "withdrawn",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum ApproverKindArg {
    Human,
    Agent,
}

impl ApproverKindArg {
    fn as_str(self) -> &'static str {
        match self {
            ApproverKindArg::Human => "human",
            ApproverKindArg::Agent => "agent",
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum ApprovalCommand {
    /// DS-1050/1051/1052: the sole way承認レコード are created.
    /// `vtest vo approve` (not yet ported to this canonical CLI in this
    /// slice) is documented as an alias of this command (BD-074, DS-1045);
    /// it is not a second, independent implementation.
    Create {
        #[arg(long = "subject-type", value_enum)]
        subject_type: SubjectTypeArg,
        #[arg(long = "subject-id")]
        subject_id: String,
        #[arg(long = "state", value_enum)]
        state: ApprovedStateArg,
        #[arg(long = "approver-kind", value_enum)]
        approver_kind: ApproverKindArg,
        #[arg(long = "approver-id")]
        approver_id: String,
        #[arg(long)]
        model: Option<String>,
        #[arg(long = "basis")]
        basis: Vec<String>,
        #[arg(long = "supersedes")]
        supersedes: Vec<String>,
    },
    /// BD-307/DS-1056: writes `state: withdrawn` + `supersedes:
    /// [approval-id]`, copying the target record's subject fields.
    Withdraw {
        approval_id: String,
        #[arg(long = "approver-kind", value_enum)]
        approver_kind: ApproverKindArg,
        #[arg(long = "approver-id")]
        approver_id: String,
        #[arg(long)]
        model: Option<String>,
        #[arg(long = "basis")]
        basis: Vec<String>,
    },
    /// DS-1057/BD-308: the subject's full承認レコード history plus its
    /// current effective承認 state (`draft` / `approved`).
    Show {
        #[arg(long = "subject-type", value_enum)]
        subject_type: SubjectTypeArg,
        #[arg(long = "subject-id")]
        subject_id: String,
    },
}

pub fn run(cli: Cli) -> ExitCode {
    match cli.command {
        Command::Init { name } => run_init(&cli.project, name.as_deref(), cli.format, cli.quiet),
        Command::Scan => run_scan(&cli.project, cli.format, cli.quiet),
        Command::Doctor => run_doctor(&cli.project, cli.format, cli.quiet),
        Command::Run {
            test,
            vo,
            all,
            fast,
        } => run_run(&cli.project, test, vo, all, fast, cli.format, cli.quiet),
        Command::Approval(command) => run_approval(&cli.project, command, cli.format, cli.quiet),
        Command::Doc(command) => run_doc(&cli.project, command, cli.format, cli.quiet),
        Command::Verify {
            items,
            doc,
            vo,
            test,
            gate,
            summary,
        } => run_verify(
            &cli.project,
            &items,
            doc,
            vo,
            test,
            gate.as_deref(),
            summary,
            cli.format,
            cli.quiet,
        ),
    }
}

// ---------------------------------------------------------------------------
// init / scan / doctor
// ---------------------------------------------------------------------------

fn run_init(project: &Path, name: Option<&str>, format: OutputFormat, quiet: bool) -> ExitCode {
    let root = absolute_path(project);
    let project_name = name
        .map(str::to_owned)
        .or_else(|| {
            root.file_name()
                .and_then(|value| value.to_str())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "project".to_owned());
    let (exit, envelope) = ops::init::execute(&root, &project_name);
    emit_value(format, quiet, &envelope, |_| {
        format!("initialised {}\n", root.display())
    });
    exit
}

fn run_scan(project: &Path, format: OutputFormat, quiet: bool) -> ExitCode {
    let root = match resolve_root(project, format, quiet) {
        Ok(root) => root,
        Err(code) => return code,
    };
    let (exit, envelope) = ops::scan::execute(&root);
    emit_value(format, quiet, &envelope, |envelope| {
        let tests = envelope
            .get("data")
            .and_then(|data| data.get("tests"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default();
        let sources = envelope
            .get("data")
            .and_then(|data| data.get("sources"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default();
        let diagnostics = envelope
            .get("diagnostics")
            .and_then(serde_json::Value::as_array)
            .map_or(0, Vec::len);
        format!("scan: {tests} test(s), {sources} source(s), {diagnostics} diagnostic(s)\n")
    });
    exit
}

fn run_doctor(project: &Path, format: OutputFormat, quiet: bool) -> ExitCode {
    let root = match resolve_root(project, format, quiet) {
        Ok(root) => root,
        Err(code) => return code,
    };
    let (exit, envelope) = ops::doctor::execute(&root);
    emit_value(format, quiet, &envelope, |envelope| {
        let project = envelope
            .get("data")
            .and_then(|data| data.get("project"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let adapters = envelope
            .get("data")
            .and_then(|data| data.get("adapters"))
            .and_then(serde_json::Value::as_array)
            .map_or(0, Vec::len);
        let diagnostics = envelope
            .get("diagnostics")
            .and_then(serde_json::Value::as_array)
            .map_or(0, Vec::len);
        format!("doctor: project {project}, {adapters} adapter(s), {diagnostics} diagnostic(s)\n")
    });
    exit
}

// ---------------------------------------------------------------------------
// run
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn run_run(
    project: &Path,
    test_ids: Vec<String>,
    vo: Option<String>,
    all: bool,
    fast: bool,
    format: OutputFormat,
    quiet: bool,
) -> ExitCode {
    let root = match resolve_root(project, format, quiet) {
        Ok(root) => root,
        Err(code) => return code,
    };
    if let Err(error) = load_config(&root) {
        return usage_failure(format, quiet, "E-CONFIG-001", &error.to_string());
    }
    let scan = match scan_project(&root) {
        Ok(scan) => scan,
        Err(error) => return scan_error_exit(&error, format, quiet),
    };
    let layout = VerifyLayout::new(&root);
    let target = if let Some(vo_id) = vo {
        ops::run::RunTarget::Vo(vo_id)
    } else if all {
        ops::run::RunTarget::All
    } else {
        ops::run::RunTarget::Test(test_ids)
    };
    match ops::run::run(&root, &layout, &scan, &target, fast) {
        Ok(result) => {
            let has_errors = result.has_errors();
            let data = serde_json::json!({
                "evidence": result.evidence.len(),
                "evidence_ids": result.evidence.iter().map(|record| record.id.clone()).collect::<Vec<_>>(),
                "fast": fast,
            });
            let envelope = JsonEnvelope::new(!has_errors, data, result.diagnostics.clone());
            emit(format, quiet, &envelope, |envelope| {
                format!(
                    "run: {} evidence record(s) written, {} diagnostic(s)\n",
                    result.evidence.len(),
                    envelope.diagnostics.len()
                )
            });
            if has_errors {
                ExitCode::VerificationFailed
            } else {
                ExitCode::Ok
            }
        }
        Err(
            error @ ops::run::RunOpError::UnknownTestId(_)
            | error @ ops::run::RunOpError::UnknownVoId(_),
        ) => usage_failure(format, quiet, "E-OP-001", &error.to_string()),
        Err(error @ ops::run::RunOpError::Execution(_)) => {
            emit_failure(format, quiet, "E-CORE-001", &error.to_string());
            ExitCode::Internal
        }
        Err(error @ ops::run::RunOpError::Store(_)) => {
            emit_failure(format, quiet, "E-CORE-001", &error.to_string());
            ExitCode::Internal
        }
    }
}

fn scan_error_exit(error: &vtest_scan::ScanError, format: OutputFormat, quiet: bool) -> ExitCode {
    let code = error.code().unwrap_or("E-CORE-001");
    let exit = if error.code().is_some() {
        ExitCode::Usage
    } else {
        ExitCode::Internal
    };
    emit_failure(format, quiet, code, &error.to_string());
    exit
}

// ---------------------------------------------------------------------------
// approval
// ---------------------------------------------------------------------------

fn run_approval(
    project: &Path,
    command: ApprovalCommand,
    format: OutputFormat,
    quiet: bool,
) -> ExitCode {
    let root = match resolve_root(project, format, quiet) {
        Ok(root) => root,
        Err(code) => return code,
    };
    let layout = vtest_store::VerifyLayout::new(&root);

    match command {
        ApprovalCommand::Create {
            subject_type,
            subject_id,
            state,
            approver_kind,
            approver_id,
            model,
            basis,
            supersedes,
        } => {
            let result = ops::approval::create(
                &layout,
                ops::approval::CreateArgs {
                    subject_type: subject_type.as_str().to_owned(),
                    subject_id,
                    approved_state: state.as_str().to_owned(),
                    approver_kind: approver_kind.as_str().to_owned(),
                    approver_id,
                    approver_model: model,
                    basis,
                    supersedes,
                },
            );
            approval_result(result, format, quiet)
        }
        ApprovalCommand::Withdraw {
            approval_id,
            approver_kind,
            approver_id,
            model,
            basis,
        } => {
            let result = ops::approval::withdraw(
                &layout,
                ops::approval::WithdrawArgs {
                    approval_id,
                    approver_kind: approver_kind.as_str().to_owned(),
                    approver_id,
                    approver_model: model,
                    basis,
                },
            );
            approval_result(result, format, quiet)
        }
        ApprovalCommand::Show {
            subject_type,
            subject_id,
        } => match ops::approval::show(&layout, subject_type.as_str(), &subject_id) {
            Ok(result) => {
                let effective = match result.effective_state {
                    vtest_store::approval::EffectiveApprovalState::Draft => "draft",
                    vtest_store::approval::EffectiveApprovalState::Approved => "approved",
                };
                let data = serde_json::json!({
                    "records": result.records.iter().map(|record| serde_json::json!({
                        "id": record.id,
                        "subject_type": record.subject_type,
                        "subject": record.subject,
                        "approved_state": record.approved_state,
                        "supersedes": record.supersedes,
                        "approved_at": record.approved_at,
                    })).collect::<Vec<_>>(),
                    "effective_state": effective,
                });
                let envelope = JsonEnvelope::new(true, data, Vec::new());
                emit(format, quiet, &envelope, |envelope| {
                    format!(
                        "approval show: {} record(s), effective state {}\n",
                        envelope.data["records"].as_array().map_or(0, Vec::len),
                        effective
                    )
                });
                ExitCode::Ok
            }
            Err(error) => approval_error_exit(&error, format, quiet),
        },
    }
}

fn approval_result(
    result: Result<vtest_store::records::ApprovalRecord, ops::approval::ApprovalOpError>,
    format: OutputFormat,
    quiet: bool,
) -> ExitCode {
    match result {
        Ok(record) => {
            let data = serde_json::json!({
                "id": record.id,
                "subject_type": record.subject_type,
                "subject": record.subject,
                "approved_state": record.approved_state,
                "supersedes": record.supersedes,
                "approved_at": record.approved_at,
            });
            let envelope = JsonEnvelope::new(true, data, Vec::new());
            emit(format, quiet, &envelope, |envelope| {
                format!(
                    "approval {}: {}\n",
                    envelope.data["approved_state"].as_str().unwrap_or(""),
                    envelope.data["id"].as_str().unwrap_or(""),
                )
            });
            ExitCode::Ok
        }
        Err(error) => approval_error_exit(&error, format, quiet),
    }
}

fn approval_error_exit(
    error: &ops::approval::ApprovalOpError,
    format: OutputFormat,
    quiet: bool,
) -> ExitCode {
    use ops::approval::ApprovalOpError;
    match error {
        ApprovalOpError::JudgmentSubjectTypeUnsupported => {
            usage_failure(format, quiet, "E-OP-001", &error.to_string())
        }
        ApprovalOpError::UnresolvedSubject(_) => {
            usage_failure(format, quiet, "E-APPROVAL-001", &error.to_string())
        }
        ApprovalOpError::InvalidRequest(_) => {
            usage_failure(format, quiet, "E-APPROVAL-002", &error.to_string())
        }
        ApprovalOpError::Store(_) => {
            emit_failure(format, quiet, "E-CORE-001", &error.to_string());
            ExitCode::Internal
        }
    }
}

// ---------------------------------------------------------------------------
// doc
// ---------------------------------------------------------------------------

fn run_doc(project: &Path, command: DocCommand, format: OutputFormat, quiet: bool) -> ExitCode {
    let root = match resolve_root(project, format, quiet) {
        Ok(root) => root,
        Err(code) => return code,
    };
    let layout = vtest_store::VerifyLayout::new(&root);

    match command {
        DocCommand::Add {
            id,
            path,
            title,
            derives_from,
            root: root_flag,
            no_root,
            update,
        } => {
            let root_arg = if root_flag {
                Some(true)
            } else if no_root {
                Some(false)
            } else {
                None
            };
            match ops::doc::add(
                &root,
                &layout,
                ops::doc::AddArgs {
                    id,
                    path,
                    title,
                    derives_from,
                    root: root_arg,
                    update,
                },
            ) {
                Ok(record) => {
                    let data = doc_record_json(&record);
                    let envelope = JsonEnvelope::new(true, data, Vec::new());
                    emit(format, quiet, &envelope, |envelope| {
                        format!("doc add: {}\n", envelope.data["id"].as_str().unwrap_or(""))
                    });
                    ExitCode::Ok
                }
                Err(error) => doc_error_exit(&error, format, quiet),
            }
        }
        DocCommand::List { tree, roots } => match ops::doc::list(&layout) {
            Ok(result) => {
                let records: Vec<_> = result.records.iter().map(doc_record_json).collect();
                let data = serde_json::json!({
                    "records": records,
                    "roots": result.records.iter().filter(|r| r.root).map(|r| r.id.clone()).collect::<Vec<_>>(),
                    "unresolved_derives_from": result.unresolved,
                });
                let envelope = JsonEnvelope::new(true, data, Vec::new());
                emit(format, quiet, &envelope, |envelope| {
                    render_doc_list_text(envelope, tree, roots)
                });
                ExitCode::Ok
            }
            Err(error) => doc_error_exit(&error, format, quiet),
        },
        DocCommand::Show { id } => match ops::doc::show(&root, &layout, &id) {
            Ok(result) => {
                let mut data = doc_record_json(&result.record);
                match &result.fresh {
                    Ok(fresh) => data["fresh"] = serde_json::json!(fresh),
                    Err(message) => data["fresh_error"] = serde_json::json!(message),
                }
                let envelope = JsonEnvelope::new(true, data, Vec::new());
                emit(format, quiet, &envelope, render_doc_show_text);
                ExitCode::Ok
            }
            Err(error) => doc_error_exit(&error, format, quiet),
        },
    }
}

fn doc_record_json(record: &vtest_model::DocRegistryRecord) -> serde_json::Value {
    serde_json::json!({
        "id": record.id,
        "path": record.path,
        "title": record.title,
        "content_hash": record.content_hash.as_str(),
        "derives_from": record.derives_from,
        "root": record.root,
        "registered_at": record.registered_at,
    })
}

fn render_doc_list_text(
    envelope: &JsonEnvelope<serde_json::Value>,
    tree: bool,
    roots_only: bool,
) -> String {
    let data = &envelope.data;
    let mut out = String::new();
    if roots_only {
        out.push_str("Roots:\n");
        for root in data["roots"].as_array().into_iter().flatten() {
            out.push_str(&format!("  {}\n", root.as_str().unwrap_or("")));
        }
        return out;
    }
    let records = data["records"].as_array().cloned().unwrap_or_default();
    if tree {
        out.push_str("Document tree (derives_from):\n");
        for record in &records {
            let id = record["id"].as_str().unwrap_or("");
            let derives_from = record["derives_from"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|entry| entry.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("  {id} -> [{derives_from}]\n"));
        }
    } else {
        for record in &records {
            out.push_str(&format!(
                "{}\t{}\n",
                record["id"].as_str().unwrap_or(""),
                record["path"].as_str().unwrap_or("")
            ));
        }
    }
    let unresolved = data["unresolved_derives_from"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if !unresolved.is_empty() {
        out.push_str("\nUnresolved derives_from (E-SCAN-012 equivalent, registry level):\n");
        for entry in &unresolved {
            if let Some(pair) = entry.as_array() {
                out.push_str(&format!(
                    "  {} -> {}\n",
                    pair.first().and_then(|v| v.as_str()).unwrap_or(""),
                    pair.get(1).and_then(|v| v.as_str()).unwrap_or("")
                ));
            }
        }
    }
    out
}

fn render_doc_show_text(envelope: &JsonEnvelope<serde_json::Value>) -> String {
    let data = &envelope.data;
    let mut out = format!(
        "id: {}\npath: {}\ncontent_hash: {}\nroot: {}\n",
        data["id"].as_str().unwrap_or(""),
        data["path"].as_str().unwrap_or(""),
        data["content_hash"].as_str().unwrap_or(""),
        data["root"].as_bool().unwrap_or(false),
    );
    if let Some(fresh) = data.get("fresh").and_then(serde_json::Value::as_bool) {
        out.push_str(&format!("fresh: {fresh}\n"));
    }
    if let Some(error) = data.get("fresh_error").and_then(serde_json::Value::as_str) {
        out.push_str(&format!("fresh: unknown ({error})\n"));
    }
    out.push_str("derives_from:\n");
    for entry in data["derives_from"].as_array().into_iter().flatten() {
        out.push_str(&format!("  {}\n", entry.as_str().unwrap_or("")));
    }
    out
}

fn doc_error_exit(error: &ops::doc::DocOpError, format: OutputFormat, quiet: bool) -> ExitCode {
    match error {
        ops::doc::DocOpError::Usage(_) => {
            usage_failure(format, quiet, "E-OP-001", &error.to_string())
        }
        ops::doc::DocOpError::Store(_) => {
            emit_failure(format, quiet, "E-CORE-001", &error.to_string());
            ExitCode::Internal
        }
    }
}

// ---------------------------------------------------------------------------
// Shared envelope construction — `ops::*` modules build the same
// `JsonEnvelope` shape the CLI's own `emit`/`emit_failure` produce, so a
// caller (CLI text renderer or `vtest-mcp`'s tool handler) sees identical
// `ok` / `data` / `diagnostics` for identical input (DS-1563).
// ---------------------------------------------------------------------------

/// Builds the same JSON shape `JsonEnvelope::new` serializes to, without
/// requiring the caller to hold a typed `Diagnostic` slice reference.
pub(crate) fn envelope_json<T: serde::Serialize>(
    ok: bool,
    data: T,
    diagnostics: &[Diagnostic],
) -> serde_json::Value {
    serde_json::to_value(JsonEnvelope::new(ok, data, diagnostics.to_vec()))
        .unwrap_or_else(|error| serde_json::json!({"ok": false, "data": null, "diagnostics": [{"code": "E-CORE-001", "severity": "error", "message": error.to_string()}]}))
}

/// The `E-CONFIG-001` config-load failure envelope shared by `init` /
/// `scan` / `doctor`.
pub(crate) fn config_failure_envelope(message: &str) -> serde_json::Value {
    envelope_json(
        false,
        serde_json::Value::Null,
        &[Diagnostic::error("E-CONFIG-001", message.to_owned())],
    )
}

/// DS-935「`vtest scan` / `vtest doctor`では、registry・config・adapter契約の
/// 検証またはadapter呼出しがE-ADAPTER-* / E-CONFIG-*で拒否された場合は2とする」。
pub(crate) fn scan_error_result(error: &vtest_scan::ScanError) -> (ExitCode, serde_json::Value) {
    let code = error.code().unwrap_or("E-CORE-001");
    let exit = if error.code().is_some() {
        ExitCode::Usage
    } else {
        ExitCode::Internal
    };
    let envelope = envelope_json(
        false,
        serde_json::Value::Null,
        &[Diagnostic::error(code, error.to_string())],
    );
    (exit, envelope)
}

/// Prints a `serde_json::Value` envelope the way [`emit`] prints a typed
/// [`JsonEnvelope`] — used by wrappers whose operation body now lives in
/// `ops::*` and returns pre-built JSON rather than a typed struct.
fn emit_value(
    format: OutputFormat,
    quiet: bool,
    envelope: &serde_json::Value,
    render_text: impl FnOnce(&serde_json::Value) -> String,
) {
    if quiet {
        return;
    }
    match format {
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(envelope).unwrap_or_else(|error| format!(
                "{{\"ok\":false,\"data\":null,\"diagnostics\":[\"{error}\"]}}"
            ))
        ),
        OutputFormat::Text => {
            print!("{}", render_text(envelope));
            if let Some(diagnostics) = envelope
                .get("diagnostics")
                .and_then(serde_json::Value::as_array)
            {
                for diagnostic in diagnostics {
                    let code = diagnostic
                        .get("code")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("");
                    let message = diagnostic
                        .get("message")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("");
                    println!("[{code}] {message}");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// verify
// ---------------------------------------------------------------------------

use ops::verify::{state_name, VerifyData};

#[allow(clippy::too_many_arguments)]
fn run_verify(
    project: &Path,
    items: &[String],
    doc: Option<String>,
    vo: Option<String>,
    test: Option<String>,
    gate: Option<&str>,
    summary: bool,
    format: OutputFormat,
    quiet: bool,
) -> ExitCode {
    let root = match resolve_root(project, format, quiet) {
        Ok(root) => root,
        Err(code) => return code,
    };

    match ops::verify::execute(&root, items, doc, vo, test, gate, summary) {
        Ok((exit, data, diagnostics)) => {
            // `ok` はこの実行が返す 0/1 と同義にする — ゲート指定時はゲート
            // 充足、非指定時は総合 OK。検証状態そのものは `state` field に
            // 常に別途出す。
            let envelope = JsonEnvelope::new(exit == ExitCode::Ok, data, diagnostics);
            emit(format, quiet, &envelope, render_verify_text);
            exit
        }
        Err(ops::verify::VerifyOpError::Usage { code, message }) => {
            usage_failure(format, quiet, code, &message)
        }
        Err(ops::verify::VerifyOpError::Scan(error)) => scan_error_exit(&error, format, quiet),
    }
}

fn label_name(label: vtest_model::DiagnosticLabel) -> &'static str {
    match label {
        vtest_model::DiagnosticLabel::Missing => "MISSING",
        vtest_model::DiagnosticLabel::NotChecked => "NOT_CHECKED",
        vtest_model::DiagnosticLabel::NotExecuted => "NOT_EXECUTED",
        vtest_model::DiagnosticLabel::Stale => "STALE",
    }
}

/// DS-1118「`vtest verify` は状態列…と診断ラベル列…を分離して表示する」。
fn render_verify_text(envelope: &JsonEnvelope<VerifyData>) -> String {
    let data = &envelope.data;
    let mut out = String::new();

    // DS-1113「scope を限定した場合、出力冒頭に要求 scope と「scope 外は
    // 未検証」の旨を必ず表示する」。
    out.push_str(&format!(
        "Requested scope: {}\n",
        data.scope
            .requested_checks
            .iter()
            .map(|check| check_name(*check))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    if let Some(entity) = &data.scope.entity {
        out.push_str(&format!("Entity scope: {}\n", entity.id()));
    }
    if data.scope.outside_scope_is_unverified {
        out.push_str("Anything outside the requested scope is UNVERIFIED, not PASS.\n");
    }

    if !data.structural.is_empty() {
        out.push_str("\nStructural checks:\n");
        for check in &data.structural {
            out.push_str(&render_check(check, 2));
        }
    }

    if !data.unevaluated.is_empty() {
        out.push_str("\nChecks with no evaluation point (NOT verified, not PASS):\n");
        for check in &data.unevaluated {
            out.push_str(&render_check(check, 2));
        }
    }

    if !data.tree.is_empty() {
        out.push('\n');
        for node in &data.tree {
            render_node(node, 0, &mut out);
        }
    }

    if let Some(gate) = &data.gate {
        out.push_str(&format!(
            "\nGate {}: {} (verification {}, approvals {})\n",
            gate.name,
            if gate.satisfied {
                "SATISFIED"
            } else {
                "NOT SATISFIED"
            },
            if gate.verification_satisfied {
                "ok"
            } else {
                "no"
            },
            if gate.approvals_satisfied { "ok" } else { "no" },
        ));
        for reason in &gate.reasons {
            out.push_str(&format!("  - {reason}\n"));
        }
    }

    out.push_str(&format!(
        "\nAggregate state: {}\nNon-PASS checks: {}\nResult: {}\n",
        data.state, data.non_pass, data.result
    ));
    out
}

fn render_node(node: &TreeNode, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    out.push_str(&format!(
        "{pad}{:?} {} [{}]\n",
        node.kind,
        node.id,
        state_name(node.state)
    ));
    for check in &node.checks {
        out.push_str(&render_check(check, (depth + 1) * 2));
    }
    for child in &node.children {
        render_node(child, depth + 1, out);
    }
}

/// 状態列と診断ラベル列を分けて描画する（DS-1118）。診断ラベルは代表値の
/// 順位に用いず併記するだけである（DS-1119）。
fn render_check(check: &CheckOutcome, indent: usize) -> String {
    let pad = " ".repeat(indent);
    let labels = if check.labels.is_empty() {
        String::new()
    } else {
        format!(
            "  {}",
            check
                .labels
                .iter()
                .map(|label| format!("[{}]", label_name(*label)))
                .collect::<Vec<_>>()
                .join(" ")
        )
    };
    let mut line = format!(
        "{pad}{:<18} {:<12}{labels}\n",
        check_name(check.check),
        state_name(check.state)
    );
    for basis in &check.basis {
        line.push_str(&format!("{pad}    {basis}\n"));
    }
    line
}

// ---------------------------------------------------------------------------
// Shared plumbing
// ---------------------------------------------------------------------------

fn emit<T: serde::Serialize>(
    format: OutputFormat,
    quiet: bool,
    envelope: &JsonEnvelope<T>,
    render_text: impl FnOnce(&JsonEnvelope<T>) -> String,
) {
    if quiet {
        return;
    }
    match format {
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(envelope).unwrap_or_else(|error| format!(
                "{{\"ok\":false,\"data\":null,\"diagnostics\":[\"{error}\"]}}"
            ))
        ),
        OutputFormat::Text => {
            print!("{}", render_text(envelope));
            for diagnostic in &envelope.diagnostics {
                println!("[{}] {}", diagnostic.code, diagnostic.message);
            }
        }
    }
}

fn emit_failure(format: OutputFormat, quiet: bool, code: &str, message: &str) {
    let envelope = JsonEnvelope::new(
        false,
        serde_json::Value::Null,
        vec![Diagnostic::error(code, message.to_owned())],
    );
    emit(format, quiet, &envelope, |_| String::new());
}

fn usage_failure(format: OutputFormat, quiet: bool, code: &str, message: &str) -> ExitCode {
    emit_failure(format, quiet, code, message);
    ExitCode::Usage
}

fn absolute_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_owned())
    }
}

/// Walk upwards looking for a `.verify/` directory, so the CLI works from a
/// subdirectory of the project.
fn resolve_root(project: &Path, format: OutputFormat, quiet: bool) -> Result<PathBuf, ExitCode> {
    let start = absolute_path(project);
    let mut current = start.as_path();
    loop {
        if current.join(".verify").is_dir() {
            return Ok(current.to_owned());
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => {
                return Err(usage_failure(
                    format,
                    quiet,
                    "E-OP-001",
                    &format!(
                        "could not find a .verify directory at or above {}",
                        start.display()
                    ),
                ));
            }
        }
    }
}

// `parse_items` / `entity_scope` argument-mapping tests now live with their
// implementation in `ops::verify` (moved there so both the CLI and
// `vtest-mcp` share one operation body — see DS-1563).
