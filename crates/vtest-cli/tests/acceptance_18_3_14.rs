//! Acceptance coverage for 別紙C §18.3.14 `vtest test show / list / query`
//! (DS-S167: DS-1837〜DS-1848, DS-1861〜DS-1877).
//!
//! The clauses that name the shipped `vtest` run the binary, and the MCP
//! clauses go through the real `vtest mcp` stdio transport. The clauses that
//! name the acceptance-only registry call the list and query operations
//! in-process with `rust-cargo` and `synthetic` registered, because
//! `synthetic` is not in the shipped registry.

mod support;

use std::path::Path;

use serde_json::{json, Value};
use support::spec_acceptance::{
    acceptance_registry, codes, commit, exit, json_of, keys, mcp_call, mcp_input_schema,
    mixed_project, run_test, rust_project, rust_project_with, stdout, submit_judgment, test_ids,
    tree, vtest, Project, GONE_RS, LIB_RS, TESTS_RS,
};
use vtest_model::ExitCode;
use vtest_ops::test_query::{list, query, ListFilter};

/// The stdout lines that are not diagnostics (`[code] message`).
fn result_lines(root: &Path, args: &[&str]) -> (i32, Vec<String>) {
    let output = vtest(root, args);
    let lines = stdout(&output)
        .lines()
        .filter(|line| !line.starts_with('['))
        .map(str::to_owned)
        .collect();
    (exit(&output), lines)
}

/// @vtest.id TEST-ACCEPTANCE-TEST-LIST-AGREES-WITH-EXPORT
/// @vtest.covers VO-ACCEPTANCE-TEST-LIST-IDS-EQUAL-EXPORT, VO-ACCEPTANCE-TEST-LIST-UNREGISTERED-EQUALS-EXPORT
/// @vtest.target crates/vtest-ops/src/test_query.rs::list
/// @vtest.intent the Test IDs of `data.tests` of the shipped `vtest test list --format json` equal, in order, the `id`s of `data.tests` of `vtest export --format json` for the same project, and every element has `execution` and `targets`; `vtest test list --unregistered` has at least one element in `data.unmanaged`, equal as a JSON value to `data.unmanaged` of the export
#[test]
fn list_agrees_with_the_test_specification() {
    let project = rust_project("list-export");
    let (_, exported) = json_of(&project.root, &["export"]);
    let (code, listed) = json_of(&project.root, &["test", "list"]);
    assert_eq!(code, 0, "{listed}");
    assert_eq!(test_ids(&listed["data"]), test_ids(&exported["data"]));
    assert_eq!(test_ids(&listed["data"]).len(), 3);
    for test in listed["data"]["tests"].as_array().unwrap() {
        assert!(test["execution"].is_object(), "{test}");
        assert!(test["targets"].is_array(), "{test}");
    }

    let (code, unregistered) = json_of(&project.root, &["test", "list", "--unregistered"]);
    assert_eq!(code, 0, "{unregistered}");
    let unmanaged = unregistered["data"]["unmanaged"].as_array().unwrap();
    assert!(!unmanaged.is_empty(), "{unregistered}");
    assert_eq!(
        unregistered["data"]["unmanaged"],
        exported["data"]["unmanaged"]
    );
}

/// @vtest.id TEST-ACCEPTANCE-TEST-LIST-VO-FILTER
/// @vtest.covers VO-ACCEPTANCE-TEST-LIST-VO-DECLARED-ONLY, VO-ACCEPTANCE-TEST-LIST-VO-WITH-UNREGISTERED-REJECTED
/// @vtest.target crates/vtest-ops/src/test_query.rs::list
/// @vtest.intent `vtest test list --vo` returns only the Tests that declare the VO ID in `covers`, including the Test that declares a VO ID without a VO record when that ID is given, and `--vo` together with `--unregistered` exits 2
/// @vtest.case VO with a record
/// @vtest.case VO without a record
/// @vtest.case --vo with --unregistered
#[test]
fn list_vo_returns_the_declaring_tests_only() {
    let project = rust_project("list-vo");
    project.write("tests/gone.rs", GONE_RS);
    let covering = |vo: &str| {
        let (_, listed) = json_of(&project.root, &["test", "list", "--vo", vo]);
        for test in listed["data"]["tests"].as_array().unwrap() {
            assert!(
                test["covers"].as_array().unwrap().contains(&json!(vo)),
                "{vo}: {test}"
            );
        }
        let mut ids = test_ids(&listed["data"]);
        ids.sort();
        ids
    };
    assert_eq!(covering("VO-X"), ["TEST-ADD", "TEST-SUB"]);
    assert_eq!(covering("VO-Y"), ["TEST-ADD-BY-SRC", "TEST-SUB"]);
    assert_eq!(covering("VO-GONE"), ["TEST-GONE"]);

    let rejected = vtest(
        &project.root,
        &["test", "list", "--vo", "VO-X", "--unregistered"],
    );
    assert_eq!(exit(&rejected), 2);
}

