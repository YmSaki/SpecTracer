//! CLI application layer shared by the future MCP adapter.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
    process::Command as ProcessCommand,
};

use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use vtest_audit::{audit_static, persist_static_audits, AuditOptions, AuditVerdict};
use vtest_exec::{run_tests, RunnableTest};
use vtest_model::{
    ContentHash, Diagnostic, ExitCode, JsonEnvelope, Locator, ReqId, Revision, ScanSummary,
    SourceFunction, SpecId, TargetRef, TestEntity, VoId,
};
use vtest_scan::{
    create_test, edit_test, list_tests, parse_test_set_values, query_tests, scan_project,
    show_test, ScanResult,
};
use vtest_store::{
    find_project_root, init_project, load_config, new_record_id, now_rfc3339, read_approval,
    read_form_answers, read_req, read_spec, read_text, read_vo, write_atomic, yaml_scalar_value,
    ApprovalRecord, Approver, Dimension, ReqRecord, SpecRecord, SpecRef, StoreError, VerifyLayout,
    VoRecord,
};
use vtest_verify::verify_project;

#[derive(Clone, Debug, Parser)]
#[command(name = "vtest", version, about = "Fail-closed test verification")]
pub struct Cli {
    /// Project root, or a path below a project containing `.verify/`.
    #[arg(long, global = true, default_value = ".")]
    pub project: PathBuf,
    /// Output format.
    #[arg(long, global = true, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
    /// Suppress human-readable output.
    #[arg(long, global = true)]
    pub quiet: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Clone, Debug, Subcommand)]
pub enum Command {
    /// Create the `.verify/` canonical directory layout.
    Init {
        #[arg(long)]
        name: Option<String>,
    },
    /// Scan Rust sources and report diagnostics.
    Scan,
    /// Alias for `scan`, intended for CI checks.
    Doctor,
    /// Manage canonical SPEC records.
    Spec {
        #[command(subcommand)]
        command: SpecCommand,
    },
    /// Manage canonical REQ records.
    Req {
        #[command(subcommand)]
        command: ReqCommand,
    },
    /// Manage canonical VO records and approvals.
    Vo {
        #[command(subcommand)]
        command: VoCommand,
    },
    /// Inspect registered and unregistered tests.
    Test {
        #[command(subcommand)]
        command: TestCommand,
    },
    /// Run deterministic static audit rules.
    Audit {
        #[command(subcommand)]
        command: AuditCommand,
    },
    /// Execute selected tests and record Evidence.
    Run {
        #[arg(long)]
        test: Option<String>,
        #[arg(long)]
        vo: Option<String>,
        #[arg(long)]
        req: Option<String>,
        #[arg(long)]
        all: bool,
        #[arg(long)]
        fast: bool,
    },
    /// Evaluate the requested fail-closed verification scope.
    Verify {
        #[arg(long)]
        items: Option<String>,
        #[arg(long)]
        req: Option<String>,
        #[arg(long)]
        vo: Option<String>,
        #[arg(long)]
        test: Option<String>,
        #[arg(long)]
        summary: bool,
    },
    /// Produce the same aggregate with detailed basis references.
    Report {
        #[arg(long)]
        items: Option<String>,
        #[arg(long)]
        req: Option<String>,
        #[arg(long)]
        vo: Option<String>,
        #[arg(long)]
        test: Option<String>,
    },
}

#[derive(Clone, Debug, Subcommand)]
pub enum AuditCommand {
    Static {
        #[arg(long)]
        test: Option<String>,
    },
    Bundle {
        #[arg(long)]
        kind: String,
        #[arg(long)]
        test: Option<String>,
        #[arg(long)]
        vo: Option<String>,
        #[arg(long)]
        req: Option<String>,
        #[arg(long)]
        include_failed: bool,
    },
    Submit {
        #[arg(long)]
        file: PathBuf,
    },
}

#[derive(Clone, Debug, Subcommand)]
pub enum SpecCommand {
    Add {
        #[arg(long)]
        id: String,
        #[arg(long)]
        path: PathBuf,
        #[arg(long, default_value = "document")]
        kind: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        update: bool,
    },
    List,
    Show {
        id: String,
    },
}

#[derive(Clone, Debug, Subcommand)]
pub enum ReqCommand {
    Add {
        #[arg(long)]
        id: String,
        #[arg(long)]
        summary: String,
        #[arg(long)]
        parent: Option<String>,
        #[arg(long = "spec")]
        specs: Vec<String>,
        #[arg(long)]
        sections: Vec<String>,
    },
    Edit {
        id: String,
        #[arg(long)]
        summary: Option<String>,
        #[arg(long)]
        parent: Option<String>,
        #[arg(long)]
        status: Option<String>,
    },
    List {
        #[arg(long)]
        tree: bool,
    },
    Show {
        id: String,
    },
}

#[derive(Clone, Debug, Subcommand)]
pub enum VoCommand {
    Add {
        #[arg(long)]
        id: String,
        #[arg(long)]
        claim: String,
        #[arg(long = "req")]
        requirements: Vec<String>,
        #[arg(long)]
        parent: Option<String>,
        #[arg(long = "spec")]
        specs: Vec<String>,
        #[arg(long)]
        sections: Vec<String>,
        #[arg(long = "dimension")]
        dimensions: Vec<String>,
        #[arg(long)]
        policy: Option<String>,
    },
    Edit {
        id: String,
        #[arg(long)]
        claim: Option<String>,
        #[arg(long)]
        parent: Option<String>,
        #[arg(long)]
        status: Option<String>,
    },
    List {
        #[arg(long)]
        tree: bool,
        #[arg(long)]
        req: Option<String>,
        #[arg(long)]
        status: Option<String>,
    },
    Show {
        id: String,
    },
    Expand {
        id: String,
        #[arg(long)]
        dry_run: bool,
    },
    Approve {
        id: String,
        #[arg(long)]
        approver_kind: String,
        #[arg(long)]
        approver_id: String,
        #[arg(long)]
        model: Option<String>,
        #[arg(long = "basis")]
        basis: Vec<String>,
    },
}

#[derive(Clone, Debug, Subcommand)]
pub enum TestCommand {
    Create {
        #[arg(long)]
        form: String,
        #[arg(long)]
        answers: PathBuf,
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    Edit {
        id: String,
        #[arg(long)]
        answers: Option<PathBuf>,
        #[arg(long = "set")]
        set: Vec<String>,
        #[arg(long)]
        body_file: Option<PathBuf>,
        #[arg(long)]
        dry_run: bool,
    },
    Show {
        id: String,
    },
    List {
        #[arg(long)]
        vo: Option<String>,
        #[arg(long)]
        unregistered: bool,
    },
    Query {
        #[arg(long)]
        source: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Clone, Debug, Serialize)]
pub struct ScanData {
    pub summary: ScanSummary,
    pub tests: Vec<vtest_model::TestEntity>,
    pub sources: Vec<vtest_model::SourceFunction>,
}

impl From<ScanResult> for ScanData {
    fn from(value: ScanResult) -> Self {
        Self {
            summary: value.summary,
            tests: value.tests,
            sources: value.sources,
        }
    }
}

pub fn run(cli: Cli) -> ExitCode {
    let project = cli.project;
    let format = cli.format;
    let quiet = cli.quiet;
    match cli.command {
        Command::Init { name } => run_init(&project, name.as_deref(), format, quiet),
        Command::Scan | Command::Doctor => run_scan(&project, format, quiet),
        Command::Spec { command } => run_spec(&project, command, format, quiet),
        Command::Req { command } => run_req(&project, command, format, quiet),
        Command::Vo { command } => run_vo(&project, command, format, quiet),
        Command::Test { command } => run_test(&project, command, format, quiet),
        Command::Audit { command } => run_audit(&project, command, format, quiet),
        Command::Run {
            test,
            vo,
            req,
            all,
            fast,
        } => run_run(&project, test, vo, req, all, fast, format, quiet),
        Command::Verify {
            items,
            req,
            vo,
            test,
            summary,
        } => run_verify(&project, items, req, vo, test, summary, format, quiet),
        Command::Report {
            items,
            req,
            vo,
            test,
        } => run_verify(&project, items, req, vo, test, false, format, quiet),
    }
}

fn run_test(project: &Path, command: TestCommand, format: OutputFormat, quiet: bool) -> ExitCode {
    let root = match resolve_root(project, format, quiet) {
        Ok(root) => root,
        Err(code) => return code,
    };
    let scan = match scan_project(&root) {
        Ok(scan) => scan,
        Err(error) => {
            return finish_record_command(
                Err(failure("E-CORE-001", error.to_string(), ExitCode::Internal)),
                format,
                quiet,
            )
        }
    };
    let result = match command {
        TestCommand::Create {
            form,
            answers,
            id,
            dry_run,
        } => (|| {
            let answers_path = safe_project_path(&root, &answers)?;
            let supplied = read_form_answers(&answers_path)
                .map_err(|error| failure("E-OP-001", error.to_string(), ExitCode::Usage))?;
            create_test(&root, &form, &supplied, id.as_deref(), dry_run)
                .map_err(operation_failure)
                .and_then(|result| {
                    serde_json::to_value(result).map_err(|error| {
                        failure("E-CORE-001", error.to_string(), ExitCode::Internal)
                    })
                })
        })(),
        TestCommand::Edit {
            id,
            answers,
            set,
            body_file,
            dry_run,
        } => (|| {
            let supplied = answers
                .as_ref()
                .map(|answers| {
                    let path = safe_project_path(&root, answers)?;
                    read_form_answers(&path)
                        .map_err(|error| failure("E-OP-001", error.to_string(), ExitCode::Usage))
                })
                .transpose()?;
            let set = parse_test_set_values(&set).map_err(operation_failure)?;
            let body = body_file
                .as_ref()
                .map(|body_file| {
                    let path = safe_project_path(&root, body_file)?;
                    fs::read_to_string(&path)
                        .map_err(|error| failure("E-OP-001", error.to_string(), ExitCode::Usage))
                })
                .transpose()?;
            edit_test(
                &root,
                &id,
                supplied.as_ref(),
                &set,
                body.as_deref(),
                dry_run,
            )
            .map_err(operation_failure)
            .and_then(|result| {
                serde_json::to_value(result)
                    .map_err(|error| failure("E-CORE-001", error.to_string(), ExitCode::Internal))
            })
        })(),
        TestCommand::Show { id } => show_test(&root, &scan, &id)
            .and_then(|test| {
                serde_json::to_value(test)
                    .map_err(|error| Diagnostic::error("E-CORE-001", error.to_string()))
            })
            .map_err(operation_failure),
        TestCommand::List { vo, unregistered } => {
            serde_json::to_value(list_tests(&scan, vo.as_deref(), unregistered))
                .map_err(|error| failure("E-CORE-001", error.to_string(), ExitCode::Internal))
        }
        TestCommand::Query { source } => query_tests(&scan, &source)
            .and_then(|tests| {
                serde_json::to_value(tests)
                    .map_err(|error| Diagnostic::error("E-CORE-001", error.to_string()))
            })
            .map_err(operation_failure),
    };
    finish_record_command(result, format, quiet)
}

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
    match init_project(&root, &project_name) {
        Ok(_) => {
            let data = serde_json::json!({ "project": root, "initialized": true });
            emit(format, quiet, JsonEnvelope::new(true, data, Vec::new()));
            ExitCode::Ok
        }
        Err(error) => {
            let (diagnostic, code) = store_error_with_code(error, ExitCode::Usage);
            emit(
                format,
                quiet,
                JsonEnvelope::new(false, serde_json::Value::Null, vec![diagnostic]),
            );
            code
        }
    }
}

