//! M3 deterministic-audit command-line acceptance coverage.
//!
//! This test invokes the public binary against an isolated copy of the M1
//! fixture.  Keeping every audit invocation in one test makes the required
//! sequence deterministic while avoiding writes to tracked canonical data.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::Value;
use vtest_store::read_audit;

static NEXT_PROJECT: AtomicU64 = AtomicU64::new(0);

struct TempProject {
    root: PathBuf,
}

impl TempProject {
    fn from_m1_base(name: &str) -> Self {
        let sequence = NEXT_PROJECT.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "vtest-cli-m3-{name}-{}-{nanos}-{sequence}",
            std::process::id()
        ));
        copy_tree(&fixture_path("m1/base"), &root);
        fs::create_dir_all(root.join(".verify/audits"))
            .expect("restore the canonical audit directory");
        Self { root }
    }

    fn write_m3_cases(&self) {
        let config_path = self.root.join(".verify/config.yaml");
        let config = fs::read_to_string(&config_path).expect("read temporary project config");
        let configured = config.replace(
            "  assertion_macros: []",
            "  assertion_macros:\n    - check_known",
        );
        assert_ne!(
            configured, config,
            "fixture config has assertion macro field"
        );
        fs::write(config_path, configured).expect("configure the custom assertion macro");
        fs::write(
            self.root.join("src/lib.rs"),
            "/// @vtest.src-id SRC-M3-KNOWN\npub fn known() {}\n\npub fn m3_other() {}\n",
        )
        .expect("add the cross-file helper to the temporary fixture");
        fs::write(self.root.join("tests/m3_rules.rs"), M3_CASES)
            .expect("write deterministic-audit cases");
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        if self.root.exists() {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

fn fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/calc")
        .join(relative)
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create fixture destination");
    for entry in fs::read_dir(source).expect("read fixture directory") {
        let entry = entry.expect("read fixture entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type().expect("read fixture entry type").is_dir() {
            copy_tree(&source_path, &destination_path);
        } else {
            fs::copy(source_path, destination_path).expect("copy fixture file");
        }
    }
}

fn invoke_static(project: &Path, test_id: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vtest"))
        .args(["--project"])
        .arg(project)
        .args(["--format", "json", "audit", "static", "--test", test_id])
        .output()
        .expect("run vtest audit static")
}

fn invoke_static_all(project: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vtest"))
        .args(["--project"])
        .arg(project)
        .args(["--format", "json", "audit", "static", "--all"])
        .output()
        .expect("run vtest audit static --all")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("vtest emits UTF-8 JSON")
}

fn audit_response(output: &Output, expected_exit: i32, context: &str) -> Value {
    assert_eq!(
        output.status.code(),
        Some(expected_exit),
        "{context}: stdout={} stderr={}",
        stdout(output),
        String::from_utf8_lossy(&output.stderr)
    );
    let response: Value = serde_json::from_str(&stdout(output)).expect("valid JSON envelope");
    assert!(response["ok"].is_boolean(), "{context}: {response}");
    assert!(response["data"].is_object(), "{context}: {response}");
    assert!(response["diagnostics"].is_array(), "{context}: {response}");
    response
}

fn only_audit<'a>(response: &'a Value, test_id: &str) -> &'a Value {
    let audits = response["data"]["audits"]
        .as_array()
        .expect("static audit has an audits array");
    assert_eq!(
        audits.len(),
        1,
        "test filter must return one audit: {response}"
    );
    assert_eq!(
        audits[0]["test_id"], test_id,
        "wrong test audit: {response}"
    );
    &audits[0]
}

fn rule<'a>(audit: &'a Value, name: &str) -> &'a Value {
    audit["rules"]
        .as_array()
        .expect("audit has a rules array")
        .iter()
        .find(|rule| rule["rule"] == name)
        .unwrap_or_else(|| panic!("missing {name}: {audit}"))
}

