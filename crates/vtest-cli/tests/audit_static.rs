//! Acceptance coverage for `vtest audit static` (SPEC-297/SPEC-387,
//! DS-1208/DS-1209/DS-1227).

use std::{
    collections::BTreeSet,
    fs,
    io::{BufRead, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

use serde_json::{json, Value};

static NEXT: AtomicU64 = AtomicU64::new(0);

const TESTS: &str = "/// @vtest.id TEST-GOOD\n\
/// @vtest.covers VO-X\n\
/// @vtest.target src/lib.rs::add\n\
/// @vtest.target src/lib.rs::sub\n\
/// @vtest.intent adds\n\
#[test]\n\
fn good() {\n    assert_eq!(audit_fixture::add(2, 3), 5);\n    audit_fixture::sub(1, 1);\n}\n\
\n\
/// @vtest.id TEST-EMPTY\n\
/// @vtest.covers VO-X\n\
/// @vtest.target src/lib.rs::add\n\
/// @vtest.intent nothing\n\
#[test]\n\
fn empty() {}\n";

fn fixture(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "vtest-audit-static-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("tests")).unwrap();
    vtest_store::init_project(&root, "audit-static").unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"audit-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
    )
    .unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\npub fn sub(a: i32, b: i32) -> i32 { a - b }\n",
    )
    .unwrap();
    fs::write(root.join("tests/audit.rs"), TESTS).unwrap();
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
    let envelope = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&output.stdout)));
    (output.status.code().unwrap_or(-1), envelope)
}

/// Every file under `.verify/` except the rebuildable `cache/`.
fn canonical_files(root: &Path) -> BTreeSet<PathBuf> {
    fn walk(dir: &Path, out: &mut BTreeSet<PathBuf>) {
        for entry in fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name == "cache") {
                    continue;
                }
                walk(&path, out);
            } else {
                out.insert(path);
            }
        }
    }
    let mut out = BTreeSet::new();
    walk(&root.join(".verify"), &mut out);
    out
}

fn verdict<'a>(test: &'a Value, rule: &str, target: Option<&str>) -> &'a str {
    test["rules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["rule"] == rule && entry["target"].as_str() == target)
        .and_then(|entry| entry["verdict"].as_str())
        .unwrap_or_else(|| panic!("no {rule} {target:?} verdict in {test}"))
}