/// @vtest.id TEST-ACCEPTANCE-TEST-QUERY-SOURCE
/// @vtest.covers VO-ACCEPTANCE-TEST-QUERY-LOCATOR-AND-SRC-ID-SAME-RESULT, VO-ACCEPTANCE-TEST-QUERY-UNRESOLVED-SOURCE-E-OP-001
/// @vtest.target crates/vtest-ops/src/test_query.rs::query
/// @vtest.intent with one Test declaring a Source Target by Locator and another by SRC ID, `vtest test query --source` returns the same `data.tests` holding both for the Locator and for the SRC ID, without the Test of another Source Target; a reference to no Source Target, an adapter ID that is not registered and a value without `SRC-` and `::` are each rejected with E-OP-001, exit code 2, `ok: false` and `data: null`
/// @vtest.case Locator and SRC ID
/// @vtest.case no such Source Target
/// @vtest.case unregistered adapter ID
/// @vtest.case neither SRC ID nor Locator
#[test]
fn query_resolves_the_source_before_comparing() {
    let project = rust_project("query");
    let (code, by_locator) = json_of(
        &project.root,
        &["test", "query", "--source", "rust-cargo::src/lib.rs::add"],
    );
    assert_eq!(code, 0, "{by_locator}");
    let (code, by_src_id) = json_of(&project.root, &["test", "query", "--source", "SRC-ADD"]);
    assert_eq!(code, 0, "{by_src_id}");
    assert_eq!(by_locator["data"], by_src_id["data"]);
    let mut ids = test_ids(&by_locator["data"]);
    ids.sort();
    assert_eq!(ids, ["TEST-ADD", "TEST-ADD-BY-SRC"]);

    for source in [
        "rust-cargo::src/lib.rs::missing",
        "no-such-adapter::src/lib.rs::add",
        "add",
    ] {
        let (code, rejected) = json_of(&project.root, &["test", "query", "--source", source]);
        assert_eq!(code, 2, "{source}: {rejected}");
        assert_eq!(rejected["ok"], false, "{source}");
        assert_eq!(rejected["data"], Value::Null, "{source}");
        assert_eq!(codes(&rejected), ["E-OP-001"], "{source}");
    }
}

/// @vtest.id TEST-ACCEPTANCE-TEST-SHOW-EQUALS-VERIFY
/// @vtest.covers VO-ACCEPTANCE-TEST-SHOW-FOUR-KEYS-AND-VERIFY-RESULT, VO-ACCEPTANCE-TEST-SHOW-UNKNOWN-AND-DUPLICATED-ID-REJECTED
/// @vtest.target crates/vtest-ops/src/test_query.rs::show
/// @vtest.intent `data` of `vtest test show <Test ID> --format json` has the four keys `test`, `verification`, `judgments` and `evidence` only; `data.verification`, `ok`, `diagnostics` and the exit code equal `data`, `ok`, `diagnostics` and the exit code of `vtest verify --test <Test ID> --format json`; `data.test` has `execution` and `targets`; a Test ID no Test has is rejected with E-OP-001 and one two managed Tests have with E-SCAN-002, both with exit code 2
/// @vtest.case existing Test
/// @vtest.case unknown Test ID
/// @vtest.case Test ID of two managed Tests
#[test]
fn show_returns_what_verify_returns_for_the_test() {
    let project = rust_project("show");
    let (code, shown) = json_of(&project.root, &["test", "show", "TEST-SUB"]);
    assert_eq!(
        keys(&shown["data"]),
        ["evidence", "judgments", "test", "verification"]
    );
    let (verify_code, verified) = json_of(&project.root, &["verify", "--test", "TEST-SUB"]);
    assert_eq!(code, verify_code);
    assert_eq!(shown["data"]["verification"], verified["data"]);
    assert_eq!(shown["ok"], verified["ok"]);
    assert_eq!(shown["diagnostics"], verified["diagnostics"]);
    assert!(!verified["data"].is_null());
    assert_eq!(shown["data"]["test"]["id"], "TEST-SUB");
    assert!(shown["data"]["test"]["execution"].is_object());
    assert!(shown["data"]["test"]["targets"].is_array());

    let (code, unknown) = json_of(&project.root, &["test", "show", "TEST-NOBODY"]);
    assert_eq!((code, codes(&unknown)), (2, vec!["E-OP-001".to_owned()]));

    project.write(
        "tests/again.rs",
        "/// @vtest.id TEST-SUB\n/// @vtest.covers VO-X\n\
         /// @vtest.target src/lib.rs::sub\n/// @vtest.intent subtracts again\n\
         #[test]\nfn subtracts_twice() {}\n",
    );
    let (code, duplicated) = json_of(&project.root, &["test", "show", "TEST-SUB"]);
    assert_eq!(
        (code, codes(&duplicated)),
        (2, vec!["E-SCAN-002".to_owned()])
    );
}

