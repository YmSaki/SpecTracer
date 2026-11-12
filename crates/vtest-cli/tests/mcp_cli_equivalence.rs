//! DS-1563 over the real transport (#178, #188): the same input gives the
//! same `ok` / `data` / `diagnostics` from the CLI and from `vtest mcp`
//! `tools/call`, and a scan error elsewhere in the project does not stop an
//! MCP operation before it applies its own checks.
//!
//! The second half of this file runs every MCP tool that has a CLI command
//! against that command, on accepted and rejected inputs. Both sides go
//! through the `vtest` binary, so the entry code of both crates is compared,
//! not only the shared operation behind them.
//!
//! Not compared here:
//! - `form_get` has no CLI command.
//! - `doc_get` takes `tree` / `roots`, which `doc show` does not have.
//! - Input-shape errors that only one surface can express (a missing
//!   required flag is rejected by clap before any JSON is produced).

use std::{
    fs,
    io::{BufRead, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

mod support;

use serde_json::{json, Value};
use vtest_model::{
    DerivesFrom, DocumentFile, DocumentId, NodeSource, RootNode, SentenceNode, VoId, VoRecord,
};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A project with ROOT-001 → R-001 and VO-X deriving from `vo_derives_from`.
fn fixture(name: &str, vo_derives_from: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "vtest-mcp-cli-equivalence-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    write_fixture(&root, vo_derives_from);
    root
}

/// Writes the project of [`fixture`] at `root`.
fn write_fixture(root: &Path, vo_derives_from: &str) {
    fs::create_dir_all(root.join("src")).unwrap();
    let layout = vtest_store::init_project(root, "mcp-cli-equivalence").unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"mcp_cli_equivalence\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), "pub fn target() {}\n").unwrap();
    write_records(&layout, vo_derives_from);
}

/// ROOT-001 → R-001 as document `fixture`, and VO-X deriving from
/// `vo_derives_from`, with fixed timestamps so their hashes do not depend on
/// when the fixture was written.
fn write_records(layout: &vtest_store::VerifyLayout, vo_derives_from: &str) {
    let source = |id: &str| NodeSource {
        doc: format!("{id}.md"),
        heading: "fixture".to_owned(),
        lines: [1, 1],
    };
    let file = DocumentFile {
        schema_version: "0.1".to_owned(),
        root: vec![RootNode {
            id: DocumentId::new("ROOT-001"),
            statement: "root".to_owned(),
            description: None,
            source: source("ROOT-001"),
        }],
        request: vec![SentenceNode {
            id: DocumentId::new("R-001"),
            statement: "request".to_owned(),
            description: None,
            derives_from: vec![DocumentId::new("ROOT-001")],
            cites: None,
            source: source("R-001"),
        }],
        require: Vec::new(),
        spec: Vec::new(),
        detailed_spec: Vec::new(),
        basic_design: Vec::new(),
        design: Vec::new(),
    };
    vtest_store::write_document_file(layout, "fixture", &file).unwrap();
    let now = "2026-11-07T00:00:00Z".to_owned();
    vtest_store::canonical::write_vo_record(
        layout,
        &VoRecord {
            id: VoId::new("VO-X"),
            parent: None,
            derives_from: vec![DerivesFrom {
                doc: DocumentId::new(vo_derives_from),
                anchor: None,
                note: None,
            }],
            claim: "claim".to_owned(),
            dimensions: Vec::new(),
            coverage_policy: None,
            combinations: Vec::new(),
            representative_cases: Vec::new(),
            created: now.clone(),
            updated: now,
        },
    )
    .unwrap();
}

/// A Test unrelated to the approval whose target does not resolve.
fn add_unresolved_target_test(root: &Path) {
    fs::write(
        root.join("src/lib.rs"),
        "pub fn target() {}\n\n#[cfg(test)]\nmod tests {\n    \
         /// @vtest.id TEST-X\n    /// @vtest.covers VO-X\n    \
         /// @vtest.target src/lib.rs::missing\n    /// @vtest.intent fixture\n    \
         #[test]\n    fn targets_nothing() {\n        super::target();\n    }\n}\n",
    )
    .unwrap();
}

/// The `vtest` binary without the outer `cargo test` / `cargo llvm-cov`
/// settings that would reach the `cargo` a fixture's `vtest run` starts and
/// change that fixture's Execution State between snapshots (E-EXEC-004).
fn vtest(root: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_vtest"));
    for variable in [
        "RUSTC_WRAPPER",
        "LLVM_PROFILE_FILE",
        "CARGO_LLVM_COV",
        "CARGO_LLVM_COV_TARGET_DIR",
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_INCREMENTAL",
        "CARGO_TARGET_DIR",
    ] {
        command.env_remove(variable);
    }
    command.arg("--project").arg(root);
    command
}

fn cli(root: &Path, args: &[&str]) -> Value {
    cli_with_exit(root, args).1
}

/// `vtest --format json <args>`: the exit code and the envelope on stdout.
fn cli_with_exit(root: &Path, args: &[&str]) -> (i32, Value) {
    let output = vtest(root)
        .args(["--format", "json"])
        .args(args)
        .output()
        .unwrap();
    let envelope = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "non-JSON output ({error}) for {args:?}: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (output.status.code().expect("exit code"), envelope)
}

/// One `tools/call` through the real `vtest mcp` stdio server; returns the
/// tool's `structuredContent` envelope.
fn mcp(root: &Path, name: &str, arguments: Value) -> Value {
    let mut child = vtest(root)
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut stdin = child.stdin.take().unwrap();
        for request in [
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {"name": name, "arguments": arguments}
            }),
        ] {
            writeln!(stdin, "{request}").unwrap();
        }
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let responses: Vec<Value> = output
        .stdout
        .lines()
        .map(|line| serde_json::from_str(&line.unwrap()).unwrap())
        .collect();
    assert_eq!(responses.len(), 2, "{responses:?}");
    responses[1]["result"]["structuredContent"].clone()
}

fn approval_records(root: &Path) -> usize {
    fs::read_dir(root.join(".verify/approvals")).map_or(0, |entries| {
        entries
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "yaml"))
            .count()
    })
}

