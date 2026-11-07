//! Acceptance coverage for `vtest vo add|edit|list|show|expand|approve`
//! (別紙A §12.2, DS-1021〜DS-1048). Every case drives the real binary so the
//! order-sensitive `--derives-from` / `--anchor` binding is exercised as a
//! user types it.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

use serde_json::Value;
use vtest_model::{DocumentFile, DocumentId, NodeSource, RootNode, SentenceNode};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn source(id: &str) -> NodeSource {
    NodeSource {
        doc: format!("{id}.md"),
        heading: "fixture".to_owned(),
        lines: [1, 1],
    }
}

/// A project with two document chains: ROOT-001 → R-001 and ROOT-002 →
/// R-002.
fn fixture(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "vtest-vo-commands-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).unwrap();
    let layout = vtest_store::init_project(&root, "vo-commands").unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"vo_commands\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), "pub fn target() {}\n").unwrap();
    let sentence = |id: &str, parent: &str| SentenceNode {
        id: DocumentId::new(id),
        statement: format!("{id} statement"),
        description: None,
        derives_from: vec![DocumentId::new(parent)],
        cites: None,
        source: source(id),
    };
    let file = DocumentFile {
        schema_version: "0.1".to_owned(),
        root: ["ROOT-001", "ROOT-002"]
            .iter()
            .map(|id| RootNode {
                id: DocumentId::new(*id),
                statement: format!("{id} root"),
                description: None,
                source: source(id),
            })
            .collect(),
        request: vec![sentence("R-001", "ROOT-001"), sentence("R-002", "ROOT-002")],
        require: Vec::new(),
        spec: Vec::new(),
        detailed_spec: Vec::new(),
        basic_design: Vec::new(),
        design: Vec::new(),
    };
    vtest_store::write_document_file(&layout, "fixture", &file).unwrap();
    root
}