fn run_scan(project: &Path, format: OutputFormat, quiet: bool) -> ExitCode {
    let root = match resolve_root(project, format, quiet) {
        Ok(root) => root,
        Err(code) => return code,
    };
    match scan_project(&root) {
        Ok(result) => {
            let has_errors = result.has_errors();
            let diagnostics = result.diagnostics.clone();
            emit(
                format,
                quiet,
                JsonEnvelope::new(!has_errors, ScanData::from(result), diagnostics),
            );
            if has_errors {
                ExitCode::VerificationFailed
            } else {
                ExitCode::Ok
            }
        }
        Err(error) => {
            let diagnostic = Diagnostic::error("E-CORE-001", error.to_string());
            emit(
                format,
                quiet,
                JsonEnvelope::new(false, serde_json::Value::Null, vec![diagnostic]),
            );
            ExitCode::Internal
        }
    }
}

fn run_spec(project: &Path, command: SpecCommand, format: OutputFormat, quiet: bool) -> ExitCode {
    let root = match resolve_root(project, format, quiet) {
        Ok(root) => root,
        Err(code) => return code,
    };
    let layout = VerifyLayout::new(&root);
    let result = match command {
        SpecCommand::Add {
            id,
            path,
            kind,
            title,
            note,
            update,
        } => add_spec(&root, &layout, &id, &path, &kind, title, note, update),
        SpecCommand::List => list_specs(&layout),
        SpecCommand::Show { id } => show_spec(&layout, &id),
    };
    finish_record_command(result, format, quiet)
}

fn run_req(project: &Path, command: ReqCommand, format: OutputFormat, quiet: bool) -> ExitCode {
    let root = match resolve_root(project, format, quiet) {
        Ok(root) => root,
        Err(code) => return code,
    };
    let layout = VerifyLayout::new(&root);
    let result = match command {
        ReqCommand::Add {
            id,
            summary,
            parent,
            specs,
            sections,
        } => add_req(&layout, &id, &summary, parent, &specs, &sections),
        ReqCommand::Edit {
            id,
            summary,
            parent,
            status,
        } => edit_req(&layout, &id, summary, parent, status),
        ReqCommand::List { tree: _ } => list_reqs(&layout),
        ReqCommand::Show { id } => show_req(&layout, &id),
    };
    finish_record_command(result, format, quiet)
}

fn run_vo(project: &Path, command: VoCommand, format: OutputFormat, quiet: bool) -> ExitCode {
    let root = match resolve_root(project, format, quiet) {
        Ok(root) => root,
        Err(code) => return code,
    };
    let layout = VerifyLayout::new(&root);
    let result = match command {
        VoCommand::Add {
            id,
            claim,
            requirements,
            parent,
            specs,
            sections,
            dimensions,
            policy,
        } => add_vo(
            &layout,
            &id,
            &claim,
            &requirements,
            parent,
            &specs,
            &sections,
            &dimensions,
            policy,
        ),
        VoCommand::Edit {
            id,
            claim,
            parent,
            status,
        } => edit_vo(&layout, &id, claim, parent, status),
        VoCommand::List {
            tree: _,
            req,
            status,
        } => list_vos(&layout, req.as_deref(), status.as_deref()),
        VoCommand::Show { id } => show_vo(&layout, &id),
        VoCommand::Expand { id, dry_run } => expand_vo(&layout, &id, dry_run),
        VoCommand::Approve {
            id,
            approver_kind,
            approver_id,
            model,
            basis,
        } => approve_vo(&layout, &id, &approver_kind, &approver_id, model, &basis),
    };
    finish_record_command(result, format, quiet)
}

fn run_audit(project: &Path, command: AuditCommand, format: OutputFormat, quiet: bool) -> ExitCode {
    match command {
        AuditCommand::Static { test } => run_audit_static(project, test, format, quiet),
        AuditCommand::Bundle {
            kind,
            test,
            vo,
            req,
            include_failed,
        } => run_audit_bundle(
            project,
            &kind,
            test.as_deref(),
            vo.as_deref(),
            req.as_deref(),
            include_failed,
            format,
            quiet,
        ),
        AuditCommand::Submit { file } => run_audit_submit(project, &file, format, quiet),
    }
}

fn run_audit_static(
    project: &Path,
    test: Option<String>,
    format: OutputFormat,
    quiet: bool,
) -> ExitCode {
    let root = match resolve_root(project, format, quiet) {
        Ok(root) => root,
        Err(code) => return code,
    };
    let layout = VerifyLayout::new(&root);
    let scan = match scan_project(&root) {
        Ok(scan) => scan,
        Err(error) => {
            emit(
                format,
                quiet,
                JsonEnvelope::new(
                    false,
                    serde_json::Value::Null,
                    vec![Diagnostic::error("E-CORE-001", error.to_string())],
                ),
            );
            return ExitCode::Internal;
        }
    };
    let scan_has_errors = scan.has_errors();
    let summary = match audit_static(&root, &scan, &AuditOptions { test_id: test }) {
        Ok(summary) => summary,
        Err(error) => {
            emit(
                format,
                quiet,
                JsonEnvelope::new(
                    false,
                    serde_json::Value::Null,
                    vec![Diagnostic::error("E-OP-001", error.to_string())],
                ),
            );
            return ExitCode::Usage;
        }
    };
    if let Err(error) = persist_static_audits(&layout, &summary) {
        emit(
            format,
            quiet,
            JsonEnvelope::new(
                false,
                serde_json::Value::Null,
                vec![Diagnostic::error("E-CORE-001", error.to_string())],
            ),
        );
        return ExitCode::Internal;
    }
    let has_non_pass = scan_has_errors
        || summary
            .audits
            .iter()
            .any(|audit| audit.verdict != AuditVerdict::Pass);
    emit(
        format,
        quiet,
        JsonEnvelope::new(
            !has_non_pass,
            serde_json::to_value(&summary).expect("audit summary"),
            scan.diagnostics,
        ),
    );
    if has_non_pass {
        ExitCode::VerificationFailed
    } else {
        ExitCode::Ok
    }
}

#[allow(clippy::too_many_arguments)]
fn run_audit_bundle(
    project: &Path,
    kind: &str,
    test_id: Option<&str>,
    vo_id: Option<&str>,
    req_id: Option<&str>,
    include_failed: bool,
    format: OutputFormat,
    quiet: bool,
) -> ExitCode {
    let root = match resolve_root(project, format, quiet) {
        Ok(root) => root,
        Err(code) => return code,
    };
    let layout = VerifyLayout::new(&root);
    let scan = match scan_project(&root) {
        Ok(scan) => scan,
        Err(error) => {
            emit(
                format,
                quiet,
                JsonEnvelope::new(
                    false,
                    serde_json::Value::Null,
                    vec![Diagnostic::error("E-CORE-001", error.to_string())],
                ),
            );
            return ExitCode::Internal;
        }
    };
    let selector_count = usize::from(test_id.is_some())
        + usize::from(vo_id.is_some())
        + usize::from(req_id.is_some());
    let selector_error = |message: String| {
        emit(
            format,
            quiet,
            JsonEnvelope::new(
                false,
                serde_json::Value::Null,
                vec![Diagnostic::error("E-OP-001", message)],
            ),
        );
        ExitCode::Usage
    };
    let supported = matches!(kind, "test-semantic" | "vo-coverage" | "impl-consistency");
    if !supported {
        return selector_error(format!("unsupported audit bundle kind {kind}"));
    }
    let selector_valid = match kind {
        "test-semantic" => selector_count == 1 && test_id.is_some(),
        "vo-coverage" => selector_count == 1 && (vo_id.is_some() || req_id.is_some()),
        "impl-consistency" => selector_count == 1 && (test_id.is_some() || vo_id.is_some()),
        _ => false,
    };
    if !selector_valid {
        return selector_error(format!(
            "audit bundle {kind} requires exactly one compatible --test, --vo, or --req selector"
        ));
    }

    let bundle = match build_bundle(
        &root,
        &layout,
        &scan,
        kind,
        test_id,
        vo_id,
        req_id,
        include_failed,
    ) {
        Ok(bundle) => bundle,
        Err(error) => {
            emit(
                format,
                quiet,
                JsonEnvelope::new(false, serde_json::Value::Null, vec![*error.diagnostic]),
            );
            return error.code;
        }
    };
    if bundle
        .get("skipped")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        emit(
            format,
            quiet,
            JsonEnvelope::new(true, bundle, scan.diagnostics),
        );
        return ExitCode::Ok;
    }
    let bundle_id = bundle
        .get("bundle_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let path = layout
        .cache_dir()
        .join("bundles")
        .join(format!("{bundle_id}.json"));
    if let Err(error) = fs::create_dir_all(layout.cache_dir().join("bundles")).and_then(|_| {
        serde_json::to_string_pretty(&bundle)
            .map_err(std::io::Error::other)
            .and_then(|text| fs::write(&path, format!("{text}\n")))
    }) {
        emit(
            format,
            quiet,
            JsonEnvelope::new(
                false,
                serde_json::Value::Null,
                vec![Diagnostic::error(
                    "E-CORE-001",
                    format!("failed to write bundle {}: {error}", path.display()),
                )],
            ),
        );
        return ExitCode::Internal;
    }
    let data = serde_json::json!({
        "bundle_id": bundle_id,
        "kind": kind,
        "path": relative_path(&root, &path),
    });
    emit(
        format,
        quiet,
        JsonEnvelope::new(true, data, scan.diagnostics),
    );
    ExitCode::Ok
}

fn run_audit_submit(project: &Path, file: &Path, format: OutputFormat, quiet: bool) -> ExitCode {
    let root = match resolve_root(project, format, quiet) {
        Ok(root) => root,
        Err(code) => return code,
    };
    let layout = VerifyLayout::new(&root);
    let input = absolute_path(file);
    let text = match fs::read_to_string(&input) {
        Ok(text) => text,
        Err(error) => {
            emit(
                format,
                quiet,
                JsonEnvelope::new(
                    false,
                    serde_json::Value::Null,
                    vec![Diagnostic::error(
                        "E-OP-001",
                        format!("failed to read submission {}: {error}", input.display()),
                    )],
                ),
            );
            return ExitCode::Usage;
        }
    };
    let result = match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(value) => value,
        Err(error) => {
            emit(
                format,
                quiet,
                JsonEnvelope::new(
                    false,
                    serde_json::Value::Null,
                    vec![Diagnostic::error(
                        "E-AUDIT-003",
                        format!("submission is not valid JSON: {error}"),
                    )],
                ),
            );
            return ExitCode::Usage;
        }
    };
    let bundle_id = match nonempty_string(&result, "bundle_id") {
        Ok(value) => value,
        Err(error) => return emit_command_failure(error, format, quiet),
    };
    if !is_safe_record_id(bundle_id) {
        return emit_command_failure(
            failure(
                "E-AUDIT-003",
                "bundle_id is not a safe record id",
                ExitCode::Usage,
            ),
            format,
            quiet,
        );
    }
    let kind = match nonempty_string(&result, "kind") {
        Ok(value) => value,
        Err(error) => return emit_command_failure(error, format, quiet),
    };
    let bundle_path = layout
        .cache_dir()
        .join("bundles")
        .join(format!("{bundle_id}.json"));
    let bundle_text = match fs::read_to_string(&bundle_path) {
        Ok(text) => text,
        Err(_) => {
            return emit_command_failure(
                failure(
                    "E-AUDIT-001",
                    format!("bundle {bundle_id} does not exist"),
                    ExitCode::Usage,
                ),
                format,
                quiet,
            )
        }
    };
    let bundle = match serde_json::from_str::<serde_json::Value>(&bundle_text) {
        Ok(value) => value,
        Err(error) => {
            return emit_command_failure(
                failure(
                    "E-AUDIT-003",
                    format!("cached bundle is invalid JSON: {error}"),
                    ExitCode::Usage,
                ),
                format,
                quiet,
            )
        }
    };
    if bundle.get("kind").and_then(serde_json::Value::as_str) != Some(kind) {
        return emit_command_failure(
            failure(
                "E-AUDIT-003",
                "submission kind does not match the cached bundle",
                ExitCode::Usage,
            ),
            format,
            quiet,
        );
    }
    let scan = match scan_project(&root) {
        Ok(scan) => scan,
        Err(error) => {
            return emit_command_failure(
                failure("E-CORE-001", error.to_string(), ExitCode::Internal),
                format,
                quiet,
            )
        }
    };
    if let Err(error) = validate_bundle_subjects(&root, &layout, &scan, &bundle) {
        return emit_command_failure(error, format, quiet);
    }
    let verdict = match submitted_verdict(kind, &result) {
        Ok(value) => value,
        Err(error) => return emit_command_failure(error, format, quiet),
    };
    if let Err(error) = validate_reasons(kind, &result) {
        return emit_command_failure(error, format, quiet);
    }
    let audit_id = new_record_id();
    let audit_path = layout.audits_dir().join(format!("{audit_id}.yaml"));
    let audit_yaml = audit_record_yaml(&audit_id, bundle_id, kind, &bundle, &result, &verdict);
    if let Err(error) = fs::create_dir_all(layout.audits_dir()).and_then(|_| {
        write_atomic(&audit_path, &audit_yaml)
            .map_err(|error| std::io::Error::other(error.to_string()))
    }) {
        return emit_command_failure(
            failure(
                "E-CORE-001",
                format!("failed to write audit record: {error}"),
                ExitCode::Internal,
            ),
            format,
            quiet,
        );
    }
    let data = serde_json::json!({
        "accepted": true,
        "audit_id": audit_id,
        "bundle_id": bundle_id,
        "kind": kind,
        "verdict": verdict,
    });
    emit(
        format,
        quiet,
        JsonEnvelope::new(true, data, scan.diagnostics),
    );
    ExitCode::Ok
}