/// The four inputs of the MCP tools with the CLI arguments they correspond to.
fn tool_cases() -> [(&'static str, Value, Vec<&'static str>); 4] {
    [
        (
            "test_get",
            json!({"id": "TEST-ADD"}),
            vec!["test", "show", "TEST-ADD"],
        ),
        (
            "test_query",
            json!({"vo": "VO-X"}),
            vec!["test", "list", "--vo", "VO-X"],
        ),
        (
            "test_query",
            json!({"source": "SRC-ADD"}),
            vec!["test", "query", "--source", "SRC-ADD"],
        ),
        (
            "test_query",
            json!({"unregistered": true}),
            vec!["test", "list", "--unregistered"],
        ),
    ]
}

/// @vtest.id TEST-ACCEPTANCE-TEST-QUERY-MCP-TOOLS
/// @vtest.covers VO-ACCEPTANCE-TEST-MCP-RESULTS-EQUAL-CLI, VO-ACCEPTANCE-TEST-MCP-INPUT-COUNT-REJECTED, VO-ACCEPTANCE-TEST-MCP-TOOLS-LISTED, VO-ACCEPTANCE-TEST-MCP-INPUT-SCHEMAS, VO-ACCEPTANCE-TEST-MCP-OTHER-ARGUMENTS-REJECTED
/// @vtest.target crates/vtest-mcp/src/lib.rs::tools_call_result
/// @vtest.intent over the real `vtest mcp` stdio transport, `tools/list` names `test_get` and `test_query` with their input schemas (`id` only and required; `vo`, `source` and `unregistered` only; no additional property); `test_get` equals `vtest test show --format json` and `test_query` with `vo`, `source` and `unregistered` equals the corresponding CLI output in `ok`, `data` and `diagnostics`, with `isError` true only when `ok` is false; `test_query` without input, with `vo` and `source`, with `unregistered: false`, `test_get` without `id`, and either tool with another argument are E-OP-001 tool errors
/// @vtest.case results without an error diagnostic
/// @vtest.case results with an error diagnostic
/// @vtest.case rejected input counts
/// @vtest.case other arguments
#[test]
fn the_mcp_tools_return_what_the_cli_prints() {
    let project = rust_project("mcp");
    let root = &project.root;

    let get = mcp_input_schema(root, "test_get").expect("test_get is listed");
    assert_eq!(get["properties"], json!({"id": {"type": "string"}}));
    assert_eq!(get["required"], json!(["id"]));
    assert_eq!(get["additionalProperties"], false);
    let query = mcp_input_schema(root, "test_query").expect("test_query is listed");
    assert_eq!(
        query["properties"],
        json!({
            "vo": {"type": "string"},
            "source": {"type": "string"},
            "unregistered": {"type": "boolean"}
        })
    );
    assert_eq!(query["additionalProperties"], false);

    let compare = |scan_has_error: bool| {
        for (tool, arguments, cli_args) in tool_cases() {
            let (_, via_cli) = json_of(root, &cli_args);
            let via_mcp = mcp_call(root, tool, arguments.clone());
            let envelope = &via_mcp["structuredContent"];
            for key in ["ok", "data", "diagnostics"] {
                assert_eq!(envelope[key], via_cli[key], "{tool} {arguments}: {key}");
            }
            assert!(!via_cli["data"].is_null(), "{tool} {arguments}");
            // `test show` carries a verification without Evidence: not OK.
            let ok = !scan_has_error && tool != "test_get";
            assert_eq!(via_cli["ok"], ok, "{tool} {arguments}");
            assert_eq!(via_mcp["isError"], !ok, "{tool} {arguments}");
        }
    };
    compare(false);

    for (tool, arguments) in [
        ("test_query", json!({})),
        ("test_query", json!({"vo": "VO-X", "source": "SRC-ADD"})),
        ("test_query", json!({"unregistered": false})),
        ("test_get", json!({})),
        ("test_get", json!({"id": "TEST-ADD", "format": "json"})),
        ("test_query", json!({"vo": "VO-X", "id": "TEST-ADD"})),
        ("test_query", json!({"test": "TEST-ADD"})),
    ] {
        let rejected = mcp_call(root, tool, arguments.clone());
        assert_eq!(rejected["isError"], true, "{tool} {arguments}");
        assert_eq!(
            rejected["structuredContent"]["diagnostics"][0]["code"], "E-OP-001",
            "{tool} {arguments}"
        );
    }

    project.write("tests/gone.rs", GONE_RS);
    compare(true);
}

