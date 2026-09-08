//! Acceptance coverage for the `vtest run` CLI surface (DS-1101-1103).
//!
//! `crates/vtest-cli/tests/evidence_e2e.rs` already exercises the underlying
//! library entry points (`scan_project` / `run_tests` / `verify_project`)
//! directly and disclosed that no `vtest run` CLI surface existed yet. This
//! file exercises the same fixture shape through `vtest_cli::run` /
//! `Command::Run` instead, so the CLI wiring itself (unknown `--test` id
//! rejection, exit codes, Evidence count in the JSON envelope) has coverage.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use vtest_cli::{run, Cli, Command, OutputFormat};
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
        "vtest-cli-run-acceptance-{name}-{}-{nanos}-{sequence}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create fixture root");
    root
}

fn git(root: &Path, args: &[&str]) {
    let status = ProcessCommand::new("git")
        .current_dir(root)
        .args(args)
        .status()
        .unwrap_or_else(|error| panic!("failed to run git {args:?}: {error}"));
    assert!(status.success(), "git {args:?} failed");
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
        created: "2026-09-08T00:00:00Z".to_owned(),
        updated: "2026-09-08T00:00:00Z".to_owned(),
    }
}

const TEST_BODY: &str = "/// @vtest.id TEST-RUN-CLI-DOUBLE\n\
     /// @vtest.covers VO-RUN-CLI-DOUBLE\n\
     /// @vtest.target src/lib.rs::double\n\
     /// @vtest.intent doubles the input\n\
     #[test]\n\
     fn it_doubles() {\n    assert_eq!(vtest_run_cli_fixture::double(2), 4);\n}\n";

fn build_fixture_project(root: &Path) {
    fs::create_dir_all(root.join("src")).expect("mkdir src");
    fs::create_dir_all(root.join("tests")).expect("mkdir tests");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"vtest-run-cli-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
         [workspace]\n",
    )
    .expect("write Cargo.toml");
    fs::write(
        root.join("src").join("lib.rs"),
        "pub fn double(x: i32) -> i32 { x * 2 }\n",
    )
    .expect("write src/lib.rs");
    fs::write(root.join("tests").join("registered.rs"), TEST_BODY).expect("write test file");

    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "vtest-fixture@example.com"]);
    git(root, &["config", "user.name", "vtest fixture"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "initial fixture commit"]);

    let layout = init_project(root, "vtest-run-cli-fixture").expect("init .verify/ layout");
    write_document_file(&layout, "fixture", &document_file()).expect("write document file");
    write_vo_record(&layout, &vo("VO-RUN-CLI-DOUBLE")).expect("write VO record");
}

const BROKEN_TEST_BODY: &str = "/// @vtest.id TEST-RUN-CLI-BROKEN\n\
     /// @vtest.covers VO-RUN-CLI-DOUBLE\n\
     /// @vtest.target src/lib.rs::double\n\
     /// @vtest.intent doubles the input\n\
     #[test]\n\
     fn it_doubles() {\n    assert_eq!(vtest_run_cli_fixture::this_symbol_does_not_exist(2), 4);\n}\n";

/// Same fixture shape as [`build_fixture_project`], but the annotated
/// `#[test]` fn references an undefined symbol, so `cargo test` fails to
/// compile the test binary at all -- no `test <name> ... ok/FAILED` line
/// is ever printed for the declared Test id, which is exactly the
/// "requested Test has no result line" (E-EXEC-001/E-EXEC-002) case,
/// distinct from `UnknownTestId`/`UnknownVoId` (those are usage rejections
/// before any execution is attempted at all).
fn build_broken_fixture_project(root: &Path) {
    fs::create_dir_all(root.join("src")).expect("mkdir src");
    fs::create_dir_all(root.join("tests")).expect("mkdir tests");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"vtest-run-cli-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
         [workspace]\n",
    )
    .expect("write Cargo.toml");
    fs::write(
        root.join("src").join("lib.rs"),
        "pub fn double(x: i32) -> i32 { x * 2 }\n",
    )
    .expect("write src/lib.rs");
    fs::write(root.join("tests").join("registered.rs"), BROKEN_TEST_BODY).expect("write test file");

    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "vtest-fixture@example.com"]);
    git(root, &["config", "user.name", "vtest fixture"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "initial fixture commit"]);

    let layout = init_project(root, "vtest-run-cli-fixture").expect("init .verify/ layout");
    write_document_file(&layout, "fixture", &document_file()).expect("write document file");
    write_vo_record(&layout, &vo("VO-RUN-CLI-DOUBLE")).expect("write VO record");
}

fn cli(root: &Path, command: Command) -> Cli {
    Cli {
        project: root.to_path_buf(),
        format: OutputFormat::Json,
        quiet: true,
        command,
    }
}

/// `vtest run` outside any project is an operation rejection (exit 2), not a
/// silent "nothing to run" success — mirrors `verify`'s own contract.
#[test]
fn run_outside_a_project_is_an_operation_rejection() {
    let root = temp_root("no-project");
    let exit = run(cli(
        &root,
        Command::Run {
            test: Vec::new(),
            vo: None,
            all: false,
            fast: false,
        },
    ));
    assert_eq!(exit, ExitCode::Usage);
}