fn emit_command_failure(error: CommandFailure, format: OutputFormat, quiet: bool) -> ExitCode {
    let code = error.code;
    emit(
        format,
        quiet,
        JsonEnvelope::new(false, serde_json::Value::Null, vec![*error.diagnostic]),
    );
    code
}

#[allow(clippy::too_many_arguments)]
fn build_bundle(
    root: &Path,
    layout: &VerifyLayout,
    scan: &ScanResult,
    kind: &str,
    test_id: Option<&str>,
    vo_id: Option<&str>,
    req_id: Option<&str>,
    include_failed: bool,
) -> CommandResult<serde_json::Value> {
    match kind {
        "test-semantic" => {
            let test = find_test(scan, Some(test_id.expect("validated selector")))?;
            let static_summary = audit_static(
                root,
                scan,
                &AuditOptions {
                    test_id: Some(test.id.to_string()),
                },
            )
            .map_err(|error| failure("E-OP-001", error.to_string(), ExitCode::Usage))?;
            let static_audit = static_summary.audits.first();
            if static_audit
                .is_some_and(|audit| audit.verdict == AuditVerdict::Fail && !include_failed)
            {
                return Ok(serde_json::json!({
                    "skipped": true,
                    "reason": "static audit verdict is FAIL; pass --include-failed to force a bundle",
                    "test_id": test.id,
                }));
            }
            let target = find_target_source(scan, &test.target).ok_or_else(|| {
                failure(
                    "E-SCAN-004",
                    format!("test {} target cannot be resolved", test.id),
                    ExitCode::Usage,
                )
            })?;
            let vos = test
                .covers
                .iter()
                .map(|id| vo_value(layout, id.as_str()))
                .collect::<Result<Vec<_>, _>>()?;
            let mut subjects = vec![subject_value(
                "test",
                Some(test.id.as_str()),
                None,
                &test.content_hash,
            )];
            subjects.push(subject_value(
                "target",
                None,
                Some(&target.locator.as_string()),
                &target.content_hash,
            ));
            for (vo, subject) in test.covers.iter().zip(vos.iter()) {
                subjects.push(subject_value(
                    "vo",
                    Some(vo.as_str()),
                    None,
                    &subject["content_hash"]
                        .as_str()
                        .and_then(|hash| hash.parse().ok())
                        .unwrap_or_else(|| ContentHash::from_text("")),
                ));
            }
            let related_ids = test
                .related
                .iter()
                .map(ToString::to_string)
                .collect::<BTreeSet<_>>();
            let related_tests = related_ids
                .iter()
                .filter_map(|id| scan.tests.iter().find(|candidate| candidate.id.as_str() == id))
                .map(|candidate| serde_json::json!({ "id": candidate.id, "intent": candidate.intent }))
                .collect::<Vec<_>>();
            let sibling_tests = scan
                .tests
                .iter()
                .filter(|candidate| {
                    candidate.id != test.id
                        && candidate
                            .covers
                            .iter()
                            .any(|covered| test.covers.contains(covered))
                })
                .map(|candidate| serde_json::json!({ "id": candidate.id, "intent": candidate.intent }))
                .collect::<Vec<_>>();
            let test_value = test_value(root, test)?;
            let target_value = target_value(root, target)?;
            let static_value = static_audit.map(|audit| {
                serde_json::json!({
                    "verdict": audit.verdict,
                    "rules": audit.rules,
                })
            });
            Ok(serde_json::json!({
                "bundle_id": new_record_id(),
                "kind": kind,
                "generated_at": now_rfc3339(),
                "revision": git_revision_json(root),
                "test": test_value,
                "vos": vos,
                "target": target_value,
                "related_tests": related_tests,
                "sibling_tests": sibling_tests,
                "static_audit": static_value,
                "prior_audits": prior_audits(layout, test.id.as_str(), &test.content_hash),
                "subjects": subjects,
            }))
        }
        "vo-coverage" => {
            let selected = collect_scope_vos(layout, vo_id, req_id)?;
            let mut subjects = Vec::new();
            let mut requirements = BTreeMap::new();
            let mut specs = BTreeMap::new();
            for vo in &selected {
                let value = vo_value(layout, vo.id.as_str())?;
                let hash = value["content_hash"]
                    .as_str()
                    .and_then(|hash| hash.parse().ok())
                    .unwrap_or_else(|| ContentHash::from_text(""));
                subjects.push(subject_value("vo", Some(vo.id.as_str()), None, &hash));
                for requirement in &vo.requirements {
                    if let Ok(record) = read_req(layout, requirement.as_str()) {
                        let key = record.id.to_string();
                        let path = layout.req_dir().join(format!("{key}.yaml"));
                        let record_hash = fs::read_to_string(path)
                            .map(|text| ContentHash::from_text(&text))
                            .unwrap_or_else(|_| ContentHash::from_text(""));
                        subjects.push(subject_value(
                            "req",
                            Some(record.id.as_str()),
                            None,
                            &record_hash,
                        ));
                        requirements.insert(key, serde_json::to_value(record).expect("REQ record"));
                    }
                }
                for spec_ref in &vo.spec_refs {
                    if let Ok(record) = read_spec(layout, spec_ref.spec.as_str()) {
                        let key = record.id.to_string();
                        subjects.push(subject_value(
                            "spec",
                            Some(record.id.as_str()),
                            None,
                            &record.sha256,
                        ));
                        specs.insert(
                            key,
                            serde_json::json!({
                                "id": record.id,
                                "path": record.path,
                                "sha256": record.sha256,
                                "section": spec_ref.section,
                            }),
                        );
                    }
                }
            }
            for requirement in requirements.values() {
                let Some(spec_refs) = requirement
                    .get("spec_refs")
                    .and_then(serde_json::Value::as_array)
                else {
                    continue;
                };
                for spec_ref in spec_refs {
                    let Some(spec_id) = spec_ref.get("spec").and_then(serde_json::Value::as_str)
                    else {
                        continue;
                    };
                    if let Ok(record) = read_spec(layout, spec_id) {
                        let key = record.id.to_string();
                        subjects.push(subject_value(
                            "spec",
                            Some(record.id.as_str()),
                            None,
                            &record.sha256,
                        ));
                        specs.entry(key).or_insert_with(|| {
                            serde_json::json!({
                                "id": record.id,
                                "path": record.path,
                                "sha256": record.sha256,
                                "section": spec_ref.get("section"),
                            })
                        });
                    }
                }
            }
            let leaf_coverage = selected
                .iter()
                .filter(|vo| {
                    !selected
                        .iter()
                        .any(|candidate| candidate.parent.as_ref() == Some(&vo.id))
                })
                .map(|vo| {
                    let tests = scan
                        .tests
                        .iter()
                        .filter(|test| test.covers.contains(&vo.id))
                        .map(|test| test.id.to_string())
                        .collect::<Vec<_>>();
                    serde_json::json!({ "vo": vo.id, "tests": tests })
                })
                .collect::<Vec<_>>();
            Ok(serde_json::json!({
                "bundle_id": new_record_id(),
                "kind": kind,
                "generated_at": now_rfc3339(),
                "revision": git_revision_json(root),
                "vos": selected.iter().map(|vo| vo_value(layout, vo.id.as_str())).collect::<Result<Vec<_>, _>>()?,
                "requirements": requirements.into_values().collect::<Vec<_>>(),
                "specs": specs.into_values().collect::<Vec<_>>(),
                "leaf_coverage": leaf_coverage,
                "subjects": subjects,
            }))
        }
        "impl-consistency" => {
            if let Some(test_id) = test_id {
                let test = find_test(scan, Some(test_id))?;
                let target = find_target_source(scan, &test.target).ok_or_else(|| {
                    failure(
                        "E-SCAN-004",
                        format!("test {} target cannot be resolved", test.id),
                        ExitCode::Usage,
                    )
                })?;
                let vos = test
                    .covers
                    .iter()
                    .map(|id| vo_value(layout, id.as_str()))
                    .collect::<Result<Vec<_>, _>>()?;
                let mut subjects = vec![subject_value(
                    "test",
                    Some(test.id.as_str()),
                    None,
                    &test.content_hash,
                )];
                subjects.push(subject_value(
                    "target",
                    None,
                    Some(&target.locator.as_string()),
                    &target.content_hash,
                ));
                for (vo, subject) in test.covers.iter().zip(vos.iter()) {
                    let hash = subject["content_hash"]
                        .as_str()
                        .and_then(|hash| hash.parse().ok())
                        .unwrap_or_else(|| ContentHash::from_text(""));
                    subjects.push(subject_value("vo", Some(vo.as_str()), None, &hash));
                }
                Ok(serde_json::json!({
                    "bundle_id": new_record_id(),
                    "kind": kind,
                    "generated_at": now_rfc3339(),
                    "revision": git_revision_json(root),
                    "test": test_value(root, test)?,
                    "vos": vos,
                    "target": target_value(root, target)?,
                    "related_tests": test.related.iter().filter_map(|id| {
                        scan.tests.iter().find(|candidate| candidate.id == *id)
                    }).map(|candidate| serde_json::json!({ "id": candidate.id, "intent": candidate.intent })).collect::<Vec<_>>(),
                    "subjects": subjects,
                }))
            } else {
                let vo =
                    read_vo(layout, vo_id.expect("validated selector")).map_err(store_failure)?;
                let mut subjects = Vec::new();
                let vo_value = vo_value(layout, vo.id.as_str())?;
                let hash = vo_value["content_hash"]
                    .as_str()
                    .and_then(|hash| hash.parse().ok())
                    .unwrap_or_else(|| ContentHash::from_text(""));
                subjects.push(subject_value("vo", Some(vo.id.as_str()), None, &hash));
                let tests = scan
                    .tests
                    .iter()
                    .filter(|test| test.covers.contains(&vo.id))
                    .collect::<Vec<_>>();
                let mut targets = Vec::new();
                for test in &tests {
                    if let Some(target) = find_target_source(scan, &test.target) {
                        subjects.push(subject_value(
                            "test",
                            Some(test.id.as_str()),
                            None,
                            &test.content_hash,
                        ));
                        subjects.push(subject_value(
                            "target",
                            None,
                            Some(&target.locator.as_string()),
                            &target.content_hash,
                        ));
                        targets.push(target_value(root, target)?);
                    }
                }
                if targets.is_empty() {
                    return Err(failure(
                        "E-SCAN-004",
                        format!("VO {} has no resolvable target function", vo.id),
                        ExitCode::Usage,
                    ));
                }
                Ok(serde_json::json!({
                    "bundle_id": new_record_id(),
                    "kind": kind,
                    "generated_at": now_rfc3339(),
                    "revision": git_revision_json(root),
                    "vo": vo_value,
                    "tests": tests.iter().map(|test| test_value(root, test)).collect::<Result<Vec<_>, _>>()?,
                    "targets": targets,
                    "subjects": subjects,
                }))
            }
        }
        _ => Err(failure(
            "E-OP-001",
            "unsupported bundle kind",
            ExitCode::Usage,
        )),
    }
}