fn codes(envelope: &Value) -> Vec<Value> {
    envelope["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|diagnostic| diagnostic["code"].clone())
        .collect()
}

/// Drops the fields that differ between two runs (record id, time).
fn stable_data(envelope: &Value) -> Value {
    let mut data = envelope["data"].clone();
    if let Some(object) = data.as_object_mut() {
        for volatile in ["id", "approved_at"] {
            object.remove(volatile);
        }
    }
    data
}

/// @vtest.id TEST-MCP-TRANSPORT-APPROVAL-MATCHES-CLI
/// @vtest.covers VO-MCP-TOOL-CLI-EQUIVALENCE
/// @vtest.target crates/vtest-mcp/src/lib.rs::tools_call_result
/// @vtest.intent 無関係なTestに未解決target(E-SCAN-004)がある状態と、承認対象VO自身のderives_fromが存在しないdocumentを指す状態のそれぞれで、CLIのapproval createと実際のvtest mcp stdioのtools/callのapproval_createが、同じok・data・診断コード・承認記録の作成有無になることを確認する
#[test]
fn approval_create_gives_the_same_result_over_cli_and_the_mcp_transport() {
    let cli_args = [
        "approval",
        "create",
        "--subject-type",
        "vo",
        "--subject-id",
        "VO-X",
        "--state",
        "approved",
        "--approver-kind",
        "agent",
        "--approver-id",
        "bot",
        "--model",
        "m",
    ];
    let mcp_args = json!({
        "subject": {"type": "vo", "id": "VO-X"},
        "state": "approved",
        "approver": {"kind": "agent", "id": "bot", "model": "m"}
    });
    // (case, VO-X's derives_from, add the unrelated E-SCAN-004, accepted)
    for (name, derives_from, unrelated_error, accepted) in [
        ("unrelated-scan-error", "R-001", true, true),
        ("dangling-subject", "R-999", false, false),
    ] {
        let prepare = |surface: &str| {
            let root = fixture(&format!("{name}-{surface}"), derives_from);
            if unrelated_error {
                add_unresolved_target_test(&root);
                let scan = cli(&root, &["scan"]);
                assert!(
                    codes(&scan).contains(&json!("E-SCAN-004")),
                    "fixture must start with E-SCAN-004: {scan}"
                );
            }
            root
        };
        let cli_root = prepare("cli");
        let via_cli = cli(&cli_root, &cli_args);
        let mcp_root = prepare("mcp");
        let via_mcp = mcp(&mcp_root, "approval_create", mcp_args.clone());

        assert_eq!(via_cli["ok"], json!(accepted), "{name} cli: {via_cli}");
        assert_eq!(via_mcp["ok"], via_cli["ok"], "{name} mcp: {via_mcp}");
        assert_eq!(codes(&via_mcp), codes(&via_cli), "{name}");
        assert_eq!(stable_data(&via_mcp), stable_data(&via_cli), "{name}");
        assert_eq!(approval_records(&cli_root), usize::from(accepted), "{name}");
        assert_eq!(
            approval_records(&mcp_root),
            approval_records(&cli_root),
            "{name}"
        );
        fs::remove_dir_all(cli_root).ok();
        fs::remove_dir_all(mcp_root).ok();
    }
}

/// @vtest.id TEST-MCP-TRANSPORT-SWIFT-BODYLESS-PARITY
/// @vtest.covers VO-ADAPTER-SWIFT-REGISTERS-WITHOUT-CORE-CHANGES
/// @vtest.target crates/vtest-adapter-lexical/src/swift.rs::Reader::declaration_end
/// @vtest.intent verifies that a valid bodyless Swift protocol requirement is discovered, bodyless declarations outside protocols fail closed, and the real CLI and MCP scan transports return identical envelopes
#[test]
fn swift_bodyless_requirements_match_cli_and_mcp() {
    let declare =
        "// @vtest.id TEST-X\n// @vtest.covers VO-X\n// @vtest.intent complete declaration\n";
    for (case, source, expected_tests, unreadable) in [
        (
            "protocol-requirement",
            format!("protocol P {{\n{declare}    func f() -> Int\n}}\n"),
            1,
            false,
        ),
        (
            "struct-missing-body",
            format!("struct S {{\n{declare}    func f() -> Int\n}}\n"),
            0,
            true,
        ),
        (
            "class-missing-body",
            format!("class C {{\n{declare}    func f() -> Int\n}}\n"),
            0,
            true,
        ),
        (
            "nested-missing-body",
            format!("func outer() {{\n{declare}    func f() -> Int\n}}\n"),
            0,
            true,
        ),
    ] {
        let project = support::fixture(
            "swift",
            &[("Sources/Bodyless.swift", source.as_str())],
            &["VO-X"],
        );
        let (scan, _) = project.scan_and_verify();
        assert_eq!(
            scan.tests.len(),
            expected_tests,
            "{case}: {:?}",
            scan.diagnostics
        );
        assert_eq!(
            scan.diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code.as_str() == "E-SCAN-001"),
            unreadable,
            "{case}: {:?}",
            scan.diagnostics
        );

        let via_cli = cli(project.path(), &["scan"]);
        let via_mcp = mcp(project.path(), "scan", json!({}));
        assert_eq!(via_cli["ok"], via_mcp["ok"], "{case}");
        assert_eq!(stable_data(&via_cli), stable_data(&via_mcp), "{case}");
        assert_eq!(via_cli["diagnostics"], via_mcp["diagnostics"], "{case}");
        assert_eq!(
            codes(&via_cli).contains(&json!("E-SCAN-001")),
            unreadable,
            "{case}: {via_cli}"
        );
    }
}

// ---------------------------------------------------------------------------
// Every MCP tool against its CLI command (#188)
// ---------------------------------------------------------------------------

const TOOL_FIXTURE_SOURCE: &str = "pub fn target() -> u32 {\n    1\n}\n\n\
     #[cfg(test)]\nmod tests {\n    \
     /// @vtest.id TEST-X\n    /// @vtest.covers VO-X\n    \
     /// @vtest.target src/lib.rs::target\n    /// @vtest.intent fixture\n    \
     #[test]\n    fn target_returns_one() {\n        assert_eq!(super::target(), 1);\n    }\n}\n";

/// A Cargo package initialized by `vtest init` (so the built-in Forms are
/// present) with the records of [`write_records`], one managed Test
/// `TEST-X` covering `VO-X` and targeting `src/lib.rs::target`, and an empty
/// `tests/calc.rs` for `test create`.
fn tool_fixture(root: &Path) {
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("tests")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"equivalence_fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
         [workspace]\n",
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), TOOL_FIXTURE_SOURCE).unwrap();
    fs::write(root.join("tests/calc.rs"), "").unwrap();
    let (exit, init) = cli_with_exit(root, &["init", "--name", "equivalence-fixture"]);
    assert_eq!(exit, 0, "init: {init}");
    write_records(&vtest_store::VerifyLayout::new(root), "R-001");
}

/// Where a case keeps the files the CLI reads its input from (`--answers`,
/// `--body-file`, `--file`), outside the project so they are not part of it.
fn inputs(root: &Path) -> PathBuf {
    let mut name = root.file_name().unwrap().to_owned();
    name.push("-inputs");
    root.with_file_name(name)
}

fn write_input(root: &Path, name: &str, text: &str) -> String {
    let path = inputs(root).join(name);
    fs::write(&path, text).unwrap();
    path.to_string_lossy().into_owned()
}

fn replace_in(root: &Path, relative: &str, from: &str, to: &str) {
    let path = root.join(relative);
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains(from), "{relative} does not contain {from:?}");
    fs::write(path, text.replace(from, to)).unwrap();
}

/// A `config.yaml` that cannot be parsed (E-CONFIG-001).
fn break_config(root: &Path) {
    fs::write(root.join(".verify/config.yaml"), "version: 2\nproject: [\n").unwrap();
}

