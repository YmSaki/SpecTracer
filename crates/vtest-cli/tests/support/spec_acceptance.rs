//! Fixtures and runners shared by the acceptance tests of the test
//! specification output (`acceptance_18_3_13.rs`) and of `vtest test show /
//! list / query` (`acceptance_18_3_14.rs`): a `rust-cargo` project, a mixed
//! `rust-cargo` + `synthetic` project with its acceptance-only registry
//! (DS-1780, DS-1861), and the `vtest` binary over the CLI and the MCP stdio
//! transport.

use std::{
    env, fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

use serde_json::{json, Value};
use vtest_adapter_api::{
    Adapter, AdapterDescriptor, AdapterRegistry, AdapterScanConfig, DiscoveryError,
    DiscoveryOutcome, SourceDiscoveryAdapter,
};
use vtest_adapter_rust::RustCargoAdapter;
use vtest_adapter_synthetic::SyntheticAdapter;
use vtest_model::{
    DerivesFrom, DocumentFile, DocumentId, NodeSource, RootNode, SentenceNode, VoId, VoRecord,
};
use vtest_store::{init_project, write_document_file, write_vo_record, VerifyLayout};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A project in a temporary directory, removed when dropped.
pub struct Project {
    pub root: PathBuf,
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Project {
    pub fn write(&self, relative: &str, text: &str) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("create directory");
        fs::write(path, text).expect("write file");
    }
}

fn temp_root(name: &str) -> PathBuf {
    let root = env::temp_dir().join(format!(
        "vtest-spec-acceptance-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create project root");
    root
}

fn write_records(root: &Path, vo_ids: &[&str]) {
    let layout = VerifyLayout::new(root);
    let source = |id: &str| NodeSource {
        doc: format!("{id}.md"),
        heading: "acceptance".to_owned(),
        lines: [1, 1],
    };
    let document = DocumentFile {
        schema_version: "0.1".to_owned(),
        root: vec![RootNode {
            id: DocumentId::new("ROOT-001"),
            statement: "acceptance root".to_owned(),
            description: None,
            source: source("root"),
        }],
        request: vec![SentenceNode {
            id: DocumentId::new("R-001"),
            statement: "acceptance request".to_owned(),
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
    write_document_file(&layout, "acceptance", &document).expect("write document file");
    for id in vo_ids {
        write_vo_record(
            &layout,
            &VoRecord {
                id: VoId::new(*id),
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
        .expect("write VO record");
    }
}

/// `add` declares the permanent SRC ID `SRC-ADD`; `sub` and `unused` do not.
pub const LIB_RS: &str = "/// @vtest.src-id SRC-ADD\n\
pub fn add(a: i32, b: i32) -> i32 { a + b }\n\
pub fn sub(a: i32, b: i32) -> i32 { a - b }\n\
pub fn unused() {}\n";

/// Three managed Tests and one test function without a declaration:
/// `TEST-ADD` (VO-X, `add` by Locator), `TEST-SUB` (VO-X and VO-Y, `sub`),
/// `TEST-ADD-BY-SRC` (VO-Y, `add` by SRC ID) and `bare`.
pub const TESTS_RS: &str = "/// @vtest.id TEST-ADD\n\
/// @vtest.covers VO-X\n\
/// @vtest.target src/lib.rs::add\n\
/// @vtest.intent adds two numbers\n\
#[test]\n\
fn adds() {\n    assert_eq!(acceptance_fixture::add(2, 3), 5);\n}\n\n\
/// @vtest.id TEST-SUB\n\
/// @vtest.covers VO-X, VO-Y\n\
/// @vtest.target src/lib.rs::sub\n\
/// @vtest.intent subtracts two numbers\n\
#[test]\n\
fn subtracts() {\n    assert_eq!(acceptance_fixture::sub(5, 3), 2);\n}\n\n\
/// @vtest.id TEST-ADD-BY-SRC\n\
/// @vtest.covers VO-Y\n\
/// @vtest.target SRC-ADD\n\
/// @vtest.intent adds through the SRC ID\n\
#[test]\n\
fn adds_again() {\n    assert_eq!(acceptance_fixture::add(1, 1), 2);\n}\n\n\
#[test]\n\
fn bare() {\n    assert_eq!(acceptance_fixture::sub(1, 1), 0);\n}\n";

/// A Test whose `covers` names a VO without a VO record (E-SCAN-003).
pub const GONE_RS: &str = "/// @vtest.id TEST-GONE\n\
/// @vtest.covers VO-GONE\n\
/// @vtest.target src/lib.rs::sub\n\
/// @vtest.intent covers a VO without a record\n\
#[test]\n\
fn gone() {\n    assert_eq!(acceptance_fixture::sub(2, 1), 1);\n}\n";

/// A `rust-cargo` project with the VO records `VO-X` and `VO-Y`, `src/lib.rs`
/// = `lib_rs` and `tests/acceptance.rs` = `tests_rs`.
pub fn rust_project_with(name: &str, lib_rs: &str, tests_rs: &str) -> Project {
    let project = Project {
        root: temp_root(name),
    };
    init_project(&project.root, "spec-acceptance").expect("init project");
    project.write(
        "Cargo.toml",
        "[package]\nname = \"acceptance-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
    );
    project.write("src/lib.rs", lib_rs);
    project.write("tests/acceptance.rs", tests_rs);
    write_records(&project.root, &["VO-X", "VO-Y"]);
    project
}

/// [`rust_project_with`] [`LIB_RS`] and [`TESTS_RS`]: the scan reports
/// W-SCAN-101 for `bare` and no error.
pub fn rust_project(name: &str) -> Project {
    rust_project_with(name, LIB_RS, TESTS_RS)
}

const MIXED_CONFIG: &str = r#"version: 2
project:
  name: mixed-fixture
adapters:
  - id: synthetic
    roots:
      - '.'
    scan:
      include:
        - '.'
      assertion_macros: []
    run:
      coverage: none
  - id: rust-cargo
    roots:
      - '.'
    scan:
      include:
        - src
        - tests
      assertion_macros: []
    run:
      coverage: llvm-cov
verify:
  full_scope:
    - chain_integrity
    - orphan_detection
    - target_binding
    - oracle_presence
"#;

/// A project both adapters return managed Tests for: two `rust-cargo` Tests
/// and one test function without a declaration in `tests/rust_side.rs`, and
/// three `synthetic` Tests with two Source Targets in two `.synthetic`
/// files. `VO-MIXED` has a record; `VO-GONE` (declared by one Test of each
/// adapter) has none.
pub fn mixed_project(name: &str) -> Project {
    let project = Project {
        root: temp_root(name),
    };
    init_project(&project.root, "mixed-fixture").expect("init project");
    project.write(".verify/config.yaml", MIXED_CONFIG);
    project.write(
        "Cargo.toml",
        "[package]\nname = \"mixed-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
    );
    project.write("src/lib.rs", "pub fn implementation() -> bool { true }\n");
    project.write(
        "tests/rust_side.rs",
        "/// @vtest.id TEST-MIXED-RUST-A\n\
         /// @vtest.covers VO-MIXED\n\
         /// @vtest.target src/lib.rs::implementation\n\
         /// @vtest.intent exercises the rust-cargo side\n\
         #[test]\n\
         fn rust_side() { assert!(mixed_fixture::implementation()); }\n\n\
         /// @vtest.id TEST-MIXED-RUST-B\n\
         /// @vtest.covers VO-MIXED, VO-GONE\n\
         /// @vtest.target src/lib.rs::implementation\n\
         /// @vtest.intent exercises the rust-cargo side again\n\
         #[test]\n\
         fn rust_side_again() { assert!(mixed_fixture::implementation()); }\n\n\
         #[test]\n\
         fn bare() { assert!(mixed_fixture::implementation()); }\n",
    );
    project.write(
        "a.synthetic",
        "@synthetic.test\n\
         @synthetic.id TEST-MIXED-SYN-A1\n\
         @synthetic.covers VO-MIXED\n\
         @synthetic.target synthetic-target-a\n\
         @synthetic.intent exercises the first synthetic target\n\
         plain synthetic Test construct body\n\
         ---\n\
         @synthetic.test\n\
         @synthetic.id TEST-MIXED-SYN-A2\n\
         @synthetic.covers VO-MIXED, VO-GONE\n\
         @synthetic.target synthetic-target-b\n\
         @synthetic.intent exercises the second synthetic target\n\
         another synthetic Test construct body\n\
         ---\n\
         @synthetic.source synthetic-target-a\n\
         plain synthetic Source Target body\n",
    );
    project.write(
        "b.synthetic",
        "@synthetic.test\n\
         @synthetic.id TEST-MIXED-SYN-B1\n\
         @synthetic.covers VO-MIXED\n\
         @synthetic.target synthetic-target-b\n\
         @synthetic.intent exercises the second synthetic target from another file\n\
         a third synthetic Test construct body\n\
         ---\n\
         @synthetic.source synthetic-target-b\n\
         another synthetic Source Target body\n",
    );
    write_records(&project.root, &["VO-MIXED"]);
    project
}

/// The `synthetic` adapter with everything it enumerates reversed: its
/// Tests, its Source Targets and, because both come back file by file, the
/// files (DS-1791, DS-1863).
struct ReversedSynthetic(SyntheticAdapter);

impl Adapter for ReversedSynthetic {
    fn descriptor(&self) -> AdapterDescriptor {
        self.0.descriptor()
    }

    fn as_source_discovery(&self) -> Option<&dyn SourceDiscoveryAdapter> {
        Some(self)
    }
}

impl SourceDiscoveryAdapter for ReversedSynthetic {
    fn id(&self) -> &'static str {
        SourceDiscoveryAdapter::id(&self.0)
    }

    fn discover(
        &self,
        root: &Path,
        fallback_package: &str,
        config: &AdapterScanConfig,
    ) -> Result<DiscoveryOutcome, DiscoveryError> {
        let mut outcome = self.0.discover(root, fallback_package, config)?;
        outcome.tests.reverse();
        outcome.missing_tests.reverse();
        outcome.sources.reverse();
        outcome.diagnostics.reverse();
        Ok(outcome)
    }
}

/// The acceptance-only registry: `rust-cargo` and `synthetic`. With
/// `reversed`, the adapters are registered in the opposite order and the
/// `synthetic` adapter returns its enumeration reversed.
pub fn acceptance_registry(reversed: bool) -> AdapterRegistry {
    let mut registry = AdapterRegistry::new();
    if reversed {
        registry
            .register(Box::new(ReversedSynthetic(SyntheticAdapter::new())))
            .expect("register synthetic");
        registry
            .register(Box::new(RustCargoAdapter::new()))
            .expect("register rust-cargo");
    } else {
        registry
            .register(Box::new(RustCargoAdapter::new()))
            .expect("register rust-cargo");
        registry
            .register(Box::new(SyntheticAdapter::new()))
            .expect("register synthetic");
    }
    registry
}

/// `vtest --project <root> <args>`, the shipped binary.
pub fn vtest(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vtest"))
        .arg("--project")
        .arg(root)
        .args(args)
        .output()
        .expect("run vtest")
}

pub fn exit(output: &Output) -> i32 {
    output.status.code().expect("exit code")
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("UTF-8 stdout")
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("UTF-8 stderr")
}

/// `vtest --project <root> --format json <args>`: exit code and envelope.
pub fn json_of(root: &Path, args: &[&str]) -> (i32, Value) {
    let mut all = vec!["--format", "json"];
    all.extend_from_slice(args);
    let output = vtest(root, &all);
    let envelope = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("non-JSON stdout ({error}): {}", stdout(&output)));
    (exit(&output), envelope)
}

pub fn keys(value: &Value) -> Vec<String> {
    value
        .as_object()
        .unwrap_or_else(|| panic!("not an object: {value}"))
        .keys()
        .cloned()
        .collect()
}

/// The `id` of each element of `data.tests`.
pub fn test_ids(data: &Value) -> Vec<String> {
    data["tests"]
        .as_array()
        .unwrap_or_else(|| panic!("no tests: {data}"))
        .iter()
        .map(|test| test["id"].as_str().expect("id").to_owned())
        .collect()
}

pub fn codes(envelope: &Value) -> Vec<String> {
    envelope["diagnostics"]
        .as_array()
        .expect("diagnostics")
        .iter()
        .map(|diagnostic| diagnostic["code"].as_str().expect("code").to_owned())
        .collect()
}

/// Every file below `root` with its bytes, and every directory.
pub fn tree(root: &Path) -> Vec<(String, Option<Vec<u8>>)> {
    fn visit(root: &Path, directory: &Path, entries: &mut Vec<(String, Option<Vec<u8>>)>) {
        for entry in fs::read_dir(directory).expect("read directory") {
            let path = entry.expect("entry").path();
            let relative = path
                .strip_prefix(root)
                .expect("below root")
                .to_string_lossy()
                .into_owned();
            if path.is_dir() {
                entries.push((relative, None));
                visit(root, &path, entries);
            } else {
                entries.push((relative, Some(fs::read(&path).expect("read file"))));
            }
        }
    }
    let mut entries = Vec::new();
    visit(root, root, &mut entries);
    entries.sort();
    entries
}

fn mcp_exchange(root: &Path, request: Value) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_vtest"))
        .arg("--project")
        .arg(root)
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn vtest mcp");
    {
        let mut stdin = child.stdin.take().expect("stdin");
        for message in [
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            request,
        ] {
            writeln!(stdin, "{message}").expect("write request");
        }
    }
    let output = child.wait_with_output().expect("vtest mcp exits");
    BufReader::new(output.stdout.as_slice())
        .lines()
        .map(|line| serde_json::from_str::<Value>(&line.expect("line")).expect("JSON response"))
        .find(|response| response["id"] == 2)
        .unwrap_or_else(|| panic!("{}", String::from_utf8_lossy(&output.stderr)))["result"]
        .clone()
}

/// The result of `tools/call` over the real `vtest mcp` stdio transport:
/// `isError` and `structuredContent` (the envelope).
pub fn mcp_call(root: &Path, name: &str, arguments: Value) -> Value {
    mcp_exchange(
        root,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {"name": name, "arguments": arguments}
        }),
    )
}

/// The input schema `tools/list` gives for `name` over the stdio transport,
/// or `None` when the tool is not listed.
pub fn mcp_input_schema(root: &Path, name: &str) -> Option<Value> {
    let listed = mcp_exchange(
        root,
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
    );
    listed["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .find(|tool| tool["name"] == name)
        .map(|tool| tool["inputSchema"].clone())
}

/// Commits the project, lockfile included, so `vtest run` sees the same
/// Execution State before and after the run.
pub fn commit(root: &Path) {
    let run = |program: &str, args: &[&str]| {
        let output = Command::new(program)
            .args(args)
            .current_dir(root)
            .output()
            .expect("run command");
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

/// `vtest run --test <test_id> --fast`: the Evidence ID it wrote. `--fast`
/// keeps the run on `cargo test`, so it does not depend on cargo-llvm-cov.
pub fn run_test(root: &Path, test_id: &str) -> String {
    let (code, ran) = json_of(root, &["run", "--test", test_id, "--fast"]);
    assert_eq!(code, 0, "run: {ran}");
    let ids = ran["data"]["evidence_ids"]
        .as_array()
        .expect("evidence_ids");
    assert_eq!(ids.len(), 1, "{ran}");
    ids[0].as_str().expect("Evidence ID").to_owned()
}

/// Bundles and submits one judgment of `test_id`; returns the decision ID.
pub fn submit_judgment(
    root: &Path,
    test_id: &str,
    kind: &str,
    decision: &str,
    supersedes: &[&str],
) -> String {
    let (code, bundle) = json_of(
        root,
        &["audit", "bundle", "--test", test_id, "--kind", kind],
    );
    assert_eq!(code, 0, "bundle: {bundle}");
    let mut submission = json!({
        "bundle_id": bundle["data"]["bundle_id"],
        "subject": test_id,
        "judgment_kind": kind,
        "decision": decision,
        "actor": {"kind": "human", "id": "reviewer"}
    });
    if !supersedes.is_empty() {
        submission["supersedes"] = json!(supersedes);
    }
    // Outside the project, so that the input is not part of it.
    let path = env::temp_dir().join(format!(
        "vtest-spec-acceptance-submission-{}-{}.json",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&path, submission.to_string()).expect("write submission");
    let (code, submitted) = json_of(
        root,
        &["audit", "submit", "--file", path.to_str().expect("path")],
    );
    fs::remove_file(&path).ok();
    assert_eq!(code, 0, "submit: {submitted}");
    submitted["data"]["decision_id"]
        .as_str()
        .unwrap_or_else(|| panic!("no decision_id: {submitted}"))
        .to_owned()
}