fn find_test<'a>(scan: &'a ScanResult, id: Option<&str>) -> CommandResult<&'a TestEntity> {
    let id = id.unwrap_or_default();
    scan.tests
        .iter()
        .find(|test| test.id.as_str() == id)
        .ok_or_else(|| {
            failure(
                "E-OP-001",
                format!("test {id} was not found"),
                ExitCode::Usage,
            )
        })
}

fn find_target_source<'a>(scan: &'a ScanResult, target: &TargetRef) -> Option<&'a SourceFunction> {
    match target {
        TargetRef::Locator(locator) => scan
            .sources
            .iter()
            .find(|source| source.locator == *locator),
        TargetRef::SrcId(src_id) => scan.sources.iter().find(|source| {
            source
                .src_id
                .as_ref()
                .is_some_and(|candidate| candidate == src_id)
        }),
    }
}

fn source_slice(root: &Path, location: &vtest_model::SourceLocation) -> CommandResult<String> {
    let path = root.join(&location.file);
    let source = fs::read_to_string(&path).map_err(|error| {
        failure(
            "E-CORE-001",
            format!("failed to read {}: {error}", path.display()),
            ExitCode::Internal,
        )
    })?;
    let value = source
        .get(location.start_byte..location.end_byte)
        .ok_or_else(|| {
            failure(
                "E-SCAN-004",
                format!(
                    "invalid source span {}:{}-{}",
                    location.file, location.start_byte, location.end_byte
                ),
                ExitCode::Usage,
            )
        })?;
    Ok(value.to_owned())
}

fn test_value(root: &Path, test: &TestEntity) -> CommandResult<serde_json::Value> {
    let source = source_slice(root, &test.location)?;
    Ok(serde_json::json!({
        "id": test.id,
        "intent": test.intent,
        "annotations": {
            "input": test.input,
            "expect": test.expect,
            "kind": test.kind,
            "cases": test.cases,
        },
        "location": {
            "file": test.location.file,
            "function": test.location.function,
            "start_line": test.location.start_line,
            "end_line": test.location.end_line,
        },
        "source": source,
        "content_hash": test.content_hash,
    }))
}

fn target_value(root: &Path, target: &SourceFunction) -> CommandResult<serde_json::Value> {
    Ok(serde_json::json!({
        "locator": target.locator.as_string(),
        "source": source_slice(root, &target.location)?,
        "content_hash": target.content_hash,
    }))
}

fn vo_value(layout: &VerifyLayout, id: &str) -> CommandResult<serde_json::Value> {
    let record = read_vo(layout, id).map_err(store_failure)?;
    let path = layout.vo_dir().join(format!("{id}.yaml"));
    let text = read_text(&path).map_err(store_failure)?;
    Ok(serde_json::json!({
        "id": record.id,
        "claim": record.claim,
        "dimensions": record.dimensions,
        "spec_refs": record.spec_refs,
        "requirements": record.requirements,
        "status": record.status,
        "content_hash": ContentHash::from_text(&text),
    }))
}

fn collect_scope_vos(
    layout: &VerifyLayout,
    vo_id: Option<&str>,
    req_id: Option<&str>,
) -> CommandResult<Vec<VoRecord>> {
    let ids = read_record_ids_for(&layout.vo_dir())?;
    let mut records = Vec::new();
    for id in ids {
        records.push(read_vo(layout, &id).map_err(store_failure)?);
    }
    let selected = if let Some(vo_id) = vo_id {
        if !records.iter().any(|vo| vo.id.as_str() == vo_id) {
            return Err(failure(
                "E-OP-001",
                format!("VO {vo_id} was not found"),
                ExitCode::Usage,
            ));
        }
        records
            .iter()
            .filter(|vo| vo.id.as_str() == vo_id || has_vo_ancestor(&records, vo, vo_id))
            .cloned()
            .collect::<Vec<_>>()
    } else {
        let req_id = req_id.unwrap_or_default();
        if read_req(layout, req_id).is_err() {
            return Err(failure(
                "E-OP-001",
                format!("REQ {req_id} was not found"),
                ExitCode::Usage,
            ));
        }
        records
            .iter()
            .filter(|vo| {
                vo.requirements.iter().any(|req| req.as_str() == req_id)
                    || has_vo_requirement_ancestor(&records, vo, req_id)
            })
            .cloned()
            .collect::<Vec<_>>()
    };
    if selected.is_empty() {
        return Err(failure(
            "E-OP-001",
            "the selected scope contains no VO records",
            ExitCode::Usage,
        ));
    }
    Ok(selected)
}

fn has_vo_ancestor(records: &[VoRecord], child: &VoRecord, ancestor: &str) -> bool {
    let mut current = child.parent.as_ref().map(ToString::to_string);
    while let Some(id) = current {
        if id == ancestor {
            return true;
        }
        current = records
            .iter()
            .find(|candidate| candidate.id.as_str() == id)
            .and_then(|candidate| candidate.parent.as_ref().map(ToString::to_string));
    }
    false
}

fn has_vo_requirement_ancestor(records: &[VoRecord], child: &VoRecord, req_id: &str) -> bool {
    let mut current = child.parent.as_ref().map(ToString::to_string);
    while let Some(id) = current {
        let Some(parent) = records.iter().find(|candidate| candidate.id.as_str() == id) else {
            break;
        };
        if parent.requirements.iter().any(|req| req.as_str() == req_id) {
            return true;
        }
        current = parent.parent.as_ref().map(ToString::to_string);
    }
    false
}

fn subject_value(
    kind: &str,
    id: Option<&str>,
    locator: Option<&str>,
    hash: &ContentHash,
) -> serde_json::Value {
    let mut value = serde_json::Map::new();
    value.insert(
        "kind".to_owned(),
        serde_json::Value::String(kind.to_owned()),
    );
    if let Some(id) = id {
        value.insert("id".to_owned(), serde_json::Value::String(id.to_owned()));
    }
    if let Some(locator) = locator {
        value.insert(
            "locator".to_owned(),
            serde_json::Value::String(locator.to_owned()),
        );
    }
    value.insert(
        "hash".to_owned(),
        serde_json::Value::String(hash.to_string()),
    );
    serde_json::Value::Object(value)
}

fn git_revision_json(root: &Path) -> serde_json::Value {
    let commit = ProcessCommand::new("git")
        .current_dir(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|value| !value.is_empty());
    let dirty = ProcessCommand::new("git")
        .current_dir(root)
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .is_some_and(|output| output.status.success() && !output.stdout.is_empty());
    serde_json::to_value(Revision { commit, dirty }).expect("revision is serializable")
}

fn prior_audits(
    layout: &VerifyLayout,
    test_id: &str,
    current_hash: &ContentHash,
) -> Vec<serde_json::Value> {
    let Ok(entries) = fs::read_dir(layout.audits_dir()) else {
        return Vec::new();
    };
    let mut records = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("yaml") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let is_subject = vtest_store::yaml_scalar_value(&text, "test_id")
            .is_some_and(|value| value == test_id)
            || text.contains(&format!("id: {test_id}"));
        if !is_subject {
            continue;
        }
        let id = vtest_store::yaml_scalar_value(&text, "id").or_else(|| {
            path.file_stem()
                .and_then(|value| value.to_str())
                .map(str::to_owned)
        });
        let valid = yaml_scalar_value(&text, "test_hash")
            .and_then(|hash| hash.parse::<ContentHash>().ok())
            .is_some_and(|hash| &hash == current_hash)
            || text.lines().any(|line| {
                line.trim_start().starts_with("hash:")
                    && yaml_scalar_value(line, "hash")
                        .is_some_and(|hash| hash == current_hash.to_string())
            });
        records.push(serde_json::json!({
            "id": id,
            "verdict": vtest_store::yaml_scalar_value(&text, "verdict"),
            "audited_at": vtest_store::yaml_scalar_value(&text, "audited_at"),
            "valid": valid,
        }));
    }
    records.sort_by(|left, right| {
        left["id"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["id"].as_str().unwrap_or_default())
    });
    records
}

fn nonempty_string<'a>(
    value: &'a serde_json::Value,
    field: &str,
) -> Result<&'a str, CommandFailure> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            failure(
                "E-AUDIT-003",
                format!("submission requires a non-empty {field}"),
                ExitCode::Usage,
            )
        })
}

fn is_safe_record_id(value: &str) -> bool {
    value.len() == 26
        && value
            .bytes()
            .all(|byte| b"0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(&byte))
}

fn validate_bundle_subjects(
    root: &Path,
    layout: &VerifyLayout,
    scan: &ScanResult,
    bundle: &serde_json::Value,
) -> Result<(), CommandFailure> {
    let subjects = bundle
        .get("subjects")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            failure(
                "E-AUDIT-003",
                "cached bundle has no subjects",
                ExitCode::Usage,
            )
        })?;
    for subject in subjects {
        let kind = subject
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let expected = subject
            .get("hash")
            .and_then(serde_json::Value::as_str)
            .and_then(|hash| hash.parse::<ContentHash>().ok())
            .ok_or_else(|| {
                failure(
                    "E-AUDIT-003",
                    "cached bundle contains an invalid subject hash",
                    ExitCode::Usage,
                )
            })?;
        let actual = match kind {
            "test" => {
                let id = subject
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                scan.tests
                    .iter()
                    .find(|test| test.id.as_str() == id)
                    .map(|test| test.content_hash.clone())
            }
            "target" => {
                let locator = subject
                    .get("locator")
                    .and_then(serde_json::Value::as_str)
                    .and_then(Locator::parse);
                locator.and_then(|locator| {
                    scan.sources
                        .iter()
                        .find(|source| source.locator == locator)
                        .map(|source| source.content_hash.clone())
                })
            }
            "vo" => current_record_hash(&layout.vo_dir(), subject),
            "req" => current_record_hash(&layout.req_dir(), subject),
            "spec" => {
                let id = subject
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                read_spec(layout, id).ok().and_then(|record| {
                    let path = root.join(&record.path);
                    fs::read_to_string(path)
                        .ok()
                        .map(|text| ContentHash::from_text(&text))
                })
            }
            _ => None,
        };
        if actual.as_ref() != Some(&expected) {
            return Err(failure(
                "E-AUDIT-002",
                format!("subject {kind} hash no longer matches the cached bundle"),
                ExitCode::VerificationFailed,
            ));
        }
    }
    Ok(())
}

