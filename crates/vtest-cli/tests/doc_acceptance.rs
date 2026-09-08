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

/// @vtest.id TEST-DOC-ADD-REGISTERS
/// @vtest.covers VO-DOC-ADD-REGISTERS-NODE-TREE
/// @vtest.target crates/vtest-cli/src/ops/doc.rs::add
/// @vtest.intent doc add registers a --path node-tree file as one document and writes .verify/doc/<id>.json
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
/// @vtest.id TEST-DOC-ADD-DUPLICATE-ID
/// @vtest.covers VO-DOC-ADD-UPDATE-PARTITION
/// @vtest.target crates/vtest-cli/src/ops/doc.rs::add
/// @vtest.intent a second add for the same id without --update is a usage rejection
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
/// @vtest.id TEST-DOC-UPDATE-UNREGISTERED-ID
/// @vtest.covers VO-DOC-ADD-UPDATE-PARTITION
/// @vtest.target crates/vtest-cli/src/ops/doc.rs::add
/// @vtest.intent --update on an id that was never registered is a usage rejection
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
/// @vtest.id TEST-DOC-UPDATE-RECOMPUTES-HASH
/// @vtest.covers VO-DOC-UPDATE-RECOMPUTES-HASH
/// @vtest.target crates/vtest-cli/src/ops/doc.rs::add
/// @vtest.intent --update re-reads the current node-tree file so doc show's content reflects the new content
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
    assert_eq!(view.view.file.root[0].statement, "fixture root, changed");
}

/// A `--path` that does not resolve to a valid node-tree JSON file is
/// rejected (an internal store-layer error, not a silently-written record).
/// @vtest.id TEST-DOC-ADD-INVALID-NODE-TREE
/// @vtest.covers VO-DOC-ADD-REGISTERS-NODE-TREE
/// @vtest.target crates/vtest-cli/src/ops/doc.rs::add
/// @vtest.intent a --path that does not resolve to a valid node-tree JSON file is rejected and writes no record
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
/// @vtest.id TEST-DOC-LIST-AFTER-REGISTRATIONS
/// @vtest.covers VO-DOC-LIST-OUTPUTS-DOCUMENT-RECORDS
/// @vtest.target crates/vtest-cli/src/ops/doc.rs::list
/// @vtest.intent doc list after two registrations succeeds and exits 0
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

/// DS-1003/1681/1685/1686: `--derives-from` writes onto every top-level
/// node's own `derives_from` (here, the fixture's single `request[0]`
/// sentence node) — DS-1685's "置換（追記ではない）" and DS-1686's "一律
/// 適用" (PR #49, `24c3cbe`).
/// @vtest.id TEST-DOC-DERIVES-FROM-TOP-LEVEL
/// @vtest.covers VO-DOC-ADD-DERIVES-FROM-TOP-LEVEL-EDGE
/// @vtest.target crates/vtest-cli/src/ops/doc.rs::add
/// @vtest.intent --derives-from writes onto every top-level node's own derives_from
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
        view.view.file.request[0].derives_from,
        vec![vtest_model::DocumentId::new("ROOT-001")]
    );
}

/// DS-1685/1686: `--derives-from` **replaces** each top-level node's
/// existing `derives_from` (not appends to it, DS-1685) and applies the
/// same new id list **uniformly to every** top-level node (DS-1686), not
/// just the first one — a fixture with a single populated node cannot
/// distinguish "wrote onto every node" from "wrote onto the only node",
/// so this uses two request-layer nodes, each pre-populated with a
/// different existing value the new `--derives-from` call must overwrite.
/// @vtest.id TEST-DOC-DERIVES-FROM-REPLACE-UNIFORM
/// @vtest.covers VO-DOC-DERIVES-FROM-REPLACE-UNIFORM
/// @vtest.target crates/vtest-cli/src/ops/doc.rs::add
/// @vtest.intent --derives-from replaces each top-level node's existing derives_from and applies uniformly across multiple nodes
#[test]
fn derives_from_replaces_and_applies_uniformly_across_multiple_nodes() {
    let root = temp_root("derives-from-replace-uniform");
    init_project(&root, "vtest-doc-fixture").expect("init .verify/ layout");
    let with_two_requests = FIXTURE_NODE_TREE.replacen(
        r#""request": [],"#,
        r#""request": [
            {"id":"R-001","statement":"first","derives_from":["ROOT-001"],"source":{"doc":"fixture.md","heading":"fixture","lines":[1,1]}},
            {"id":"R-002","statement":"second","derives_from":[],"source":{"doc":"fixture.md","heading":"fixture","lines":[1,1]}}
        ],"#,
        1,
    );
    fs::write(root.join("basic-spec.json"), &with_two_requests).expect("write source file");

    let exit = run(cli(
        &root,
        Command::Doc(DocCommand::Add {
            id: "DOC-BASIC-001".to_owned(),
            path: "basic-spec.json".to_owned(),
            derives_from: vec!["ROOT-999".to_owned()],
            root: false,
            no_root: false,
            update: false,
        }),
    ));
    assert_eq!(exit, ExitCode::Ok);

    let layout = vtest_store::VerifyLayout::new(&root);
    let view = vtest_cli::ops::doc::show(&layout, "DOC-BASIC-001").expect("show must succeed");
    let expected = vec![vtest_model::DocumentId::new("ROOT-999")];
    assert_eq!(
        view.view.file.request[0].derives_from, expected,
        "R-001's prior derives_from (ROOT-001) must be replaced, not appended to"
    );
    assert_eq!(
        view.view.file.request[1].derives_from, expected,
        "DS-1686: the same new id list must apply uniformly to every top-level node, not \
         just the first"
    );
}