/// Makes the config unreadable (not rejected): a directory where the file
/// should be, an I/O failure no diagnostic code is assigned to.
fn make_config_unreadable(root: &Path) {
    let config = root.join(".verify/config.yaml");
    fs::remove_file(&config).unwrap();
    fs::create_dir(&config).unwrap();
}

/// Adds a VO record the store rejects: a field outside the schema.
fn write_malformed_vo(root: &Path) {
    fs::write(
        root.join(".verify/vo/VO-BAD.yaml"),
        "id: VO-BAD\nnot_a_field: 1\n",
    )
    .unwrap();
}

/// Commits the fixture, lockfile included, so `vtest run` sees the same
/// Execution State before and after the run. The commit date is fixed so the
/// revision recorded in Evidence is the same for both surfaces.
fn commit(root: &Path) {
    let run = |program: &str, args: &[&str]| {
        let output = Command::new(program)
            .args(args)
            .current_dir(root)
            .env("GIT_AUTHOR_DATE", "2026-11-07T00:00:00Z")
            .env("GIT_COMMITTER_DATE", "2026-11-07T00:00:00Z")
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

fn is_ulid(token: &str) -> bool {
    token.len() == 26
        && token
            .bytes()
            .all(|byte| b"0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(&byte))
}

/// Length of an RFC 3339 UTC timestamp (`2026-11-07T00:00:00Z`, with
/// optional fractional seconds) at the start of `bytes`.
fn timestamp_length(bytes: &[u8]) -> Option<usize> {
    const SHAPE: &[u8] = b"dddd-dd-ddTdd:dd:dd";
    if bytes.len() <= SHAPE.len() {
        return None;
    }
    for (byte, shape) in bytes.iter().zip(SHAPE) {
        let matches = match shape {
            b'd' => byte.is_ascii_digit(),
            other => byte == other,
        };
        if !matches {
            return None;
        }
    }
    let mut length = SHAPE.len();
    if bytes[length] == b'.' {
        length += 1;
        while bytes.get(length).is_some_and(u8::is_ascii_digit) {
            length += 1;
        }
    }
    (bytes.get(length) == Some(&b'Z')).then_some(length + 1)
}

/// Replaces what differs between two runs of the same operation: record ids
/// (ULID) and timestamps. Hashes, paths and messages are left as they are.
fn mask_text(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut masked = String::with_capacity(text.len());
    let mut index = 0;
    while index < bytes.len() {
        if let Some(length) = timestamp_length(&bytes[index..]) {
            masked.push_str("<time>");
            index += length;
        } else if bytes[index].is_ascii_alphanumeric() {
            let length = bytes[index..]
                .iter()
                .take_while(|byte| byte.is_ascii_alphanumeric())
                .count();
            let token = &text[index..index + length];
            masked.push_str(if is_ulid(token) { "<id>" } else { token });
            index += length;
        } else {
            let character = text[index..].chars().next().unwrap();
            masked.push(character);
            index += character.len_utf8();
        }
    }
    masked
}

fn mask(value: &Value) -> Value {
    match value {
        Value::String(text) => Value::String(mask_text(text)),
        Value::Array(items) => Value::Array(items.iter().map(mask).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| (mask_text(key), mask(value)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Every file of the project after an operation, as (masked path, masked
/// text), sorted. Two records can share a masked path, so this is a list,
/// not a map. `.verify/cache/` is derived data (logs carry durations), so
/// only its file names are compared; `.git/` and `target/` are not project
/// state.
fn project_files(root: &Path) -> Vec<(String, String)> {
    fn visit(root: &Path, directory: &Path, files: &mut Vec<(String, String)>) {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if relative == ".git" || relative == "target" {
                continue;
            }
            if path.is_dir() {
                visit(root, &path, files);
            } else {
                let text = if relative.starts_with(".verify/cache/") {
                    String::new()
                } else {
                    mask_text(&String::from_utf8_lossy(&fs::read(&path).unwrap()))
                };
                files.push((mask_text(&relative), text));
            }
        }
    }
    let mut files = Vec::new();
    visit(root, root, &mut files);
    files.sort();
    files
}

/// The files of `files` that `other` does not have with the same text.
fn only_in<'a>(
    files: &'a [(String, String)],
    other: &[(String, String)],
) -> Vec<&'a (String, String)> {
    files.iter().filter(|file| !other.contains(file)).collect()
}

/// What both surfaces must return for one input: `ok`, the CLI exit code,
/// and the code of the first diagnostic of a rejected input.
#[derive(Clone, Copy)]
struct Expect {
    ok: bool,
    exit: i32,
    code: Option<&'static str>,
}

/// The operation was carried out.
const ACCEPTED: Expect = Expect {
    ok: true,
    exit: 0,
    code: None,
};

/// A verification result was produced and it is not OK (exit 1).
const NOT_OK: Expect = Expect {
    ok: false,
    exit: 1,
    code: None,
};

/// The tool itself failed: E-INTERNAL-001 with exit 3 (DS-1690).
const INTERNAL: Expect = Expect {
    ok: false,
    exit: 3,
    code: Some("E-INTERNAL-001"),
};

/// The input or the operation was rejected with `code` (exit 2).
const fn rejected(code: &'static str) -> Expect {
    Expect {
        ok: false,
        exit: 2,
        code: Some(code),
    }
}

/// Runs one input through `vtest --format json <cli args>` and through
/// `tools/call <tool>`, each on a fresh build of [`tool_fixture`] at the same
/// path, and requires the same envelope and the same project files
/// afterwards. `setup` adds state on top of the fixture and returns the
/// values the arguments depend on (ids it created, input file paths).
fn assert_same_with(
    name: &str,
    setup: &dyn Fn(&Path) -> Value,
    cli_args: &dyn Fn(&Value) -> Vec<String>,
    tool: &str,
    mcp_args: &dyn Fn(&Value) -> Value,
    expect: Expect,
) {
    let root = std::env::temp_dir().join(format!(
        "vtest-tool-equivalence-{}-{name}",
        std::process::id()
    ));
    let prepare = || {
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(inputs(&root));
        fs::create_dir_all(inputs(&root)).unwrap();
        tool_fixture(&root);
        setup(&root)
    };

    let context = prepare();
    let args = cli_args(&context);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let (exit, via_cli) = cli_with_exit(&root, &args);
    let files_after_cli = project_files(&root);

    let context = prepare();
    let via_mcp = mcp(&root, tool, mcp_args(&context));
    let files_after_mcp = project_files(&root);

    assert_eq!(exit, expect.exit, "{name}: cli exit code, {via_cli}");
    assert_eq!(via_cli["ok"], json!(expect.ok), "{name}: cli {via_cli}");
    if let Some(code) = expect.code {
        assert_eq!(codes(&via_cli).first(), Some(&json!(code)), "{name}");
    }
    assert_eq!(mask(&via_mcp), mask(&via_cli), "{name}: envelope");
    assert!(
        files_after_mcp == files_after_cli,
        "{name}: project files differ\nonly after the CLI: {:#?}\nonly after MCP: {:#?}",
        only_in(&files_after_cli, &files_after_mcp),
        only_in(&files_after_mcp, &files_after_cli)
    );

    fs::remove_dir_all(&root).ok();
    fs::remove_dir_all(inputs(&root)).ok();
}

fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| (*arg).to_owned()).collect()
}

/// [`assert_same_with`] for arguments that do not depend on the setup.
fn assert_same_after(
    name: &str,
    setup: &dyn Fn(&Path),
    cli_args: &[&str],
    tool: &str,
    mcp_args: Value,
    expect: Expect,
) {
    assert_same_with(
        name,
        &|root| {
            setup(root);
            Value::Null
        },
        &|_| strings(cli_args),
        tool,
        &|_| mcp_args.clone(),
        expect,
    );
}

/// [`assert_same_after`] on the unchanged fixture.
fn assert_same(name: &str, cli_args: &[&str], tool: &str, mcp_args: Value, expect: Expect) {
    assert_same_after(name, &|_| {}, cli_args, tool, mcp_args, expect);
}

const APPROVER_FLAGS: [&str; 6] = [
    "--approver-kind",
    "agent",
    "--approver-id",
    "bot",
    "--model",
    "m",
];

fn approver() -> Value {
    json!({"kind": "agent", "id": "bot", "model": "m"})
}

/// @vtest.id TEST-MCP-TRANSPORT-SCAN-RUN-MATCH-CLI
/// @vtest.covers VO-MCP-TOOL-CLI-EQUIVALENCE
/// @vtest.target crates/vtest-mcp/src/lib.rs::tools_call_result
/// @vtest.intent scanとrun_testsについて、成功、検証NG(E-SCAN-004)、Execution State変化(E-EXEC-004)、未知のTest・VO(E-OP-001)、読めないconfig(E-CONFIG-001)のそれぞれで、CLIのJSONと実際のvtest mcp stdioのtools/callが同じok・data・diagnosticsを返し、実行後のプロジェクトのファイルも同じになることを確認する
#[test]
fn scan_and_run_tests_match_their_cli_commands() {
    let unresolved_target = |root: &Path| {
        replace_in(
            root,
            "src/lib.rs",
            "src/lib.rs::target",
            "src/lib.rs::missing",
        )
    };

    assert_same("scan", &["scan"], "scan", json!({}), ACCEPTED);
    assert_same_after(
        "scan-unresolved-target",
        &unresolved_target,
        &["scan"],
        "scan",
        json!({}),
        Expect {
            code: Some("E-SCAN-004"),
            ..NOT_OK
        },
    );
    assert_same_after(
        "scan-config",
        &break_config,
        &["scan"],
        "scan",
        json!({}),
        rejected("E-CONFIG-001"),
    );

    // `--fast` keeps the run on `cargo test`, so the result does not depend
    // on whether cargo-llvm-cov is installed.
    assert_same_after(
        "run-pass",
        &commit,
        &["run", "--test", "TEST-X", "--fast"],
        "run_tests",
        json!({"test": ["TEST-X"], "fast": true}),
        ACCEPTED,
    );
    assert_same_after(
        "run-fail",
        &|root| {
            replace_in(root, "src/lib.rs", "target(), 1)", "target(), 2)");
            commit(root);
        },
        &["run", "--vo", "VO-X", "--fast"],
        "run_tests",
        json!({"vo": "VO-X", "fast": true}),
        ACCEPTED,
    );
    // Without a committed lockfile the run itself changes the project.
    assert_same(
        "run-execution-state-changed",
        &["run", "--all", "--fast"],
        "run_tests",
        json!({"all": true, "fast": true}),
        Expect {
            code: Some("E-EXEC-004"),
            ..NOT_OK
        },
    );
    assert_same(
        "run-unknown-test",
        &["run", "--test", "TEST-NOPE", "--fast"],
        "run_tests",
        json!({"test": ["TEST-NOPE"], "fast": true}),
        rejected("E-OP-001"),
    );
    assert_same(
        "run-unknown-vo",
        &["run", "--vo", "VO-NOPE", "--fast"],
        "run_tests",
        json!({"vo": "VO-NOPE", "fast": true}),
        rejected("E-OP-001"),
    );
    assert_same_after(
        "run-config",
        &break_config,
        &["run", "--fast"],
        "run_tests",
        json!({"fast": true}),
        rejected("E-CONFIG-001"),
    );
}

/// @vtest.id TEST-MCP-TRANSPORT-VERIFY-REPORT-MATCH-CLI
/// @vtest.covers VO-MCP-TOOL-CLI-EQUIVALENCE
/// @vtest.target crates/vtest-mcp/src/lib.rs::tools_call_result
/// @vtest.intent verifyとreportについて、既定のscope、検査軸とエンティティ軸の指定、summary、projection、未定義のgate(E-CONFIG-002)、不正な検査名・view・起点(E-OP-001)、読めないconfig(E-CONFIG-001)のそれぞれで、CLIのJSONと実際のvtest mcp stdioのtools/callが同じok・data・diagnosticsを返すことを確認する
#[test]
fn verify_and_report_match_their_cli_commands() {
    assert_same("verify", &["verify"], "verify", json!({}), NOT_OK);
    assert_same(
        "verify-summary",
        &["verify", "--summary"],
        "verify",
        json!({"summary": true}),
        NOT_OK,
    );
    assert_same(
        "verify-scope",
        &[
            "verify",
            "--items",
            "chain_integrity,orphan_detection",
            "--test",
            "TEST-X",
        ],
        "verify",
        json!({"items": ["chain_integrity", "orphan_detection"], "test": "TEST-X"}),
        NOT_OK,
    );
    assert_same(
        "verify-vo-outside-project",
        &["verify", "--vo", "VO-NOPE"],
        "verify",
        json!({"vo": "VO-NOPE"}),
        NOT_OK,
    );
    assert_same(
        "verify-gate",
        &["verify", "--gate", "nope"],
        "verify",
        json!({"gate": "nope"}),
        rejected("E-CONFIG-002"),
    );
    assert_same(
        "verify-item",
        &["verify", "--items", "bogus"],
        "verify",
        json!({"items": ["bogus"]}),
        rejected("E-OP-001"),
    );
    assert_same_after(
        "verify-config",
        &break_config,
        &["verify"],
        "verify",
        json!({}),
        rejected("E-CONFIG-001"),
    );

    assert_same("report", &["report"], "report", json!({}), NOT_OK);
    assert_same(
        "report-projection",
        &[
            "report",
            "--from",
            "VO-X",
            "--view",
            "tester",
            "--depth",
            "1",
            "--direction",
            "down",
        ],
        "report",
        json!({"from": "VO-X", "view": "tester", "depth": 1, "direction": "down"}),
        NOT_OK,
    );
    assert_same(
        "report-view",
        &["report", "--view", "bogus"],
        "report",
        json!({"view": "bogus"}),
        rejected("E-OP-001"),
    );
    assert_same(
        "report-from",
        &["report", "--from", "VO-NOPE"],
        "report",
        json!({"from": "VO-NOPE"}),
        rejected("E-OP-001"),
    );
    assert_same_after(
        "report-config",
        &break_config,
        &["report"],
        "report",
        json!({}),
        rejected("E-CONFIG-001"),
    );
}

/// Records an approval of VO-X and returns its id.
fn approve_vo_x(root: &Path) -> Value {
    let mut args = vec![
        "approval",
        "create",
        "--subject-type",
        "vo",
        "--subject-id",
        "VO-X",
        "--state",
        "approved",
    ];
    args.extend(APPROVER_FLAGS);
    let (exit, created) = cli_with_exit(root, &args);
    assert_eq!(exit, 0, "{created}");
    created["data"]["id"].clone()
}

/// @vtest.id TEST-MCP-TRANSPORT-APPROVAL-TOOLS-MATCH-CLI
/// @vtest.covers VO-MCP-TOOL-CLI-EQUIVALENCE
/// @vtest.target crates/vtest-mcp/src/lib.rs::tools_call_result
/// @vtest.intent approval_create・approval_withdraw・approval_get・vo_approveについて、受理、解決できないsubject(E-APPROVAL-001)、存在しない承認レコードの取り下げ(E-APPROVAL-002)、レコードのないsubjectの照会のそれぞれで、CLIのJSONと実際のvtest mcp stdioのtools/callが同じok・data・diagnosticsを返し、承認レコードの作成有無も同じになることを確認する
#[test]
fn approval_tools_match_their_cli_commands() {
    let create = |subject_type: &'static str, id: &'static str| {
        let mut args = vec![
            "approval",
            "create",
            "--subject-type",
            subject_type,
            "--subject-id",
            id,
            "--state",
            "approved",
        ];
        args.extend(APPROVER_FLAGS);
        args
    };
    let create_args = |subject_type: &str, id: &str| {
        json!({
            "subject": {"type": subject_type, "id": id},
            "state": "approved",
            "approver": approver()
        })
    };
    assert_same(
        "approval-create",
        &create("vo", "VO-X"),
        "approval_create",
        create_args("vo", "VO-X"),
        ACCEPTED,
    );
    assert_same(
        "approval-create-document",
        &create("document", "R-001"),
        "approval_create",
        create_args("document", "R-001"),
        ACCEPTED,
    );
    assert_same(
        "approval-create-unknown-subject",
        &create("vo", "VO-NOPE"),
        "approval_create",
        create_args("vo", "VO-NOPE"),
        rejected("E-APPROVAL-001"),
    );
    assert_same(
        "approval-create-judgment-subject",
        &create("judgment", "J-1"),
        "approval_create",
        create_args("judgment", "J-1"),
        rejected("E-APPROVAL-001"),
    );

    assert_same_with(
        "approval-withdraw",
        &approve_vo_x,
        &|id| {
            let mut args = strings(&["approval", "withdraw", id.as_str().unwrap()]);
            args.extend(strings(&APPROVER_FLAGS));
            args
        },
        "approval_withdraw",
        &|id| json!({"approval_id": id, "approver": approver()}),
        ACCEPTED,
    );
    let mut withdraw_unknown = vec!["approval", "withdraw", "APR-NOPE"];
    withdraw_unknown.extend(APPROVER_FLAGS);
    assert_same(
        "approval-withdraw-unknown",
        &withdraw_unknown,
        "approval_withdraw",
        json!({"approval_id": "APR-NOPE", "approver": approver()}),
        rejected("E-APPROVAL-002"),
    );

    let show = [
        "approval",
        "show",
        "--subject-type",
        "vo",
        "--subject-id",
        "VO-X",
    ];
    assert_same_after(
        "approval-show",
        &|root| {
            approve_vo_x(root);
        },
        &show,
        "approval_get",
        json!({"subject": {"type": "vo", "id": "VO-X"}}),
        ACCEPTED,
    );
    assert_same(
        "approval-show-no-record",
        &show,
        "approval_get",
        json!({"subject": {"type": "vo", "id": "VO-X"}}),
        ACCEPTED,
    );

    let approve = |id: &'static str| {
        let mut args = vec!["vo", "approve", id, "--state", "approved"];
        args.extend(APPROVER_FLAGS);
        args
    };
    assert_same(
        "vo-approve",
        &approve("VO-X"),
        "vo_approve",
        json!({"id": "VO-X", "state": "approved", "approver": approver()}),
        ACCEPTED,
    );
    assert_same(
        "vo-approve-unknown",
        &approve("VO-NOPE"),
        "vo_approve",
        json!({"id": "VO-NOPE", "state": "approved", "approver": approver()}),
        rejected("E-APPROVAL-001"),
    );
}

/// Writes `extra.json` in the project: one more document node, in `root[]`
/// (ROOT-002) or in `request[]` (R-002 deriving from ROOT-001).
fn write_extra_document(root: &Path, in_root_layer: bool) {
    let source =
        |id: &str| json!({"doc": format!("{id}.md"), "heading": "fixture", "lines": [1, 1]});
    let mut file = json!({
        "schema_version": "0.1",
        "root": [],
        "request": [],
        "require": [],
        "spec": [],
        "detailed_spec": [],
        "basic_design": [],
        "design": []
    });
    if in_root_layer {
        file["root"] =
            json!([{"id": "ROOT-002", "statement": "root two", "source": source("ROOT-002")}]);
    } else {
        file["request"] = json!([{
            "id": "R-002",
            "statement": "request two",
            "derives_from": ["ROOT-001"],
            "source": source("R-002")
        }]);
    }
    fs::write(root.join("extra.json"), file.to_string()).unwrap();
}

/// @vtest.id TEST-MCP-TRANSPORT-DOC-TOOLS-MATCH-CLI
/// @vtest.covers VO-MCP-TOOL-CLI-EQUIVALENCE
/// @vtest.target crates/vtest-mcp/src/lib.rs::tools_call_result
/// @vtest.intent doc_upsert・doc_get・doc_listについて、登録、derives_from指定、root指定、更新、重複ID・root層でない内容へのroot指定・更新時のroot指定・未登録IDの照会(E-OP-001)、壊れたdocument file(E-SCAN-010)のそれぞれで、CLIのJSONと実際のvtest mcp stdioのtools/callが同じok・data・diagnosticsを返し、登録後のdocument fileも同じになることを確認する
#[test]
fn doc_tools_match_their_cli_commands() {
    let request_document = |root: &Path| write_extra_document(root, false);
    let root_document = |root: &Path| write_extra_document(root, true);
    let add = ["doc", "add", "--id", "extra", "--path", "extra.json"];

    assert_same_after(
        "doc-add",
        &request_document,
        &add,
        "doc_upsert",
        json!({"id": "extra", "path": "extra.json"}),
        ACCEPTED,
    );
    assert_same_after(
        "doc-add-derives-from",
        &request_document,
        &[&add[..], &["--derives-from", "ROOT-001"]].concat(),
        "doc_upsert",
        json!({"id": "extra", "path": "extra.json", "derives_from": ["ROOT-001"]}),
        ACCEPTED,
    );
    assert_same_after(
        "doc-add-root",
        &root_document,
        &[&add[..], &["--root"]].concat(),
        "doc_upsert",
        json!({"id": "extra", "path": "extra.json", "root": true}),
        ACCEPTED,
    );
    assert_same_after(
        "doc-add-root-on-request-layer",
        &request_document,
        &[&add[..], &["--root"]].concat(),
        "doc_upsert",
        json!({"id": "extra", "path": "extra.json", "root": true}),
        rejected("E-OP-001"),
    );
    assert_same_after(
        "doc-add-duplicate-id",
        &request_document,
        &["doc", "add", "--id", "fixture", "--path", "extra.json"],
        "doc_upsert",
        json!({"id": "fixture", "path": "extra.json"}),
        rejected("E-OP-001"),
    );
    let update = [
        "doc",
        "add",
        "--id",
        "fixture",
        "--path",
        "extra.json",
        "--update",
    ];
    assert_same_after(
        "doc-update",
        &request_document,
        &update,
        "doc_upsert",
        json!({"id": "fixture", "path": "extra.json", "update": true}),
        ACCEPTED,
    );
    assert_same_after(
        "doc-update-with-root",
        &root_document,
        &[&update[..], &["--root"]].concat(),
        "doc_upsert",
        json!({"id": "fixture", "path": "extra.json", "update": true, "root": true}),
        rejected("E-OP-001"),
    );

    assert_same(
        "doc-show",
        &["doc", "show", "fixture"],
        "doc_get",
        json!({"id": "fixture"}),
        ACCEPTED,
    );
    assert_same(
        "doc-show-unknown",
        &["doc", "show", "nope"],
        "doc_get",
        json!({"id": "nope"}),
        rejected("E-OP-001"),
    );

    assert_same_after(
        "doc-list-unreadable-file",
        &|root| fs::write(root.join(".verify/doc/broken.json"), "{").unwrap(),
        &["doc", "list"],
        "doc_list",
        json!({}),
        rejected("E-SCAN-010"),
    );

    // `--tree` / `--roots` choose what the text output shows; the JSON carries
    // every key either way, and `doc_list` returns the same (#207).
    for (name, flags, arguments) in [
        ("doc-list", &[][..], json!({})),
        ("doc-list-tree", &["--tree"][..], json!({"tree": true})),
        ("doc-list-roots", &["--roots"][..], json!({"roots": true})),
    ] {
        assert_same(
            name,
            &[&["doc", "list"], flags].concat(),
            "doc_list",
            arguments,
            ACCEPTED,
        );
    }
}

fn unit_test_answers() -> Value {
    json!({
        "target": "src/lib.rs::target",
        "covers": ["VO-X"],
        "behavior": "target returns one",
        "test_kind": "normal",
        "input": "nothing",
        "expect": "one",
        "fn_name": "target_is_one",
        "file": "tests/calc.rs"
    })
}

/// The answers file `vtest test create --answers` reads, for the same
/// values `test_create` takes as JSON.
fn write_answers(root: &Path, form: &str, answers: &Value) -> String {
    let mut text = format!("form: {form}\nanswers:\n");
    for (name, value) in answers.as_object().unwrap() {
        match value {
            Value::Array(items) => {
                text.push_str(&format!("  {name}:\n"));
                for item in items {
                    text.push_str(&format!("    - '{}'\n", item.as_str().unwrap()));
                }
            }
            other => text.push_str(&format!("  {name}: '{}'\n", other.as_str().unwrap())),
        }
    }
    write_input(root, "answers.yaml", &text)
}

/// Creates the Test of [`unit_test_answers`] and returns its id.
fn create_unit_test(root: &Path) -> String {
    let answers = write_answers(root, "rust-unit-function", &unit_test_answers());
    let (exit, created) = cli_with_exit(
        root,
        &[
            "test",
            "create",
            "--form",
            "rust-unit-function",
            "--answers",
            &answers,
        ],
    );
    assert_eq!(exit, 0, "{created}");
    created["data"]["test_id"].as_str().unwrap().to_owned()
}

/// @vtest.id TEST-MCP-TRANSPORT-TEST-TOOLS-MATCH-CLI
/// @vtest.covers VO-MCP-TOOL-CLI-EQUIVALENCE
/// @vtest.target crates/vtest-mcp/src/lib.rs::tools_call_result
/// @vtest.intent test_createとtest_editについて、dry-run、挿入、ID指定、set・answers・bodyによる編集、未知のForm・解決できないsymbol・未知のfield(E-OP-001)、未知のTest(E-OP-002)、読めないconfig(E-CONFIG-001)のそれぞれで、CLIのJSONと実際のvtest mcp stdioのtools/callが同じok・data・diagnosticsを返し、編集後のソースファイルも同じになることを確認する
#[test]
fn test_tools_match_their_cli_commands() {
    let create_case = |name: &str,
                       form: &'static str,
                       answers: Value,
                       extra_flags: &'static [&'static str],
                       extra_arguments: Value,
                       setup: &dyn Fn(&Path),
                       expect: Expect| {
        assert_same_with(
            name,
            &|root| {
                setup(root);
                json!(write_answers(root, form, &answers))
            },
            &|path| {
                let mut args = strings(&["test", "create", "--form", form, "--answers"]);
                args.push(path.as_str().unwrap().to_owned());
                args.extend(strings(extra_flags));
                args
            },
            "test_create",
            &|_| {
                let mut arguments = json!({"form": form, "answers": answers});
                for (key, value) in extra_arguments.as_object().unwrap() {
                    arguments[key] = value.clone();
                }
                arguments
            },
            expect,
        );
    };
    let unchanged = |_: &Path| {};
    let mut unresolved = unit_test_answers();
    unresolved["target"] = json!("src/lib.rs::missing");

    create_case(
        "test-create-dry-run",
        "rust-unit-function",
        unit_test_answers(),
        &["--dry-run"],
        json!({"dry_run": true}),
        &unchanged,
        ACCEPTED,
    );
    create_case(
        "test-create",
        "rust-unit-function",
        unit_test_answers(),
        &[],
        json!({}),
        &unchanged,
        ACCEPTED,
    );
    create_case(
        "test-create-id",
        "rust-unit-function",
        unit_test_answers(),
        &["--id", "TEST-NEW"],
        json!({"id": "TEST-NEW"}),
        &unchanged,
        ACCEPTED,
    );
    create_case(
        "test-create-unknown-form",
        "nope",
        unit_test_answers(),
        &[],
        json!({}),
        &unchanged,
        rejected("E-OP-001"),
    );
    create_case(
        "test-create-unresolved-symbol",
        "rust-unit-function",
        unresolved,
        &[],
        json!({}),
        &unchanged,
        rejected("E-OP-001"),
    );
    create_case(
        "test-create-config",
        "rust-unit-function",
        unit_test_answers(),
        &[],
        json!({}),
        &break_config,
        rejected("E-CONFIG-001"),
    );

    let edit_case =
        |name: &str, flags: &'static [&'static str], arguments: Value, expect: Expect| {
            assert_same_with(
                name,
                &|root| json!(create_unit_test(root)),
                &|id| {
                    let mut args = strings(&["test", "edit", id.as_str().unwrap()]);
                    args.extend(strings(flags));
                    args
                },
                "test_edit",
                &|id| {
                    let mut arguments = arguments.clone();
                    arguments["id"] = id.clone();
                    arguments
                },
                expect,
            );
        };
    edit_case(
        "test-edit-set",
        &["--set", "behavior=changed"],
        json!({"set": {"behavior": "changed"}}),
        ACCEPTED,
    );
    edit_case(
        "test-edit-set-dry-run",
        &["--set", "behavior=changed", "--dry-run"],
        json!({"set": {"behavior": "changed"}, "dry_run": true}),
        ACCEPTED,
    );
    edit_case(
        "test-edit-unknown-field",
        &["--set", "nope=x"],
        json!({"set": {"nope": "x"}}),
        rejected("E-OP-001"),
    );

    let mut edited = unit_test_answers();
    edited["behavior"] = json!("edited");
    edited["input"] = json!("still nothing");
    assert_same_with(
        "test-edit-answers",
        &|root| {
            let id = create_unit_test(root);
            json!({"id": id, "answers": write_answers(root, "rust-unit-function", &edited)})
        },
        &|context| {
            strings(&[
                "test",
                "edit",
                context["id"].as_str().unwrap(),
                "--answers",
                context["answers"].as_str().unwrap(),
            ])
        },
        "test_edit",
        &|context| {
            json!({
                "id": context["id"],
                "answers": {"form": "rust-unit-function", "answers": edited}
            })
        },
        ACCEPTED,
    );
    let body = "    assert_eq!(equivalence_fixture::target(), 1);\n";
    assert_same_with(
        "test-edit-body",
        &|root| {
            let id = create_unit_test(root);
            json!({"id": id, "body": write_input(root, "body.rs", body)})
        },
        &|context| {
            strings(&[
                "test",
                "edit",
                context["id"].as_str().unwrap(),
                "--body-file",
                context["body"].as_str().unwrap(),
            ])
        },
        "test_edit",
        &|context| json!({"id": context["id"], "body": body}),
        ACCEPTED,
    );
    assert_same(
        "test-edit-unknown-test",
        &["test", "edit", "TEST-NOPE", "--set", "behavior=x"],
        "test_edit",
        json!({"id": "TEST-NOPE", "set": {"behavior": "x"}}),
        rejected("E-OP-002"),
    );
}