fn current_record_hash(directory: &Path, subject: &serde_json::Value) -> Option<ContentHash> {
    let id = subject
        .get("id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| is_safe_entity_id(value))?;
    let path = directory.join(format!("{id}.yaml"));
    fs::read_to_string(path)
        .ok()
        .map(|text| ContentHash::from_text(&text))
}

fn is_safe_entity_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn submitted_verdict(kind: &str, result: &serde_json::Value) -> Result<String, CommandFailure> {
    let verdict = result
        .get("verdict")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let valid = match kind {
        "vo-coverage" => matches!(verdict, "COMPLETE" | "INCOMPLETE" | "UNKNOWN"),
        "test-semantic" | "impl-consistency" => matches!(verdict, "PASS" | "FAIL" | "UNKNOWN"),
        _ => false,
    };
    if !valid {
        return Err(failure(
            "E-AUDIT-004",
            format!("verdict {verdict} is not valid for {kind}"),
            ExitCode::Usage,
        ));
    }
    Ok(match verdict {
        "COMPLETE" => "PASS".to_owned(),
        "INCOMPLETE" => "FAIL".to_owned(),
        other => other.to_owned(),
    })
}

fn validate_reasons(kind: &str, result: &serde_json::Value) -> Result<(), CommandFailure> {
    let reasons = result
        .get("reasons")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            failure(
                "E-AUDIT-005",
                "reasons must be a non-empty array",
                ExitCode::Usage,
            )
        })?;
    if reasons.is_empty() {
        return Err(failure(
            "E-AUDIT-005",
            "reasons must be a non-empty array",
            ExitCode::Usage,
        ));
    }
    for reason in reasons {
        let claim = reason
            .get("claim")
            .and_then(serde_json::Value::as_str)
            .filter(|claim| !claim.trim().is_empty());
        let basis = reason.get("basis").and_then(serde_json::Value::as_array);
        if claim.is_none() || basis.is_none_or(|basis| basis.is_empty()) {
            return Err(failure(
                "E-AUDIT-005",
                "each reason requires a non-empty claim and at least one basis",
                ExitCode::Usage,
            ));
        }
        for item in basis.expect("checked above") {
            let basis_kind = item.get("kind").and_then(serde_json::Value::as_str);
            let basis_ref = item.get("ref").and_then(serde_json::Value::as_str);
            if basis_kind.is_none_or(|kind| kind.trim().is_empty())
                || basis_ref.is_none_or(|reference| reference.trim().is_empty())
            {
                return Err(failure(
                    "E-AUDIT-005",
                    "each basis requires kind and ref",
                    ExitCode::Usage,
                ));
            }
        }
    }
    if kind == "vo-coverage" {
        let has_viewpoint = reasons.iter().any(|reason| {
            reason.get("kind").and_then(serde_json::Value::as_str)
                == Some("decomposition-viewpoint")
        });
        let has_spec = reasons
            .iter()
            .flat_map(|reason| {
                reason
                    .get("basis")
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
            })
            .any(|basis| basis.get("kind").and_then(serde_json::Value::as_str) == Some("spec"));
        if !has_viewpoint || !has_spec {
            return Err(failure(
                "E-AUDIT-006",
                "vo-coverage requires a decomposition-viewpoint reason and a spec basis",
                ExitCode::Usage,
            ));
        }
    }
    Ok(())
}

fn audit_record_yaml(
    audit_id: &str,
    bundle_id: &str,
    kind: &str,
    bundle: &serde_json::Value,
    result: &serde_json::Value,
    verdict: &str,
) -> String {
    let mut out = format!(
        "id: {}\nkind: {}\nbundle_id: {}\nsubjects:\n",
        yaml_quote(audit_id),
        yaml_quote(kind),
        yaml_quote(bundle_id),
    );
    if let Some(subjects) = bundle.get("subjects").and_then(serde_json::Value::as_array) {
        for subject in subjects {
            let subject_kind = subject
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown");
            let hash = subject
                .get("hash")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            out.push_str(&format!(
                "  - kind: {}\n    id: {}\n    locator: {}\n    hash: {}\n",
                yaml_quote(subject_kind),
                yaml_quote(
                    subject
                        .get("id")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("")
                ),
                yaml_quote(
                    subject
                        .get("locator")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("")
                ),
                yaml_quote(hash),
            ));
        }
    }
    out.push_str(&format!("verdict: {}\nreasons:\n", yaml_quote(verdict)));
    if let Some(reasons) = result.get("reasons").and_then(serde_json::Value::as_array) {
        for reason in reasons {
            let claim = reason
                .get("claim")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            out.push_str(&format!("  - claim: {}\n", yaml_quote(claim)));
            if let Some(kind) = reason.get("kind").and_then(serde_json::Value::as_str) {
                out.push_str(&format!("    kind: {}\n", yaml_quote(kind)));
            }
            out.push_str("    basis:\n");
            if let Some(basis) = reason.get("basis").and_then(serde_json::Value::as_array) {
                for item in basis {
                    out.push_str(&format!(
                        "      - kind: {}\n        ref: {}\n",
                        yaml_quote(
                            item.get("kind")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("")
                        ),
                        yaml_quote(
                            item.get("ref")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("")
                        ),
                    ));
                }
            }
        }
    }
    out.push_str(&format!(
        "auditor:\n  kind: {}\n  id: vtest\naudited_at: {}\n",
        yaml_quote("agent"),
        yaml_quote(&now_rfc3339())
    ));
    out
}

fn yaml_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[allow(clippy::too_many_arguments)]
fn run_run(
    project: &Path,
    test: Option<String>,
    vo: Option<String>,
    req: Option<String>,
    all: bool,
    fast: bool,
    format: OutputFormat,
    quiet: bool,
) -> ExitCode {
    let root = match resolve_root(project, format, quiet) {
        Ok(root) => root,
        Err(code) => return code,
    };
    let config = match load_config(&root) {
        Ok(config) => config,
        Err(error) => {
            emit(
                format,
                quiet,
                JsonEnvelope::new(
                    false,
                    serde_json::Value::Null,
                    vec![Diagnostic::error("E-CORE-001", error.to_string())],
                ),
            );
            return ExitCode::Internal;
        }
    };
    let fast = fast || config.run.coverage == "off";
    let selected_count = usize::from(test.is_some())
        + usize::from(vo.is_some())
        + usize::from(req.is_some())
        + usize::from(all);
    if selected_count != 1 {
        emit(
            format,
            quiet,
            JsonEnvelope::new(
                false,
                serde_json::Value::Null,
                vec![Diagnostic::error(
                    "E-OP-001",
                    "run requires exactly one of --test, --vo, --req, or --all",
                )],
            ),
        );
        return ExitCode::Usage;
    }
    let scan = match scan_project(&root) {
        Ok(scan) => scan,
        Err(error) => {
            emit(
                format,
                quiet,
                JsonEnvelope::new(
                    false,
                    serde_json::Value::Null,
                    vec![Diagnostic::error("E-CORE-001", error.to_string())],
                ),
            );
            return ExitCode::Internal;
        }
    };
    let layout = VerifyLayout::new(&root);
    let requested = match select_tests(
        &layout,
        &scan.tests,
        test.as_deref(),
        vo.as_deref(),
        req.as_deref(),
        all,
    ) {
        Ok(requested) => requested,
        Err(error) => {
            emit(
                format,
                quiet,
                JsonEnvelope::new(false, serde_json::Value::Null, vec![*error.diagnostic]),
            );
            return error.code;
        }
    };
    let runnable = requested
        .into_iter()
        .map(|entity| {
            let target = scan.sources.iter().find(|source| match &entity.target {
                vtest_model::TargetRef::Locator(locator) => source.locator == *locator,
                vtest_model::TargetRef::SrcId(src_id) => source.src_id.as_ref() == Some(src_id),
            });
            let target_hash = target
                .map(|source| source.content_hash.clone())
                .unwrap_or_else(|| ContentHash::from_text(""));
            RunnableTest {
                entity,
                target_hash,
                target_locator: target.map(|source| source.locator.clone()),
            }
        })
        .collect::<Vec<_>>();
    let result = match run_tests(&root, &layout, &runnable, fast) {
        Ok(result) => result,
        Err(error) => {
            emit(
                format,
                quiet,
                JsonEnvelope::new(
                    false,
                    serde_json::Value::Null,
                    vec![Diagnostic::error("E-CORE-001", error.to_string())],
                ),
            );
            return ExitCode::Internal;
        }
    };
    let has_errors = scan.has_errors() || result.has_errors();
    let diagnostics = scan
        .diagnostics
        .into_iter()
        .chain(result.diagnostics.clone())
        .collect::<Vec<_>>();
    emit(
        format,
        quiet,
        JsonEnvelope::new(
            !has_errors,
            serde_json::to_value(&result).expect("execution result"),
            diagnostics,
        ),
    );
    if has_errors {
        ExitCode::VerificationFailed
    } else {
        ExitCode::Ok
    }
}

fn select_tests(
    layout: &VerifyLayout,
    tests: &[vtest_model::TestEntity],
    test: Option<&str>,
    vo: Option<&str>,
    req: Option<&str>,
    all: bool,
) -> Result<Vec<vtest_model::TestEntity>, CommandFailure> {
    if let Some(test_id) = test {
        return tests
            .iter()
            .find(|entity| entity.id.as_str() == test_id)
            .cloned()
            .map(|entity| vec![entity])
            .ok_or_else(|| {
                failure(
                    "E-OP-001",
                    format!("Test `{test_id}` was not found"),
                    ExitCode::Usage,
                )
            });
    }
    if let Some(vo_id) = vo {
        validate_id(vo_id, "VO-")?;
        if read_vo(layout, vo_id).is_err() {
            return Err(failure(
                "E-OP-001",
                format!("VO `{vo_id}` was not found"),
                ExitCode::Usage,
            ));
        }
        return Ok(tests
            .iter()
            .filter(|entity| {
                entity
                    .covers
                    .iter()
                    .any(|covered| covered.as_str() == vo_id)
            })
            .cloned()
            .collect());
    }
    if let Some(req_id) = req {
        validate_id(req_id, "REQ-")?;
        let vo_ids = read_record_ids_for(&layout.vo_dir())?
            .into_iter()
            .filter_map(|id| {
                read_vo(layout, &id)
                    .ok()
                    .filter(|vo| vo.requirements.iter().any(|req| req.as_str() == req_id))
                    .map(|_| id)
            })
            .collect::<Vec<_>>();
        return Ok(tests
            .iter()
            .filter(|entity| {
                entity
                    .covers
                    .iter()
                    .any(|covered| vo_ids.iter().any(|id| id == covered.as_str()))
            })
            .cloned()
            .collect());
    }
    if all {
        return Ok(tests.to_vec());
    }
    Ok(Vec::new())
}

#[allow(clippy::too_many_arguments)]
fn run_verify(
    project: &Path,
    items: Option<String>,
    req: Option<String>,
    vo: Option<String>,
    test: Option<String>,
    _summary: bool,
    format: OutputFormat,
    quiet: bool,
) -> ExitCode {
    let root = match resolve_root(project, format, quiet) {
        Ok(root) => root,
        Err(code) => return code,
    };
    if req.is_some() || vo.is_some() || test.is_some() {
        // Entity-scoped filtering is part of the same aggregate engine; until
        // the graph scope selector is added, retain the explicit request in a
        // diagnostic rather than silently broadening it.
        emit(
            format,
            quiet,
            JsonEnvelope::new(
                false,
                serde_json::Value::Null,
                vec![Diagnostic::error(
                    "E-OP-001",
                    "entity-scoped verify/report is not implemented yet",
                )],
            ),
        );
        return ExitCode::Usage;
    }
    let config = match load_config(&root) {
        Ok(config) => config,
        Err(error) => {
            emit(
                format,
                quiet,
                JsonEnvelope::new(
                    false,
                    serde_json::Value::Null,
                    vec![Diagnostic::error("E-CORE-001", error.to_string())],
                ),
            );
            return ExitCode::Internal;
        }
    };
    let scan = match scan_project(&root) {
        Ok(scan) => scan,
        Err(error) => {
            emit(
                format,
                quiet,
                JsonEnvelope::new(
                    false,
                    serde_json::Value::Null,
                    vec![Diagnostic::error("E-CORE-001", error.to_string())],
                ),
            );
            return ExitCode::Internal;
        }
    };
    let requested = items.map(|items| {
        items
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>()
    });
    if let Some(scope) = &requested {
        if let Some(unknown) = scope
            .iter()
            .find(|item| !ALL_VERIFY_ITEMS.contains(&item.as_str()))
        {
            emit(
                format,
                quiet,
                JsonEnvelope::new(
                    false,
                    serde_json::Value::Null,
                    vec![Diagnostic::error(
                        "E-OP-001",
                        format!("unknown check item `{unknown}`"),
                    )],
                ),
            );
            return ExitCode::Usage;
        }
    }
    let result = verify_project(&root, &scan, &config, requested);
    let ok = result.is_ok();
    let diagnostics = result.diagnostics.clone();
    emit(
        format,
        quiet,
        JsonEnvelope::new(
            ok,
            serde_json::to_value(&result).expect("verification result"),
            diagnostics,
        ),
    );
    if ok {
        ExitCode::Ok
    } else {
        ExitCode::VerificationFailed
    }
}