/// DS-1003/1687: `--derives-from` has no top-level node to attach to when
/// the document's only content is in the `root` layer (RootNode has no
/// `derives_from` field, DS-1592/1593) — this is a usage rejection, not a
/// silent no-op (PR #49, `24c3cbe`).
/// @vtest.id TEST-DOC-DERIVES-FROM-ROOT-ONLY
/// @vtest.covers VO-DOC-DERIVES-FROM-ROOT-ONLY-REJECTED
/// @vtest.target crates/vtest-cli/src/ops/doc.rs::add
/// @vtest.intent --derives-from on a document whose content is entirely root[] is a usage rejection, since RootNode has no derives_from field
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

/// DS-1685: `derives_from: None` (the argument not given at all) and
/// `derives_from: Some(&[])` (given, with zero ids) must behave
/// differently -- `None` leaves an existing `derives_from` untouched on
/// `--update`, `Some(empty)` replaces it with empty (clears it). The CLI's
/// own repeatable-value flag cannot express `Some(empty)` (see
/// `ops::doc::AddArgs::derives_from`'s doc comment), so this calls
/// `ops::doc::add` directly, the same function both the CLI and MCP
/// dispatch to.
/// @vtest.id TEST-DOC-DERIVES-FROM-NONE-VS-EMPTY
/// @vtest.covers VO-DOC-DERIVES-FROM-NONE-VS-EMPTY
/// @vtest.target crates/vtest-cli/src/ops/doc.rs::add
/// @vtest.intent derives_from: None leaves an existing derives_from untouched on --update, while Some(empty) clears it
#[test]
fn derives_from_none_vs_some_empty_are_distinct_on_update() {
    let root = temp_root("derives-from-none-vs-empty");
    init_project(&root, "vtest-doc-fixture").expect("init .verify/ layout");
    // The source file already declares derives_from: [ROOT-001] -- `--update`
    // re-reads `--path` fresh each call (DES-595's registration act), so
    // this fixture's own content, not a prior registration's mutated
    // state, is what `derives_from: None` must be shown to leave alone.
    let with_request = FIXTURE_NODE_TREE.replacen(
        r#""request": [],"#,
        r#""request": [{"id":"R-001","statement":"fixture requirement","derives_from":["ROOT-001"],"source":{"doc":"fixture.md","heading":"fixture","lines":[1,1]}}],"#,
        1,
    );
    fs::write(root.join("basic-spec.json"), &with_request).expect("write source file");
    let layout = vtest_store::VerifyLayout::new(&root);

    vtest_cli::ops::doc::add(
        &root,
        &layout,
        vtest_cli::ops::doc::AddArgs {
            id: "DOC-BASIC-001".to_owned(),
            path: "basic-spec.json".to_owned(),
            derives_from: None,
            root: false,
            root_specified: false,
            update: false,
        },
    )
    .expect("initial add must succeed");

    // `--update` with `derives_from: None` must leave it untouched.
    vtest_cli::ops::doc::add(
        &root,
        &layout,
        vtest_cli::ops::doc::AddArgs {
            id: "DOC-BASIC-001".to_owned(),
            path: "basic-spec.json".to_owned(),
            derives_from: None,
            root: false,
            root_specified: false,
            update: true,
        },
    )
    .expect("update with derives_from: None must succeed");
    let after_none = vtest_cli::ops::doc::show(&layout, "DOC-BASIC-001").expect("show");
    assert_eq!(
        after_none.view.file.request[0].derives_from,
        vec![vtest_model::DocumentId::new("ROOT-001")],
        "derives_from: None must leave the existing value untouched"
    );

    // `--update` with `derives_from: Some(&[])` must clear it.
    vtest_cli::ops::doc::add(
        &root,
        &layout,
        vtest_cli::ops::doc::AddArgs {
            id: "DOC-BASIC-001".to_owned(),
            path: "basic-spec.json".to_owned(),
            derives_from: Some(Vec::new()),
            root: false,
            root_specified: false,
            update: true,
        },
    )
    .expect("update with derives_from: Some(empty) must succeed");
    let after_empty = vtest_cli::ops::doc::show(&layout, "DOC-BASIC-001").expect("show");
    assert!(
        after_empty.view.file.request[0].derives_from.is_empty(),
        "derives_from: Some(empty) must clear the existing value, not leave it untouched"
    );
}