fn vtest(root: &Path, args: &[&str]) -> (i32, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_vtest"))
        .arg("--project")
        .arg(root)
        .args(["--format", "json"])
        .args(args)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let envelope = serde_json::from_str(&stdout).unwrap_or_else(|error| {
        panic!(
            "non-JSON output ({error}): {stdout}\nstderr: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (output.status.code().unwrap_or(-1), envelope)
}

fn vo_files(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(root.join(".verify/vo"))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".yaml"))
        .collect();
    names.sort();
    names
}

fn add_basic(root: &Path, id: &str, doc: &str) {
    let (code, envelope) = vtest(
        root,
        &[
            "vo",
            "add",
            "--id",
            id,
            "--claim",
            "c",
            "--derives-from",
            doc,
        ],
    );
    assert_eq!(code, 0, "{envelope}");
}

fn code_of(envelope: &Value) -> &str {
    envelope["diagnostics"][0]["code"].as_str().unwrap_or("")
}

/// @vtest.id TEST-CLI-VO-ANCHOR-BINDING
/// @vtest.covers VO-CLI-VO-ANCHOR-BINDING
/// @vtest.target crates/vtest-cli/src/ops/vo.rs::parse_vo_args
/// @vtest.intent each --anchor/--note binds to the preceding --derives-from; an unbound or second --anchor is exit 2 with no record written
#[test]
fn anchors_bind_to_the_preceding_derives_from() {
    let root = fixture("anchor");
    let (code, envelope) = vtest(
        &root,
        &[
            "vo",
            "add",
            "--id",
            "VO-A",
            "--claim",
            "c",
            "--derives-from",
            "R-001",
            "--anchor",
            "§1",
            "--note",
            "n1",
            "--derives-from",
            "R-002",
        ],
    );
    assert_eq!(code, 0, "{envelope}");
    let (_, shown) = vtest(&root, &["vo", "show", "VO-A"]);
    let derives = &shown["data"]["record"]["derives_from"];
    assert_eq!(derives[0]["doc"], "R-001");
    assert_eq!(derives[0]["anchor"], "§1");
    assert_eq!(derives[0]["note"], "n1");
    assert!(derives[1].get("anchor").is_none());

    for args in [
        vec![
            "vo",
            "add",
            "--id",
            "VO-B",
            "--claim",
            "c",
            "--anchor",
            "§1",
            "--derives-from",
            "R-001",
        ],
        vec![
            "vo",
            "add",
            "--id",
            "VO-B",
            "--claim",
            "c",
            "--derives-from",
            "R-001",
            "--anchor",
            "§1",
            "--anchor",
            "§2",
        ],
    ] {
        let (code, envelope) = vtest(&root, &args);
        assert_eq!(code, 2, "{envelope}");
        assert_eq!(code_of(&envelope), "E-OP-001");
    }
    assert_eq!(
        vo_files(&root),
        vec!["VO-A.yaml"],
        "no record for rejected adds"
    );
}

/// @vtest.id TEST-CLI-VO-INVALID-COMBINATIONS-REJECTED
/// @vtest.covers VO-CLI-VO-COMBINATIONS-REJECTED
/// @vtest.target crates/vtest-cli/src/ops/vo.rs::add
/// @vtest.intent vo add and vo edit with combinations invalid for the declared dimensions are E-SCAN-017 / exit 2 and leave the records unchanged
#[test]
fn invalid_combinations_are_rejected_without_writing() {
    let root = fixture("bad-combinations");
    let base = [
        "vo",
        "add",
        "--id",
        "VO-X",
        "--claim",
        "c",
        "--derives-from",
        "R-001",
        "--dimension",
        "sign=positive,negative",
        "--dimension",
        "op=add,div",
    ];
    for extra in [
        vec!["--policy", "explicit"], // explicit, no tuple
        vec![
            "--policy",
            "full-product",
            "--combination",
            "sign=positive,op=add",
        ], // non-explicit
        vec!["--policy", "explicit", "--combination", "sign=zero,op=add"], // unknown partition
        vec!["--policy", "explicit", "--combination", "sign=positive"], // missing dimension
        vec![
            "--policy",
            "explicit",
            "--combination",
            "sign=positive,op=add",
            "--combination",
            "op=add,sign=positive",
        ], // duplicate tuple
    ] {
        let args: Vec<&str> = base.iter().copied().chain(extra.iter().copied()).collect();
        let (code, envelope) = vtest(&root, &args);
        assert_eq!(code, 2, "{args:?}: {envelope}");
        assert_eq!(code_of(&envelope), "E-SCAN-017", "{args:?}");
    }
    assert!(vo_files(&root).is_empty());

    let (code, _) = vtest(
        &root,
        &[
            &base[..],
            &[
                "--policy",
                "explicit",
                "--combination",
                "sign=positive,op=add",
            ],
        ]
        .concat(),
    );
    assert_eq!(code, 0);
    let before = fs::read_to_string(root.join(".verify/vo/VO-X.yaml")).unwrap();
    let (code, envelope) = vtest(
        &root,
        &[
            "vo",
            "edit",
            "VO-X",
            "--combination",
            "sign=positive,op=mul",
        ],
    );
    assert_eq!((code, code_of(&envelope)), (2, "E-SCAN-017"));
    assert_eq!(
        fs::read_to_string(root.join(".verify/vo/VO-X.yaml")).unwrap(),
        before
    );
}

/// @vtest.id TEST-CLI-VO-EDIT-COMBINATIONS-DESIRED-STATE
/// @vtest.covers VO-CLI-VO-COMBINATIONS-DESIRED-STATE
/// @vtest.target crates/vtest-cli/src/ops/vo.rs::edit
/// @vtest.intent vo edit --combination replaces the stored set, omitting it keeps the set, and --clear-combinations empties it
#[test]
fn edit_combinations_is_desired_state() {
    let root = fixture("desired-state");
    let (code, envelope) = vtest(
        &root,
        &[
            "vo",
            "add",
            "--id",
            "VO-X",
            "--claim",
            "c",
            "--derives-from",
            "R-001",
            "--dimension",
            "sign=positive,negative",
            "--policy",
            "explicit",
            "--combination",
            "sign=positive",
        ],
    );
    assert_eq!(code, 0, "{envelope}");
    let combos = |root: &Path| {
        let (_, shown) = vtest(root, &["vo", "show", "VO-X"]);
        shown["data"]["record"]["combinations"].clone()
    };
    let (code, _) = vtest(
        &root,
        &["vo", "edit", "VO-X", "--combination", "sign=negative"],
    );
    assert_eq!(code, 0);
    assert_eq!(
        combos(&root),
        serde_json::json!([{"sign": "negative"}]),
        "replaced, not appended"
    );
    let (code, _) = vtest(&root, &["vo", "edit", "VO-X", "--claim", "c2"]);
    assert_eq!(code, 0);
    assert_eq!(
        combos(&root),
        serde_json::json!([{"sign": "negative"}]),
        "kept"
    );
    let (code, _) = vtest(
        &root,
        &[
            "vo",
            "edit",
            "VO-X",
            "--policy",
            "null",
            "--clear-combinations",
        ],
    );
    assert_eq!(code, 0);
    assert_eq!(combos(&root), serde_json::json!([]), "cleared");
}

/// @vtest.id TEST-CLI-VO-EXPAND-POLICIES
/// @vtest.covers VO-CLI-VO-EXPAND-MATERIALIZATION
/// @vtest.target crates/vtest-cli/src/ops/vo.rs::expand
/// @vtest.intent vo expand yields 6 children for independent-axes, 8 for full-product and one per tuple for explicit, named by upper-cased partitions in dimension order; --dry-run writes nothing
#[test]
fn expand_materializes_each_policy() {
    let root = fixture("expand");
    let dims = [
        "--dimension",
        "operand-sign=positive,negative",
        "--dimension",
        "operator=add,sub,mul,div",
    ];
    for (id, policy) in [("VO-I", "independent-axes"), ("VO-F", "full-product")] {
        let args = [
            &[
                "vo",
                "add",
                "--id",
                id,
                "--claim",
                "c",
                "--derives-from",
                "R-001",
            ][..],
            &dims,
            &["--policy", policy],
        ]
        .concat();
        assert_eq!(vtest(&root, &args).0, 0);
    }
    let args = [
        &[
            "vo",
            "add",
            "--id",
            "VO-E",
            "--claim",
            "c",
            "--derives-from",
            "R-001",
        ][..],
        &dims,
        &[
            "--policy",
            "explicit",
            "--combination",
            "operator=div,operand-sign=positive",
            "--combination",
            "operand-sign=negative,operator=div",
        ],
    ]
    .concat();
    assert_eq!(vtest(&root, &args).0, 0);

    let ids = |envelope: &Value| -> Vec<String> {
        envelope["data"]["children"]
            .as_array()
            .unwrap()
            .iter()
            .map(|child| child["id"].as_str().unwrap().to_owned())
            .collect()
    };
    let before = vo_files(&root);
    let (code, dry) = vtest(&root, &["vo", "expand", "VO-I", "--dry-run"]);
    assert_eq!(code, 0);
    assert_eq!(ids(&dry).len(), 6);
    assert_eq!(vo_files(&root), before, "--dry-run writes nothing");

    let (_, full) = vtest(&root, &["vo", "expand", "VO-F", "--dry-run"]);
    assert_eq!(ids(&full).len(), 8);
    assert!(ids(&full).contains(&"VO-F-NEGATIVE-MUL".to_owned()));

    let (code, explicit) = vtest(&root, &["vo", "expand", "VO-E"]);
    assert_eq!(code, 0, "{explicit}");
    assert_eq!(
        ids(&explicit),
        vec!["VO-E-POSITIVE-DIV", "VO-E-NEGATIVE-DIV"]
    );
    let (_, child) = vtest(&root, &["vo", "show", "VO-E-POSITIVE-DIV"]);
    assert_eq!(child["data"]["record"]["parent"], "VO-E");
    assert_eq!(child["data"]["record"]["derives_from"][0]["doc"], "R-001");
}

/// @vtest.id TEST-CLI-VO-EXPAND-NO-PARTIAL
/// @vtest.covers VO-CLI-VO-EXPAND-NO-PARTIAL
/// @vtest.target crates/vtest-cli/src/ops/vo.rs::expand
/// @vtest.intent vo expand on a VO with invalid combinations is E-SCAN-017 / exit 2, and an existing child id aborts the whole expansion, so no child is ever written partially
#[test]
fn expand_never_generates_partially() {
    let root = fixture("no-partial");
    let layout = vtest_store::VerifyLayout::new(&root);
    // A record with invalid combinations can exist on disk (hand-written);
    // expand must refuse it.
    fs::write(
        layout.vo_dir().join("VO-BAD.yaml"),
        "id: VO-BAD\nparent: null\nderives_from:\n  - doc: R-001\nclaim: c\n\
         dimensions:\n  - name: sign\n    partitions: [positive]\n\
         coverage_policy: explicit\ncombinations: []\nrepresentative_cases: []\n\
         created: 2026-01-01T00:00:00Z\nupdated: 2026-01-01T00:00:00Z\n",
    )
    .unwrap();
    let (code, envelope) = vtest(&root, &["vo", "expand", "VO-BAD"]);
    assert_eq!((code, code_of(&envelope)), (2, "E-SCAN-017"));

    let (code, _) = vtest(
        &root,
        &[
            "vo",
            "add",
            "--id",
            "VO-X",
            "--claim",
            "c",
            "--derives-from",
            "R-001",
            "--dimension",
            "sign=positive,negative",
            "--policy",
            "independent-axes",
        ],
    );
    assert_eq!(code, 0);
    add_basic(&root, "VO-X-NEGATIVE", "R-001");
    let before = vo_files(&root);
    let (code, envelope) = vtest(&root, &["vo", "expand", "VO-X"]);
    assert_eq!((code, code_of(&envelope)), (2, "E-OP-001"));
    assert_eq!(vo_files(&root), before, "VO-X-POSITIVE must not be written");
}

/// Every file under `.verify/` and the project root, with its bytes, so a
/// rejected command can be checked to have changed nothing anywhere.
fn snapshot_tree(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn walk(dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        for entry in fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else {
                out.push((path.clone(), fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, &mut out);
    out.sort();
    out
}

/// PR #147 review: a VO id is its file name (BD-322), so an id with `..`,
/// a separator or an absolute path must not reach `.verify/vo/{id}.yaml`.
/// `vo add --id ../config` used to overwrite `.verify/config.yaml`.
/// @vtest.id TEST-CLI-VO-ID-PATH-TRAVERSAL-REJECTED
/// @vtest.covers VO-STORE-VO-ID-IS-ITS-FILE-NAME
/// @vtest.target crates/vtest-cli/src/ops/vo.rs::add
/// @vtest.intent vo add/edit/expandで..・区切り文字・絶対パスを含むVO IDや子IDを E-OP-001 / exit 2 で拒否し、vo show/approveも失敗して、既存config・VO・承認・外部ファイルの内容が一切変わらないことを確認する
#[test]
fn vo_ids_that_are_not_a_file_name_are_rejected_without_writing() {
    let root = fixture("id-traversal");
    let outside = root.with_extension("outside.yaml");
    fs::write(&outside, "outside: untouched\n").unwrap();
    add_basic(&root, "VO-KEEP", "R-001");
    let (code, _) = vtest(
        &root,
        &[
            "vo",
            "add",
            "--id",
            "VO-AXIS",
            "--claim",
            "c",
            "--derives-from",
            "R-001",
            "--dimension",
            "where=../../escape,in",
            "--policy",
            "independent-axes",
        ],
    );
    assert_eq!(code, 0);
    let outside_name = outside.file_stem().unwrap().to_str().unwrap().to_owned();
    let absolute = outside.with_extension("");
    let before = snapshot_tree(&root);

    let ids: Vec<String> = vec![
        "../config".to_owned(),
        // `.verify/vo/../../../<stem>.yaml` is the file next to the root.
        format!("../../../{outside_name}"),
        absolute.to_str().unwrap().to_owned(),
        "VO-A/VO-B".to_owned(),
        "VO-A\\VO-B".to_owned(),
        ".hidden".to_owned(),
        "..".to_owned(),
    ];
    for id in ids.iter().map(String::as_str) {
        for args in [
            vec![
                "vo",
                "add",
                "--id",
                id,
                "--claim",
                "overwrite",
                "--derives-from",
                "ROOT-001",
            ],
            vec!["vo", "edit", id, "--claim", "overwrite"],
            vec!["vo", "expand", id],
        ] {
            let (code, envelope) = vtest(&root, &args);
            assert_eq!(
                (code, code_of(&envelope)),
                (2, "E-OP-001"),
                "{args:?}: {envelope}"
            );
        }
        // The read-only and approval paths resolve the id against the VOs
        // that exist, so such an id is simply unknown; they fail and write
        // nothing (no approval record either).
        for args in [
            vec!["vo", "show", id],
            vec![
                "vo",
                "approve",
                id,
                "--state",
                "approved",
                "--approver-kind",
                "human",
                "--approver-id",
                "alice",
                "--basis",
                "review",
            ],
        ] {
            let (code, envelope) = vtest(&root, &args);
            assert_ne!(code, 0, "{args:?}: {envelope}");
            assert_eq!(envelope["ok"], false, "{args:?}: {envelope}");
        }
    }
    // A partition value that would put a child outside `.verify/vo/`.
    let (code, envelope) = vtest(&root, &["vo", "expand", "VO-AXIS"]);
    assert_eq!((code, code_of(&envelope)), (2, "E-OP-001"), "{envelope}");

    assert_eq!(
        snapshot_tree(&root),
        before,
        "nothing under the project changed"
    );
    assert_eq!(
        fs::read_to_string(&outside).unwrap(),
        "outside: untouched\n"
    );
    assert!(!root.parent().unwrap().join("escape.yaml").exists());
    fs::remove_file(&outside).ok();
}

/// @vtest.id TEST-CLI-VO-ID-CASE-VARIANT-REJECTED
/// @vtest.covers VO-STORE-VO-ID-IS-ITS-FILE-NAME
/// @vtest.target crates/vtest-cli/src/ops/vo.rs::add
/// @vtest.intent 既存VOと大文字小文字だけが違うIDのvo addと、そうした子IDを生むvo expandをE-OP-001 / exit 2で拒否し、大文字小文字を区別しないファイルシステムでも既存VOのファイルを上書きしないことを確認する
#[test]
fn vo_ids_that_differ_only_in_case_are_rejected_without_writing() {
    let root = fixture("id-case");
    add_basic(&root, "VO-KEEP", "R-001");
    add_basic(&root, "VO-AXIS-IN", "R-001");
    let (code, envelope) = vtest(
        &root,
        &[
            "vo",
            "add",
            "--id",
            "VO-AXIS",
            "--claim",
            "c",
            "--derives-from",
            "R-001",
            "--dimension",
            "where=in,out",
            "--policy",
            "independent-axes",
        ],
    );
    assert_eq!(code, 0, "{envelope}");
    // `VO-AXIS-IN` exists, so its exact id is the existing-child check;
    // rename it to a case variant to reach the case check instead.
    let vo_dir = root.join(".verify/vo");
    let text = fs::read_to_string(vo_dir.join("VO-AXIS-IN.yaml")).unwrap();
    fs::remove_file(vo_dir.join("VO-AXIS-IN.yaml")).unwrap();
    fs::write(
        vo_dir.join("VO-Axis-in.yaml"),
        text.replace("VO-AXIS-IN", "VO-Axis-in"),
    )
    .unwrap();
    let before = snapshot_tree(&root);

    for args in [
        vec![
            "vo",
            "add",
            "--id",
            "vo-keep",
            "--claim",
            "overwrite",
            "--derives-from",
            "ROOT-001",
        ],
        vec!["vo", "expand", "VO-AXIS"],
    ] {
        let (code, envelope) = vtest(&root, &args);
        assert_eq!(
            (code, code_of(&envelope)),
            (2, "E-OP-001"),
            "{args:?}: {envelope}"
        );
    }
    assert_eq!(
        snapshot_tree(&root),
        before,
        "nothing under the project changed"
    );
}

/// @vtest.id TEST-CLI-VO-APPROVE-IS-APPROVAL-CREATE
/// @vtest.covers VO-CLI-VO-APPROVE-ALIAS
/// @vtest.target crates/vtest-cli/src/lib.rs::run
/// @vtest.intent vo approve writes the same approval record shape as approval create --subject-type vo, and rejects an unknown VO with the same code
#[test]
fn vo_approve_is_approval_create() {
    let root = fixture("approve");
    add_basic(&root, "VO-A", "R-001");
    add_basic(&root, "VO-B", "R-001");
    let approval = [
        "--state",
        "approved",
        "--approver-kind",
        "human",
        "--approver-id",
        "alice",
        "--basis",
        "review",
    ];
    let (code, via_vo) = vtest(&root, &[&["vo", "approve", "VO-A"][..], &approval].concat());
    assert_eq!(code, 0, "{via_vo}");
    let (code, via_approval) = vtest(
        &root,
        &[
            &[
                "approval",
                "create",
                "--subject-type",
                "vo",
                "--subject-id",
                "VO-B",
            ][..],
            &approval,
        ]
        .concat(),
    );
    assert_eq!(code, 0);
    let shape = |envelope: &Value| {
        let mut data = envelope["data"].clone();
        let object = data.as_object_mut().unwrap();
        for volatile in ["id", "approved_at", "subject"] {
            object.remove(volatile);
        }
        data
    };
    assert_eq!(shape(&via_vo), shape(&via_approval));
    assert_eq!(via_vo["data"]["subject"], "VO-A");

    let (code_vo, unknown_vo) = vtest(
        &root,
        &[&["vo", "approve", "VO-NONE"][..], &approval].concat(),
    );
    let (code_ap, unknown_ap) = vtest(
        &root,
        &[
            &[
                "approval",
                "create",
                "--subject-type",
                "vo",
                "--subject-id",
                "VO-NONE",
            ][..],
            &approval,
        ]
        .concat(),
    );
    assert_eq!(
        (code_vo, code_of(&unknown_vo)),
        (code_ap, code_of(&unknown_ap))
    );
    assert_eq!(code_of(&unknown_vo), "E-APPROVAL-001");
}

/// @vtest.id TEST-CLI-VO-EDIT-APPROVED-WARNS
/// @vtest.covers VO-CLI-VO-EDIT-APPROVED-WARNS
/// @vtest.target crates/vtest-cli/src/ops/vo.rs::edit
/// @vtest.intent editing an approved VO succeeds with a warning; a claim edit lapses the approval to draft while an anchor-only edit keeps it approved
#[test]
fn editing_an_approved_vo_warns_and_lapses_by_hash() {
    let root = fixture("edit-approved");
    add_basic(&root, "VO-A", "R-001");
    let approve = |root: &Path| {
        let (code, envelope) = vtest(
            root,
            &[
                "vo",
                "approve",
                "VO-A",
                "--state",
                "approved",
                "--approver-kind",
                "human",
                "--approver-id",
                "alice",
            ],
        );
        assert_eq!(code, 0, "{envelope}");
    };
    let status = |root: &Path| vtest(root, &["vo", "show", "VO-A"]).1["data"]["status"].clone();
    approve(&root);
    assert_eq!(status(&root), "approved");

    let (code, anchor_only) = vtest(
        &root,
        &[
            "vo",
            "edit",
            "VO-A",
            "--derives-from",
            "R-001",
            "--anchor",
            "§2",
        ],
    );
    assert_eq!(code, 0);
    assert_eq!(anchor_only["data"]["was_approved"], true);
    assert_eq!(anchor_only["data"]["approval_invalidated"], false);
    assert_eq!(anchor_only["data"]["warnings"].as_array().unwrap().len(), 1);
    assert_eq!(
        status(&root),
        "approved",
        "DS-1031: anchor is outside the subject hash"
    );

    let (code, claim_edit) = vtest(&root, &["vo", "edit", "VO-A", "--claim", "changed"]);
    assert_eq!(code, 0);
    assert_eq!(claim_edit["data"]["approval_invalidated"], true);
    assert_eq!(status(&root), "draft");
}

/// @vtest.id TEST-CLI-VO-LIST-STATUS
/// @vtest.covers VO-CLI-VO-STATUS-DERIVED
/// @vtest.target crates/vtest-cli/src/ops/vo.rs::list
/// @vtest.intent vo list --status filters by the effective approval state derived from approval records, so a rejected record returns a VO to draft
#[test]
fn list_status_uses_the_effective_approval_state() {
    let root = fixture("status");
    add_basic(&root, "VO-A", "R-001");
    add_basic(&root, "VO-B", "R-001");
    let approve = |state: &str| {
        let (code, envelope) = vtest(
            &root,
            &[
                "vo",
                "approve",
                "VO-A",
                "--state",
                state,
                "--approver-kind",
                "human",
                "--approver-id",
                "alice",
            ],
        );
        assert_eq!(code, 0, "{envelope}");
    };
    let listed = |status: &str| -> Vec<String> {
        let (_, envelope) = vtest(&root, &["vo", "list", "--status", status]);
        envelope["data"]["vos"]
            .as_array()
            .unwrap()
            .iter()
            .map(|vo| vo["id"].as_str().unwrap().to_owned())
            .collect()
    };
    approve("approved");
    assert_eq!(listed("approved"), vec!["VO-A"]);
    assert_eq!(listed("draft"), vec!["VO-B"]);
    approve("rejected");
    assert!(
        listed("approved").is_empty(),
        "a rejected record in the effective set means draft"
    );
    let (code, envelope) = vtest(&root, &["vo", "list", "--status", "pending"]);
    assert_eq!((code, code_of(&envelope)), (2, "E-OP-001"));
}

/// @vtest.id TEST-CLI-VO-LIST-DOC-FILTER
/// @vtest.covers VO-CLI-VO-LIST-DOC-FILTER
/// @vtest.target crates/vtest-cli/src/ops/vo.rs::list
/// @vtest.intent vo list --doc keeps only the VOs downstream of that document node, including child VOs by parent
#[test]
fn list_doc_keeps_downstream_vos() {
    let root = fixture("doc-filter");
    add_basic(&root, "VO-ONE", "R-001");
    add_basic(&root, "VO-TWO", "R-002");
    let (code, _) = vtest(
        &root,
        &[
            "vo",
            "add",
            "--id",
            "VO-ONE-CHILD",
            "--claim",
            "c",
            "--derives-from",
            "R-002",
            "--parent",
            "VO-ONE",
        ],
    );
    assert_eq!(code, 0);
    let ids = |doc: &str| -> Vec<String> {
        let (code, envelope) = vtest(&root, &["vo", "list", "--doc", doc]);
        assert_eq!(code, 0, "{envelope}");
        envelope["data"]["vos"]
            .as_array()
            .unwrap()
            .iter()
            .map(|vo| vo["id"].as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(ids("ROOT-001"), vec!["VO-ONE", "VO-ONE-CHILD"]);
    assert_eq!(ids("R-002"), vec!["VO-ONE-CHILD", "VO-TWO"]);
    let (code, envelope) = vtest(&root, &["vo", "list", "--doc", "NOPE"]);
    assert_eq!((code, code_of(&envelope)), (2, "E-OP-001"));
}
