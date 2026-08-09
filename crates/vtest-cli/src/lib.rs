//! CLI application layer shared by the future MCP adapter.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
    process::Command as ProcessCommand,
};

use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use vtest_adapter_rust::{
    create_test, edit_test, list_tests_from_scan, parse_test_set_values, query_tests_from_scan,
    show_test_from_scan,
};
use vtest_audit::{audit_static, persist_static_audits, AuditOptions, AuditVerdict};
use vtest_exec::{run_tests, RunnableTest};
use vtest_model::{
    ContentHash, Diagnostic, ExitCode, JsonEnvelope, ReqId, Revision, ScanSummary, SourceFunction,
    SpecId, TestEntity, VoId,
};
use vtest_scan::{scan_project, ScanResult};
use vtest_store::{
    find_project_root, init_project, is_valid_ulid, load_config, new_record_id, now_rfc3339,
    read_approval, read_form_answers, read_req, read_spec, read_text, read_vo, write_atomic,
    write_new_record, yaml_scalar_value, ApprovalBasis, ApprovalRecord, Approver, Dimension,
    ReqRecord, SpecRecord, SpecRef, StoreError, VerifyLayout, VoRecord,
};
use vtest_verify::{verify_project_scoped, EntityScope};

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
    /// Start the MCP stdio server.
    Mcp,
}