/// DS-1017 new: `doc show` reports `freshness` (node id -> `Option<bool>`
/// -- `Some(true)`/`Some(false)`/`None`, see `ShowResult`'s doc comment
/// for the full definition) and `approval_states` (node id ->
/// `draft`/`approved`, one entry per top-level node the document owns --
/// Approval's `document` subject_type binds per node, DS-1051). A
/// document with no Approval record for any of its nodes reads `draft`
/// for every one, and `None` (no comparison target) for freshness.
/// @vtest.id TEST-DOC-SHOW-FRESHNESS-AND-APPROVAL-STATES
/// @vtest.covers VO-DOC-SHOW-FRESHNESS-AND-APPROVAL-STATES
/// @vtest.target crates/vtest-cli/src/ops/doc.rs::show
/// @vtest.intent doc show reports freshness None and approval_states draft for a node with no Approval record
#[test]
fn show_reports_freshness_and_per_node_approval_states() {
    let root = temp_root("show-freshness-approval-states");
    build_fixture_project(&root); // FIXTURE_NODE_TREE's single node is ROOT-001
    assert_eq!(
        run(cli(&root, add_command("DOC-BASIC-001", "basic-spec.json"))),
        ExitCode::Ok
    );

    let layout = vtest_store::VerifyLayout::new(&root);
    let result = vtest_cli::ops::doc::show(&layout, "DOC-BASIC-001").expect("show must succeed");
    assert_eq!(
        result.freshness.get("ROOT-001").copied(),
        Some(None),
        "DS-1017 new: no Approval record depends on ROOT-001, so there is no comparison \
         target -- None, not rounded to Some(true)"
    );
    assert_eq!(
        result.approval_states.get("ROOT-001").map(String::as_str),
        Some("draft"),
        "a node with no Approval record must read draft"
    );
}