/// @vtest.id TEST-ACCEPTANCE-TEST-QUERY-WRITES-NOTHING
/// @vtest.covers VO-ACCEPTANCE-TEST-QUERY-PROJECT-FILES-UNCHANGED
/// @vtest.target crates/vtest-ops/src/test_query.rs::show
/// @vtest.intent the set of files and directories below the project root and the content of every file are the same before and after `vtest test list`, `vtest test query` and `vtest test show`, and after calls of `test_get` and `test_query`
#[test]
fn the_query_commands_and_tools_leave_the_project_files_as_they_were() {
    let project = rust_project("no-writes");
    let before = tree(&project.root);
    for (tool, arguments, cli_args) in tool_cases() {
        vtest(&project.root, &cli_args);
        assert!(tree(&project.root) == before, "{cli_args:?}");
        let mut json_args = vec!["--format", "json"];
        json_args.extend_from_slice(&cli_args);
        vtest(&project.root, &json_args);
        assert!(tree(&project.root) == before, "{json_args:?}");
        let called = mcp_call(&project.root, tool, arguments.clone());
        assert!(!called["structuredContent"]["data"].is_null(), "{called}");
        assert!(tree(&project.root) == before, "{tool} {arguments}");
    }
    vtest(&project.root, &["test", "list"]);
    assert!(tree(&project.root) == before, "test list");
}

fn operation_data(result: (ExitCode, Value)) -> Value {
    let (_, envelope) = result;
    assert!(!envelope["data"].is_null(), "{envelope}");
    envelope["data"].clone()
}

fn adapters_of(data: &Value) -> Vec<String> {
    let mut adapters: Vec<String> = data["tests"]
        .as_array()
        .unwrap()
        .iter()
        .map(|test| test["execution"]["adapter"].as_str().unwrap().to_owned())
        .collect();
    adapters.sort();
    adapters.dedup();
    adapters
}

fn mixed_data(project: &Project, reversed: bool) -> [Value; 4] {
    let registry = acceptance_registry(reversed);
    [
        operation_data(list(&project.root, &registry, &ListFilter::All)),
        operation_data(list(
            &project.root,
            &registry,
            &ListFilter::Vo("VO-MIXED".to_owned()),
        )),
        operation_data(list(&project.root, &registry, &ListFilter::Unregistered)),
        operation_data(query(
            &project.root,
            &registry,
            "synthetic::synthetic-target-b",
        )),
    ]
}

/// @vtest.id TEST-ACCEPTANCE-TEST-QUERY-MIXED-ADAPTERS
/// @vtest.covers VO-ACCEPTANCE-TEST-LIST-MIXED-BOTH-ADAPTERS, VO-ACCEPTANCE-TEST-LIST-MIXED-UNREGISTERED-EQUALS-EXPORT, VO-ACCEPTANCE-TEST-QUERY-MIXED-REVERSED-IDENTICAL, VO-ACCEPTANCE-TEST-QUERY-MIXED-SYNTHETIC-SOURCE
/// @vtest.target crates/vtest-ops/src/test_query.rs::list
/// @vtest.target crates/vtest-ops/src/test_query.rs::query
/// @vtest.intent with the acceptance-only registry of `rust-cargo` and `synthetic` on a mixed project, the list operation returns the Tests of both adapters, each with `execution`, with Test IDs in the order of the export operation's `data.tests`; the unregistered list has at least one element and equals the export operation's `data.unmanaged`; a query for a `synthetic` Source Target returns the `synthetic` Tests that declare it and no `rust-cargo` Test; and reversing the registration order and every enumeration of the `synthetic` adapter gives byte-identical serialized `data` for the unfiltered list, the list by VO, the unregistered list and the query
/// @vtest.case list against export
/// @vtest.case unregistered against export
/// @vtest.case query for a synthetic Source Target
/// @vtest.case reversed registration and enumeration
#[test]
fn list_and_query_handle_both_adapters_of_a_mixed_project() {
    let project = mixed_project("mixed");
    let exported = vtest_ops::export::execute(&project.root, &acceptance_registry(false))
        .data
        .expect("export data");
    let exported = serde_json::to_value(&exported).unwrap();
    let [all, by_vo, unregistered, by_source] = mixed_data(&project, false);

    assert_eq!(adapters_of(&all), ["rust-cargo", "synthetic"]);
    assert_eq!(test_ids(&all), test_ids(&exported));
    assert_eq!(test_ids(&all).len(), 5);
    for test in all["tests"].as_array().unwrap() {
        assert!(test["execution"].is_object(), "{test}");
    }
    assert_eq!(test_ids(&by_vo).len(), 5);

    let unmanaged = unregistered["unmanaged"].as_array().unwrap();
    assert!(!unmanaged.is_empty(), "{unregistered}");
    assert_eq!(unregistered["unmanaged"], exported["unmanaged"]);

    assert_eq!(adapters_of(&by_source), ["synthetic"]);
    let mut ids = test_ids(&by_source);
    ids.sort();
    assert_eq!(ids, ["TEST-MIXED-SYN-A2", "TEST-MIXED-SYN-B1"]);

    let forward = mixed_data(&project, false);
    let reversed = mixed_data(&project, true);
    for (forward, reversed) in forward.iter().zip(&reversed) {
        assert_eq!(
            serde_json::to_string(forward).unwrap(),
            serde_json::to_string(reversed).unwrap()
        );
    }
    assert_eq!(adapters_of(&reversed[0]), ["rust-cargo", "synthetic"]);
}