const ALL_VERIFY_ITEMS: [&str; 11] = [
    "spec_coverage",
    "vo_decomposition",
    "vo_coverage",
    "test_existence",
    "static_audit",
    "semantic_audit",
    "impl_consistency",
    "test_execution",
    "runtime_result",
    "target_execution",
    "evidence_validity",
];

type CommandResult<T = serde_json::Value> = Result<T, CommandFailure>;

#[derive(Debug)]
struct CommandFailure {
    diagnostic: Box<Diagnostic>,
    code: ExitCode,
}

fn finish_record_command(result: CommandResult, format: OutputFormat, quiet: bool) -> ExitCode {
    match result {
        Ok(data) => {
            emit(format, quiet, JsonEnvelope::new(true, data, Vec::new()));
            ExitCode::Ok
        }
        Err(error) => {
            emit(
                format,
                quiet,
                JsonEnvelope::new(false, serde_json::Value::Null, vec![*error.diagnostic]),
            );
            error.code
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn add_spec(
    root: &Path,
    layout: &VerifyLayout,
    id: &str,
    path: &Path,
    kind: &str,
    title: Option<String>,
    note: Option<String>,
    update: bool,
) -> CommandResult {
    validate_id(id, "SPEC-")?;
    let source_path = safe_project_path(root, path)?;
    let source = fs::read_to_string(&source_path)
        .map_err(|error| failure("E-OP-001", error.to_string(), ExitCode::Usage))?;
    let record_path = layout.spec_dir().join(format!("{id}.yaml"));
    let old = if record_path.exists() {
        if !update {
            return Err(failure(
                "E-OP-001",
                format!("SPEC `{id}` already exists; pass --update to replace it"),
                ExitCode::Usage,
            ));
        }
        Some(read_spec(layout, id).map_err(store_failure)?)
    } else {
        None
    };
    let record = SpecRecord {
        id: SpecId::new(id),
        kind: kind.to_owned(),
        path: relative_path(root, &source_path),
        sha256: ContentHash::from_text(&source),
        title: title.or_else(|| old.as_ref().and_then(|value| value.title.clone())),
        note: note.or_else(|| old.as_ref().and_then(|value| value.note.clone())),
        registered_at: old
            .as_ref()
            .map(|value| value.registered_at.clone())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(now_rfc3339),
    };
    write_atomic(&record_path, &record.to_yaml()).map_err(store_failure)?;
    Ok(serde_json::to_value(record).expect("record is serializable"))
}

fn list_specs(layout: &VerifyLayout) -> CommandResult {
    let ids = read_record_ids_for(&layout.spec_dir())?;
    let mut records = Vec::new();
    for id in ids {
        records.push(
            serde_json::to_value(read_spec(layout, &id).map_err(store_failure)?).expect("record"),
        );
    }
    Ok(serde_json::json!({ "items": records }))
}

fn show_spec(layout: &VerifyLayout, id: &str) -> CommandResult {
    validate_id(id, "SPEC-")?;
    Ok(serde_json::to_value(read_spec(layout, id).map_err(store_failure)?).expect("record"))
}

fn add_req(
    layout: &VerifyLayout,
    id: &str,
    summary: &str,
    parent: Option<String>,
    specs: &[String],
    sections: &[String],
) -> CommandResult {
    validate_id(id, "REQ-")?;
    if summary.trim().is_empty() {
        return Err(failure(
            "E-OP-001",
            "REQ summary must not be empty",
            ExitCode::Usage,
        ));
    }
    let path = layout.req_dir().join(format!("{id}.yaml"));
    if path.exists() {
        return Err(failure(
            "E-OP-001",
            format!("REQ `{id}` already exists"),
            ExitCode::Usage,
        ));
    }
    let spec_refs = spec_refs_from_args(layout, specs, sections)?;
    if let Some(parent) = &parent {
        validate_id(parent, "REQ-")?;
        read_req(layout, parent).map_err(store_failure)?;
    }
    let now = now_rfc3339();
    let record = ReqRecord {
        id: ReqId::new(id),
        parent: parent.map(ReqId::new),
        spec_refs,
        summary: summary.to_owned(),
        status: "active".to_owned(),
        created: now.clone(),
        updated: now,
    };
    write_atomic(&path, &record.to_yaml()).map_err(store_failure)?;
    Ok(serde_json::to_value(record).expect("record"))
}

fn edit_req(
    layout: &VerifyLayout,
    id: &str,
    summary: Option<String>,
    parent: Option<String>,
    status: Option<String>,
) -> CommandResult {
    validate_id(id, "REQ-")?;
    let path = layout.req_dir().join(format!("{id}.yaml"));
    let mut record = read_req(layout, id).map_err(store_failure)?;
    if summary.is_none() && parent.is_none() && status.is_none() {
        return Err(failure(
            "E-OP-001",
            "REQ edit needs at least one field",
            ExitCode::Usage,
        ));
    }
    if let Some(summary) = summary {
        if summary.trim().is_empty() {
            return Err(failure(
                "E-OP-001",
                "REQ summary must not be empty",
                ExitCode::Usage,
            ));
        }
        record.summary = summary;
    }
    if let Some(parent) = parent {
        validate_id(&parent, "REQ-")?;
        read_req(layout, &parent).map_err(store_failure)?;
        record.parent = Some(ReqId::new(parent));
    }
    if let Some(status) = status {
        if !matches!(status.as_str(), "active" | "withdrawn") {
            return Err(failure(
                "E-OP-001",
                "REQ status must be active or withdrawn",
                ExitCode::Usage,
            ));
        }
        record.status = status;
    }
    record.updated = now_rfc3339();
    write_atomic(&path, &record.to_yaml()).map_err(store_failure)?;
    Ok(serde_json::to_value(record).expect("record"))
}

fn list_reqs(layout: &VerifyLayout) -> CommandResult {
    let ids = read_record_ids_for(&layout.req_dir())?;
    let mut records = Vec::new();
    for id in ids {
        records.push(
            serde_json::to_value(read_req(layout, &id).map_err(store_failure)?).expect("record"),
        );
    }
    Ok(serde_json::json!({ "items": records }))
}

fn show_req(layout: &VerifyLayout, id: &str) -> CommandResult {
    validate_id(id, "REQ-")?;
    Ok(serde_json::to_value(read_req(layout, id).map_err(store_failure)?).expect("record"))
}

#[allow(clippy::too_many_arguments)]
fn add_vo(
    layout: &VerifyLayout,
    id: &str,
    claim: &str,
    requirements: &[String],
    parent: Option<String>,
    specs: &[String],
    sections: &[String],
    dimensions: &[String],
    policy: Option<String>,
) -> CommandResult {
    validate_id(id, "VO-")?;
    if claim.trim().is_empty() {
        return Err(failure(
            "E-OP-001",
            "VO claim must not be empty",
            ExitCode::Usage,
        ));
    }
    let path = layout.vo_dir().join(format!("{id}.yaml"));
    if path.exists() {
        return Err(failure(
            "E-OP-001",
            format!("VO `{id}` already exists"),
            ExitCode::Usage,
        ));
    }
    for req in requirements {
        validate_id(req, "REQ-")?;
        read_req(layout, req).map_err(store_failure)?;
    }
    if let Some(parent) = &parent {
        validate_id(parent, "VO-")?;
        read_vo(layout, parent).map_err(store_failure)?;
    }
    let spec_refs = spec_refs_from_args(layout, specs, sections)?;
    let dimensions = parse_dimensions(dimensions)?;
    if let Some(policy) = &policy {
        if !matches!(
            policy.as_str(),
            "independent-axes" | "full-product" | "explicit"
        ) {
            return Err(failure(
                "E-OP-001",
                "unsupported VO coverage policy",
                ExitCode::Usage,
            ));
        }
    }
    let now = now_rfc3339();
    let record = VoRecord {
        id: VoId::new(id),
        parent: parent.map(VoId::new),
        requirements: requirements.iter().cloned().map(ReqId::new).collect(),
        spec_refs,
        claim: claim.to_owned(),
        dimensions,
        coverage_policy: policy,
        representative_cases: Vec::new(),
        status: "draft".to_owned(),
        created: now.clone(),
        updated: now,
    };
    write_atomic(&path, &record.to_yaml()).map_err(store_failure)?;
    Ok(serde_json::to_value(record).expect("record"))
}

fn edit_vo(
    layout: &VerifyLayout,
    id: &str,
    claim: Option<String>,
    parent: Option<String>,
    status: Option<String>,
) -> CommandResult {
    validate_id(id, "VO-")?;
    if claim.is_none() && parent.is_none() && status.is_none() {
        return Err(failure(
            "E-OP-001",
            "VO edit needs at least one field",
            ExitCode::Usage,
        ));
    }
    let path = layout.vo_dir().join(format!("{id}.yaml"));
    let mut record = read_vo(layout, id).map_err(store_failure)?;
    if let Some(claim) = claim {
        if claim.trim().is_empty() {
            return Err(failure(
                "E-OP-001",
                "VO claim must not be empty",
                ExitCode::Usage,
            ));
        }
        record.claim = claim;
    }
    if let Some(parent) = parent {
        validate_id(&parent, "VO-")?;
        read_vo(layout, &parent).map_err(store_failure)?;
        record.parent = Some(VoId::new(parent));
    }
    if let Some(status) = status {
        if !matches!(status.as_str(), "draft" | "approved") {
            return Err(failure(
                "E-OP-001",
                "VO status must be draft or approved",
                ExitCode::Usage,
            ));
        }
        record.status = status;
    }
    record.updated = now_rfc3339();
    write_atomic(&path, &record.to_yaml()).map_err(store_failure)?;
    Ok(serde_json::to_value(record).expect("record"))
}

fn list_vos(layout: &VerifyLayout, req: Option<&str>, status: Option<&str>) -> CommandResult {
    let ids = read_record_ids_for(&layout.vo_dir())?;
    let mut records = Vec::new();
    for id in ids {
        let record = read_vo(layout, &id).map_err(store_failure)?;
        if let Some(req) = req {
            if !record
                .requirements
                .iter()
                .any(|value| value.as_str() == req)
            {
                continue;
            }
        }
        let effective = effective_vo_status(layout, &record);
        if status.is_some_and(|wanted| wanted != effective) {
            continue;
        }
        records.push(serde_json::json!({
            "id": record.id,
            "claim": record.claim,
            "status": effective,
            "stored_status": record.status,
        }));
    }
    Ok(serde_json::json!({ "items": records }))
}

fn show_vo(layout: &VerifyLayout, id: &str) -> CommandResult {
    validate_id(id, "VO-")?;
    let record = read_vo(layout, id).map_err(store_failure)?;
    let effective = effective_vo_status(layout, &record);
    let mut value = serde_json::to_value(record).expect("record");
    if let Some(object) = value.as_object_mut() {
        object.insert("effective_status".to_owned(), serde_json::json!(effective));
    }
    Ok(value)
}

fn expand_vo(layout: &VerifyLayout, id: &str, dry_run: bool) -> CommandResult {
    validate_id(id, "VO-")?;
    let parent = read_vo(layout, id).map_err(store_failure)?;
    let policy = parent.coverage_policy.as_deref().ok_or_else(|| {
        failure(
            "E-OP-001",
            "VO has no coverage_policy for expansion",
            ExitCode::Usage,
        )
    })?;
    let combinations = dimension_combinations(&parent.dimensions, policy)?;
    let mut children = Vec::new();
    for combination in combinations {
        let suffix = combination
            .iter()
            .map(|value| slug(value))
            .collect::<Vec<_>>()
            .join("-");
        let child_id = format!("{}-{}", parent.id, suffix);
        let child_path = layout.vo_dir().join(format!("{child_id}.yaml"));
        let mut created = false;
        if !dry_run && !child_path.exists() {
            let now = now_rfc3339();
            let child = VoRecord {
                id: VoId::new(&child_id),
                parent: Some(parent.id.clone()),
                requirements: parent.requirements.clone(),
                spec_refs: parent.spec_refs.clone(),
                claim: format!("{} [{}]", parent.claim, combination.join(", ")),
                dimensions: Vec::new(),
                coverage_policy: None,
                representative_cases: Vec::new(),
                status: "draft".to_owned(),
                created: now.clone(),
                updated: now,
            };
            write_atomic(&child_path, &child.to_yaml()).map_err(store_failure)?;
            created = true;
        }
        children.push(serde_json::json!({
            "id": child_id,
            "combination": combination,
            "created": created,
        }));
    }
    Ok(serde_json::json!({ "parent": parent.id, "dry_run": dry_run, "children": children }))
}

fn approve_vo(
    layout: &VerifyLayout,
    id: &str,
    approver_kind: &str,
    approver_id: &str,
    model: Option<String>,
    basis: &[String],
) -> CommandResult {
    validate_id(id, "VO-")?;
    if !matches!(approver_kind, "human" | "agent") || approver_id.trim().is_empty() {
        return Err(failure("E-OP-001", "invalid approver", ExitCode::Usage));
    }
    let vo_path = layout.vo_dir().join(format!("{id}.yaml"));
    let vo = read_vo(layout, id).map_err(store_failure)?;
    let subject_hash = ContentHash::from_text(&read_text(&vo_path).map_err(store_failure)?);
    let approval = ApprovalRecord {
        id: new_record_id(),
        subject: vo.id,
        subject_hash,
        approver: Approver {
            kind: approver_kind.to_owned(),
            id: approver_id.to_owned(),
            model,
        },
        basis: basis.to_vec(),
        approved_at: now_rfc3339(),
    };
    let path = layout.approvals_dir().join(format!("{}.yaml", approval.id));
    write_atomic(&path, &approval.to_yaml()).map_err(store_failure)?;
    Ok(serde_json::to_value(approval).expect("record"))
}

fn dimension_combinations(
    dimensions: &[Dimension],
    policy: &str,
) -> Result<Vec<Vec<String>>, CommandFailure> {
    if dimensions.is_empty() {
        return Ok(Vec::new());
    }
    match policy {
        "independent-axes" => Ok(dimensions
            .iter()
            .flat_map(|dimension| {
                dimension
                    .partitions
                    .iter()
                    .map(|partition| vec![format!("{}-{}", dimension.name, partition)])
            })
            .collect()),
        "full-product" => {
            let mut combinations = vec![Vec::new()];
            for dimension in dimensions {
                let mut next = Vec::new();
                for prefix in &combinations {
                    for partition in &dimension.partitions {
                        let mut combination = prefix.clone();
                        combination.push(format!("{}-{}", dimension.name, partition));
                        next.push(combination);
                    }
                }
                combinations = next;
            }
            Ok(combinations)
        }
        "explicit" => Err(failure(
            "E-OP-001",
            "explicit expansion requires combinations in a future record schema",
            ExitCode::Usage,
        )),
        _ => Err(failure(
            "E-OP-001",
            "unsupported VO coverage policy",
            ExitCode::Usage,
        )),
    }
}

fn parse_dimensions(values: &[String]) -> Result<Vec<Dimension>, CommandFailure> {
    let mut dimensions = Vec::new();
    for value in values {
        let Some((name, partitions)) = value.split_once('=') else {
            return Err(failure(
                "E-OP-001",
                format!("dimension `{value}` must be name=p1,p2"),
                ExitCode::Usage,
            ));
        };
        let partitions = partitions
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if name.trim().is_empty() || partitions.is_empty() {
            return Err(failure(
                "E-OP-001",
                "dimension name and partitions are required",
                ExitCode::Usage,
            ));
        }
        dimensions.push(Dimension {
            name: name.trim().to_owned(),
            partitions,
        });
    }
    Ok(dimensions)
}

fn spec_refs_from_args(
    layout: &VerifyLayout,
    specs: &[String],
    sections: &[String],
) -> Result<Vec<SpecRef>, CommandFailure> {
    if !sections.is_empty() && sections.len() != specs.len() {
        return Err(failure(
            "E-OP-001",
            "--spec and --section counts must match",
            ExitCode::Usage,
        ));
    }
    specs
        .iter()
        .enumerate()
        .map(|(index, spec)| {
            validate_id(spec, "SPEC-")?;
            read_spec(layout, spec).map_err(store_failure)?;
            Ok(SpecRef {
                spec: SpecId::new(spec),
                section: sections.get(index).cloned().unwrap_or_default(),
            })
        })
        .collect()
}

fn effective_vo_status(layout: &VerifyLayout, record: &VoRecord) -> String {
    let path = layout.vo_dir().join(format!("{}.yaml", record.id));
    let Ok(text) = read_text(&path) else {
        return "draft".to_owned();
    };
    let hash = ContentHash::from_text(&text);
    let Ok(entries) = fs::read_dir(layout.approvals_dir()) else {
        return "draft".to_owned();
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("yaml") {
            continue;
        }
        if let Ok(approval) = read_approval(&path) {
            if approval.subject == record.id && approval.subject_hash == hash {
                return "approved".to_owned();
            }
        }
    }
    "draft".to_owned()
}

fn read_record_ids_for(directory: &Path) -> CommandResult<Vec<String>> {
    let entries = fs::read_dir(directory)
        .map_err(|error| failure("E-CORE-001", error.to_string(), ExitCode::Internal))?;
    let mut ids = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|error| failure("E-CORE-001", error.to_string(), ExitCode::Internal))?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) == Some("yaml") {
            if let Some(id) = path.file_stem().and_then(|value| value.to_str()) {
                ids.push(id.to_owned());
            }
        }
    }
    ids.sort();
    Ok(ids)
}