/// DS-1017 new: a node id that some *other* subject's Approval record
/// lists in its `dependencies[]`, with a hash matching that node's
/// *current* subject hash, reads `Some(true)` (fresh) -- the positive
/// comparison path, not just the "no comparison target" case above. Uses
/// a VO deriving from a request node that itself derives from ROOT-001,
/// so approving that VO records ROOT-001 as a dependency entry (via the
/// document ancestor closure, DS-1487) without ROOT-001 ever being an
/// approval *subject* itself.
/// @vtest.id TEST-DOC-SHOW-FRESHNESS-FRESH
/// @vtest.covers VO-DOC-SHOW-FRESHNESS-FRESH
/// @vtest.target crates/vtest-cli/src/ops/doc.rs::show
/// @vtest.intent a node id listed with a matching hash in an approval's dependency closure reads freshness Some(true)
#[test]
fn show_reports_fresh_when_a_dependency_entry_matches_the_current_hash() {
    use vtest_model::{DerivesFrom, DocumentId, VoId, VoRecord};

    let root = temp_root("show-freshness-fresh");
    init_project(&root, "vtest-doc-fixture").expect("init .verify/ layout");
    let with_request = FIXTURE_NODE_TREE.replacen(
        r#""request": [],"#,
        r#""request": [{"id":"R-001","statement":"fixture requirement","derives_from":["ROOT-001"],"source":{"doc":"fixture.md","heading":"fixture","lines":[1,1]}}],"#,
        1,
    );
    fs::write(root.join("basic-spec.json"), &with_request).expect("write source file");
    assert_eq!(
        run(cli(&root, add_command("DOC-BASIC-001", "basic-spec.json"))),
        ExitCode::Ok
    );

    let layout = vtest_store::VerifyLayout::new(&root);
    vtest_store::write_vo_record(
        &layout,
        &VoRecord {
            id: VoId::new("VO-FRESHNESS-CHECK"),
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
            created: "2026-09-10T00:00:00Z".to_owned(),
            updated: "2026-09-10T00:00:00Z".to_owned(),
        },
    )
    .expect("write VO record");

    vtest_cli::ops::approval::create(
        &layout,
        vtest_cli::ops::approval::CreateArgs {
            subject_type: "vo".to_owned(),
            subject_id: "VO-FRESHNESS-CHECK".to_owned(),
            approved_state: "approved".to_owned(),
            approver_kind: "human".to_owned(),
            approver_id: "reviewer".to_owned(),
            approver_model: None,
            basis: Vec::new(),
            supersedes: Vec::new(),
        },
    )
    .expect("approval create must succeed against a resolvable VO subject");

    let result = vtest_cli::ops::doc::show(&layout, "DOC-BASIC-001").expect("show must succeed");
    assert_eq!(
        result.freshness.get("ROOT-001").copied(),
        Some(Some(true)),
        "the just-written VO approval's dependency closure includes ROOT-001 with its \
         current hash, so ROOT-001 must read fresh (Some(true)), not no-comparison-target"
    );
}

/// DS-1017 new: once an Approval record's recorded dependency hash for a
/// node no longer matches that node's *current* subject hash (DES-572 --
/// changing `statement` changes the hash, DS-1601/1605), the node reads
/// `Some(false)` (stale), not `Some(true)` or `None`. Same fixture as
/// `show_reports_fresh_when_a_dependency_entry_matches_the_current_hash`,
/// with ROOT-001's own `statement` changed and re-registered (`--update`)
/// *after* the approval was written, so the approval's dependency entry
/// is now stale relative to the node it names.
/// @vtest.id TEST-DOC-SHOW-FRESHNESS-STALE
/// @vtest.covers VO-DOC-SHOW-FRESHNESS-STALE
/// @vtest.target crates/vtest-cli/src/ops/doc.rs::show
/// @vtest.intent a node whose current subject hash no longer matches an approval's recorded dependency hash reads freshness Some(false)
#[test]
fn show_reports_stale_when_a_dependency_entry_no_longer_matches() {
    use vtest_model::{DerivesFrom, DocumentId, VoId, VoRecord};

    let root = temp_root("show-freshness-stale");
    init_project(&root, "vtest-doc-fixture").expect("init .verify/ layout");
    let with_request = FIXTURE_NODE_TREE.replacen(
        r#""request": [],"#,
        r#""request": [{"id":"R-001","statement":"fixture requirement","derives_from":["ROOT-001"],"source":{"doc":"fixture.md","heading":"fixture","lines":[1,1]}}],"#,
        1,
    );
    fs::write(root.join("basic-spec.json"), &with_request).expect("write source file");
    assert_eq!(
        run(cli(&root, add_command("DOC-BASIC-001", "basic-spec.json"))),
        ExitCode::Ok
    );

    let layout = vtest_store::VerifyLayout::new(&root);
    vtest_store::write_vo_record(
        &layout,
        &VoRecord {
            id: VoId::new("VO-FRESHNESS-STALE"),
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
            created: "2026-09-10T00:00:00Z".to_owned(),
            updated: "2026-09-10T00:00:00Z".to_owned(),
        },
    )
    .expect("write VO record");
    vtest_cli::ops::approval::create(
        &layout,
        vtest_cli::ops::approval::CreateArgs {
            subject_type: "vo".to_owned(),
            subject_id: "VO-FRESHNESS-STALE".to_owned(),
            approved_state: "approved".to_owned(),
            approver_kind: "human".to_owned(),
            approver_id: "reviewer".to_owned(),
            approver_model: None,
            basis: Vec::new(),
            supersedes: Vec::new(),
        },
    )
    .expect("approval create must succeed against a resolvable VO subject");

    // Change ROOT-001's normative content and re-register -- its subject
    // hash changes, but the approval's dependency entry above was
    // snapshotted before this change.
    let changed = with_request.replace("fixture root", "fixture root, changed");
    fs::write(root.join("basic-spec.json"), changed).expect("rewrite source file");
    assert_eq!(
        run(cli(
            &root,
            update_command("DOC-BASIC-001", "basic-spec.json")
        )),
        ExitCode::Ok
    );

    let result = vtest_cli::ops::doc::show(&layout, "DOC-BASIC-001").expect("show must succeed");
    assert_eq!(
        result.freshness.get("ROOT-001").copied(),
        Some(Some(false)),
        "ROOT-001's current subject hash no longer matches the approval's recorded \
         dependency hash, so it must read stale (Some(false)), not fresh or no-comparison-target"
    );
}

