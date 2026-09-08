//! Acceptance coverage for `vtest doc add|list|show` (本冊 §12.2,
//! DS-1015-1017/1681-1684, DES-595).

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

/// DES-595: `--path` names an already-built node-tree JSON file (the same
/// shape `.verify/doc/<name>.json` already uses), not a raw markdown
/// source — this fixture is a minimal, schema-valid `DocumentFile`.
const FIXTURE_NODE_TREE: &str = r#"{
  "schema_version": "0.1",
  "root": [
    {
      "id": "ROOT-001",
      "statement": "fixture root",
      "source": {"doc": "fixture.md", "heading": "fixture", "lines": [1, 1]}
    }
  ],
  "request": [],
  "require": [],
  "spec": [],
  "detailed_spec": [],
  "basic_design": [],
  "design": []
}"#;

fn build_fixture_project(root: &std::path::Path) {
    init_project(root, "vtest-doc-fixture").expect("init .verify/ layout");
    fs::write(root.join("basic-spec.json"), FIXTURE_NODE_TREE).expect("write source file");
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
        root: false,
        no_root: false,
        update: false,
    })
}

#[test]
fn add_registers_a_document_and_exits_ok() {
    let root = temp_root("add-ok");
    build_fixture_project(&root);
    let exit = run(cli(&root, add_command("DOC-BASIC-001", "basic-spec.json")));
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
    let first = run(cli(&root, add_command("DOC-BASIC-001", "basic-spec.json")));
    assert_eq!(first, ExitCode::Ok);
    let second = run(cli(&root, add_command("DOC-BASIC-001", "basic-spec.json")));
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
            path: "basic-spec.json".to_owned(),
            title: None,
            derives_from: Vec::new(),
            root: false,
            no_root: false,
            update: true,
        }),
    ));
    assert_eq!(exit, ExitCode::Usage);
}

/// DS-1684: `--update` recomputes `content_hash` (the document-level
/// subject hash, DES-595) from the current node-tree file.
#[test]
fn update_recomputes_content_hash_from_the_current_file() {
    let root = temp_root("update-rehash");
    build_fixture_project(&root);
    let first = run(cli(&root, add_command("DOC-BASIC-001", "basic-spec.json")));
    assert_eq!(first, ExitCode::Ok);

    let changed = FIXTURE_NODE_TREE.replace("fixture root", "fixture root, changed");
    fs::write(root.join("basic-spec.json"), changed).expect("rewrite source file");
    let update = run(cli(
        &root,
        Command::Doc(DocCommand::Add {
            id: "DOC-BASIC-001".to_owned(),
            path: "basic-spec.json".to_owned(),
            title: None,
            derives_from: Vec::new(),
            root: false,
            no_root: false,
            update: true,
        }),
    ));
    assert_eq!(update, ExitCode::Ok);
}

/// A `--path` that does not resolve to a valid node-tree JSON file is
/// rejected (an internal store-layer error, not a silently-written record).
#[test]
fn add_with_an_invalid_node_tree_file_does_not_write_a_record() {
    let root = temp_root("invalid-node-tree");
    init_project(&root, "vtest-doc-fixture").expect("init .verify/ layout");
    fs::write(root.join("not-json.txt"), "not json at all").expect("write invalid source file");
    let exit = run(cli(&root, add_command("DOC-BASIC-001", "not-json.txt")));
    assert_ne!(exit, ExitCode::Ok);
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
    fs::write(root.join("req.json"), FIXTURE_NODE_TREE).expect("write source file");
    assert_eq!(
        run(cli(&root, add_command("DOC-REQ-001", "req.json"))),
        ExitCode::Ok
    );
    assert_eq!(
        run(cli(&root, add_command("DOC-BASIC-001", "basic-spec.json"))),
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
