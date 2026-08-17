//! W4 cluster 9 acceptance coverage: the text `report`/`verify` channel
//! rebuilt against 別紙A §12.2's Annex A detail-report contract.
//!
//! VO-REPORT-05 (sibling-aware branch marks, no ancestor-mark inheritance),
//! VO-AGG-04 / VO-AGG-08 (a `Project checks` section for every requested item
//! no tree node carries, and `basis` visible in text -- not JSON-only),
//! VO-EXIST-09 (a non-PASS `test_traceability` cites adapter ID, source
//! location, diagnostic code and verdict, in text as well as JSON).

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT_PROJECT: AtomicU64 = AtomicU64::new(0);

struct TempProject {
    root: PathBuf,
}

impl TempProject {
    /// A bare `vtest init` project -- no pre-existing entities -- for tests
    /// that build their own SPEC/REQ/VO/Test graph from scratch.
    fn fresh(name: &str) -> Self {
        let project = Self { root: temp_root(name) };
        fs::create_dir_all(&project.root).expect("create temporary project root");
        let output = invoke(&project.root, "init", &["--name", name]);
        assert_exit(&output, 0, "initialize a fresh temporary project");
        project
    }

    fn from_m1_base(name: &str) -> Self {
        let root = temp_root(name);
        copy_tree(&fixture_path("m1/base"), &root);
        fs::create_dir_all(root.join(".verify/approvals"))
            .expect("restore canonical approval directory");
        Self { root }
    }