/// Builds a `test-semantic` bundle for TEST-X and writes a submission for
/// it, with `change` applied to the submission; returns the submission and
/// the path of its file.
fn bundle_and_submission(root: &Path, change: &dyn Fn(&mut Value)) -> Value {
    let (exit, bundle) = cli_with_exit(
        root,
        &[
            "audit",
            "bundle",
            "--test",
            "TEST-X",
            "--kind",
            "test-semantic",
        ],
    );
    assert_eq!(exit, 0, "{bundle}");
    let mut submission = json!({
        "bundle_id": bundle["data"]["bundle_id"],
        "subject": "TEST-X",
        "judgment_kind": "test-semantic",
        "decision": "accepted",
        "actor": {"kind": "agent", "id": "bot", "model": "m"}
    });
    change(&mut submission);
    let file = write_input(root, "submission.json", &submission.to_string());
    json!({"submission": submission, "file": file})
}

/// @vtest.id TEST-MCP-TRANSPORT-AUDIT-TOOLS-MATCH-CLI
/// @vtest.covers VO-MCP-TOOL-CLI-EQUIVALENCE
/// @vtest.target crates/vtest-mcp/src/lib.rs::tools_call_result
/// @vtest.intent audit_static・audit_bundle・audit_submitについて、全Testと単一Testの静的監査、Test・VOのbundle生成、判定の受理、対象未指定・未知のTest・不正なkind(E-OP-001)、存在しないbundle(E-AUDIT-001)、bundle生成後に変わったsubject(E-AUDIT-002)、bundleと合わないsubject(E-AUDIT-003)、不正なdecision(E-AUDIT-004)、スキーマ違反のVOレコード(E-SCAN-010)、読めないconfig(E-CONFIG-001)のそれぞれで、CLIのJSONと実際のvtest mcp stdioのtools/callが同じok・data・diagnosticsを返し、判定レコードの作成有無も同じになることを確認する
#[test]
fn audit_tools_match_their_cli_commands() {
    assert_same(
        "audit-static-all",
        &["audit", "static", "--all"],
        "audit_static",
        json!({"all": true}),
        ACCEPTED,
    );
    assert_same(
        "audit-static-test",
        &["audit", "static", "--test", "TEST-X"],
        "audit_static",
        json!({"test": "TEST-X"}),
        ACCEPTED,
    );
    assert_same(
        "audit-static-no-target",
        &["audit", "static"],
        "audit_static",
        json!({}),
        rejected("E-OP-001"),
    );
    assert_same(
        "audit-static-unknown-test",
        &["audit", "static", "--test", "TEST-NOPE"],
        "audit_static",
        json!({"test": "TEST-NOPE"}),
        rejected("E-OP-001"),
    );
    assert_same_after(
        "audit-static-config",
        &break_config,
        &["audit", "static", "--all"],
        "audit_static",
        json!({"all": true}),
        rejected("E-CONFIG-001"),
    );

    assert_same(
        "audit-bundle-test",
        &[
            "audit",
            "bundle",
            "--test",
            "TEST-X",
            "--kind",
            "test-semantic",
        ],
        "audit_bundle",
        json!({"test": "TEST-X", "kind": "test-semantic"}),
        ACCEPTED,
    );
    assert_same(
        "audit-bundle-default-kind",
        &["audit", "bundle", "--test", "TEST-X"],
        "audit_bundle",
        json!({"test": "TEST-X"}),
        ACCEPTED,
    );
    assert_same(
        "audit-bundle-vo",
        &["audit", "bundle", "--vo", "VO-X", "--kind", "case-coverage"],
        "audit_bundle",
        json!({"vo": "VO-X", "kind": "case-coverage"}),
        ACCEPTED,
    );
    assert_same_after(
        "audit-bundle-malformed-vo",
        &write_malformed_vo,
        &[
            "audit",
            "bundle",
            "--vo",
            "VO-BAD",
            "--kind",
            "case-coverage",
        ],
        "audit_bundle",
        json!({"vo": "VO-BAD", "kind": "case-coverage"}),
        rejected("E-SCAN-010"),
    );
    assert_same(
        "audit-bundle-unknown-test",
        &["audit", "bundle", "--test", "TEST-NOPE"],
        "audit_bundle",
        json!({"test": "TEST-NOPE"}),
        rejected("E-OP-001"),
    );
    assert_same(
        "audit-bundle-kind",
        &["audit", "bundle", "--test", "TEST-X", "--kind", "bogus"],
        "audit_bundle",
        json!({"test": "TEST-X", "kind": "bogus"}),
        rejected("E-OP-001"),
    );
    assert_same_after(
        "audit-bundle-config",
        &break_config,
        &["audit", "bundle", "--test", "TEST-X"],
        "audit_bundle",
        json!({"test": "TEST-X"}),
        rejected("E-CONFIG-001"),
    );

    let submit_case = |name: &str, setup: &dyn Fn(&Path) -> Value, expect: Expect| {
        assert_same_with(
            name,
            setup,
            &|context| {
                strings(&[
                    "audit",
                    "submit",
                    "--file",
                    context["file"].as_str().unwrap(),
                ])
            },
            "audit_submit",
            &|context| context["submission"].clone(),
            expect,
        );
    };
    submit_case(
        "audit-submit",
        &|root| bundle_and_submission(root, &|_| {}),
        ACCEPTED,
    );
    submit_case(
        "audit-submit-unknown-bundle",
        &|root| {
            bundle_and_submission(root, &|submission| {
                submission["bundle_id"] = json!("01M717WSCR6HW7WP0000SWJ999");
            })
        },
        rejected("E-AUDIT-001"),
    );
    submit_case(
        "audit-submit-stale-bundle",
        &|root| {
            let context = bundle_and_submission(root, &|_| {});
            replace_in(root, "src/lib.rs", "    1\n", "    2 - 1\n");
            context
        },
        rejected("E-AUDIT-002"),
    );
    submit_case(
        "audit-submit-other-subject",
        &|root| {
            bundle_and_submission(root, &|submission| {
                submission["subject"] = json!("TEST-OTHER");
            })
        },
        rejected("E-AUDIT-003"),
    );
    submit_case(
        "audit-submit-decision",
        &|root| {
            bundle_and_submission(root, &|submission| {
                submission["decision"] = json!("PASS");
            })
        },
        rejected("E-AUDIT-004"),
    );
}