/// @vtest.id TEST-AUDIT-STATIC-RULE-VERDICTS
/// @vtest.covers VO-AUDIT-STATIC-RULE-VERDICTS
/// @vtest.target crates/vtest-cli/src/ops/audit.rs::audit_static
/// @vtest.intent audit static --all reports per-rule FAIL / UNKNOWN / NO_VIOLATION verdicts with the Test's evidence span and writes the same data to cache/
#[test]
fn audit_static_reports_rule_verdicts_with_spans() {
    let root = fixture("verdicts");
    let (code, envelope) = vtest(&root, &["audit", "static", "--all"]);
    assert_eq!(code, 0, "{envelope}");
    let tests = envelope["data"]["tests"].as_array().unwrap();
    let good = tests
        .iter()
        .find(|test| test["test"] == "TEST-GOOD")
        .unwrap();
    let empty = tests
        .iter()
        .find(|test| test["test"] == "TEST-EMPTY")
        .unwrap();
    assert_eq!(verdict(empty, "DA-005", None), "FAIL");
    assert_eq!(verdict(good, "DA-005", None), "NO_VIOLATION");
    assert_eq!(verdict(good, "DA-006", None), "NO_VIOLATION");
    let span = &good["rules"][0]["span"];
    assert_eq!(span["path"], "tests/audit.rs");
    assert!(span["byte_end"].as_u64().unwrap() > span["byte_start"].as_u64().unwrap());

    let cache: Value = serde_json::from_str(
        &fs::read_to_string(root.join(".verify/cache/audit-static.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        cache, envelope["data"],
        "stdout and cache/ carry the same result"
    );
}

/// @vtest.id TEST-AUDIT-STATIC-PER-TARGET
/// @vtest.covers VO-AUDIT-STATIC-TARGET-SCOPED-VERDICTS
/// @vtest.target crates/vtest-cli/src/ops/audit.rs::audit_static
/// @vtest.intent DA-002 and DA-003 are reported once per declared target; DA-003 distinguishes a verified target call from an unverified one and DA-002 stays UNKNOWN at the analysis boundary
#[test]
fn target_scoped_rules_are_reported_per_target() {
    let root = fixture("per-target");
    let (code, envelope) = vtest(&root, &["audit", "static", "--test", "TEST-GOOD"]);
    assert_eq!(code, 0, "{envelope}");
    let good = &envelope["data"]["tests"][0];
    assert_eq!(envelope["data"]["tests"].as_array().unwrap().len(), 1);
    assert_eq!(
        verdict(good, "DA-003", Some("src/lib.rs::add")),
        "NO_VIOLATION"
    );
    assert_ne!(
        verdict(good, "DA-003", Some("src/lib.rs::sub")),
        "NO_VIOLATION"
    );
    for target in ["src/lib.rs::add", "src/lib.rs::sub"] {
        assert_eq!(verdict(good, "DA-002", Some(target)), "UNKNOWN");
    }
}

/// @vtest.id TEST-AUDIT-STATIC-NO-CANONICAL-RECORD
/// @vtest.covers VO-AUDIT-STATIC-NO-CANONICAL-RECORD
/// @vtest.target crates/vtest-cli/src/ops/audit.rs::audit_static
/// @vtest.intent audit static creates no canonical record and returns no audit record id; selector misuse is E-OP-001 / exit 2
#[test]
fn audit_static_writes_no_canonical_record() {
    let root = fixture("no-record");
    let before = canonical_files(&root);
    let (code, envelope) = vtest(&root, &["audit", "static", "--all"]);
    assert_eq!(code, 0);
    assert_eq!(canonical_files(&root), before, "only cache/ may change");
    assert!(envelope["data"].get("id").is_none());
    for test in envelope["data"]["tests"].as_array().unwrap() {
        assert!(test.get("id").is_none());
    }

    for args in [
        vec!["audit", "static"],
        vec!["audit", "static", "--test", "TEST-NONE"],
    ] {
        let (code, envelope) = vtest(&root, &args);
        assert_eq!(code, 2, "{args:?}");
        assert_eq!(envelope["diagnostics"][0]["code"], "E-OP-001");
    }
}

/// Two Test constructs declaring the same ID: `tests/a.rs` passes every rule,
/// `tests/b.rs` is empty (DA-005 `FAIL`). Auditing either one alone would hide
/// the other's result.
fn colliding_fixture(name: &str) -> PathBuf {
    let root = fixture(name);
    fs::remove_file(root.join("tests/audit.rs")).unwrap();
    fs::write(
        root.join("tests/a.rs"),
        "/// @vtest.id TEST-A\n/// @vtest.covers VO-X\n/// @vtest.target src/lib.rs::add\n\
         /// @vtest.intent adds\n#[test]\nfn a() {\n    assert_eq!(audit_fixture::add(2, 3), 5);\n}\n",
    )
    .unwrap();
    fs::write(
        root.join("tests/b.rs"),
        "/// @vtest.id TEST-A\n/// @vtest.covers VO-X\n/// @vtest.target src/lib.rs::add\n\
         /// @vtest.intent nothing\n#[test]\nfn b() {}\n",
    )
    .unwrap();
    root
}

/// @vtest.id TEST-AUDIT-STATIC-COLLIDING-TEST-ID
/// @vtest.covers VO-AUDIT-STATIC-COLLIDING-TEST-ID-E-SCAN-002
/// @vtest.target crates/vtest-cli/src/ops/audit.rs::audit_static
/// @vtest.intent a Test ID declared by two constructs (one clean, one empty) makes audit static --test and --all fail with E-SCAN-002 / exit 2, audit neither construct, and write no cache
#[test]
fn a_colliding_test_id_is_rejected_without_a_cache() {
    for args in [
        &["audit", "static", "--test", "TEST-A"][..],
        &["audit", "static", "--all"][..],
    ] {
        let root = colliding_fixture("colliding");
        let (code, envelope) = vtest(&root, args);
        assert_eq!(code, 2, "{args:?}: {envelope}");
        assert_eq!(envelope["ok"], false, "{args:?}: {envelope}");
        assert_eq!(envelope["data"], Value::Null, "{args:?}: {envelope}");
        assert_eq!(
            envelope["diagnostics"][0]["code"], "E-SCAN-002",
            "{args:?}: {envelope}"
        );
        let message = envelope["diagnostics"][0]["message"].as_str().unwrap();
        assert!(message.contains("TEST-A"), "{message}");
        assert!(
            !root.join(".verify/cache/audit-static.json").exists(),
            "{args:?}: no derived cache for a rejected operation"
        );
    }
}

/// One `tools/call` through the real `vtest mcp` stdio server; returns the
/// tool's `structuredContent` envelope.
fn mcp(root: &Path, name: &str, arguments: Value) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_vtest"))
        .arg("--project")
        .arg(root)
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

/// Runs the same audit over the CLI and over `tools/call`, each on a clean
/// cache, and asserts the same `ok` / `data` / `diagnostics` and the same
/// cache outcome. Returns the CLI envelope.
fn assert_cli_and_mcp_agree(root: &Path, cli_args: &[&str], arguments: Value) -> Value {
    let cache = root.join(".verify/cache/audit-static.json");
    let _ = fs::remove_file(&cache);
    let (_, via_cli) = vtest(root, cli_args);
    let cli_cache = fs::read_to_string(&cache).ok();
    let _ = fs::remove_file(&cache);
    let via_mcp = mcp(root, "audit_static", arguments);
    let mcp_cache = fs::read_to_string(&cache).ok();
    assert_eq!(
        via_cli["ok"], via_mcp["ok"],
        "{cli_args:?}\nCLI {via_cli}\nMCP {via_mcp}"
    );
    assert_eq!(via_cli["data"], via_mcp["data"], "{cli_args:?}");
    assert_eq!(
        via_cli["diagnostics"], via_mcp["diagnostics"],
        "{cli_args:?}"
    );
    assert_eq!(cli_cache, mcp_cache, "{cli_args:?}: same cache outcome");
    if via_cli["ok"] == true {
        assert!(
            cli_cache.is_some(),
            "{cli_args:?}: success writes the cache"
        );
    } else {
        assert!(
            cli_cache.is_none(),
            "{cli_args:?}: rejection writes nothing"
        );
    }
    via_cli
}

/// @vtest.id TEST-AUDIT-STATIC-CLI-MCP-TRANSPORT-EQUIVALENCE
/// @vtest.covers VO-MCP-TOOL-CLI-EQUIVALENCE
/// @vtest.target crates/vtest-cli/src/ops/audit.rs::audit_static
/// @vtest.intent 無関係なTestに未解決target(E-SCAN-004)がある状態での正常Testの監査、未解決targetを持つTest自身の監査、--all、Test ID重複(E-SCAN-002)のそれぞれで、CLIのaudit staticと実際のvtest mcp stdioのtools/callのaudit_staticが同じok・data・diagnostics・cache作成有無になることを確認する
#[test]
fn audit_static_gives_the_same_result_over_cli_and_the_mcp_transport() {
    let root = fixture("transport");
    fs::write(
        root.join("tests/unrelated.rs"),
        "/// @vtest.id TEST-UNRESOLVED\n/// @vtest.covers VO-X\n\
         /// @vtest.target src/lib.rs::missing\n/// @vtest.intent nothing\n\
         #[test]\nfn unresolved() {\n    assert_eq!(audit_fixture::add(1, 1), 2);\n}\n",
    )
    .unwrap();

    // An unrelated Test's scan error does not stop the requested audit.
    let good = assert_cli_and_mcp_agree(
        &root,
        &["audit", "static", "--test", "TEST-GOOD"],
        json!({"test": "TEST-GOOD"}),
    );
    assert_eq!(good["ok"], true, "{good}");
    assert_eq!(good["data"]["tests"][0]["test"], "TEST-GOOD", "{good}");

    // The requested Test itself has the unresolved target.
    assert_cli_and_mcp_agree(
        &root,
        &["audit", "static", "--test", "TEST-UNRESOLVED"],
        json!({"test": "TEST-UNRESOLVED"}),
    );
    assert_cli_and_mcp_agree(&root, &["audit", "static", "--all"], json!({"all": true}));

    // A colliding Test ID is rejected the same way over both.
    fs::write(
        root.join("tests/duplicate.rs"),
        "/// @vtest.id TEST-GOOD\n/// @vtest.covers VO-X\n/// @vtest.target src/lib.rs::add\n\
         /// @vtest.intent nothing\n#[test]\nfn duplicate() {}\n",
    )
    .unwrap();
    let collided = assert_cli_and_mcp_agree(
        &root,
        &["audit", "static", "--test", "TEST-GOOD"],
        json!({"test": "TEST-GOOD"}),
    );
    assert_eq!(
        collided["diagnostics"][0]["code"], "E-SCAN-002",
        "{collided}"
    );
}