    fn commit_baseline(&self) {
        run_git(&self.root, ["init", "-q"], "initialize temporary git repository");
        run_git(
            &self.root,
            ["config", "user.email", "w4-annex-a@example.invalid"],
            "configure temporary git email",
        );
        run_git(&self.root, ["config", "user.name", "W4 Annex A"], "configure temporary git name");
        run_git(
            &self.root,
            ["config", "commit.gpgsign", "false"],
            "disable signing for the disposable baseline commit",
        );
        run_git(&self.root, ["add", "."], "stage temporary baseline");
        run_git(&self.root, ["commit", "-qm", "w4 annex-a baseline"], "commit temporary baseline");
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        if self.root.exists() {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

fn temp_root(name: &str) -> PathBuf {
    let sequence = NEXT_PROJECT.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    env::temp_dir().join(format!("vtest-cli-w4annexa-{name}-{}-{nanos}-{sequence}", std::process::id()))
}

fn fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/calc")
        .join(relative)
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create fixture destination");
    for entry in fs::read_dir(source).expect("read tracked fixture directory") {
        let entry = entry.expect("read tracked fixture entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type().expect("read fixture file type").is_dir() {
            copy_tree(&source_path, &destination_path);
        } else {
            fs::copy(source_path, destination_path).expect("copy tracked fixture file");
        }
    }
}

fn run_git<const N: usize>(root: &Path, args: [&str; N], context: &str) {
    let output = Command::new("git").current_dir(root).args(args).output().expect("run git");
    assert!(
        output.status.success(),
        "{context}: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn invoke(project: &Path, command: &str, args: &[&str]) -> Output {
    invoke_format(project, "json", command, args)
}

fn invoke_text(project: &Path, command: &str, args: &[&str]) -> Output {
    invoke_format(project, "text", command, args)
}

fn invoke_format(project: &Path, format: &str, command: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vtest"))
        .arg("--project")
        .arg(project)
        .args(["--format", format, command])
        .args(args)
        .output()
        .expect("run vtest process")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("vtest emits UTF-8 text")
}

fn assert_exit(output: &Output, expected: i32, context: &str) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{context}: stdout={} stderr={}",
        stdout(output),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// @vtest.id TEST-CLI-150
/// @vtest.covers VO-REPORT-05
/// @vtest.target crates/vtest-cli/src/lib.rs::print_tree_node
/// @vtest.intent a middle sibling gets `├─ `, the last sibling gets `└─ `,
/// the two are byte-distinguishable, and the old hardcoded-root-mark bug
/// (every descendant line opening with the top node's own `├─ `) is gone.
#[test]
fn m6_two_vo_siblings_get_distinct_middle_and_last_branch_marks() {
    let project = TempProject::fresh("branch-marks");
    fs::write(project.root.join("SPEC.md"), "spec text\n").expect("write fixture spec source");
    assert_exit(
        &invoke(&project.root, "spec", &["add", "--id", "SPEC-A", "--path", "SPEC.md"]),
        0,
        "register SPEC-A",
    );
    assert_exit(
        &invoke(&project.root, "req", &["add", "--id", "REQ-A", "--summary", "req a", "--spec", "SPEC-A"]),
        0,
        "register REQ-A",
    );
    assert_exit(
        &invoke(&project.root, "vo", &["add", "--id", "VO-A", "--claim", "claim a", "--req", "REQ-A"]),
        0,
        "register VO-A",
    );
    assert_exit(
        &invoke(&project.root, "vo", &["add", "--id", "VO-B", "--claim", "claim b", "--req", "REQ-A"]),
        0,
        "register VO-B",
    );

    let verify = invoke_text(&project.root, "verify", &["--req", "REQ-A"]);
    let output = stdout(&verify);

    // VO-A is the middle sibling (VO-B follows it under the same REQ), VO-B
    // is the last -- 別紙A L232-234 requires distinct marks for each, not
    // the pre-fix behavior where every node inherited the root's own `├─ `.
    assert!(output.contains("├─ vo:VO-A"), "middle VO mark: {output}");
    assert!(output.contains("└─ vo:VO-B"), "last VO mark: {output}");
    assert!(
        !output.contains("├─ │"),
        "an ancestor's own mark must not be baked into every descendant line: {output}"
    );
    assert!(
        output.matches("└─ ").count() >= 2,
        "the last-sibling glyph must appear (root AND VO-B at minimum): {output}"
    );
}

/// @vtest.id TEST-CLI-151
/// @vtest.covers VO-EXIST-09
/// @vtest.target crates/vtest-cli/src/lib.rs::render_report_text
/// @vtest.intent a non-PASS `test_traceability` is listed under a `Project
/// checks` section in TEXT with adapter ID, source location, diagnostic code
/// and verdict -- the same tuple 別紙A §12.2 requires JSON to carry, not
/// silently dropped the way the pre-fix renderer dropped the whole item.
#[test]
fn m6_text_report_lists_unregistered_test_under_project_checks_with_full_citation() {
    let project = TempProject::from_m1_base("exist-09");
    fs::copy(
        fixture_path("m1/cases/unregistered.rs"),
        project.root.join("tests/unregistered.rs"),
    )
    .expect("add an unregistered test fixture");
    project.commit_baseline();

    let verify = invoke_text(&project.root, "verify", &["--items", "test_traceability"]);
    assert_exit(&verify, 1, "unregistered test drives test_traceability MISSING");
    let output = stdout(&verify);

    assert!(output.contains("Project checks:"), "{output}");
    assert!(output.contains("test_traceability"), "{output}");
    assert!(output.contains("W-SCAN-101"), "diagnostic code: {output}");
    assert!(output.contains("rust-cargo"), "adapter ID: {output}");
    assert!(
        output.contains("tests/unregistered.rs::unregistered_fixture_test"),
        "source location: {output}"
    );
    assert!(output.contains("Missing"), "verdict: {output}");
}

/// @vtest.id TEST-CLI-152
/// @vtest.covers VO-AGG-04
/// @vtest.target crates/vtest-cli/src/lib.rs::render_report_text
/// @vtest.intent when `spec_coverage` decides the overall NG result but no
/// SPEC/REQ node exists in the tree (a VO-only project), the text channel
/// still shows the item and its value under `Project checks` -- detail
/// output must not lose the deciding value (基本仕様 §4.3 L220).
#[test]
fn m6_text_report_surfaces_the_deciding_item_missing_from_every_tree_node() {
    let project = TempProject::from_m1_base("agg-04");
    project.commit_baseline();

    let verify = invoke_text(&project.root, "verify", &[]);
    assert_exit(&verify, 1, "no SPEC exists, so spec_coverage decides MISSING");
    let output = stdout(&verify);

    assert!(output.contains("Project checks:"), "{output}");
    let checks_section = output
        .split("Project checks:")
        .nth(1)
        .and_then(|rest| rest.split("Result:").next())
        .unwrap_or_default();
    assert!(
        checks_section.contains("spec_coverage") && checks_section.contains("MISSING"),
        "the deciding spec_coverage=MISSING must be visible under Project checks: {output}"
    );
    assert!(output.contains("Result: MISSING"), "{output}");
}

/// @vtest.id TEST-CLI-153
/// @vtest.covers VO-AGG-08
/// @vtest.target crates/vtest-cli/src/lib.rs::print_item_line
/// @vtest.intent `ReportItem.basis` -- the audit-record citation -- is
/// printed in the TEXT channel, not only JSON (別紙A §12.2's "JSONでも同じ
/// 根拠一覧を返す" makes text the primary detail channel).
#[test]
fn m6_text_report_prints_the_static_audit_basis_not_only_json() {
    let project = TempProject::from_m1_base("agg-08-basis");
    project.commit_baseline();
    assert_exit(
        &invoke(&project.root, "audit", &["static", "--all"]),
        1,
        "static-audit the single fixture test",
    );

    let verify = invoke_text(&project.root, "verify", &["--items", "static_audit"]);
    let output = stdout(&verify);
    assert!(
        output.contains("current static audit record(s)"),
        "the audit-record citation must reach the text channel, not only JSON: {output}"
    );
}