/// @vtest.id TEST-ACCEPTANCE-TEST-QUERY-ARGUMENTS
/// @vtest.covers VO-ACCEPTANCE-TEST-SHOW-ARGUMENT-FORMS, VO-ACCEPTANCE-TEST-LIST-AND-QUERY-ARGUMENT-FORMS
/// @vtest.target crates/vtest-cli/src/lib.rs::TestCommand
/// @vtest.intent `vtest test show` exits 2 without a Test ID, with two Test IDs and with an option that is not global, and is not rejected as an argument error with `--format json`, `--format text` and `--project <dir>`; `vtest test list` exits 2 for a positional argument and for `--vo` given twice; `vtest test query` exits 2 without `--source`, with `--source` given twice and with a positional argument
/// @vtest.case test show
/// @vtest.case test list
/// @vtest.case test query
#[test]
fn the_query_commands_reject_arguments_outside_their_forms() {
    let project = rust_project("arguments");
    let root = &project.root;
    for args in [
        vec!["test", "show"],
        vec!["test", "show", "TEST-ADD", "TEST-SUB"],
        vec!["test", "show", "TEST-ADD", "--vo", "VO-X"],
        vec!["test", "list", "TEST-ADD"],
        vec!["test", "list", "--vo", "VO-X", "--vo", "VO-Y"],
        vec!["test", "query"],
        vec![
            "test", "query", "--source", "SRC-ADD", "--source", "SRC-ADD",
        ],
        vec!["test", "query", "--source", "SRC-ADD", "TEST-ADD"],
    ] {
        assert_eq!(exit(&vtest(root, &args)), 2, "{args:?}");
    }

    // `--project` is given by `vtest`; the verification has no Evidence, so
    // an accepted `test show` exits 1, not 2.
    for args in [
        vec!["test", "show", "TEST-ADD", "--format", "json"],
        vec!["test", "show", "TEST-ADD", "--format", "text"],
        vec!["test", "show", "TEST-ADD"],
    ] {
        let output = vtest(root, &args);
        assert_eq!(exit(&output), 1, "{args:?}");
        assert!(stdout(&output).contains("TEST-ADD"), "{args:?}");
    }
    let trailing = std::process::Command::new(env!("CARGO_BIN_EXE_vtest"))
        .args(["test", "show", "TEST-ADD", "--project"])
        .arg(root)
        .output()
        .unwrap();
    assert_eq!(exit(&trailing), 1);
    assert!(stdout(&trailing).contains("TEST-ADD"));
}

