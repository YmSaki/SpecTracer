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
        root: false,
        no_root: false,
        update: false,
    })
}

fn update_command(id: &str, path: &str) -> Command {
    Command::Doc(DocCommand::Add {
        id: id.to_owned(),
        path: path.to_owned(),
        derives_from: Vec::new(),
        root: false,
        no_root: false,
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
            root: false,
            no_root: false,
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
            root: false,
            no_root: false,
            update: false,
        }),
    ));
    assert_eq!(exit, ExitCode::Usage);
}

/// DS-1683/DS-1658: `--root` succeeds when the source file's own top-level
/// content is entirely `root[]` (already `ROOT-`-prefixed, matching
/// DS-1658's id-prefix-to-layer rule) — it validates placement, it does
/// not convert a non-root node into one.
#[test]
fn root_flag_accepts_a_source_file_that_is_already_all_root() {
    let root = temp_root("root-flag-accept");
    build_fixture_project(&root); // FIXTURE_NODE_TREE is entirely root[] (ROOT-001)

    let exit = run(cli(
        &root,
        Command::Doc(DocCommand::Add {
            id: "DOC-BASIC-001".to_owned(),
            path: "basic-spec.json".to_owned(),
            derives_from: Vec::new(),
            root: true,
            no_root: false,
            update: false,
        }),
    ));
    assert_eq!(exit, ExitCode::Ok);

    let layout = vtest_store::VerifyLayout::new(&root);
    let view = vtest_cli::ops::doc::show(&layout, "DOC-BASIC-001").expect("show must succeed");
    assert!(view.is_root);
}

/// DS-1683/DS-1658: `--root` is rejected against a source file whose
/// top-level content is a non-root layer (`request` here) — DS-1658 ties
/// the node's `R-…` id prefix to the `request` layer, so `--root` cannot
/// move it into `root[]` without fabricating it a `ROOT-…` id.
#[test]
fn root_flag_on_a_non_root_layer_document_is_a_usage_error() {
    let root = temp_root("root-flag-non-root-layer");
    init_project(&root, "vtest-doc-fixture").expect("init .verify/ layout");
    let request_only = r#"{"schema_version":"0.1","root":[],"request":[{"id":"R-001","statement":"fixture requirement","derives_from":[],"source":{"doc":"fixture.md","heading":"fixture","lines":[1,1]}}],"require":[],"spec":[],"detailed_spec":[],"basic_design":[],"design":[]}"#;
    fs::write(root.join("req-only.json"), request_only).expect("write source file");

    let exit = run(cli(
        &root,
        Command::Doc(DocCommand::Add {
            id: "DOC-REQ-001".to_owned(),
            path: "req-only.json".to_owned(),
            derives_from: Vec::new(),
            root: true,
            no_root: false,
            update: false,
        }),
    ));
    assert_eq!(exit, ExitCode::Usage);
}

/// DS-1683: `--no-root` is rejected against a source file whose own
/// `root[]` is already non-empty (reconstructing what it would take to
/// move those nodes out is undefined).
#[test]
fn no_root_flag_on_an_already_root_document_is_a_usage_error() {
    let root = temp_root("no-root-flag-already-root");
    build_fixture_project(&root); // FIXTURE_NODE_TREE's root[] has ROOT-001
    let exit = run(cli(
        &root,
        Command::Doc(DocCommand::Add {
            id: "DOC-BASIC-001".to_owned(),
            path: "basic-spec.json".to_owned(),
            derives_from: Vec::new(),
            root: false,
            no_root: true,
            update: false,
        }),
    ));
    assert_eq!(exit, ExitCode::Usage);
}

/// DS-1015: `doc list --tree` renders the `derives_from` chain as a nested
/// tree (a document with no `derives_from` edges at depth 0, its dependents
/// indented beneath it), not a flat `id -> [parents]` listing.
#[test]
fn list_tree_renders_a_nested_derives_from_tree() {
    let root = temp_root("list-tree");
    init_project(&root, "vtest-doc-fixture").expect("init .verify/ layout");

    // DOC-PARENT: no derives_from (a display root). DOC-CHILD: derives_from
    // ROOT-001 (an id outside the registered set, unresolved but harmless
    // to the tree itself, which is keyed on the registered ids).
    let parent = r#"{"schema_version":"0.1","root":[],"request":[{"id":"R-101","statement":"parent","derives_from":[],"source":{"doc":"fixture.md","heading":"fixture","lines":[1,1]}}],"require":[],"spec":[],"detailed_spec":[],"basic_design":[],"design":[]}"#;
    fs::write(root.join("parent.json"), parent).expect("write source file");
    assert_eq!(
        run(cli(&root, add_command("DOC-PARENT", "parent.json"))),
        ExitCode::Ok
    );

    let child = r#"{"schema_version":"0.1","root":[],"request":[{"id":"R-102","statement":"child","derives_from":["R-101"],"source":{"doc":"fixture.md","heading":"fixture","lines":[1,1]}}],"require":[],"spec":[],"detailed_spec":[],"basic_design":[],"design":[]}"#;
    fs::write(root.join("child.json"), child).expect("write source file");
    assert_eq!(
        run(cli(&root, add_command("DOC-CHILD", "child.json"))),
        ExitCode::Ok
    );

    let layout = vtest_store::VerifyLayout::new(&root);
    let result = vtest_cli::ops::doc::list(&layout).expect("list must succeed");
    // `render_doc_tree` is exactly the rendering `doc list --tree` uses on
    // this same resolved document-level chain (see `render_doc_list_text`).
    let rendered = vtest_cli::render_doc_tree(&result.document_chain);

    let parent_line = rendered.lines().find(|line| line.trim() == "DOC-PARENT");
    let child_line = rendered.lines().find(|line| line.contains("DOC-CHILD"));
    assert!(parent_line.is_some(), "rendered tree:\n{rendered}");
    assert!(child_line.is_some(), "rendered tree:\n{rendered}");
    let child_indent = child_line.unwrap().len() - child_line.unwrap().trim_start().len();
    assert!(
        child_indent > 0,
        "DOC-CHILD must be indented under DOC-PARENT in the tree, got:\n{rendered}"
    );
}

/// DS-1016: `doc list --roots` lists the current `root[]`-layer document
/// set.
#[test]
fn list_roots_lists_the_current_root_set() {
    let root = temp_root("list-roots");
    build_fixture_project(&root);
    assert_eq!(
        run(cli(&root, add_command("DOC-BASIC-001", "basic-spec.json"))),
        ExitCode::Ok
    );

    let layout = vtest_store::VerifyLayout::new(&root);
    let result = vtest_cli::ops::doc::list(&layout).expect("list must succeed");
    let roots: Vec<_> = result
        .records
        .iter()
        .filter(|view| view.is_root)
        .map(|view| view.id.clone())
        .collect();
    assert_eq!(roots, vec!["DOC-BASIC-001".to_owned()]);
}