/// DS-1017 new "各トップレベルノード": a section node's own `items`
/// children must NOT appear in `approval_states`/`freshness` -- only the
/// section's own id (a document's actual top-level node set), regardless
/// of how many nested sentence nodes it declares. Regression for the
/// earlier round's over-inclusive deep walk (`document_node_ids` reused
/// `collect_all_ids`, meant for a different purpose -- resolving
/// arbitrary `derives_from` targets anywhere in the corpus -- and picked
/// up nested `items`/`sections` children too).
/// @vtest.id TEST-DOC-SHOW-EXCLUDES-SECTION-ITEMS
/// @vtest.covers VO-DOC-SHOW-FRESHNESS-AND-APPROVAL-STATES
/// @vtest.target crates/vtest-cli/src/ops/doc.rs::show
/// @vtest.intent approval_states and freshness contain exactly the document's top-level node ids, not nested section item children
#[test]
fn show_states_and_freshness_exclude_section_item_children() {
    let root = temp_root("show-top-level-only");
    init_project(&root, "vtest-doc-fixture").expect("init .verify/ layout");
    let with_section_items = r#"{"schema_version":"0.1","root":[{"id":"ROOT-001","statement":"fixture root","source":{"doc":"fixture.md","heading":"fixture","lines":[1,1]}}],"request":[],"require":[],"spec":[{"id":"SPEC-001","title":"fixture section","source":{"doc":"fixture.md","heading":"fixture","lines":[1,1]},"items":[{"id":"SPEC-002","statement":"nested item","derives_from":[],"source":{"doc":"fixture.md","heading":"fixture","lines":[1,1]}}]}],"detailed_spec":[],"basic_design":[],"design":[]}"#;
    fs::write(root.join("basic-spec.json"), with_section_items).expect("write source file");
    assert_eq!(
        run(cli(&root, add_command("DOC-BASIC-001", "basic-spec.json"))),
        ExitCode::Ok
    );

    let layout = vtest_store::VerifyLayout::new(&root);
    let result = vtest_cli::ops::doc::show(&layout, "DOC-BASIC-001").expect("show must succeed");

    let approval_keys: std::collections::BTreeSet<&str> =
        result.approval_states.keys().map(String::as_str).collect();
    let freshness_keys: std::collections::BTreeSet<&str> =
        result.freshness.keys().map(String::as_str).collect();
    let expected: std::collections::BTreeSet<&str> = ["ROOT-001", "SPEC-001"].into();

    assert_eq!(
        approval_keys, expected,
        "approval_states must contain exactly the document's top-level node ids (ROOT-001, \
         SPEC-001), not the nested SPEC-002 item"
    );
    assert_eq!(
        freshness_keys, expected,
        "freshness must contain exactly the document's top-level node ids (ROOT-001, \
         SPEC-001), not the nested SPEC-002 item"
    );
}

/// DS-1683/DS-1658: `--root` succeeds when the source file's own top-level
/// content is entirely `root[]` (already `ROOT-`-prefixed, matching
/// DS-1658's id-prefix-to-layer rule) — it validates placement, it does
/// not convert a non-root node into one.
/// @vtest.id TEST-DOC-ROOT-FLAG-ACCEPTS-ALL-ROOT
/// @vtest.covers VO-DOC-ROOT-FLAG-LAYER-VALIDATION
/// @vtest.target crates/vtest-cli/src/ops/doc.rs::add
/// @vtest.intent --root succeeds when the source file's top-level content is entirely root[]
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
    assert!(view.view.is_root);
}