#[derive(Clone, Debug, Subcommand)]
pub enum AuditCommand {
    Static {
        #[arg(long)]
        test: Option<String>,
        /// Audit every registered test. This is also the legacy default when no selector is given.
        #[arg(long)]
        all: bool,
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
        #[arg(long = "combination")]
        combinations: Vec<String>,
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
    pub tests: Vec<serde_json::Value>,
    pub sources: Vec<vtest_model::SourceFunction>,
}

impl From<ScanResult> for ScanData {
    fn from(value: ScanResult) -> Self {
        Self {
            summary: value.summary,
            tests: value
                .tests
                .iter()
                .map(|test| {
                    vtest_adapter_rust::encode_test_wire(test)
                        .unwrap_or_else(|_| serde_json::to_value(test).unwrap_or_default())
                })
                .collect(),
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
        Command::Mcp => run_mcp(&project, format, quiet),
    }
}

fn run_mcp(project: &Path, format: OutputFormat, quiet: bool) -> ExitCode {
    match vtest_mcp::serve(project) {
        Ok(()) => ExitCode::Ok,
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
            ExitCode::Internal
        }
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
        TestCommand::Show { id } => {
            show_test_from_scan(&root, &scan.tests, &scan.sources, &scan.diagnostics, &id)
                .and_then(|test| {
                    serde_json::to_value(test)
                        .map_err(|error| Diagnostic::error("E-CORE-001", error.to_string()))
                })
                .map_err(operation_failure)
        }
        TestCommand::List { vo, unregistered } => serde_json::to_value(list_tests_from_scan(
            &scan.tests,
            &scan.diagnostics,
            vo.as_deref(),
            unregistered,
        ))
        .map_err(|error| failure("E-CORE-001", error.to_string(), ExitCode::Internal)),
        TestCommand::Query { source } => {
            query_tests_from_scan(&scan.tests, &scan.sources, &scan.diagnostics, &source)
                .and_then(|tests| {
                    serde_json::to_value(tests)
                        .map_err(|error| Diagnostic::error("E-CORE-001", error.to_string()))
                })
                .map_err(operation_failure)
        }
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
            let (diagnostic, code) = match error {
                vtest_scan::ScanError::Adapter { code, message } => {
                    (Diagnostic::error(code, message), ExitCode::Usage)
                }
                other => (
                    Diagnostic::error("E-CORE-001", other.to_string()),
                    ExitCode::Internal,
                ),
            };
            emit(
                format,
                quiet,
                JsonEnvelope::new(false, serde_json::Value::Null, vec![diagnostic]),
            );
            code
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
        ReqCommand::List { tree } => list_reqs(&layout, tree),
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
            combinations,
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
            &combinations,
        ),
        VoCommand::Edit {
            id,
            claim,
            parent,
            status,
        } => edit_vo(&layout, &id, claim, parent, status),
        VoCommand::List { tree, req, status } => {
            list_vos(&layout, tree, req.as_deref(), status.as_deref())
        }
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
        AuditCommand::Static { test, all } => run_audit_static(project, test, all, format, quiet),
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
    all: bool,
    format: OutputFormat,
    quiet: bool,
) -> ExitCode {
    let root = match resolve_root(project, format, quiet) {
        Ok(root) => root,
        Err(code) => return code,
    };
    let layout = VerifyLayout::new(&root);
    if test.is_some() && all {
        emit(
            format,
            quiet,
            JsonEnvelope::new(
                false,
                serde_json::Value::Null,
                vec![Diagnostic::error(
                    "E-OP-001",
                    "audit static accepts either --test or --all, not both",
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
    let diagnostics = merge_diagnostics(
        scan.diagnostics,
        summary
            .audits
            .iter()
            .flat_map(|audit| audit.diagnostics.iter().cloned()),
    );
    emit(
        format,
        quiet,
        JsonEnvelope::new(
            !has_non_pass,
            serde_json::to_value(&summary).expect("audit summary"),
            diagnostics,
        ),
    );
    if has_non_pass {
        ExitCode::VerificationFailed
    } else {
        ExitCode::Ok
    }
}

fn merge_diagnostics(
    scan_diagnostics: Vec<Diagnostic>,
    audit_diagnostics: impl IntoIterator<Item = Diagnostic>,
) -> Vec<Diagnostic> {
    let mut diagnostics = scan_diagnostics;
    diagnostics.extend(audit_diagnostics);
    diagnostics.sort_by(|left, right| {
        let left_location = left.location.as_deref();
        let right_location = right.location.as_deref();
        let left_severity = match left.severity {
            vtest_model::DiagnosticSeverity::Error => 0,
            vtest_model::DiagnosticSeverity::Warning => 1,
        };
        let right_severity = match right.severity {
            vtest_model::DiagnosticSeverity::Error => 0,
            vtest_model::DiagnosticSeverity::Warning => 1,
        };
        (
            left_severity,
            &left.code,
            &left.message,
            &left.candidates,
            left_location.map(|location| location.file.as_str()),
            left_location.map(|location| location.function.as_str()),
            left_location.map_or(0, |location| location.start_line),
            left_location.map_or(0, |location| location.end_line),
            left_location.map_or(0, |location| location.start_byte),
            left_location.map_or(0, |location| location.end_byte),
        )
            .cmp(&(
                right_severity,
                &right.code,
                &right.message,
                &right.candidates,
                right_location.map(|location| location.file.as_str()),
                right_location.map(|location| location.function.as_str()),
                right_location.map_or(0, |location| location.start_line),
                right_location.map_or(0, |location| location.end_line),
                right_location.map_or(0, |location| location.start_byte),
                right_location.map_or(0, |location| location.end_byte),
            ))
    });
    diagnostics
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
    let supported = matches!(
        kind,
        "test-semantic" | "vo-coverage" | "impl-consistency" | "spec-coverage"
    );
    if !supported {
        return selector_error(format!("unsupported audit bundle kind {kind}"));
    }
    let selector_valid = match kind {
        "test-semantic" => selector_count == 1 && test_id.is_some(),
        "vo-coverage" => selector_count == 1 && (vo_id.is_some() || req_id.is_some()),
        "impl-consistency" => selector_count == 1 && (test_id.is_some() || vo_id.is_some()),
        "spec-coverage" => selector_count == 1 && req_id.is_some(),
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
        write_new_record(&audit_path, &audit_yaml)
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
            let target = test
                .targets
                .first()
                .and_then(|target| find_target_source(scan, target))
                .ok_or_else(|| {
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
            let analysis_source = source_slice(root, &test.location)?;
            let analysis_hash = ContentHash::from_domain_fields(
                "vtest:static-analysis-source:v1",
                &[
                    ("test", test.content_hash.as_str().as_bytes()),
                    ("source", analysis_source.as_bytes()),
                ],
            );
            let analysis_locator = format!("{}::{}", test.location.file, test.location.function);
            subjects.push(subject_value(
                "static_analysis_source",
                None,
                Some(&analysis_locator),
                &analysis_hash,
            ));
            subjects.push(subject_value(
                "target",
                None,
                Some(&target.target.value),
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
        "spec-coverage" => {
            let req_id = req_id.expect("validated selector");
            let Ok(record) = read_req(layout, req_id) else {
                return Ok(serde_json::json!({
                    "skipped": true,
                    "reason": format!("REQ {req_id} was not found"),
                    "kind": kind,
                    "req": req_id,
                }));
            };
            let path = layout.req_dir().join(format!("{req_id}.yaml"));
            let hash = fs::read_to_string(path)
                .map(|text| ContentHash::from_text(&text))
                .unwrap_or_else(|_| ContentHash::from_text(""));
            Ok(serde_json::json!({
                "bundle_id": new_record_id(),
                "kind": kind,
                "generated_at": now_rfc3339(),
                "revision": git_revision_json(root),
                "requirements": [serde_json::to_value(record).expect("REQ record")],
                "subjects": [subject_value("req", Some(req_id), None, &hash)],
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
            deduplicate_subjects(&mut subjects);
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
                let target = test
                    .targets
                    .first()
                    .and_then(|target| find_target_source(scan, target))
                    .ok_or_else(|| {
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
                    Some(&target.target.value),
                    &target.content_hash,
                ));
                for (vo, subject) in test.covers.iter().zip(vos.iter()) {
                    let hash = subject["content_hash"]
                        .as_str()
                        .and_then(|hash| hash.parse().ok())
                        .unwrap_or_else(|| ContentHash::from_text(""));
                    subjects.push(subject_value("vo", Some(vo.as_str()), None, &hash));
                    if let Ok(vo_record) = read_vo(layout, vo.as_str()) {
                        for spec_ref in vo_record.spec_refs {
                            if let Ok(spec) = read_spec(layout, spec_ref.spec.as_str()) {
                                subjects.push(subject_value(
                                    "spec",
                                    Some(spec.id.as_str()),
                                    None,
                                    &spec.sha256,
                                ));
                            }
                        }
                    }
                }
                deduplicate_subjects(&mut subjects);
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
                    if let Some(target) = test
                        .targets
                        .first()
                        .and_then(|target| find_target_source(scan, target))
                    {
                        subjects.push(subject_value(
                            "test",
                            Some(test.id.as_str()),
                            None,
                            &test.content_hash,
                        ));
                        subjects.push(subject_value(
                            "target",
                            None,
                            Some(&target.target.value),
                            &target.content_hash,
                        ));
                        for vo_id in &test.covers {
                            if let Ok(vo_record) = read_vo(layout, vo_id.as_str()) {
                                for spec_ref in vo_record.spec_refs {
                                    if let Ok(spec) = read_spec(layout, spec_ref.spec.as_str()) {
                                        subjects.push(subject_value(
                                            "spec",
                                            Some(spec.id.as_str()),
                                            None,
                                            &spec.sha256,
                                        ));
                                    }
                                }
                            }
                        }
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
                deduplicate_subjects(&mut subjects);
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

fn find_target_source<'a>(
    scan: &'a ScanResult,
    target: &vtest_model::NeutralTargetRef,
) -> Option<&'a SourceFunction> {
    scan.sources.iter().find(|source| {
        source.target.adapter == target.adapter
            && (source.target.value == target.value
                || source
                    .src_id
                    .as_ref()
                    .is_some_and(|candidate| candidate.as_str() == target.value))
    })
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
        "locator": target.target.value,
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

fn deduplicate_subjects(subjects: &mut Vec<serde_json::Value>) {
    let mut unique = Vec::with_capacity(subjects.len());
    for subject in subjects.drain(..) {
        let duplicate = unique.iter().any(|existing: &serde_json::Value| {
            existing.get("kind") == subject.get("kind")
                && existing.get("id") == subject.get("id")
                && existing.get("locator") == subject.get("locator")
                && existing.get("hash") == subject.get("hash")
        });
        if !duplicate {
            unique.push(subject);
        }
    }
    *subjects = unique;
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
                let locator = subject.get("locator").and_then(serde_json::Value::as_str);
                locator.and_then(|locator| {
                    scan.sources
                        .iter()
                        .find(|source| source.target.value == locator)
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
            "static_analysis_source" => {
                let locator = subject
                    .get("locator")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                scan.tests
                    .iter()
                    .find(|test| {
                        format!("{}::{}", test.location.file, test.location.function) == locator
                    })
                    .and_then(|test| source_slice(root, &test.location).ok())
                    .map(|source| {
                        ContentHash::from_domain_fields(
                            "vtest:static-analysis-source:v1",
                            &[
                                (
                                    "test",
                                    scan.tests
                                        .iter()
                                        .find(|test| {
                                            format!(
                                                "{}::{}",
                                                test.location.file, test.location.function
                                            ) == locator
                                        })
                                        .map(|test| test.content_hash.as_str().as_bytes())
                                        .unwrap_or_default(),
                                ),
                                ("source", source.as_bytes()),
                            ],
                        )
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
            let hash = subject
                .get("hash")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let subject_id = subject
                .get("id")
                .and_then(serde_json::Value::as_str)
                .map(|id| format!("id: {}", yaml_quote(id)));
            let subject_locator = subject
                .get("locator")
                .and_then(serde_json::Value::as_str)
                .map(|locator| {
                    let kind = subject
                        .get("kind")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default();
                    let locator = if kind == "static_analysis_source" {
                        format!("static-analysis-source::{locator}")
                    } else {
                        locator.to_owned()
                    };
                    format!("locator: {}", yaml_quote(&locator))
                });
            let identity = subject_id
                .or(subject_locator)
                .unwrap_or_else(|| "id: ''".to_owned());
            out.push_str(&format!("  - {identity}\n"));
            out.push_str(&format!("    hash: {}\n", yaml_quote(hash)));
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
    if let Some(exclusions) = result
        .get("exclusions")
        .and_then(serde_json::Value::as_array)
    {
        if exclusions.is_empty() {
            out.push_str("exclusions: []\n");
        } else {
            out.push_str("exclusions:\n");
            for exclusion in exclusions {
                out.push_str(&format!(
                    "  - item: {}\n    basis: {}\n",
                    yaml_quote(
                        exclusion
                            .get("item")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("")
                    ),
                    yaml_quote(
                        exclusion
                            .get("basis")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("")
                    ),
                ));
            }
        }
    } else {
        out.push_str("exclusions: []\n");
    }
    let auditor = result.get("auditor");
    let auditor_kind = auditor
        .and_then(|value| value.get("kind"))
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("agent");
    let auditor_id = auditor
        .and_then(|value| value.get("id"))
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("vtest");
    out.push_str(&format!(
        "auditor:\n  kind: {}\n  id: {}\n",
        yaml_quote(auditor_kind),
        yaml_quote(auditor_id),
    ));
    if let Some(model) = auditor
        .and_then(|value| value.get("model"))
        .and_then(serde_json::Value::as_str)
    {
        out.push_str(&format!("  model: {}\n", yaml_quote(model)));
    }
    if let Some(confidence) = result.get("confidence").and_then(serde_json::Value::as_str) {
        out.push_str(&format!("confidence: {}\n", yaml_quote(confidence)));
    }
    let revision = bundle.get("revision");
    let commit = revision
        .and_then(|value| value.get("commit"))
        .and_then(serde_json::Value::as_str)
        .map(yaml_quote)
        .unwrap_or_else(|| "null".to_owned());
    let dirty = revision
        .and_then(|value| value.get("dirty"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    out.push_str(&format!(
        "audited_at: {}\nrevision: {{ commit: {commit}, dirty: {dirty} }}\n",
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
            let (diagnostic, code) = store_error_with_code(error, ExitCode::Internal);
            emit(
                format,
                quiet,
                JsonEnvelope::new(false, serde_json::Value::Null, vec![diagnostic]),
            );
            return code;
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
            let (diagnostic, code) = scan_error_with_code(error);
            emit(
                format,
                quiet,
                JsonEnvelope::new(false, serde_json::Value::Null, vec![diagnostic]),
            );
            return code;
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
            let targets = entity
                .targets
                .iter()
                .map(|target_ref| find_target_source(&scan, target_ref))
                .collect::<Vec<_>>();
            let target_hashes = targets
                .iter()
                .map(|source| {
                    source
                        .map(|source| source.content_hash.clone())
                        .unwrap_or_else(|| ContentHash::from_text(""))
                })
                .collect::<Vec<_>>();
            RunnableTest {
                entity,
                target_hashes,
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
    summary: bool,
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
            let (diagnostic, code) = store_error_with_code(error, ExitCode::Internal);
            emit(
                format,
                quiet,
                JsonEnvelope::new(false, serde_json::Value::Null, vec![diagnostic]),
            );
            return code;
        }
    };
    let scan = match scan_project(&root) {
        Ok(scan) => scan,
        Err(error) => {
            let (diagnostic, code) = scan_error_with_code(error);
            emit(
                format,
                quiet,
                JsonEnvelope::new(false, serde_json::Value::Null, vec![diagnostic]),
            );
            return code;
        }
    };
    let layout = VerifyLayout::new(&root);
    let entity_scope = match resolve_entity_scope(
        &layout,
        &scan,
        req.as_deref(),
        vo.as_deref(),
        test.as_deref(),
    ) {
        Ok(scope) => scope,
        Err(error) => {
            emit(
                format,
                quiet,
                JsonEnvelope::new(false, serde_json::Value::Null, vec![*error.diagnostic]),
            );
            return error.code;
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
    if requested.as_ref().is_some_and(Vec::is_empty) {
        emit(
            format,
            quiet,
            JsonEnvelope::new(
                false,
                serde_json::Value::Null,
                vec![Diagnostic::error(
                    "E-OP-001",
                    "--items must name at least one verification item",
                )],
            ),
        );
        return ExitCode::Usage;
    }
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
        let unique = scope.iter().collect::<std::collections::BTreeSet<_>>();
        if unique.len() != scope.len() {
            emit(
                format,
                quiet,
                JsonEnvelope::new(
                    false,
                    serde_json::Value::Null,
                    vec![Diagnostic::error(
                        "E-OP-001",
                        "--items must not contain duplicate verification items",
                    )],
                ),
            );
            return ExitCode::Usage;
        }
    }
    let result = verify_project_scoped(&root, &scan, &config, requested, entity_scope);
    let ok = result.is_ok();
    let diagnostics = result.diagnostics.clone();
    if summary {
        let requested = &result.report.requested_scope;
        let non_pass_items = result
            .report
            .items
            .iter()
            .filter(|item| {
                requested.iter().any(|requested| requested == &item.item)
                    && item.value != vtest_model::CheckValue::Pass
            })
            .count();
        let summary_data = serde_json::json!({
            "result": result.report.result,
            "requested_scope": result.report.requested_scope,
            "entity_scope": result.report.entity_scope,
            "scope_outside_not_checked": result.report.scope_outside_not_checked,
            "non_pass_items": non_pass_items,
        });
        emit(
            format,
            quiet,
            JsonEnvelope::new(ok, summary_data, diagnostics),
        );
    } else {
        emit(
            format,
            quiet,
            JsonEnvelope::new(
                ok,
                serde_json::to_value(&result).expect("verification result"),
                diagnostics,
            ),
        );
    }
    if ok {
        ExitCode::Ok
    } else {
        ExitCode::VerificationFailed
    }
}

fn resolve_entity_scope(
    layout: &VerifyLayout,
    scan: &ScanResult,
    req: Option<&str>,
    vo: Option<&str>,
    test: Option<&str>,
) -> Result<Option<EntityScope>, CommandFailure> {
    let selected =
        usize::from(req.is_some()) + usize::from(vo.is_some()) + usize::from(test.is_some());
    if selected > 1 {
        return Err(failure(
            "E-OP-001",
            "verify/report accepts at most one of --req, --vo, or --test",
            ExitCode::Usage,
        ));
    }
    if let Some(id) = req {
        validate_id(id, "REQ-")?;
        if read_req(layout, id).is_err() {
            return Err(failure(
                "E-OP-001",
                format!("REQ `{id}` was not found"),
                ExitCode::Usage,
            ));
        }
        return Ok(Some(EntityScope::Req(id.to_owned())));
    }
    if let Some(id) = vo {
        validate_id(id, "VO-")?;
        if read_vo(layout, id).is_err() {
            return Err(failure(
                "E-OP-001",
                format!("VO `{id}` was not found"),
                ExitCode::Usage,
            ));
        }
        return Ok(Some(EntityScope::Vo(id.to_owned())));
    }
    if let Some(id) = test {
        validate_id(id, "TEST-")?;
        if !scan.tests.iter().any(|entity| entity.id.as_str() == id) {
            return Err(failure(
                "E-OP-001",
                format!("Test `{id}` was not found"),
                ExitCode::Usage,
            ));
        }
        return Ok(Some(EntityScope::Test(id.to_owned())));
    }
    Ok(None)
}

const ALL_VERIFY_ITEMS: [&str; 12] = [
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
    "test_traceability",
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
    if !matches!(
        kind,
        "document" | "api-schema" | "type-spec" | "db-schema" | "other"
    ) {
        return Err(failure(
            "E-OP-001",
            "SPEC kind must be document, api-schema, type-spec, db-schema, or other",
            ExitCode::Usage,
        ));
    }
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
    let previous_hash = old.as_ref().map(|value| value.sha256.clone());
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
    let content_changed = previous_hash.is_some_and(|hash| hash != record.sha256);
    let mut value = serde_json::to_value(record).expect("record is serializable");
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "content_changed".to_owned(),
            serde_json::json!(content_changed),
        );
        object.insert(
            "dependent_facts_may_be_stale".to_owned(),
            serde_json::json!(content_changed),
        );
    }
    Ok(value)
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

fn list_reqs(layout: &VerifyLayout, tree: bool) -> CommandResult {
    let ids = read_record_ids_for(&layout.req_dir())?;
    let mut entries = BTreeMap::new();
    for id in ids {
        let record = read_req(layout, &id).map_err(store_failure)?;
        let parent = record
            .parent
            .as_ref()
            .map(|value| value.as_str().to_owned());
        entries.insert(
            id,
            (
                parent,
                serde_json::to_value(record).expect("record is serializable"),
            ),
        );
    }
    if tree {
        Ok(serde_json::json!({ "tree": true, "items": record_tree(entries) }))
    } else {
        Ok(serde_json::json!({
            "tree": false,
            "items": entries.into_values().map(|(_, value)| value).collect::<Vec<_>>()
        }))
    }
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
    combinations: &[String],
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
        if dimensions.is_empty() {
            return Err(failure(
                "E-OP-001",
                "VO coverage policy requires at least one dimension",
                ExitCode::Usage,
            ));
        }
    }
    let combinations = parse_explicit_combinations(&dimensions, policy.as_deref(), combinations)?;
    let now = now_rfc3339();
    let record = VoRecord {
        id: VoId::new(id),
        parent: parent.map(VoId::new),
        requirements: requirements.iter().cloned().map(ReqId::new).collect(),
        spec_refs,
        claim: claim.to_owned(),
        dimensions,
        coverage_policy: policy,
        combinations,
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
    let approval_invalidated = effective_vo_status(layout, &record) == "approved";
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
        if status != "draft" {
            return Err(failure(
                "E-OP-001",
                "VO status can only be set to draft; approved is derived from approval records",
                ExitCode::Usage,
            ));
        }
    }
    record.status = "draft".to_owned();
    record.updated = now_rfc3339();
    write_atomic(&path, &record.to_yaml()).map_err(store_failure)?;
    let mut value = serde_json::to_value(record).expect("record");
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "approval_invalidated".to_owned(),
            serde_json::json!(approval_invalidated),
        );
    }
    Ok(value)
}

fn list_vos(
    layout: &VerifyLayout,
    tree: bool,
    req: Option<&str>,
    status: Option<&str>,
) -> CommandResult {
    if let Some(req) = req {
        validate_id(req, "REQ-")?;
        read_req(layout, req).map_err(store_failure)?;
    }
    if status.is_some_and(|value| !matches!(value, "draft" | "approved")) {
        return Err(failure(
            "E-OP-001",
            "VO status filter must be draft or approved",
            ExitCode::Usage,
        ));
    }
    let ids = read_record_ids_for(&layout.vo_dir())?;
    let mut entries = BTreeMap::new();
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
        let parent = record
            .parent
            .as_ref()
            .map(|value| value.as_str().to_owned());
        entries.insert(
            id,
            (
                parent.clone(),
                serde_json::json!({
                    "id": record.id,
                    "claim": record.claim,
                    "status": effective,
                    "stored_status": record.status,
                    "parent": parent,
                    "requirements": record.requirements,
                }),
            ),
        );
    }
    if tree {
        Ok(serde_json::json!({ "tree": true, "items": record_tree(entries) }))
    } else {
        Ok(serde_json::json!({
            "tree": false,
            "items": entries.into_values().map(|(_, value)| value).collect::<Vec<_>>()
        }))
    }
}

fn record_tree(
    entries: BTreeMap<String, (Option<String>, serde_json::Value)>,
) -> Vec<serde_json::Value> {
    let mut children = BTreeMap::<String, Vec<String>>::new();
    let mut roots = Vec::new();
    for (id, (parent, _)) in &entries {
        if let Some(parent) = parent
            .as_ref()
            .filter(|parent| entries.contains_key(parent.as_str()))
        {
            children.entry(parent.clone()).or_default().push(id.clone());
        } else {
            roots.push(id.clone());
        }
    }
    for ids in children.values_mut() {
        ids.sort();
    }
    roots.sort();

    let mut expanded = BTreeSet::new();
    let mut output = Vec::new();
    for id in roots {
        if let Some(value) = record_tree_node(&id, &entries, &children, &mut expanded) {
            output.push(value);
        }
    }
    // Malformed cyclic graphs have no natural root. Keep list output total and
    // deterministic; scanner diagnostics remain the authority for the cycle.
    for id in entries.keys() {
        if let Some(value) = record_tree_node(id, &entries, &children, &mut expanded) {
            output.push(value);
        }
    }
    output
}

fn record_tree_node(
    id: &str,
    entries: &BTreeMap<String, (Option<String>, serde_json::Value)>,
    children: &BTreeMap<String, Vec<String>>,
    expanded: &mut BTreeSet<String>,
) -> Option<serde_json::Value> {
    if !expanded.insert(id.to_owned()) {
        return None;
    }
    let mut value = entries.get(id)?.1.clone();
    let nested = children
        .get(id)
        .into_iter()
        .flatten()
        .filter_map(|child| record_tree_node(child, entries, children, expanded))
        .collect::<Vec<_>>();
    if let Some(object) = value.as_object_mut() {
        object.insert("children".to_owned(), serde_json::Value::Array(nested));
    }
    Some(value)
}

fn show_vo(layout: &VerifyLayout, id: &str) -> CommandResult {
    validate_id(id, "VO-")?;
    let record = read_vo(layout, id).map_err(store_failure)?;
    let effective = effective_vo_status(layout, &record);
    let scan = scan_project(&layout.root).map_err(|error| {
        failure(
            "E-CORE-001",
            format!("could not scan covering tests for VO {id}: {error}"),
            ExitCode::Internal,
        )
    })?;
    let covering_tests = scan
        .tests
        .iter()
        .filter(|test| test.covers.iter().any(|vo| vo.as_str() == id))
        .map(|test| {
            serde_json::json!({
                "id": test.id,
                "file": test.location.file,
                "function": test.location.function,
            })
        })
        .collect::<Vec<_>>();
    let vo_path = layout.vo_dir().join(format!("{id}.yaml"));
    let current_hash = ContentHash::from_text(&read_text(&vo_path).map_err(store_failure)?);
    let approvals = approval_history(layout, id, &current_hash);
    let audits = vo_audit_history(layout, id, &current_hash);
    let mut value = serde_json::to_value(record).expect("record");
    if let Some(object) = value.as_object_mut() {
        object.insert("effective_status".to_owned(), serde_json::json!(effective));
        object.insert(
            "covering_tests".to_owned(),
            serde_json::Value::Array(covering_tests),
        );
        object.insert("approvals".to_owned(), serde_json::Value::Array(approvals));
        object.insert("audits".to_owned(), serde_json::Value::Array(audits));
        // Full audit validity depends on every recorded subject and is an M5
        // concern. A show command must not promote a partial hash comparison.
        object.insert("audit_state".to_owned(), serde_json::json!("NOT_CHECKED"));
    }
    Ok(value)
}

fn approval_history(
    layout: &VerifyLayout,
    vo_id: &str,
    current_hash: &ContentHash,
) -> Vec<serde_json::Value> {
    let Ok(entries) = fs::read_dir(layout.approvals_dir()) else {
        return Vec::new();
    };
    let mut paths = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("yaml"))
        .collect::<Vec<_>>();
    paths.sort();
    paths
        .into_iter()
        .filter_map(|path| read_approval(&path).ok())
        .filter(|approval| approval.subject.as_str() == vo_id)
        .map(|approval| {
            serde_json::json!({
                "id": approval.id,
                "approved_at": approval.approved_at,
                "approver": approval.approver,
                "basis": approval.basis,
                "valid": &approval.subject_hash == current_hash,
            })
        })
        .collect()
}

fn vo_audit_history(
    layout: &VerifyLayout,
    vo_id: &str,
    current_hash: &ContentHash,
) -> Vec<serde_json::Value> {
    let Ok(entries) = fs::read_dir(layout.audits_dir()) else {
        return Vec::new();
    };
    let mut paths = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("yaml"))
        .collect::<Vec<_>>();
    paths.sort();
    paths
        .into_iter()
        .filter_map(|path| {
            let text = read_text(&path).ok()?;
            let subject_hash = audit_subject_hash(&text, vo_id)?;
            Some(serde_json::json!({
                "id": yaml_scalar_value(&text, "id").or_else(|| path.file_stem().and_then(|value| value.to_str()).map(str::to_owned)),
                "kind": yaml_scalar_value(&text, "kind"),
                "verdict": yaml_scalar_value(&text, "verdict"),
                "audited_at": yaml_scalar_value(&text, "audited_at"),
                "subject_hash_matches": &subject_hash == current_hash,
            }))
        })
        .collect()
}

fn audit_subject_hash(text: &str, subject_id: &str) -> Option<ContentHash> {
    let lines = text.lines().collect::<Vec<_>>();
    let start = lines.iter().position(|line| {
        line.trim_start().starts_with("id:")
            && yaml_scalar_value(line, "id").as_deref() == Some(subject_id)
    })?;
    lines
        .iter()
        .skip(start + 1)
        .take_while(|line| {
            let trimmed = line.trim_start();
            line.starts_with(' ') && !trimmed.starts_with("- kind:")
        })
        .find_map(|line| yaml_scalar_value(line, "hash"))?
        .parse()
        .ok()
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
    let combinations = dimension_combinations(&parent.dimensions, policy, &parent.combinations)?;
    let mut planned = Vec::new();
    let mut child_ids = BTreeSet::new();
    for combination in combinations {
        let suffix_parts = combination
            .iter()
            .map(|value| slug(value))
            .collect::<Vec<_>>();
        if suffix_parts.iter().any(String::is_empty) {
            return Err(failure(
                "E-OP-001",
                "VO partitions must contain an alphanumeric character",
                ExitCode::Usage,
            ));
        }
        let suffix = suffix_parts.join("-");
        let child_id = format!("{}-{}", parent.id, suffix);
        if !child_ids.insert(child_id.clone()) {
            return Err(failure(
                "E-OP-001",
                format!("VO expansion generates duplicate child ID `{child_id}`"),
                ExitCode::Usage,
            ));
        }
        let child_path = layout.vo_dir().join(format!("{child_id}.yaml"));
        let claim = format!("{} [{}]", parent.claim, combination.join(", "));
        if child_path.exists() {
            let existing = read_vo(layout, &child_id).map_err(store_failure)?;
            if existing.parent.as_ref() != Some(&parent.id)
                || existing.requirements != parent.requirements
                || existing.spec_refs != parent.spec_refs
                || existing.claim != claim
            {
                return Err(failure(
                    "E-OP-001",
                    format!("VO expansion child ID `{child_id}` already has different content"),
                    ExitCode::Usage,
                ));
            }
        }
        planned.push((combination, child_id, child_path, claim));
    }

    let mut children = Vec::new();
    for (combination, child_id, child_path, claim) in planned {
        let mut created = false;
        if !dry_run && !child_path.exists() {
            let now = now_rfc3339();
            let child = VoRecord {
                id: VoId::new(&child_id),
                parent: Some(parent.id.clone()),
                requirements: parent.requirements.clone(),
                spec_refs: parent.spec_refs.clone(),
                claim,
                dimensions: Vec::new(),
                coverage_policy: None,
                combinations: Vec::new(),
                representative_cases: Vec::new(),
                status: "draft".to_owned(),
                created: now.clone(),
                updated: now,
            };
            write_new_record(&child_path, &child.to_yaml()).map_err(store_failure)?;
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
    for reference in basis {
        if !is_valid_ulid(reference)
            || !layout
                .audits_dir()
                .join(format!("{reference}.yaml"))
                .is_file()
        {
            return Err(failure(
                "E-OP-001",
                format!("approval basis audit `{reference}` does not exist"),
                ExitCode::Usage,
            ));
        }
    }
    let vo_path = layout.vo_dir().join(format!("{id}.yaml"));
    let mut vo = read_vo(layout, id).map_err(store_failure)?;
    if vo.status != "approved" {
        vo.status = "approved".to_owned();
        vo.updated = now_rfc3339();
        write_atomic(&vo_path, &vo.to_yaml()).map_err(store_failure)?;
    }
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
        basis: basis
            .iter()
            .map(|reference| ApprovalBasis {
                kind: "audit".to_owned(),
                reference: reference.clone(),
            })
            .collect(),
        approved_at: now_rfc3339(),
    };
    let path = layout.approvals_dir().join(format!("{}.yaml", approval.id));
    write_new_record(&path, &approval.to_yaml()).map_err(store_failure)?;
    Ok(serde_json::to_value(approval).expect("record"))
}

fn dimension_combinations(
    dimensions: &[Dimension],
    policy: &str,
    explicit_combinations: &[Vec<String>],
) -> Result<Vec<Vec<String>>, CommandFailure> {
    let mut names = BTreeSet::new();
    if dimensions.is_empty()
        || dimensions.iter().any(|dimension| {
            dimension.name.trim().is_empty()
                || !names.insert(dimension.name.as_str())
                || dimension.partitions.is_empty()
                || dimension
                    .partitions
                    .iter()
                    .any(|partition| partition.trim().is_empty())
                || dimension.partitions.iter().collect::<BTreeSet<_>>().len()
                    != dimension.partitions.len()
        })
    {
        return Err(failure(
            "E-OP-001",
            "VO has invalid dimensions; run vtest scan for record diagnostics",
            ExitCode::Usage,
        ));
    }
    match policy {
        "independent-axes" | "full-product" if !explicit_combinations.is_empty() => Err(failure(
            "E-OP-001",
            "non-explicit coverage policy cannot store combinations",
            ExitCode::Usage,
        )),
        "independent-axes" => Ok(dimensions
            .iter()
            .flat_map(|dimension| {
                dimension
                    .partitions
                    .iter()
                    .map(|partition| vec![partition.clone()])
            })
            .collect()),
        "full-product" => {
            let mut combinations = vec![Vec::new()];
            for dimension in dimensions {
                let mut next = Vec::new();
                for prefix in &combinations {
                    for partition in &dimension.partitions {
                        let mut combination = prefix.clone();
                        combination.push(partition.clone());
                        next.push(combination);
                    }
                }
                combinations = next;
            }
            Ok(combinations)
        }
        "explicit" if !explicit_combinations.is_empty() => {
            let mut unique = BTreeSet::new();
            if explicit_combinations.iter().any(|combination| {
                combination.len() != dimensions.len()
                    || !combination
                        .iter()
                        .zip(dimensions)
                        .all(|(partition, dimension)| dimension.partitions.contains(partition))
                    || !unique.insert(combination)
            }) {
                return Err(failure(
                    "E-OP-001",
                    "VO has invalid explicit combinations; run vtest scan for record diagnostics",
                    ExitCode::Usage,
                ));
            }
            Ok(explicit_combinations.to_vec())
        }
        "explicit" => Err(failure(
            "E-OP-001",
            "explicit expansion requires at least one combination",
            ExitCode::Usage,
        )),
        _ => Err(failure(
            "E-OP-001",
            "unsupported VO coverage policy",
            ExitCode::Usage,
        )),
    }
}

fn parse_explicit_combinations(
    dimensions: &[Dimension],
    policy: Option<&str>,
    values: &[String],
) -> Result<Vec<Vec<String>>, CommandFailure> {
    if policy != Some("explicit") {
        if values.is_empty() {
            return Ok(Vec::new());
        }
        return Err(failure(
            "E-OP-001",
            "--combination is only valid with --policy explicit",
            ExitCode::Usage,
        ));
    }
    if dimensions.is_empty() || values.is_empty() {
        return Err(failure(
            "E-OP-001",
            "explicit coverage requires dimensions and at least one --combination",
            ExitCode::Usage,
        ));
    }
    let mut combinations = Vec::new();
    let mut unique = BTreeSet::new();
    for value in values {
        let combination = value
            .split(',')
            .map(str::trim)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if combination.len() != dimensions.len()
            || combination.iter().any(|partition| partition.is_empty())
        {
            return Err(failure(
                "E-OP-001",
                format!(
                    "combination `{value}` must provide one partition for each of {} dimensions",
                    dimensions.len()
                ),
                ExitCode::Usage,
            ));
        }
        for (dimension, partition) in dimensions.iter().zip(&combination) {
            if !dimension.partitions.contains(partition) {
                return Err(failure(
                    "E-OP-001",
                    format!(
                        "partition `{partition}` is not declared by dimension `{}`",
                        dimension.name
                    ),
                    ExitCode::Usage,
                ));
            }
        }
        if !unique.insert(combination.clone()) {
            return Err(failure(
                "E-OP-001",
                format!("duplicate explicit combination `{value}`"),
                ExitCode::Usage,
            ));
        }
        combinations.push(combination);
    }
    Ok(combinations)
}

fn parse_dimensions(values: &[String]) -> Result<Vec<Dimension>, CommandFailure> {
    let mut dimensions = Vec::new();
    let mut names = BTreeSet::new();
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
        let name = name.trim();
        if name.is_empty() || partitions.is_empty() {
            return Err(failure(
                "E-OP-001",
                "dimension name and partitions are required",
                ExitCode::Usage,
            ));
        }
        if !names.insert(name.to_owned()) {
            return Err(failure(
                "E-OP-001",
                format!("duplicate VO dimension `{name}`"),
                ExitCode::Usage,
            ));
        }
        let mut unique_partitions = BTreeSet::new();
        for partition in &partitions {
            if slug(partition).is_empty() {
                return Err(failure(
                    "E-OP-001",
                    format!("partition `{partition}` has no alphanumeric ID component"),
                    ExitCode::Usage,
                ));
            }
            if !unique_partitions.insert(partition.clone()) {
                return Err(failure(
                    "E-OP-001",
                    format!("duplicate partition `{partition}` in dimension `{name}`"),
                    ExitCode::Usage,
                ));
            }
        }
        dimensions.push(Dimension {
            name: name.to_owned(),
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
        let file_id = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if !is_valid_ulid(file_id) {
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
    if !id.starts_with(prefix)
        || id.len() <= prefix.len()
        || !id.chars().all(|character| {
            character.is_ascii_uppercase() || character.is_ascii_digit() || character == '-'
        })
    {
        return Err(failure(
            "E-OP-001",
            format!(
                "id must start with `{prefix}` and contain only uppercase ASCII letters, digits, and hyphens"
            ),
            ExitCode::Usage,
        ));
    }
    Ok(())
}

fn slug(value: &str) -> String {
    let mut slug = String::new();
    let mut separator = true;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_uppercase());
            separator = false;
        } else if !separator {
            slug.push('-');
            separator = true;
        }
    }
    if separator {
        slug.pop();
    }
    slug
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
        StoreError::InvalidConfig(message) => {
            (Diagnostic::error("E-CONFIG-001", message), ExitCode::Usage)
        }
        other => (Diagnostic::error("E-CORE-001", other.to_string()), default),
    }
}

fn scan_error_with_code(error: vtest_scan::ScanError) -> (Diagnostic, ExitCode) {
    match error {
        vtest_scan::ScanError::Adapter { code, message } => {
            (Diagnostic::error(code, message), ExitCode::Usage)
        }
        vtest_scan::ScanError::Store(store_error) => {
            store_error_with_code(store_error, ExitCode::Internal)
        }
        other => (
            Diagnostic::error("E-CORE-001", other.to_string()),
            ExitCode::Internal,
        ),
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
        if let Some(report) = value.get("report").and_then(serde_json::Value::as_object) {
            let requested_scope = report
                .get("requested_scope")
                .and_then(serde_json::Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .unwrap_or_else(|| "full".to_owned());
            println!("Requested scope: {requested_scope}");
            if let Some(entity) = report.get("entity_scope") {
                println!("Entity scope: {entity}");
            }
            if report
                .get("scope_outside_not_checked")
                .and_then(serde_json::Value::as_bool)
                == Some(true)
            {
                println!("Scope outside requested range: NOT_CHECKED");
            }
            if let Some(tree) = report.get("tree").and_then(serde_json::Value::as_array) {
                for (index, node) in tree.iter().enumerate() {
                    print_text_tree_node(node, &[], index + 1 == tree.len());
                }
                let mut rendered_items = BTreeSet::new();
                for node in tree {
                    collect_text_tree_items(node, &mut rendered_items);
                }
                let repository_items = report
                    .get("items")
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter(|item| {
                        item.get("item")
                            .and_then(serde_json::Value::as_str)
                            .is_some_and(|name| !rendered_items.contains(name))
                    })
                    .collect::<Vec<_>>();
                if !repository_items.is_empty() {
                    println!("Repository checks:");
                    for (index, item) in repository_items.iter().enumerate() {
                        let key = item.get("item").and_then(serde_json::Value::as_str);
                        let value = item.get("value").and_then(serde_json::Value::as_str);
                        if let (Some(key), Some(value)) = (key, value) {
                            let branch = if index + 1 == repository_items.len() {
                                "└─"
                            } else {
                                "├─"
                            };
                            println!("{branch} {key:<24} {value}");
                        }
                    }
                }
            } else if let Some(items) = report.get("items").and_then(serde_json::Value::as_array) {
                for item in items {
                    let key = item.get("item").and_then(serde_json::Value::as_str);
                    let value = item.get("value").and_then(serde_json::Value::as_str);
                    if let (Some(key), Some(value)) = (key, value) {
                        println!("├─ {key:<24} {value}");
                    }
                }
            }
            if let Some(result) = report.get("result").and_then(serde_json::Value::as_str) {
                println!("Result: {result}");
            }
        } else if value.get("result").is_some() && value.get("non_pass_items").is_some() {
            println!(
                "Result: {}",
                value
                    .get("result")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("UNKNOWN")
            );
            println!(
                "Non-PASS items: {}",
                value
                    .get("non_pass_items")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or_default()
            );
        } else if let Some(summary) = value.get("summary") {
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

fn render_tree_prefix(ancestors_have_next: &[bool], is_last: bool) -> String {
    let mut prefix = String::new();
    for has_next in ancestors_have_next {
        prefix.push_str(if *has_next { "│  " } else { "   " });
    }
    prefix.push_str(if is_last { "└─ " } else { "├─ " });
    prefix
}

fn print_text_tree_node(node: &serde_json::Value, ancestors_have_next: &[bool], is_last: bool) {
    let kind = node
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("entity");
    let id = node
        .get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("?");
    let value = node
        .get("value")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("UNKNOWN");
    println!(
        "{}{}:{id:<28} {value}",
        render_tree_prefix(ancestors_have_next, is_last),
        kind
    );
    let items = node
        .get("items")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let children = node
        .get("children")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let child_ancestors = ancestors_have_next
        .iter()
        .copied()
        .chain(std::iter::once(!is_last))
        .collect::<Vec<_>>();
    let total = items.len() + children.len();
    for (index, item) in items.iter().enumerate() {
        let key = item.get("item").and_then(serde_json::Value::as_str);
        let value = item.get("value").and_then(serde_json::Value::as_str);
        if let (Some(key), Some(value)) = (key, value) {
            println!(
                "{}{key:<24} {value}",
                render_tree_prefix(&child_ancestors, index + 1 == total)
            );
        }
    }
    for (index, child) in children.iter().enumerate() {
        print_text_tree_node(child, &child_ancestors, items.len() + index + 1 == total);
    }
}

fn collect_text_tree_items(node: &serde_json::Value, names: &mut BTreeSet<String>) {
    if let Some(items) = node.get("items").and_then(serde_json::Value::as_array) {
        for item in items {
            if let Some(name) = item.get("item").and_then(serde_json::Value::as_str) {
                names.insert(name.to_owned());
            }
        }
    }
    if let Some(children) = node.get("children").and_then(serde_json::Value::as_array) {
        for child in children {
            collect_text_tree_items(child, names);
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
        for directory in [
            "spec",
            "req",
            "vo",
            "rel",
            "approvals",
            "audits",
            "evidence",
        ] {
            assert!(root
                .join(".verify")
                .join(directory)
                .join(".gitkeep")
                .is_file());
        }
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
                    combinations: Vec::new(),
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
        assert_eq!(integration.targets.len(), 2);
        assert_eq!(integration.kind.as_deref(), Some("integration-normal"));
        assert!(query_tests_from_scan(
            &integration_scan.tests,
            &integration_scan.sources,
            &integration_scan.diagnostics,
            "src/lib.rs::subtract",
        )
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

    #[test]
    fn audit_diagnostics_are_merged_in_deterministic_order() {
        let diagnostics = merge_diagnostics(
            vec![Diagnostic::warning("W-SCAN-101", "scanner warning")],
            vec![
                Diagnostic::warning("W-DA-101", "audit warning"),
                Diagnostic::error("E-DA-001", "audit error"),
            ],
        );
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.as_str())
                .collect::<Vec<_>>(),
            vec!["E-DA-001", "W-DA-101", "W-SCAN-101"]
        );
    }

    #[test]
    fn static_audit_rejects_combined_test_and_all_selectors() {
        let root = root();
        assert_eq!(
            run(Cli {
                project: root.clone(),
                format: OutputFormat::Json,
                quiet: true,
                command: Command::Audit {
                    command: AuditCommand::Static {
                        test: Some("TEST-EXAMPLE-001".to_owned()),
                        all: true,
                    },
                },
            }) as u8,
            ExitCode::Usage as u8
        );
        fs::remove_dir_all(root).unwrap();
    }
}
