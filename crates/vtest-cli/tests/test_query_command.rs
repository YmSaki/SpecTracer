//! `vtest test list`, `vtest test query` and `vtest test show` through the
//! `vtest` binary (#70): what each returns, the rejected arguments, the
//! text output, `--quiet`, and that none of them writes anything. The
//! comparison with the MCP tools `test_query` and `test_get` is in
//! `mcp_cli_equivalence.rs`.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

use serde_json::{json, Value};
use vtest_model::{
    DerivesFrom, DocumentFile, DocumentId, NodeSource, RootNode, SentenceNode, VoId, VoRecord,
};
use vtest_store::{write_document_file, write_vo_record, VerifyLayout};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// `add` declares a permanent SRC ID, `sub` and `unused` do not.
const LIB_RS: &str = "/// @vtest.src-id SRC-ADD\n\
pub fn add(a: i32, b: i32) -> i32 { a + b }\n\
pub fn sub(a: i32, b: i32) -> i32 { a - b }\n\
pub fn unused() {}\n";

/// Three managed Tests and one test function without a declaration.
/// `TEST-ADD` names `add` by Locator and `TEST-ADD-BY-SRC` by SRC ID.
const TESTS_RS: &str = "/// @vtest.id TEST-ADD\n\
/// @vtest.covers VO-X\n\
/// @vtest.target src/lib.rs::add\n\
/// @vtest.intent adds two numbers\n\
#[test]\n\
fn adds() {\n    assert_eq!(query_fixture::add(2, 3), 5);\n}\n\n\
/// @vtest.id TEST-SUB\n\
/// @vtest.covers VO-X, VO-Y\n\
/// @vtest.target src/lib.rs::sub\n\
/// @vtest.intent subtracts two numbers\n\
#[test]\n\
fn subtracts() {\n    assert_eq!(query_fixture::sub(5, 3), 2);\n}\n\n\
/// @vtest.id TEST-ADD-BY-SRC\n\
/// @vtest.covers VO-Y\n\
/// @vtest.target SRC-ADD\n\
/// @vtest.intent adds through the SRC ID\n\
#[test]\n\
fn adds_again() {\n    assert_eq!(query_fixture::add(1, 1), 2);\n}\n\n\
#[test]\n\
fn bare() {\n    assert_eq!(query_fixture::sub(1, 1), 0);\n}\n";

/// A Test whose `covers` names a VO without a record (E-SCAN-003).
const GONE_RS: &str = "/// @vtest.id TEST-GONE\n\
/// @vtest.covers VO-GONE\n\
/// @vtest.target src/lib.rs::sub\n\
/// @vtest.intent covers a VO without a record\n\
#[test]\n\
fn gone() {\n    assert_eq!(query_fixture::sub(2, 1), 1);\n}\n";

fn project(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "vtest-test-query-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("tests")).unwrap();
    vtest_store::init_project(&root, "test-query").unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"query-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), LIB_RS).unwrap();
    fs::write(root.join("tests/query.rs"), TESTS_RS).unwrap();

    let layout = VerifyLayout::new(&root);
    let source = |id: &str| NodeSource {
        doc: format!("{id}.md"),
        heading: "query".to_owned(),
        lines: [1, 1],
    };
    let document = DocumentFile {
        schema_version: "0.1".to_owned(),
        root: vec![RootNode {
            id: DocumentId::new("ROOT-001"),
            statement: "query root".to_owned(),
            description: None,
            source: source("root"),
        }],
        request: vec![SentenceNode {
            id: DocumentId::new("R-001"),
            statement: "arithmetic works".to_owned(),
            description: None,
            derives_from: vec![DocumentId::new("ROOT-001")],
            cites: None,
            source: source("request"),
        }],
        require: Vec::new(),
        spec: Vec::new(),
        detailed_spec: Vec::new(),
        basic_design: Vec::new(),
        design: Vec::new(),
    };
    write_document_file(&layout, "query", &document).unwrap();
    for id in ["VO-X", "VO-Y"] {
        write_vo_record(
            &layout,
            &VoRecord {
                id: VoId::new(id),
                parent: None,
                derives_from: vec![DerivesFrom {
                    doc: DocumentId::new("R-001"),
                    anchor: None,
                    note: None,
                }],
                claim: format!("claim of {id}"),
                dimensions: Vec::new(),
                coverage_policy: None,
                combinations: Vec::new(),
                representative_cases: Vec::new(),
                created: "2026-11-04T00:00:00Z".to_owned(),
                updated: "2026-11-04T00:00:00Z".to_owned(),
            },
        )
        .unwrap();
    }
    root
}

/// `vtest --project <root> <args>`.
fn vtest(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vtest"))
        .arg("--project")
        .arg(root)
        .args(args)
        .output()
        .unwrap()
}

fn exit(output: &Output) -> i32 {
    output.status.code().expect("exit code")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn envelope(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("non-JSON stdout ({error}): {}", stdout(output)))
}

/// `vtest --project <root> --format json <args>`: exit code and envelope.
fn json_of(root: &Path, args: &[&str]) -> (i32, Value) {
    let mut all = vec!["--format", "json"];
    all.extend_from_slice(args);
    let output = vtest(root, &all);
    (exit(&output), envelope(&output))
}

fn keys(value: &Value) -> Vec<String> {
    value
        .as_object()
        .unwrap_or_else(|| panic!("not an object: {value}"))
        .keys()
        .cloned()
        .collect()
}

fn ids(envelope: &Value) -> Vec<String> {
    envelope["data"]["tests"]
        .as_array()
        .unwrap_or_else(|| panic!("no data.tests: {envelope}"))
        .iter()
        .map(|test| test["id"].as_str().unwrap().to_owned())
        .collect()
}

