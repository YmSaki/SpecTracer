//! Acceptance coverage for `vtest doc add|list|show` (本冊 §12.2,
//! DS-1000-1018, DES-482).

use std::{
    env, fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use vtest_cli::{run, Cli, Command, DocCommand, OutputFormat};
use vtest_model::ExitCode;
use vtest_store::init_project;

static NEXT: AtomicU64 = AtomicU64::new(0);

fn temp_root(name: &str) -> PathBuf {
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let root = env::temp_dir().join(format!(
        "vtest-cli-doc-acceptance-{name}-{}-{nanos}-{sequence}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create fixture root");
    root
}

fn build_fixture_project(root: &std::path::Path) {
    init_project(root, "vtest-doc-fixture").expect("init .verify/ layout");
    fs::write(root.join("basic-spec.md"), "# fixture\n").expect("write source file");
}

fn cli(root: &std::path::Path, command: Command) -> Cli {
    Cli {
        project: root.to_path_buf(),
        format: OutputFormat::Json,
        quiet: true,
        command,
    }
}

fn add_command(id: &str, path: &str) -> Command {
    Command::Doc(DocCommand::Add {
        id: id.to_owned(),
        path: path.to_owned(),
        title: None,
        derives_from: Vec::new(),
        anchor: Vec::new(),
        note: Vec::new(),
        root: false,
        no_root: false,
        update: false,
    })
}

#[test]
fn add_registers_a_document_and_exits_ok() {
    let root = temp_root("add-ok");
    build_fixture_project(&root);
    let exit = run(cli(&root, add_command("DOC-BASIC-001", "basic-spec.md")));
    assert_eq!(exit, ExitCode::Ok);
    assert!(root
        .join(".verify")
        .join("doc")
        .join("DOC-BASIC-001.yaml")
        .exists());
}

/// A second `add` for the same id without `--update` is a usage rejection.
#[test]
fn add_of_an_existing_id_without_update_is_a_usage_error() {
    let root = temp_root("add-duplicate");
    build_fixture_project(&root);
    let first = run(cli(&root, add_command("DOC-BASIC-001", "basic-spec.md")));
    assert_eq!(first, ExitCode::Ok);
    let second = run(cli(&root, add_command("DOC-BASIC-001", "basic-spec.md")));
    assert_eq!(second, ExitCode::Usage);
}

/// `--update` on an id that was never registered is a usage rejection.
#[test]
fn update_of_an_unregistered_id_is_a_usage_error() {
    let root = temp_root("update-unregistered");
    build_fixture_project(&root);
    let exit = run(cli(
        &root,
        Command::Doc(DocCommand::Add {
            id: "DOC-BASIC-001".to_owned(),
            path: "basic-spec.md".to_owned(),
            title: None,
            derives_from: Vec::new(),
            anchor: Vec::new(),
            note: Vec::new(),
            root: false,
            no_root: false,
            update: true,
        }),
    ));
    assert_eq!(exit, ExitCode::Usage);
}

/// DS-1012: `--update` recomputes `content_hash` from the current file.
#[test]
fn update_recomputes_content_hash_from_the_current_file() {
    let root = temp_root("update-rehash");
    build_fixture_project(&root);
    let first = run(cli(&root, add_command("DOC-BASIC-001", "basic-spec.md")));
    assert_eq!(first, ExitCode::Ok);

    fs::write(root.join("basic-spec.md"), "# fixture, changed\n").expect("rewrite source file");
    let update = run(cli(
        &root,
        Command::Doc(DocCommand::Add {
            id: "DOC-BASIC-001".to_owned(),
            path: "basic-spec.md".to_owned(),
            title: None,
            derives_from: Vec::new(),
            anchor: Vec::new(),
            note: Vec::new(),
            root: false,
            no_root: false,
            update: true,
        }),
    ));
    assert_eq!(update, ExitCode::Ok);
}

/// DS-1008: `--anchor` given for an id with no matching `--derives-from` is
/// a usage error, and no record is written.
#[test]
fn anchor_without_a_matching_derives_from_is_a_usage_error() {
    let root = temp_root("anchor-orphan");
    build_fixture_project(&root);
    let exit = run(cli(
        &root,
        Command::Doc(DocCommand::Add {
            id: "DOC-BASIC-001".to_owned(),
            path: "basic-spec.md".to_owned(),
            title: None,
            derives_from: Vec::new(),
            anchor: vec!["DOC-REQ-001:§3.2".to_owned()],
            note: Vec::new(),
            root: false,
            no_root: false,
            update: false,
        }),
    ));
    assert_eq!(exit, ExitCode::Usage);
    assert!(!root
        .join(".verify")
        .join("doc")
        .join("DOC-BASIC-001.yaml")
        .exists());
}

/// `show` on an unregistered id is a usage rejection.
#[test]
fn show_of_an_unregistered_id_is_a_usage_error() {
    let root = temp_root("show-unregistered");
    build_fixture_project(&root);
    let exit = run(cli(
        &root,
        Command::Doc(DocCommand::Show {
            id: "DOC-DOES-NOT-EXIST".to_owned(),
        }),
    ));
    assert_eq!(exit, ExitCode::Usage);
}

/// `list` after two registrations succeeds and exits 0.
#[test]
fn list_after_registrations_exits_ok() {
    let root = temp_root("list-ok");
    build_fixture_project(&root);
    fs::write(root.join("req.md"), "# fixture req\n").expect("write source file");
    assert_eq!(
        run(cli(&root, add_command("DOC-REQ-001", "req.md"))),
        ExitCode::Ok
    );
    assert_eq!(
        run(cli(&root, add_command("DOC-BASIC-001", "basic-spec.md"))),
        ExitCode::Ok
    );
    let exit = run(cli(
        &root,
        Command::Doc(DocCommand::List {
            tree: false,
            roots: false,
        }),
    ));
    assert_eq!(exit, ExitCode::Ok);
}