fn assert_failed_rule(response: &Value, test_id: &str, rule_name: &str) {
    let audit = only_audit(response, test_id);
    assert_eq!(audit["verdict"], "FAIL", "{test_id}: {audit}");
    let rule = rule(audit, rule_name);
    assert_eq!(rule["verdict"], "FAIL", "{test_id}: {rule}");
    assert!(
        rule["reason"]
            .as_str()
            .is_some_and(|reason| !reason.is_empty()),
        "{test_id} {rule_name} has no concrete reason: {rule}"
    );
    assert_eq!(rule["location"]["path"], "tests/m3_rules.rs");
    assert!(
        rule["location"]["locator"]
            .as_str()
            .is_some_and(|name| !name.is_empty())
            && rule["location"]["byte_range"]["start_line"]
                .as_u64()
                .is_some_and(|line| line > 0),
        "{test_id} {rule_name} has no concrete source location: {rule}"
    );
}

#[test]
fn m3_static_audit_maps_failures_preserves_unknown_and_warns_for_ignored_tests() {
    let project = TempProject::from_m1_base("deterministic-rules");
    project.write_m3_cases();

    for (test_id, rule_name) in [
        ("TEST-M3-DA-001", "DA-001"),
        ("TEST-M3-DA-002", "DA-002"),
        ("TEST-M3-DA-003", "DA-003"),
        ("TEST-M3-DA-004", "DA-004"),
        ("TEST-M3-DA-005", "DA-005"),
        ("TEST-M3-DA-006", "DA-006"),
    ] {
        let response = audit_response(
            &invoke_static(&project.root, test_id),
            1,
            "intentional deterministic violation must fail",
        );
        assert_failed_rule(&response, test_id, rule_name);
    }

    let normal = audit_response(
        &invoke_static(&project.root, "TEST-M3-NORMAL"),
        0,
        "a normal test has no deterministic violation",
    );
    let normal_audit = only_audit(&normal, "TEST-M3-NORMAL");
    assert_eq!(normal_audit["verdict"], "PASS", "{normal_audit}");
    assert!(
        normal_audit["rules"]
            .as_array()
            .expect("normal audit rules")
            .iter()
            .all(|rule| rule["verdict"] == "PASS"),
        "normal test has a deterministic violation: {normal_audit}"
    );

    let cross_file = audit_response(
        &invoke_static(&project.root, "TEST-M3-CROSS-FILE"),
        1,
        "a cross-file call leaves target reachability unknown",
    );
    let cross_file_audit = only_audit(&cross_file, "TEST-M3-CROSS-FILE");
    assert_eq!(cross_file_audit["verdict"], "UNKNOWN", "{cross_file_audit}");
    assert_eq!(rule(cross_file_audit, "DA-002")["verdict"], "UNKNOWN");
    assert_ne!(rule(cross_file_audit, "DA-002")["verdict"], "FAIL");

    let ignored = audit_response(
        &invoke_static(&project.root, "TEST-M3-IGNORED"),
        0,
        "an ignored test is warning-only for deterministic audit",
    );
    let ignored_audit = only_audit(&ignored, "TEST-M3-IGNORED");
    assert_eq!(ignored_audit["verdict"], "PASS", "{ignored_audit}");
    let warning = rule(ignored_audit, "W-DA-101");
    assert_eq!(
        warning["verdict"], "PASS",
        "ignored is not a failure: {warning}"
    );
    assert!(
        warning["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("ignore")),
        "ignored warning has no warning basis: {warning}"
    );
    assert_eq!(warning["location"]["path"], "tests/m3_rules.rs");

    assert_eq!(
        ignored["ok"], true,
        "ignored warning must not fail the command"
    );
    let diagnostic = ignored["diagnostics"]
        .as_array()
        .expect("top-level diagnostics array")
        .iter()
        .find(|diagnostic| diagnostic["code"] == "W-DA-101")
        .unwrap_or_else(|| panic!("ignored warning is absent from diagnostics: {ignored}"));
    assert_eq!(diagnostic["severity"], "warning", "{diagnostic}");
    assert_eq!(diagnostic["location"]["path"], "tests/m3_rules.rs");
    assert_eq!(diagnostic["location"]["locator"], "ignored_test_warns");

    for test_id in ["TEST-M3-CONFIGURED-MACRO", "TEST-M3-SRC-ID"] {
        let response = audit_response(
            &invoke_static(&project.root, test_id),
            0,
            "bounded target analysis must recognize a trustworthy test",
        );
        let audit = only_audit(&response, test_id);
        assert_eq!(audit["verdict"], "PASS", "{test_id}: {audit}");
        assert_eq!(rule(audit, "DA-002")["verdict"], "PASS", "{audit}");
        assert!(
            audit["rules"]
                .as_array()
                .expect("edge-case audit rules")
                .iter()
                .all(|rule| rule["verdict"] == "PASS"),
            "{test_id} has an unexpected non-pass rule: {audit}"
        );
    }

    let helper = audit_response(
        &invoke_static(&project.root, "TEST-M3-SAME-FILE-HELPER"),
        1,
        "one-hop reachability is proven but helper result-flow remains bounded",
    );
    let helper = only_audit(&helper, "TEST-M3-SAME-FILE-HELPER");
    assert_eq!(helper["verdict"], "UNKNOWN", "{helper}");
    assert_eq!(rule(helper, "DA-002")["verdict"], "PASS", "{helper}");
    assert_eq!(rule(helper, "DA-003")["verdict"], "UNKNOWN", "{helper}");

    let substring = audit_response(
        &invoke_static(&project.root, "TEST-M3-DA-003-SUBSTRING"),
        1,
        "an unrelated target-named flag cannot verify a discarded result",
    );
    assert_failed_rule(&substring, "TEST-M3-DA-003-SUBSTRING", "DA-003");

    let nested_comma = audit_response(
        &invoke_static(&project.root, "TEST-M3-DA-004-NESTED-COMMA"),
        1,
        "nested commas cannot hide a token-identical self-comparison",
    );
    assert_failed_rule(&nested_comma, "TEST-M3-DA-004-NESTED-COMMA", "DA-004");

    let inline_a = audit_response(
        &invoke_static(&project.root, "TEST-M3-INLINE-A"),
        0,
        "first same-named inline-module test is selected by exact path",
    );
    let inline_a = only_audit(&inline_a, "TEST-M3-INLINE-A");
    assert_eq!(inline_a["verdict"], "PASS", "{inline_a}");
    assert_eq!(
        rule(inline_a, "DA-004")["location"]["locator"],
        "inline_a::same_name"
    );

    let inline_b = audit_response(
        &invoke_static(&project.root, "TEST-M3-INLINE-B"),
        1,
        "second same-named inline-module test is selected by exact path",
    );
    assert_failed_rule(&inline_b, "TEST-M3-INLINE-B", "DA-004");
    assert_eq!(
        rule(only_audit(&inline_b, "TEST-M3-INLINE-B"), "DA-004")["location"]["locator"],
        "inline_b::same_name"
    );

    let all_project = TempProject::from_m1_base("audit-all-persistence");
    all_project.write_m3_cases();
    let all = audit_response(
        &invoke_static_all(&all_project.root),
        1,
        "audit static --all includes intentional failures",
    );
    let all_audits = all["data"]["audits"]
        .as_array()
        .expect("--all returns an audits array");
    assert!(all_audits.len() > 1, "--all audited no test set: {all}");
    let audit_files = fs::read_dir(all_project.root.join(".verify/audits"))
        .expect("read persisted static audits")
        .map(|entry| entry.expect("read persisted audit entry").path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("yaml"))
        .collect::<Vec<_>>();
    assert_eq!(
        audit_files.len(),
        all_audits.len(),
        "--all persists exactly one canonical audit per returned test"
    );
    let mut saw_per_target_verdict = false;
    for audit in all_audits {
        let audit_id = audit["id"].as_str().expect("audit response has an ID");
        let test_id = audit["test_id"]
            .as_str()
            .expect("audit response has a Test ID");
        let path = all_project
            .root
            .join(format!(".verify/audits/{audit_id}.yaml"));
        let yaml = fs::read_to_string(&path).expect("read persisted audit YAML");
        assert!(
            yaml.contains("\nrevision: { commit:"),
            "missing revision: {yaml}"
        );
        let record = read_audit(&path).expect("vtest-store parses persisted static audit");
        assert_eq!(record.id, audit_id);
        assert_eq!(record.kind, "static");
        assert_eq!(record.bundle_id, None);
        assert!(
            record
                .subjects
                .iter()
                .any(|subject| subject.id.as_deref() == Some(test_id)),
            "audit has no Test subject: {record:?}"
        );
        assert!(
            record
                .subjects
                .iter()
                .filter(|subject| subject.locator.is_some())
                .count()
                >= 2,
            "audit does not bind both Test code and its resolved target: {record:?}"
        );
        assert!(
            !record.reasons.is_empty(),
            "audit has no reasons: {record:?}"
        );
        assert!(
            record.reasons.iter().all(|reason| {
                reason.rule.as_deref().is_some_and(|rule| !rule.is_empty())
                    && reason
                        .verdict
                        .as_deref()
                        .is_some_and(|verdict| !verdict.is_empty())
                    && !reason.claim.is_empty()
                    && !reason.basis.is_empty()
                    && reason
                        .basis
                        .iter()
                        .all(|basis| !basis.kind.is_empty() && !basis.reference.is_empty())
            }),
            "audit reasons are not structured and traceable: {record:?}"
        );
        // The target-scoped rules carry a per-target verdict list keyed by the
        // resolved canonical Locator (詳細設計 §3.6, §7.2). Every M3 case declares
        // a resolvable target, so DA-002 / DA-003 emit the list.
        for reason in &record.reasons {
            if !matches!(reason.rule.as_deref(), Some("DA-002" | "DA-003")) {
                assert!(
                    reason.targets.is_empty(),
                    "only DA-002/DA-003 carry per-target verdicts: {reason:?}"
                );
                continue;
            }
            for target in &reason.targets {
                saw_per_target_verdict = true;
                assert!(
                    target.target.starts_with("rust-cargo::"),
                    "per-target identity must be a canonical Locator, not a declared spelling: {target:?}"
                );
                assert!(
                    matches!(target.verdict.as_str(), "PASS" | "UNKNOWN" | "FAIL"),
                    "per-target verdict must be PASS/UNKNOWN/FAIL: {target:?}"
                );
                assert!(
                    !target.basis.is_empty(),
                    "per-target verdict must cite a basis: {target:?}"
                );
            }
        }
    }
    assert!(
        saw_per_target_verdict,
        "the M3 batch produced no per-target DA-002/DA-003 verdicts"
    );
}

const M3_CASES: &str = r#"
use calc_m1_base::m3_other;

/// @vtest.src-id SRC-M3-LOCAL-KNOWN
fn known() {}

macro_rules! check_known {
    ($actual:expr) => {
        assert_eq!($actual, ())
    };
}

fn same_file_helper() {
    known();
}

fn nested_pair(_: (), pair: (i32, i32)) -> (i32, i32) {
    pair
}

/// @vtest.id TEST-M3-DA-001
/// @vtest.covers VO-KNOWN
/// @vtest.target tests/m3_rules.rs::known
/// @vtest.intent constant assertion is deterministically weak
#[test]
fn da001_constant_assertion() {
    let _ = known();
    assert!(true);
}

/// @vtest.id TEST-M3-DA-002
/// @vtest.covers VO-KNOWN
/// @vtest.target tests/m3_rules.rs::known
/// @vtest.intent declared target is never called
#[test]
fn da002_target_not_called() {
    assert_eq!(1, 1);
}

/// @vtest.id TEST-M3-DA-003
/// @vtest.covers VO-KNOWN
/// @vtest.target tests/m3_rules.rs::known
/// @vtest.intent target result is never verified
#[test]
fn da003_result_not_verified() {
    let _ = known();
    assert!(true);
}

/// @vtest.id TEST-M3-DA-004
/// @vtest.covers VO-KNOWN
/// @vtest.target tests/m3_rules.rs::known
/// @vtest.intent self-comparison is deterministically weak
#[test]
fn da004_self_comparison() {
    let actual = known();
    assert_eq!(actual, actual);
}

/// @vtest.id TEST-M3-DA-005
/// @vtest.covers VO-KNOWN
/// @vtest.target tests/m3_rules.rs::known
/// @vtest.intent empty test body is invalid
#[test]
fn da005_empty_test() {}

/// @vtest.id TEST-M3-DA-006
/// @vtest.covers VO-KNOWN
/// @vtest.target tests/m3_rules.rs::known
/// @vtest.intent verification syntax is absent
#[test]
fn da006_no_verification_syntax() {
    let _ = known();
}

/// @vtest.id TEST-M3-NORMAL
/// @vtest.covers VO-KNOWN
/// @vtest.target tests/m3_rules.rs::known
/// @vtest.intent target result is checked
#[test]
fn normal_test() {
    assert_eq!(known(), ());
}

/// @vtest.id TEST-M3-CROSS-FILE
/// @vtest.covers VO-KNOWN
/// @vtest.target tests/m3_rules.rs::known
/// @vtest.intent a different source-file call could invoke the target indirectly
#[test]
fn cross_file_call_is_unknown() {
    let indirect = m3_other();
    assert_eq!(indirect, ());
}

/// @vtest.id TEST-M3-IGNORED
/// @vtest.covers VO-KNOWN
/// @vtest.target tests/m3_rules.rs::known
/// @vtest.intent ignored tests are warning-only
#[test]
#[ignore]
fn ignored_test_warns() {
    assert_eq!(known(), ());
}

/// @vtest.id TEST-M3-CONFIGURED-MACRO
/// @vtest.covers VO-KNOWN
/// @vtest.target tests/m3_rules.rs::known
/// @vtest.intent configured assertion macros verify target results
#[test]
fn configured_assertion_macro() {
    check_known!(known());
}

/// @vtest.id TEST-M3-SAME-FILE-HELPER
/// @vtest.covers VO-KNOWN
/// @vtest.target tests/m3_rules.rs::known
/// @vtest.intent one same-file helper hop reaches the declared target
#[test]
fn same_file_helper_reaches_target() {
    assert_eq!(same_file_helper(), ());
}

/// @vtest.id TEST-M3-SRC-ID
/// @vtest.covers VO-KNOWN
/// @vtest.target SRC-M3-LOCAL-KNOWN
/// @vtest.intent permanent source IDs resolve to the called target
#[test]
fn src_id_target_resolves() {
    assert_eq!(known(), ());
}

/// @vtest.id TEST-M3-DA-003-SUBSTRING
/// @vtest.covers VO-KNOWN
/// @vtest.target tests/m3_rules.rs::known
/// @vtest.intent a similarly named flag does not verify discarded target output
#[test]
fn target_name_substring_is_not_result_flow() {
    let _ = known();
    let known_called = true;
    assert!(known_called);
}

/// @vtest.id TEST-M3-DA-004-NESTED-COMMA
/// @vtest.covers VO-KNOWN
/// @vtest.target tests/m3_rules.rs::known
/// @vtest.intent nested comma expressions remain comparable token sequences
#[test]
fn nested_comma_self_comparison() {
    assert_eq!(
        nested_pair(known(), (1, 2)),
        nested_pair(known(), (1, 2))
    );
}

mod inline_a {
    /// @vtest.id TEST-M3-INLINE-A
    /// @vtest.covers VO-KNOWN
    /// @vtest.target tests/m3_rules.rs::known
    /// @vtest.intent exact module path selects this normal same-named test
    #[test]
    fn same_name() {
        assert_eq!(crate::known(), ());
    }
}

mod inline_b {
    /// @vtest.id TEST-M3-INLINE-B
    /// @vtest.covers VO-KNOWN
    /// @vtest.target tests/m3_rules.rs::known
    /// @vtest.intent exact module path selects this weak same-named test
    #[test]
    fn same_name() {
        let actual = crate::known();
        assert_eq!(actual, actual);
    }
}
"#;