/// DS-1683/DS-1658: `--root` is rejected against a source file whose
/// top-level content is a non-root layer (`request` here) — DS-1658 ties
/// the node's `R-…` id prefix to the `request` layer, so `--root` cannot
/// move it into `root[]` without fabricating it a `ROOT-…` id.
/// @vtest.id TEST-DOC-ROOT-FLAG-NON-ROOT-LAYER
/// @vtest.covers VO-DOC-ROOT-FLAG-LAYER-VALIDATION
/// @vtest.target crates/vtest-cli/src/ops/doc.rs::add
/// @vtest.intent --root is rejected against a source file whose top-level content is a non-root layer
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

/// DS-1195: `root` is a plain bool, so `--no-root` and omitting both flags
/// are now behaviourally identical (no assertion) -- `--no-root` against a
/// source file whose own `root[]` is non-empty succeeds, the same as
/// registering it with neither flag given (see `apply_root`'s doc
/// comment; the previous round's separate "explicit --no-root asserts
/// root[] is empty" behavior was folded away by the `Option<bool>` ->
/// `bool` correction, since DS-1195 gives `root` no third state to carry
/// that assertion).
/// @vtest.id TEST-DOC-NO-ROOT-FLAG-NOOP
/// @vtest.covers VO-DOC-NO-ROOT-FLAG-NOOP
/// @vtest.target crates/vtest-cli/src/ops/doc.rs::add
/// @vtest.intent --no-root against a source file whose root[] is non-empty succeeds identically to omitting both flags
#[test]
fn no_root_flag_is_a_no_op_identical_to_omitting_both_flags() {
    let root = temp_root("no-root-flag-noop");
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
    assert_eq!(exit, ExitCode::Ok);
}

/// DS-1683/BD-331 (`233caec`/PR #50): root designation is fixed at
/// initial registration only -- `--root`/`--no-root` combined with
/// `--update` is a usage rejection, not a silently-applied or silently
/// -ignored change (the retired DS-1014 was the only ground for allowing
/// `--update` to also change root designation).
/// @vtest.id TEST-DOC-ROOT-FLAG-WITH-UPDATE
/// @vtest.covers VO-DOC-UPSERT-ROOT-FIXED-AT-REGISTRATION
/// @vtest.target crates/vtest-cli/src/ops/doc.rs::add
/// @vtest.intent --root or --no-root combined with --update is a usage rejection, since root designation is fixed at initial registration only
#[test]
fn root_flag_combined_with_update_is_a_usage_error() {
    let root = temp_root("root-flag-with-update");
    build_fixture_project(&root);
    assert_eq!(
        run(cli(&root, add_command("DOC-BASIC-001", "basic-spec.json"))),
        ExitCode::Ok
    );

    let exit = run(cli(
        &root,
        Command::Doc(DocCommand::Add {
            id: "DOC-BASIC-001".to_owned(),
            path: "basic-spec.json".to_owned(),
            derives_from: Vec::new(),
            root: true,
            no_root: false,
            update: true,
        }),
    ));
    assert_eq!(
        exit,
        ExitCode::Usage,
        "--root combined with --update must be rejected, not silently applied"
    );

    let exit = run(cli(
        &root,
        Command::Doc(DocCommand::Add {
            id: "DOC-BASIC-001".to_owned(),
            path: "basic-spec.json".to_owned(),
            derives_from: Vec::new(),
            root: false,
            no_root: true,
            update: true,
        }),
    ));
    assert_eq!(
        exit,
        ExitCode::Usage,
        "--no-root combined with --update must be rejected too, not silently ignored"
    );
}

/// DS-1015: `doc list --tree` renders the `derives_from` chain as a nested
/// tree (a document with no `derives_from` edges at depth 0, its dependents
/// indented beneath it), not a flat `id -> [parents]` listing.
/// @vtest.id TEST-DOC-LIST-TREE-NESTED
/// @vtest.covers VO-DOC-LIST-TREE-NESTED
/// @vtest.target crates/vtest-cli/src/lib.rs::render_doc_tree
/// @vtest.intent doc list --tree renders the derives_from chain as a nested tree, not a flat listing
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

/// A document reachable only through a cycle (every document on the cycle
/// has *some* `derives_from` edge, so none of them is a display root) must
/// not silently vanish from `--tree`'s output -- it must still appear
/// somewhere, rather than being dropped because it was never reached by a
/// walk starting from an actual root.
#[test]
fn list_tree_does_not_drop_a_document_reachable_only_through_a_cycle() {
    use std::collections::BTreeMap;
    let mut chain = BTreeMap::new();
    chain.insert("DOC-A".to_owned(), vec!["DOC-B".to_owned()]);
    chain.insert("DOC-B".to_owned(), vec!["DOC-A".to_owned()]);
    let rendered = vtest_cli::render_doc_tree(&chain);
    assert!(
        rendered.contains("DOC-A") && rendered.contains("DOC-B"),
        "both documents in the cycle must appear in the rendered tree, got:\n{rendered}"
    );
}

