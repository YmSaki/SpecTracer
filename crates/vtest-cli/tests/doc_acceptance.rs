//! Acceptance coverage for `vtest doc add|list|show` (本冊 §3.1,
//! DES-585/586/595, DS-1015-1017/1681-1684).

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
        derives_from: Vec::new(),
        update: false,
    })
}

fn update_command(id: &str, path: &str) -> Command {
    Command::Doc(DocCommand::Add {
        id: id.to_owned(),
        path: path.to_owned(),
        derives_from: Vec::new(),
        update: true,
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
        .join("DOC-BASIC-001.json")
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
        update_command("DOC-BASIC-001", "basic-spec.json"),
    ));
    assert_eq!(exit, ExitCode::Usage);
}

/// DS-1684: `--update` re-reads the current node-tree file, so `doc show`'s
/// content_hash (computed live, DES-595) reflects the new content.
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
        update_command("DOC-BASIC-001", "basic-spec.json"),
    ));
    assert_eq!(update, ExitCode::Ok);

    let layout = vtest_store::VerifyLayout::new(&root);
    let view = vtest_cli::ops::doc::show(&layout, "DOC-BASIC-001").expect("show must succeed");
    assert_eq!(view.file.root[0].statement, "fixture root, changed");
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
        .join("DOC-BASIC-001.json")
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

/// DS-1003/1681: `--derives-from` writes onto every top-level node's own
/// `derives_from` (here, the fixture's single `request[0]` sentence node).
#[test]
fn derives_from_writes_onto_every_top_level_node() {
    let root = temp_root("derives-from");
    init_project(&root, "vtest-doc-fixture").expect("init .verify/ layout");
    let with_request = FIXTURE_NODE_TREE.replacen(
        r#""request": [],"#,
        r#""request": [{"id":"R-001","statement":"fixture requirement","derives_from":[],"source":{"doc":"fixture.md","heading":"fixture","lines":[1,1]}}],"#,
        1,
    );
    fs::write(root.join("basic-spec.json"), &with_request).expect("write source file");

    let exit = run(cli(
        &root,
        Command::Doc(DocCommand::Add {
            id: "DOC-BASIC-001".to_owned(),
            path: "basic-spec.json".to_owned(),
            derives_from: vec!["ROOT-001".to_owned()],
            update: false,
        }),
    ));
    assert_eq!(exit, ExitCode::Ok);

    let layout = vtest_store::VerifyLayout::new(&root);
    let view = vtest_cli::ops::doc::show(&layout, "DOC-BASIC-001").expect("show must succeed");
    assert_eq!(
        view.file.request[0].derives_from,
        vec![vtest_model::DocumentId::new("ROOT-001")]
    );
}

/// DS-1003: `--derives-from` has no top-level node to attach to when the
/// document's only content is in the `root` layer (RootNode has no
/// `derives_from` field, DS-1592/1593) — this is a usage rejection, not a
/// silent no-op.
#[test]
fn derives_from_on_a_root_only_document_is_a_usage_error() {
    let root = temp_root("derives-from-root-only");
    build_fixture_project(&root);
    let exit = run(cli(
        &root,
        Command::Doc(DocCommand::Add {
            id: "DOC-BASIC-001".to_owned(),
            path: "basic-spec.json".to_owned(),
            derives_from: vec!["ROOT-001".to_owned()],
            update: false,
        }),
    ));
    assert_eq!(exit, ExitCode::Usage);
}