/// @vtest.id TEST-ACCEPTANCE-TEST-LIST-QUERY-SCAN-ERROR
/// @vtest.covers VO-ACCEPTANCE-TEST-LIST-QUERY-EXIT-1-WITH-DATA
/// @vtest.target crates/vtest-ops/src/test_query.rs::list
/// @vtest.intent with a Test whose `covers` cannot be resolved, `vtest test list --format json` and `vtest test query --format json` for that Test's target exit 1 with `ok: false`, non-null `data.tests` holding that Test and E-SCAN-003 in `diagnostics`
#[test]
fn list_and_query_keep_their_result_next_to_a_scan_error() {
    let project = rust_project("scan-error");
    project.write("tests/gone.rs", GONE_RS);
    for args in [
        vec!["test", "list"],
        vec!["test", "query", "--source", "rust-cargo::src/lib.rs::sub"],
    ] {
        let (code, result) = json_of(&project.root, &args);
        assert_eq!(code, 1, "{args:?}: {result}");
        assert_eq!(result["ok"], false, "{args:?}");
        assert!(
            test_ids(&result["data"]).contains(&"TEST-GONE".to_owned()),
            "{args:?}"
        );
        assert!(
            codes(&result).contains(&"E-SCAN-003".to_owned()),
            "{args:?}"
        );
    }
}

/// @vtest.id TEST-ACCEPTANCE-TEST-LIST-QUERY-TEXT-AND-QUIET
/// @vtest.covers VO-ACCEPTANCE-TEST-LIST-QUERY-TEXT-LINES, VO-ACCEPTANCE-TEST-QUERY-COMMANDS-QUIET-EXIT-CODES
/// @vtest.target crates/vtest-cli/src/lib.rs::render_test_list
/// @vtest.target crates/vtest-cli/src/lib.rs::run_test
/// @vtest.intent with two or more managed Tests, `--format text` of `vtest test list` and `vtest test query` has each Test ID of the JSON `data.tests`, in the same order, on its own line, `vtest test list --unregistered --format text` has the path and locator of each `data.unmanaged` element on its own line, and the exit codes equal those of `--format json`; with `--quiet` the three commands print nothing on stdout and stderr and return the exit code they return without it, for 0, 1, and 2 by E-OP-001
/// @vtest.case text of list and query
/// @vtest.case text of list --unregistered
/// @vtest.case --quiet with exit codes 0, 1 and 2
#[test]
fn text_lists_one_test_per_line_and_quiet_prints_nothing() {
    let project = rust_project("text");
    let root = &project.root;
    for args in [
        vec!["test", "list"],
        vec!["test", "query", "--source", "SRC-ADD"],
    ] {
        let (json_code, json) = json_of(root, &args);
        let expected = test_ids(&json["data"]);
        assert!(expected.len() >= 2, "{args:?}");
        let mut text_args = vec!["--format", "text"];
        text_args.extend_from_slice(&args);
        let (text_code, lines) = result_lines(root, &text_args);
        assert_eq!(text_code, json_code, "{args:?}");
        assert_eq!(lines.len(), expected.len(), "{args:?}: {lines:?}");
        for (line, id) in lines.iter().zip(&expected) {
            assert!(
                line.split_whitespace().any(|word| word == id),
                "{id} not in `{line}`"
            );
        }
    }

    project.write("tests/more.rs", "#[test]\nfn another_bare() {}\n");
    let args = ["test", "list", "--unregistered"];
    let (json_code, json) = json_of(root, &args);
    let unmanaged = json["data"]["unmanaged"].as_array().unwrap();
    assert_eq!(unmanaged.len(), 2, "{json}");
    let (text_code, lines) = result_lines(
        root,
        &["--format", "text", "test", "list", "--unregistered"],
    );
    assert_eq!(text_code, json_code);
    assert_eq!(lines.len(), unmanaged.len(), "{lines:?}");
    for (line, entry) in lines.iter().zip(unmanaged) {
        assert!(line.contains(entry["location"]["path"].as_str().unwrap()));
        assert!(line.contains(entry["location"]["locator"].as_str().unwrap()));
    }

    for (args, expected) in [
        (vec!["test", "list"], 0),
        (vec!["test", "query", "--source", "SRC-ADD"], 0),
        (vec!["test", "show", "TEST-ADD"], 1),
        (vec!["test", "query", "--source", "SRC-MISSING"], 2),
        (vec!["test", "show", "TEST-NOBODY"], 2),
    ] {
        assert_eq!(exit(&vtest(root, &args)), expected, "{args:?}");
        let mut quiet_args = args.clone();
        quiet_args.push("--quiet");
        let quiet = vtest(root, &quiet_args);
        assert_eq!(exit(&quiet), expected, "{args:?}");
        assert!(
            quiet.stdout.is_empty() && quiet.stderr.is_empty(),
            "{args:?}"
        );
    }
    project.write("tests/gone.rs", GONE_RS);
    for args in [
        vec!["test", "list"],
        vec!["test", "query", "--source", "rust-cargo::src/lib.rs::sub"],
    ] {
        assert_eq!(exit(&vtest(root, &args)), 1, "{args:?}");
        let mut quiet_args = args.clone();
        quiet_args.push("--quiet");
        let quiet = vtest(root, &quiet_args);
        assert_eq!(exit(&quiet), 1, "{args:?}");
        assert!(
            quiet.stdout.is_empty() && quiet.stderr.is_empty(),
            "{args:?}"
        );
    }
}