/// DS-1016: `doc list --roots` lists the current `root[]`-layer document
/// set.
/// @vtest.id TEST-DOC-LIST-ROOTS
/// @vtest.covers VO-DOC-LIST-ROOTS
/// @vtest.target crates/vtest-cli/src/ops/doc.rs::list
/// @vtest.intent doc list --roots lists the current root[]-layer document set
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

/// DS-1017 new/DS-1194: `doc list`'s own `freshness` field must carry the
/// actual expected per-node values, not merely be internally consistent
/// with itself (the two-function comparison this crate's `node_freshness`
/// shares with `doc show` would pass even if both sides made the same
/// mistake) -- so every expected value here is hand-derived from the
/// fixture's own construction, independent of the function under test.
/// @vtest.id TEST-DOC-LIST-FRESHNESS-VALUES
/// @vtest.covers VO-DOC-LIST-FRESHNESS-VALUES
/// @vtest.target crates/vtest-cli/src/ops/doc.rs::list
/// @vtest.intent doc list's freshness field carries hand-derived expected per-node values, not merely internal self-consistency
#[test]
fn list_reports_the_expected_freshness_values_per_document() {
    use vtest_model::{DerivesFrom, DocumentId, VoId, VoRecord};

    let root = temp_root("list-freshness-values");
    init_project(&root, "vtest-doc-fixture").expect("init .verify/ layout");
    let with_request = FIXTURE_NODE_TREE.replacen(
        r#""request": [],"#,
        r#""request": [{"id":"R-001","statement":"fixture requirement","derives_from":["ROOT-001"],"source":{"doc":"fixture.md","heading":"fixture","lines":[1,1]}}],"#,
        1,
    );
    fs::write(root.join("basic-spec.json"), &with_request).expect("write source file");
    assert_eq!(
        run(cli(&root, add_command("DOC-BASIC-001", "basic-spec.json"))),
        ExitCode::Ok
    );

    let layout = vtest_store::VerifyLayout::new(&root);

    // No Approval record exists yet: every node must have no comparison
    // target (None), not Some(true)/Some(false).
    let before = vtest_cli::ops::doc::list(&layout).expect("list must succeed");
    assert_eq!(
        before.freshness.get("DOC-BASIC-001"),
        Some(&std::collections::BTreeMap::from([
            ("ROOT-001".to_owned(), None),
            ("R-001".to_owned(), None),
        ])),
        "before any Approval record exists, every node must read no-comparison-target"
    );

    // Approve VO-LIST-FRESHNESS (deriving from R-001, whose own ancestor
    // closure includes ROOT-001) -- both nodes now have a current
    // dependency entry, so both must read fresh (Some(true)).
    vtest_store::write_vo_record(
        &layout,
        &VoRecord {
            id: VoId::new("VO-LIST-FRESHNESS"),
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
            created: "2026-09-10T00:00:00Z".to_owned(),
            updated: "2026-09-10T00:00:00Z".to_owned(),
        },
    )
    .expect("write VO record");
    vtest_cli::ops::approval::create(
        &layout,
        vtest_cli::ops::approval::CreateArgs {
            subject_type: "vo".to_owned(),
            subject_id: "VO-LIST-FRESHNESS".to_owned(),
            approved_state: "approved".to_owned(),
            approver_kind: "human".to_owned(),
            approver_id: "reviewer".to_owned(),
            approver_model: None,
            basis: Vec::new(),
            supersedes: Vec::new(),
        },
    )
    .expect("approval create must succeed against a resolvable VO subject");

    let after = vtest_cli::ops::doc::list(&layout).expect("list must succeed");
    assert_eq!(
        after.freshness.get("DOC-BASIC-001"),
        Some(&std::collections::BTreeMap::from([
            ("ROOT-001".to_owned(), Some(true)),
            ("R-001".to_owned(), Some(true)),
        ])),
        "after approving a VO whose dependency closure covers both nodes with their current \
         hashes, both must read fresh (Some(true))"
    );
}