/// @vtest.id TEST-MCP-TRANSPORT-VO-TOOLS-MATCH-CLI
/// @vtest.covers VO-MCP-TOOL-CLI-EQUIVALENCE
/// @vtest.target crates/vtest-mcp/src/lib.rs::tools_call_result
/// @vtest.intent vo_list・vo_get・vo_upsert・vo_expandについて、一覧と絞り込み、照会、追加、編集、dimensionsの展開とdry-run、不正なstatus・未知のVO・不正なpolicy・dimensionsのないVOの展開(E-OP-001)、解釈できないconfig(E-CONFIG-001)、I/Oとして読めないconfig(E-INTERNAL-001)のそれぞれで、CLIのJSONと実際のvtest mcp stdioのtools/callが同じok・data・diagnosticsを返し、VOレコードも同じになることを確認する
#[test]
fn vo_tools_match_their_cli_commands() {
    assert_same("vo-list", &["vo", "list"], "vo_list", json!({}), ACCEPTED);
    assert_same(
        "vo-list-status",
        &["vo", "list", "--status", "draft"],
        "vo_list",
        json!({"status": "draft"}),
        ACCEPTED,
    );
    assert_same(
        "vo-list-doc",
        &["vo", "list", "--doc", "R-001"],
        "vo_list",
        json!({"doc": "R-001"}),
        ACCEPTED,
    );
    assert_same(
        "vo-list-unknown-status",
        &["vo", "list", "--status", "bogus"],
        "vo_list",
        json!({"status": "bogus"}),
        rejected("E-OP-001"),
    );

    assert_same(
        "vo-show",
        &["vo", "show", "VO-X"],
        "vo_get",
        json!({"id": "VO-X"}),
        ACCEPTED,
    );
    assert_same(
        "vo-show-unknown",
        &["vo", "show", "VO-NOPE"],
        "vo_get",
        json!({"id": "VO-NOPE"}),
        rejected("E-OP-001"),
    );
    assert_same_after(
        "vo-show-config",
        &break_config,
        &["vo", "show", "VO-X"],
        "vo_get",
        json!({"id": "VO-X"}),
        rejected("E-CONFIG-001"),
    );
    assert_same_after(
        "vo-show-unreadable-config",
        &make_config_unreadable,
        &["vo", "show", "VO-X"],
        "vo_get",
        json!({"id": "VO-X"}),
        INTERNAL,
    );

    assert_same(
        "vo-add",
        &[
            "vo",
            "add",
            "--id",
            "VO-Y",
            "--claim",
            "c",
            "--derives-from",
            "R-001",
            "--anchor",
            "a",
            "--note",
            "n",
            "--parent",
            "VO-X",
            "--dimension",
            "size=small,large",
            "--policy",
            "independent-axes",
        ],
        "vo_upsert",
        json!({
            "id": "VO-Y",
            "claim": "c",
            "derives_from": [{"doc": "R-001", "anchor": "a", "note": "n"}],
            "parent": "VO-X",
            "dimensions": [{"name": "size", "partitions": ["small", "large"]}],
            "coverage_policy": "independent-axes"
        }),
        ACCEPTED,
    );
    assert_same(
        "vo-edit",
        &["vo", "edit", "VO-X", "--claim", "new"],
        "vo_upsert",
        json!({"id": "VO-X", "claim": "new", "derives_from": [{"doc": "R-001"}]}),
        ACCEPTED,
    );
    assert_same(
        "vo-add-unknown-policy",
        &[
            "vo",
            "add",
            "--id",
            "VO-Y",
            "--claim",
            "c",
            "--derives-from",
            "R-001",
            "--policy",
            "bogus",
        ],
        "vo_upsert",
        json!({
            "id": "VO-Y",
            "claim": "c",
            "derives_from": [{"doc": "R-001"}],
            "coverage_policy": "bogus"
        }),
        rejected("E-OP-001"),
    );

    let with_dimensions = |root: &Path| {
        let (exit, edited) = cli_with_exit(
            root,
            &[
                "vo",
                "edit",
                "VO-X",
                "--dimension",
                "size=small,large",
                "--policy",
                "independent-axes",
            ],
        );
        assert_eq!(exit, 0, "{edited}");
    };
    assert_same_after(
        "vo-expand-dry-run",
        &with_dimensions,
        &["vo", "expand", "VO-X", "--dry-run"],
        "vo_expand",
        json!({"id": "VO-X", "dry_run": true}),
        ACCEPTED,
    );
    assert_same_after(
        "vo-expand",
        &with_dimensions,
        &["vo", "expand", "VO-X"],
        "vo_expand",
        json!({"id": "VO-X"}),
        ACCEPTED,
    );
    assert_same(
        "vo-expand-no-dimensions",
        &["vo", "expand", "VO-X"],
        "vo_expand",
        json!({"id": "VO-X"}),
        rejected("E-OP-001"),
    );
    assert_same(
        "vo-expand-unknown",
        &["vo", "expand", "VO-NOPE"],
        "vo_expand",
        json!({"id": "VO-NOPE"}),
        rejected("E-OP-001"),
    );
}