fn judgments(root: &Path) -> Value {
    let (_, shown) = json_of(root, &["test", "show", "TEST-ADD"]);
    shown["data"]["judgments"].clone()
}

fn case_coverage(root: &Path) -> Value {
    judgments(root)
        .as_array()
        .unwrap()
        .iter()
        .find(|judgment| judgment["judgment_kind"] == "case-coverage")
        .cloned()
        .unwrap_or_else(|| panic!("no case-coverage element"))
}

fn pending_case_coverage(root: &Path) -> usize {
    let (_, verified) = json_of(root, &["verify", "--test", "TEST-ADD"]);
    verified["data"]["pending"]
        .as_array()
        .unwrap_or_else(|| panic!("no pending list: {verified}"))
        .iter()
        .filter(|item| item.to_string().contains("case-coverage"))
        .count()
}

/// @vtest.id TEST-ACCEPTANCE-TEST-SHOW-JUDGMENTS
/// @vtest.covers VO-ACCEPTANCE-TEST-SHOW-ACCEPTED-JUDGMENT, VO-ACCEPTANCE-TEST-SHOW-CONFLICTING-AND-SUPERSEDING-JUDGMENTS, VO-ACCEPTANCE-TEST-SHOW-INVALID-AND-ABSENT-JUDGMENTS
/// @vtest.target crates/vtest-ops/src/test_query.rs::show
/// @vtest.intent a Test without a judgment record has an empty `data.judgments`; once its `case-coverage` is effectively `accepted`, `vtest verify --test` no longer lists that item as pending and `data.judgments` has the element with `decision: accepted`, that decision ID alone in `effective` and empty `superseded` and `invalid`; a second valid record with another value gives `decision: null` and both IDs in `effective`; a record that names the other in `supersedes` gives its own value as `decision`, itself alone in `effective` and the named record alone in `superseded`; and after the Test changes the decision IDs appear in `invalid` only and `decision` is null
/// @vtest.case no record
/// @vtest.case accepted
/// @vtest.case two values
/// @vtest.case supersedes
/// @vtest.case Test changed
#[test]
fn show_reports_the_state_of_the_judgment_records() {
    // With a declared case, `verify` lists the Test's `case-coverage` as
    // pending until its effective judgment is `accepted` (DS-1441).
    let with_case = TESTS_RS.replace(
        "/// @vtest.intent adds two numbers\n",
        "/// @vtest.intent adds two numbers\n/// @vtest.case two and three\n",
    );
    let project = rust_project_with("judgments", LIB_RS, &with_case);
    let root = &project.root;
    assert_eq!(judgments(root), json!([]));
    assert_eq!(pending_case_coverage(root), 1);

    let accepted = submit_judgment(root, "TEST-ADD", "case-coverage", "accepted", &[]);
    assert_eq!(pending_case_coverage(root), 0);
    assert_eq!(
        case_coverage(root),
        json!({
            "judgment_kind": "case-coverage",
            "decision": "accepted",
            "effective": [accepted],
            "superseded": [],
            "invalid": [],
        })
    );

    let rejected = submit_judgment(root, "TEST-ADD", "case-coverage", "rejected", &[]);
    let conflicting = case_coverage(root);
    assert_eq!(conflicting["decision"], Value::Null);
    let mut both = vec![accepted.clone(), rejected.clone()];
    both.sort();
    assert_eq!(conflicting["effective"], json!(both));

    // A fresh project for the pair where one names the other.
    let project = rust_project_with("judgments-supersedes", LIB_RS, &with_case);
    let root = &project.root;
    let first = submit_judgment(root, "TEST-ADD", "case-coverage", "rejected", &[]);
    let second = submit_judgment(root, "TEST-ADD", "case-coverage", "accepted", &[&first]);
    assert_eq!(
        case_coverage(root),
        json!({
            "judgment_kind": "case-coverage",
            "decision": "accepted",
            "effective": [second],
            "superseded": [first],
            "invalid": [],
        })
    );

    project.write(
        "tests/acceptance.rs",
        &with_case.replace("add(2, 3), 5", "add(3, 3), 6"),
    );
    let changed = case_coverage(root);
    assert_eq!(changed["decision"], Value::Null);
    assert_eq!(changed["effective"], json!([]));
    assert_eq!(changed["superseded"], json!([]));
    let mut all = vec![first, second];
    all.sort();
    assert_eq!(changed["invalid"], json!(all));
}