/// An explicit `--test` id the scan never discovered must be rejected
/// (E-OP-001, exit 2) rather than silently ignored — the same "don't narrow
/// the requested scope without disclosure" rule `verify --items` applies to
/// unknown check names.
#[test]
fn an_unknown_test_id_is_a_usage_error() {
    let root = temp_root("unknown-test-id");
    build_fixture_project(&root);
    let exit = run(cli(
        &root,
        Command::Run {
            test: vec!["TEST-DOES-NOT-EXIST".to_owned()],
            vo: None,
            all: false,
            fast: false,
        },
    ));
    assert_eq!(exit, ExitCode::Usage);
}

/// Running the fixture's real Test executes it and writes one Evidence
/// record; exit 0 when execution reports no error diagnostics.
#[test]
fn running_a_real_test_writes_evidence_and_exits_ok() {
    let root = temp_root("real-test");
    build_fixture_project(&root);
    let exit = run(cli(
        &root,
        Command::Run {
            test: vec!["TEST-RUN-CLI-DOUBLE".to_owned()],
            vo: None,
            all: false,
            fast: false,
        },
    ));
    assert_eq!(exit, ExitCode::Ok);
    let evidence_dir = root.join(".verify").join("evidence");
    let count = fs::read_dir(&evidence_dir)
        .expect("evidence dir exists")
        .count();
    assert!(
        count >= 1,
        "expected at least one Evidence record on disk, found {count}"
    );
}

/// DS-744 axis 2/3: `--vo` selects every Test whose `covers` intersects the
/// VO subtree rooted at the given id — here the fixture's single VO with a
/// single covering Test, so the effect is the same as naming that Test
/// directly, but reached through the VO axis.
#[test]
fn vo_axis_selects_tests_covering_the_named_vo() {
    let root = temp_root("vo-axis");
    build_fixture_project(&root);
    let exit = run(cli(
        &root,
        Command::Run {
            test: Vec::new(),
            vo: Some("VO-RUN-CLI-DOUBLE".to_owned()),
            all: false,
            fast: false,
        },
    ));
    assert_eq!(exit, ExitCode::Ok);
    let evidence_dir = root.join(".verify").join("evidence");
    let count = fs::read_dir(&evidence_dir)
        .expect("evidence dir exists")
        .count();
    assert!(
        count >= 1,
        "expected at least one Evidence record via the --vo axis, found {count}"
    );
}

/// DS-744: an unresolved `--vo` id is a usage rejection (E-OP-001), the same
/// "don't silently narrow to nothing" rule an unknown `--test` id follows.
#[test]
fn an_unknown_vo_id_is_a_usage_error() {
    let root = temp_root("unknown-vo-id");
    build_fixture_project(&root);
    let exit = run(cli(
        &root,
        Command::Run {
            test: Vec::new(),
            vo: Some("VO-DOES-NOT-EXIST".to_owned()),
            all: false,
            fast: false,
        },
    ));
    assert_eq!(exit, ExitCode::Usage);
}

/// D-#1: `vtest run`'s exit code is deliberately silent in 正本 §17.2 (only
/// `scan`/`doctor` are enumerated there) — this is upstream silence, not a
/// gap this codebase fills in; team-lead ruling 2026-09-10 confirms the
/// silence is correct and asks for a fail-closed regression test instead:
/// a `run` call that reaches `Ok(ExecutionResult)` but carries an error
/// diagnostic (Test id resolved fine, but its execution produced no result
/// line -- distinct from `UnknownTestId`, which never reaches execution at
/// all) must exit 1, matching `running_a_real_test_writes_evidence_and_exits_ok`'s
/// exit-0 control case on the same fixture shape.
#[test]
fn a_test_that_executes_but_produces_no_result_line_exits_verification_failed() {
    let root = temp_root("broken-test");
    build_broken_fixture_project(&root);
    let exit = run(cli(
        &root,
        Command::Run {
            test: vec!["TEST-RUN-CLI-BROKEN".to_owned()],
            vo: None,
            all: false,
            fast: false,
        },
    ));
    assert_eq!(
        exit,
        ExitCode::VerificationFailed,
        "an error diagnostic from execution (no result line, code found via compile failure) \
         must fail closed to exit 1, not exit 0"
    );
}

/// DS-744 axis 3/3: `--all` runs every Test the scan materialized, named
/// explicitly rather than relying on the empty-`--test`-list default.
#[test]
fn all_axis_runs_every_discovered_test() {
    let root = temp_root("all-axis");
    build_fixture_project(&root);
    let exit = run(cli(
        &root,
        Command::Run {
            test: Vec::new(),
            vo: None,
            all: true,
            fast: false,
        },
    ));
    assert_eq!(exit, ExitCode::Ok);
    let evidence_dir = root.join(".verify").join("evidence");
    let count = fs::read_dir(&evidence_dir)
        .expect("evidence dir exists")
        .count();
    assert!(
        count >= 1,
        "expected at least one Evidence record via the --all axis, found {count}"
    );
}