fn codes(envelope: &Value) -> Vec<String> {
    envelope["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|diagnostic| diagnostic["code"].as_str().unwrap().to_owned())
        .collect()
}

/// Every file below `root` with its bytes, and every directory.
fn tree(root: &Path) -> Vec<(String, Option<Vec<u8>>)> {
    fn visit(root: &Path, directory: &Path, entries: &mut Vec<(String, Option<Vec<u8>>)>) {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            if path.is_dir() {
                entries.push((relative, None));
                visit(root, &path, entries);
            } else {
                entries.push((relative, Some(fs::read(&path).unwrap())));
            }
        }
    }
    let mut entries = Vec::new();
    visit(root, root, &mut entries);
    entries.sort();
    entries
}

/// The stdout lines that are not diagnostics (`[code] message`).
fn result_lines(output: &Output) -> Vec<String> {
    stdout(output)
        .lines()
        .filter(|line| !line.starts_with('['))
        .map(str::to_owned)
        .collect()
}

/// @vtest.id TEST-CLI-TEST-LIST-RETURNS-MANAGED-TESTS-IN-SPEC-ORDER
/// @vtest.covers VO-CLI-TEST-QUERY-TEST-JSON, VO-CLI-TEST-LIST-ALL-MANAGED-TESTS, VO-CLI-TEST-LIST-QUERY-EXIT-FROM-SCAN-ERRORS
/// @vtest.target crates/vtest-ops/src/test_query.rs::list
/// @vtest.intent `vtest test list` without a filter returns `data` with `tests` only, holding every managed Test as a Test JSON with `execution` and a `targets` list in the order of `vtest export`, with exit code 0, `ok: true` and every scan diagnostic when the scan has no error
#[test]
fn list_returns_every_managed_test_in_the_order_of_the_test_specification() {
    let root = project("list-all");
    let (code, listed) = json_of(&root, &["test", "list"]);
    assert_eq!(code, 0, "{listed}");
    assert_eq!(listed["ok"], true);
    assert_eq!(keys(&listed["data"]), ["tests"]);
    assert_eq!(codes(&listed), ["W-SCAN-101"]);

    let (_, exported) = json_of(&root, &["export"]);
    let exported_ids: Vec<String> = exported["data"]["tests"]
        .as_array()
        .unwrap()
        .iter()
        .map(|test| test["id"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(exported_ids.len(), 3);
    assert_eq!(ids(&listed), exported_ids);
    assert_eq!(listed["diagnostics"], exported["diagnostics"]);
    for test in listed["data"]["tests"].as_array().unwrap() {
        assert!(test["execution"].is_object(), "{test}");
        assert!(test["targets"].is_array(), "{test}");
        // The test specification's Test item has `adapter` and no
        // `execution` (DS-1707); this is the Test JSON.
        assert!(test.get("adapter").is_none(), "{test}");
    }
}

/// @vtest.id TEST-CLI-TEST-LIST-VO-FILTERS-BY-DECLARED-COVERS
/// @vtest.covers VO-CLI-TEST-LIST-VO-FILTER, VO-CLI-TEST-LIST-VO-WITHOUT-RECORD, VO-CLI-TEST-LIST-QUERY-EXIT-FROM-SCAN-ERRORS
/// @vtest.target crates/vtest-ops/src/test_query.rs::list
/// @vtest.intent `vtest test list --vo` returns only the managed Tests that declare the VO ID in `covers`, in the unfiltered order; a VO ID nothing declares gives an empty list with exit code 0; a VO ID without a VO record is not rejected and returns the Test that declares it, with exit code 1, `ok: false` and E-SCAN-003 in `diagnostics`
/// @vtest.case VO with a record
/// @vtest.case VO ID nothing declares
/// @vtest.case VO ID without a VO record
#[test]
fn list_vo_returns_the_tests_that_declare_the_vo_even_without_a_vo_record() {
    let root = project("list-vo");
    let (_, all) = json_of(&root, &["test", "list"]);
    let all = ids(&all);

    for (vo, expected) in [
        ("VO-X", vec!["TEST-ADD", "TEST-SUB"]),
        ("VO-Y", vec!["TEST-ADD-BY-SRC", "TEST-SUB"]),
    ] {
        let (code, listed) = json_of(&root, &["test", "list", "--vo", vo]);
        assert_eq!(code, 0, "{vo}: {listed}");
        assert_eq!(keys(&listed["data"]), ["tests"], "{vo}");
        let in_order: Vec<&String> = all
            .iter()
            .filter(|id| expected.contains(&id.as_str()))
            .collect();
        assert_eq!(ids(&listed).iter().collect::<Vec<_>>(), in_order, "{vo}");
    }

    let (code, none) = json_of(&root, &["test", "list", "--vo", "VO-NOBODY"]);
    assert_eq!((code, &none["ok"]), (0, &json!(true)), "{none}");
    assert_eq!(none["data"], json!({"tests": []}));

    fs::write(root.join("tests/gone.rs"), GONE_RS).unwrap();
    let (code, gone) = json_of(&root, &["test", "list", "--vo", "VO-GONE"]);
    assert_eq!(code, 1, "{gone}");
    assert_eq!(gone["ok"], false);
    assert_eq!(ids(&gone), ["TEST-GONE"]);
    assert!(codes(&gone).contains(&"E-SCAN-003".to_owned()), "{gone}");
    // DS-1815: every scan diagnostic, not only the ones of the listed Tests.
    assert!(codes(&gone).contains(&"W-SCAN-101".to_owned()), "{gone}");
}

/// @vtest.id TEST-CLI-TEST-LIST-UNREGISTERED-MATCHES-EXPORT-UNMANAGED
/// @vtest.covers VO-CLI-TEST-LIST-UNREGISTERED
/// @vtest.target crates/vtest-ops/src/test_query.rs::list
/// @vtest.intent `vtest test list --unregistered` returns `data` with `unmanaged` only, equal to `data.unmanaged` of `vtest export` for the same project
#[test]
fn list_unregistered_returns_the_unmanaged_list_of_the_test_specification() {
    let root = project("list-unregistered");
    let (code, listed) = json_of(&root, &["test", "list", "--unregistered"]);
    assert_eq!(code, 0, "{listed}");
    assert_eq!(keys(&listed["data"]), ["unmanaged"]);
    let unmanaged = listed["data"]["unmanaged"].as_array().unwrap();
    assert_eq!(unmanaged.len(), 1, "{listed}");
    assert_eq!(unmanaged[0]["location"]["locator"], "bare");

    let (_, exported) = json_of(&root, &["export"]);
    assert_eq!(listed["data"]["unmanaged"], exported["data"]["unmanaged"]);
}

/// @vtest.id TEST-CLI-TEST-LIST-REJECTS-OTHER-ARGUMENTS
/// @vtest.covers VO-CLI-TEST-LIST-ARGUMENTS
/// @vtest.target crates/vtest-cli/src/lib.rs::TestCommand
/// @vtest.intent `vtest test list` accepts `--vo` and `--unregistered` beside the global options and rejects a positional argument, another option, `--vo` given twice and `--vo` together with `--unregistered` with exit code 2 and nothing on stdout
/// @vtest.case positional argument
/// @vtest.case unknown option
/// @vtest.case --vo twice
/// @vtest.case --vo with --unregistered
#[test]
fn list_rejects_arguments_outside_its_options() {
    let root = project("list-arguments");
    for args in [
        vec!["test", "list", "TEST-ADD"],
        vec!["test", "list", "--source", "SRC-ADD"],
        vec!["test", "list", "--vo", "VO-X", "--vo", "VO-Y"],
        vec!["test", "list", "--vo", "VO-X", "--unregistered"],
    ] {
        let output = vtest(&root, &args);
        assert_eq!(exit(&output), 2, "{args:?}");
        assert_eq!(stdout(&output), "", "{args:?}");
    }
    for args in [
        vec!["test", "list", "--vo", "VO-X"],
        vec!["test", "list", "--unregistered"],
        vec!["--format", "text", "test", "list"],
    ] {
        assert_eq!(exit(&vtest(&root, &args)), 0, "{args:?}");
    }
}

/// @vtest.id TEST-CLI-TEST-QUERY-COMPARES-RESOLVED-SOURCE-TARGETS
/// @vtest.covers VO-CLI-TEST-QUERY-SOURCE-REFERENCE-FORMS, VO-CLI-TEST-QUERY-RETURNS-TESTS-OF-SOURCE-TARGET, VO-OPS-TEST-QUERY-COMPARES-CANONICAL-SOURCE-TARGETS
/// @vtest.target crates/vtest-ops/src/test_query.rs::query
/// @vtest.intent `vtest test query --source` reads `SRC-…` as a SRC ID and anything else as `<adapter ID>::<locator>`, and returns the same `data.tests` for both spellings of one Source Target: the Tests that declare it by Locator or by SRC ID, in the unfiltered order, without the Tests of another Source Target, and an empty list for a Source Target no Test declares
/// @vtest.case Locator reference
/// @vtest.case SRC ID reference
/// @vtest.case Source Target no Test declares
#[test]
fn query_returns_the_tests_whose_targets_resolve_to_the_same_source_target() {
    let root = project("query");
    let (_, all) = json_of(&root, &["test", "list"]);
    let expected: Vec<String> = ids(&all)
        .into_iter()
        .filter(|id| id == "TEST-ADD" || id == "TEST-ADD-BY-SRC")
        .collect();
    assert_eq!(expected.len(), 2);

    let (code, by_locator) = json_of(
        &root,
        &["test", "query", "--source", "rust-cargo::src/lib.rs::add"],
    );
    assert_eq!(code, 0, "{by_locator}");
    assert_eq!(by_locator["ok"], true);
    assert_eq!(keys(&by_locator["data"]), ["tests"]);
    assert_eq!(ids(&by_locator), expected);

    let (code, by_src_id) = json_of(&root, &["test", "query", "--source", "SRC-ADD"]);
    assert_eq!(code, 0, "{by_src_id}");
    assert_eq!(by_src_id["data"], by_locator["data"]);

    let (code, unused) = json_of(
        &root,
        &[
            "test",
            "query",
            "--source",
            "rust-cargo::src/lib.rs::unused",
        ],
    );
    assert_eq!(code, 0, "{unused}");
    assert_eq!(unused["data"], json!({"tests": []}));
}

/// @vtest.id TEST-CLI-TEST-QUERY-REJECTS-UNRESOLVED-SOURCE
/// @vtest.covers VO-CLI-TEST-QUERY-SOURCE-REFERENCE-FORMS, VO-CLI-TEST-QUERY-UNRESOLVED-SOURCE-REJECTED, VO-OPS-TEST-QUERY-INPUT-ERROR-RETURNS-NO-RESULT, VO-MCP-INPUT-ERROR-CANDIDATES
/// @vtest.target crates/vtest-ops/src/test_query.rs::query
/// @vtest.target crates/vtest-scan/src/operations.rs::source_target_candidates
/// @vtest.intent `vtest test query --source` rejects a value that neither starts with `SRC-` nor contains `::`, an adapter ID that is not registered, a reference that resolves to no Source Target and a SRC ID two Source Targets declare with E-OP-001, exit code 2, `ok: false` and `data: null`, names the canonical Locators of the near Source Targets of the same adapter and of the Source Targets that declare a near SRC ID in `candidates`, each once, and has no `candidates` when none is near
/// @vtest.case no `::` and no `SRC-`
/// @vtest.case unregistered adapter ID
/// @vtest.case no such Source Target
/// @vtest.case unknown SRC ID
/// @vtest.case near Locator
/// @vtest.case same last segment in another file
/// @vtest.case near SRC ID
/// @vtest.case SRC ID declared twice
#[test]
fn query_rejects_a_source_that_does_not_resolve_to_one_source_target() {
    let root = project("query-rejected");
    for source in [
        "add",
        "no-such-adapter::src/lib.rs::add",
        "rust-cargo::src/lib.rs::missing",
        "SRC-MISSING",
    ] {
        let (code, rejected) = json_of(&root, &["test", "query", "--source", source]);
        assert_eq!(code, 2, "{source}: {rejected}");
        assert_eq!(rejected["ok"], false, "{source}");
        assert_eq!(rejected["data"], Value::Null, "{source}");
        assert_eq!(codes(&rejected), ["E-OP-001"], "{source}");
        assert!(
            rejected["diagnostics"][0].get("candidates").is_none(),
            "{source}: {rejected}"
        );
    }

    // DS-1823 / DS-1192: the near Source Targets are the candidates, as
    // canonical Locators: the same last segment first (DES-404), then the
    // last segments within edit distance 2 (DES-405), each once.
    fs::create_dir_all(root.join("src/deep")).unwrap();
    fs::write(root.join("src/deep/mod.rs"), "pub fn add() {}\n").unwrap();
    fs::write(root.join("src/lib.rs"), format!("pub mod deep;\n{LIB_RS}")).unwrap();
    for (source, expected) in [
        (
            "rust-cargo::src/lib.rs::ad",
            json!([
                "rust-cargo::src/deep/mod.rs::add",
                "rust-cargo::src/lib.rs::add",
                "rust-cargo::tests/query.rs::adds"
            ]),
        ),
        (
            "rust-cargo::src/nowhere.rs::adds",
            json!([
                "rust-cargo::tests/query.rs::adds",
                "rust-cargo::src/deep/mod.rs::add",
                "rust-cargo::src/lib.rs::add"
            ]),
        ),
        ("SRC-AD", json!(["rust-cargo::src/lib.rs::add"])),
    ] {
        for _ in 0..2 {
            let (code, near) = json_of(&root, &["test", "query", "--source", source]);
            assert_eq!(code, 2, "{source}: {near}");
            assert_eq!(near["data"], Value::Null, "{source}");
            assert_eq!(codes(&near), ["E-OP-001"], "{source}");
            assert_eq!(near["diagnostics"][0]["candidates"], expected, "{source}");
        }
    }

    fs::write(
        root.join("src/lib.rs"),
        format!("{LIB_RS}/// @vtest.src-id SRC-ADD\npub fn add_again() {{}}\n"),
    )
    .unwrap();
    let (code, collision) = json_of(&root, &["test", "query", "--source", "SRC-ADD"]);
    assert_eq!(code, 2, "{collision}");
    assert_eq!(collision["data"], Value::Null);
    assert_eq!(codes(&collision), ["E-OP-001"]);
    assert_eq!(
        collision["diagnostics"][0]["candidates"],
        json!([
            "rust-cargo::src/lib.rs::add",
            "rust-cargo::src/lib.rs::add_again"
        ])
    );
}

/// @vtest.id TEST-CLI-TEST-QUERY-REQUIRES-SOURCE-ONCE
/// @vtest.covers VO-CLI-TEST-QUERY-ARGUMENTS
/// @vtest.target crates/vtest-cli/src/lib.rs::TestCommand
/// @vtest.intent `vtest test query` needs `--source` exactly once and rejects a missing `--source`, `--source` given twice, a positional argument and another option with exit code 2 and nothing on stdout
/// @vtest.case --source missing
/// @vtest.case --source twice
/// @vtest.case positional argument
/// @vtest.case unknown option
#[test]
fn query_requires_source_exactly_once() {
    let root = project("query-arguments");
    for args in [
        vec!["test", "query"],
        vec![
            "test", "query", "--source", "SRC-ADD", "--source", "SRC-ADD",
        ],
        vec!["test", "query", "--source", "SRC-ADD", "TEST-ADD"],
        vec!["test", "query", "--source", "SRC-ADD", "--vo", "VO-X"],
    ] {
        let output = vtest(&root, &args);
        assert_eq!(exit(&output), 2, "{args:?}");
        assert_eq!(stdout(&output), "", "{args:?}");
    }
    assert_eq!(
        exit(&vtest(&root, &["test", "query", "--source", "SRC-ADD"])),
        0
    );
}

/// @vtest.id TEST-CLI-TEST-LIST-QUERY-KEEP-RESULT-ON-SCAN-ERROR
/// @vtest.covers VO-CLI-TEST-LIST-QUERY-EXIT-FROM-SCAN-ERRORS
/// @vtest.target crates/vtest-ops/src/test_query.rs::query
/// @vtest.intent with a Test whose `covers` cannot be resolved, `vtest test list` and `vtest test query` for that Test's target still return the Tests in `data.tests`, with exit code 1, `ok: false` and E-SCAN-003 in `diagnostics`
#[test]
fn list_and_query_return_the_tests_with_exit_code_1_on_a_scan_error() {
    let root = project("scan-error");
    fs::write(root.join("tests/gone.rs"), GONE_RS).unwrap();
    for args in [
        vec!["test", "list"],
        vec!["test", "query", "--source", "rust-cargo::src/lib.rs::sub"],
    ] {
        let (code, result) = json_of(&root, &args);
        assert_eq!(code, 1, "{args:?}: {result}");
        assert_eq!(result["ok"], false, "{args:?}");
        assert!(ids(&result).contains(&"TEST-GONE".to_owned()), "{args:?}");
        assert!(
            codes(&result).contains(&"E-SCAN-003".to_owned()),
            "{args:?}"
        );
    }
}

/// @vtest.id TEST-CLI-TEST-SHOW-CARRIES-THE-VERIFY-RESULT
/// @vtest.covers VO-CLI-TEST-QUERY-TEST-JSON, VO-CLI-TEST-SHOW-DATA-FOUR-KEYS, VO-CLI-TEST-SHOW-EXIT-OK-DIAGNOSTICS-FROM-VERIFY, VO-OPS-TEST-SHOW-VERIFICATION-UNCONVERTED, VO-CLI-TEST-SHOW-NO-JUDGMENT-RECORDS, VO-CLI-TEST-SHOW-EVIDENCE-LATEST-RECORD
/// @vtest.target crates/vtest-ops/src/test_query.rs::show
/// @vtest.intent `vtest test show <Test ID>` returns `data` with `test`, `verification`, `judgments` and `evidence` only: `test` is the Test JSON of that Test, `verification`, `ok`, `diagnostics` and the exit code equal those of `vtest verify --test <Test ID>` for the same project, `judgments` is an empty list without a judgment record and `evidence` is null without an Evidence record
#[test]
fn show_returns_the_test_with_the_result_of_verify_for_that_test() {
    let root = project("show");
    let (code, shown) = json_of(&root, &["test", "show", "TEST-ADD"]);
    assert_eq!(
        keys(&shown["data"]),
        ["evidence", "judgments", "test", "verification"],
        "{shown}"
    );
    assert_eq!(shown["data"]["test"]["id"], "TEST-ADD");
    assert!(shown["data"]["test"]["execution"].is_object());
    assert_eq!(
        shown["data"]["test"]["targets"].as_array().map(Vec::len),
        Some(1)
    );
    let (_, listed) = json_of(&root, &["test", "list", "--vo", "VO-X"]);
    assert_eq!(shown["data"]["test"], listed["data"]["tests"][0]);

    let (verify_code, verified) = json_of(&root, &["verify", "--test", "TEST-ADD"]);
    assert_eq!(code, verify_code);
    assert_eq!(code, 1, "no Evidence yet: {shown}");
    assert_eq!(shown["ok"], verified["ok"]);
    assert_eq!(shown["diagnostics"], verified["diagnostics"]);
    assert_eq!(shown["data"]["verification"], verified["data"]);

    assert_eq!(shown["data"]["judgments"], json!([]));
    assert_eq!(shown["data"]["evidence"], Value::Null);
}

/// @vtest.id TEST-CLI-TEST-SHOW-REJECTS-UNKNOWN-AND-DUPLICATED-ID
/// @vtest.covers VO-CLI-TEST-SHOW-UNKNOWN-OR-DUPLICATED-ID, VO-OPS-TEST-QUERY-INPUT-ERROR-RETURNS-NO-RESULT, VO-MCP-INPUT-ERROR-CANDIDATES
/// @vtest.target crates/vtest-ops/src/test_query.rs::show
/// @vtest.intent `vtest test show` rejects a Test ID no managed Test has with E-OP-001 and a Test ID two managed Tests have with E-SCAN-002, both with exit code 2, `ok: false` and `data: null`, names the Test IDs within edit distance 2 of an unknown one in `candidates`, each once and in ascending order, and has no `candidates` when none is near, while `vtest test list` still returns both Tests of the duplicated ID
/// @vtest.case unknown Test ID
/// @vtest.case unknown Test ID near existing ones
/// @vtest.case Test ID of two managed Tests
#[test]
fn show_rejects_a_test_id_that_does_not_name_exactly_one_test() {
    let root = project("show-rejected");
    let (code, unknown) = json_of(&root, &["test", "show", "TEST-NOBODY"]);
    assert_eq!(code, 2, "{unknown}");
    assert_eq!(unknown["ok"], false);
    assert_eq!(unknown["data"], Value::Null);
    assert_eq!(codes(&unknown), ["E-OP-001"]);
    assert!(
        unknown["diagnostics"][0].get("candidates").is_none(),
        "no Test ID is near: {unknown}"
    );

    // DS-1192: a near Test ID is named, each once, in ascending order.
    for (requested, expected) in [
        ("TEST-AD", json!(["TEST-ADD"])),
        ("TEST-SUD", json!(["TEST-ADD", "TEST-SUB"])),
    ] {
        let (code, near) = json_of(&root, &["test", "show", requested]);
        assert_eq!(code, 2, "{near}");
        assert_eq!(near["data"], Value::Null);
        assert_eq!(codes(&near), ["E-OP-001"]);
        assert_eq!(near["diagnostics"][0]["candidates"], expected, "{near}");
    }

    fs::write(
        root.join("tests/again.rs"),
        "/// @vtest.id TEST-ADD\n/// @vtest.covers VO-X\n\
         /// @vtest.target src/lib.rs::add\n/// @vtest.intent adds again\n\
         #[test]\nfn adds_twice() {\n    assert_eq!(query_fixture::add(0, 0), 0);\n}\n",
    )
    .unwrap();
    let (code, duplicated) = json_of(&root, &["test", "show", "TEST-ADD"]);
    assert_eq!(code, 2, "{duplicated}");
    assert_eq!(duplicated["ok"], false);
    assert_eq!(duplicated["data"], Value::Null);
    assert_eq!(codes(&duplicated), ["E-SCAN-002"]);
    // The duplicated ID is one candidate, not two.
    let (_, near) = json_of(&root, &["test", "show", "TEST-AD"]);
    assert_eq!(near["diagnostics"][0]["candidates"], json!(["TEST-ADD"]));

    // DS-1817: the unfiltered list holds the Tests whose IDs collide.
    let (code, listed) = json_of(&root, &["test", "list"]);
    assert_eq!(code, 1, "{listed}");
    assert_eq!(
        ids(&listed).iter().filter(|id| *id == "TEST-ADD").count(),
        2,
        "{listed}"
    );
}

/// @vtest.id TEST-CLI-TEST-SHOW-TAKES-ONE-TEST-ID
/// @vtest.covers VO-CLI-TEST-SHOW-ARGUMENTS
/// @vtest.target crates/vtest-cli/src/lib.rs::TestCommand
/// @vtest.intent `vtest test show` needs exactly one Test ID and has no option beside the global ones: no Test ID, two Test IDs and another option are rejected with exit code 2 and nothing on stdout, while `--format json`, `--format text` and `--project` are not rejected as arguments
/// @vtest.case no Test ID
/// @vtest.case two Test IDs
/// @vtest.case unknown option
#[test]
fn show_takes_exactly_one_test_id_and_no_option_of_its_own() {
    let root = project("show-arguments");
    for args in [
        vec!["test", "show"],
        vec!["test", "show", "TEST-ADD", "TEST-SUB"],
        vec!["test", "show", "TEST-ADD", "--vo", "VO-X"],
    ] {
        let output = vtest(&root, &args);
        assert_eq!(exit(&output), 2, "{args:?}");
        assert_eq!(stdout(&output), "", "{args:?}");
    }
    for format in ["json", "text"] {
        let output = vtest(&root, &["test", "show", "TEST-ADD", "--format", format]);
        assert_eq!(exit(&output), 1, "{format}");
        assert!(!stdout(&output).is_empty(), "{format}");
    }
}

/// @vtest.id TEST-CLI-TEST-LIST-QUERY-TEXT-ONE-LINE-PER-TEST
/// @vtest.covers VO-CLI-TEST-LIST-QUERY-TEXT-OUTPUT
/// @vtest.target crates/vtest-cli/src/lib.rs::render_test_list
/// @vtest.intent `--format text` of `vtest test list` and `vtest test query` prints one line per element of the JSON `data.tests`, in the same order, containing that Test's ID, `vtest test list --unregistered` prints one line per element of `data.unmanaged` containing its path and locator, and the exit codes equal those of `--format json`
/// @vtest.case list
/// @vtest.case list --vo
/// @vtest.case query
/// @vtest.case list --unregistered
#[test]
fn list_and_query_text_prints_one_line_per_test_in_the_order_of_the_json() {
    let root = project("text");
    for args in [
        vec!["test", "list"],
        vec!["test", "list", "--vo", "VO-Y"],
        vec!["test", "query", "--source", "SRC-ADD"],
    ] {
        let (code, json) = json_of(&root, &args);
        let expected = ids(&json);
        assert!(expected.len() >= 2, "{args:?}");
        let text = vtest(&root, &args);
        assert_eq!(exit(&text), code, "{args:?}");
        let lines = result_lines(&text);
        assert_eq!(lines.len(), expected.len(), "{args:?}: {lines:?}");
        for (line, id) in lines.iter().zip(&expected) {
            assert_eq!(
                line.split_whitespace().next(),
                Some(id.as_str()),
                "{args:?}"
            );
        }
    }

    let args = ["test", "list", "--unregistered"];
    let (code, json) = json_of(&root, &args);
    let text = vtest(&root, &args);
    assert_eq!(exit(&text), code);
    let lines = result_lines(&text);
    let unmanaged = json["data"]["unmanaged"].as_array().unwrap();
    assert_eq!(lines.len(), unmanaged.len());
    assert_eq!(lines.len(), 1);
    for (line, entry) in lines.iter().zip(unmanaged) {
        assert!(line.contains(entry["location"]["path"].as_str().unwrap()));
        assert!(line.contains(entry["location"]["locator"].as_str().unwrap()));
    }
}

/// @vtest.id TEST-CLI-TEST-QUERY-COMMANDS-QUIET
/// @vtest.covers VO-CLI-TEST-QUERY-COMMANDS-QUIET
/// @vtest.target crates/vtest-cli/src/lib.rs::run_test
/// @vtest.intent with `--quiet`, `vtest test list`, `vtest test query` and `vtest test show` write nothing to stdout or stderr and return the exit code they return without it, when that is 0, 1, and 2 by E-OP-001
/// @vtest.case exit code 0
/// @vtest.case exit code 1
/// @vtest.case exit code 2 by E-OP-001
#[test]
fn quiet_prints_nothing_and_keeps_the_exit_code() {
    let root = project("quiet");
    for (args, expected) in [
        (vec!["test", "list"], 0),
        (vec!["test", "list", "--unregistered"], 0),
        (vec!["test", "query", "--source", "SRC-ADD"], 0),
        (vec!["test", "show", "TEST-ADD"], 1),
        (vec!["test", "query", "--source", "SRC-MISSING"], 2),
        (vec!["test", "show", "TEST-NOBODY"], 2),
    ] {
        assert_eq!(exit(&vtest(&root, &args)), expected, "{args:?}");
        let mut quiet = vec!["--quiet"];
        quiet.extend_from_slice(&args);
        let output = vtest(&root, &quiet);
        assert_eq!(exit(&output), expected, "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
        assert!(output.stderr.is_empty(), "{args:?}");
    }

    fs::write(root.join("tests/gone.rs"), GONE_RS).unwrap();
    let output = vtest(&root, &["--quiet", "test", "list"]);
    assert_eq!(exit(&output), 1);
    assert!(output.stdout.is_empty() && output.stderr.is_empty());
}

/// @vtest.id TEST-CLI-TEST-QUERY-COMMANDS-WRITE-NOTHING
/// @vtest.covers VO-CLI-TEST-QUERY-COMMANDS-WRITE-NOTHING
/// @vtest.target crates/vtest-ops/src/test_query.rs::show
/// @vtest.intent `vtest test list`, `vtest test query` and `vtest test show` leave the set of files and directories below the project root and the content of every file unchanged, in JSON and in text, accepted or rejected
#[test]
fn the_query_commands_create_change_and_delete_nothing() {
    let root = project("no-writes");
    let before = tree(&root);
    for args in [
        vec!["test", "list"],
        vec!["test", "list", "--vo", "VO-X"],
        vec!["test", "list", "--unregistered"],
        vec!["test", "query", "--source", "SRC-ADD"],
        vec!["test", "query", "--source", "SRC-MISSING"],
        vec!["test", "show", "TEST-ADD"],
        vec!["test", "show", "TEST-NOBODY"],
    ] {
        for format in ["json", "text"] {
            let mut all = vec!["--format", format];
            all.extend_from_slice(&args);
            vtest(&root, &all);
            assert!(tree(&root) == before, "{args:?} --format {format}");
        }
    }
}

/// Commits the fixture, lockfile included, so `vtest run` sees the same
/// Execution State before and after the run.
fn commit(root: &Path) {
    let run = |program: &str, args: &[&str]| {
        let output = Command::new(program)
            .args(args)
            .current_dir(root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{program} {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    run("cargo", &["generate-lockfile", "--offline"]);
    run("git", &["init", "-q", "-b", "main"]);
    run(
        "git",
        &["config", "user.email", "vtest-fixture@example.com"],
    );
    run("git", &["config", "user.name", "vtest fixture"]);
    run("git", &["add", "."]);
    run("git", &["commit", "-q", "-m", "fixture"]);
}

fn submit(root: &Path, kind: &str, decision: &str, supersedes: &[&str]) -> String {
    let (code, bundle) = json_of(
        root,
        &["audit", "bundle", "--test", "TEST-ADD", "--kind", kind],
    );
    assert_eq!(code, 0, "bundle: {bundle}");
    let mut submission = json!({
        "bundle_id": bundle["data"]["bundle_id"],
        "subject": "TEST-ADD",
        "judgment_kind": kind,
        "decision": decision,
        "actor": {"kind": "human", "id": "reviewer"}
    });
    if !supersedes.is_empty() {
        submission["supersedes"] = json!(supersedes);
    }
    // Outside the project, so that the input is not part of it.
    let path = std::env::temp_dir().join(format!(
        "vtest-test-query-submission-{}-{}.json",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&path, submission.to_string()).unwrap();
    let (code, submitted) = json_of(root, &["audit", "submit", "--file", path.to_str().unwrap()]);
    fs::remove_file(&path).ok();
    assert_eq!(code, 0, "submit: {submitted}");
    submitted["data"]["decision_id"]
        .as_str()
        .unwrap_or_else(|| panic!("no decision_id: {submitted}"))
        .to_owned()
}

fn judgments(root: &Path) -> Value {
    let (_, shown) = json_of(root, &["test", "show", "TEST-ADD"]);
    shown["data"]["judgments"].clone()
}

/// @vtest.id TEST-CLI-TEST-SHOW-JUDGMENTS-BY-KIND
/// @vtest.covers VO-CLI-TEST-SHOW-JUDGMENTS-PER-KIND, VO-CLI-TEST-SHOW-JUDGMENT-ID-LISTS, VO-CLI-TEST-SHOW-JUDGMENT-DECISION, VO-OPS-TEST-SHOW-JUDGMENTS-SPLIT-BY-VALIDITY-AND-SUPERSEDES
/// @vtest.target crates/vtest-ops/src/test_query.rs::show
/// @vtest.target crates/vtest-store/src/decisions.rs::effective_set
/// @vtest.intent `data.judgments` has one element per judgment kind with a record of the Test, in ascending kind order; one accepted record is `effective` with its value as `decision`; two valid records with different values are both `effective` with `decision` null; a record that names another in `supersedes` leaves only itself `effective`, the named one `superseded` and its value as `decision`; after the Test changes every record is `invalid` only and `decision` is null
/// @vtest.case one accepted record
/// @vtest.case two valid records with different values
/// @vtest.case one record supersedes the others
/// @vtest.case Test changed after the records
#[test]
fn show_splits_judgment_records_into_effective_superseded_and_invalid() {
    let root = project("judgments");
    let accepted = submit(&root, "case-coverage", "accepted", &[]);
    assert_eq!(
        judgments(&root),
        json!([{
            "judgment_kind": "case-coverage",
            "decision": "accepted",
            "effective": [accepted],
            "superseded": [],
            "invalid": [],
        }])
    );

    let rejected = submit(&root, "case-coverage", "rejected", &[]);
    let semantic = submit(&root, "test-semantic", "accepted", &[]);
    let mut both = vec![accepted.clone(), rejected.clone()];
    both.sort();
    assert_eq!(
        judgments(&root),
        json!([
            {
                "judgment_kind": "case-coverage",
                "decision": null,
                "effective": both,
                "superseded": [],
                "invalid": [],
            },
            {
                "judgment_kind": "test-semantic",
                "decision": "accepted",
                "effective": [semantic],
                "superseded": [],
                "invalid": [],
            }
        ])
    );

    let deferred = submit(&root, "case-coverage", "deferred", &[&accepted, &rejected]);
    let shown = judgments(&root);
    assert_eq!(
        shown[0],
        json!({
            "judgment_kind": "case-coverage",
            "decision": "deferred",
            "effective": [deferred],
            "superseded": both,
            "invalid": [],
        })
    );

    let text = stdout(&vtest(&root, &["test", "show", "TEST-ADD"]));
    assert!(
        text.contains(&format!(
            "judgment case-coverage: deferred (effective: {deferred})"
        )),
        "{text}"
    );
    assert!(
        text.contains(&format!(
            "judgment test-semantic: accepted (effective: {semantic})"
        )),
        "{text}"
    );

    fs::write(
        root.join("tests/query.rs"),
        TESTS_RS.replace("add(2, 3), 5", "add(3, 3), 6"),
    )
    .unwrap();
    let mut all = vec![accepted, rejected, deferred];
    all.sort();
    assert_eq!(
        judgments(&root),
        json!([
            {
                "judgment_kind": "case-coverage",
                "decision": null,
                "effective": [],
                "superseded": [],
                "invalid": all,
            },
            {
                "judgment_kind": "test-semantic",
                "decision": null,
                "effective": [],
                "superseded": [],
                "invalid": [semantic],
            }
        ])
    );
    let text = stdout(&vtest(&root, &["test", "show", "TEST-ADD"]));
    assert!(
        text.contains("judgment case-coverage: UNKNOWN (effective: )"),
        "{text}"
    );
}

/// @vtest.id TEST-CLI-TEST-SHOW-IGNORES-RECORDS-OUTSIDE-THE-KIND-DOMAIN
/// @vtest.covers VO-CLI-TEST-SHOW-JUDGMENT-KIND-OUTSIDE-DOMAIN
/// @vtest.target crates/vtest-ops/src/test_query.rs::show
/// @vtest.intent a judgment record of the Test whose `judgment_kind` is outside the domain appears in no element of `data.judgments`
#[test]
fn show_leaves_out_judgment_records_with_a_kind_outside_the_domain() {
    let root = project("judgment-kind");
    let accepted = submit(&root, "case-coverage", "accepted", &[]);
    let decisions = root.join(".verify/decisions");
    let record = fs::read_to_string(decisions.join(format!("{accepted}.yaml"))).unwrap();
    assert!(record.contains("judgment_kind: case-coverage"), "{record}");
    fs::write(
        decisions.join("DEC-LEGACY.yaml"),
        record
            .replace(&accepted, "DEC-LEGACY")
            .replace("judgment_kind: case-coverage", "judgment_kind: legacy-kind"),
    )
    .unwrap();
    let shown = judgments(&root);
    assert_eq!(shown.as_array().map(Vec::len), Some(1), "{shown}");
    assert_eq!(shown[0]["judgment_kind"], "case-coverage");
    assert_eq!(shown[0]["effective"], json!([accepted]));
    assert!(!shown.to_string().contains("DEC-LEGACY"), "{shown}");
}

/// @vtest.id TEST-CLI-TEST-SHOW-EVIDENCE-AND-TEXT
/// @vtest.covers VO-CLI-TEST-SHOW-EVIDENCE-LATEST-RECORD, VO-CLI-TEST-SHOW-EVIDENCE-NEVER-AN-OLDER-RECORD, VO-OPS-TEST-SHOW-EVIDENCE-FOUR-FIELDS, VO-CLI-TEST-SHOW-TEXT-OUTPUT
/// @vtest.target crates/vtest-ops/src/test_query.rs::show
/// @vtest.target crates/vtest-cli/src/lib.rs::render_test_show
/// @vtest.intent `data.evidence` is null and the text has the line `evidence: none` before a run; after `vtest run` it holds exactly `id`, `result`, `executed_at` and `valid: true` of the Evidence that run wrote and after a second run those of the second; when the latest record is not a valid Evidence record, `valid` is false, `result` and `executed_at` are null and the older record is not shown; the text contains the Test ID, intent, each covered VO ID, each target reference, path, locator, each check name with its state, and the Evidence ID with its result
/// @vtest.case no Evidence record
/// @vtest.case one run
/// @vtest.case second run
/// @vtest.case latest record invalid
#[test]
fn show_reports_the_latest_evidence_record_and_prints_the_text_fields() {
    let root = project("evidence");
    let text = stdout(&vtest(&root, &["test", "show", "TEST-SUB"]));
    assert!(text.lines().any(|line| line == "evidence: none"), "{text}");

    commit(&root);
    // `--fast` keeps the run on `cargo test`, so the result does not depend
    // on whether cargo-llvm-cov is installed.
    let run = |root: &Path| -> String {
        let (code, ran) = json_of(root, &["run", "--test", "TEST-SUB", "--fast"]);
        assert_eq!(code, 0, "run: {ran}");
        assert_eq!(
            ran["data"]["evidence_ids"].as_array().map(Vec::len),
            Some(1)
        );
        ran["data"]["evidence_ids"][0]
            .as_str()
            .unwrap_or_else(|| panic!("no Evidence ID: {ran}"))
            .to_owned()
    };
    let first = run(&root);
    let (_, shown) = json_of(&root, &["test", "show", "TEST-SUB"]);
    let evidence = &shown["data"]["evidence"];
    assert_eq!(keys(evidence), ["executed_at", "id", "result", "valid"]);
    assert_eq!(evidence["id"], json!(first));
    assert_eq!(evidence["result"], "PASS");
    assert_eq!(evidence["valid"], true);
    assert!(evidence["executed_at"].is_string(), "{evidence}");

    let text = stdout(&vtest(&root, &["test", "show", "TEST-SUB"]));
    for expected in [
        "TEST-SUB",
        "intent: subtracts two numbers",
        "VO-X",
        "VO-Y",
        "target: rust-cargo::src/lib.rs::sub",
        "path: tests/query.rs",
        "locator: subtracts",
        &format!("evidence: {first} PASS"),
    ] {
        assert!(text.contains(expected), "missing `{expected}` in:\n{text}");
    }
    assert!(!text.contains("evidence: none"), "{text}");
    fn checks(node: &Value, out: &mut Vec<(String, String)>) {
        for check in node["checks"].as_array().into_iter().flatten() {
            out.push((
                check["check"].as_str().unwrap().to_owned(),
                check["state"].as_str().unwrap().to_owned(),
            ));
        }
        for child in node["children"].as_array().into_iter().flatten() {
            checks(child, out);
        }
    }
    let verification = &shown["data"]["verification"];
    let mut expected_checks = Vec::new();
    for check in verification["structural"].as_array().unwrap() {
        expected_checks.push((
            check["check"].as_str().unwrap().to_owned(),
            check["state"].as_str().unwrap().to_owned(),
        ));
    }
    for node in verification["tree"].as_array().unwrap() {
        checks(node, &mut expected_checks);
    }
    assert!(expected_checks.len() >= 4, "{verification}");
    for (check, state) in expected_checks {
        assert!(
            text.contains(&format!("check {check}: {state}")),
            "missing check {check}: {state} in:\n{text}"
        );
    }

    let second = run(&root);
    assert_ne!(second, first);
    let (_, shown) = json_of(&root, &["test", "show", "TEST-SUB"]);
    assert_eq!(shown["data"]["evidence"]["id"], json!(second));

    // The latest record stops being a valid Evidence record: the first one,
    // still valid, must not stand in for it (DS-1855).
    let latest = root.join(format!(".verify/evidence/{second}.yaml"));
    assert!(latest.is_file(), "{}", latest.display());
    let record = fs::read_to_string(&latest).unwrap();
    fs::write(&latest, record.replace("result: 'PASS'", "result: 'MAYBE'")).unwrap();
    let (_, shown) = json_of(&root, &["test", "show", "TEST-SUB"]);
    assert_eq!(
        shown["data"]["evidence"],
        json!({"id": second, "result": null, "executed_at": null, "valid": false})
    );
    let text = stdout(&vtest(&root, &["test", "show", "TEST-SUB"]));
    assert!(
        text.contains(&format!("evidence: {second} (invalid)")),
        "{text}"
    );
}