fn resolve_root(project: &Path, format: OutputFormat, quiet: bool) -> Result<PathBuf, ExitCode> {
    match find_project_root(&absolute_path(project)) {
        Ok(root) => Ok(root),
        Err(error) => {
            let (diagnostic, code) = store_error_with_code(error, ExitCode::Usage);
            emit(
                format,
                quiet,
                JsonEnvelope::new(false, serde_json::Value::Null, vec![diagnostic]),
            );
            Err(code)
        }
    }
}

fn safe_project_path(root: &Path, relative: &Path) -> Result<PathBuf, CommandFailure> {
    if relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(failure(
            "E-OP-001",
            "path must be project-relative",
            ExitCode::Usage,
        ));
    }
    let path = root.join(relative);
    if !path.is_file() {
        return Err(failure(
            "E-OP-001",
            format!("file does not exist: {}", path.display()),
            ExitCode::Usage,
        ));
    }
    Ok(path)
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn validate_id(id: &str, prefix: &str) -> Result<(), CommandFailure> {
    if !id.starts_with(prefix) || id.len() <= prefix.len() || id.chars().any(char::is_whitespace) {
        return Err(failure(
            "E-OP-001",
            format!("id must start with `{prefix}` and contain no whitespace"),
            ExitCode::Usage,
        ));
    }
    Ok(())
}

fn slug(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
}

fn absolute_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn store_failure(error: StoreError) -> CommandFailure {
    failure("E-CORE-001", error.to_string(), ExitCode::Internal)
}

fn store_error_with_code(error: StoreError, default: ExitCode) -> (Diagnostic, ExitCode) {
    match error {
        StoreError::AlreadyInitialized(path) => (
            Diagnostic::error(
                "E-OP-001",
                format!(".verify already exists at {}", path.display()),
            ),
            ExitCode::Usage,
        ),
        StoreError::NotInitialized(path) => (
            Diagnostic::error(
                "E-OP-001",
                format!(
                    "could not find a .verify directory above {}",
                    path.display()
                ),
            ),
            ExitCode::Usage,
        ),
        other => (Diagnostic::error("E-CORE-001", other.to_string()), default),
    }
}

fn failure(code: &str, message: impl Into<String>, exit_code: ExitCode) -> CommandFailure {
    CommandFailure {
        diagnostic: Box::new(Diagnostic::error(code, message)),
        code: exit_code,
    }
}

fn operation_failure(diagnostic: Diagnostic) -> CommandFailure {
    let code = if diagnostic.code == "E-CORE-001" {
        ExitCode::Internal
    } else {
        ExitCode::Usage
    };
    CommandFailure {
        diagnostic: Box::new(diagnostic),
        code,
    }
}

fn emit<T: Serialize>(format: OutputFormat, quiet: bool, envelope: JsonEnvelope<T>) {
    if quiet {
        return;
    }
    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&envelope).expect("JSON serialization")
            );
        }
        OutputFormat::Text => emit_text(envelope),
    }
}