/// @vtest.id TEST-ACCEPTANCE-TEST-SHOW-EVIDENCE-AND-TEXT
/// @vtest.covers VO-ACCEPTANCE-TEST-SHOW-EVIDENCE-AFTER-RUNS, VO-ACCEPTANCE-TEST-SHOW-TEXT-FIELDS, VO-ACCEPTANCE-TEST-GET-JUDGMENTS-AND-EVIDENCE-EQUAL-SHOW
/// @vtest.target crates/vtest-ops/src/test_query.rs::show
/// @vtest.target crates/vtest-cli/src/lib.rs::render_test_show
/// @vtest.intent `data.evidence` is null and the text has the line `evidence: none` for a Test without an Evidence record; after `vtest run` it has the Evidence ID that run printed, its result and `valid: true`, and after a second run the second ID; for a Test with a judgment record and an Evidence record, the text of `vtest test show` has the Test ID, intent, each covered VO ID, path, locator, each check name with its state, each judgment kind with its effective judgment and effective decision IDs, and the Evidence ID with its result, and `test_get` returns at least one `data.judgments` element and a non-null `data.evidence`, both equal to those of `vtest test show --format json`
/// @vtest.case no Evidence record
/// @vtest.case first run
/// @vtest.case second run
/// @vtest.case text and test_get with a judgment and Evidence
#[test]
fn show_reports_the_latest_evidence_and_prints_every_field_as_text() {
    let project = rust_project("evidence");
    let root = &project.root;
    commit(root);
    let (_, shown) = json_of(root, &["test", "show", "TEST-SUB"]);
    assert_eq!(shown["data"]["evidence"], Value::Null);
    let text = stdout(&vtest(
        root,
        &["test", "show", "TEST-SUB", "--format", "text"],
    ));
    assert!(text.lines().any(|line| line == "evidence: none"), "{text}");

    let first = run_test(root, "TEST-SUB");
    let (_, shown) = json_of(root, &["test", "show", "TEST-SUB"]);
    let evidence = &shown["data"]["evidence"];
    assert_eq!(evidence["id"], json!(first));
    assert_eq!(evidence["result"], "PASS");
    assert_eq!(evidence["valid"], true);

    let second = run_test(root, "TEST-SUB");
    assert_ne!(second, first);
    let decision = submit_judgment(root, "TEST-SUB", "test-semantic", "accepted", &[]);
    let (_, shown) = json_of(root, &["test", "show", "TEST-SUB"]);
    let data = &shown["data"];
    assert_eq!(data["evidence"]["id"], json!(second));
    assert_eq!(data["judgments"].as_array().map(Vec::len), Some(1));

    let text = stdout(&vtest(
        root,
        &["test", "show", "TEST-SUB", "--format", "text"],
    ));
    let mut expected = vec![
        "TEST-SUB".to_owned(),
        "subtracts two numbers".to_owned(),
        "VO-X".to_owned(),
        "VO-Y".to_owned(),
        "tests/acceptance.rs".to_owned(),
        "locator: subtracts".to_owned(),
        format!("judgment test-semantic: accepted (effective: {decision})"),
        format!("evidence: {second} PASS"),
    ];
    fn checks(node: &Value, out: &mut Vec<String>) {
        for check in node["checks"].as_array().into_iter().flatten() {
            out.push(format!(
                "check {}: {}",
                check["check"].as_str().unwrap(),
                check["state"].as_str().unwrap()
            ));
        }
        for child in node["children"].as_array().into_iter().flatten() {
            checks(child, out);
        }
    }
    let before = expected.len();
    for check in data["verification"]["structural"].as_array().unwrap() {
        expected.push(format!(
            "check {}: {}",
            check["check"].as_str().unwrap(),
            check["state"].as_str().unwrap()
        ));
    }
    for node in data["verification"]["tree"].as_array().unwrap() {
        checks(node, &mut expected);
    }
    assert!(expected.len() >= before + 4, "{data}");
    for needle in &expected {
        assert!(text.contains(needle), "missing `{needle}` in:\n{text}");
    }
    assert!(!text.contains("evidence: none"), "{text}");

    let via_mcp = mcp_call(root, "test_get", json!({"id": "TEST-SUB"}));
    let got = &via_mcp["structuredContent"]["data"];
    assert!(!got["judgments"].as_array().unwrap().is_empty(), "{got}");
    assert!(!got["evidence"].is_null(), "{got}");
    assert_eq!(got["judgments"], data["judgments"]);
    assert_eq!(got["evidence"], data["evidence"]);
}