fn emit_text<T: Serialize>(envelope: JsonEnvelope<T>) {
    println!("{}", if envelope.ok { "OK" } else { "NG" });
    if let Ok(value) = serde_json::to_value(envelope.data) {
        if let Some(summary) = value.get("summary") {
            println!("summary: {summary}");
        } else if !value.is_null() {
            println!("data: {value}");
        }
    }
    for diagnostic in envelope.diagnostics {
        println!(
            "{} {}: {}",
            match diagnostic.severity {
                vtest_model::DiagnosticSeverity::Error => "error",
                vtest_model::DiagnosticSeverity::Warning => "warning",
            },
            diagnostic.code,
            diagnostic.message
        );
        for candidate in diagnostic.candidates {
            println!("  candidate: {candidate}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn root() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("vtest-cli-{suffix}"));
        assert_eq!(
            run(Cli {
                project: root.clone(),
                format: OutputFormat::Json,
                quiet: true,
                command: Command::Init {
                    name: Some("calc".to_owned()),
                },
            }) as u8,
            0
        );
        root
    }

    #[test]
    fn init_creates_project_and_second_init_is_usage_error() {
        let root = root();
        assert!(root.join(".verify/forms/rust-unit-function.yaml").is_file());
        assert!(root.join(".verify/forms/rust-integration.yaml").is_file());
        assert_eq!(
            run(Cli {
                project: root.clone(),
                format: OutputFormat::Json,
                quiet: true,
                command: Command::Init { name: None },
            }) as u8,
            2
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn approval_is_derived_and_edit_makes_it_draft() {
        let root = root();
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join("docs/spec.md"), "calculator\n").unwrap();
        let invoke = |command| {
            run(Cli {
                project: root.clone(),
                format: OutputFormat::Json,
                quiet: true,
                command,
            })
        };
        assert_eq!(
            invoke(Command::Spec {
                command: SpecCommand::Add {
                    id: "SPEC-CALC".to_owned(),
                    path: PathBuf::from("docs/spec.md"),
                    kind: "document".to_owned(),
                    title: None,
                    note: None,
                    update: false,
                },
            }) as u8,
            0
        );
        assert_eq!(
            invoke(Command::Req {
                command: ReqCommand::Add {
                    id: "REQ-CALC".to_owned(),
                    summary: "calculator works".to_owned(),
                    parent: None,
                    specs: vec!["SPEC-CALC".to_owned()],
                    sections: vec!["1".to_owned()],
                },
            }) as u8,
            0
        );
        assert_eq!(
            invoke(Command::Vo {
                command: VoCommand::Add {
                    id: "VO-CALC".to_owned(),
                    claim: "addition works".to_owned(),
                    requirements: vec!["REQ-CALC".to_owned()],
                    parent: None,
                    specs: vec!["SPEC-CALC".to_owned()],
                    sections: vec!["1".to_owned()],
                    dimensions: Vec::new(),
                    policy: None,
                },
            }) as u8,
            0
        );
        assert_eq!(
            invoke(Command::Vo {
                command: VoCommand::Approve {
                    id: "VO-CALC".to_owned(),
                    approver_kind: "human".to_owned(),
                    approver_id: "reviewer".to_owned(),
                    model: None,
                    basis: Vec::new(),
                },
            }) as u8,
            0
        );
        let layout = VerifyLayout::new(&root);
        let approved = read_vo(&layout, "VO-CALC").unwrap();
        assert_eq!(effective_vo_status(&layout, &approved), "approved");
        assert_eq!(
            invoke(Command::Vo {
                command: VoCommand::Edit {
                    id: "VO-CALC".to_owned(),
                    claim: Some("changed claim".to_owned()),
                    parent: None,
                    status: None,
                },
            }) as u8,
            0
        );
        let edited = read_vo(&layout, "VO-CALC").unwrap();
        assert_eq!(effective_vo_status(&layout, &edited), "draft");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn structured_create_generates_a_scannable_test() {
        let root = root();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"structured-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
        )
        .unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "pub enum CalcError { Overflow, Underflow }\npub fn add(left: i32, right: i32) -> i32 { left + right }\npub fn subtract(left: i32, right: i32) -> i32 { left - right }\n",
        )
        .unwrap();
        fs::write(
            root.join(".verify/vo/VO-CALC-ADD.yaml"),
            "id: VO-CALC-ADD\nparent: null\nrequirements: []\nspec_refs: []\nclaim: addition works\ndimensions: []\ncoverage_policy: null\nrepresentative_cases: []\nstatus: draft\ncreated: '2026-01-01'\nupdated: '2026-01-01'\n",
        )
        .unwrap();
        fs::write(
            root.join("answers.yaml"),
            "form: rust-unit-function\nanswers:\n  target: src/lib.rs::add\n  covers: [VO-CALC-ADD]\n  behavior: adds two integers\n  test_kind: normal\n  input: two integers\n  expect: their sum\n  fn_name: adds_two_integers\n",
        )
        .unwrap();

        let parsed_answers = read_form_answers(&root.join("answers.yaml")).unwrap();
        assert_eq!(
            create_test(&root, "rust-unit-function", &parsed_answers, None, true)
                .unwrap()
                .test_id,
            "TEST-CALC-001"
        );
        let before_dry_run = fs::read_to_string(root.join("src/lib.rs")).unwrap();
        assert_eq!(
            run(Cli {
                project: root.clone(),
                format: OutputFormat::Json,
                quiet: true,
                command: Command::Test {
                    command: TestCommand::Create {
                        form: "rust-unit-function".to_owned(),
                        answers: PathBuf::from("answers.yaml"),
                        id: Some("TEST-CALC-001".to_owned()),
                        dry_run: true,
                    },
                },
            }) as u8,
            0
        );
        assert_eq!(
            fs::read_to_string(root.join("src/lib.rs")).unwrap(),
            before_dry_run
        );

        assert_eq!(
            run(Cli {
                project: root.clone(),
                format: OutputFormat::Json,
                quiet: true,
                command: Command::Test {
                    command: TestCommand::Create {
                        form: "rust-unit-function".to_owned(),
                        answers: PathBuf::from("answers.yaml"),
                        id: Some("TEST-CALC-001".to_owned()),
                        dry_run: false,
                    },
                },
            }) as u8,
            0
        );
        let scan = scan_project(&root).unwrap();
        let created = scan
            .tests
            .iter()
            .find(|test| test.id.as_str() == "TEST-CALC-001")
            .expect("generated test is scanned");
        assert_eq!(created.intent, "adds two integers");
        assert_eq!(created.location.file, "src/lib.rs");
        assert!(fs::read_to_string(root.join("src/lib.rs"))
            .unwrap()
            .contains("todo!(\"implement test body\")"));

        fs::write(
            root.join(".verify/vo/VO-CALC-ALT.yaml"),
            "id: VO-CALC-ALT\nparent: null\nrequirements: []\nspec_refs: []\nclaim: alternate addition check\ndimensions: []\ncoverage_policy: null\nrepresentative_cases: []\nstatus: draft\ncreated: '2026-01-01'\nupdated: '2026-01-01'\n",
        )
        .unwrap();
        fs::write(
            root.join("second-answers.yaml"),
            "form: rust-unit-function\nanswers:\n  target: src/lib.rs::add\n  covers: [VO-CALC-ADD]\n  behavior: second addition check\n  test_kind: boundary\n  input: zeroes\n  expect: zero\n  fn_name: adds_zeroes\n",
        )
        .unwrap();
        assert_eq!(
            run(Cli {
                project: root.clone(),
                format: OutputFormat::Json,
                quiet: true,
                command: Command::Test {
                    command: TestCommand::Create {
                        form: "rust-unit-function".to_owned(),
                        answers: PathBuf::from("second-answers.yaml"),
                        id: Some("TEST-CALC-002".to_owned()),
                        dry_run: false,
                    },
                },
            }) as u8,
            0
        );
        let with_free_doc = fs::read_to_string(root.join("src/lib.rs"))
            .unwrap()
            .replace(
                "/// @vtest.intent adds two integers\n",
                "/// @vtest.intent adds two integers\n/// retained free-form explanation\n",
            );
        fs::write(root.join("src/lib.rs"), with_free_doc).unwrap();
        let before_edit = scan_project(&root).unwrap();
        let other_hash = before_edit
            .tests
            .iter()
            .find(|test| test.id.as_str() == "TEST-CALC-002")
            .unwrap()
            .content_hash
            .clone();
        let edit_command = || Command::Test {
            command: TestCommand::Edit {
                id: "TEST-CALC-001".to_owned(),
                answers: None,
                set: vec!["covers=VO-CALC-ALT".to_owned()],
                body_file: None,
                dry_run: false,
            },
        };
        let before_edit_dry_run = fs::read_to_string(root.join("src/lib.rs")).unwrap();
        let dry_edit_command = Command::Test {
            command: TestCommand::Edit {
                id: "TEST-CALC-001".to_owned(),
                answers: None,
                set: vec!["covers=VO-CALC-ALT".to_owned()],
                body_file: None,
                dry_run: true,
            },
        };
        assert_eq!(
            run(Cli {
                project: root.clone(),
                format: OutputFormat::Json,
                quiet: true,
                command: dry_edit_command,
            }) as u8,
            0
        );
        assert_eq!(
            fs::read_to_string(root.join("src/lib.rs")).unwrap(),
            before_edit_dry_run
        );
        assert_eq!(
            run(Cli {
                project: root.clone(),
                format: OutputFormat::Json,
                quiet: true,
                command: edit_command(),
            }) as u8,
            0
        );
        let after_edit = scan_project(&root).unwrap();
        assert_eq!(
            after_edit
                .tests
                .iter()
                .find(|test| test.id.as_str() == "TEST-CALC-001")
                .unwrap()
                .covers[0]
                .as_str(),
            "VO-CALC-ALT"
        );
        assert_eq!(
            after_edit
                .tests
                .iter()
                .find(|test| test.id.as_str() == "TEST-CALC-002")
                .unwrap()
                .content_hash,
            other_hash
        );
        let once = fs::read_to_string(root.join("src/lib.rs")).unwrap();
        assert!(once.contains("/// retained free-form explanation"));
        assert_eq!(
            run(Cli {
                project: root.clone(),
                format: OutputFormat::Json,
                quiet: true,
                command: edit_command(),
            }) as u8,
            0
        );
        assert_eq!(
            fs::read_to_string(root.join("src/lib.rs")).unwrap(),
            once,
            "reapplying the same desired state must be idempotent"
        );

        fs::write(root.join("invalid-body.rs"), "let = ;").unwrap();
        assert_eq!(
            run(Cli {
                project: root.clone(),
                format: OutputFormat::Json,
                quiet: true,
                command: Command::Test {
                    command: TestCommand::Edit {
                        id: "TEST-CALC-001".to_owned(),
                        answers: None,
                        set: Vec::new(),
                        body_file: Some(PathBuf::from("invalid-body.rs")),
                        dry_run: false,
                    },
                },
            }) as u8,
            2
        );
        assert_eq!(fs::read_to_string(root.join("src/lib.rs")).unwrap(), once);

        fs::write(root.join("body.rs"), "assert_eq!(add(2, 3), 5);").unwrap();
        assert_eq!(
            run(Cli {
                project: root.clone(),
                format: OutputFormat::Json,
                quiet: true,
                command: Command::Test {
                    command: TestCommand::Edit {
                        id: "TEST-CALC-001".to_owned(),
                        answers: None,
                        set: Vec::new(),
                        body_file: Some(PathBuf::from("body.rs")),
                        dry_run: false,
                    },
                },
            }) as u8,
            0
        );
        let body_scan = scan_project(&root).unwrap();
        assert_eq!(
            body_scan
                .tests
                .iter()
                .find(|test| test.id.as_str() == "TEST-CALC-002")
                .unwrap()
                .content_hash,
            other_hash
        );
        assert!(fs::read_to_string(root.join("src/lib.rs"))
            .unwrap()
            .contains("assert_eq!(add(2, 3), 5);"));

        fs::write(
            root.join("integration-answers.yaml"),
            "form: rust-integration\nanswers:\n  targets: [src/lib.rs::add, src/lib.rs::subtract]\n  covers: [VO-CALC-ADD]\n  behavior: combines calculator operations\n  test_kind: normal\n  input: two integers\n  expect: consistent arithmetic\n  fn_name: combines_operations\n  file: src/lib.rs\n",
        )
        .unwrap();
        assert_eq!(
            run(Cli {
                project: root.clone(),
                format: OutputFormat::Json,
                quiet: true,
                command: Command::Test {
                    command: TestCommand::Create {
                        form: "rust-integration".to_owned(),
                        answers: PathBuf::from("integration-answers.yaml"),
                        id: Some("TEST-CALC-004".to_owned()),
                        dry_run: false,
                    },
                },
            }) as u8,
            0
        );
        let integration_scan = scan_project(&root).unwrap();
        let integration = integration_scan
            .tests
            .iter()
            .find(|test| test.id.as_str() == "TEST-CALC-004")
            .unwrap();
        assert_eq!(integration.additional_targets.len(), 1);
        assert_eq!(integration.kind.as_deref(), Some("integration-normal"));
        assert!(query_tests(&integration_scan, "src/lib.rs::subtract")
            .unwrap()
            .iter()
            .any(|test| test.id.as_str() == "TEST-CALC-004"));

        fs::write(
            root.join("bad-answers.yaml"),
            "form: rust-unit-function\nanswers:\n  target: src/lib.rs::ad\n  covers: [VO-CALC-ADD]\n  behavior: typo\n  test_kind: normal\n  input: two integers\n  expect: their sum\n  fn_name: typo_is_rejected\n",
        )
        .unwrap();
        let bad_answers = read_form_answers(&root.join("bad-answers.yaml")).unwrap();
        let error = create_test(
            &root,
            "rust-unit-function",
            &bad_answers,
            Some("TEST-CALC-003"),
            true,
        )
        .expect_err("unknown symbol must be rejected");
        assert_eq!(error.code, "E-OP-001");
        assert_eq!(error.candidates, vec!["src/lib.rs::add"]);

        fs::write(
            root.join("bad-enum-answers.yaml"),
            "form: rust-unit-function\nanswers:\n  target: src/lib.rs::add\n  covers: [VO-CALC-ADD]\n  behavior: invalid enum variant\n  test_kind: error\n  input: large integers\n  expect: CalcError::Missing\n  fn_name: enum_variant_is_rejected\n",
        )
        .unwrap();
        let bad_enum_answers = read_form_answers(&root.join("bad-enum-answers.yaml")).unwrap();
        let enum_error = create_test(
            &root,
            "rust-unit-function",
            &bad_enum_answers,
            Some("TEST-CALC-005"),
            true,
        )
        .expect_err("known enum with an unknown variant must be rejected");
        assert_eq!(enum_error.code, "E-OP-001");
        assert_eq!(
            enum_error.candidates,
            vec!["CalcError::Overflow", "CalcError::Underflow"]
        );
        fs::remove_dir_all(root).unwrap();
    }
}
